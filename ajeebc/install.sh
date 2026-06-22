#!/usr/bin/env bash
# Ajeeb install script — sirf ek baar chalao!
set -euo pipefail

ROOT="$(cd "$(dirname "$0")" && pwd)"
cd "$ROOT"

echo "Ajeeb install ho raha hai..."

# Step 1: Build ajeebc (Rust compiler via rustc, no Cargo)
echo ""
echo "=== Step 1: ajeebc (Rust compiler) ==="
make rust
cp build/ajeeb_compiler build/ajeebc
echo "  ✓ build/ajeebc ready"

# Step 2: Build native compiler (compiler.ajb → native)
echo ""
echo "=== Step 2: Native compiler (compiler.ajb) ==="
./build/ajeebc compiler/compiler.ajb --skip-run
echo "  ✓ build/compiler ready"

# Step 3: Build parth (Ajeeb package manager — no Cargo!)
echo ""
echo "=== Step 3: parth (Ajeeb CLI) ==="
./build/ajeebc crates/parth/parth.ajb --skip-run
echo "  ✓ build/parth ready"

echo ""
echo "=================================================="
echo "Install complete! Ajeeb ready hai!"
echo "=================================================="
echo ""
echo "Use karo:"
echo "  ./build/parth init my-project"
echo "  cd my-project"
echo "  ../build/parth build      # Native binary banaye"
echo "  ../build/parth test       # Tests chalao"
echo ""
echo "Ya PATH mein daal do:"
echo "  export PATH=\"$ROOT/build:\$PATH\""
echo "  parth init demo && cd demo && parth build"
