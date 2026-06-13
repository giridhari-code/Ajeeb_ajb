# Runtime Memory Management — Design Document

## 1. Audit Summary

### Current State
- **`ajeeb_runtime.c`** (420 lines): all strings allocated via `malloc`, **never freed** — every call to `str_concat`, `substring`, `replace`, `trim`, `toUpperCase`, `toLowerCase`, `itoa`, `readFile` leaks
- **Value representation**: `intptr_t` — no tag, no discriminant; impossible to distinguish Int from Pointer at runtime
- **Rust evaluator** (`eval.rs`): `RuntimeValue` enum with `Rc<RefCell<String>>` — reference cycles in arrays of struct instances never collected; every function call clones the entire variable scope (`self.variables.clone()` at line 1522)
- **No GC, no RC, no arena, no region allocator**

### Identified Gaps
| Gap | Severity | Description |
|-----|----------|-------------|
| No memory management strategy | BLOCKER | Every string leaks; `intptr_t` untyped is a security/safety risk |
| No allocator abstraction | HIGH | All allocations go through `malloc`; no arena, no bump, no region |
| No value tagging | HIGH | `intptr_t` can't distinguish Int(42) from Pointer("hello") |
| Rc cycles leak in Rust eval | MEDIUM | Arrays of struct instances can form reference cycles |

### Breaking Changes
1. **RuntimeValue in eval.rs** — current representation uses `Rc<RefCell<String>>` and `Rc<RefCell<Vec<RuntimeValue>>>`; reference-counted values must move to `Weak`-aware patterns or arena indices
2. **C runtime ABI** — all function signatures currently return/take `intptr_t`; must change to a tagged union `AjeebValue`
3. **All builtin function signatures** in `exec_fn_call_body` — pattern matching on `intptr_t` must change to pattern matching on `AjeebValue`

---

## 2. Design: Arena Allocator

### 2.1 C Runtime Arena

```c
typedef struct {
    char* base;
    size_t offset;
    size_t capacity;
} Arena;

Arena* arena_create(size_t initial_capacity);
void* arena_alloc(Arena* a, size_t size);    // bump allocate
void arena_reset(Arena* a);                  // reset entire arena (fast free)
void arena_destroy(Arena* a);
```

**Strategy**: Bump-pointer arena. One arena per request/execution context. Reset entire arena between top-level evaluations. No individual frees within arena — arena reset is O(1).

### 2.2 Tagged Runtime Value

```c
typedef enum {
    AJB_INT,
    AJB_FLOAT,
    AJB_BOOL,
    AJB_STRING,     // arena-allocated
    AJB_VOID,
    AJB_ARRAY,      // arena-allocated
    AJB_STRUCT,     // arena-allocated
    AJB_ENUM,       // arena-allocated
} AjeebType;

typedef struct {
    AjeebType type;
    int64_t as_int;
    double as_float;
} AjeebValue;

// String data lives in arena; AjeebValue holds offset + length
typedef struct {
    size_t offset;   // offset into current arena
    size_t len;
} AjeebString;

// Array (flat, arena-backed)
typedef struct {
    size_t count;
    size_t capacity;
    size_t data_offset;  // offset into arena of AjeebValue[]
} AjeebArray;

// For struct/enum — similar flat representation
```

### 2.3 Rust Evaluator — Arena-Backed Values

Replace `RuntimeValue` enum with arena-indexed handles:

```rust
struct AjeebArena {
    data: Vec<u8>,
    offset: usize,
}

#[derive(Copy, Clone)]
struct ValueHandle(u64);  // arena_id | offset | tag

enum ValueTag {
    Int, Float, Bool, String, Void, Array, Struct, Enum,
}
```

---

## 3. Design: Reference Counting

### 3.1 When RC Is Needed

Arena reset is coarse-grained. For values that outlive their arena (e.g., returned from a function whose arena was reset), use reference counting.

**Simplified approach for Phase 1**: Don't mix arenas. Use a single arena per program execution. Reset at program end. No individual object lifetimes needed.

**Phase 2**: Per-function arenas with RC for returned values.

### 3.2 C Reference Counting

```c
typedef struct {
    int refcount;
    AjeebType type;
    union { /* ... */ } data;
} AjeebRcObject;

AjeebRcObject* rc_alloc(AjeebType type);
void rc_retain(AjeebRcObject* obj);
void rc_release(AjeebRcObject* obj);  // frees if refcount == 0
```

### 3.3 Externally Managed Strings (Zero-Copy Interop)

For FFI interop with C libraries (SQLite, libcurl), strings from C must not be arena-managed. Use a separate `ExternalString` type:

```c
typedef struct {
    char* data;        // externally owned
    size_t len;
    void (*free_fn)(char*);
} AjeebExternalString;
```

---

## 4. Implementation Plan

### Phase 1A: Arena Allocator in C Runtime (2-3 weeks)

1. Add `Arena` struct + `arena_create`, `arena_alloc`, `arena_reset`, `arena_destroy` to `ajeeb_runtime.c`
2. Add `AjeebValue` tagged union type
3. Replace all `intptr_t` returns with `AjeebValue`
4. Replace all `char*` allocations with `arena_alloc`
5. Update all string builtins to use arena: `str_concat`, `substring`, `replace`, `trim`, `toUpperCase`, `toLowerCase`, `itoa`, `readFile`
6. Add single global arena; reset at startup

### Phase 1B: Rust Evaluator — Arena-Backed Values (2-3 weeks)

1. Add `AjeebArena` struct to `eval.rs`
2. Replace `RuntimeValue::String(Rc<RefCell<String>>)` with arena-handle type
3. Replace `RuntimeValue::Array(Rc<RefCell<Vec<RuntimeValue>>>)` with flat arena-backed array
4. Update all builtin functions to work with new value representation
5. Add `ValueHandle` as the primary return type from C runtime calls

### Phase 1C: Leak Detection (1 week)

1. Add `Arena::alloc_count` debug counter
2. At arena reset, assert `alloc_count == free_count` (or report leaked allocations)
3. Integration test that runs a complex program and asserts no leaks
4. `valgrind` CI integration to verify zero leaks post-arena-reset

### Phase 1D: Remove `intptr_t` Ownership Ambiguity (1 week)

1. Verify no `intptr_t` remains in public API of `ajeeb_runtime.c`
2. All function signatures use `AjeebValue`
3. Rust `extern "C"` bindings use `AjeebValue` type
4. No raw pointer casts in upper layers

---

## 5. Complexity Estimate

| Step | Lines Changed | Files | Effort |
|------|--------------|-------|--------|
| Arena allocator in C | ~200 new | `runtime.c` | 1 week |
| AjeebValue tagged union | ~50 new | `runtime.h` (new) | 2 days |
| Port builtins to arena | ~300 changed | `runtime.c` | 1 week |
| Rust arena + ValueHandle | ~400 changed | `eval.rs` | 2 weeks |
| Leak detection | ~100 new | `runtime.c`, `test_*.ajb` | 1 week |
| Remove intptr_t | ~100 changed | `main.rs`, `eval.rs` | 2 days |
| **Total** | **~1150** | **~6 files** | **4-5 weeks** |

---

## 6. Migration Path

1. Add new types alongside existing — dual API during migration
2. First port a single builtin (e.g., `str_concat`) as proof of concept
3. Run existing test suite — all tests must pass at each step
4. Remove old `intptr_t` paths only when all builtins are ported
5. Final step: `valgrind --leak-check=full` must show 0 leaks
