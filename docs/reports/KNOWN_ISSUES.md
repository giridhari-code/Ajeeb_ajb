# Ajeeb Known Issues

**Last Updated:** 2026-06-22
**Version:** v0.2.4

---

## 1. Language Limitations

| ID | Issue | Severity | Workaround |
|----|-------|----------|------------|
| L-1 | `set` requires initializer (`set x: int;` invalid) | MEDIUM | Use `set x: int = 0;` |
| L-2 | No forward declarations (`function foo(): int;` not supported) | MEDIUM | Declare functions before use or organize files top-down |
| L-3 | No global variables referenced from functions | HIGH | Use HIR buffer slot 509 for type communication |
| L-4 | Duplicate `set` in same function (even in different if-branches) is error | MEDIUM | Declare once at function top, use plain assignments in branches |
| L-5 | `class` has semantic analyzer bug — first pass doesn't register in `struct_defs` | HIGH | Use `struct` instead of `class` |
| L-6 | Nested `str_concat` calls (3+ levels) break parser | MEDIUM | Flatten nested calls into sequential assignments |
| L-7 | No `switch` statement — must use chained `if/else if` | LOW | Use `if/else if` chains |
| L-8 | No closures or lambda expressions | LOW | Use named functions |
| L-9 | No enums (only `struct` and `class`) | LOW | Use integer constants |

---

## 2. Compiler Bugs

| ID | Issue | Severity | Status |
|----|-------|----------|--------|
| B-1 | `println(str_concat(...))` may print "true"/"false" instead of string content | HIGH | Known, documented — avoid nested `str_concat` in `println` |
| B-2 | LLVM codegen `__index` limitation for non-constant index expressions | MEDIUM | Use constant index values |
| B-3 | `eval/traits.rs` and `eval/modules.rs` are empty stub modules | LOW | Not yet implemented |
| B-4 | Interpreter output differs from LLVM for some edge cases | MEDIUM | Use LLVM backend for production builds |
| B-5 | `bootstrap_check.sh` missing — AGENTS.md references non-existent script | HIGH | Use `make bootstrap` instead |

---

## 3. Missing Features

| ID | Feature | Priority | Notes |
|----|---------|----------|-------|
| M-1 | Package registry (crates.io-style) | HIGH | Parth only resolves local/GitHub packages |
| M-2 | Language Server Protocol (LSP) | MEDIUM | No IDE support currently |
| M-3 | JSON module | HIGH | Commonly needed for data interchange |
| M-4 | HTTP client module | HIGH | Currently requires `exec("curl ...")` |
| M-5 | Testing framework | HIGH | No built-in assert/test runner |
| M-6 | `switch` statement | LOW | Chained `if/else if` works |
| M-7 | Closures/lambdas | LOW | Named functions sufficient |
| M-8 | Enums | LOW | Integer constants work |
| M-9 | Documentation suite | HIGH | Only AGENTS.md exists |
| M-10 | Incremental compilation | LOW | Full rebuild works |

---

## 4. Platform-Specific Issues

| ID | Platform | Issue | Severity |
|----|----------|-------|----------|
| P-1 | Windows | Bootstrap check not verified in CI | MEDIUM |
| P-2 | Windows | Path separator handling in Parth untested | MEDIUM |
| P-3 | Windows | Line ending (CRLF) handling not verified | LOW |
| P-4 | macOS | No known issues | — |
| P-5 | Linux ARM | Source fallback required for install.sh | LOW |
| P-6 | All | No CI test step in release workflow (only build) | MEDIUM |

---

## 5. Parth Limitations

| ID | Issue | Severity | Notes |
|----|-------|----------|-------|
| PT-1 | Ajeeb-level Parth has 7 commands vs Rust version's 35 | HIGH | Feature gap |
| PT-2 | `main.rs` is 1,898 lines (monster file) | MEDIUM | Needs split |
| PT-3 | `registry.rs` is 1,454 lines (monster file) | MEDIUM | Needs split |
| PT-4 | Low test coverage — only parser tests (12 files) | HIGH | No tests for resolver, builder, runner, CLI |
| PT-5 | No lockfile integrity verification | MEDIUM | Parth.lock not cryptographically signed |
| PT-6 | No workspace support | LOW | Single-package only |

---

## 6. Testing Gaps

| Area | Current Coverage | Gap |
|------|-----------------|-----|
| Compiler tests | 24/24 pass | Edge cases in generics, closures |
| Parth parser tests | 12 files | No resolver/builder/runner tests |
| Standard library | Manual verification | No automated test suite |
| Bootstrap | Works via `make bootstrap` | Script missing from repo |
| CI/CD | Multi-platform builds | No test step in workflow |

---

## 7. Documentation Gaps

| Area | Status |
|------|--------|
| User guide | Not implemented |
| Language reference | Not implemented |
| API documentation | Not implemented |
| Contributing guide | Not implemented |
| Architecture docs | Not implemented |
| Release notes | CHANGELOG_v0.2.md only |
| Roadmap | ROADMAP_v0.3.md only |

---

## Workarounds Summary

| Workaround | Applies to |
|------------|-----------|
| Use `struct` instead of `class` | L-5 |
| Use `set x: int = 0;` instead of `set x: int;` | L-1 |
| Flatten nested `str_concat` calls | L-6 |
| Use constant index for `__index` | B-2 |
| Use `make bootstrap` instead of `bash tests/bootstrap_check.sh` | B-5 |
| Use LLVM backend for production builds | B-4 |
| Use `exec("curl ...")` for HTTP requests | M-4 |

---

## Reporting New Issues

Please file issues at: https://github.com/anomalyco/opencode/issues

Include:
1. Ajeeb version (`cargo run --bin ajeeb_compiler -- --version`)
2. Minimal reproducible example
3. Expected vs actual behavior
4. Platform and Rust version
