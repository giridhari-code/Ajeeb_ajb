# CHANGELOG.md

## Ajeeb Changelog

### v1.0.1 (June 26, 2026)

#### Bug Fixes

- **`parth init`**: Now generates `[compiler]` section in `parth.das` (consistency with `parth new`)
- **`parth build`**: Uses `--emit-llvm-only` flag for clean pipeline separation
- **`parth run --native`**: Fixed crash on missing `.c` file (was using wrong compiler flag)
- **`parth test`**: Gracefully handles missing `tests/` directory instead of exiting with error
- **`parth --version`**: Now shows "(ajeeb-native)" to indicate self-hosted status
- **Self-hosted parth**: Binary discovery is portable across user accounts (no hardcoded `/root/`)

#### Installer Improvements

- **Removed Cargo suggestions**: Never suggests building from source when downloads fail
- **Added checksum verification**: Downloads and verifies SHA256SUMS.txt
- **Fixed runtime URL**: Now downloads from release assets instead of raw GitHub content
- **Added all standard library packages**: Downloads all 13 `.ajb` files
- **Removed `--build-from-source` flag**: Installer only downloads prebuilt binaries

#### Release Assets

- Added `parthi-linux-x86_64` binary
- Regenerated `SHA256SUMS.txt` with all three binaries
- Updated `version.txt` to `v1.0.1`

### v1.0.0 (June 25, 2026)

#### Initial Release

- **Compiler**: Full Ajeeb compiler with LLVM backend (default) and C fallback
- **Package Manager**: Self-hosted `parth` with init, build, run, test commands
- **Interpreter**: `parthi` MIR interpreter for fast development
- **Standard Library**: 14 packages (io, math, string, array, fs, result, collections, option, path, process, test, time, json)
- **Installer**: Bash installer for Linux/macOS, PowerShell for Windows

#### Features

- Ajeeb language with functions, classes, enums, arrays, strings
- Module system with imports
- Package management with dependencies
- Workspace support
- Self-hosting compiler (compiles itself)

#### Known Issues

- `parth run` uses interpreter by default (not native binary)
- Windows support requires PowerShell
- ARM64 binaries require CI build
