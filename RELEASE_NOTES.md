# RELEASE_NOTES.md

## Ajeeb v1.0.1 — Rust-Free User Experience

**Release Date:** June 26, 2026

### What's New

This release makes Ajeeb fully usable without Rust or Cargo. Users can install and use Ajeeb with a single command, with no build tools required.

### Key Changes

- **Installer**: Never suggests building from source when downloads fail. Fails with clear error message and GitHub issue link.
- **Checksum Verification**: Installer now downloads and verifies SHA256SUMS.txt to ensure binary integrity.
- **Binary Discovery**: All binaries (ajeebc, parth, parthi) are automatically found via PATH or `~/.ajeeb/bin/`.
- **Runtime Discovery**: `ajeeb_runtime.c` is automatically located relative to the installed binaries.
- **Self-Hosted Parth**: The package manager is now self-hosted (compiled by ajeebc itself), eliminating Rust dependency for the user-facing tool.

### Bug Fixes

- `parth init` now generates `[compiler]` section in `parth.das` (consistency with `parth new`)
- `parth build` uses `--emit-llvm-only` flag for clean pipeline separation
- `parth run --native` no longer crashes on missing `.c` file
- `parth test` gracefully handles missing `tests/` directory
- `parth --version` now shows "(ajeeb-native)" to indicate self-hosted status
- Self-hosted parth binary discovery is portable across user accounts (no hardcoded `/root/`)

### Platform Support

| Platform | Binary | Status |
|----------|--------|--------|
| Linux x86_64 | `ajeebc-linux-x86_64`, `parth-linux-x86_64`, `parthi-linux-x86_64` | ✅ Available |
| Linux aarch64 | `ajeebc-linux-aarch64`, `parth-linux-aarch64`, `parthi-linux-aarch64` | ⏳ CI builds |
| macOS ARM64 | `ajeebc-macos-arm64`, `parth-macos-arm64`, `parthi-macos-arm64` | ⏳ CI builds |
| macOS x86_64 | `ajeebc-macos-x86_64`, `parth-macos-x86_64`, `parthi-macos-x86_64` | ⏳ CI builds |
| Windows x86_64 | `ajeebc-windows-x86_64.exe`, `parth-windows-x86_64.exe` | ⏳ CI builds |

### Installation

**Linux / macOS:**
```bash
curl -sSf https://raw.githubusercontent.com/giridhari-code/Ajeeb_ajb/main/scripts/install.sh | bash
```

**After installation:**
```bash
export PATH="$HOME/.ajeeb/bin:$PATH"
parth init hello
cd hello
parth run
```

### Requirements

- `gcc` or `clang` (for compiling generated code)
- `llc` (LLVM, for code generation)
- **NO Rust or Cargo required**

### SHA256 Checksums

See `SHA256SUMS.txt` for binary checksums.

### Known Issues

- `parth run` (without arguments) uses interpreter mode by default (fast). Use `parth run src/main.ajb --native` for native compilation.
- Windows support requires PowerShell installer (`install.ps1`).
