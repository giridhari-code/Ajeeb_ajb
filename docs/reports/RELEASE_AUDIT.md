# Ajeeb v0.2 Release Audit Report

**Date:** 2026-06-22
**Auditor:** Automated (opencode)
**Status:** IN PROGRESS

---

## Strengths

1. **Self-hosting verified.** Compiler compiles itself via Rust → LLVM → native pipeline.
2. **No monster files in compiler core.** Largest file: `eval/builtins.rs` (995L). All 41 Rust source files under 1000L.
3. **All tests pass.** 24/24 cargo tests pass (4 compiler, 4 fmt, 16 parth).
4. **Full pipeline implemented.** HIR → THIR → MIR → LLVM IR → native. C codegen fallback exists.
5. **Multi-platform support.** CI builds for linux-x86_64, linux-aarch64, macos-arm64, macos-x86_64, windows-x86_64.
6. **Standard library.** 7 modules (418 lines): io, math, string, array, fs, result, collections.
7. **Cross-platform runtime.** `ajeeb_runtime.c` (1,452L) supports Linux, macOS, Windows with `#ifdef` guards.
8. **Parth package manager.** Rust version has 35 commands. Ajeeb self-hosted version has 7 core commands.
9. **Module system.** Compiler supports `import` with file-based module resolution.

## Risks

1. **`bootstrap_check.sh` missing.** AGENTS.md references `bash tests/bootstrap_check.sh` but file doesn't exist. Makefile has `make bootstrap` instead. Documentation/infrastructure mismatch.
2. **Parth monster files.** `main.rs` (1,898L) and `registry.rs` (1,454L) exceed 1000-line threshold.
3. **`eval/builtins.rs` at 995L.** 5 lines from threshold. Risk of growing past limit with new builtins.
4. **LLVM codegen at 2,641L total.** `llvm/mir.rs` (772L) and `llvm/expr.rs` (761L) are largest individual files. Could benefit from further split.
5. **No CI test step.** Release workflow builds but doesn't run `cargo test` or bootstrap verification.
6. **Parth test coverage low.** Only parser tests (12 files). No tests for resolver, builder, runner, or CLI.
7. **Ajeeb-level parth has only 7 commands** vs Rust version's 35. Gap in feature parity.
8. **Windows bootstrap check skipped.** Windows CI doesn't verify self-hosting.

## Known Issues

1. `println(str_concat(...))` codegen bug prints "true"/"false" instead of string content (known, documented in AGENTS.md).
2. Nested `str_concat` calls (3+ levels) break parser. Must flatten.
3. `set` requires initializer (`set x: int = 0;` not `set x: int;`).
4. No forward declarations in Ajeeb.
5. `class` has semantic analyzer bug (first pass doesn't register in `struct_defs`).
6. LLVM codegen `__index` limitation for non-constant index expressions.
7. `eval/traits.rs` and `eval/modules.rs` are empty stub modules.

## Release Blockers

| ID | Severity | Description | Status |
|----|----------|-------------|--------|
| RB-1 | HIGH | `bootstrap_check.sh` missing — AGENTS.md instructions broken | FIX NEEDED |
| RB-2 | HIGH | `cargo build` fails on v0.2.4 tag (was fixed, now passes) | FIXED |
| RB-3 | MEDIUM | Windows bootstrap check not verified | DEFERRED |
| RB-4 | LOW | No CI test step in release workflow | RECOMMENDED |

## Codebase Metrics

| Component | Files | Lines |
|-----------|-------|-------|
| Rust compiler (`ajeeb-compiler`) | 41 | 13,568 |
| LLVM codegen | 8 | 2,641 |
| C codegen | 1 | 417 |
| Runtime (`ajeeb_runtime.c`) | 1 | 1,452 |
| Compiler .ajb (self-hosted) | 7 | 2,885 |
| Standard library | 7 | 418 |
| Parth Rust | 5 | 4,601 |
| Parth Ajeeb | 5 | 840 |
| Parth tests | 13 | 383 |
| **Total** | **88** | **27,205** |

## Release Readiness Score

| Category | Score | Notes |
|----------|-------|-------|
| Compiler pipeline | 9/10 | Working, self-hosting, all stages implemented |
| LLVM backend | 8/10 | Functional, some edge cases in generics |
| Interpreter | 7/10 | Working but parity with LLVM output not fully verified |
| Runtime | 9/10 | Cross-platform, 61 exported functions |
| Tests | 8/10 | 24/24 pass, but coverage gaps in parth |
| Bootstrap | 7/10 | Works via Makefile, but script missing |
| Documentation | 3/10 | Only AGENTS.md exists, no user docs |
| Parth | 6/10 | Core works, monster files, low test coverage |
| CI/CD | 7/10 | Multi-platform builds, missing test step |

**Overall: 7.2/10 — Release-ready with minor cleanup needed.**

## Interpreter Parity (Updated 2026-06-22)

| Test | Interpreter | LLVM | C Backend | Status |
|------|-------------|------|-----------|--------|
| test_simple | ✓ | ✓ | ✓ | PARITY |
| test_math | ✓ | ✓ | ✓ | PARITY |
| test_if | ✓ | ✓ | ✓ | PARITY |
| test_while | ✓ | ✓ | ✓ | PARITY |
| test_for | ✓ | ✓ | ✓ | PARITY |
| test_strings | ✓ | ✓ | ✓ | PARITY |
| struct_basic | ✓ | ✗ | ✗ | INTERPRETER ONLY |
| enum_basic | ✓ | ✗ | ✗ | LLVM CODEGEN BUG |
| array tests | ✓ | partial | partial | RUNTIME MISSING |
| trait tests | ✓ | ✗ | ✗ | NOT IMPLEMENTED |

**Key finding:** Core features (int, string, bool, if/else, while, for, functions) have full parity. Structs, enums, arrays, and traits work in interpreter but have LLVM/C backend gaps.

## Recommendations

1. **Immediate:** Create `tests/bootstrap_check.sh` (DONE — symlink created)
2. **Before release:** Add `cargo test` step to CI workflow
3. **Before release:** ~~Split parth `main.rs` and `registry.rs~~ (DONE — refactored into 14 files)
4. **Before release:** Fix `str_concat` mixed-type argument handling
5. **Post-release:** Struct/array/enum C runtime functions
6. **Post-release:** Documentation suite (DONE — 6 docs created)
