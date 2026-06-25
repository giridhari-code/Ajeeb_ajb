# PURE_AJEEB_GAP_ANALYSIS.md

**Date:** 2026-06-22
**Status:** EVIDENCE COMPLETE — DO NOT MODIFY CODE
**Goal:** Determine exact readiness for Pure-Ajeeb Stage A

---

## Executive Summary

Two Ajeeb compilers exist:

| Compiler | LOC | Pipeline | Self-Hosts? |
|----------|-----|----------|-------------|
| `compiler.ajb` (6 files) | 1,248 | Lex → Parse → C codegen (transpiler) | **YES** — compiles `main.ajb` |
| `main.ajb` (1 file) | 1,627 | Lex → Parse → HIR → MIR → Optimize → C codegen | **NO** — missing 6 features |

**The bootstrap chain currently uses the Rust compiler**, not `main.ajb`. The Rust compiler compiles `compiler.ajb` → native binary via MIR→LLVM pipeline. This binary can then compile `main.ajb`.

**Critical blocker:** `main.ajb` cannot compile `compiler.ajb` because `compiler.ajb` uses `import`, `class`, `self`, `new`, `true`/`false`, and `const` — none of which `main.ajb` handles.

---

## 1. Current Bootstrap Chain (Verified)

```
Step 1: Rust compiler (cargo build) → build/ajeeb_compiler
Step 2: build/ajeeb_compiler compiles compiler/compiler.ajb → build/compiler (native binary)
Step 3: build/compiler compiles compiler/compiler.ajb → build/output2.c
Step 4: diff + sha256sum verify output.c ≡ output2.c
```

**Status:** Works via Rust. Does NOT work via pure Ajeeb.

---

## 2. Lexer Parity

### Rust Compiler Lexer (349 LOC — `lexer.rs`)

| Feature | Rust | main.ajb | compiler.ajb | Gap |
|---------|------|----------|--------------|-----|
| 64 token types | ✅ | 48 tokens | 48 tokens | main.ajb missing: `struct`, `enum`, `match`, `trait`, `impl`, `::`, `@`, `=>`, `_`, `pub`, `float`, `char`, `byte`, `lifetime`, `interpolated string`, `range` |
| String interpolation `{expr}` | ✅ | ❌ | ❌ | P2 |
| Escape sequences `\xHH`, `\u{HHHHHH}` | ✅ | ❌ | ❌ (basic `\n\t\"\\0` only) | P2 |
| Number formats (hex `0x`, binary `0b`, octal `0o`) | ✅ | ❌ | ❌ | P2 |
| Float literals | ✅ | ❌ | ❌ | P1 |
| `pub`/`pri`/`prot` visibility | ✅ | ❌ | ❌ | P2 |
| `struct`/`enum`/`trait`/`impl` keywords | ✅ | ❌ | ❌ | P1 |
| `match` keyword | ✅ | ❌ | ❌ | P1 |
| `::` (double colon) | ✅ | ❌ | ❌ | P1 |
| `=>` (fat arrow) | ✅ | ❌ | ❌ | P1 |
| `_` (underscore) | ✅ | ❌ | ❌ | P1 |
| `@import` syntax | ✅ | ❌ | ❌ (uses `import name;`) | P2 |
| Compound assignment (`+=`, `-=`, `*=`, `/=`) | ✅ | ✅ | ❌ | main.ajb has, compiler.ajb missing |
| `++`/`--` | ✅ | ✅ | ❌ | main.ajb has, compiler.ajb missing |
| `**` (power) | ✅ | ✅ | ❌ | main.ajb has, compiler.ajb missing |
| `%` (modulo) | ✅ | ✅ | ❌ | main.ajb has, compiler.ajb missing |

**Lexer parity: ~60%** (main.ajb), **~50%** (compiler.ajb)

---

## 3. Parser Parity

### Rust Compiler Parser (1,976 LOC — `parser/` directory)

| Feature | Rust | main.ajb | compiler.ajb | Gap |
|---------|------|----------|--------------|-----|
| Functions (generic, trait bounds, default params) | ✅ | ✅ (no generics) | ✅ (no generics) | P1: generics |
| `set` / `const` declarations | ✅ | ✅ `set` only | ✅ both | main.ajb missing `const` |
| `if`/`else if`/`else` | ✅ | ✅ | ✅ | — |
| `while` loops | ✅ | ✅ | ✅ | — |
| `for` loops (C-style) | ✅ | ✅ | ✅ | — |
| `for`-`in` loops | ✅ | ❌ | ❌ | P2 |
| `return` / `break` / `continue` | ✅ | ✅ | ✅ | — |
| `class` declarations | ✅ | ❌ | ✅ | main.ajb missing |
| `struct` declarations | ✅ | ❌ | ❌ | P1 |
| `enum` declarations | ✅ | ❌ | ❌ | P1 |
| `trait` declarations | ✅ | ❌ | ❌ | P2 |
| `impl` blocks | ✅ | ❌ | ❌ | P2 |
| `import` / `@import` | ✅ | ❌ | ✅ (`import name;`) | main.ajb missing |
| `match` expressions | ✅ | ❌ | ❌ | P1 |
| Closures / lambda `\|params\| => body` | ✅ | ❌ | ❌ | P2 |
| Generic params `[T]` | ✅ | ❌ | ❌ | P1 |
| Type arg lists `[Int, String]` | ✅ | ❌ | ❌ | P1 |
| Pattern matching | ✅ | ❌ | ❌ | P2 |
| `self` / `&self` / `&mut self` | ✅ | ❌ | ✅ (`self` only) | main.ajb missing |
| `new ClassName()` | ✅ | ❌ | ✅ | main.ajb missing |
| `true` / `false` literals | ✅ | ❌ | ✅ | main.ajb missing |
| Array literals `[1, 2, 3]` | ✅ | ✅ | ✅ | — |
| Field access `obj.field` | ✅ | ✅ | ✅ | — |
| Method calls `obj.method()` | ✅ | ✅ | ✅ | — |
| Array indexing `arr[i]` | ✅ | ✅ | ✅ | — |
| `as` cast | ✅ | ❌ | ❌ | P2 |
| `unsafe` blocks | ✅ | ❌ | ❌ | P3 |
| `defer` statements | ✅ | ❌ | ❌ | P3 |
| Range expressions `..` / `..=` | ✅ | ❌ | ❌ | P2 |
| String interpolation in expressions | ✅ | ❌ | ❌ | P2 |
| `pub`/`pri`/`prot` visibility | ✅ | ❌ | ❌ | P2 |
| Multi-file `import name;` resolution | ✅ (`@import`) | ❌ | ✅ | main.ajb missing |
| Operator precedence (full Pratt) | ✅ | ✅ (15 levels) | ✅ (8 levels) | — |

**Parser parity: ~45%** (main.ajb), **~40%** (compiler.ajb)

---

## 4. Semantic Analysis Parity

### Rust Compiler (1,679 LOC — `semantic/` directory)

| Feature | Rust | main.ajb | compiler.ajb | Gap |
|---------|------|----------|--------------|-----|
| Duplicate definition detection | ✅ | ❌ | ❌ | P1 |
| Unknown type in field check | ✅ | ❌ | ❌ | P1 |
| Impl trait method validation | ✅ | ❌ | ❌ | P2 |
| Generic type arg count validation | ✅ | ❌ | ❌ | P1 |
| Generic bound satisfaction | ✅ | ❌ | ❌ | P2 |
| Trait impl lookup | ✅ | ❌ | ❌ | P2 |
| Type substitution | ✅ | ❌ | ❌ | P2 |
| Scope-based variable lookup | ✅ | ❌ | ❌ | P1 |
| Method dispatch (inherent → trait → generic) | ✅ | ❌ | ❌ | P2 |
| Lambda type inference | ✅ | ❌ | ❌ | P2 |
| Import function registration | ✅ | ❌ | ❌ | P1 |
| Return type validation | ✅ | ❌ | ❌ | P1 |
| Binary op type compatibility | ✅ | ❌ | ❌ | P1 |
| Unused variable warnings | ✅ | ❌ | ❌ | P3 |
| Missing return detection | ✅ | ❌ | ❌ | P2 |

**Semantic parity: 0%** — entirely missing from both Ajeeb compilers.

---

## 5. HIR Parity

### Rust Compiler HIR (282 LOC — `hir.rs`)

| Feature | Rust | main.ajb | Gap |
|---------|------|----------|-----|
| HirType: Int, Bool, Str, Void | ✅ | ✅ | — |
| HirType: Float, Named, Array, Generic, Fn | ✅ | ❌ | P1 |
| HirExpr: 20 variants | ✅ | 12/20 | 8 missing |
| HirStmt: 12 variants | ✅ | 8/12 | 4 missing |
| HirFn, HirStruct, HirEnum, HirImpl, HirTrait | ✅ | HirFn only | P1 |
| HirProgram, HirItem | ✅ | HirProgram only | P1 |

**HIR parity: ~60%**

---

## 6. MIR Parity

### Rust Compiler MIR (320 LOC — `mir.rs`)

| Feature | Rust | main.ajb | Gap |
|---------|------|----------|-----|
| MirInstr: 35+ opcodes | ✅ | 16 opcodes | 19 missing |
| MirTerminator: 6 variants | ✅ | 3 (Goto, Branch, Return) | 3 missing (Unreachable, Drop, Defer) |
| Constant folding | ✅ | ✅ | — |
| Dead code elimination | ✅ | ✅ | — |
| Dead block elimination | ✅ | ✅ | — |
| **Extra in main.ajb:** `%` (modulo), `**` (power) | — | ✅ | Ajeeb extras |

Missing MIR opcodes in main.ajb:
- FloatLit, StringLit, BoolLit (only IntLit)
- Unary (only Binop)
- Cast
- FieldAccess, MethodCall
- ClosureCreate, ClosureCapture, ClosureCall
- ArrayInit, ArrayIndex, ArrayLen
- TupleInit, TupleField
- MapInit, MapGet
- MakeOption, UnwrapOption, IsSome, IsNone
- MakeResult, UnwrapResult, IsErr
- EnumConstruct, EnumField, IsEnumVariant
- Phi (for loop-carried values)
- Drop, Defer

**MIR parity: ~45%**

---

## 7. LLVM Backend Parity

### Rust Compiler LLVM (2,640 LOC — `llvm/` directory)

| Feature | Rust | main.ajb | Gap |
|---------|------|----------|-----|
| All LLVM codegen | ✅ | ❌ | P1 (entirely missing) |
| String `==` content comparison | ✅ | N/A | Known bug: LLVM uses pointer cmp |
| 40+ extern declarations | ✅ | N/A | — |
| Generic monomorphization | ✅ | N/A | — |
| Closures | ✅ (partial) | N/A | — |
| Float ops (bitcast i64↔double) | ✅ | N/A | — |

**LLVM parity: 0%** — entirely missing from Ajeeb compilers.

---

## 8. Interpreter Parity

### Rust Compiler Interpreter (1,897 LOC — `eval/` directory)

| Feature | Rust | main.ajb | Gap |
|---------|------|----------|-----|
| 65 builtin functions | ✅ | 0 | P2 |
| RuntimeValue: 11 variants | ✅ | N/A | — |
| Scope management | ✅ | N/A | — |
| Pattern matching | ✅ | N/A | — |
| Class/struct/enum instantiation | ✅ | N/A | — |
| Closures (stub) | ✅ (stub) | N/A | — |
| Lambda (stub) | ✅ (stub) | N/A | — |

**Interpreter parity: 0%** — entirely missing from Ajeeb compilers.

---

## 9. Runtime Parity

### C Runtime (1,452 LOC — `ajeeb_runtime.c`)

| Category | Rust Builtins | Runtime API | main.ajb declares | compiler.ajb declares | Gap |
|----------|--------------|-------------|-------------------|----------------------|-----|
| Memory | 5 | 27 | ✅ | ✅ | — |
| String | 15 | 26 | 12 | 12 | Missing: `split`, `replace_all`, `parseInt`, `parseFloat` |
| I/O | 8 | 13 | ✅ | ✅ | — |
| Network | 10 | 16 | ❌ | ❌ | P3 |
| Array | 3 | 3 | ✅ | ❌ | compiler.ajb missing `arr_len`, `__array_lit`, `array_to_string` |
| System | 2 | 2 | ✅ | ❌ | compiler.ajb missing `exec`, `mkdir` |
| FFI | 3 | 3 | ❌ | ❌ | P3 |
| DB | 6 | 6 | ❌ | ❌ | P3 |
| Time | 1 | 1 | ❌ | ❌ | P3 |

**Runtime parity: ~70%** (main.ajb), **~60%** (compiler.ajb)

---

## 10. Package Manager Parity

### Parth (Rust, 4,995 LOC)

| Feature | Rust Parth | Ajeeb Parth | Gap |
|---------|-----------|-------------|-----|
| Commands | 37 | 7 | 30 missing |
| Config parser (parth.das) | ✅ | ✅ | — |
| Lock file | ✅ | ✅ | — |
| Dependency resolver | ✅ (PubGrub) | ✅ (basic) | P2 |
| Registry (HTTP) | ✅ | ❌ | P2 |
| Package signing (Ed25519) | ✅ | ❌ | P2 |
| Security audit | ✅ | ❌ | P3 |
| FFI (dlopen) | ✅ | ❌ | P3 |
| Cache management | ✅ | ❌ | P2 |
| Workspace support | ✅ | ❌ | P3 |

**Package manager parity: ~20%**

---

## 11. Critical Self-Hosting Blockers

These features MUST be added to `main.ajb` before it can compile `compiler.ajb`:

| # | Feature | Where Used in compiler.ajb | In main.ajb? | LOC to Add | Dependency |
|---|---------|---------------------------|-------------|-----------|------------|
| 1 | `import name;` statement | `compiler.ajb:28-32`, `stmt.ajb:316-355`, `pass1.ajb:109-141` | **NO** | 200-300 | None (can start here) |
| 2 | `class Name { }` declaration | `stmt.ajb:252-315` | **NO** | 150-200 | None |
| 3 | `self` keyword handling | `expr.ajb:41-89` | **NO** | 50-80 | Requires `class` |
| 4 | `new ClassName()` expression | `expr.ajb:91-96` | **NO** | 20-30 | Requires `class` |
| 5 | `true` / `false` literals | `expr.ajb:39-40` | **NO** | 15-20 | None |
| 6 | `const` declaration | `stmt.ajb:56-74` | **NO** | 20-30 | None |

**Total LOC to add: ~455-660**

Without these 6 features, `main.ajb` cannot compile any code that uses `import`, `class`, `self`, `new`, `true`/`false`, or `const`. Since `compiler.ajb` uses ALL of these, self-hosting is impossible.

---

## 12. Full Feature Gap Matrix (Rust vs Ajeeb)

### Per-File Inventory

| Subsystem | Rust LOC | main.ajb LOC | compiler.ajb LOC | Parity |
|-----------|----------|-------------|------------------|--------|
| Lexer | 349 | 120 (embedded) | 227 | 55% |
| Parser | 1,976 | 350 (embedded) | 535 | 35% |
| AST/HIR | 282 + 335 = 617 | 100 (embedded) | 0 | 20% |
| THIR | 380 | 0 | 0 | 0% |
| MIR | 320 | 200 (embedded) | 0 | 50% |
| Semantic | 1,679 | 0 | 0 | 0% |
| LLVM | 2,640 | 0 | 0 | 0% |
| C Codegen | 417 | 250 (embedded) | 100 | 55% |
| Interpreter | 1,897 | 0 | 0 | 0% |
| Module System | 257 | 0 | 50 (inline) | 15% |
| Cache | 1,094 | 0 | 0 | 0% |
| **TOTAL** | **~10,010** | **~1,020** | **~912** | **~20%** |

---

## 13. Missing Work Ranked by ROI

### P0 — Required for Self-Hosting (455-660 LOC)

| # | Feature | Source | LOC | Bootstrap Impact |
|---|---------|--------|-----|-----------------|
| 1 | `import name;` statement | main.ajb:parseStmt | 200-300 | **BLOCKER** — compiler.ajb uses 5 imports |
| 2 | `class Name { }` declaration | main.ajb:parseStmt | 150-200 | **BLOCKER** — compiler.ajb uses classes in stmt.ajb |
| 3 | `true` / `false` literals | main.ajb:parseAtom | 15-20 | **BLOCKER** — used in expr.ajb |
| 4 | `const` declaration | main.ajb:parseStmt | 20-30 | **BLOCKER** — compiler.ajb uses `const` in stmt.ajb |
| 5 | `self` keyword | main.ajb:parseAtom | 50-80 | **BLOCKER** — used in expr.ajb class methods |
| 6 | `new ClassName()` | main.ajb:parseAtom | 20-30 | **BLOCKER** — used in expr.ajb |

**Total P0: ~455-660 LOC**
**Estimated time: 1-2 weeks**

### P1 — Required for v0.2 Feature Parity (2,000-3,000 LOC)

| # | Feature | Source | LOC | Impact |
|---|---------|--------|-----|--------|
| 7 | `struct` declarations | main.ajb:parseStmt | 100-150 | Core language feature |
| 8 | `enum` declarations | main.ajb:parseStmt | 100-150 | Core language feature |
| 9 | `match` expressions | main.ajb:parseExpr | 200-300 | Core control flow |
| 10 | Generic params `[T]` | main.ajb:parseType | 80-120 | Required for stdlib |
| 11 | Float type + literals | lexer + parser + codegen | 200-300 | Required for math |
| 12 | Duplicate definition detection | new: semantic.ajb | 100-150 | Error reporting |
| 13 | Scope-based variable lookup | main.ajb:scopeTable | 50-80 | Already partially exists |
| 14 | Return type validation | new: typecheck.ajb | 100-150 | Error reporting |
| 15 | Binary op type compatibility | new: typecheck.ajb | 50-80 | Error reporting |
| 16 | Import function registration | main.ajb:parseStmt (import) | 30-50 | Required for multi-file |
| 17 | Float codegen (C backend) | main.ajb:emitCBlockCode | 30-50 | Required for float |
| 18 | Bool codegen (C backend) | main.ajb:emitCBlockCode | 15-20 | Required for bool |
| 19 | LLVM backend skeleton | new: llvm.ajb | 800-1,200 | Performance |
| 20 | C header completeness | main.ajb:emitCHeader | 20-30 | 7 missing declarations |

**Total P1: ~2,000-3,000 LOC**
**Estimated time: 4-6 weeks**

### P2 — Required for Package Ecosystem (1,500-2,500 LOC)

| # | Feature | Source | LOC | Impact |
|---|---------|--------|-----|--------|
| 21 | `trait` declarations | main.ajb:parseStmt | 100-150 | Abstraction |
| 22 | `impl` blocks | main.ajb:parseStmt | 150-200 | Method dispatch |
| 23 | Closures / lambda | main.ajb:parseExpr | 200-300 | Functional programming |
| 24 | `pub`/`pri`/`prot` visibility | lexer + parser | 50-80 | Module encapsulation |
| 25 | `@import` C FFI | main.ajb:parseStmt | 100-150 | C interop |
| 26 | Interpreter | new: interp.ajb | 1,000-1,500 | Direct execution |
| 27 | Module cache | new: cache.ajb | 200-300 | Fast recompilation |
| 28 | Package resolution (parth) | ajeeb-parth | 500-800 | Package ecosystem |
| 29 | Pattern matching (basic) | main.ajb:parseExpr | 100-150 | Error handling |
| 30 | `for`-`in` loops | main.ajb:parseStmt | 30-50 | Iteration |
| 31 | Range expressions `..` | main.ajb:parseExpr | 20-30 | Iteration |
| 32 | `as` cast | main.ajb:parseExpr | 20-30 | Type conversion |
| 33 | String interpolation | lexer + parser | 50-80 | Ergonomics |
| 34 | Number formats (hex/bin/oct) | lexer | 30-50 | Ergonomics |

**Total P2: ~1,500-2,500 LOC**
**Estimated time: 3-5 weeks**

### P3 — Future Features (800-1,200 LOC)

| # | Feature | Source | LOC | Impact |
|---|---------|--------|-----|--------|
| 35 | TCP/TLS networking | runtime + codegen | 100-150 | Network apps |
| 36 | SQLite | runtime + codegen | 100-150 | Database apps |
| 37 | FFI (dlopen/dlsym) | runtime + codegen | 100-150 | C library interop |
| 38 | `unsafe` blocks | parser + codegen | 50-80 | Low-level control |
| 39 | `defer` statements | parser + codegen | 30-50 | Resource management |
| 40 | Unused variable warnings | semantic | 30-50 | Developer experience |
| 41 | Missing return detection | semantic | 30-50 | Developer experience |
| 42 | Workspace support | parth | 100-150 | Multi-package projects |
| 43 | Security audit | parth | 100-150 | Supply chain security |
| 44 | Code formatter (ajeeb-fmt) | ajeeb-fmt | 200-300 | Developer experience |
| 45 | LSP server | ajeeb-lsp | 300-500 | IDE support |

**Total P3: ~800-1,200 LOC**
**Estimated time: 2-3 weeks**

---

## 14. Summary

| Priority | LOC Range | Time | Blocks |
|----------|-----------|------|--------|
| **P0** | 455-660 | 1-2 weeks | Self-hosting |
| **P1** | 2,000-3,000 | 4-6 weeks | v0.2 feature parity |
| **P2** | 1,500-2,500 | 3-5 weeks | Package ecosystem |
| **P3** | 800-1,200 | 2-3 weeks | Future features |
| **TOTAL** | **4,755-7,360** | **10-16 weeks** | Full parity |

**Current state:** 20% parity with Rust compiler.
**After P0:** Self-hosting achieved (but limited feature set).
**After P1:** ~55% parity (core language features).
**After P2:** ~80% parity (ecosystem features).
**After P3:** ~95% parity (remaining features).

---

## 15. Recommended Execution Order

### Phase A: Self-Hosting (P0 — 1-2 weeks)
1. Add `true`/`false` literals to lexer + parser (15-20 LOC)
2. Add `const` declaration to parser (20-30 LOC)
3. Add `class Name { }` declaration to parser (150-200 LOC)
4. Add `self` keyword handling (50-80 LOC)
5. Add `new ClassName()` expression (20-30 LOC)
6. Add `import name;` statement with file resolution (200-300 LOC)
7. Test: `main.ajb` compiles `compiler.ajb` → C → binary

### Phase B: Feature Parity (P1 — 4-6 weeks)
1. Add `struct`/`enum` declarations
2. Add `match` expressions
3. Add generic params `[T]`
4. Add float type + literals
5. Add semantic analysis (duplicate detection, type checking)
6. Add LLVM backend skeleton
7. Complete C header declarations

### Phase C: Ecosystem (P2 — 3-5 weeks)
1. Add `trait`/`impl` blocks
2. Add closures/lambda
3. Add interpreter
4. Add module cache
5. Add package resolution
6. Add visibility modifiers

### Phase D: Future (P3 — 2-3 weeks)
1. Add networking (TCP/TLS)
2. Add SQLite
3. Add FFI
4. Add tooling (fmt, LSP)
