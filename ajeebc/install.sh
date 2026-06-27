#!/usr/bin/env bash
# Ajeeb install script — sirf ek baar chalao!
set -euo pipefail

ROOT="$(cd "$(dirname "$0")" && pwd)"
cd "$ROOT"

echo "Ajeeb install ho raha hai..."

# Step 1: ajeebc compiler ready hai?
echo ""
echo "=== Step 1: ajeebc compiler ==="
if [ ! -x "build/ajeebc" ]; then
  echo "❌ build/ajeebc nahi mila. Pehle 'make native' chalao."
  exit 1
fi
echo "  ✓ build/ajeebc ready"

# Step 2: Build native compiler (compiler.ajb → native)
echo ""
echo "=== Step 2: Native compiler (compiler.ajb) ==="
./build/ajeebc compiler/compiler.ajb --skip-run
echo "  ✓ build/compiler ready"

# Step 3: Build parth (Ajeeb package manager)
echo ""
echo "=== Step 3: parth (Ajeeb CLI) ==="
./build/ajeebc ../parth/parth_m1.ajb --skip-run
echo "  ✓ build/parth_m2 ready"

echo ""
echo "=================================================="
echo "Install complete! Ajeeb ready hai!"
echo "=================================================="
echo ""
echo "Use karo:"
echo "  ./build/parth_m2 init my-project"
echo "  cd my-project"
echo "  ../build/parth_m2 build      # Native binary banaye"
echo "  ../build/parth_m2 test       # Tests chalao"
echo ""
echo "Ya PATH mein daal do:"
echo "  export PATH=\"$ROOT/build:\$PATH\""
echo "  parth_m2 init demo && cd demo && parth_m2 build"
