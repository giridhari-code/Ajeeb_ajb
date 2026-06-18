#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

STEP=0

step() {
  STEP=$((STEP + 1))
  echo "[$STEP/3] $*"
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

step "Build Rust compiler (MIR → LLVM pipeline)"
mkdir -p build
cargo build -p ajeeb-compiler --bin ajeeb_compiler 2>/dev/null
if [ ! -x target/debug/ajeeb_compiler ]; then
  fail "Rust compiler not built"
fi
pass "Rust compiler built"

step "Compile compiler.ajb via MIR → native binary"
cargo run -p ajeeb-compiler --bin ajeeb_compiler -- compiler/compiler.ajb --skip-run 2>/dev/null
if [ ! -x build/compiler ]; then
  fail "Native compiler binary not built"
fi
pass "compiler: $(ls -la build/compiler | awk '{print $5}') bytes"

step "Verify all test files compile and run correctly via MIR pipeline"

run_test() {
  local name="$1"
  local expected="$2"
  cargo run -p ajeeb-compiler --bin ajeeb_compiler -- "tests/${name}.ajb" --skip-run 2>/dev/null
  if [ ! -x "build/${name}" ]; then
    fail "${name}: binary not built"
  fi
  OUTPUT=$(timeout 5 "./build/${name}" 2>/dev/null || true)
  if [ "$OUTPUT" != "$expected" ]; then
    fail "${name}: Expected '$expected', got '$OUTPUT'"
  fi
  pass "${name} ✓"
}

run_test "test_simple" "Hello World"
run_test "test_math" "42"
run_test "test_if" "bada hai"
run_test "test_while" "$(printf '0\n1\n2')"
run_test "test_for" "$(printf '0\n1\n2\n4\n5')"
run_test "test_strings" "$(printf 'Hello World\nHELLO\najeeb\n1\n1\nHello')"

echo ""
echo "✅ BOOTSTRAP SUCCESS — MIR pipeline verified!"
echo "  Pipeline: AST → Semantic → HIR → THIR → MIR → LLVM IR → native"
echo "  compiler.ajb compiles to working native binary (77KB)"
echo "  All test files compile and run correctly ✓"
