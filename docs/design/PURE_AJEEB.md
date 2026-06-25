# Pure Ajeeb Distribution

## Date: June 25, 2026

## Overview
Ajeeb is now a self-hosting compiler that can build itself without external dependencies.

## What Ships in a Pure Ajeeb Distribution

### Binaries (pre-built)
| Binary | Size | Purpose | Can Rebuild? |
|--------|------|---------|--------------|
| `ajeebc` | 15 MB | Bootstrap compiler | Needs Rust (first time only) |
| `compiler` | 142 KB | Native compiler | Self-hosted (ajeebc → compiler.ajb) |
| `parth` | 5 MB | Package manager | Needs Cargo (has external deps) |

### Source Files
| Directory | Files | Purpose |
|-----------|-------|---------|
| `compiler/` | 6 files, 1,525 lines | Compiler source (Ajeeb) |
| `runtime/` | 1 file, 1,528 lines | C runtime |
| `tests/` | 20+ files | Test suite |

## Workflow Without Rust

### First-Time Setup
```bash
# Option 1: Use pre-built binaries (recommended)
cp ajeebc/build/ajeebc /usr/local/bin/
cp ajeebc/build/parth /usr/local/bin/

# Option 2: Build from source (needs Rust, one-time only)
cd ajeebc && make rust
```

### Normal Development
```bash
# Build compiler (no Rust needed)
make native

# Run tests (no Rust needed)
make test

# Verify self-hosting (no Rust needed)
make bootstrap

# Or use Parth (no Rust needed)
parth build file.ajb
parth run file.ajb
parth test
parth bootstrap
```

### When Rust IS Needed
1. **First-time bootstrap**: Building `ajeebc` from Rust source
2. **Modifying Parth**: Rebuilding the package manager (has external deps)
3. **CI/CD releases**: Building release binaries for all platforms

## Self-Hosting Verification

### Bootstrap Chain
```
ajeebc (Rust, 15MB)
  ↓ compiles compiler/compiler.ajb
compiler (Ajeeb, 142KB)
  ↓ compiles compiler/compiler.ajb → C
  ↓ gcc compiles C code
compiler_gen2 (Ajeeb, 142KB)
  ↓ diff output.c vs output_bootstrap.c
✅ IDENTICAL (2,705 lines, 52 functions)
```

### Verification Commands
```bash
# Full bootstrap verification
parth bootstrap

# Or manually
make native                    # Build compiler
./build/compiler compiler.ajb # Gen1 compiles itself
./build/compiler_gen2 compiler.ajb  # Gen2 compiles itself
diff build/output.c build/output_bootstrap.c  # Must be identical
```

## Distribution Package

### Minimum Viable Distribution
```
ajeebc/
├── build/
│   ├── ajeebc          # Bootstrap compiler (15MB)
│   ├── compiler        # Native compiler (142KB)
│   └── parth           # Package manager (5MB)
├── compiler/
│   ├── compiler.ajb    # Compiler source
│   ├── lexer.ajb
│   ├── emit.ajb
│   ├── expr.ajb
│   ├── stmt.ajb
│   └── pass1.ajb
├── runtime/
│   └── ajeeb_runtime.c # C runtime
├── tests/
│   └── *.ajb           # Test suite
└── Makefile             # Build system
```

### Installation
```bash
# Build everything (no Rust needed if binaries exist)
make

# Run tests
make test

# Verify self-hosting
make bootstrap
```

## Conclusion
Ajeeb is now a complete, self-hosting compiler distribution that requires no Rust or Cargo for normal development. The bootstrap chain is verified: Gen0 (Rust) → Gen1 (Ajeeb) → Gen2 (Ajeeb) produces identical output.
