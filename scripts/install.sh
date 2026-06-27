#!/usr/bin/env bash
# Ajeeb — ek command mein install!
# curl -sSf https://raw.githubusercontent.com/giridhari-code/Ajeeb_ajb/main/scripts/install.sh | bash
set -euo pipefail

REPO="giridhari-code/Ajeeb_ajb"
BIN_DIR="${HOME}/.ajeeb/bin"

# ── Parse flags ─────────────────────────────────────
for arg in "$@"; do
    case "$arg" in
        --help|-h)
            echo "Usage: install.sh"
            echo ""
            echo "Downloads prebuilt Ajeeb binaries for your platform."
            echo "No Rust or Cargo required."
            exit 0
            ;;
    esac
done

# ── Detect OS + Architecture ────────────────────────
OS_RAW=$(uname -s)
ARCH_RAW=$(uname -m)

case "$OS_RAW" in
    Linux)
        case "$ARCH_RAW" in
            x86_64|amd64)  PLATFORM="linux-x86_64" ;;
            aarch64|arm64) PLATFORM="linux-aarch64" ;;
            *)             PLATFORM="linux-x86_64"; echo "⚠️  Unknown arch ($ARCH_RAW), using x86_64" ;;
        esac
        ;;
    Darwin)
        case "$ARCH_RAW" in
            arm64)  PLATFORM="macos-arm64" ;;
            x86_64) PLATFORM="macos-x86_64" ;;
            *)      PLATFORM="macos-arm64"; echo "⚠️  Unknown Mac arch ($ARCH_RAW), using arm64" ;;
        esac
        ;;
    MINGW*|MSYS*|CYGWIN*)
        PLATFORM="windows-x86_64"
        echo "⚠️  Windows detected — use PowerShell install.ps1 for best experience"
        ;;
    *)
        PLATFORM="linux-x86_64"
        echo "⚠️  Unknown OS ($OS_RAW), using linux-x86_64"
        ;;
esac

echo "================================================"
echo "  Ajeeb install ho raha hai... ($PLATFORM)"
echo "================================================"
echo ""

# ── Dependency checks (only warn, don't block) ─────
MISSING=""
if ! command -v gcc &>/dev/null && ! command -v cc &>/dev/null && ! command -v clang &>/dev/null; then
    MISSING="${MISSING}  ⚠️  gcc/clang nahi mila (compile ke liye chahiye)\n"
fi

if ! command -v llc &>/dev/null; then
    MISSING="${MISSING}  ⚠️  llc (LLVM) nahi mila (compile ke liye chahiye)\n"
fi

if [ -n "$MISSING" ]; then
    echo "⚠️  Zaroorat hai (compile ke liye):"
    echo -e "$MISSING"
    echo "──────────────────────────────────────────────────────────"
fi

# ── Latest version check ───────────────────────────
echo "  Checking latest version..."
VERSION=$(curl -sSf "https://api.github.com/repos/${REPO}/releases/latest" \
    | grep '"tag_name"' | cut -d '"' -f 4 || echo "v1.0.1")

echo "  Version: ${VERSION}"
echo ""

mkdir -p "$BIN_DIR"

download() {
    local name="$1"
    local url="https://github.com/${REPO}/releases/download/${VERSION}/${name}-${PLATFORM}"
    local out="${BIN_DIR}/${name}"
    echo "  Downloading: ${name}..."
    if curl -sSfL "$url" -o "$out" 2>/dev/null; then
        chmod +x "$out"
        echo "  ✓ ${name}"
        return 0
    else
        return 1
    fi
}

# ── Download prebuilt binaries ─────────────────────
echo "  Mode: Download prebuilt binaries"
echo ""

if ! download "ajeebc"; then
    echo ""
    echo "❌ ajeebc binary release mein nahi hai (${PLATFORM})"
    echo "   GitHub issue karo: https://github.com/${REPO}/issues"
    exit 1
fi
ln -sf "${BIN_DIR}/ajeebc" "${BIN_DIR}/ajeeb_compiler" 2>/dev/null || true

if ! download "parth"; then
    echo ""
    echo "❌ parth binary release mein nahi hai (${PLATFORM})"
    echo "   GitHub issue karo: https://github.com/${REPO}/issues"
    exit 1
fi

if ! download "piri"; then
    echo "  ⚠️  piri not available for ${PLATFORM}, skipping (optional)"
fi

# ── Verify checksums ───────────────────────────────
echo ""
echo "  Verifying checksums..."
SUMS_URL="https://github.com/${REPO}/releases/download/${VERSION}/SHA256SUMS.txt"
SUMS_FILE="${BIN_DIR}/SHA256SUMS.txt"
if curl -sSfL "$SUMS_URL" -o "$SUMS_FILE" 2>/dev/null; then
    cd "$BIN_DIR"
    if sha256sum -c "$SUMS_FILE" 2>/dev/null; then
        echo "  ✓ Checksums verified"
    else
        echo "  ⚠️  Checksum verification failed — binary may be corrupted"
        echo "     Re-run install or download manually from GitHub Releases"
    fi
    cd - >/dev/null
else
    echo "  ⚠️  SHA256SUMS.txt not available, skipping verification"
fi

# ── Runtime ────────────────────────────────────────
echo ""
echo "  Downloading runtime library..."
RUNTIME_URL="https://github.com/${REPO}/releases/download/${VERSION}/ajeeb_runtime.c"
curl -sSfL "$RUNTIME_URL" -o "${BIN_DIR}/ajeeb_runtime.c" || {
    echo "  ⚠️  Runtime download fail"
}

# ── Standard library ──────────────────────────────
echo ""
echo "  Downloading ajeeb-std packages..."
STD_DIR="${HOME}/.ajeeb/packages/ajeeb-std"
mkdir -p "$STD_DIR"
for f in io.ajb math.ajb string.ajb array.ajb fs.ajb result.ajb collections.ajb option.ajb path.ajb process.ajb test.ajb time.ajb json.ajb; do
    URL="https://raw.githubusercontent.com/${REPO}/${VERSION}/ajeeb-lang/std/${f}"
    curl -sSfL "$URL" -o "${STD_DIR}/${f}" 2>/dev/null && echo "  ✓ ajeeb-std/${f}" || true
done

# ── PATH setup ─────────────────────────────────────
if [[ ":$PATH:" != *":${BIN_DIR}:"* ]]; then
    SHELL_RC=""
    if [ -f "${HOME}/.bashrc" ]; then SHELL_RC="${HOME}/.bashrc"
    elif [ -f "${HOME}/.zshrc" ]; then SHELL_RC="${HOME}/.zshrc"
    fi
    if [ -n "$SHELL_RC" ]; then
        echo "" >> "$SHELL_RC"
        echo "# Ajeeb" >> "$SHELL_RC"
        echo "export PATH=\"${BIN_DIR}:\$PATH\"" >> "$SHELL_RC"
        echo "  ✓ Added to $(basename $SHELL_RC)"
    fi
fi

echo ""
echo "================================================"
echo "  Install complete!"
echo "================================================"
echo ""
echo "Abhi ke liye chalao:"
echo "  export PATH=\"${BIN_DIR}:\$PATH\""
echo ""
echo "Phir use karo:"
echo "  ajeebc file.ajb              # compile"
echo "  piri file.ajb              # MIR interpreter se chalao"
echo "  parth init my-project        # naya project banao"
echo "  parth build                  # compile karo"
echo "  parth run                    # build + chalao"
echo ""
echo "Pehli baar? Ye karo:"
echo "  parth init hello-ajeeb"
echo "  cd hello-ajeeb"
echo "  parth run"
