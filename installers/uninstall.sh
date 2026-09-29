#!/bin/sh
# York toolchain uninstaller for macOS / Linux.
# Run:  curl -fsSL https://raw.githubusercontent.com/TheRealClyp/York/main/installers/uninstall.sh | sh

set -e

BOLD=$( [ -t 1 ] && printf "\033[1m" || printf "" )
GREEN=$( [ -t 1 ] && printf "\033[32m" || printf "" )
CYAN=$( [ -t 1 ] && printf "\033[36m" || printf "" )
YELLOW=$( [ -t 1 ] && printf "\033[33m" || printf "" )
RESET=$( [ -t 1 ] && printf "\033[0m" || printf "" )

cat <<'EOF'
   ██╗  ██╗ ██████╗ ██████╗ ██╗  ██╗
   ██║ ██╔╝██╔═══██╗██╔══██╗██║ ██╔╝
   █████╔╝ ██║   ██║██████╔╝█████╔╝ 
   ██╔═██╗ ██║   ██║██╔══██╗██╔═██╗ 
   ██║  ██╗╚██████╔╝██║  ██║██║  ██╗
   ╚═╝  ╚═╝ ╚═════╝ ╚═╝  ╚═╝╚═╝  ╚═╝
EOF

echo ""
echo "${YELLOW}York Uninstaller${RESET}"
echo "────────────────────────────────────────────────────────────"

INSTALL_DIR="$HOME/.york"

if [ ! -d "$INSTALL_DIR" ]; then
    echo "${YELLOW}York directory ($INSTALL_DIR) does not exist.${RESET}"
fi

# Clean files
echo "${CYAN}[1/2]${RESET} Removing $INSTALL_DIR..."
rm -rf "$INSTALL_DIR"
echo "      ${GREEN}Removed $INSTALL_DIR${RESET}"

# Clean profile
echo "${CYAN}[2/2]${RESET} Cleaning PATH exports from shell configuration..."
for rc in "$HOME/.zshrc" "$HOME/.bashrc" "$HOME/.profile"; do
    if [ -f "$rc" ]; then
        if grep -q "york/bin" "$rc" 2>/dev/null; then
            # Remove lines referencing york/bin
            sed -i.bak '/york\/bin/d' "$rc" 2>/dev/null || sed -i '' '/york\/bin/d' "$rc" 2>/dev/null || true
            echo "      ${GREEN}Cleaned $rc${RESET}"
        fi
    fi
done

echo ""
echo "${GREEN}York has been successfully uninstalled from your system.${RESET}"
echo "${YELLOW}Open a new terminal session or reload your shell profile to apply changes.${RESET}"
echo ""
