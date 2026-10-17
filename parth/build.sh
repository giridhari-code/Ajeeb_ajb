#!/usr/bin/env bash
# Build Parth package manager with ajeebc
set -euo pipefail
ROOT="$(cd "$(dirname "$0")" && pwd)"
cd "$ROOT"

echo "=== Building Parth (Ajeeb package manager) ==="

AJEEBC=""
if [ -n "${AJEEBC_PATH:-}" ] && [ -x "${AJEEBC_PATH}" ]; then
  AJEEBC="$AJEEBC_PATH"
elif command -v ajeebc &>/dev/null; then
  AJEEBC="$(command -v ajeebc)"
elif [ -x "${ROOT}/../ajeebc/build/compiler" ]; then
  AJEEBC="${ROOT}/../ajeebc/build/compiler"
elif [ -x "${ROOT}/../ajeebc/build/ajeebc" ]; then
  AJEEBC="${ROOT}/../ajeebc/build/ajeebc"
else
  echo "ajeebc not found. Build the compiler first: cd ajeebc && make native"
  exit 1
fi

RUNTIME=""
for c in \
  "${ROOT}/../ajeebc/runtime/ajeeb_runtime.c" \
  "${HOME}/.ajeeb/bin/ajeeb_runtime.c" \
  "${ROOT}/../runtime/ajeeb_runtime.c"; do
  if [ -f "$c" ]; then RUNTIME="$c"; break; fi
done
if [ -z "$RUNTIME" ]; then
  echo "ajeeb_runtime.c not found"
  exit 1
fi

echo "  Compiler: $AJEEBC"
echo "  Runtime:  $RUNTIME"
mkdir -p build

# Flatten modules for single-file compile (order: deps first)
FLAT="build/parth_all.ajb"
{
  sed '/^import /d' src/parser.ajb
  echo ""
  sed '/^import /d' src/resolver.ajb
  echo ""
  sed '/^import /d' src/builder.ajb
  echo ""
  sed '/^import /d' src/runner.ajb
  echo ""
  sed '/^import /d' src/main.ajb
} > "$FLAT"

"$AJEEBC" "$FLAT" --skip-run -o build/parth 2>&1 || {
  # fallback: compile then link with cc
  "$AJEEBC" "$FLAT" --skip-run 2>&1 || true
  if [ -f build/output.c ] || [ -f output.c ]; then
    CFILE=build/output.c
    [ -f output.c ] && CFILE=output.c
    cc "$CFILE" "$RUNTIME" -o build/parth -Wno-int-to-pointer-cast -Wno-pointer-to-int-cast -ldl
  else
    echo "Build failed"
    exit 1
  fi
}

chmod +x build/parth 2>/dev/null || true
echo ""
echo "Parth built: $ROOT/build/parth"
echo "  ./build/parth init my-app"
echo "  ./build/parth new my-app"
echo "  ./build/parth build"
echo "  ./build/parth run"
