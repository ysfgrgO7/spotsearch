#!/usr/bin/env bash

# Styling colors
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

echo -e "${BLUE}===============================================${NC}"
echo -e "${BLUE}        SpotSearch Desktop Uninstaller         ${NC}"
echo -e "${BLUE}===============================================${NC}"

# Kill running processes of spotsearch
if pgrep -x "spotsearch" &> /dev/null; then
    echo -e "${YELLOW}🛑 Stopping running SpotSearch processes...${NC}"
    pkill -x "spotsearch" || true
fi

# Remove binary from local and system bin
if [ -f "$HOME/.local/bin/spotsearch" ]; then
    echo -e "${YELLOW}🗑️  Removing binary from ~/.local/bin/...${NC}"
    rm "$HOME/.local/bin/spotsearch"
else
    echo -e "• Binary ~/.local/bin/spotsearch already removed."
fi

if [ -f "/usr/local/bin/spotsearch" ]; then
    echo -e "${YELLOW}🗑️  Removing binary from /usr/local/bin/... (Requires sudo)${NC}"
    sudo rm -f "/usr/local/bin/spotsearch" || true
fi

# Remove version and repo path metadata
if [ -d "$HOME/.local/share/spotsearch" ]; then
    echo -e "${YELLOW}🗑️  Removing application metadata directory...${NC}"
    rm -rf "$HOME/.local/share/spotsearch"
fi

# Remove icons
echo -e "${YELLOW}🗑️  Removing application icons...${NC}"
rm -f "$HOME/.local/share/icons/hicolor/scalable/apps/spotsearch.svg"
rm -f "$HOME/.local/share/icons/hicolor/128x128/apps/spotsearch.png"
rm -f "$HOME/.local/share/icons/spotsearch.svg"
rm -f "$HOME/.local/share/icons/spotsearch.png"

# Remove desktop entry
if [ -f "$HOME/.local/share/applications/spotsearch.desktop" ]; then
    echo -e "${YELLOW}🗑️  Removing desktop entry...${NC}"
    rm "$HOME/.local/share/applications/spotsearch.desktop"
else
    echo -e "• Desktop launcher already removed."
fi

# Update desktop application database if utility exists
if command -v update-desktop-database &> /dev/null; then
    update-desktop-database "$HOME/.local/share/applications" || true
fi

# Update icon cache if utility exists
if command -v gtk-update-icon-cache &> /dev/null; then
    gtk-update-icon-cache -f -t "$HOME/.local/share/icons/hicolor" || true
fi

echo -e "${BLUE}===============================================${NC}"
echo -e "${GREEN}🎉 SpotSearch has been completely uninstalled!${NC}"
echo -e "${BLUE}===============================================${NC}"
