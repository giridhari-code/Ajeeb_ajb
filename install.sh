#!/usr/bin/env bash
# Ajeeb — ek command mein install!
# curl -sSf https://raw.githubusercontent.com/giridhari-code/Ajeeb_ajb/main/install.sh | bash
set -euo pipefail

REPO="giridhari-code/Ajeeb_ajb"
BIN_DIR="${HOME}/.ajeeb/bin"

# Auto-detect architecture
ARCH_RAW=$(uname -m)
case "$ARCH_RAW" in
    x86_64|amd64)  ARCH="linux-x86_64" ;;
    aarch64|arm64) ARCH="linux-aarch64" ;;
    armv7l|armhf)  ARCH="linux-armv7" ;;
    *)             ARCH="linux-x86_64"; echo "⚠️  Unknown arch ($ARCH_RAW), using x86_64" ;;
esac

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
    if curl -sSfL "$url" -o "$out" 2>/dev/null; then
        chmod +x "$out"
        echo "  ✓ ${name}"
        return 0
    else
        echo "  ⚠️  ${name} (${ARCH}) release mein nahi hai"
        return 1
    fi
}

# Download or build from source
BUILT_FROM_SOURCE=""
if ! download "ajeebc"; then
    if command -v cargo &>/dev/null; then
        echo "  Building ajeebc from source..."
        TMPDIR=$(mktemp -d)
        git clone --depth 1 "https://github.com/${REPO}.git" "$TMPDIR" 2>/dev/null
        cd "$TMPDIR/ajeebc/crates/ajeeb-compiler" && cargo build --release 2>/dev/null
        cp target/release/ajeeb_compiler "${BIN_DIR}/ajeebc"
        cd / && rm -rf "$TMPDIR"
        echo "  ✓ ajeebc (built from source)"
    else
        echo "  ❌ ajeebc nahi mila aur cargo bhi nahi hai. Install Rust: curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
        exit 1
    fi
fi
ln -sf "${BIN_DIR}/ajeebc" "${BIN_DIR}/ajeeb_compiler" 2>/dev/null || true

if ! download "parthi"; then
    if command -v cargo &>/dev/null && [ -f "${BIN_DIR}/ajeebc" ]; then
        echo "  Building parthi from source..."
        TMPDIR=$(mktemp -d)
        git clone --depth 1 "https://github.com/${REPO}.git" "$TMPDIR" 2>/dev/null
        cd "$TMPDIR" && AJEEBC_PATH="${BIN_DIR}/ajeebc" bash parthi/build.sh 2>/dev/null
        cp parthi/build/parthi "${BIN_DIR}/parthi"
        cd / && rm -rf "$TMPDIR"
        echo "  ✓ parthi (built from source)"
    else
        echo "  ⚠️  parthi skip — cargo ya ajeebc nahi hai"
    fi
fi

if ! download "parth"; then
    if command -v cargo &>/dev/null; then
        echo "  Building parth from source..."
        TMPDIR=$(mktemp -d)
        git clone --depth 1 "https://github.com/${REPO}.git" "$TMPDIR" 2>/dev/null
        cd "$TMPDIR/ajeebc/crates/parth" && cargo build --release 2>/dev/null
        cp target/release/parth "${BIN_DIR}/parth"
        cd / && rm -rf "$TMPDIR"
        echo "  ✓ parth (built from source)"
    else
        echo "  ⚠️  parth skip — cargo nahi hai"
    fi
fi

# ── Runtime jugaad ─────────────────────────────────
echo ""
echo "  Downloading runtime library..."
RUNTIME_URL="https://raw.githubusercontent.com/${REPO}/${VERSION}/ajeebc/runtime/ajeeb_runtime.c"
curl -sSfL "$RUNTIME_URL" -o "${BIN_DIR}/ajeeb_runtime.c" || {
    echo "  ⚠️  Runtime download fail — chalega to chalega lekin guarantee nahi"
}

# ── Standard library ──────────────────────────────
echo ""
echo "  Downloading ajeeb-std packages..."
STD_DIR="${BIN_DIR}/../packages/ajeeb-std"
mkdir -p "$STD_DIR"
for f in io.ajb math.ajb string.ajb array.ajb fs.ajb result.ajb collections.ajb; do
    URL="https://raw.githubusercontent.com/${REPO}/${VERSION}/packages/ajeeb-std/${f}"
    curl -sSfL "$URL" -o "${STD_DIR}/${f}" 2>/dev/null && echo "  ✓ ajeeb-std/${f}" || true
done

# ── Default parth.das template ────────────────────
cat > "${BIN_DIR}/parth.das.template" << 'DASTPL'
[package]
name = "my-project"
version = "0.1.0"
author = ""

[dependencies]

[compiler]
target = "native"
output = "build/"
runtime = "runtime/ajeeb_runtime.c"
DASTPL
echo "  ✓ parth.das template"

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
echo "  ajeebc file.ajb              # compile → LLVM IR + native binary"
echo "  parthi file.ajb              # MIR interpreter se chalao"
echo "  parth init my-project        # naya project banao"
echo "  parth build                  # compile karo (native target)"
echo "  parth run                    # build + chalao"
echo "  parth generate-lockfile      # lock file banao"
echo ""
echo "Pehli baar? Ye karo:"
echo "  parth init hello-ajeeb"
echo "  cd hello-ajeeb"
echo "  parth run"
