#!/bin/sh
# York toolchain installer for macOS / Linux.
# Run:  curl -fsSL https://raw.githubusercontent.com/york-lang/york/main/installers/install.sh | sh

set -e

BOLD=$( [ -t 1 ] && printf "\033[1m" || printf "" )
GREEN=$( [ -t 1 ] && printf "\033[32m" || printf "" )
CYAN=$( [ -t 1 ] && printf "\033[36m" || printf "" )
PURPLE=$( [ -t 1 ] && printf "\033[35m" || printf "" )
YELLOW=$( [ -t 1 ] && printf "\033[33m" || printf "" )
RESET=$( [ -t 1 ] && printf "\033[0m" || printf "" )

# ── YORK banner ──────────────────────────────────────────────
cat <<'EOF'
   ██╗  ██╗ ██████╗ ██████╗ ██╗  ██╗
   ██║ ██╔╝██╔═══██╗██╔══██╗██║ ██╔╝
   █████╔╝ ██║   ██║██████╔╝█████╔╝
   ██╔═██╗ ██║   ██║██╔══██╗██╔═██╗
   ██║  ██╗╚██████╔╝██║  ██║██║  ██╗
   ╚═╝  ╚═╝ ╚═════╝ ╚═╝  ╚═╝╚═╝  ╚═╝
EOF
cat <<'EOF'
██╗    ██╗  █████╗  ███████╗  █████╗  ██╗   ██╗
██║    ██║  ██╔══██╗  ██╔════╝  ██╔══██╗  ╚██╗ ██╔╝
██║ █╗ ██║  ███████║  ███████╗  ███████║   ╚████╔╝
██║███╗██║  ██╔══██║  ╚════██║  ██╔══██║    ╚██╔╝
╚███╔███╔╝  ██║  ██║  ███████║  ██║  ██║     ██║
 ╚══╝╚══╝  ╚═╝  ╚═╝  ╚══════╝  ╚═╝  ╚═╝     ╚═╝
EOF

echo ""
echo "${GREEN}York installer${RESET}"

VERSION="${YORK_VERSION:-latest}"
BASE="${YORK_INSTALL_BASE:-https://github.com/york-lang/york/releases}"

# Detect OS
case "$(uname -s)" in
    Darwin*) OS="macos" ;;
    Linux*)  OS="linux" ;;
    *) echo "unsupported OS: $(uname -s)" >&2; exit 1 ;;
esac

# Detect architecture
ARCH="$(uname -m)"
case "$ARCH" in
    x86_64|amd64) TARGET="x86_64" ;;
    arm64|aarch64) TARGET="aarch64" ;;
    *) echo "unsupported architecture: $ARCH" >&2; exit 1 ;;
esac

# Install location ~/.york/bin
INSTALL_DIR="$HOME/.york"
BIN_DIR="$INSTALL_DIR/bin"
mkdir -p "$BIN_DIR"

EXE="$BIN_DIR/york"

ARTIFACT="york-$TARGET-$OS.tar.gz"
URL="$BASE/download/$VERSION/$ARTIFACT"
TMP=$(mktemp -d)

echo "${CYAN}[1/4] ${RESET}detected ${PURPLE}$OS/$TARGET${RESET}"
echo "${CYAN}[2/4] ${RESET}downloading $URL"
if command -v curl >/dev/null 2>&1; then
    curl -fsSL "$URL" -o "$TMP/$ARTIFACT"
else
    wget -q "$URL" -O "$TMP/$ARTIFACT"
fi

echo "${CYAN}[3/4] ${RESET}extracting $ARTIFACT"
tar -xzf "$TMP/$ARTIFACT" -C "$TMP"

SRC=$(find "$TMP" -name york -type f | head -n 1)
if [ -z "$SRC" ]; then
    echo "could not find 'york' in archive" >&2
    exit 1
fi

mv "$SRC" "$EXE"
chmod +x "$EXE"
rm -rf "$TMP"

# Add to PATH via shell profile if not present
PROFILE=""
if [ -n "$ZSH_VERSION" ] || [ -f "$HOME/.zshrc" ]; then
    PROFILE="$HOME/.zshrc"
elif [ -f "$HOME/.bashrc" ]; then
    PROFILE="$HOME/.bashrc"
else
    PROFILE="$HOME/.profile"
fi

case ":${PATH}:" in
    *":$BIN_DIR:"*) ;;
    *)
        if ! grep -qF "export PATH=\"\$HOME/.york/bin" "$PROFILE" 2>/dev/null; then
            printf '\nexport PATH="$HOME/.york/bin:$PATH"\n' >> "$PROFILE"
            echo "${CYAN}[4/4] ${RESET}${YELLOW}added $BIN_DIR to PATH in $PROFILE${RESET}"
        else
            echo "${CYAN}[4/4] ${RESET}PATH already configured in $PROFILE"
        fi
        ;;
esac

echo ""
echo "${GREEN}YORK ${RESET}${PURPLE}WASAY${RESET}"
echo ""
echo "${GREEN}york $VERSION installed.${RESET}"
echo "Run 'york --help' to get started."
echo "(start a new terminal, or run 'export PATH=\"\$HOME/.york/bin:\$PATH\"' now)"