#!/usr/bin/env bash
# Ajeeb — ek command mein install!
# curl -sSf https://raw.githubusercontent.com/giridhari-code/Ajeeb_ajb/main/install.sh | bash
set -euo pipefail

REPO="giridhari-code/Ajeeb_ajb"
BIN_DIR="${HOME}/.ajeeb/bin"
ARCH="linux-x86_64"

echo "================================================"
echo "  Ajeeb install ho raha hai..."
echo "================================================"
echo ""

# ── Dependency checks ──────────────────────────────
MISSING=""
if ! command -v gcc &>/dev/null && ! command -v clang &>/dev/null; then
    MISSING="${MISSING}  ❌ gcc/clang nahi mila! Pehle install karo:
       sudo apt install gcc          # Ubuntu/Debian
       sudo dnf install gcc          # Fedora
       sudo pacman -S gcc            # Arch
       brew install gcc              # macOS\n"
fi

if ! command -v llc &>/dev/null; then
    MISSING="${MISSING}  ❌ llc (LLVM) nahi mila! Pehle install karo:
       sudo apt install llvm         # Ubuntu/Debian
       sudo dnf install llvm         # Fedora
       sudo pacman -S llvm           # Arch
       brew install llvm             # macOS\n"
fi

if [ -n "$MISSING" ]; then
    echo "⚠️  Zaroorat hai:"
    echo -e "$MISSING"
    echo ""
    echo "Ajeeb binaries download ho jayenge, lekin compile tabhi karega"
    echo "jab upar ke tools available honge."
    echo "──────────────────────────────────────────────────────────"
fi

# ── Latest version check ───────────────────────────
echo "  Checking latest version..."
VERSION=$(curl -sSf "https://api.github.com/repos/${REPO}/releases/latest" \
    | grep '"tag_name"' | cut -d '"' -f 4 || echo "v0.1.1")

echo "  Version: ${VERSION}"
echo ""

mkdir -p "$BIN_DIR"

download() {
    local name="$1"
    local url="https://github.com/${REPO}/releases/download/${VERSION}/${name}-${ARCH}"
    local out="${BIN_DIR}/${name}"
    echo "  Downloading: ${name}..."
    curl -sSfL "$url" -o "$out" || {
        echo "  Error: ${name} download fail! Internet check karo."
        exit 1
    }
    chmod +x "$out"
    echo "  ✓ ${name}"
}

download "ajeebc"
ln -sf "${BIN_DIR}/ajeebc" "${BIN_DIR}/ajeeb_compiler" 2>/dev/null || true

download "parthi"
download "parth"

# ── Runtime jugaad ─────────────────────────────────
echo ""
echo "  Downloading runtime library..."
RUNTIME_URL="https://raw.githubusercontent.com/${REPO}/${VERSION}/ajeebc/runtime/ajeeb_runtime.c"
curl -sSfL "$RUNTIME_URL" -o "${BIN_DIR}/ajeeb_runtime.c" || {
    echo "  ⚠️  Runtime download fail — chalega to chalega lekin guarantee nahi"
}

# ── PATH setup ─────────────────────────────────────
if [[ ":$PATH:" != *":${BIN_DIR}:"* ]]; then
    echo "" >> "${HOME}/.bashrc"
    echo "# Ajeeb" >> "${HOME}/.bashrc"
    echo "export PATH=\"${BIN_DIR}:\$PATH\"" >> "${HOME}/.bashrc"
    echo "  ✓ Added to ~/.bashrc"
fi

echo ""
echo "================================================"
echo "  Install complete! 🎉"
echo "================================================"
echo ""
echo "Abhi ke liye chalao:"
echo "  export PATH=\"${BIN_DIR}:\$PATH\""
echo ""
echo "Phir use karo:"
echo "  ajeebc file.ajb              # compile → LLVM IR"
echo "  parthi file.ajb              # MIR interpreter se chalao"
echo "  parth init my-project        # naya project"
echo ""
echo "Pehli baar? Ye karo:"
echo "  parth init hello-ajeeb"
echo "  cd hello-ajeeb"
echo "  parth run"
