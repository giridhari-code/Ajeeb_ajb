# Release Notes — Ajeeb v1.0.1

**Hotfix Release**

v1.0.1 is a hotfix release that fixes ARM64 Linux support and improves the installer and package manager binary lookup.

## Fixes

### Installer

- **ARM64 Linux support** — Pre-built binaries now available for linux-aarch64
- **No more Cargo/Rustc by default** — Installer downloads prebuilt binaries only; use `--build-from-source` flag to build from source
- **Clear error messages** — When binary is not available, shows how to build from source instead of silently falling back to cargo

### Parth Package Manager

- **Fixed binary lookup** — `parth run`, `parth build`, `parth test` no longer search `build/parthi` or `build/ajeebc` (repository build paths)
- **New search order**: bundled beside parth → `~/.ajeeb/bin/` → `PATH` → error with install instructions
- **No more `build/parthi not found`** — Never requires repository build artifacts
- **No more `Run: make`** — Never suggests building from source
- **Workspace members** — `parth build` now invokes `parth build` for workspace members instead of `cargo run -p parth`

### Compiler Lookup

- Same search order as parthi: bundled → `~/.ajeeb/bin/` → `PATH` → clear error
- No more dependency on `find_ajeeb_root()` for installed binary search
- Runtime file (`ajeeb_runtime.c`) also searched in `~/.ajeeb/bin/` and beside parth

## Release Assets

| Platform | Binary |
|----------|--------|
| Linux x86_64 | `ajeebc-linux-x86_64`, `parthi-linux-x86_64`, `parth-linux-x86_64` |
| Linux aarch64 | `ajeebc-linux-aarch64`, `parthi-linux-aarch64`, `parth-linux-aarch64` |

## Installation

```bash
# Download prebuilt binaries (default)
curl -sSf https://raw.githubusercontent.com/giridhari-code/Ajeeb_ajb/main/scripts/install.sh | bash

# Build from source (requires Rust)
curl -sSf https://raw.githubusercontent.com/giridhari-code/Ajeeb_ajb/main/scripts/install.sh | bash -s -- --build-from-source
```

## Verification

After install, verify:
```bash
export PATH="$HOME/.ajeeb/bin:$PATH"
ajeebc --version
parthi --version
parth --version
```

## Platforms

- Linux x86_64
- Linux aarch64 (new in v1.0.1)
- macOS x86_64
- macOS arm64
- Windows x86_64
