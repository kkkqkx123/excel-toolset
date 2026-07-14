#!/usr/bin/env bash
# uninstall-global.sh
# Remove the globally installed excel-diff git diff driver and binary.
#
# Usage:
#   ./scripts/uninstall-global.sh [<prefix>]
#
# What this does:
#   1. Removes the global git diff driver configuration
#   2. Removes the excel-diff binary from <prefix>/bin (default: /usr/local/bin)
#
# The global gitattributes file (~/.config/git/attributes) is cleaned up
# but only the excel-diff entries are removed. Other entries are preserved.

set -euo pipefail

BINARY_NAME="excel-diff"
PREFIX="${1:-/usr/local}"
GITATTR_PATH="$HOME/.config/git/attributes"

echo "=== excel-diff global git diff driver uninstallation ==="
echo ""

# --- Step 1: Remove global git diff driver configuration ---

echo "[1/2] Removing global git diff driver configuration..."

# Unset the diff driver command
git config --global --unset diff.excel-diff.command 2>/dev/null || true

# Clean up excel-diff entries from the global gitattributes file
if [ -f "$GITATTR_PATH" ]; then
    if grep -q "excel-diff" "$GITATTR_PATH" 2>/dev/null; then
        # Remove all lines containing excel-diff
        sed -i '/excel-diff/d' "$GITATTR_PATH"
        echo "  Removed excel-diff entries from $GITATTR_PATH"

        # If the file is now empty, remove it
        if [ ! -s "$GITATTR_PATH" ]; then
            rm -f "$GITATTR_PATH"
            echo "  Removed empty gitattributes file"
        fi
    else
        echo "  No excel-diff entries found in $GITATTR_PATH"
    fi
else
    echo "  No global gitattributes file found"
fi

echo ""

# --- Step 2: Remove binary ---

echo "[2/2] Removing excel-diff binary from ${PREFIX}/bin/..."
BIN_DST="${PREFIX}/bin/${BINARY_NAME}"

if [ -f "$BIN_DST" ]; then
    if [ "$(id -u)" -eq 0 ]; then
        rm -f "$BIN_DST"
    else
        sudo rm -f "$BIN_DST" || {
            echo "WARNING: Cannot remove $BIN_DST. You may need sudo."
        }
    fi
    echo "  Removed: $BIN_DST"
else
    echo "  Binary not found at $BIN_DST (already removed)"
fi

echo ""
echo "=== Uninstallation complete ==="
echo ""
echo "Note: The global core.attributesfile git config is intentionally preserved"
echo "in case you have other entries in it."
echo ""
echo "To completely clean up the attributesfile reference, run:"
echo "  git config --global --unset core.attributesfile"
