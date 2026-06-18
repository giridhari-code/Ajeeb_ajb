#!/usr/bin/env bash
# Ajeeb install script — sirf ek baar chalao!
set -euo pipefail

ROOT="$(cd "$(dirname "$0")" && pwd)"
cd "$ROOT"

echo "Ajeeb install ho raha hai..."

# Step 1: Build ajeebc (Rust compiler → LLVM)
echo ""
echo "=== Step 1: ajeebc (Rust compiler) ==="
cargo build --release -p ajeeb-compiler
cp target/release/ajeeb_compiler build/ajeebc
echo "  ✓ build/ajeebc ready"

# Step 2: Build parthi (ParthI interpreter)
echo ""
echo "=== Step 2: parthi (MIR Interpreter) ==="
echo "  Compiling ParthI with ajeebc..."
./build/ajeebc parthi/src/main.ajb build/parthi.ll
echo "  Assembling with llc..."
llc build/parthi.ll -o build/parthi.s
echo "  Linking with gcc..."
gcc build/parthi.s runtime/ajeeb_runtime.c -o build/parthi -lm -ldl
echo "  ✓ build/parthi ready"

# Step 3: Build parth (Rust CLI)
echo ""
echo "=== Step 3: parth (Rust CLI) ==="
cargo build --release -p parth
cp target/release/parth build/parth
echo "  ✓ build/parth ready"

echo ""
echo "=================================================="
echo "Install complete! Ajeeb ready hai!"
echo "=================================================="
echo ""
echo "Use karo:"
echo "  ./build/parth init my-project"
echo "  cd my-project"
echo "  ../build/parth run        # ParthI se interpret"
echo "  ../build/parth build      # Native binary banaye"
echo "  ../build/parth test       # Tests chalao"
echo ""
echo "Ya PATH mein daal do:"
echo "  export PATH=\"$ROOT/build:\$PATH\""
echo "  parth init demo && cd demo && parth run"
