#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

STEP=0

step() {
  STEP=$((STEP + 1))
  echo "[$STEP/4] $*"
}

pass() {
  echo "  ✓ $*"
}

fail() {
  echo "  ✗ $*"
  echo ""
  echo "❌ BOOTSTRAP FAILED at step $STEP"
  exit 1
}

step "Build Rust interpreter and compile compiler.ajb → output.c"
mkdir -p build
cargo run -p ajeeb-compiler --bin ajeeb_compiler \
  compiler/compiler.ajb compiler/compiler.ajb build/output.c 2>/dev/null
if [ ! -s build/output.c ]; then
  fail "output.c is empty or missing"
fi
pass "output.c: $(wc -l < build/output.c) lines, $(wc -c < build/output.c) bytes"

step "Compile output.c with gcc → compiler_v1 (build/ajeeb_native)"
gcc -o build/ajeeb_native build/output.c runtime/ajeeb_runtime.c \
    -Wall -Wno-int-to-pointer-cast -Wno-pointer-to-int-cast 2>/dev/null
if [ ! -x build/ajeeb_native ]; then
  fail "compiler_v1 binary not built"
fi
pass "compiler_v1: $(ls -la build/ajeeb_native | awk '{print $5}') bytes"

step "compiler_v1 compiles compiler.ajb → output2.c"
./build/ajeeb_native compiler/compiler.ajb build/output2.c 2>/dev/null
if [ ! -s build/output2.c ]; then
  fail "output2.c is empty or missing"
fi
pass "output2.c: $(wc -l < build/output2.c) lines, $(wc -c < build/output2.c) bytes"

step "Verify Stage 1 and Stage 2 output are identical"
if ! diff build/output.c build/output2.c >/dev/null 2>&1; then
  fail "output.c and output2.c differ"
fi
HASH1=$(sha256sum build/output.c | awk '{print $1}')
HASH2=$(sha256sum build/output2.c | awk '{print $1}')
if [ "$HASH1" != "$HASH2" ]; then
  fail "SHA256 mismatch"
fi
pass "SHA256: $HASH1 (identical)"

echo ""
echo "✅ BOOTSTRAP SUCCESS — Self-hosting verified!"
echo "  Stage 1: Rust interpreter → compiler_v1"
echo "  Stage 2: compiler_v1 → compiler_v2"
echo "  output.c ≡ output2.c ✓"
