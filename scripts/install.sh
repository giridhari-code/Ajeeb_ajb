#!/usr/bin/env bash
# Ajeeb — ek command mein install!
# curl -sSf https://raw.githubusercontent.com/giridhari-code/Ajeeb_ajb/main/scripts/install.sh | bash
set -euo pipefail

REPO="giridhari-code/Ajeeb_ajb"
BIN_DIR="${HOME}/.ajeeb/bin"
BUILD_FROM_SOURCE=0

# ── Parse flags ─────────────────────────────────────
for arg in "$@"; do
    case "$arg" in
        --build-from-source) BUILD_FROM_SOURCE=1 ;;
        --help|-h)
            echo "Usage: install.sh [--build-from-source]"
            echo ""
            echo "  (default)  Download prebuilt binaries for your platform"
            echo "  --build-from-source  Build from source using Rust/Cargo"
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
            armv7l|armhf)  PLATFORM="linux-armv7" ;;
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
    MISSING="${MISSING}  ⚠️  gcc/clang nahi mila\n"
fi

if ! command -v llc &>/dev/null; then
    MISSING="${MISSING}  ⚠️  llc (LLVM) nahi mila\n"
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
        echo "  ⚠️  ${name} (${PLATFORM}) release mein nahi hai"
        return 1
    fi
}

# ── Download prebuilt binaries (default) ───────────
if [ "$BUILD_FROM_SOURCE" -eq 0 ]; then
    echo "  Mode: Download prebuilt binaries"
    echo ""

    if ! download "ajeebc"; then
        echo ""
        echo "❌ ajeebc binary not available for ${PLATFORM}"
        echo "   To build from source, re-run with:"
        echo "     curl -sSf .../install.sh | bash -s -- --build-from-source"
        exit 1
    fi
    ln -sf "${BIN_DIR}/ajeebc" "${BIN_DIR}/ajeeb_compiler" 2>/dev/null || true

    if ! download "parthi"; then
        echo "  ⚠️  parthi not available for ${PLATFORM}, skipping"
    fi

    if ! download "parth"; then
        echo "  ⚠️  parth not available for ${PLATFORM}, skipping"
    fi

# ── Build from source (explicit flag) ──────────────
else
    echo "  Mode: Build from source"
    echo ""

    if ! command -v cargo &>/dev/null; then
        echo "❌ --build-from-source requires cargo (Rust toolchain)"
        echo "   Install Rust: curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
        exit 1
    fi

    echo "  Building ajeebc from source (cargo)..."
    TMPDIR=$(mktemp -d)
    git clone --depth 1 "https://github.com/${REPO}.git" "$TMPDIR" 2>/dev/null
    if cd "$TMPDIR/ajeebc/crates/ajeeb-compiler" && cargo build --release; then
        cp target/release/ajeeb_compiler "${BIN_DIR}/ajeebc"
        echo "  ✓ ajeebc (built from source)"
    else
        echo "  ❌ cargo build fail — Rust toolchain check karo"
        cd / && rm -rf "$TMPDIR"
        exit 1
    fi
    cd / && rm -rf "$TMPDIR"
    ln -sf "${BIN_DIR}/ajeebc" "${BIN_DIR}/ajeeb_compiler" 2>/dev/null || true

    if command -v cargo &>/dev/null && [ -f "${BIN_DIR}/ajeebc" ]; then
        echo "  Building parthi from source..."
        TMPDIR=$(mktemp -d)
        git clone --depth 1 "https://github.com/${REPO}.git" "$TMPDIR" 2>/dev/null
        cd "$TMPDIR" && AJEEBC_PATH="${BIN_DIR}/ajeebc" bash parthi/build.sh
        cp parthi/build/parthi "${BIN_DIR}/parthi"
        cd / && rm -rf "$TMPDIR"
        echo "  ✓ parthi (built from source)"
    else
        echo "  ⚠️  parthi skip — cargo ya ajeebc nahi hai"
    fi

    if command -v cargo &>/dev/null; then
        echo "  Building parth from source..."
        TMPDIR=$(mktemp -d)
        git clone --depth 1 "https://github.com/${REPO}.git" "$TMPDIR" 2>/dev/null
        cd "$TMPDIR/ajeebc/crates/parth" && cargo build --release
        cp target/release/parth "${BIN_DIR}/parth"
        cd / && rm -rf "$TMPDIR"
        echo "  ✓ parth (built from source)"
    else
        echo "  ⚠️  parth skip — cargo nahi hai"
    fi
fi

# ── Runtime ────────────────────────────────────────
echo ""
echo "  Downloading runtime library..."
RUNTIME_URL="https://raw.githubusercontent.com/${REPO}/${VERSION}/ajeebc/runtime/ajeeb_runtime.c"
curl -sSfL "$RUNTIME_URL" -o "${BIN_DIR}/ajeeb_runtime.c" || {
    echo "  ⚠️  Runtime download fail"
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
echo "  parthi file.ajb              # MIR interpreter se chalao"
echo "  parth init my-project        # naya project banao"
echo "  parth build                  # compile karo"
echo "  parth run                    # build + chalao"
echo ""
echo "Pehli baar? Ye karo:"
echo "  parth init hello-ajeeb"
echo "  cd hello-ajeeb"
echo "  parth run"
