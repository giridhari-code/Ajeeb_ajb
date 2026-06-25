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
"$AJEEBC" "parth_m1.ajb" "build/output.c"
echo "  ✓ C codegen generated"

# Add missing runtime declarations (mkdir, exec) that compiler doesn't emit
# Insert after the last #include line
sed -i '/#include <stdint.h>/a\intptr_t mkdir(intptr_t path);\nintptr_t exec(intptr_t cmd);' "build/output.c"
echo "  ✓ Patched runtime declarations"

echo "  Compiling C to binary with gcc..."
gcc -no-pie "build/output.c" "$RUNTIME" \
    -o "build/parth_m2" -lm -ldl -Wno-int-to-pointer-cast -Wno-pointer-to-int-cast 2>&1

echo ""
echo "✅ Parth build complete!"
echo "   ./build/parth init my-project"
