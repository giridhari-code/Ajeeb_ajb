# Feature Parity Audit: Rust vs Ajeeb Self-Hosted Compiler

**Date:** 2026-06-22
**Status:** COMPLETE

---

## Executive Summary

| Subsystem | Rust LOC | Ajeeb LOC | Parity |
|-----------|----------|-----------|--------|
| Lexer | 364 | 227 | **82%** — missing float, pub, struct, enum, match, trait, impl, ::, @, string escapes |
| Parser | 1,976 | 535 | **45%** — missing 7 statement types, 10 expression types, generics, patterns |
| AST | 312 | (embedded) | **55%** — missing Float, Struct, Enum, Trait, Impl, Closure, Lambda, Match nodes |
| HIR | 958 | (embedded) | **53%** — 9/17 expr types, 8/8 stmt types, 5/10 type variants |
| THIR | 322 | 0 | **0%** — no type checking pass |
| MIR | 253 | (embedded) | **90%** — all opcodes + 2 extras (%, **) |
| Semantic | 1,617 | 0 | **0%** — no semantic analysis |
| Type Checker | 881 | 0 | **0%** — no type checking |
| Trait System | 23 | 0 | **0%** — no trait support |
| Generic System | 199 | 0 | **0%** — no generics |
| Module System | 257 | 211 | **40%** — single ident import, no cache, no cycle detection |
| Structs | (in parser/HIR/codegen) | 0 | **0%** — no struct token/parsing/codegen |
| Enums | (in parser/HIR/codegen) | 0 | **0%** — no enum token/parsing/codegen |
| Pattern Matching | 62+31 | 0 | **0%** — no match/pattern support |
| Closures | ~100 | 0 | **0%** — no closure/lambda support |
| Runtime Calls | 990 builtins | 25 codegen | **85%** — core functions covered, missing FFI/TCP/TLS/FFI |
| LLVM Codegen | 2,325 | 0 | **0%** — entirely missing |
| C Backend | 417 | ~370 | **85%** — missing float, bool const, FFI declarations |
| Interpreter | 1,895 | 0 | **0%** — entirely missing |
| Cache | 1,057 | 0 | **0%** — no module caching |
| **TOTAL** | **~13,568** | **~1,343** | **~30%** |

---

## Detailed Feature Tables

### Lexer (364 Rust / 227 Ajeeb)

| Feature | Rust LOC | Ajeeb LOC | Status |
|---------|----------|-----------|--------|
| Keywords: set, const, if, else, while, fn/function, return, true, false, int, string, bool, void, for, break, continue, class, self, new, import | token.rs:3-55 | lexer.ajb:114-154 | COMPLETE |
| Operators: +, -, *, /, ==, !=, <, >, <=, >=, &&, ||, !, =, ->, ++, --, +=, -=, **, % | token.rs:24-37, lexer.rs | lexer.ajb:174-214 | COMPLETE |
| Punctuation: ;, :, ,, ., (, ), {, }, [, ] | token.rs:38-49 | lexer.ajb:205-214 | COMPLETE |
| Whitespace skip (space, tab, CR, LF) | lexer.rs:43-51 | lexer.ajb:23-44 | COMPLETE |
| Line comments `//` | lexer.rs:53-58 | lexer.ajb:29-31 | COMPLETE |
| Block comments `/* */` | lexer.rs:62-69 | lexer.ajb:33-40 | COMPLETE |
| `float` keyword | token.rs:16 | — | MISSING |
| `pub` keyword | token.rs:19 | — | MISSING |
| `struct` keyword | token.rs:56 | — | MISSING |
| `enum` keyword | token.rs:57 | — | MISSING |
| `match` keyword | token.rs:58 | — | MISSING |
| `trait` keyword | token.rs:61 | — | MISSING |
| `impl` keyword | token.rs:62 | — | MISSING |
| `@` (at-import) | token.rs:18 | — | MISSING |
| `::` (double colon) | token.rs:40 | — | MISSING |
| `=>` (fat arrow) | token.rs:59 | — | MISSING |
| `_` (underscore) | token.rs:60 | — | MISSING |
| Float literal (number.`.`number) | lexer.rs:70-88 | — | MISSING |
| String escapes (\n, \t, \", \\, \0) | lexer.rs:89-93 | — | MISSING |
| **Keywords total** | 30 | 21 | 70% |
| **Operators total** | 20 | 20 | 100% |
| **Punctuation total** | 12 | 10 | 83% |

### Parser (1,976 Rust / 535 Ajeeb)

| Feature | Rust LOC | Ajeeb LOC | Status |
|---------|----------|-----------|--------|
| Operator precedence (or, and, eq, cmp, add, mul, unary, primary) | expr.rs:94-680 | expr.ajb:201-303 | COMPLETE |
| `set name: Type = expr;` | decls.rs:88-104 | stmt.ajb:29-55 | COMPLETE |
| `const name: Type = expr;` | decls.rs:106-122 | stmt.ajb:56-74 | COMPLETE |
| `if/else if/else` | decls.rs + stmt.rs | stmt.ajb:75-112 | COMPLETE |
| `while (cond) {}` | stmt.rs | stmt.ajb:159-171 | COMPLETE |
| `for (init; cond; update) {}` | stmt.rs | stmt.ajb:114-157 | COMPLETE |
| `break;` / `continue;` | stmt.rs | stmt.ajb:240-250 | COMPLETE |
| `return expr;` | stmt.rs | stmt.ajb:227-238 | COMPLETE |
| `function name(params) { body }` | decls.rs:124-222 | stmt.ajb:173-225 | PARTIAL — no generics, no trait bounds, no pub |
| `class Name { fields; methods; }` | decls.rs:224-263 | stmt.ajb:252-314 | PARTIAL — no pub modifier |
| `import module;` | decls.rs:8-39 | stmt.ajb:316-354 | PARTIAL — single ident only, no `::` path |
| `@import "lib" as name;` | decls.rs:41-86 | — | MISSING |
| `struct Name { fields }` | decls.rs:265-333 | — | MISSING |
| `enum Name { Variants }` | decls.rs:335-431 | — | MISSING |
| `trait Name { methods }` | decls.rs:441-532 | — | MISSING |
| `impl Type { methods }` | decls.rs:534-686 | — | MISSING |
| Generic params `[T]` | generics.rs:7-50 | — | MISSING |
| Type arg list `[Int, String]` | generics.rs:52-64 | — | MISSING |
| Type param bounds `T: Trait` | decls.rs:147-163 | — | MISSING |
| Match expression | expr.rs:14-47 | — | MISSING |
| Lambda `|params| => body` | expr.rs:700-835 | — | MISSING |
| `pub` modifier | mod.rs:217-224 | — | MISSING |
| **Statement types** | 16 | 10 | 63% |
| **Expression types** | 22 | 12 | 55% |

### HIR (958 Rust / embedded in Ajeeb)

| Feature | Rust | Ajeeb | Status |
|---------|------|-------|--------|
| HirType: Int, Bool, Str, Void | hir.rs:5-9 | main.ajb:238-243 | COMPLETE |
| HirType: Float, Named, Array, Generic, Fn | hir.rs:6-16 | — | MISSING |
| HirStmt: Set, Return, If, While, For, Expr, Break, Continue | hir.rs:85-108 | main.ajb:79-124 | COMPLETE |
| HirExpr: Int, Str, Bool, Var, BinOp, Call, ArrayLit, Index, Assign | hir.rs:129-197 | main.ajb:94-130 | COMPLETE |
| HirExpr: Float, MethodCall, StructLit, Field, FieldAssign, IndexAssign, EnumCtor, UnaryMinus, UnaryNot, Closure, ClosureCall | hir.rs:130-209 | — | MISSING |
| Top-level: HirFn | hir.rs:74-81 | main.ajb:72-75 | COMPLETE |
| Top-level: HirStructDef, HirEnumDef, HirTraitDef, HirImplBlock | hir.rs:240-273 | — | MISSING |
| **Total: 8/8 stmt types, 9/17 expr types, 5/10 type variants** | | | **~60%** |

### MIR (253 Rust / embedded in Ajeeb)

| Feature | Rust | Ajeeb | Status |
|---------|------|-------|--------|
| MirStmt: Assign, Call | mir.rs:28-36 | main.ajb:648 | COMPLETE |
| MirRvalue: Use, BinaryOp, Const | mir.rs:41-43 | main.ajb:684-686 | COMPLETE |
| Terminator: Goto, SwitchInt, Return | mir.rs:78-84 | main.ajb:648 (ops 5,6,7) | COMPLETE |
| Terminator: Unreachable | mir.rs:85 | — | MISSING |
| MirBinOp: Add, Sub, Mul, Div, Eq, Neq, Lt, Gt, Le, Ge, And, Or | mir.rs:62-73 | main.ajb:38 (codes 1-12) | COMPLETE |
| **EXTRA: Modulo (%)** | — | main.ajb:38 (code 15) | AJEEB EXTRA |
| **EXTRA: Power (**)** | — | main.ajb:38 (code 16) | AJEEB EXTRA |
| **EXTRA: str_concat opcode** | — | main.ajb:38 (code 12) | AJEEB EXTRA |
| Constant folding | mir.rs:97-138 | main.ajb:958-998 | COMPLETE |
| Dead block elimination | mir.rs:140-253 | main.ajb:1051-1094 | COMPLETE |
| **EXTRA: Dead code elimination** | — | main.ajb:1017-1050 | AJEEB EXTRA |
| **Total: 12/12 BinOps, 3/4 terminators, 2+extras optimizations** | | | **~95%** |

### Semantic Analysis (1,617 Rust / 0 Ajeeb)

| Feature | Rust | Ajeeb | Status |
|---------|------|-------|--------|
| Duplicate function/struct/enum/trait detection | mod.rs:67-195 | — | MISSING |
| Unknown type in field check | mod.rs:121-134 | — | MISSING |
| Impl trait method validation | mod.rs:263-296 | — | MISSING |
| Generic type arg count validation | generics.rs:67-84 | — | MISSING |
| Generic bound satisfaction | generics.rs:91-111 | — | MISSING |
| Trait impl lookup | traits.rs:4-22 | — | MISSING |
| Type substitution | generics.rs:34-53 | — | MISSING |
| Scope-based variable lookup | mod.rs:405-412 | — | MISSING |
| Method dispatch | typecheck.rs:454-529 | — | MISSING |
| Lambda type inference | typecheck.rs:276-307 | — | MISSING |
| Import function registration | mod.rs:367-385 | — | MISSING |
| **Total: 0/25+ checks** | | | **0%** |

### THIR Type Checking (322 Rust / 0 Ajeeb)

| Feature | Rust | Ajeeb | Status |
|---------|------|-------|--------|
| Type mismatch (Set) | thir.rs:86-95 | — | MISSING |
| Return type check | thir.rs:96-105 | — | MISSING |
| BinOp type compatibility | thir.rs:147-163 | — | MISSING |
| Call arg count check | thir.rs:165-176 | — | MISSING |
| FieldAccess existence check | thir.rs:211-223 | — | MISSING |
| StructLit field count/type | thir.rs:238-266 | — | MISSING |
| EnumCtor variant check | thir.rs:268-287 | — | MISSING |
| Unknown variable check | thir.rs:305-312 | — | MISSING |
| **Total: 0/13 checks** | | | **0%** |

### C Backend (417 Rust / ~370 Ajeeb)

| Feature | Rust | Ajeeb | Status |
|---------|------|-------|--------|
| C header with #include | c_codegen.rs:39-43 | main.ajb:1276-1278 | COMPLETE |
| Runtime function declarations | c_codegen.rs:46-89 | main.ajb:1280-1311 | PARTIAL — missing 17 functions |
| Global buffers | c_codegen.rs:92-94 | main.ajb:1313-1314 | COMPLETE |
| Function emission | c_codegen.rs:104-184 | main.ajb:1319-1361 | COMPLETE |
| Assign/Call/Control flow | c_codegen.rs:212-320 | main.ajb:1156-1274 | COMPLETE |
| String concat detection | c_codegen.rs:225-240 | main.ajb:1146-1149 | COMPLETE |
| String var tracking | c_codegen.rs:215-245 | main.ajb:1114-1131 | COMPLETE |
| Rvalue emission | c_codegen.rs:322-367 | main.ajb:1166-1271 | COMPLETE |
| Const emission (Int, Str) | c_codegen.rs:376-389 | main.ajb:1166-1172 | PARTIAL — missing Float, Bool |
| Terminator emission | c_codegen.rs:391-416 | main.ajb:1259-1265 | COMPLETE |
| **EXTRA: Modulo (%)** | — | main.ajb | AJEEB EXTRA |
| **EXTRA: Power (__ipow)** | — | main.ajb:1316 | AJEEB EXTRA |
| Float constants | c_codegen.rs:379-382 | — | MISSING |
| Bool constants | c_codegen.rs | — | MISSING |
| FFI/TCP/TLS declarations | c_codegen.rs:46-89 | — | MISSING |
| **Total: ~85% parity** | | | |

### LLVM Codegen (2,325 Rust / 0 Ajeeb)

| Feature | Rust | Ajeeb | Status |
|---------|------|-------|--------|
| All 57 features listed in LLVM_PARITY.md | llvm/ (7 files) | — | **0%** |

### Interpreter (1,895 Rust / 0 Ajeeb)

| Feature | Rust | Ajeeb | Status |
|---------|------|-------|--------|
| All 83 features listed in audit | eval/ (5 files) | — | **0%** |

---

## Summary

| Category | Complete | Partial | Missing | Parity |
|----------|----------|---------|---------|--------|
| Lexer | 29 tokens | 0 | 13 tokens | 70% |
| Parser | 8 features | 4 features | 13 features | 45% |
| HIR | 8 stmts, 9 exprs | 0 | 9 exprs, 5 types, 4 defs | 53% |
| MIR | 12 ops, 3 terms | 0 | 1 term + extras | 95% |
| Semantic | 0 | 0 | 25+ checks | 0% |
| THIR | 0 | 0 | 13 checks | 0% |
| C Backend | 10 features | 2 features | 3 features | 85% |
| LLVM | 0 | 0 | 57 features | 0% |
| Interpreter | 0 | 0 | 83 features | 0% |
| Module System | 2 features | 2 features | 8 features | 20% |
| **Weighted Average** | | | | **~30%** |
