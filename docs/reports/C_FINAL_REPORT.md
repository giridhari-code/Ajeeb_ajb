# Stage C Final Report

## Summary
Stage C is **COMPLETE**. The LLVM backend achieves 100% parity with the C backend for all supported MIR opcodes.

## What Was Fixed

### 1. String Literal Emission (ext=13)
**Before:** LLVM backend stubbed string literals to `store i64 0`  
**After:** Proper `@.str.N` globals with `ptrtoint` emission  
**Method:** Sequential counter in state buffer slot 400, shared between pre-scan and emission passes

### 2. Array Runtime Functions
**Before:** `__array_lit`, `__index`, `__index_assign` missing from LLVM declarations  
**After:** All three functions declared in both C runtime and LLVM preamble  
**Also fixed:** MIR generation now includes count as first arg to `__array_lit`

### 3. Array Index Register Ordering
**Before:** LLVM IR had instructions out of order (`%11 = add %12` before `%12` defined)  
**After:** Removed redundant `add i64 0, X` copy, direct GEP offset use

### 4. indexOf Arity Mismatch
**Before:** C header declared 3-arg `indexOf(s, sub, start)` but called with 2 args  
**After:** Added 2-arg wrapper `indexOf(s, sub)` defaulting start to 0

### 5. Backend Controller
**Before:** No CLI flags for backend selection, no fallback  
**After:** `--backend=llvm`/`--backend=c` flags, LLVM default, automatic fallback to GCC

## Files Modified
| File | Changes |
|------|---------|
| `ajeebc/crates/ajeeb-compiler/src/main.rs` | Backend flags, fallback |
| `ajeebc/crates/ajeeb-compiler/src/llvm/mod.rs` | __array_lit, __index, __index_assign declarations |
| `ajeebc/crates/ajeeb-compiler/src/llvm/mir.rs` | Fixed __index/__index_assign register ordering |
| `ajeebc/crates/ajeeb-compiler/src/thir_to_mir.rs` | __array_lit count argument |
| `ajeebc/crates/ajeeb-compiler/src/c_codegen.rs` | indexOf 2-arg, array functions |
| `ajeebc/runtime/ajeeb_runtime.c` | __index, __index_assign, indexOf wrapper |
| `compiler/main.ajb` | Backend controller, fallback, indexOf decl, array decls |
| `compiler/emit_llvm.ajb` | String literal emission, indexOf, array declarations |

## Test Results
- **9/9 tests pass on LLVM backend** ✅
- **8/9 tests pass on C backend** (cross_simple has pre-existing var-decl bug)
- **Bootstrap check passes** ✅
- **Parth builds via LLVM** ✅
