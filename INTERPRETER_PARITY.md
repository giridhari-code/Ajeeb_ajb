# Interpreter Parity Audit — Ajeeb Compiler

**Date:** 2026-06-22
**Compiler binary:** `ajeebc/build/ajeeb_compiler` (LLVM backend: llc + as + ld)
**Interpreter mode:** `--interpret` flag (skips codegen, runs Evaluator directly)
**Native mode:** default / `--skip-run` (compiles to native binary, then runs it)

---

## Summary

| Category | Count |
|----------|-------|
| Tests with **identical output** in both modes | 14 |
| Tests **interpreter-only** (native build/runtime fails) | 11 |
| Tests that **fail in both modes** (not parity issues) | 7 |
| **Critical mismatches** (segfault / wrong output) | 2 |

**Overall parity: GOOD for core features** (if/else, while, for, functions, string ops, basic control flow). **Broken for structs, enums, traits, generics, and multi-arg println with non-string arguments.**

---

## Tests with Identical Output (Parity OK)

| Test File | Expected Output | Interpreter | Native | Status |
|-----------|----------------|-------------|--------|--------|
| test_simple | `Hello World` | ✓ | ✓ | MATCH |
| test_math | `42` | ✓ | ✓ | MATCH |
| test_if | `bada hai` | ✓ | ✓ | MATCH |
| test_while | `0\n1\n2` | ✓ | ✓ | MATCH |
| test_for | `0\n1\n2\n4\n5` | ✓ | ✓ | MATCH |
| test_strings | `Hello World\nHELLO\najeeb\n1\n1\nHello` | ✓ | ✓ | MATCH |
| test_fncall | `30` | ✓ | ✓ | MATCH |
| cross_simple | `sum: 30\nfactorial(5): 120\nHello World\nsum 0..4: 10\nDONE` | ✓ | ✓ | MATCH |
| test_echo | `Hello from Ajeeb!` | ✓ | ✓ | MATCH |
| test_while_simple | `0\n1\n2\n3\n4` | ✓ | ✓ | MATCH |
| test_while_bug | `0\n1\n2` | ✓ | ✓ | MATCH |
| test_while2 | `0\n1\n2` | ✓ | ✓ | MATCH |
| test_llvm_concat_only | `Hello World` | ✓ | ✓ | MATCH |
| llvm_feat_struct | `3\n4\n10` | ✓ | ✓ | MATCH |

---

## Critical Mismatches

### 1. SEGFAULT: `test_llvm_comprehensive` — Native binary crashes

- **Interpreter output (correct):** All 12 test sections print correctly.
- **Native output:** Segfault (exit code 139).
- **Root cause:** `println("String length ", result)` where `result` is an `int`. The LLVM codegen passes the raw integer to `str_concat(i64, i64)` which expects two string pointers. The interpreter auto-converts integers to strings via `itoa()` before concatenation. The codegen does **not** insert `itoa()` calls for non-string arguments to `str_concat`.
- **Impact:** Any `println` with mixed string+int arguments will segfault in native mode.
- **Codegen location:** `llvm/expr.rs:342-363` — the generic function call path passes args directly without type-based conversion.
- **C codegen has the same bug:** `c_codegen.rs:259-266` — multi-arg println calls `str_concat` without converting non-string args.
- **Severity: HIGH** — common pattern in user code.

### 2. WRONG OUTPUT: `struct_basic` — Native prints garbage value

- **Interpreter output:** `Ajeeb`
- **Native output:** `206158457604` (garbage pointer dereference)
- **Root cause:** The C codegen emits `__struct_User(...)` and `__struct_get__name(self)` but the C runtime has no implementations for these helper functions. The compiler resolves struct operations natively in the interpreter but generates undefined function calls in C code.
- **Severity: HIGH** — structs are unusable in native mode.

---

## Interpreter-Only Tests (Native Build/Runtime Fails)

### Category A: C Codegen Missing Runtime Functions

| Test | Interpreter | Native Failure |
|------|-------------|----------------|
| test_array | `10\n99\n30` | Missing `__array_lit`, `__index`, `__index_assign` |
| inherent_basic | `Hello from Ajmal\nPASS` | Missing `__struct_get__name`, implicit function declarations |
| llvm_feat_method | `25` | Missing `__struct_Point`, `__struct_get__x/y` |
| llvm_feat_generic | `42\nhello` | Missing `__struct_Box[String]_value` — generic struct names illegal in C |
| llvm_feat_trait | `Ajmal` | Missing `User_Printable_format`, `__struct_get__name` |
| llvm_feat_enum | `Red\n42` | Missing `Option_Some` — enum constructors undefined |
| enum_payload | (no output) | Missing `Result_Ok`, `Result_Err` |
| regression_fixes | multi-line PASS | Missing `__array_lit`, undeclared variables, str_concat with int |
| struct_verify | — | Missing `__struct_*` functions |

**Pattern:** The interpreter handles structs, arrays, enums, and traits as first-class runtime values. The C codegen emits calls to `__struct_*`, `__array_lit`, `__index`, `__index_assign`, `Result_Ok`, etc., but the C runtime (`runtime/ajeeb_runtime.c`) has no implementations for them.

### Category B: LLVM Codegen Bugs

| Test | Interpreter | Native Failure |
|------|-------------|----------------|
| enum_basic | (no output) | LLVM IR syntax error: `store i64 %Color::Color` — `::` invalid in LLVM identifier |
| trait_basic | `Ajmal` | Undefined value `@exit` — missing `declare void @exit(i32)` |
| test_traits | `Ajmal` | Undefined value `@exit` — same issue |
| test_llvm_comprehensive | full output | Segfault (see Critical Mismatch #1) |

---

## Tests Failing in Both Modes (Not Parity Issues)

| Test | Failure |
|------|---------|
| test_ops | Lexer error: `%` character not supported |
| test_generics | Type mismatch: `expected Box<Int>, got Box[Int]` |
| test_option | Module resolution: `Module 'option' not found` |
| test_result | Module resolution: `Module 'result' not found` |
| test_exhaustive | Module resolution: `Module 'option' not found` |
| test_stdlib | Module resolution: `Module 'std::string' not found` |
| compiler_test | Interpreter hangs; native linker error (multiple definition of `chr`) |
| string_corruption_test | Native: linker error (multiple definition of `chr`) |

---

## Root Cause Analysis

### 1. `str_concat` with Non-String Arguments (SEGFAULT)

**Both codegen paths** call `str_concat(a, b)` directly with raw integer arguments. The C runtime's `str_concat` expects two valid string pointers. When an integer is passed, it's treated as a pointer address, causing undefined behavior or segfault.

**Fix needed:** Before calling `str_concat`, check if each argument is a string. If not, wrap it in `itoa()`. This must happen in:
- `llvm/expr.rs:342-363` (generic function call)
- `llvm/stmt.rs` (multi-arg println handling via MIR)
- `c_codegen.rs:259-266` (multi-arg println)

### 2. Missing C Runtime Functions for High-Level Constructs

The C codegen emits calls to helper functions that don't exist in `ajeeb_runtime.c`:

| Missing Function | Purpose |
|-----------------|---------|
| `__struct_User(...)` | Struct constructor |
| `__struct_get__name(self)` | Struct field accessor |
| `__array_lit(...)` | Array literal constructor |
| `__index(arr, i)` | Array index read |
| `__index_assign(arr, i, v)` | Array index write |
| `Result_Ok(v)` | Enum variant constructor |
| `Option_Some(v)` | Enum variant constructor |

**Options:**
1. Implement these as C runtime functions (complex, type-dependent)
2. Lower these to simpler primitives in MIR before C codegen
3. Use the LLVM backend for struct/enum/array features (already partially working)

### 3. LLVM Enum Variant Storage

`store i64 %Color::Color, ptr %1` — the `::` operator produces invalid LLVM IR identifiers. The codegen needs to sanitize enum variant names (e.g., `Color.Red` → `Color_dot_Red` or use numeric indices).

### 4. Missing `exit` Declaration

LLVM codegen uses `call void @exit(i32 1)` in assertion failures but never declares `@exit`. Fix: add `declare void @exit(i32)` to the LLVM IR preamble.

### 5. Multiple Definition of `chr`

Both `ajeeb_runtime.c` and the generated code define `chr()`. The codegen should not emit a `chr` function definition if it already exists in the runtime. Alternatively, the runtime should not define `chr` and let the codegen handle it.

---

## Recommendations

### Priority 1: Fix `str_concat` type coercion (affects all codegen)

Both LLVM and C codegen must convert non-string arguments to strings before passing to `str_concat`. This is the most impactful fix — it unblocks `println` with mixed arguments.

### Priority 2: Implement struct/array/enum runtime support in C backend

Either:
- Add `__struct_*`, `__array_lit`, `__index`, `__index_assign` to `ajeeb_runtime.c`
- Or restrict struct/array/enum features to LLVM backend only (currently the default for features like `llvm_feat_struct`)

### Priority 3: Fix LLVM enum variant codegen

Sanitize enum variant names and use numeric indices for storage instead of string-encoded variant names.

### Priority 4: Add `declare void @exit(i32)` to LLVM preamble

Simple one-line fix in `llvm/mod.rs`.

### Priority 5: Resolve `chr` multiple definition

Either remove `chr` from `ajeeb_runtime.c` or suppress `chr` emission in codegen.

### Priority 6: Fix module resolution

`import option;`, `import result;`, `import std::string;` fail in both modes. These modules need to be placed in the correct search path or bundled with the compiler.

---

## Test Methodology

1. **Interpreter mode:** `./build/ajeeb_compiler tests/<file>.ajb --interpret`
2. **Native mode:** `./build/ajeeb_compiler tests/<file>.ajb --skip-run` (compiles), then `timeout 5 ./build/<file>` (runs)
3. **Output comparison:** Direct string comparison of stdout between both modes
4. **Test files:** All `.ajb` files in `ajeebc/tests/` that successfully pass the interpreter
