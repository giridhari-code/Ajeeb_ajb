# LLVM Codegen Parity Audit: Rust vs Ajeeb Self-Hosted

**Date:** 2026-06-22
**Status:** COMPLETE

---

## Executive Summary

The Rust compiler's LLVM backend generates **real LLVM IR** (not C code compiled by clang). The Ajeeb compiler currently has **only C codegen** — it outputs C source text compiled by GCC. Achieving LLVM parity means Ajeeb must generate LLVM IR directly, or use a C→LLVM path.

**Rust LLVM backend: 2,325 LOC across 7 files.**
**Ajeeb LLVM backend: 0 LOC.**

---

## Architecture Comparison

### Rust LLVM Backend

```
Source → Lexer → Parser → AST → HIR → THIR → MIR → LLVM
                                            ↓
                                      LLVMContext
                                            ↓
                                       Module (IRBuilder)
                                            ↓
                                      LLVM bitcode/object
                                            ↓
                                        LLD linker
                                            ↓
                                        Native binary
```

- Uses **inkwell** Rust bindings to LLVM C API
- Each MIR basic block → LLVM BasicBlock
- Each MIR statement → LLVM instruction via IRBuilder
- Each MIR terminator → LLVM terminator
- Supports: `i64`, `i1` (bool), `ptr` (strings), arrays, structs
- String literals → `@.str` global constants
- Function calls → `call i64 @fn_name(args...)`
- Control flow → `br i1 %cond, label %then, label %else`

### Ajeeb C Backend (Current)

```
Source → Lexer → Parser → AST → HIR → MIR → C text → GCC → binary
```

- Generates C source as strings via `emitC()`
- String concat detected via pattern matching (5 patterns)
- Global state: `stateBuf`, `outbuf`, `lenBuf`
- Compiles to native via GCC (not clang)

### Target: Ajeeb LLVM Backend

Two possible approaches:

**Option A: Direct LLVM IR generation (like Rust)**
- Ajeeb emits LLVM IR text strings
- Compiled with `llc` + `ld` (no C compiler needed)
- Pro: True LLVM optimization, faster runtime
- Con: Most complex to implement

**Option B: C→LLVM via clang**
- Ajeeb still emits C code
- Clang generates LLVM IR
- Pro: Simplest, uses existing C backend
- Con: Clang dependency (not pure Ajeeb)

**Recommendation:** Option A (direct LLVM IR) — aligns with pure Ajeeb goal.

---

## Feature-by-Feature Parity

### 1. Type System Mapping

| LLVM Type | Rust Usage | Ajeeb Status |
|-----------|-----------|--------------|
| `i64` | All integers | AVAILABLE (C backend uses `int64_t`) |
| `i1` | Boolean | AVAILABLE (C backend uses `int`) |
| `i8*` / `ptr` | Strings, arrays | AVAILABLE (C backend uses `char*`) |
| `[N x i8]` | String literals | NEEDS WORK |
| `{ i64, i8* }` | String with length | NEEDS WORK |
| `{ i64, ptr }` | Structs | NEEDS WORK |
| `[N x i64]` | Arrays | NEEDS WORK |
| `void` | Void returns | AVAILABLE |
| `double` | Float (NOT YET IN AJEEB) | BLOCKED — no float type |

### 2. Core Codegen Operations

| Operation | Rust LLVM IR | Ajeeb Status | Complexity |
|-----------|-------------|--------------|------------|
| Integer literal | `@.str = private constant [N x i8] c"..."` | NEEDED | Easy |
| String literal | `@.str.1 = private constant [N x i8] c"...\00"` | NEEDED | Easy |
| Boolean | `i1 1` / `i1 0` | NEEDED | Easy |
| Void | `void` | NEEDED | Trivial |
| Variable load | `%var = load i64, ptr %addr` | NEEDED | Medium |
| Variable store | `store i64 %val, ptr %addr` | NEEDED | Medium |
| Global load | `%val = load i64, ptr @global` | NEEDED | Medium |
| Global store | `store i64 %val, ptr @global` | NEEDED | Medium |
| Binary op | `%result = add i64 %a, %b` | NEEDED | Easy |
| Comparison | `%cmp = icmp eq i64 %a, %b` | NEEDED | Easy |
| Function call | `%result = call i64 @fn(i64 %arg)` | NEEDED | Medium |
| Void call | `call void @fn()` | NEEDED | Easy |
| String concat (2 args) | `call i8* @str_concat(i8*, i8*)` | NEEDED | Medium |
| String concat (3+ args) | Nested calls or @tmp vars | NEEDED | Hard |
| Memory allocation | `@arena = global ptr null` | NEEDED | Hard |
| Array allocation | `[N x i64]` or `ptr` to malloc | NEEDED | Hard |
| Struct allocation | `{ i64, ptr }` or malloc | NEEDED | Hard |

### 3. Control Flow

| Operation | Rust LLVM IR | Ajeeb Status |
|-----------|-------------|--------------|
| Basic block creation | `append_basic_block(ctx, "name")` | NEEDED |
| Conditional branch | `br i1 %cond, label %then, label %else` | NEEDED |
| Unconditional branch | `br label %target` | NEEDED |
| Return | `ret i64 %val` | NEEDED |
| Void return | `ret void` | NEEDED |
| Phi node (for loops) | `%phi = phi i64 [%init, %entry], [%next, %latch]` | NEEDED |
| Loop label tracking | `loopStack` for break/continue | NEEDED |

### 4. Runtime Function Declarations

| Function | Rust LLVM Signature | Ajeeb Status |
|----------|-------------------|--------------|
| `println` | `declare void @println(i8*)` | NEEDED |
| `str_concat` | `declare i8* @str_concat(i8*, i8*)` | NEEDED |
| `strcmp` | `declare i64 @strcmp(i8*, i8*)` | NEEDED |
| `len` | `declare i64 @len(i8*)` | NEEDED |
| `charCode` | `declare i64 @charCode(i8*, i64)` | NEEDED |
| `chr` | `declare i8* @chr(i64)` | NEEDED |
| `getOutbuf` | `declare i8* @getOutbuf()` | NEEDED |
| `setOutbuf` | `declare void @setOutbuf(i8*)` | NEEDED |
| `rdPos` / `wrPos` | `declare i64 @rdPos(i8*)` / `declare void @wrPos(i8*, i64)` | NEEDED |
| `writeFile` | `declare i64 @writeFile(i8*, i8*)` | NEEDED |
| `readFile` | `declare i8* @readFile(i8*)` | NEEDED |
| `exec` | `declare i64 @exec(i8*)` | NEEDED |
| `itoa` | `declare i8* @itoa(i64)` | NEEDED |
| 57 total functions | (see runtime/ajeeb_runtime.c) | ALL NEEDED |

### 5. Global State

| State | Rust LLVM | Ajeeb Status |
|-------|----------|--------------|
| Arena allocator | `@arena = global ptr null` | NEEDED |
| State buffer (i64 array) | `@state = internal global [1024 x i64] zeroinitializer` | NEEDED |
| Output buffer | `@outbuf = internal global [65536 x i8] zeroinitializer` | NEEDED |
| Length buffer | `@lenbuf = internal global [65536 x i64] zeroinitializer` | NEEDED |
| String concatenation temp | `@concatbuf = internal global [256 x i8] zeroinitializer` | NEEDED |

### 6. String Concatenation Pattern (Critical)

The Rust codegen handles 5 patterns for `println(str_concat(a, b))`:

| Pattern | Rust Strategy | Ajeeb Status |
|---------|--------------|--------------|
| Two string literals | Inline C string concat | NEEDED |
| Two variables | `call i8* @str_concat(i8*, i8*)` | NEEDED |
| Three literals | Inline C string concat | NEEDED |
| Mixed literal+variable | `call i8* @str_concat(i8*, i8*)` with literal as first arg | NEEDED |
| Nested concat | Flatten to 2-arg calls | NEEDED |

**Ajeeb C backend workaround:** Detected `str_concat` in `println` args and emitted C string literals directly. This is **not possible** in LLVM — must generate actual LLVM IR calls.

### 7. Optimization Pipeline

| Optimization | Rust | Ajeeb Status |
|-------------|------|--------------|
| Dead block elimination (100-101) | `eliminate_dead_blocks()` | NEEDED |
| Unreachable block elimination (102-103) | `eliminate_unreachable_blocks()` | NEEDED |
| Dead code elimination (104) | `eliminate_dead_code()` | NEEDED |
| Constant folding (105-106) | `constant_fold()` | NEEDED |

---

## Implementation Estimate

| Phase | LOC | Complexity | Dependency |
|-------|-----|-----------|------------|
| LLVM IR emission (core) | 800-1,000 | High | None |
| Type mapping | 100-150 | Low | None |
| Control flow | 200-300 | Medium | None |
| Runtime function declarations | 100-150 | Low | None |
| Global state | 100-150 | Low | None |
| String concat patterns | 200-300 | High | Runtime functions |
| Array/struct allocation | 200-300 | High | None |
| Optimization passes | 200-300 | Medium | None |
| **Total** | **1,900-2,650** | | |

**vs Rust LLVM backend:** 2,325 LOC

**Estimated timeline:** 2-3 weeks (single developer, full-time)

---

## Recommendation

1. **Start with C backend** (already working, 85% parity)
2. **Add LLVM backend in parallel** as a separate module
3. **Use same MIR intermediate format** — just add new codegen target
4. **Ship both backends** — C for compatibility, LLVM for performance

The C backend gets 85% of the way there. The LLVM backend is a performance optimization, not a feature requirement.
