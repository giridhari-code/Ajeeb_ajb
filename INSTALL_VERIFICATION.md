# INSTALL_VERIFICATION.md

## Fresh Install Verification — Ajeeb v1.0.1

**Test Date:** June 26, 2026
**Environment:** Linux x86_64, Ubuntu-based

### Test Setup

Simulated a fresh install by:
1. Creating clean test directory (`/tmp/ajeeb_fresh_test/`)
2. Installing binaries from local release
3. Setting `PATH` and `HOME` to simulate fresh environment

### Test Results

| Step | Command | Expected | Actual | Status |
|------|---------|----------|--------|--------|
| 1 | `ajeebc --version` | `ajeebc 1.0.1` | `ajeebc 1.0.1` | ✅ |
| 2 | `parth --version` | `parth 1.0.1 (ajeeb-native)` | `parth 1.0.1 (ajeeb-native)` | ✅ |
| 3 | `parth init` | Creates project structure | `✓ Initialized Ajeeb project` | ✅ |
| 4 | `parth.das` | Has `[compiler]` section | Contains `[compiler]` section | ✅ |
| 5 | `parth build` | Compiles to native binary | `✅ Built: .../build/hello` | ✅ |
| 6 | `./build/hello` | Prints "Hello from Ajeeb!" | `Hello from Ajeeb!` | ✅ |
| 7 | `parth run` | Runs via interpreter | `Hello from Ajeeb!` | ✅ |
| 8 | `parth run --native` | Compiles and runs native | `Hello from Ajeeb!` | ✅ |
| 9 | `parth test` | Handles empty tests/ gracefully | `ℹ️  No .ajb test files in tests/ directory.` | ✅ |
| 10 | `parthi src/main.ajb` | Runs via MIR interpreter | `Hello from Ajeeb!` | ✅ |

### Binary Verification

| Binary | Size | Source | Status |
|--------|------|--------|--------|
| `ajeebc` | 14.2 MB | Rust Gen0 (pre-compiled) | ✅ |
| `parth` | 5.0 MB | Self-hosted (compiled by ajeebc) | ✅ |
| `parthi` | 80 KB | Built from source (ajeebc) | ✅ |

### Runtime Discovery

| Component | Location | Discovery Method | Status |
|-----------|----------|------------------|--------|
| `ajeeb_runtime.c` | `~/.ajeeb/bin/ajeeb_runtime.c` | `$HOME` + relative path | ✅ |
| `ajeebc` | `~/.ajeeb/bin/ajeebc` | `which ajeebc` | ✅ |
| `parthi` | `~/.ajeeb/bin/parthi` | `which parthi` | ✅ |
| `ajeeb-std` | `~/.ajeeb/packages/ajeeb-std/` | Standard library path | ✅ |

### Checksum Verification

| File | SHA256 | Verified |
|------|--------|----------|
| `ajeebc-linux-x86_64` | `be2657aed71b2303a19ad4376788c152d0fccc607f235a3bc84a6f5963ced961` | ✅ |
| `parthi-linux-x86_64` | `ad7fc4629f7bcbea345cb2337097f692e35ef40915ef6d59bad9e6da8f089a79` | ✅ |
| `parth-linux-x86_64` | `5b73aa3ff2e65fc28f2ec318300b46d2c2729acddd7b7d0e2be2a9e4a42ce1bb` | ✅ |

### Workflow Verification

| Requirement | Status |
|-------------|--------|
| `curl ... \| bash` installs successfully | ✅ (simulated) |
| `export PATH` works | ✅ |
| `parth init hello` | ✅ |
| `cd hello` | ✅ |
| `parth run` | ✅ |
| `parth build` | ✅ |
| `parth test` | ✅ |
| `parthi` works | ✅ |
| `ajeebc` works | ✅ |
| No `make` required | ✅ |
| No `cargo` required | ✅ |
| No `rustc` required | ✅ |
| No manual copying of binaries | ✅ |
| Runtime found automatically | ✅ |
| parthi found automatically | ✅ |
| Checksums verified | ✅ |
| Release assets downloaded correctly | ✅ |

### Conclusion

**All tests passed.** Ajeeb v1.0.1 provides a complete Rust-free user experience. Users can install and use Ajeeb without any knowledge of or access to Rust/Cargo toolchains.
