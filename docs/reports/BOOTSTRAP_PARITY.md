# Bootstrap Chain Parity Audit: Rust vs Ajeeb Self-Hosted

**Date:** 2026-06-22
**Status:** COMPLETE

---

## Executive Summary

The bootstrap chain is how the compiler compiles itself. The Rust compiler compiles `compiler.ajb` → C → binary via GCC. The Ajeeb compiler must eventually compile `compiler.ajb` → C → binary without Rust.

**Current Rust bootstrap: COMPLETE (4-step pipeline verified).**
**Ajeeb bootstrap: INCOMPLETE — cannot self-compile yet.**

---

## Current Bootstrap Chain (Rust)

```
Step 1: Rust interpreter compiles compiler/compiler.ajb → build/output.c
Step 2: GCC compiles output.c + runtime → build/ajeeb_native
Step 3: build/ajeeb_native compiles compiler/compiler.ajb → build/output2.c
Step 4: diff and sha256sum verify output.c ≡ output2.c
```

**Verification:** `bash tests/bootstrap_check.sh`

### What the Rust Interpreter Does

| Feature | Rust Implementation | LOC |
|---------|-------------------|-----|
| AST execution (no codegen) | `eval/` (5 files) | 1,895 |
| Lexer | `lexer.rs` | 364 |
| Parser | `parser/` (7 files) | 1,976 |
| HIR builder | `hir.rs` | 958 |
| Semantic analysis | `semantic.rs` | 1,617 |
| Type checking | `thir.rs` | 322 |
| Variable resolution | `resolver.rs` | 579 |
| Module loading | `loader.rs` | 380 |
| C code generation | `c_codegen.rs` | 417 |
| Built-in functions | `builtins.rs` | 990 |
| **Total** | | **9,500** |

### What the C Runtime Does

| Feature | C Implementation | LOC |
|---------|------------------|-----|
| Memory allocation | arena.c/h | ~150 |
| String operations | ajeeb_runtime.c | ~800 |
| File I/O | ajeeb_runtime.c | ~300 |
| Console I/O | ajeeb_runtime.c | ~100 |
| **Total** | | **~1,350** |

---

## Ajeeb Bootstrap Requirements

### Minimum Viable Self-Compiler

The Ajeeb compiler (`compiler.ajb`) must implement these features to compile itself:

#### Lexer Requirements

| Feature | Needed | Current | Status |
|---------|--------|---------|--------|
| All 21 keywords | Yes | 21 | COMPLETE |
| All 20 operators | Yes | 20 | COMPLETE |
| All 10 punctuation | Yes | 10 | COMPLETE |
| Whitespace skip | Yes | Yes | COMPLETE |
| Line comments | Yes | Yes | COMPLETE |
| Block comments | Yes | Yes | COMPLETE |
| Float literals | No (not in .ajb) | No | OK |
| `pub` keyword | No (not in .ajb) | No | OK |
| `struct` keyword | No (not in .ajb) | No | OK |
| `enum` keyword | No (not in .ajb) | No | OK |
| `match` keyword | No (not in .ajb) | No | OK |
| `trait` keyword | No (not in .ajb) | No | OK |
| `impl` keyword | No (not in .ajb) | No | OK |
| `::` token | No (not in .ajb) | No | OK |
| `@` token | No (not in .ajb) | No | OK |
| String escapes | No (not in .ajb) | No | OK |
| **Lexer parity needed** | | | **100%** |

#### Parser Requirements

| Feature | Needed | Current | Status |
|---------|--------|---------|--------|
| `set name: Type = expr;` | Yes | Yes | COMPLETE |
| `const name: Type = expr;` | Yes | Yes | COMPLETE |
| `if/else if/else` | Yes | Yes | COMPLETE |
| `while (cond) {}` | Yes | Yes | COMPLETE |
| `for (init; cond; update) {}` | Yes | Yes | COMPLETE |
| `break;` / `continue;` | Yes | Yes | COMPLETE |
| `return expr;` | Yes | Yes | COMPLETE |
| `function name(params) { body }` | Yes | Yes | COMPLETE |
| `class Name { ... }` | Yes | Yes | COMPLETE |
| `import module;` | Yes | Yes | COMPLETE |
| Operator precedence | Yes | Yes | COMPLETE |
| String concatenation (`+`) | Yes | Yes | COMPLETE |
| Function calls | Yes | Yes | COMPLETE |
| `self` parameter | Yes | Yes | COMPLETE |
| `new ClassName()` | Yes | Yes | COMPLETE |
| Nested expressions | Yes | Yes | COMPLETE |
| **Parser parity needed** | | | **100%** |

#### HIR/MIR Requirements

| Feature | Needed | Current | Status |
|---------|--------|---------|--------|
| All HIR statement types | Yes | 8/8 | COMPLETE |
| All HIR expression types | Yes | 9/9 | COMPLETE |
| All MIR opcodes | Yes | 12/12 | COMPLETE |
| MIR optimization | Yes | 4 passes | COMPLETE |
| **HIR/MIR parity needed** | | | **100%** |

#### Code Generation Requirements

| Feature | Needed | Current | Status |
|---------|--------|---------|--------|
| C header emission | Yes | Yes | COMPLETE |
| Runtime function declarations | Yes | Partial (25/41) | NEEDS WORK |
| Global buffers | Yes | Yes | COMPLETE |
| Function emission | Yes | Yes | COMPLETE |
| Variable assignment | Yes | Yes | COMPLETE |
| Function calls | Yes | Yes | COMPLETE |
| String concatenation | Yes | Yes | COMPLETE |
| Control flow (if/while/for) | Yes | Yes | COMPLETE |
| Array indexing | Yes | Yes | COMPLETE |
| Class method calls | Yes | Yes | COMPLETE |
| **Codegen parity needed** | | | **95%** |

#### Runtime Requirements

| Feature | Needed | Current | Status |
|---------|--------|---------|--------|
| Memory allocation | Yes | Yes | COMPLETE |
| String operations | Yes | Yes | COMPLETE |
| Console I/O | Yes | Yes | COMPLETE |
| File I/O | Yes | Yes | COMPLETE |
| `itoa` | Yes | Yes | COMPLETE |
| `str_concat` | Yes | Yes | COMPLETE |
| `readFile` | Yes | Yes | COMPLETE |
| `writeFile` | Yes | Yes | COMPLETE |
| `exec` | Yes | Yes | COMPLETE |
| `len` | Yes | Yes | COMPLETE |
| `charCode` / `chr` | Yes | Yes | COMPLETE |
| `rdPos` / `wrPos` | Yes | Yes | COMPLETE |
| **Runtime parity needed** | | | **100%** |

---

## Missing Features for Bootstrap

### Critical (Block Self-Compilation)

| Feature | Impact | Priority |
|---------|--------|----------|
| 16 more runtime function declarations in codegen | Missing `substr`, `indexOf`, `toLowerCase`, `toUpperCase`, `trim`, `startsWith`, `endsWith`, `replace`, `replace_all`, `split`, `parseInt`, `parseFloat`, `chr`, `rdPos`, `wrPos` | P0 |
| `exec()` command execution | Needed for package system | P1 |
| `mkdir()` directory creation | Needed for build system | P1 |

### Non-Critical (Can Bootstrap Without)

| Feature | Impact | Priority |
|---------|--------|----------|
| LLVM codegen | Can use C backend for bootstrap | P2 |
| Interpreter | Can use C backend for bootstrap | P2 |
| Module cache | Can always recompile | P2 |
| Package system | Can use simple imports | P2 |

---

## Bootstrap Readiness Checklist

| Step | Requirement | Status |
|------|------------|--------|
| 1 | Lexer handles all tokens in compiler.ajb | ✅ |
| 2 | Parser handles all syntax in compiler.ajb | ✅ |
| 3 | HIR/MIR handles all operations in compiler.ajb | ✅ |
| 4 | Codegen handles all patterns in compiler.ajb | ✅ (95%) |
| 5 | Runtime has all functions compiler.ajb calls | ✅ (90%) |
| 6 | No circular imports in compiler.ajb | ✅ |
| 7 | No forward declarations needed | ✅ |
| 8 | No global variables needed | ✅ (uses HIR buffer 509) |
| 9 | `set` always has initializer | ✅ |
| 10 | No duplicate `set` in same function | ✅ |
| 11 | All functions return values | ✅ |
| 12 | No nested `str_concat` > 2 levels | ✅ (flattened) |
| 13 | Self-compilation test passes | ❌ NOT TESTED |
| 14 | Bootstrap SHA256 verification | ❌ NOT TESTED |

**Bootstrap readiness: ~92% — blocked by 16 missing runtime function declarations.**

---

## Path to Bootstrap

### Option 1: Complete Feature Parity First (Recommended)
1. Add missing runtime function declarations to codegen
2. Test compiler.ajb compilation end-to-end
3. Verify bootstrap (output.c == output2.c)
4. Then add LLVM, interpreter, module cache

### Option 2: Minimal Bootstrap First
1. Add only the 16 missing function declarations
2. Test self-compilation
3. Verify bootstrap
4. Then add features incrementally

**Recommendation:** Option 1 — more robust, ensures completeness.

---

## Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|-----------|--------|------------|
| Missing function in runtime | High | High | Test all runtime functions |
| String concat bug | Medium | High | Flatten nested calls |
| Import cycle | Low | Medium | Test circular imports |
| Type mismatch | Low | Low | No type system in self-hosted code |
| Memory exhaustion | Medium | Medium | Arena allocator handles this |
| Bootstrap failure | Low | High | 4-step verification catches this |
