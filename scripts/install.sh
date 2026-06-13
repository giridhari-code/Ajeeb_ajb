#!/usr/bin/env bash
# Install Ajeeb self-hosted compiler
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

echo "🔨 Building Ajeeb self-hosted compiler..."

# Stage 0: Bootstrap with Rust
# Args: [source to interpret] [input for compiler.ajb] [output file]
cargo run -p ajeeb-compiler --bin ajeeb_compiler -- \
    compiler/compiler.ajb compiler/compiler.ajb build/output.c

# Stage 1: Compile to native binary
gcc build/output.c runtime/ajeeb_runtime.c \
    -o build/ajeeb_native \
    -Wall -Wno-int-to-pointer-cast \
    -Wno-pointer-to-int-cast

echo "✅ Self-hosted compiler ready: build/ajeeb_native"
echo ""
echo "Usage:"
echo "  ./build/ajeeb_native your_program.ajb build/output.c"
echo "  gcc build/output.c runtime/ajeeb_runtime.c -o your_program"
echo "  ./your_program"
