#!/usr/bin/env bash
# parthi build script — Miri-inspired modular architecture
# Concatenates module files in order, then compiles via ajeebc
# Usage:
#   bash build.sh                 — full build (LLIR → native binary)
#   bash build.sh --emit-llvm-only — generate LLVM IR only

set -euo pipefail

ROOT="$(cd "$(dirname "$0")" && pwd)"
cd "$ROOT"

EMIT_IR_ONLY=false
for arg in "$@"; do
    [ "$arg" = "--emit-llvm-only" ] && EMIT_IR_ONLY=true
done

# Ensure llc/gcc are on PATH (Homebrew LLVM on macOS)
if [ "$(uname)" = "Darwin" ]; then
    for p in /opt/homebrew/opt/llvm/bin /usr/local/opt/llvm/bin; do
        [ -d "$p" ] && export PATH="$p:$PATH"
    done
fi

echo "=== Building ParthI (modular) ==="

# Step 1: Concatenate modules in dependency order
COMBINED="build/parthi_combined.ajb"
mkdir -p build
rm -f "$COMBINED"

echo "// ParthI — concatenated modules" > "$COMBINED"
echo "// Build: $(date)" >> "$COMBINED"
echo "" >> "$COMBINED"

for module in core value diagnostics builtins lexer hir parser mir eval main; do
    SRC="src/${module}.ajb"
    if [ -f "$SRC" ]; then
        echo "  Adding: $module"
        echo "// === Module: ${module}.ajb ===" >> "$COMBINED"
        cat "$SRC" >> "$COMBINED"
        echo "" >> "$COMBINED"
    else
        echo "  ⚠️  Warning: $SRC not found, skipping"
    fi
done

echo "  ✓ Combined file: $(wc -l < "$COMBINED") lines"

# Step 2: Find ajeebc
AJEEBC="${ROOT}/../ajeebc/build/ajeebc"
if [ ! -x "$AJEEBC" ]; then
    echo "  ajeebc not found at $AJEEBC — cannot build"
    exit 1
fi

# Step 3: Compile with ajeebc → LLVM IR
echo "  Compiling with ajeebc..."
"$AJEEBC" "$COMBINED" "build/parthi.ll" --emit-llvm-only
echo "  ✓ LLVM IR generated"

if [ "$EMIT_IR_ONLY" = true ]; then
    echo ""
    echo "✅ ParthI IR generation complete!"
    echo "   build/parthi.ll"
    exit 0
fi

# Step 4: Assemble + link (native build)
echo "  Assembling with llc..."
llc "build/parthi.ll" -o "build/parthi.s"
echo "  Linking with gcc..."
gcc "build/parthi.s" "${ROOT}/../ajeebc/runtime/ajeeb_runtime.c" \
    -o "build/parthi" -lm -ldl -Wno-int-to-pointer-cast

echo ""
echo "✅ ParthI build complete!"
echo "   ./build/parthi <file.ajb>"
