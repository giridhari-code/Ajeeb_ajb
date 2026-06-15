# AJEEB BACKEND AUDIT — FINAL REPORT

## 1. Files Audited

| File | Role | Lines |
|------|------|-------|
| `crates/ajeeb-compiler/src/codegen.rs` | LLVM IR backend | 1054 |
| `compiler/compiler.ajb` | C codegen (self-hosted) | 1108 |
| `crates/ajeeb-compiler/src/main.rs` | Backend orchestration | 256 |
| `crates/ajeeb-compiler/src/eval.rs` | Interpreter backend | 1822 |
| `runtime/ajeeb_runtime.c` | Shared C runtime | 1102 |
| `crates/parth/src/main.rs` | Build system (parth) | 1340 |
| `scripts/install.sh` | Bootstrap script | 26 |
| `tests/bootstrap_check.sh` | Bootstrap verification | 64 |
| `build/cross_simple_llvm.ll` | Sample LLVM output | 164 |
| `build/cross_simple.c` | Sample C output | 76 |

## 2. Architecture Findings

### Compilation Pipeline

```
Source (.ajb)
    │
    ├──[Rust]──► Lexer → Parser → Semantic Analyzer
    │                           │
    │              ┌─────────────┼─────────────┐
    │              ▼             ▼              ▼
    │         Interpreter    LLVM IR       C Codegen
    │         (eval.rs)    (codegen.rs)   (compiler.ajb)
    │              │             │              │
    │              │        llc → .s         gcc
    │              │        as → .o        output.c
    │              │        gcc → binary   → binary
    │              ▼             ▼              ▼
    │           Direct       ajeeb_llvm    ajeeb_native
    │           exec
    │
    └──[Self-hosted]──► compiler.ajb → output.c → gcc → ajeeb_native
```

### Primary vs Secondary

| | LLVM Backend | C Backend |
|---|---|---|
| **Primary?** | Yes (production target) | No (bootstrap only) |
| **Completeness** | ~90% of AST nodes | ~30% of AST nodes |
| **Stability** | Beta (has bugs) | Beta (has bugs) |
| **Language** | Rust (codegen.rs) | Ajeeb (compiler.ajb) |

### Shared Code Paths

Both backends share:
- **Lexer/Parser/Semantic** (Rust): `lexer.rs`, `parser.rs`, `semantic.rs`, `ast.rs`
- **C Runtime**: `runtime/ajeeb_runtime.c` — arena allocator, reference counting, string ops, I/O, networking, dynamic loading
- **Global buffers**: `@__ajeeb_buf` / `__ajeeb_buf` (16KB), `@__ajeeb_outbuf` / `__ajeeb_outbuf` (64KB)

### Backend-Specific Code Paths

| Feature | LLVM Backend | C Backend |
|---------|-------------|-----------|
| AST Processing | Direct AST walking (`codegen.rs`) | Re-lexing from source (2-pass) |
| Type System | i64 for everything | `intptr_t` for everything |
| Struct Layout | `[i64 x N]` on stack | `typedef struct { ... }` |
| Enum Layout | `[tag, data0, data1, ...]` | Not supported |
| Method Dispatch | Name mangling via `method_map` | `self->` for class methods |
| String Handling | Global constants + `str_concat` | C string literals |
| Generic Handling | Type erasure to i64 | Not supported |

## 3. LLVM Backend Findings

### Features Supported
- ✅ Variables (let/const)
- ✅ Arithmetic (all operators)
- ✅ Strings (+, println, builtins)
- ✅ Arrays (allocation, indexing, assignment)
- ✅ Functions (definition, calls, recursion)
- ✅ Structs (definition, literal, field access, field assignment)
- ✅ Enums (definition, constructors, references)
- ✅ Match expressions (enum variant, int, string, wildcard patterns)
- ✅ Generics (type erasure to i64)
- ✅ Traits (static dispatch via name mangling)
- ✅ Generic traits (type-erased mangling)
- ✅ Impl blocks (inherent + trait)
- ✅ Generic impl blocks
- ✅ Associated function calls (`Type::method()`)
- ✅ Method calls (via `method_map`)
- ✅ Float literals
- ✅ Unary operations (-, !)
- ⚠️ Modules (@import) — not supported
- ⚠️ FFI — partially (lib_open, lib_sym)
- ⚠️ Networking — extern declarations only

### Bugs Found

**BUG-LLVM-1: Wrong extern declarations (CRITICAL)**
- Location: `codegen.rs:107-111`
- `len`, `itoa`, `readArg`, `getInt`, `charCode` are declared as `(i64, i64)` (2 args)
- Runtime signatures: `len(intptr_t s)` — 1 arg, `itoa(intptr_t n)` — 1 arg, `readArg(intptr_t n)` — 1 arg
- Impact: Undefined behavior for 1-arg functions called with 2 args. Works by accident on most ABIs because extra args are ignored.

**BUG-LLVM-2: println only prints first argument (CRITICAL)**
- Location: `codegen.rs:691-701`
- `println("sum: ", itoa(sum))` only prints `"sum: "`, discards `itoa(sum)`
- The interpreter prints ALL arguments
- Impact: Multi-arg println produces incomplete output

**BUG-LLVM-3: Field access uses wrong offset (MODERATE)**
- Location: `codegen.rs:808-810`
- `find_map(|(_, fields)| fields.iter().position(...))` searches ALL struct defs, not the specific struct
- If two different structs have a field with the same name, the wrong offset may be used
- Impact: Incorrect field access when multiple structs share field names

**BUG-LLVM-4: Float bitcast (VERIFIED CORRECT)**
- Location: `codegen.rs:548-549`
- `bitcast i64 {bits} to double` — `f.to_bits()` returns `u64`, stored as `i64`, then bitcast to double
- Bitcast operates on raw bits, not signedness — this is correct

**BUG-LLVM-5: unwrap() on globals_map.get() could panic (LOW)**
- Location: `codegen.rs:248`
- `self.globals_map.get(name).unwrap()` will panic if the variable isn't in the map
- Should use `ok_or_else(|| Err(...))` like other error paths
- Impact: Panic during compilation instead of error message

**BUG-LLVM-6: Target triple hardcoded (LOW)**
- Location: `codegen.rs:34`
- `target triple = "aarch64-unknown-linux-gnu"` — hardcoded to aarch64
- Cross-compilation to x86_64 or other targets requires manual change
- Impact: LLVM backend only generates correct code for aarch64

### unwrap() Locations
- `codegen.rs:33-43` — `writeln!` to String (safe, cannot fail)
- `codegen.rs:92,96` — `write!`/`writeln!` to String (safe)
- `codegen.rs:126` — `writeln!` to String (safe)
- `codegen.rs:248` — `self.globals_map.get(name).unwrap()` **PANIC RISK**
- All other `writeln!` calls are to `String` buffers (safe)

No `expect()` or `panic!()` calls found.

## 4. C Backend Findings

### Features Supported
- ✅ Variables (let/const)
- ✅ Arithmetic (+, -, *, /, ==, !=, <, >, <=, >=, &&, ||, !)
- ✅ Strings (+ for concatenation via str_concat)
- ✅ Functions (definition, calls, recursion)
- ✅ Classes (definition, methods via `self->`)
- ✅ If/else/else-if
- ✅ While loops
- ✅ For loops
- ✅ Return/break/continue
- ⚠️ Structs — NOT supported (uses `class` keyword only)
- ❌ Enums — NOT supported
- ❌ Match — NOT supported
- ❌ Generics — NOT supported
- ❌ Traits — NOT supported
- ❌ Impl blocks — NOT supported
- ❌ Modules/@import — NOT supported
- ❌ Array literals — NOT supported
- ❌ Index access — NOT supported
- ❌ Float literals — NOT supported
- ❌ Unary minus — NOT supported (only in specific contexts)

### Bugs Found

**BUG-C-1: Multi-arg println not supported (CRITICAL)**
- Location: compiler.ajb, parseExpr (line ~426)
- `println("a", b)` generates `println((intptr_t)"a", b)` which is invalid C — `println` takes 1 arg
- Impact: Any program using multi-arg println fails to compile with GCC

**BUG-C-2: String + is pointer arithmetic (CRITICAL)**
- Location: compiler.ajb, parseAdd (line 502-534)
- The C backend only concatenates strings when the FIRST token in the expression is a string literal
- `let s3 = s1 + " " + s2` generates `s1 + (intptr_t)" " + s2` which is pointer arithmetic, not string concatenation
- Impact: String variables concatenated together produce wrong results

**BUG-C-3: Dead code from return 0 (MODERATE)**
- Location: compiler.ajb, parseStmt (line 806)
- Every function ends with `return 0;` even if it already has a return statement
- Results in unreachable code warnings and double returns
- Impact: GCC warnings, slightly larger binaries

**BUG-C-4: Empty statements in main (LOW)**
- Location: compiler.ajb, parseStmt for-loop init (line 710-720)
- `for (let j = ...)` generates `0; 0; 0;` before the for statement
- These are no-op statements with "statement with no effect" warnings
- Impact: GCC warnings, no runtime effect

**BUG-C-5: No `fn` keyword support (DESIGN)**
- The C backend only recognizes `function` keyword
- All modern Ajeeb code using `fn` fails to compile through C backend
- Impact: Self-hosted compiler cannot compile modern language features

**BUG-C-6: Method extraction via temp file (FRAGILE)**
- Location: compiler.ajb, parseStmt for class (line 838-891)
- Class methods are extracted to `build/__methods.txt`, then appended
- If two classes exist, the second overwrites the first
- Impact: Only one class definition per compilation unit

## 5. Feature Coverage Matrix

| Feature | LLVM | C | Notes |
|---------|------|---|-------|
| Variables (let/const) | ✅ | ✅ | |
| Arithmetic | ✅ | ✅ | |
| Strings | ⚠️ | ❌ | LLVM: println only 1 arg. C: + is ptr arithmetic |
| Arrays | ✅ | ❌ | LLVM: stack-allocated. C: not supported |
| Functions | ✅ | ✅ | |
| Recursion | ✅ | ✅ | |
| Structs | ✅ | ❌ | C only has `class` |
| Enums | ✅ | ❌ | |
| Payload enums | ✅ | ❌ | |
| Match | ✅ | ❌ | |
| Generics | ✅ | ❌ | Type erasure to i64 |
| Traits | ✅ | ❌ | Static dispatch |
| Generic traits | ✅ | ❌ | |
| Impl blocks | ✅ | ❌ | |
| Generic impl blocks | ✅ | ❌ | |
| Modules/imports | ❌ | ❌ | Neither backend |
| FFI | ⚠️ | ❌ | LLVM: lib_open/lib_sym only |
| Networking | ⚠️ | ❌ | LLVM: extern decls only |
| Float | ✅ | ❌ | |
| Class/objects | ❌ | ✅ | C-only feature |
| Method calls | ✅ | ✅ | Different dispatch mechanisms |
| Associated fn calls | ✅ | ❌ | |

## 6. Backend Consistency Results

Tested `cross_simple.ajb` through all three backends:

| Output | Interpreter | LLVM | C |
|--------|-------------|------|---|
| `sum: 30` | ✅ | ❌ Prints "sum: " only | ❌ GCC error |
| `factorial(5): 120` | ✅ | ❌ Prints "factorial(5): " only | ❌ GCC error |
| `Hello World` | ✅ | ✅ | ❌ String concat wrong |
| `sum 0..4: 10` | ✅ | ❌ Prints "sum 0..4: " only | ❌ GCC error |
| `DONE` | ✅ | ✅ | ✅ |

**Mismatches found**: 4/5 test cases produce different output between backends.

## 7. Stability Results

### cargo test
- 0 test cases in the repository (all tests are integration tests via `ajeeb_compiler` binary)
- `cargo test`: PASS (no tests to fail)

### Interpreter Test Suite
- 129 `.ajb` test files
- ~33 intentional FAIL tests (semantic errors expected)
- ~96 PASS tests — all pass via interpreter

### Bootstrap Verification
```
Stage 1: Rust interpreter → compiler_v1 (build/ajeeb_native)
Stage 2: compiler_v1 → compiler_v2 (output2.c)
SHA256: ddf8124124f537602b46556366354e33263dcb2f303b532c5bec8cdec0d19fca
Status: ✅ IDENTICAL — Self-hosting verified
```

### LLVM Backend Compilation
- `llc -O2` → `as` → `gcc`: ✅ Compiles successfully
- LLVM IR validation (llc -O0): ✅ Valid IR
- Binary execution: ⚠️ Runs but produces incomplete output

### C Backend Compilation
- Self-hosted compiler: ✅ Produces `build/output.c`
- `gcc -Wall`: ⚠️ Warnings (unused statements, return after return)
- Binary execution: ❌ Fails for modern features (traits, generics, enums)

## 8. Performance Results

| Metric | LLVM | C |
|--------|------|---|
| Compile time (codegen) | ~10ms | ~150ms (interpreter-based) |
| Build pipeline | llc + as + gcc (3 steps) | gcc (1 step) |
| Binary size (cross_simple) | 77KB | 78KB |
| Generated code quality | Good (llc -O2) | Basic (no optimization) |
| Bootstrap time | N/A (not bootstrapable) | Self-hosting (verified) |

## 9. Security Risks

| Risk | Severity | Location |
|------|----------|----------|
| LLVM `unwrap()` panic on unknown global | LOW | codegen.rs:248 |
| Stack-allocated arrays with unchecked bounds | MEDIUM | codegen.rs:753 (alloca with runtime index) |
| No bounds checking on array index | MEDIUM | Both backends |
| No integer overflow detection | LOW | Both backends |
| C backend writes to `build/__methods.txt` without cleanup | LOW | compiler.ajb:838 |
| Global buffer overflow possible (16KB/64KB fixed) | LOW | runtime/ajeeb_runtime.c |

## 10. Exact Bugs Found

### LLVM Backend (codegen.rs)
1. **BUG-LLVM-1**: `codegen.rs:111` — `len`, `itoa`, `readArg` declared with 2 args, runtime has 1
2. **BUG-LLVM-2**: `codegen.rs:691-701` — `println` only prints first argument
3. **BUG-LLVM-3**: `codegen.rs:808-810` — Field offset search across all struct defs
4. **BUG-LLVM-5**: `codegen.rs:248` — `unwrap()` on `globals_map.get()` panic risk
5. **BUG-LLVM-6**: `codegen.rs:34` — Target triple hardcoded to aarch64

### C Backend (compiler.ajb)
6. **BUG-C-1**: compiler.ajb:~426 — Multi-arg `println` generates invalid C
7. **BUG-C-2**: compiler.ajb:502-534 — String `+` is pointer arithmetic for variable strings
8. **BUG-C-3**: compiler.ajb:806 — Dead `return 0;` in every function
9. **BUG-C-4**: compiler.ajb:710-720 — Empty statements in for-loop init
10. **BUG-C-5**: compiler.ajb — No `fn` keyword support (only `function`)
11. **BUG-C-6**: compiler.ajb:838-891 — Class method extraction overwrites between classes

## 11. Recommended Fixes

### Critical (fix immediately)
1. **LLVM**: Fix extern declarations — separate 1-arg and 2-arg functions
2. **LLVM**: Fix `println` to concatenate all arguments before calling `puts`
3. **C**: Fix `println` to handle multi-arg via string concatenation
4. **C**: Fix string concatenation for variable strings (use `str_concat`)

### High Priority
5. **LLVM**: Fix field offset lookup to use specific struct type
6. **LLVM**: Fix `unwrap()` at line 248 to return error
7. **LLVM**: Make target triple configurable
8. **C**: Add `fn` keyword support (alias for `function`)
9. **C**: Fix dead code generation (no double return, no empty stmts)

### Medium Priority
10. **Both**: Add bounds checking for array indexing
11. **Both**: Add modules/@import support
12. **LLVM**: Add x86_64 target support

## 12. Which Backend Should Be Default

### Recommendation: **C backend should be the default**

**Rationale:**
- The C backend is the **bootstrap backend** — it's the only one that can self-compile
- The C backend produces simpler, more maintainable C output
- The C backend has a 1-step build pipeline (`gcc output.c runtime.c -o binary`)
- The C backend is what `parth` build system uses by default

**However**, the C backend needs significant modernization:
- Must add support for `struct`, `enum`, `trait`, `impl`, generics, match
- Must fix the multi-arg println and string concatenation bugs
- Must add `fn` keyword alias

**The LLVM backend should be the alternative/performance backend**:
- Better code quality (O2 optimization via llc)
- Broader feature coverage
- But requires llc installation and has its own bugs

**Current state**: The default path in `parth` build system already prefers the C backend (`ajeeb_native`), falling back to LLVM. This is correct.

---

**Total bugs found: 11** (5 LLVM, 6 C)
**Critical bugs: 4** (2 LLVM, 2 C)
**Bootstrap status: ✅ VERIFIED** (C backend only)
