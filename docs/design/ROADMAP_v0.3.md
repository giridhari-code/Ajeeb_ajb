# Ajeeb v0.3 Roadmap

**Target Release:** 2026-Q3
**Status:** Planning

---

## 1. Interpreter Parity with LLVM

**Priority:** HIGH
**Rationale:** The interpreter currently produces different output than LLVM for some edge cases. Full parity enables reliable testing without native compilation.

### Tasks
- Audit all 61 runtime functions for interpreter vs LLVM behavioral differences
- Implement missing LLVM-only features in interpreter (generics edge cases, float operations)
- Add interpreter-specific test suite that mirrors LLVM test cases
- Ensure `cargo run --bin ajeeb_compiler -- --interpret` produces identical output to native binary for all test files
- Document interpreter limitations vs LLVM backend

---

## 2. Package Registry (crates.io-style)

**Priority:** HIGH
**Rationale:** Parth currently only resolves local and GitHub packages. A central registry enables community contributions.

### Tasks
- Design registry API (upload, download, versioning, search)
- Implement registry server (Rust Actix/Axum or Ajeeb self-hosted)
- Package authentication (Ed25519 signatures already implemented)
- Version semver resolution
- Dependency graph visualization
- License compliance checking
- Mirror support for air-gapped environments

---

## 3. IDE Support (LSP Improvements)

**Priority:** MEDIUM
**Rationale:** IDE support dramatically improves developer experience and adoption.

### Tasks
- Implement Language Server Protocol (LSP) server
  - Real-time diagnostics
  - Auto-completion for keywords, functions, types
  - Go-to-definition for modules and functions
  - Hover documentation
  - Find references
  - Rename refactoring
- VS Code extension
- Vim/Neovim LSP client configuration
- Syntax highlighting grammar (TextMate/Tree-sitter)

---

## 4. More Standard Library Modules

**Priority:** MEDIUM
**Rationale:** Current stdlib covers basics (7 modules). Additional modules unlock more use cases.

### Proposed Modules

| Module | Description | Priority |
|--------|-------------|----------|
| `json.ajb` | JSON parse/serialize | HIGH |
| `http.ajb` | HTTP client (curl wrapper) | HIGH |
| `crypto.ajb` | SHA256, MD5, base64 encoding | MEDIUM |
| `regex.ajb` | Regular expressions (PCRE2 wrapper) | MEDIUM |
| `date.ajb` | Date/time utilities | MEDIUM |
| `os.ajb` | OS interaction (env vars, process mgmt) | LOW |
| `net.ajb` | TCP/UDP sockets | LOW |
| `test.ajb` | Testing framework (assert, test runner) | HIGH |

### Tasks
- Design API conventions for new modules
- Implement JSON parser in Ajeeb (or C runtime wrapper)
- Add HTTP client via libcurl C wrapper
- Write comprehensive tests for each module
- Add module documentation

---

## 5. Windows Full Support

**Priority:** MEDIUM
**Rationale:** Windows CI builds exist but bootstrap check is skipped. Full support needed for cross-platform adoption.

### Tasks
- Verify self-hosting on Windows (bootstrap check)
- Test Parth package manager on Windows (path separators, file permissions)
- Fix any Windows-specific runtime issues
- Add Windows-specific CI test step
- Document Windows development setup
- Test with MSVC and MinGW toolchains
- Handle Windows line endings (CRLF) in parser

---

## 6. Documentation Improvements

**Priority:** HIGH
**Rationale:** Documentation score is 3/10. User docs are critical for adoption.

### Tasks

#### User Documentation
- Getting Started guide (install, hello world, basic concepts)
- Language Reference (syntax, types, control flow, functions, classes)
- Standard Library API reference
- Parth Package Manager guide
- Migration guide from v0.1/v0.2

#### Developer Documentation
- Contributing guide
- Architecture overview (pipeline stages, codegen backends)
- How to add new builtins
- How to add new standard library modules
- Release process documentation

#### Tooling
- Generate API docs from source comments
- Add doc comments to all public functions
- Create interactive playground (web-based)

---

## 7. Performance Optimizations

**Priority:** LOW
**Rationale:** Current performance is acceptable for a self-hosted compiler. Optimizations can come post-v0.3.

### Tasks
- Profile compilation of large projects (10k+ lines)
- Identify bottlenecks in lexer, parser, codegen
- Implement incremental compilation (cache unchanged modules)
- Optimize string handling (avoid unnecessary copies)
- Add `-O` flag for optimized builds (LLVM opt passes)
- Benchmark against comparable languages (Rust, Go compile times)

---

## Release Criteria

### Must-Have (v0.3.0)
- [ ] Interpreter parity with LLVM verified
- [ ] Windows bootstrap check passing in CI
- [ ] CI test step in release workflow
- [ ] User documentation (Getting Started + Language Reference)
- [ ] At least 2 new stdlib modules (json, http)

### Nice-to-Have (v0.3.x)
- [ ] Package registry MVP
- [ ] LSP server with basic features
- [ ] 4+ additional stdlib modules
- [ ] Performance benchmarks

---

## Timeline

| Milestone | Target Date | Dependencies |
|-----------|-------------|--------------|
| v0.3.0-alpha | 2026-07-15 | Interpreter parity, Windows CI |
| v0.3.0-beta | 2026-08-01 | New stdlib modules, user docs |
| v0.3.0-rc1 | 2026-08-15 | Package registry MVP |
| v0.3.0 | 2026-09-01 | All must-have criteria met |

---

## Deferred to v0.4+

- Full LSP with rename refactoring
- Incremental compilation
- Web-based playground
- `net.ajb` (TCP/UDP sockets)
- Registry mirror support
