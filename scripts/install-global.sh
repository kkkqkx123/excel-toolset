#!/usr/bin/env bash
# install-global.sh
# Install excel-diff binary and configure it as the global git diff driver
# for all Excel files.
#
# Usage:
#   ./scripts/install-global.sh [<prefix>]
#
# Binary lookup order (first match wins):
#   1. Same directory as this script (pre-built in release archive)
#   2. Project target/release/ (built locally via cargo)
#   3. System PATH (already installed somewhere)
#   4. Build from source via cargo (fallback)
#
# What this does:
#   1. Locates or builds the excel-diff binary
#   2. Installs the binary to <prefix>/bin (default: /usr/local/bin)
#   3. Configures git globally to use excel-diff as the diff driver for
#      *.xlsx, *.xls, *.xlsm, *.xlsb files
#
# After installation, every git repository on the system will use structured
# text output for Excel file diffs (instead of binary garbage).
#
# No Rust compilation is needed if a pre-built binary is found.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"
PREFIX="${1:-/usr/local}"
BINARY_NAME="excel-diff"
GITATTR_PATH="$HOME/.config/git/attributes"

echo "=== excel-diff global git diff driver installation ==="
echo ""

# --- Step 1: Locate or build the binary ---

BIN_SRC=""

# Priority 1: same directory as this script (pre-built in release archive)
if [ -f "$SCRIPT_DIR/$BINARY_NAME" ]; then
    BIN_SRC="$SCRIPT_DIR/$BINARY_NAME"
    echo "[1/3] Found pre-built binary in script directory"

# Priority 2: project target/release/
elif [ -f "$PROJECT_DIR/target/release/$BINARY_NAME" ]; then
    BIN_SRC="$PROJECT_DIR/target/release/$BINARY_NAME"
    echo "[1/3] Found release binary in project build directory"

# Priority 3: system PATH
elif command -v "$BINARY_NAME" &>/dev/null; then
    BIN_SRC="$(command -v "$BINARY_NAME")"
    echo "[1/3] Found binary on system PATH: $BIN_SRC"

# Priority 4: build from source
else
    echo "[1/3] No pre-built binary found, building from source (release mode)..."
    if ! command -v cargo &>/dev/null; then
        echo "ERROR: cargo not found. Please install Rust toolchain or provide a pre-built excel-diff binary."
        exit 1
    fi
    cd "$PROJECT_DIR"
    cargo build --package excel-diff --release
    BIN_SRC="$PROJECT_DIR/target/release/$BINARY_NAME"
    echo "  Build complete"
fi

if [ ! -f "$BIN_SRC" ]; then
    echo "ERROR: Binary not found at $BIN_SRC"
    exit 1
fi

echo "  Source: $BIN_SRC"
echo ""

# --- Step 2: Install binary to prefix/bin ---

echo "[2/3] Installing excel-diff to ${PREFIX}/bin/..."
BIN_DST="${PREFIX}/bin/${BINARY_NAME}"

install_bin() {
    if [ "$(id -u)" -eq 0 ]; then
        cp "$BIN_SRC" "$BIN_DST"
        chmod 755 "$BIN_DST"
    else
        sudo cp "$BIN_SRC" "$BIN_DST" || {
            echo "ERROR: Cannot install to ${PREFIX}/bin/. Run with sudo or specify a writable prefix."
            exit 1
        }
        sudo chmod 755 "$BIN_DST"
    fi
}

# Skip copy if source and destination are the same file
if [ "$(realpath "$BIN_SRC" 2>/dev/null || echo "$BIN_SRC")" = "$(realpath "$BIN_DST" 2>/dev/null || echo "$BIN_DST")" ]; then
    echo "  Binary already at destination, skipping copy"
else
    install_bin
    echo "  Installed: $BIN_DST"
fi

# Verify binary is accessible
if ! command -v "$BINARY_NAME" &>/dev/null; then
    echo "  WARNING: excel-diff not found on PATH. You may need to add ${PREFIX}/bin to your PATH."
fi
echo ""

# --- Step 3: Configure global git diff driver (pure bash, no binary needed) ---

echo "[3/3] Configuring global git diff driver..."

# Ensure the global gitattributes directory exists
mkdir -p "$(dirname "$GITATTR_PATH")"

# Write gitattributes entries for all supported Excel extensions
# Only append if the pattern isn't already present
for ext in xlsx xls xlsm xlsb; do
    if ! grep -q "^\*\.${ext} diff=excel-diff" "$GITATTR_PATH" 2>/dev/null; then
        echo "*.${ext} diff=excel-diff" >> "$GITATTR_PATH"
    fi
done

# Set the diff driver command: excel-diff git-driver
git config --global diff.excel-diff.command "excel-diff git-driver"

# Point git to the global attributes file
CURRENT_ATTR="$(git config --global core.attributesfile 2>/dev/null || echo "")"
if [ "$CURRENT_ATTR" != "$GITATTR_PATH" ]; then
    git config --global core.attributesfile "$GITATTR_PATH"
fi

echo "  Git configuration complete"
echo ""

echo "=== Installation complete ==="
echo ""
echo "All git repositories on this system will now use excel-diff for Excel file diffs."
echo ""
echo "To verify, run: git diff some-file.xlsx"
echo "To uninstall, run: ./scripts/uninstall-global.sh"
