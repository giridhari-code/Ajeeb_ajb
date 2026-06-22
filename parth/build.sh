#!/usr/bin/env bash
# parth build script — uses ajeebc to compile parth to a native binary
set -euo pipefail

ROOT="$(cd "$(dirname "$0")" && pwd)"
cd "$ROOT"

echo "=== Building Parth ==="

# Find ajeeb compiler
AJEEBC=""
if [ -n "${AJEEBC_PATH:-}" ] && [ -x "$AJEEBC_PATH" ]; then
    AJEEBC="$AJEEBC_PATH"
elif [ -x "${ROOT}/../build/ajeeb_native" ]; then
    AJEEBC="${ROOT}/../build/ajeeb_native"
elif [ -x "${ROOT}/../build/ajeebc" ]; then
    AJEEBC="${ROOT}/../build/ajeebc"
elif [ -x "${ROOT}/../ajeebc/build/compiler" ]; then
    AJEEBC="${ROOT}/../ajeebc/build/compiler"
elif command -v ajeebc &>/dev/null; then
    AJEEBC="$(command -v ajeebc)"
else
    echo "ajeebc nahi mila! Pehle ajeebc install karo."
    exit 1
fi

# Find runtime
RUNTIME=""
for candidate in \
    "${ROOT}/../ajeebc/runtime/ajeeb_runtime.c" \
    "${ROOT}/../runtime/ajeeb_runtime.c" \
    "${ROOT}/runtime/ajeeb_runtime.c"; do
    if [ -f "$candidate" ]; then
        RUNTIME="$candidate"
        break
    fi
done
if [ -z "$RUNTIME" ]; then
    echo "ajeeb_runtime.c nahi mila!"
    exit 1
fi

echo "  Compiling with: $AJEEBC"
echo "  Runtime: $RUNTIME"
mkdir -p build
"$AJEEBC" --emit-llvm-only "src/main.ajb" "build/parth.ll"
echo "  ✓ LLVM IR generated"

echo "  Assembling with llc..."
llc "build/parth.ll" -o "build/parth.s"
echo "  Linking with gcc..."
gcc -no-pie "build/parth.s" "$RUNTIME" \
    -o "build/parth" -lm -ldl -Wno-int-to-pointer-cast

echo ""
echo "✅ Parth build complete!"
echo "   ./build/parth init my-project"
