# PARTH REWRITE PLAN

## Overview

Rewrite Parth (the Ajeeb package manager/build tool) entirely in Ajeeb.

**Current state:** 3 implementations exist (Rust Bootstrap 4,186 LOC, Rust ajeebc 4,661 LOC, Ajeeb self-hosted 840 LOC). The existing Ajeeb version (`parth/src/`) already has a working parser, resolver, and builder but is incomplete vs. the Rust implementation.

**Goal:** Complete the Ajeeb rewrite so `parth` is self-contained — no Rust needed for the package manager.

---

## Milestone 0: Minimal Bootstrap

**Purpose:** `parth build` and `parth run` work end-to-end for a single-project (no external deps) setup. This is what you need to build the compiler itself.

**Deliverables:**
- `parth build` compiles `src/main.ajb` → native binary
- `parth run` builds then runs it
- `parth init` scaffolds a new project
- `parth new` creates a named project directory

**Effort:** ~2-3 hours (most code already exists in `parth/src/`)

### Files to create/modify

| File | Action | Lines | Notes |
|------|--------|-------|-------|
| `parth/src/main.ajb` | Modify | ~160 | Add `init` and `new` commands (currently missing) |
| `parth/src/parser.ajb` | Keep as-is | 206 | Already complete for `parth.das` parsing |
| `parth/src/resolver.ajb` | Keep as-is | 259 | Local + git resolution already works |
| `parth/src/builder.ajb` | Keep as-is | 152 | Build pipeline already works |
| `parth/src/runner.ajb` | Keep as-is | 62 | Run pipeline already works |
| `parth/src/init.ajb` | **New** | ~50 | `initProject()`, `newProject()` scaffolding |

### What already works (no changes needed)

1. **Config parsing** (`parser.ajb`): Reads `parth.das`, extracts name/version/author/output/runtime/target/entry/deps
2. **Local resolution** (`resolver.ajb`): Checks `packages/<name>/parth.das` and `../packages/<name>/parth.das`
3. **Git fetch** (`resolver.ajb`): `git clone --depth 1` into `/root/.parth/cache/<name>/src`
4. **Build pipeline** (`builder.ajb`): `ajeebc --emit-llvm-only` → `llc` → `gcc -no-pie`
5. **Runtime finder** (`builder.ajb`): Searches multiple paths for `ajeeb_runtime.c`
6. **Lock generation** (`resolver.ajb`): Writes `parth.lock` with `[[package]]` entries

### What's missing (new code)

1. **`init` command** (`init.ajb`): Create `parth.das` with default template, create `src/` directory, create `src/main.ajb` with hello world
2. **`new` command** (`init.ajb`): Create project directory, call `init` inside it

### Verification

```bash
# Build parth from Ajeeb source
cd parth && cargo run -- init     # creates parth.das + src/main.ajb
cargo run -- build                # compiles to native binary
cargo run -- run                  # runs the binary
```

---

## Milestone 1: Feature Parity (Core)

**Purpose:** Match the Rust implementation's features that are actually used by the Ajeeb ecosystem. Add `add`, `remove`, `install`, `tree`, `test` commands.

**Deliverables:**
- `parth add <name> <version>` adds dependency to `parth.das`
- `parth remove <name>` removes dependency
- `parth install` resolves all deps (local + git)
- `parth tree` shows dependency tree
- `parth test` builds and runs test target
- `parth generate-lockfile` creates `parth.lock`
- Version constraint matching (^, ~, >=, *)
- SemVer parsing (MAJOR.MINOR.PATCH[-pre][+build])
- Dependency cycle detection

**Effort:** ~4-6 hours

### Files to create/modify

| File | Action | Lines | Notes |
|------|--------|-------|-------|
| `parth/src/init.ajb` | Keep | ~50 | From M0 |
| `parth/src/parser.ajb` | Keep | 206 | No changes |
| `parth/src/resolver.ajb` | **Extend** | ~350 | Add version matching, cycle detection |
| `parth/src/builder.ajb` | Keep | 152 | No changes |
| `parth/src/runner.ajb` | Keep | 62 | No changes |
| `parth/src/main.ajb` | **Extend** | ~250 | Add all new commands |
| `parth/src/version.ajb` | **New** | ~150 | SemVer parsing + constraint matching |
| `parth/src/tree.ajb` | **New** | ~80 | Dependency tree display |

### New features detail

#### SemVer (`version.ajb`)
```ajeb
function parseVersion(v: string): int     // Returns buffer slot with major/minor/patch
function matchConstraint(v: string, constraint: string): int  // 1=match, 0=no
function sortVersions(a: string, b: string): int  // -1, 0, 1
```

Constraint rules:
- `^1.2.3` = compatible (major must match; if major=0, minor must match)
- `~1.2.3` = patch-only changes
- `>=1.0.0 <2.0.0` = range (AND)
- `*` = any version
- Exact match = exact version

#### Cycle detection (`resolver.ajb`)
- Track visited packages during resolution
- If a package is already in the visited set → cycle detected → error

#### Tree display (`tree.ajb`)
- Recursive walk of resolved dependencies
- Print tree with indentation

### Verification

```bash
cd parth && cargo run -- add ajeeb-std 0.1.0
cargo run -- tree
cargo run -- install
cargo run -- build
cargo run -- test
```

---

## Milestone 2: Full Parity (Advanced)

**Purpose:** Match all Rust implementation features including registry, signing, caching, workspace. This is the "nice to have" milestone.

**Deliverables:**
- Local registry index (read/write package metadata)
- Content-addressed package cache
- Ed25519 signing/verification (via `exec("openssl")`)
- Package publishing
- Search/login/logout
- Workspace support (multi-project)
- Security audit
- `parth update`, `parth upgrade`, `parth outdated`
- `parth info`, `parth clean`, `parth version`

**Effort:** ~8-12 hours

### Files to create/modify

| File | Action | Lines | Notes |
|------|--------|-------|-------|
| `parth/src/registry.ajb` | **New** | ~300 | Local registry index |
| `parth/src/cache.ajb` | **New** | ~100 | Content-addressed cache |
| `parth/src/crypto.ajb` | **New** | ~150 | Signing via openssl |
| `parth/src/workspace.ajb` | **New** | ~100 | Multi-project support |
| `parth/src/main.ajb` | **Extend** | ~400 | All remaining commands |

### Registry design
- Registry stored at `/root/.parth/registry/`
- Package metadata in `/root/.parth/registry/<name>/<version>/meta.json`
- Package sources in `/root/.parth/registry/<name>/<version>/src/`
- Content hashing with SHA-256 via `exec("sha256sum")`

### Signing design
- Ed25519 via `exec("openssl pkeyutl -sign")` / `-verify`
- Keys stored at `/root/.parth/keys/<name>_private.pem` and `<name>_public.pem`
- Signature stored in lockfile entry

### Verification
```bash
cd parth && cargo run -- publish
cargo run -- search ajeeb
cargo run -- verify ajeeb-std
```

---

## First Module to Rewrite

**`parth/src/init.ajb`** (new file, ~50 LOC)

This is the simplest missing piece — scaffolding for `parth init` and `parth new`. It has no dependencies on other modules and fills the gap in the existing implementation.

After that, `parth/src/main.ajb` needs modification to wire up the `init` and `new` commands (currently it has `build`, `run`, `test`, `install`, `generate-lockfile`, `add`, `remove` — but no `init`/`new`).

---

## Summary

| Milestone | Scope | LOC (new) | LOC (total) | Effort |
|-----------|-------|-----------|-------------|--------|
| M0: Bootstrap | build, run, init, new | ~50 | ~890 | 2-3h |
| M1: Core parity | +add, remove, tree, test, version matching | ~380 | ~1,270 | 4-6h |
| M2: Full parity | +registry, signing, cache, workspace | ~650 | ~1,920 | 8-12h |

**Recommendation:** Start with M0 (init.ajb + main.ajb wiring). Most of the hard work is already done in the existing `parth/src/` files. M0 is achievable in a single session.
