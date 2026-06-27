# Parth — Package Manager for Ajeeb

Parth is the Cargo-equivalent for the Ajeeb programming language. It manages projects, dependencies, builds, and package publishing. Parth exists in two implementations:

- **Rust version** (`parth` binary): Full-featured, 35 commands, built via Cargo
- **Ajeeb version** (`parth/src/main.ajb`): Self-hosted, 7 core commands, compiles to native via Ajeeb compiler

## Commands

### Rust Version (35 commands)

| Command | Description |
|---------|-------------|
| `parth new <name>` | Create a new Ajeeb project |
| `parth init` | Initialize project in current directory |
| `parth add <pkg>[@<version>]` | Add a dependency |
| `parth remove <pkg>` | Remove a dependency |
| `parth build [file.ajb]` | Compile project or single file |
| `parth run [file.ajb]` | Run with Piri interpreter (fast) |
| `parth run file.ajb --native` | Compile to native binary, then run |
| `parth test` | Run all tests in `tests/` directory |
| `parth bench [filter]` | Run benchmarks in `benches/` directory |
| `parth fmt [files..]` | Format Ajeeb source files |
| `parth lint [path]` | Lint Ajeeb source files |
| `parth doc [--open]` | Generate documentation |
| `parth sanitize [file]` | Run sanitizer checks (memory safety, bounds) |
| `parth update` | Update all dependencies (re-resolve) |
| `parth upgrade [pkg]` | Upgrade dependencies |
| `parth outdated` | Check for outdated dependencies |
| `parth tree` | Show dependency tree |
| `parth why <pkg>` | Explain why a package is included |
| `parth info` | Show project info from `parth.das` |
| `parth version` | Show parth and project version |
| `parth clean` | Remove build artifacts |
| `parth package` | Package into tarball without publishing |
| `parth generate-lockfile` | Generate `parth.lock` without building |
| `parth vendor` | Vendor dependencies into `vendor/` directory |
| `parth ls` | List workspace packages |
| `parth link <path>` | Link a local package to cache |
| `parth list` | Show all available packages (global + local) |
| `parth search <query>` | Search packages in registry |
| `parth install <pkg>` | Install a package |
| `parth publish [url]` | Publish the package |
| `parth login [url]` | Authenticate with a registry |
| `parth logout` | Remove stored credentials |
| `parth whoami` | Show current user |
| `parth sign <pkg> <v>` | Sign a package with Ed25519 |
| `parth verify <pkg> <v>` | Verify package signature |
| `parth keygen` | Generate Ed25519 signing keypair |
| `parth yank <pkg> <v>` | Yank a package version |
| `parth unyank <pkg> <v>` | Un-yank a package version |
| `parth audit` | Security audit of dependencies |
| `parth cache <cmd>` | Cache management (`info`, `clear`, `prune`, `put`, `get`, `lookup`) |
| `parth workspace [add <path>]` | Workspace management |
| `parth help` | Show help message |

### Ajeeb Version (7 commands)

Defined in `parth/src/main.ajb`:

| Command | Description |
|---------|-------------|
| `parth init [name]` | Initialize project in current directory |
| `parth new <name>` | Create a new project directory |
| `parth build` | Compile project via Ajeeb compiler |
| `parth run` | Run project with Piri interpreter |
| `parth test` | Run tests from `tests/` folder |
| `parth install <tool>` | Download tool binary (`ajeebc` or `piri`) |
| `parth generate-lockfile` | Generate lock file for dependencies |

## Project Layout

A Parth project uses the `parth.das` configuration format (INI-like):

```ini
[package]
name = "my-project"
version = "0.1.0"
author = "Your Name"
description = "A brief description"
homepage = "https://..."
license = "MIT"
registry = "https://registry.ajeeb.dev"

[dependencies]
math = "^1.0.0"
io = ">=0.5.0"

[features]
fast = ["math"]

[workspace]
members = "sub-crate"

[profile.dev]
opt-level = "0"
debug = "true"

[profile.release]
opt-level = "3"
debug = "false"
lto = "true"

[runtime]
max_threads = "8"
log_level = "info"

[compiler]
target = "native"
output = "build/"
runtime = "runtime/ajeeb_runtime.c"
entry = "src/main.ajb"
```

### Standard Directory Structure

```
my-project/
├── parth.das          # Project manifest
├── src/
│   └── main.ajb       # Entry point
├── tests/
│   └── test_*.ajb     # Test files
├── benches/
│   └── *.ajb          # Benchmark files
├── packages/          # Local dependency packages
├── vendor/            # Vendored dependencies
├── build/             # Build output
│   ├── output.ll      # LLVM IR
│   ├── output.s       # Assembly
│   └── <project>      # Native binary
└── parth.lock         # Lock file
```

## Build Pipeline

Parth compiles Ajeeb source through a multi-stage pipeline:

1. **Dependency Resolution**: Reads `parth.das`, resolves all dependencies via version constraints
2. **Source Combination**: Merges all `.ajb` source files (deps + entry point) into `build/combined.ajb`
3. **Compilation**: Uses `ajeebc` (self-hosted compiler) or falls back to Rust interpreter
   - `ajeebc` → LLVM IR (`build/output.ll`)
   - Rust interpreter → `cargo run -p ajeeb-compiler` → LLVM IR
4. **Assembly**: `llc -O2 output.ll -o output.s`
5. **Linking**: `gcc output.s runtime/ajeeb_runtime.c -o build/<project>`

### Build Profiles

- **dev** (default): `opt-level=0`, debug symbols, no LTO
- **release**: `opt-level=3`, no debug, LTO enabled

### Dependency Search Paths

Parth searches for dependencies in this order:
1. `./packages/<name>/` (project-local)
2. `~/.parth/packages/<name>/` (global)
3. `<ajeeb_root>/packages/<name>/` (Ajeeb standard library)

## Dependency Management

### Version Constraints

Parth supports semantic versioning with these constraint operators:

| Operator | Example | Meaning |
|----------|---------|---------|
| `*` | `*` | Any version |
| `^` | `^1.0.0` | Compatible (>=1.0.0, <2.0.0; or >=0.1.0, <0.2.0 if major=0) |
| `~` | `~1.2.0` | Patch-level (~1.2.0 means >=1.2.0, <1.3.0) |
| `>=` | `>=1.0.0` | Greater than or equal |
| `>` | `>1.0.0` | Greater than |
| `<=` | `<=2.0.0` | Less than or equal |
| `<` | `<2.0.0` | Less than |
| `=` | `=1.5.0` | Exact match |
| Compound | `>=1.0.0 <2.0.0` | Range (AND) |
| OR | `^1.0.0 \|\| ^2.0.0` | Union (OR) |

### Lock File

`parth.lock` records resolved dependency versions with checksums:

```toml
[math]
version = "1.2.0"
checksum = "sha256:abc123..."
dependencies = []
registry = "local"
```

### Registry Features

- **Publish**: Package source into tarball, compute checksum, register in local index
- **Sign/Verify**: Ed25519 digital signatures for package authenticity
- **Yank/Unyank**: Mark versions as withdrawn from availability
- **Audit**: Security scan for known vulnerabilities

## Known Limitations

1. **Two implementations not feature-complete**: The Ajeeb version has 7 commands vs Rust version's 35. Feature parity is deferred.
2. **No remote registry**: Currently only works with local packages and `~/.parth/packages/`. Remote registry URL is configurable but not implemented.
3. **Monster files**: `main.rs` (1,898L) and `registry.rs` (1,454L) exceed 1000-line threshold. Splitting is recommended.
4. **Low test coverage**: Only parser tests (12 files). No tests for resolver, builder, runner, or CLI.
5. **`parth build` error messages unclear**: When native compiler (`ajeebc`) is not found, the error path can be confusing.
6. **`class` semantic analyzer bug**: First pass doesn't register classes in `struct_defs`. Workaround: use `struct` instead.
7. **Nested `str_concat` limitation**: Three or more nested calls break the parser. Must flatten into separate statements.
8. **Windows bootstrap not verified**: CI builds for Windows but does not verify self-hosting.
