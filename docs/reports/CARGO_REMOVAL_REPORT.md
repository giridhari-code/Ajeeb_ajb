# Stage F: Cargo Removal Report

## Summary
Successfully modified Parth to eliminate `cargo run` from the default development workflow. The default workflow now uses the pre-built `ajeebc` binary directly.

## Changes Made

### 1. `ajeebc/crates/parth/src/commands/build.rs`
- **Fixed `cmd_test()`**: Replaced `cargo run --bin ajeeb_compiler -- --interpret` with direct execution of `build/ajeebc --interpret`
- **Fixed `cmd_build_file()` fallback**: Replaced `cargo run --bin ajeeb_compiler` with `rustc` compilation when pre-built binary not found
- **Fixed `cmd_build()` fallback**: Replaced `cargo run --bin ajeeb_compiler` with `rustc` compilation
- **Added `cmd_bootstrap()`**: New command that runs the full Gen0→Gen1→Gen2 bootstrap chain
- **Fixed `cmd_run_file()`**: Rewrote to use `build/ajeebc --interpret` as the default interpreter (falls back to `parthi` if available)

### 2. `ajeebc/crates/parth/src/main.rs`
- Added `"bootstrap"` to the command dispatch
- Added `bootstrap` to help text

### 3. `tests/bootstrap_check.sh`
- Fixed to run from `ajeebc/` directory (where Makefile is located)

### 4. `ajeebc/build/ajeebc`
- Created symlink: `ajeebc → ajeeb_compiler`

## Verification Results

### Parth Commands
| Command | Status | Notes |
|---------|--------|-------|
| `parth build` | ✅ Works | Uses `ajeebc` binary directly |
| `parth build file.ajb` | ✅ Works | Compiles single file via LLVM |
| `parth run file.ajb` | ✅ Works | Uses `ajeebc --interpret` |
| `parth test` | ✅ Works | Uses `ajeebc --interpret` for each test |
| `parth clean` | ✅ Works | Removes build artifacts |
| `parth bootstrap` | ✅ Works | Full Gen0→Gen1→Gen2 chain |

### Bootstrap Check
```
✅ BOOTSTRAP SUCCESS — MIR pipeline verified!
  Pipeline: AST → Semantic → HIR → THIR → MIR → LLVM IR → native
  compiler.ajb compiles to working native binary (92K)
  All test files compile and run correctly ✓
```

### Cargo Test
```
test result: ok. 4 passed; 0 failed
```

## Cargo Status
- **Parth binary**: Rebuilt with Cargo (one-time build for dependencies)
- **Default workflow**: No Cargo required (uses pre-built `ajeebc`)
- **Cargo remaining**: Only needed to rebuild Parth itself (ed25519-dalek, reqwest, etc.)
- **Makefile**: Already Cargo-free (uses `rustc` directly)

## Binary Sizes
- `ajeeb_compiler` (Gen0): 14.9 MB
- `compiler` (Gen1): 145 KB
- `compiler_gen2` (Gen2): 145 KB
- `parth`: 5.1 MB
- `ajeebc`: symlink → ajeeb_compiler

## Conclusion
The default development workflow no longer requires Cargo:
1. `make` builds the compiler (no Cargo)
2. `make test` runs tests (no Cargo)
3. `make bootstrap` verifies self-hosting (no Cargo)
4. `parth build/run/test/clean/bootstrap` all work without Cargo

Cargo is only needed if modifying Parth source code (which has external dependencies).
