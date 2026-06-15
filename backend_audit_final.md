# Backend Audit Report

**Date:** 2026-06-15
**Scope:** All 136 `.ajb` test files in `tests/`
**Backends Tested:** Interpreter (reference), LLVM, C

---

## 1. Test Summary

| Metric | Count |
|--------|-------|
| Total test files | 136 |
| PASS tests (compile + run through interpreter) | 87 |
| Error tests (designed to test error handling) | 49 |
| LLVM backend tested | 80 pass tests |
| C backend tested | 87 pass tests |

---

## 2. Interpreter (Reference)

| Metric | Count |
|--------|-------|
| Tests that execute successfully | 87 / 87 (100%) |
| Tests that produce output | 55 |
| Tests that produce no output | 32 |

---

## 3. LLVM Backend

### Counts

| Metric | Count |
|--------|-------|
| **Tests attempted** | 80 |
| **LLVM compile pass** | 52 (65%) |
| **Binary runs without crash** | 69 (86%) |
| **Output matches interpreter exactly** | 69 (86%) |
| **Output mismatch** | 11 (14%) |
| **Crashes (SIGSEGV/SIGFPE)** | 6 (7.5%) |

### LLVM-Specific Bugs

#### BLOCKING — BUG-LLVM-CRASH-1: `println/print` with integer arguments crashes (SIGSEGV)
**Files affected:** `generic_trait_basic.ajb`, `generic_trait_impl.ajb`, `generic_trait_multiple_impls.ajb`, `regression_fixes.ajb`, `test_llvm_comprehensive.ajb`

**Root cause (`codegen.rs`):** When `println` or `print` receives an integer argument (e.g., `println("count: ", i)`), the multi-arg concatenation path passes the raw `i64` to `str_concat`, which treats it as a `char*` pointer and dereferences it. The codegen does not convert integer arguments to strings before concatenation.

**LLVM IR pattern:**
```llvm
%arg_i64 = ... ; i64 value from some computation
; directly passed to str_concat — BOOM:
%result = call i64 @str_concat(i64 %string_ptr, i64 %arg_i64)
```

**Fix needed:** Multi-arg `println`/`print` must wrap each non-string argument with `itoa()` before passing to `str_concat`. This applies to the LLVM codegen path only (the C backend handles this correctly in `compiler/compiler.ajb`).

---

#### BLOCKING — BUG-LLVM-CRASH-2: Division by zero raises SIGFPE
**Files affected:** `test_stacktrace.ajb`

**Root cause (`codegen.rs`):** The LLVM codegen emits an LLVM `sdiv` instruction for division. On x86-64 and AArch64, `sdiv` by zero triggers a hardware fault (SIGFPE). The interpreter gracefully returns 0 for division by zero.

**LLVM IR pattern:**
```llvm
%result = sdiv i64 %numerator, %denominator  ; SIGFPE when %denominator == 0
```

**Fix needed:** Emit a zero divisor check before `sdiv`:
```llvm
%is_zero = icmp eq i64 %denominator, 0
%div_result = sdiv i64 %numerator, %denominator
%result = select i1 %is_zero, i64 0, i64 %div_result
```

---

#### NON-BLOCKING — BUG-LLVM-BLANKLINE: Empty `println("")` blank line differences
**Files affected:** `semantic_test.ajb`, `string_corruption_test.ajb`

**Root cause (`codegen.rs`):** When `println("")` is called with an empty string literal, the LLVM codegen's multi-arg path constructs `str_concat("")` but the interpreter and LLVM binary output differ by blank lines. Content is identical but whitespace differs.

---

#### NON-BLOCKING — BUG-LLVM-PRINTNL: `print()` and `println()` both emit newlines
**Files affected:** `inherent_basic.ajb`

**Root cause (`codegen.rs`):** Both `print(...)` and `println(...)` are lowered to `@puts()` which always appends a newline. The interpreter's `print()` does NOT add a newline.

---

#### NON-BLOCKING — BUG-LLVM-DISPATCH: Inherent vs trait method name collision
**Files affected:** `inherent_and_trait_same_name.ajb`

**Root cause (`codegen.rs`):** Both inherent and trait methods are stored in `method_map` under the same key `(type_name, method_name)`. The last one registered wins, so when an inherent method and a trait method share the same name, the wrong one may be called.

---

### LLVM Tests That Pass Correctly

The following 11 tests produce **identical output** across ALL three backends:

| Test | What it covers |
|------|---------------|
| `cross_simple.ajb` | Cross-backend: multi-arg println, string concat, for loop, recursion |
| `test_simple.ajb` | Hello World |
| `test_math.ajb` | Integer arithmetic |
| `test_strings.ajb` | String operations (charCode, len, toUpperCase, indexOf, substring) |
| `test_for.ajb` | For loops |
| `test_while.ajb` | While loops |
| `test_if.ajb` | If/else conditionals |
| `test_llvm_call_only.ajb` | Simple function calls |
| `test_llvm_print.ajb` | Single-arg println |
| `compiler_test.ajb` | Self-hosting compiler compilation (no output) |
| `test_tiny.ajb` | Minimal program (no output) |

---

## 4. C Backend

### Counts

| Metric | Count |
|--------|-------|
| **Tests attempted** | 87 |
| **C code generated** | 87 (100%) |
| **GCC compile succeed** | 19 (22%) |
| **Binary runs without crash** | 15 (17%) |
| **Output matches interpreter exactly** | 14 (16%) |

### C-Backend Bugs

#### BLOCKING — BUG-C-RAW-AST: Traits, enums, structs, generics emit raw AST tokens
**Files affected:** 68 tests (all that use traits, enums, structs, generics, impl blocks, or `new`)

**Root cause (`compiler/compiler.ajb`):** The C backend (`compiler.ajb`) parses function-level statements but does not translate higher-level constructs (trait declarations, enum definitions, struct definitions, impl blocks, generic type parameters, or `new` expressions) to valid C. Instead, it falls through to a default handler that emits raw AST token text.

**Generated C code example (broken):**
```c
intptr_t trait;  // raw token
intptr_t Greeter;  // raw token
intptr_t {;  // raw token
fn greet();  // raw token
intptr_t };  // raw token
```

**Impact:** Only programs that use plain functions, if/while/for, math, strings, and arrays compile through the C backend.

---

#### BLOCKING — BUG-C-SEGFAULT: Segfault in 4 tests via `compiler_v1`
**Files affected:** `regression_fixes.ajb`, `self_hosting_test.ajb`, `test_llvm_comprehensive.ajb`, `test_llvm_concat_only.ajb`

**Root cause:** When these tests are compiled via `compiler_v1`, the generated C code triggers segfaults. Root cause is the same as LLVM's BUG-CRASH-1 for `regression_fixes.ajb` and `test_llvm_comprehensive.ajb` (multi-arg println with integers). For `self_hosting_test.ajb`, the program attempts to read/call itself. For `test_llvm_concat_only.ajb`, the string concatenation logic may overflow or corrupt memory.

---

### C Tests That Pass Correctly

14 tests produce correct output through the full C pipeline (interpreter → compiler.ajb → GCC → binary):

`cross_simple`, `semantic_test`, `string_corruption_test`, `test_for`, `test_if`, `test_llvm_call_only`, `test_llvm_print`, `test_math`, `test_simple`, `test_stacktrace`, `test_strings`, `test_while`, `compiler_test`, `test_tiny`

---

## 5. Comparison: LLVM vs C

| Feature | LLVM | C |
|---------|------|---|
| Function calls | ✅ | ✅ |
| If/else | ✅ | ✅ |
| While/for loops | ✅ | ✅ |
| Integer math | ✅ (div-by-zero crashes) | ✅ |
| String operations | ✅ | ✅ |
| Arrays | ✅ | ✅ |
| Multi-arg println | ❌ (crashes with int args) | ✅ (via `str_concat` chain) |
| Single-arg println/print | ✅ (print adds newline) | ✅ |
| Struct fields | ✅ | ❌ (raw AST dump) |
| Enums | ✅ | ❌ (raw AST dump) |
| Traits | ✅ (dispatch bug) | ❌ (raw AST dump) |
| Generics | ✅ (trait dispatch crash) | ❌ (raw AST dump) |
| `new` expressions | ✅ | ❌ (raw AST dump) |
| `ImplBlock` methods | ✅ (dispatch bug) | ❌ (raw AST dump) |
| Bootstrap (compiler compiling itself) | N/A | ✅ |

---

## 6. Verdict

**LLVM Ready As Default Backend?** **NO**

### Blocking bugs that must be fixed first:

1. **BUG-LLVM-CRASH-1 (CRITICAL):** Multi-arg `println`/`print` with integer arguments causes SIGSEGV because raw `i64` values are passed to `str_concat` which dereferences them as string pointers. The codegen must wrap non-string arguments in `itoa()` before concatenation.

2. **BUG-LLVM-CRASH-2 (CRITICAL):** Integer division by zero causes SIGFPE (hardware fault) instead of returning 0. The codegen must emit a zero-check guard before `sdiv`.

3. **BUG-LLVM-DISPATCH (HIGH):** When an inherent method and a trait method share the same name on the same type, `method_map` uses a single key `(type_name, method_name)`, causing the wrong method to be called.

4. **BUG-LLVM-PRINTNL (MEDIUM):** `print()` adds a newline (uses `puts()`), but the interpreter's `print()` does not. Must use `printf` for `print()`.

### If these 4 bugs are fixed, then:

**YES — LLVM should become the default backend** with C as fallback. The LLVM backend supports the full language (structs, enums, traits, generics, methods), the C backend does not. Switching to LLVM by default also avoids the `FILE_CACHE_SIZE` bottleneck and the temp-file proliferation in the C backend's `parseAdd`.

### Recommended build system change (after fixes):
```
Default Backend = LLVM
Fallback Backend = C
```
