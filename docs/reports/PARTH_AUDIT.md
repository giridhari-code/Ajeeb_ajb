# PARTH AUDIT

## 1. LOC Count

| Implementation | Location | LOC | Status |
|---------------|----------|-----|--------|
| Ajeeb modular | `parth/src/` (5 files) | 840 | Self-hosted, 7 commands |
| Ajeeb monolithic | `ajeebc/crates/parth/parth.ajb` | 1,204 | Self-hosted, 42 commands |
| Rust bootstrap | `ajeebBootstrap/crates/parth/` | 4,186 | Cargo workspace member |
| Rust ajeebc | `ajeebc/crates/parth/` | 4,661 | Full-featured, 35+ commands |
| **Total** | | **10,891** | |

## 2. Modules/Files

### Ajeeb Self-Hosted (`parth/src/`)

| File | LOC | Purpose |
|------|-----|---------|
| `main.ajb` | 161 | CLI dispatcher (7 commands: init, new, build, run, test, install, generate-lockfile) |
| `parser.ajb` | 206 | INI parser for `parth.das` config files. Uses buffer-slot storage. |
| `resolver.ajb` | 259 | Dependency resolution: local lookup, git clone via `exec()`, lockfile generation |
| `builder.ajb` | 152 | Build pipeline: read config → resolve deps → exec(ajeebc → llc → gcc) |
| `runner.ajb` | 62 | Run pipeline: build first, then exec(binary or parthi) |

### Rust Full Implementation (`ajeebc/crates/parth/`)

| Module | LOC | Purpose |
|--------|-----|---------|
| `main.rs` | 173 | CLI dispatcher (35+ commands) |
| `config.rs` | 230 | `parth.das` parser → `ProjectConfig` struct |
| `types.rs` | 383 | SemVer, constraints, lock entries, signatures, advisories |
| `resolver.rs` | 636 | PubGrub-style backtracking dependency resolver |
| `commands/build.rs` | 579 | Full build pipeline with LLVM IR, profiles, incremental |
| `commands/project.rs` | 628 | new/init/info/clean/version/vendor/workspace |
| `commands/deps.rs` | 202 | add/remove/install/update/upgrade/tree/why/outdated |
| `commands/registry.rs` | 250 | publish/search/login/logout/sign/verify |
| `commands/init.rs` | 100 | Project scaffolding |
| `registry/mod.rs` | 606 | Local registry index, package resolution |
| `registry/auth.rs` | 109 | Token-based authentication |
| `registry/cache.rs` | 75 | Content-addressed package cache |
| `registry/crypto.rs` | 197 | Ed25519 signing/verification |
| `registry/docs.rs` | 120 | Documentation generation |
| `registry/remote.rs` | 260 | HTTP package download |
| `registry/security.rs` | 108 | Security audit |

## 3. External Dependencies

### Rust Dependencies (from Cargo.toml)

| Crate | Purpose | Can be avoided in Ajeeb? |
|-------|---------|-------------------------|
| `ed25519-dalek` | Digital signatures | Yes — use `exec("openssl")` or skip signing |
| `getrandom` | Crypto RNG | Yes — use `exec("openssl rand")` |
| `hex` | Hex encoding | Yes — implement in Ajeeb (~20 LOC) |
| `rand` | Random numbers | Yes — use timestamp or `exec("shuf")` |
| `serde` / `serde_json` | JSON serialization | Yes — implement simple JSON in Ajeeb (~100 LOC) |
| `sha2` | SHA-256 hashing | Yes — use `exec("sha256sum")` |
| `reqwest` | HTTP client | Yes — use `exec("curl")` |
| `flate2` | Gzip compression | Yes — use `exec("gzip")` |
| `tar` | Tar archive handling | Yes — use `exec("tar")` |

**Key insight:** ALL Rust crate dependencies can be replaced by shelling out to system utilities via `exec()`. The Ajeeb self-hosted version already does this (uses `exec("git clone")` for fetching packages).

### System Dependencies

| Tool | Used by | Required? |
|------|---------|-----------|
| `ajeebc` | builder.ajb | Yes — compiles .ajb to LLVM IR |
| `llc` | builder.ajb | Yes — LLVM IR to assembly |
| `gcc` | builder.ajb | Yes — assembly to native binary |
| `git` | resolver.ajb | For fetching remote packages |
| `curl` | main.ajb | For downloading tools |
| `sha256sum` | cache (Ajeeb side) | For integrity checks |
| `mkdir` | main.ajb | For directory creation |

## 4. Public API

### Ajeeb Self-Hosted API (`parth/src/`)

```ajeb
// parser.ajb
function parseDasConfig(path: string): int          // Returns buffer slot
function getConfigName(buf: int, src: string): string
function getConfigVersion(buf: int, src: string): string
function getConfigAuthor(buf: int, src: string): string
function getBuildOutput(buf: int, src: string): string
function getBuildRuntime(buf: int, src: string): string
function getCompilerTarget(buf: int, src: string): string
function getCompilerEntry(buf: int, src: string): string
function getDepCount(buf: int): int
function getDepName(buf: int, src: string, idx: int): string
function getDepVersion(buf: int, src: string, idx: int): string

// resolver.ajb
function resolveAll(buf: int, src: string): int
function addDep(name: string, version: string): int
function removeDep(name: string): int
function generateLockfile(): int
function fileExists(path: string): int

// builder.ajb
function buildProject(): int

// runner.ajb
function runProject(): int
```

### Rust Full API (key functions)

```rust
// config
pub fn read_config(path: &Path) -> Result<ProjectConfig, String>
pub fn update_deps(path: &Path, new_deps: &[PkgDep]) -> Result<(), String>

// resolver
pub fn resolve_and_cache(deps: &[PkgDep], project_dir: &Path, registry_url: &str) -> Result<(Vec<PkgDep>, LockFile), String>
pub fn compilation_order(lock: &LockFile) -> Result<Vec<String>, String>
pub fn read_lock(path: &Path) -> LockFile
pub fn write_lock(lock: &LockFile, path: &Path) -> Result<(), String>

// registry
pub fn ensure_package(name: &str, version: &str, expected_checksum: &str) -> Result<(), String>
pub fn download_package(name: &str, version: &str, registry_url: &str) -> Result<PathBuf, String>
pub fn find_local_package(name: &str) -> Option<PathBuf>

// commands
pub fn cmd_init()
pub fn cmd_new(args: &[String])
pub fn cmd_build()
pub fn cmd_run()
pub fn cmd_add(args: &[String])
pub fn cmd_remove(args: &[String])
pub fn cmd_install(args: &[String])
pub fn cmd_tree()
pub fn cmd_test()
// ... 25+ more commands
```

## 5. Data Structures

### Ajeeb Self-Hosted (buffer-slot based)

| Structure | Storage | Purpose |
|-----------|---------|---------|
| `ProjectConfig` | Buffer slots 0-14 | name, version, author, output, runtime, target, entry, depCount |
| `Dependencies` | Buffer slots 200+ | 4 slots each: keyOff, keyLen, valOff, valLen |
| `LockFile` | `parth.lock` file | Simple `name = "version"` per line |

### Rust Full Implementation

| Structure | Fields | Purpose |
|-----------|--------|---------|
| `ProjectConfig` | name, version, description, author, homepage, license, deps, features, profiles, workspace, registry_url | Full project metadata |
| `PkgDep` | name, version_req | Single dependency |
| `LockEntry` | version, checksum, dependencies, registry | Locked dependency |
| `LockFile` | `HashMap<String, LockEntry>` | All locked deps |
| `Version` | major, minor, patch, pre, build | Semantic version |
| `VersionConstraint` | Any/Exact/Caret/Tilde/Gte/Gt/Lte/Lt/And/Or | Version constraint |
| `Decision` | package, version, dependencies, level | Resolution decision |
| `Conflict` | package, cause | Resolution conflict |
| `PackageSignature` | signer, hash, signature_hex, public_key_hex, timestamp | Package signature |
| `Advisory` | id, package, severity, versions_affected, description | Security advisory |
| `RegistryMetadata` | description, author, homepage, license, yanked | Package metadata |

## 6. Algorithms

### Semantic Version Parsing
- Parse `MAJOR.MINOR.PATCH[-pre][+build]`
- Pre-release sorts before release (`1.0.0-alpha < 1.0.0`)

### Version Constraint Matching
- `^1.2.3` = compatible (major must match; if major=0, minor must match)
- `~1.2.3` = patch-only changes
- `>=1.0.0 <2.0.0` = range (AND)
- `^1.0.0 || ^2.0.0` = union (OR)

### Dependency Resolution (PubGrub-style)
- BFS queue of dependencies
- For each dep: check lock → local cache → remote registry
- Backtracking on conflict: pop decision trail, try alternate versions
- Topological sort for compilation order with cycle detection

### Lock File Format (v2)
```ini
# parth-lock-v2
[package-name]
version = "1.0.0"
checksum = "sha256..."
registry = "url"
dependencies = "dep1@^1.0.0, dep2@>=2.0.0"
```

### Config Format (`parth.das`)
```ini
[package]
name = "my-project"
version = "0.1.0"

[dependencies]
ajeeb-std = "0.1.0"

[compiler]
target = "native"
entry = "src/main.ajb"
```

## 7. Features Actually Used by Ajeeb Ecosystem

### Used by root project (`parth.das`)
- `parth build` — compiles compiler.ajb to native binary
- `parth run` — runs the compiled binary
- `parth install` — resolves `ajeeb-std` dependency

### Used by packages
- `parth.das` manifest files (read by `parseDasConfig()`)
- Local package resolution (look in `packages/` directory)
- No package calls Parth functions directly

### NOT used
- Registry (no remote packages published)
- Signing/verification (no security)
- Workspaces (single project)
- Auth (no login)
- Publishing (no uploads)
- Audit (no security scanning)

---

## Migration Table

| Rust Module | LOC | Complexity | Rewrite Order | Notes |
|-------------|-----|-----------|---------------|-------|
| `config.rs` | 230 | Low | **M0-1** | INI parser, direct port of parser.ajb |
| `types.rs` (Version, Constraint) | 383 | Medium | **M0-2** | SemVer parsing/matching |
| `resolver.rs` (basic) | 200 | Medium | **M0-3** | Local package lookup + lockfile |
| `commands/build.rs` | 579 | Medium | **M0-4** | Build pipeline (ajeebc → llc → gcc) |
| `commands/init.rs` | 100 | Low | **M1-1** | Project scaffolding |
| `commands/project.rs` | 628 | Low | **M1-2** | new/info/clean/version |
| `commands/deps.rs` | 202 | Medium | **M1-3** | add/remove/install/tree/why |
| `resolver.rs` (full) | 436 | High | **M1-4** | PubGrub backtracking |
| `registry/mod.rs` | 606 | Medium | **M1-5** | Local registry index |
| `registry/cache.rs` | 75 | Low | **M1-6** | Content-addressed cache |
| `registry/remote.rs` | 260 | Medium | **M1-7** | HTTP download via curl |
| `registry/auth.rs` | 109 | Low | **M2-1** | Token storage |
| `registry/crypto.rs` | 197 | Medium | **M2-2** | Ed25519 via openssl |
| `registry/docs.rs` | 120 | Low | **M2-3** | Doc generation |
| `registry/security.rs` | 108 | Low | **M2-4** | Audit via advisory files |
| `commands/registry.rs` | 250 | Medium | **M2-5** | publish/search |
| `commands/misc.rs` | 273 | Low | **M2-6** | test/fmt/clean/workspace |
| **Total Rust** | **4,661** | | | |
| **Existing Ajeeb** (`parth/src/`) | **840** | | | Already partially done |
