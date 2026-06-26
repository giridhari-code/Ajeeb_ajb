#!/usr/bin/env bash
# parth build script — uses ajeebc to compile parth to a native binary
set -euo pipefail

ROOT="$(cd "$(dirname "$0")" && pwd)"
cd "$ROOT"

echo "=== Building Parth (Ajeeb-native) ==="

# Find ajeeb compiler
AJEEBC=""
if [ -n "${AJEEBC_PATH:-}" ] && [ -x "$AJEEBC_PATH" ]; then
    AJEEBC="$AJEEBC_PATH"
elif command -v ajeebc &>/dev/null; then
    AJEEBC="$(command -v ajeebc)"
elif [ -x "${ROOT}/../ajeebc/build/ajeebc" ]; then
    AJEEBC="${ROOT}/../ajeebc/build/ajeebc"
elif [ -x "${ROOT}/../build/ajeebc" ]; then
    AJEEBC="${ROOT}/../build/ajeebc"
else
    echo "ajeebc nahi mila! Pehle ajeebc install karo."
    echo "  curl -sSf https://raw.githubusercontent.com/giridhari-code/Ajeeb_ajb/main/scripts/install.sh | bash"
    exit 1
fi

# Find runtime
RUNTIME=""
for candidate in \
    "${HOME}/.ajeeb/bin/ajeeb_runtime.c" \
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

echo "  Compiler: $AJEEBC"
echo "  Runtime:  $RUNTIME"
mkdir -p build

echo "  Step 1: ajeebc → LLVM IR"
"$AJEEBC" "parth_m1.ajb" "build/output.ll" --emit-llvm-only
echo "  ✓ LLVM IR generated"

echo "  Step 2: llc → assembly"
llc -O2 "build/output.ll" -o "build/output.s"
echo "  ✓ Assembly generated"

echo "  Step 3: gcc → native binary"
gcc -no-pie "build/output.s" "$RUNTIME" \
    -o "build/parth_m2" -lm -ldl -Wno-int-to-pointer-cast 2>&1
echo "  ✓ Binary compiled"

echo ""
echo "✅ Parth build complete!"
echo "   ./build/parth_m2 init my-project"
echo "   cp build/parth_m2 ~/.ajeeb/bin/parth"
