# RUST_USAGE_AUDIT.md

**Generated:** Phase 1 Audit  
**Repository:** /root/ajeeb_compiler  
**Total Rust Code:** 66 .rs files, ~21,096 lines

---

## Executive Summary

| Component | Lines | External Deps | Ajeeb Equivalent | Status |
|-----------|-------|---------------|------------------|--------|
| ajeeb-compiler | 17,933 | **ZERO** | compiler.ajb (self-hosting) | ✅ REPLACED |
| parth CLI | 3,900 | 11 crates (~160 transitive) | parth.ajb (partial) | 🔄 PARTIAL |
| ajeeb-fmt | 1,227 | 1 (ajeeb-compiler) | None | ❌ NOT REPLACED |
| ajeeb-lsp | 600 | 5 (ajeeb-compiler + serde etc.) | None | ❌ NOT REPLACED |
| ajeeb-registry | 575 | 10 (axum, tokio, etc.) | None | ❌ NOT REPLACED |

---

## Phase 1: Rust Files Inventory

### Crate: ajeeb-compiler (55 files, 17,933 lines)
**Purpose:** Gen0 bootstrap compiler — lex → parse → semantic → HIR → THIR → MIR → LLVM/C codegen  
**External Dependencies:** NONE (pure std Rust, only dlopen FFI)  
**Ajeeb Equivalent:** compiler/compiler.ajb (self-hosting)  
**Status:** ✅ FULLY REPLACED — compiler.ajb compiles itself

Key files:
- `src/main.rs` (501 lines) — CLI entry, pipeline orchestration
- `src/lexer.rs` (364 lines) — Tokenizer
- `src/parser/` (1,277 lines) — Full parser with patterns, generics
- `src/semantic/` (1,617 lines) — Type checking, scope analysis
- `src/hir.rs` + `src/hir_lower.rs` (958 lines) — HIR generation
- `src/thir.rs` + `src/thir_to_mir.rs` (960 lines) — MIR generation
- `src/mir.rs` (253 lines) — MIR types + optimization
- `src/llvm/` (2,640 lines) — LLVM IR codegen
- `src/c_codegen.rs` (420 lines) — C fallback codegen
- `src/eval/` (1,900 lines) — Interpreter mode
- `src/cache/` (1,057 lines) — Module caching
- `src/module.rs` (257 lines) — Import resolution

### Crate: parth (12 files, 3,900 lines)
**Purpose:** Package manager CLI (40+ commands)  
**External Dependencies:** ed25519-dalek, sha2, reqwest, serde, serde_json, rand, tar, flate2, hex  
**Ajeeb Equivalent:** parth/parth_m1.ajb (1,204 lines, partial)  
**Status:** 🔄 PARTIAL — init/build/run/test/clean/bootstrap done

Key files:
- `src/main.rs` (176 lines) — CLI dispatcher
- `src/types.rs` (387 lines) — Data types
- `src/config.rs` (230 lines) — parth.das parser
- `src/resolver.rs` (657 lines) — Dependency resolution
- `src/commands/build.rs` (642 lines) — Build/run/test/bootstrap
- `src/commands/deps.rs` (240 lines) — add/remove/update
- `src/commands/project.rs` (628 lines) — fmt/bench/lint/doc/clean
- `src/commands/registry.rs` (271 lines) — search/install/publish
- `src/registry/` (1,652 lines) — Registry operations, crypto, auth

### Crate: ajeeb-fmt (5 files, 1,227 lines)
**Purpose:** Code formatter — AST → pretty-printed source  
**External Dependencies:** ajeeb-compiler (local path)  
**Ajeeb Equivalent:** None  
**Status:** ❌ NOT REPLACED

### Crate: ajeeb-lsp (1 file, 600 lines)
**Purpose:** Language Server Protocol (JSON-RPC over stdin/stdout)  
**External Dependencies:** ajeeb-compiler, serde, serde_json, tracing  
**Ajeeb Equivalent:** None  
**Status:** ❌ NOT REPLACED

### Crate: ajeeb-registry (1 file, 575 lines)
**Purpose:** HTTP package registry server (Axum)  
**External Dependencies:** axum, tokio, serde, serde_json, tower-http, sha2, hex, chrono, tracing  
**Ajeeb Equivalent:** None  
**Status:** ❌ NOT REPLACED

---

## Phase 2: Cargo/Rustc References

### Makefile (`ajeebc/Makefile`)
- `rustc --edition 2021` — builds bootstrap compiler (no Cargo)

### Installer (`scripts/install.sh`)
- `cargo build --release` — guarded by `--build-from-source` flag only

### CI (`release.yml`)
- `cargo build --release` for ajeeb-compiler + parth on all 5 platforms
- `dtolnay/rust-toolchain@stable` for Rust installation

### Documentation (91 references)
- ~30 files reference `cargo test`
- ~15 files reference `cargo run`
- ~20 files reference `cargo build`

---

## Phase 3: External Crate Dependencies

### parth (only crate with external deps)
| Crate | Version | Purpose |
|-------|---------|---------|
| ed25519-dalek | 2.2.0 | Package signing |
| getrandom | 0.3 | Crypto randomness |
| hex | 0.4 | Hex encoding |
| rand | 0.8 | Random numbers |
| rand_core | 0.6 | RNG traits |
| serde | 1 | Serialization |
| serde_json | 1 | JSON |
| sha2 | 0.10 | SHA-256 checksums |
| reqwest | 0.12 | HTTP client |
| flate2 | 1 | Gzip decompression |
| tar | 0.4 | Tar extraction |

**Total transitive crates:** ~160  
**Total lock file lines:** 1,708

---

## Phase 4: Target Directories

| Path | Size | Status |
|------|------|--------|
| ajeebc/target/ | 1.8 GB | STALE — no workspace Cargo.toml |
| ajeebc/crates/ajeeb-compiler/target/ | 5.8 MB | Active (release build) |
| ajeebc/crates/parth/target/ | 496 MB | Active (debug + release) |

---

## Rewrite Priority

1. **parth CLI** — Most user-facing, partially done
2. **ajeeb-fmt** — Developer tool, medium complexity
3. **ajeeb-lsp** — IDE support, complex (JSON-RPC)
4. **ajeeb-registry** — Server, complex (HTTP, async)
5. **Installer** — Script, not Rust (already Ajeeb-compatible)

---

## Key Insight

The ajeeb-compiler (Gen0 bootstrap) is already fully replaced by compiler.ajb. The bootstrap chain (Gen0 → Gen1 → Gen2) proves self-hosting. The remaining Rust code is tooling (parth, fmt, lsp, registry), not the compiler itself.
