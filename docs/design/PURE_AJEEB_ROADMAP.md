# Pure Ajeeb Migration Roadmap

**Goal:** Remove all Rust from the Ajeeb build chain. Final state: Ajeeb + GCC + LLC = full compiler.

**Date:** 2026-06-22
**Status:** PLANNING — no code changes yet

---

## Current State

```
┌─────────────────────────────────────────────────────┐
│                  BUILD CHAIN (TODAY)                 │
├─────────────────────────────────────────────────────┤
│                                                     │
│  Rust (cargo) ──→ ajeebc binary ──→ compiler.ajb   │
│       │                               │             │
│       │                    ┌──────────┘             │
│       │                    ▼                        │
│       │            C code / LLVM IR                 │
│       │                    │                        │
│       │                    ▼                        │
│       │              gcc / llc                      │
│       │                    │                        │
│       │                    ▼                        │
│       │              native binary                  │
│       │                    │                        │
│       │                    ▼                        │
│       └────────── self-hosting check                │
│                                                     │
└─────────────────────────────────────────────────────┘

CRATES:
  ajeeb-compiler  10,130 LOC  0 external deps  ← CAN be removed
  parth            4,995 LOC  11 external deps  ← MUST rewrite
  ajeeb-fmt        1,232 LOC   1 external dep   ← SHOULD rewrite
  ajeeb-lsp          600 LOC   3 external deps  ← CAN keep as optional
  ajeeb-registry     575 LOC  10 external deps  ← CAN keep as optional
```

---

## Dependency Graph

```
                    ┌──────────────┐
                    │ ajeebc binary│
                    │   (Rust)     │
                    └──────┬───────┘
                           │ compiles
                           ▼
                    ┌──────────────┐
                    │ compiler.ajb │──── depends on ──→ C runtime
                    │  (Ajeeb)     │                     │
                    └──────┬───────┘                     │
                           │ outputs                     │
                    ┌──────┴───────┐                     │
                    ▼              ▼                     │
              ┌──────────┐  ┌──────────┐                │
              │ C source │  │ LLVM IR  │                │
              └────┬─────┘  └────┬─────┘                │
                   │              │                      │
                   ▼              ▼                      │
              ┌──────────┐  ┌──────────┐                │
              │   gcc    │  │   llc    │                │
              └────┬─────┘  └────┬─────┘                │
                   │              │                      │
                   ▼              ▼                      │
              ┌──────────────────────────┐              │
              │     native binary        │              │
              │ (links to ajeeb_runtime) │◄─────────────┘
              └──────────────────────────┘
```

**What Rust provides today that Ajeeb doesn't:**
1. CLI argument parsing → `ajeeb_runtime.c` has `init_args()` + `readArg()`
2. File I/O → `ajeeb_runtime.c` has `readFile()`, `writeFile()`, `writeAppend()`
3. HTTP client → `ajeeb_runtime.c` has TCP sockets + TLS
4. Crypto (SHA-256, Ed25519) → Need C implementations
5. JSON parsing → Can use `cJSON` or write minimal parser
6. Process execution → `ajeeb_runtime.c` has `exec()`
7. String formatting → `ajeeb_runtime.c` has `itoa()`, `str_concat()`, `substring()`

---

## Feature Gap Analysis

### Ajeeb Self-Hosted Compiler vs Rust Compiler

| Feature | Status | Migration Priority | Effort |
|---------|--------|-------------------|--------|
| Lexer (all tokens) | ✅ DONE | — | — |
| Parser (basic statements) | ✅ DONE | — | — |
| HIR building | ✅ DONE | — | — |
| MIR building + optimization | ✅ DONE | — | — |
| C codegen | ✅ DONE | — | — |
| Classes (fields, methods, self, new) | ✅ DONE | — | — |
| Module imports (simple) | ✅ DONE | — | — |
| Semantic analysis / type checking | ❌ MISSING | Stage A | HIGH |
| Structs | ❌ MISSING | Stage A | MEDIUM |
| Enums | ❌ MISSING | Stage A | MEDIUM |
| Traits + impl blocks | ❌ MISSING | Stage A | HIGH |
| Generics (type parameters) | ❌ MISSING | Stage A | HIGH |
| Pattern matching (match) | ❌ MISSING | Stage A | MEDIUM |
| Float type | ❌ MISSING | Stage A | LOW |
| pub visibility | ❌ MISSING | Stage A | LOW |
| Closures / lambdas | ❌ MISSING | Stage B | HIGH |
| LLVM codegen | ❌ MISSING | Stage C | HIGH |
| Interpreter mode | ❌ MISSING | Stage D | MEDIUM |
| Module cache | ❌ MISSING | Stage E | LOW |
| Package system (parth.das) | ❌ MISSING | Stage E | MEDIUM |
| CLI flag parsing | ✅ DONE (basic) | Stage E | LOW |

---

## Migration Stages

### Stage A: Compiler Feature Parity
**Goal:** `compiler.ajb` can compile ALL Ajeeb language features through C codegen.
**Depends on:** Nothing
**Estimated effort:** 3-4 weeks

**Tasks:**
1. Add struct parsing + C codegen (`struct Point { x: int; y: int; }`)
2. Add enum parsing + C codegen (`enum Color { Red; Green; Blue; }`)
3. Add trait/impl parsing (at least struct dispatch)
4. Add basic type checking pass (int/string/bool inference)
5. Add pattern matching (`match expr { ... }`)
6. Add pub visibility (skip non-pub from output)
7. Add float type support
8. Add array index assignment (`arr[i] = val`)

**Verification:**
- `cargo run -- tests/test_struct.ajb` compiles and runs
- `cargo run -- tests/test_enum.ajb` compiles and runs
- `cargo run -- tests/test_trait.ajb` compiles and runs
- Self-hosting: `compiler.ajb` compiles itself

### Stage B: Parth Rewrite in Ajeeb
**Goal:** `parth` CLI tool rewritten in Ajeeb using C runtime.
**Depends on:** Stage A (needs full language)
**Estimated effort:** 2-3 weeks

**Tasks:**
1. Expand `parth/src/main.ajb` from 7 commands to 35+
2. Add config parser (`parth.das` reading)
3. Add build pipeline (already exists in `builder.ajb`)
4. Add dependency resolver (already exists in `resolver.ajb`)
5. Add local package management (copy/link)
6. Add version constraint parsing
7. Add lock file generation (already exists)
8. Add `parth test` command (run test files)
9. Add `parth clean` / `parth fmt` / `parth lint`

**What needs C runtime additions:**
- HTTP client for remote registry (TCP sockets + TLS already exist)
- SHA-256 for checksums (need C implementation or shell out to `sha256sum`)
- JSON parsing for registry responses (need minimal C JSON parser)
- Tar extraction (shell out to `tar xf`)
- Hex encoding (trivial in C)

**Verification:**
- `parth init hello` creates project
- `parth build` compiles project
- `parth run` runs project
- `parth add dep@1.0` adds dependency
- `parth test` runs tests

### Stage C: LLVM Codegen in Ajeeb
**Goal:** `compiler.ajb` can generate LLVM IR text (like Rust version does).
**Depends on:** Stage A
**Estimated effort:** 2-3 weeks

**Tasks:**
1. Port `llvm/mod.rs` logic to Ajeeb — generate `.ll` text files
2. Port `llvm/expr.rs` — expression codegen
3. Port `llvm/stmt.rs` — statement codegen
4. Port `llvm/types.rs` — type inference
5. Port `llvm/mir.rs` — MIR→LLVM mapping
6. String literal handling (global strings in LLVM IR)
7. Function definitions, calls, returns
8. Struct/array types in LLVM IR
9. Runtime function declarations

**Key insight:** The Rust compiler generates LLVM IR as **text strings** (not using inkwell/llvm-sys). This means the Ajeeb compiler can do the same — just string building.

**Verification:**
- `compiler.ajb --llvm test.ajb` generates valid `.ll`
- `llc test.ll -o test.s && gcc test.s -o test` works
- `compiler.ajb --llvm compiler.ajb` generates valid compiler

### Stage D: Interpreter Mode
**Goal:** `compiler.ajb --interpret` runs Ajeeb source directly.
**Depends on:** Stage A
**Estimated effort:** 2 weeks

**Tasks:**
1. Add interpreter mode to `main.ajb`
2. Port evaluator logic from Rust `eval/` to Ajeeb
3. RuntimeValue enum in Ajeeb (using struct + type tag)
4. Expression evaluation
5. Statement execution
6. Function call dispatch
7. Built-in function reimplementation

**Verification:**
- `compiler.ajb --interpret tests/test_simple.ajb` prints "Hello World"
- `compiler.ajb --interpret compiler.ajb` can parse Ajeeb source

### Stage E: Self-Hosting Bootstrap
**Goal:** `compiler.ajb` builds itself using only Ajeeb + C runtime + gcc + llc.
**Depends on:** Stages A, C
**Estimated effort:** 1 week

**Tasks:**
1. Create `bootstrap.sh` that:
   a. Compiles `compiler.ajb` → C → binary (using existing `ajeebc` once)
   b. Uses resulting binary to compile `compiler.ajb` again
   c. Verifies both outputs are identical (SHA-256)
2. Verify bootstrap chain:
   ```
   ajeebc → compiler.ajb → compiler_native
   compiler_native → compiler.ajb → compiler_native2
   sha256sum compiler_native compiler_native2  # must match
   ```
3. Document the bootstrap process

**Verification:**
- `bash bootstrap.sh` passes
- SHA-256 identity verified
- No Rust involved after initial `ajeebc` binary

### Stage F: Remove Cargo.toml
**Goal:** No Cargo.toml needed to build Ajeeb projects.
**Depends on:** Stages B, E
**Estimated effort:** 1 week

**Tasks:**
1. Move `Cargo.toml` → `Cargo.toml.legacy` (keep for reference)
2. Create `build.sh` that builds everything using Ajeeb
3. Update CI to use `build.sh` instead of `cargo build`
4. Verify: fresh clone + `bash build.sh` produces working compiler

**Verification:**
- No `cargo` in PATH → still builds
- All tests pass
- Bootstrap passes

### Stage G: Rust Removal
**Goal:** Rust is completely removed from the project.
**Depends on:** Stage F
**Estimated effort:** 0.5 weeks

**Tasks:**
1. Remove `Cargo.toml`, `Cargo.lock`, `target/` from repo
2. Remove `ajeebBootstrap/` directory
3. Update `README.md` — no Rust needed
4. Update `install.sh` — download pre-built binaries only
5. Update CI — builds from Ajeeb source

**Verification:**
- `find . -name "Cargo.toml"` returns nothing
- `find . -name "*.rs"` returns nothing (or only comments)
- Fresh install + build works

---

## Bootstrap Chain

```
STAGE 0: Bootstrap Seed (one-time, Rust)
  ┌─────────────────────────────────────────┐
  │  cargo build → ajeebc binary            │
  │  (This is the LAST time Rust is used)   │
  └─────────────────┬───────────────────────┘
                    │
                    ▼
STAGE 1: Self-Hosted Build
  ┌─────────────────────────────────────────┐
  │  ajeebc compiles compiler.ajb           │
  │  → compiler_native (C codegen)          │
  └─────────────────┬───────────────────────┘
                    │
                    ▼
STAGE 2: Verify Identity
  ┌─────────────────────────────────────────┐
  │  compiler_native compiles compiler.ajb  │
  │  → compiler_native2                     │
  │  sha256(compiler_native) ==             │
  │  sha256(compiler_native2) ✓             │
  └─────────────────┬───────────────────────┘
                    │
                    ▼
STAGE 3: Pure Ajeeb (no Rust)
  ┌─────────────────────────────────────────┐
  │  compiler_native compiles everything    │
  │  parth_native manages packages          │
  │  Only deps: gcc, llc                    │
  └─────────────────────────────────────────┘
```

**Critical rule:** The `ajeebc` binary is a **one-time bootstrap seed**. After Stage 2, it's never needed again. If the binary is lost, rebuild from source with Rust (one-time cost).

---

## Required Runtime Features

The C runtime (`ajeeb_runtime.c`) already has most features needed. Gaps:

| Feature | In Runtime? | Needed For | Action |
|---------|-------------|------------|--------|
| File I/O | ✅ Yes | Everything | Already done |
| String ops | ✅ Yes | Everything | Already done |
| Process exec | ✅ Yes | parth build | Already done |
| TCP sockets | ✅ Yes | Registry HTTP | Already done |
| TLS | ✅ Yes | HTTPS registry | Already done |
| SHA-256 | ❌ No | Checksums | Add C implementation |
| JSON parsing | ❌ No | Registry responses | Add minimal parser |
| Hex encode/decode | ❌ No | Checksums display | Trivial to add |
| Tar extraction | ❌ No | Package install | Shell out to `tar` |
| HTTP client | ⚠️ Partial | Registry fetch | Build on TCP+TLS |
| Random bytes | ❌ No | Nonce generation | Use `/dev/urandom` |
| Time/date | ✅ Yes | Metadata | `now_ms()` exists |
| Ed25519 | ❌ No | Package signing | Optional — defer to v0.3 |

**Estimated C runtime additions:** ~500 lines (SHA-256 + JSON parser + hex)

---

## Risk Assessment

### HIGH RISK

| Risk | Impact | Mitigation |
|------|--------|------------|
| Feature parity gap too large | Stage A takes months | Prioritize: structs, enums, traits first. Skip generics initially |
| Bootstrap identity breaks | Can't self-host | SHA-256 check at every stage; never modify compiler.ajb without re-bootstrapping |
| C runtime insufficient | Can't replace Rust stdlib | Audit every Rust stdlib usage; add only what's needed |

### MEDIUM RISK

| Risk | Impact | Mitigation |
|------|--------|------------|
| LLVM IR generation bugs | Generated code crashes | Validate with llc before gcc; test against known-good IR |
| Performance regression | Compiler too slow | Ajeeb compiler already compiles itself in reasonable time |
| Package registry unavailable | Can't install deps | Local packages + vendor mode as fallback |

### LOW RISK

| Risk | Impact | Mitigation |
|------|--------|------------|
| `ajeebc` binary lost | Can't bootstrap | Document rebuild procedure; keep binary in release artifacts |
| C runtime memory bugs | Crashes in generated code | Arena allocator already handles this |

---

## Estimated Total Effort

| Stage | Effort | Dependencies |
|-------|--------|-------------|
| Stage A: Compiler feature parity | 3-4 weeks | None |
| Stage B: Parth rewrite | 2-3 weeks | Stage A |
| Stage C: LLVM codegen | 2-3 weeks | Stage A |
| Stage D: Interpreter | 2 weeks | Stage A |
| Stage E: Bootstrap | 1 week | Stages A, C |
| Stage F: Remove Cargo.toml | 1 week | Stages B, E |
| Stage G: Rust removal | 0.5 weeks | Stage F |
| **Total** | **12-16 weeks** | |

---

## Execution Order

```
Week  1-4:  Stage A (compiler feature parity)
Week  5-7:  Stage C (LLVM codegen) + Stage D (interpreter) [parallel]
Week  8-10: Stage B (parth rewrite)
Week 11:    Stage E (bootstrap verification)
Week 12:    Stage F (remove Cargo.toml)
Week 13:    Stage G (remove Rust)
```

**Critical path:** A → C → E → F → G

---

## Success Criteria

The migration is complete when:

1. ✅ Fresh clone + `bash build.sh` produces working compiler (no cargo)
2. ✅ `compiler.ajb` compiles itself → identical output
3. ✅ `parth` manages projects (init/build/run/test)
4. ✅ All existing tests pass
5. ✅ LLVM codegen produces working binaries
6. ✅ No `Cargo.toml` in repository
7. ✅ No `.rs` files in repository (or only comments)
8. ✅ `install.sh` downloads pre-built binaries (no source build needed)

---

## What NOT to Remove (Yet)

| Component | Reason | Future |
|-----------|--------|--------|
| `ajeeb-lsp` | IDE support is developer tooling, not core | Can keep as optional Rust binary |
| `ajeeb-fmt` | Formatter is tooling | Can keep as optional Rust binary |
| `ajeeb-registry` | Server-side infra | Keep as Rust server (users don't build it) |
| `ajeeb-bootstrap/` | Reference implementation | Keep as legacy, remove from CI |
