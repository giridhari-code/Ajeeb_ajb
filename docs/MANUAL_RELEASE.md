# Manual Release Process

All CI/CD workflows have been disabled. Releases must be done manually.

## Prerequisites

- `llvm` (for `llc`): `apt install llvm` / `brew install llvm`
- `gcc` or `cc`
- `git`
- `make`

## Build Commands

### Compiler (ajeebc)

```bash
cd ajeebc && make native
```

Builds `build/compiler` from `compiler/compiler.ajb` via MIR pipeline.

### Parth

```bash
bash parth/build.sh
```

Builds `build/parth_m2` from `parth/parth_m1.ajb`.

## Test Commands

### Compiler Tests

```bash
cd ajeebc && make test
```

Compiles and runs all test files (test_simple, test_math, test_if, test_while, test_for, test_strings, test_chr_escape).

### Bootstrap Verification

```bash
cd ajeebc && make native
bash tests/bootstrap_check.sh
```

Verifies:
1. Gen0 compiles `compiler/compiler.ajb` → native binary (Gen1)
2. Gen1 re-compiles `compiler/compiler.ajb` → Gen2
3. Gen1 and Gen2 produce identical output

### Parth Tests

```bash
# Run individual test files via compiler
cd ajeebc && ./build/ajeebc tests/test_simple.ajb --interpret
```

## Manual Release Process

### 1. Set Version

```bash
VERSION="v1.0.1"
echo "$VERSION" > release/version.txt
```

### 2. Build All Binaries

```bash
# Linux x86_64
ajeebc-linux-x86_64 compiler/compiler.ajb --emit-llvm-only -o build/compiler.ll
llc -O2 --march=x86-64 build/compiler.ll -o build/compiler.s
gcc build/compiler.s runtime/ajeeb_runtime.c -o release/ajeebc-linux-x86_64 -lm -ldl

# Linux aarch64
ajeebc-linux-aarch64 compiler/compiler.ajb --emit-llvm-only -o build/compiler.ll
llc -O2 --march=aarch64 build/compiler.ll -o build/compiler.s
gcc build/compiler.s runtime/ajeeb_runtime.c -o release/ajeebc-linux-aarch64 -lm -ldl

# macOS arm64
ajeebc-macos-arm64 compiler/compiler.ajb --emit-llvm-only -o build/compiler.ll
llc -O2 --march=aarch64 build/compiler.ll -o build/compiler.s
cc build/compiler.s runtime/ajeeb_runtime.c -o release/ajeebc-macos-arm64 -lm

# macOS x86_64
ajeebc-macos-x86_64 compiler/compiler.ajb --emit-llvm-only -o build/compiler.ll
llc -O2 --march=x86-64 build/compiler.ll -o build/compiler.s
cc build/compiler.s runtime/ajeeb_runtime.c -o release/ajeebc-macos-x86_64 -lm

# Windows x86_64 (cross-compile from Linux)
llc -O2 --march=x86-64 build/compiler.ll -o build/compiler.s
x86_64-w64-mingw32-gcc build/compiler.s runtime/ajeeb_runtime.c \
  -o release/ajeebc-windows-x86_64.exe -lm
```

> **Note:** The self-hosted compiler (built from `compiler.ajb`) does NOT support `--emit-llvm-only`. Use the Gen0 bootstrap binary for cross-platform LLVM IR generation.

### 3. Run Bootstrap Check

```bash
# Verify self-hosting: Gen0 → Gen1 → Gen2
bash tests/bootstrap_check.sh
```

### 4. Create Release Archive

```bash
mkdir -p release/packages/ajeeb-std
cp runtime/ajeeb_runtime.c release/
cp packages/ajeeb-std/*.ajb release/packages/ajeeb-std/
cp scripts/install.sh release/
cp scripts/install.ps1 release/
```

### 5. Create Git Tag

```bash
git tag "$VERSION"
git push origin "$VERSION"
```

## Restoring CI/CD Workflows

To re-enable automated releases:

1. Rename the workflows directory:
   ```bash
   mv .github/workflows.disabled .github/workflows
   ```
2. Create a new tag:
   ```bash
   git tag v1.1.0
   git push origin v1.1.0
   ```
3. Verify CI run at https://github.com/giridhari-code/Ajeeb_ajb/actions

## Known Issues

- Self-hosted compiler (from `compiler.ajb`) lacks `--emit-llvm-only` — use Gen0 for cross-platform IR
- `bootstrap_check.sh` must be run from repo root (currently broken — no root Makefile)
- Windows cross-compilation requires `x86_64-w64-mingw32-gcc`
