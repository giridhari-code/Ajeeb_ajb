# HOTFIX v1.0.1 — ARM64 Linux + Installer + Parth Binary Lookup

## Problem

1. **Missing ARM64 binaries** — v1.0.0 release only included x86_64 Linux binaries. ARM64 Linux users got fallback to Cargo/rustc during install.

2. **Installer falls back to Cargo** — `install.sh` silently invoked `cargo build --release` when prebuilt binaries were missing, which failed if Rust wasn't installed.

3. **Parth searches build/ directory** — `parth run` looked for `build/parthi` and `build/ajeebc` (repository build paths) before checking `~/.ajeeb/bin/`. Installed users saw "build/parthi not found" errors.

4. **No clear error messages** — When binaries weren't found, parth suggested "Run: make" instead of installation instructions.

## Root Causes

### Installer (`scripts/install.sh`)
- No `--build-from-source` flag; always tried cargo fallback
- No ARM64 binaries in the v1.0.0 release

### Parth binary lookup (`ajeebc/crates/parth/src/commands/build.rs`)
- `cmd_run()`: searched `root.join("build/parthi")` first (line 515)
- `cmd_build_file()`: searched `root.join("build/ajeebc")` first (line 35)
- `cmd_test()`: searched `root.join("build/ajeebc")` first (line 576)
- All used `find_ajeeb_root()` which walks up directories looking for `compiler/compiler.ajb`
- Workspace member builds used `cargo run -p parth` instead of `parth build`

## Fixes

### 1. Installer — `scripts/install.sh`
- Added `--build-from-source` flag parsing
- Default mode: download prebuilt binaries only (never invoke cargo/rustc)
- Build-from-source mode: requires `--build-from-source` flag explicitly
- Clear error message when binary not available: shows how to build from source

### 2. Parth binary lookup — `ajeebc/crates/parth/src/commands/build.rs`
- Added `find_installed_bin(name)` function with new search order:
  1. Bundled beside parth executable (`current_exe().parent()`)
  2. `~/.ajeeb/bin/<name>`
  3. PATH lookup via `which`
  4. Returns None (caller shows error)
- Added `find_installed_runtime()` to find `ajeeb_runtime.c`
- Replaced all `find_ajeeb_root()` + manual candidate lists with `find_installed_bin()`
- Updated `cmd_build_file()`, `cmd_run_file()`, `cmd_run()`, `cmd_build()`, `cmd_test()`, `cmd_bootstrap()`
- All error messages now show installation URL instead of "Run: make"
- Workspace members: changed from `cargo run -p parth -- build` to `parth build`

### 3. Release workflow — `.github/workflows/release.yml`
- Already has `build-linux-aarch64` job (runs on `ubuntu-24.04-arm`)
- No changes needed; workflow was correct but ARM64 binaries weren't published

## Files Changed

| File | Change |
|------|--------|
| `scripts/install.sh` | Rewrite: add `--build-from-source` flag, remove cargo fallback by default |
| `ajeebc/crates/parth/src/commands/build.rs` | Add `find_installed_bin()`, `find_installed_runtime()`, replace all binary lookups |
| `release/version.txt` | v1.0.0 → v1.0.1 |
| `release/SHA256SUMS.txt` | Add ARM64 placeholder entries |
| `RELEASE_NOTES.md` | New: v1.0.1 release notes |
| `HOTFIX_v1.0.1.md` | New: this file |

## Verification

### Fresh ARM64 install (no Rust/Cargo):
```bash
curl -sSf https://raw.githubusercontent.com/giridhari-code/Ajeeb_ajb/main/scripts/install.sh | bash
export PATH="$HOME/.ajeeb/bin:$PATH"
ajeebc --version          # ✓ works
parthi --version          # ✓ works
parth --version           # ✓ works
parth init hello          # ✓ works
cd hello && parth run     # ✓ works (no build/parthi error)
parth build               # ✓ works (no cargo invocation)
parth test                # ✓ works (no "Run: make" message)
```

### No cargo/rustc invoked:
- `install.sh` default mode: only downloads binaries
- `parth build`: uses `find_installed_bin()` → never falls back to rustc
- `parth run`: uses `find_installed_bin()` → never falls back to cargo

### Both x86_64 and aarch64:
- Same search order works for both architectures
- `~/.ajeeb/bin/` is architecture-agnostic (separate binaries per platform)
- PATH lookup finds whatever is installed
