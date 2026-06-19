# C ABI FFI — Design Document

## 1. Audit Summary

### Current State
- **`interop.rs`** (40 lines): `LanguageBridge` struct with `HashMap<String, String>` — stores module name → language name pairs; `load_compatibility_block` just prints "Loading X compatibility for module Y" and inserts into map; `resolve` looks up the string; **zero actual FFI functionality**
- **No C ABI binding**: cannot call any C function, link to shared libraries, or pass structs across FFI boundary
- **No dynamic library loading**: no `dlopen`/`dlsym`/`GetProcAddress`
- **No WASM interop**: no `wasmtime`/`wasmer`
- **C runtime is statically linked**: `ajeeb_runtime.c` is compiled directly into the executable; no way to load third-party C libraries

```rust
// Current state — complete stub:
pub struct LanguageBridge {
    external_registry: HashMap<String, String>,  // module_name → "C"|"Rust"
}
```

### Identified Gaps
| Gap | Severity | Description |
|-----|----------|-------------|
| FFI is empty scaffolding | BLOCKER | No actual function calls across language boundary |
| No C ABI call support | BLOCKER | Can't call libc, libcurl, sqlite3, or any system library |
| No dynamic linking | HIGH | No `dlopen`/`dlsym` pattern; all C code must be compiled statically |
| No memory ownership across FFI | HIGH | String allocation/free across C/Ajeeb boundary undefined |
| No type marshaling | HIGH | No struct layout compatibility; no calling convention spec |

### Breaking Changes
1. **`interop.rs` must be completely rewritten** — current `LanguageBridge` has no useful functionality
2. **Ajeeb syntax**: `extern "C"` block becomes a new language construct (but user said "do NOT add syntax" — so must use existing AST + builtin mechanism)
3. **RuntimeValue must be FFI-safe**: `AjeebValue` from memory management design must have a stable C ABI representation

---

## 2. Design: No-New-Syntax Approach

Per constraint: **do not add new syntax**. FFI is exposed via:
1. **Builtin functions** in `eval.rs` / `ajeeb_runtime.c` (existing mechanism — just add new entries)
2. **Ajeeb functions that wrap C calls** — library functions (e.g., `sqlite3_open` becomes Ajeeb `sqlite_open` which calls C via builtin)
3. **`parth.das` configuration** — `[libraries]` section to specify which `.so`/`.dylib`/`.dll` to link

No `extern "C"` block syntax. No `#[link]` attribute. All FFI looks like regular function calls.

---

## 3. Design: Dynamic Library Loading

### 3.1 C Runtime — `dlopen`/`dlsym` Wrapper

```c
typedef struct {
    void* handle;       // from dlopen
    const char* name;   // library name for diagnostics
} AjeebLibrary;

// New builtin functions:
AjeebValue lib_open(AjeebValue path);    // dlopen → handle (or 0 on error)
AjeebValue lib_sym(AjeebValue lib, AjeebValue name);  // dlsym → fn pointer
AjeebValue lib_call(AjeebValue fn, AjeebValue* args, int nargs);  // call via fn ptr
AjeebValue lib_close(AjeebValue lib);   // dlclose
```

### 3.2 Marshaling

```c
// Internal conversion: AjeebValue → C type
void* marshal_to_c(AjeebValue v, const char* expected_type);
// expected_type: "int", "double", "char*", "void*"
// Returns stack-allocated or heap-allocated C value

AjeebValue marshal_from_c(void* c_val, const char* type);
// Converts C return value back to AjeebValue
```

**Phase 1**: Support only `int ↔ i64`, `string ↔ char*`, `void`. No struct marshaling.

### 3.3 Example: SQLite via Dynamic Loading

```ajeeb
// ajeeb-db/ajeeb-db.ajb — no new syntax
function db_open(path: string): int {
    // Internally calls lib_open("libsqlite3.so") + lib_sym("sqlite3_open") + lib_call
    return sqlite_open(path);  // existing builtin
}
```

This works today — `sqlite_open` is already a builtin. The difference: in Phase 3, `sqlite_open` will use `lib_open`/`lib_sym`/`lib_call` internally instead of compile-time linking.

---

## 4. Design: Type Marshaling Table

```rust
// interop.rs — complete rewrite
#[derive(Clone)]
enum MarshalingType {
    Int,        // i64 ↔ int64_t
    Float,      // f64 ↔ double
    String,     // String ↔ char* (arena-allocated copy)
    Void,
    Buffer,     // Vec<u8> ↔ void* + length
}

struct ForeignFunction {
    lib_handle: Option<*mut c_void>,  // from dlopen, or None for statically linked
    symbol_name: String,
    param_types: Vec<MarshalingType>,
    return_type: MarshalingType,
    variadic: bool,
}

struct ForeignLibrary {
    path: String,
    handle: Option<*mut c_void>,
    functions: HashMap<String, ForeignFunction>,
}

struct FFIBridge {
    libraries: HashMap<String, ForeignLibrary>,
    native_functions: HashMap<String, ForeignFunction>,  // statically linked
}
```

---

## 5. Design: Bindings for SQLite and libcurl

### 5.1 SQLite Bindings

Current `sqlite_*` builtins remain as the Ajeeb API. Under the hood, they:

**Compile-time (Phase 1)**: Link against `-lsqlite3` during `gcc` linking step.  
**Runtime (Phase 3)**: Use `lib_open("libsqlite3.so")` + `lib_sym("sqlite3_open")` etc.

```c
// ajeeb_runtime.c — Phase 3 implementation
AjeebValue sqlite_open(AjeebValue path) {
    static void* lib = NULL;
    static int (*real_open)(const char*, void**) = NULL;
    if (!lib) {
        lib = dlopen("libsqlite3.so", RTLD_NOW | RTLD_GLOBAL);
        if (!lib) return AJB_INT(0);
        real_open = dlsym(lib, "sqlite3_open");
    }
    void* db;
    int rc = real_open(path.as_string, &db);
    return AJB_INT((intptr_t)db);  // or use new AjeebValue tagged type
}
```

### 5.2 libcurl Bindings

```c
// New builtins in ajeeb_runtime.c
AjeebValue curl_easy_init(void);        // → handle
AjeebValue curl_set_url(AjeebValue handle, AjeebValue url);
AjeebValue curl_perform(AjeebValue handle);  // → response string
AjeebValue curl_cleanup(AjeebValue handle);
```

### 5.3 Integration with `parth.das`

```ini
[package]
name = "my-app"

[dependencies]

[libraries]
sqlite3 = "3.x"
curl = "7.x"

[compiler]
link_flags = "-lsqlite3 -lcurl"
```

---

## 6. Implementation Plan

### Phase 1A: `lib_open`/`lib_sym`/`lib_call` Builtins (2-3 weeks)

1. Add `#include <dlfcn.h>` to `ajeeb_runtime.c` (or `#include <windows.h>` for Win32)
2. Implement `lib_open(AjeebValue path)` → `AjeebValue` (handle or 0)
3. Implement `lib_sym(AjeebValue lib, AjeebValue name)` → function pointer
4. Implement `lib_call(AjeebValue fn_ptr, AjeebValue* args, int nargs)` → call via pointer
5. Register as builtins in `eval.rs` `exec_fn_call_body`
6. Test: `lib_open("libm.so.6")` → `lib_sym("sqrt")` → `lib_call(fn, [AjeebValue::from(9.0)])` → 3.0

### Phase 1B: Marshaling Layer (1-2 weeks)

1. Implement `marshal_to_c` for int, float, string, void
2. Implement `marshal_from_c` for int, float, string, void
3. Handle string ownership: arena-allocate strings from `char*`
4. Handle memory: C strings returned from functions must be freed properly
5. Unit tests for round-trip marshaling

### Phase 1C: FFIBridge in interop.rs (2 weeks)

1. Rewrite `interop.rs` with `ForeignLibrary`, `ForeignFunction`, `FFIBridge`
2. Add library registration via builtin `ffi_register_lib(path)` 
3. Add function registration via builtin `ffi_register_fn(lib, name, ret_type, param_types)`
4. Add `ffi_call(lib, fn_name, args...)` builtin that marshals and calls
5. Test: full round-trip calling `sqrt` from libm

### Phase 1D: SQLite + libcurl Bindings (2-3 weeks)

1. Rewrite `sqlite_*` builtins to use dynamic loading (with compile-time fallback)
2. Add `curl_*` builtins for HTTP requests
3. Create Ajeeb library wrappers in `packages/ajeeb-std/` or `packages/`
4. Integration tests: SQLite query, HTTP GET via libcurl

---

## 7. Complexity Estimate

| Step | Lines Changed | Files | Effort |
|------|--------------|-------|--------|
| lib_open/lib_sym/lib_call | ~150 | `ajeeb_runtime.c`, `eval.rs` | 2 weeks |
| Marshaling layer | ~200 | `ajeeb_runtime.c` (new header) | 1 week |
| FFIBridge rewrite | ~300 | `interop.rs` (complete rewrite) | 2 weeks |
| SQLite dynamic loading | ~100 | `ajeeb_runtime.c` | 1 week |
| libcurl bindings | ~200 | `ajeeb_runtime.c`, `eval.rs` | 2 weeks |
| Test suite | ~200 | `tests/*.ajb` | 1 week |
| **Total** | **~1150** | **~6 files** | **9-11 weeks** |

---

## 8. Risks and Mitigations

| Risk | Mitigation |
|------|------------|
| `dlopen` on Windows | Use `LoadLibrary`/`GetProcAddress` with `#ifdef _WIN32` |
| String memory across FFI boundary | Arena-allocate for Ajeeb→C; C string must be freed or arena-managed |
| Calling convention differences | Default to `cdecl` on x86; `_cdecl` attribute on Windows x86 |
| Thread safety | `FFIBridge` is `!Send`; single-threaded access only (matches current eval) |
| Missing lib at runtime | `lib_open` returns 0; error message lists `dlerror` |
