# Bootstrap — Self-Hosting Verification for Ajeeb

## What Bootstrap Means

Ajeeb is a self-hosting compiler: the compiler is written in Ajeeb itself (`compiler/compiler.ajb`). Bootstrap verification proves that the Rust-written compiler can compile the Ajeeb compiler source into a working native binary, and that the resulting binary is functionally equivalent to the original compilation.

This is the critical proof that Ajeeb is a real, viable programming language — it can compile itself.

## The 4-Step Bootstrap Pipeline

The bootstrap process (`ajeebc/tests/bootstrap_check.sh`) runs this pipeline:

```
Step 1: Build Rust compiler (MIR → LLVM pipeline)
    └── rustc compiles crates/ajeeb-compiler/src/main.rs → build/ajeeb_compiler

Step 2: Compile compiler.ajb via MIR → native binary
    └── build/ajeeb_compiler compiler/compiler.ajb --skip-run → build/compiler

Step 3: Verify all test files compile and run correctly via MIR pipeline
    └── For each test: build/ajeeb_compiler tests/<name>.ajb → build/<name>
    └── Run each binary and compare output to expected string
```

### Step 1: Build Rust Compiler

```bash
make rust
# Equivalent to:
rustc --edition 2021 --crate-type bin \
    crates/ajeeb-compiler/src/main.rs \
    -o build/ajeeb_compiler
```

Produces `build/ajeeb_compiler` — the Rust-compiled Ajeeb interpreter/compiler.

### Step 2: Compile Compiler.ajb → Native

```bash
./build/ajeeb_compiler compiler/compiler.ajb --skip-run
```

The Rust compiler processes `compiler/compiler.ajb` through the full pipeline:
- AST → Semantic analysis → HIR → THIR → MIR → LLVM IR → native binary

Produces `build/compiler` — a native binary that IS the Ajeeb compiler.

### Step 3: Verify Test Files

Each test file is compiled and run with output verification:

| Test | Expected Output |
|------|-----------------|
| `test_simple` | `Hello World` |
| `test_math` | `42` |
| `test_if` | `bada hai` |
| `test_while` | `0\n1\n2` |
| `test_for` | `0\n1\n2\n4\n5` |
| `test_strings` | `Hello World\nHELLO\najeeb\n1\n1\nHello` |

## How to Run Bootstrap

### Via Makefile (Recommended)

```bash
make bootstrap
```

This runs the full 3-step pipeline: `make rust` → `make native` → re-compile with native binary.

### Via Script

```bash
bash tests/bootstrap_check.sh
```

This runs the 3-step verification pipeline with output comparison.

### Manual Steps

```bash
# Step 1: Build Rust compiler
make rust

# Step 2: Compile compiler.ajb
./build/ajeeb_compiler compiler/compiler.ajb --skip-run

# Step 3: Run tests
./build/ajeeb_compiler tests/test_simple.ajb --skip-run
./build/test_simple  # should print "Hello World"
```

## SHA256 Identity Verification

The original AGENTS.md describes a 4-step pipeline that includes SHA256 verification:

1. Rust interpreter compiles `compiler/compiler.ajb` → `build/output.c`
2. GCC compiles output.c + runtime → `build/ajeeb_native`
3. `build/ajeeb_native` compiles `compiler/compiler.ajb` → `build/output2.c`
4. `diff` and `sha256sum` verify output.c ≡ output2.c

This proves deterministic output: the same source produces identical compiled output regardless of which compiler binary processes it. The current `bootstrap_check.sh` implements a simplified version of this pipeline.

## Cross-Platform Bootstrap

The CI builds for multiple platforms:

| Platform | Architecture | Bootstrap Verified |
|----------|-------------|-------------------|
| Linux | x86_64 | Yes (full pipeline) |
| Linux | aarch64 | Yes (full pipeline) |
| macOS | ARM64 | Yes (full pipeline) |
| macOS | x86_64 | Yes (full pipeline) |
| Windows | x86_64 | Build only (self-hosting not verified) |

The C runtime (`runtime/ajeeb_runtime.c`, 1,452 lines) uses `#ifdef` guards for cross-platform support:
- Linux: `#ifdef __linux__`
- macOS: `#ifdef __APPLE__`
- Windows: `#ifdef _WIN32`

## Self-Hosting Verification

Self-hosting is verified when:

1. The Rust compiler successfully compiles `compiler/compiler.ajb` into a working native binary
2. The native binary can compile and run Ajeeb programs correctly
3. All test files pass with expected output

The bootstrap success message:
```
✅ BOOTSTRAP SUCCESS — MIR pipeline verified!
  Pipeline: AST → Semantic → HIR → THIR → MIR → LLVM IR → native
  compiler.ajb compiles to working native binary (<size>)
  All test files compile and run correctly ✓
```

## After Any Change

Per AGENTS.md, after any code change:
1. Run `cargo test` — verify Rust tests pass
2. Run `bash tests/bootstrap_check.sh` — verify self-hosting works
3. Run key `.ajb` interpreter tests (e.g., `cross_simple.ajb`, `test_strings.ajb`)
