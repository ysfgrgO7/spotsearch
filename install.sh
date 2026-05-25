#!/usr/bin/env bash

# Exit immediately if any command fails
set -e

# Styling colors
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

echo -e "${BLUE}===============================================${NC}"
echo -e "${BLUE}        SpotSearch Desktop Installer           ${NC}"
echo -e "${BLUE}===============================================${NC}"

# Check for Node.js / NPM
if ! command -v npm &> /dev/null; then
    echo -e "${RED}Error: npm is not installed. Please install Node.js and npm first.${NC}"
    exit 1
fi

# Check for Cargo / Rust
if ! command -v cargo &> /dev/null; then
    echo -e "${RED}Error: Rust/cargo is not installed. Please install rustup (https://rustup.rs/) first.${NC}"
    exit 1
fi

# Ensure we are in the project root directory
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" &> /dev/null && pwd)"
cd "$SCRIPT_DIR"

# Install npm dependencies if node_modules is missing
if [ ! -d "node_modules" ]; then
    echo -e "${YELLOW}🔄 Installing NPM dependencies...${NC}"
    npm install
else
    echo -e "${GREEN}✓ NPM dependencies already installed.${NC}"
fi

# Build the Tauri application in release mode (no-bundle to bypass external packaging requirements like linuxdeploy)
echo -e "${YELLOW}🏗️  Building SpotSearch in release mode (this might take a minute)...${NC}"
npm run tauri build -- --no-bundle

# Verify build output
BINARY_SOURCE="src-tauri/target/release/tauri-app"
if [ ! -f "$BINARY_SOURCE" ]; then
    echo -e "${RED}Error: Build succeeded but binary was not found at $BINARY_SOURCE.${NC}"
    exit 1
fi

# Create target directories if they don't exist
echo -e "${YELLOW}⚙️  Setting up directories...${NC}"
mkdir -p "$HOME/.local/bin"
mkdir -p "$HOME/.local/share/icons/hicolor/scalable/apps"
mkdir -p "$HOME/.local/share/applications"

# Copy binary
echo -e "${YELLOW}📦 Installing SpotSearch binary to ~/.local/bin/...${NC}"
cp "$BINARY_SOURCE" "$HOME/.local/bin/spotsearch"
sudo cp "$BINARY_SOURCE" "/usr/local/bin/spotsearch"
chmod +x "$HOME/.local/bin/spotsearch"
sudo chmod +x "/usr/local/bin/spotsearch"

# Copy icons
echo -e "${YELLOW}🎨 Installing application icons...${NC}"
cp icon.svg "$HOME/.local/share/icons/hicolor/scalable/apps/spotsearch.svg"
cp icon.svg "$HOME/.local/share/icons/spotsearch.svg"

# Copy PNG icons as fallbacks for maximum compatibility with desktop environments like GNOME
if [ -f "src-tauri/icons/128x128.png" ]; then
    mkdir -p "$HOME/.local/share/icons/hicolor/128x128/apps"
    cp src-tauri/icons/128x128.png "$HOME/.local/share/icons/hicolor/128x128/apps/spotsearch.png"
    cp src-tauri/icons/128x128.png "$HOME/.local/share/icons/spotsearch.png"
elif [ -f "src-tauri/icons/icon.png" ]; then
    mkdir -p "$HOME/.local/share/icons/hicolor/128x128/apps"
    cp src-tauri/icons/icon.png "$HOME/.local/share/icons/hicolor/128x128/apps/spotsearch.png"
    cp src-tauri/icons/icon.png "$HOME/.local/share/icons/spotsearch.png"
fi

# Create .desktop file
echo -e "${YELLOW}🖥️  Registering desktop launcher...${NC}"
CATALOG_ENTRY="$HOME/.local/share/applications/spotsearch.desktop"

cat > "$CATALOG_ENTRY" <<EOF
[Desktop Entry]
Type=Application
Name=SpotSearch
Comment=Lightning fast desktop search launcher
Exec=$HOME/.local/bin/spotsearch
Icon=spotsearch
Terminal=false
Categories=Utility;
StartupNotify=false
EOF

chmod +x "$CATALOG_ENTRY"

# Update desktop application database if utility exists
if command -v update-desktop-database &> /dev/null; then
    update-desktop-database "$HOME/.local/share/applications" || true
fi

# Update icon cache if utility exists
if command -v gtk-update-icon-cache &> /dev/null; then
    echo -e "${YELLOW}🔄 Updating system icon cache...${NC}"
    gtk-update-icon-cache -f -t "$HOME/.local/share/icons/hicolor" || true
fi

echo -e "${BLUE}===============================================${NC}"
echo -e "${GREEN}🎉 SpotSearch installed successfully!${NC}"
echo -e "${BLUE}===============================================${NC}"
echo -e "You can now launch SpotSearch from your desktop application launcher."
echo -e "Alternatively, run it in the background using:"
echo -e "    ${GREEN}spotsearch &${NC}"
echo -e ""
echo -e "• Try ${YELLOW}Super+Space${NC} to toggle search overlay once running!"
echo -e "• Access the system tray icon to hide, show, or quit the app."
echo -e "${BLUE}===============================================${NC}"
