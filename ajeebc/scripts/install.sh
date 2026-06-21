#!/usr/bin/env bash
# Install Ajeeb self-hosted compiler (LLVM pipeline)
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

echo "🔨 Building Ajeeb self-hosted compiler (LLVM pipeline)..."

# Check dependencies
if ! command -v llc &>/dev/null; then
    echo "❌ llc (LLVM) nahi mila! Pehle install karo:"
    echo "   sudo apt install llvm    # Ubuntu/Debian"
    echo "   sudo dnf install llvm    # Fedora"
    echo "   sudo pacman -S llvm      # Arch"
    echo "   brew install llvm        # macOS"
    exit 1
fi

if ! command -v gcc &>/dev/null && ! command -v cc &>/dev/null; then
    echo "❌ gcc/cc nahi mila! Pehle install karo."
    exit 1
fi

mkdir -p build

# Stage 0: Bootstrap with Rust → LLVM IR
echo "  [1/4] Rust compiler: compiler.ajb → LLVM IR"
cargo run -p ajeeb-compiler --bin ajeeb_compiler -- \
    compiler/compiler.ajb build/output.ll --skip-run 2>/dev/null

if [ ! -f build/output.ll ]; then
    echo "❌ LLVM IR generation failed"
    exit 1
fi
echo "  ✓ LLVM IR generated"

# Stage 1: llc — LLVM IR → Assembly
echo "  [2/4] llc: LLVM IR → Assembly"
llc -O2 build/output.ll -o build/output.s
echo "  ✓ Assembly generated"

# Stage 2: as — Assembly → Object
echo "  [3/4] as: Assembly → Object"
as build/output.s -o build/output.o
echo "  ✓ Object generated"

# Stage 3: link with runtime → native binary
echo "  [4/4] cc: Object + Runtime → Native binary"
cc build/output.o runtime/ajeeb_runtime.c \
    -o build/ajeeb_native \
    -lm -ldl
echo "  ✓ Linked"

echo ""
echo "✅ Self-hosted compiler ready: build/ajeeb_native"
echo ""
echo "Usage:"
echo "  ./build/ajeeb_native your_program.ajb    # compile via LLVM"
echo "  ./your_program"
