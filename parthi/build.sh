#!/usr/bin/env bash
# parthi build script — Miri-inspired modular architecture
# Concatenates module files in order, then compiles via ajeebc

set -euo pipefail

ROOT="$(cd "$(dirname "$0")" && pwd)"
cd "$ROOT"

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

# Step 2: Compile with ajeebc (Rust stage-0 → LLVM)
echo "  Compiling with ajeebc..."
AJEEBC="${ROOT}/../ajeebc/build/ajeebc"
if [ ! -x "$AJEEBC" ]; then
    echo "  ajeebc not found at $AJEEBC — building it first..."
    (cd "${ROOT}/../ajeebc" && make rust 2>/dev/null)
fi

"$AJEEBC" "$COMBINED" "build/parthi.ll" 2>/dev/null
echo "  ✓ LLVM IR generated"

# Step 3: Assemble + link
echo "  Assembling with llc..."
llc "build/parthi.ll" -o "build/parthi.s"
echo "  Linking with gcc..."
gcc "build/parthi.s" "${ROOT}/../ajeebc/runtime/ajeeb_runtime.c" \
    -o "build/parthi" -lm -ldl -Wno-int-to-pointer-cast

echo ""
echo "✅ ParthI build complete!"
echo "   ./build/parthi <file.ajb>"
