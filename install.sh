#!/usr/bin/env bash

# Exit immediately if any command fails
set -e

# Styling colors
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

# Parse arguments
AUTO_UPDATE=false
for arg in "$@"; do
    if [ "$arg" = "--auto-update" ] || [ "$arg" = "--non-interactive" ]; then
        AUTO_UPDATE=true
    fi
done

# Confirmation prompt helper
confirm_action() {
    local prompt_msg="$1"
    local default_choice="${2:-n}"

    if [ "$AUTO_UPDATE" = "true" ]; then
        # Default to false in non-interactive/auto-update mode
        return 1
    fi

    if [ "$default_choice" = "y" ]; then
        prompt_msg="$prompt_msg [Y/n]: "
    else
        prompt_msg="$prompt_msg [y/N]: "
    fi

    read -p "$(echo -e "$prompt_msg")" choice
    case "$choice" in
        [yY][eE][sS]|[yY])
            return 0
            ;;
        [nN][oO]|[nN])
            return 1
            ;;
        *)
            if [ -z "$choice" ]; then
                if [ "$default_choice" = "y" ]; then
                    return 0
                else
                    return 1
                fi
            fi
            return 1
            ;;
    esac
}

echo -e "${BLUE}===============================================${NC}"
echo -e "${BLUE}        SpotSearch Desktop Installer           ${NC}"
echo -e "${BLUE}===============================================${NC}"

# Detect distribution
OS_ID="unknown"
OS_LIKE="unknown"
if [ -f /etc/os-release ]; then
    . /etc/os-release
    OS_ID=$ID
    OS_LIKE=$ID_LIKE
fi

echo -e "• Detected Operating System: ${GREEN}${OS_ID}${NC} (like: ${OS_LIKE})"

# Ask the user if they want to proceed before installing/configuring anything
if [ "$AUTO_UPDATE" = "false" ]; then
    if ! confirm_action "Do you want to proceed with the SpotSearch installation?" "y"; then
        echo -e "${RED}Installation cancelled by user.${NC}"
        exit 0
    fi
fi

# --- Detect and Install System Dependencies ---
MISSING_PACKAGES=()
PKG_MGR=""
INSTALL_CMD=""

if [[ "$OS_ID" == "ubuntu" || "$OS_ID" == "debian" || "$OS_ID" == "mint" || "$OS_ID" == "pop" || "$OS_LIKE" == *"debian"* ]]; then
    PKG_MGR="apt"
    INSTALL_CMD="sudo apt-get update && sudo apt-get install -y"
    
    check_debian_pkg() {
        dpkg -s "$1" &> /dev/null
    }
    
    DEPS=("build-essential" "curl" "wget" "file" "pkg-config" "libssl-dev" "libgtk-3-dev" "libayatana-appindicator3-dev" "librsvg2-dev" "libwebkit2gtk-4.1-dev" "libgtk-layer-shell-dev" "libxdo-dev")
    for dep in "${DEPS[@]}"; do
        if ! check_debian_pkg "$dep"; then
            MISSING_PACKAGES+=("$dep")
        fi
    done

elif [[ "$OS_ID" == "arch" || "$OS_ID" == "manjaro" || "$OS_LIKE" == *"arch"* ]]; then
    PKG_MGR="pacman"
    INSTALL_CMD="sudo pacman -Sy --needed --noconfirm"
    
    # Corrected package names for Arch Linux: libayatana-appindicator and xdotool
    DEPS=("curl" "wget" "openssl" "webkit2gtk-4.1" "libayatana-appindicator" "librsvg" "gtk-layer-shell" "xdotool")
    
    # Check for build tools group base-devel
    if ! command -v make &> /dev/null || ! command -v gcc &> /dev/null; then
        MISSING_PACKAGES+=("base-devel")
    fi
    
    for dep in "${DEPS[@]}"; do
        if ! pacman -Qs "^$dep$" &> /dev/null; then
            MISSING_PACKAGES+=("$dep")
        fi
    done

elif [[ "$OS_ID" == "fedora" || "$OS_ID" == "centos" || "$OS_ID" == "rhel" || "$OS_LIKE" == *"fedora"* ]]; then
    PKG_MGR="dnf"
    INSTALL_CMD="sudo dnf install -y"
    
    # Corrected package name for Fedora: libayatana-appindicator-gtk3-devel
    DEPS=("curl" "wget" "openssl-devel" "webkit2gtk4.1-devel" "libayatana-appindicator-gtk3-devel" "librsvg2-devel" "gtk-layer-shell-devel" "libxdo-devel" "pkgconf-pkg-config")
    for dep in "${DEPS[@]}"; do
        if ! rpm -q "$dep" &> /dev/null; then
            MISSING_PACKAGES+=("$dep")
        fi
    done
    # Check development-tools group
    if ! dnf group list installed "Development Tools" &> /dev/null; then
        MISSING_PACKAGES+=("@development-tools")
    fi

elif [[ "$OS_ID" == "opensuse"* || "$OS_ID" == "sles" || "$OS_LIKE" == *"suse"* ]]; then
    PKG_MGR="zypper"
    INSTALL_CMD="sudo zypper install -y"
    
    # Corrected package name for openSUSE: xdotool instead of libxdo-devel
    DEPS=("curl" "wget" "libopenssl-devel" "webkit2gtk-4.1-devel" "libayatana-appindicator3-devel" "librsvg-devel" "gtk-layer-shell-devel" "xdotool" "pkg-config")
    for dep in "${DEPS[@]}"; do
        if ! rpm -q "$dep" &> /dev/null; then
            MISSING_PACKAGES+=("$dep")
        fi
    done
    if ! zypper search -i --match-exact devel_basis &> /dev/null; then
        MISSING_PACKAGES+=("-t pattern devel_basis")
    fi
fi

# Prompt for system dependencies
if [ ${#MISSING_PACKAGES[@]} -ne 0 ]; then
    echo -e "${YELLOW}⚙️  The following missing system dependencies were detected: ${MISSING_PACKAGES[*]}${NC}"
    if confirm_action "Would you like to install these packages on your system now?" "y"; then
        echo -e "${BLUE}Installing missing dependencies...${NC}"
        eval "$INSTALL_CMD ${MISSING_PACKAGES[*]}"
    else
        echo -e "${YELLOW}⚠️  Skipping system package installation. Build might fail if dependencies are missing.${NC}"
    fi
else
    echo -e "${GREEN}✓ All core system dependencies are satisfied.${NC}"
fi


# --- Check Node.js and NPM ---
if ! command -v npm &> /dev/null; then
    echo -e "${YELLOW}⚠️  npm / Node.js is not installed.${NC}"
    if confirm_action "Would you like to install Node.js and npm?" "y"; then
        echo -e "${BLUE}Installing Node.js & npm...${NC}"
        if [ "$PKG_MGR" = "apt" ]; then
            sudo apt-get update && sudo apt-get install -y nodejs npm
        elif [ "$PKG_MGR" = "pacman" ]; then
            sudo pacman -S --noconfirm nodejs npm
        elif [ "$PKG_MGR" = "dnf" ]; then
            sudo dnf install -y nodejs npm
        elif [ "$PKG_MGR" = "zypper" ]; then
            sudo zypper install -y nodejs npm
        else
            echo -e "${RED}Could not auto-install Node.js. Please install manually.${NC}"
            exit 1
        fi
    else
        echo -e "${RED}Error: npm is not installed. Please install Node.js and npm first.${NC}"
        exit 1
    fi
else
    echo -e "${GREEN}✓ Node.js and npm detected.${NC}"
fi


# --- Check Rust and Cargo ---
if ! command -v cargo &> /dev/null; then
    echo -e "${YELLOW}⚠️  Rust/Cargo is not installed.${NC}"
    if confirm_action "Would you like to install Rust using rustup (recommended)?" "y"; then
        echo -e "${BLUE}Downloading and running rustup installer...${NC}"
        curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
        # Source the cargo environment variables
        source "$HOME/.cargo/env"
    else
        echo -e "${RED}Error: Rust/cargo is not installed. Please install rustup (https://rustup.rs/) first.${NC}"
        exit 1
    fi
else
    echo -e "${GREEN}✓ Rust and Cargo detected.${NC}"
fi

# Ensure we are in the project root directory
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" &> /dev/null && pwd)"
cd "$SCRIPT_DIR"

# Install npm dependencies if node_modules is missing or if in auto-update mode
if [ ! -d "node_modules" ]; then
    echo -e "${YELLOW}⚠️  node_modules directory is missing.${NC}"
    if confirm_action "Would you like to install NPM dependencies?" "y"; then
        echo -e "${BLUE}Installing NPM dependencies...${NC}"
        npm install
    else
        echo -e "${RED}Error: Cannot proceed without NPM dependencies.${NC}"
        exit 1
    fi
else
    if [ "$AUTO_UPDATE" = "true" ]; then
        echo -e "${YELLOW}🔄 Updating NPM dependencies for auto-update...${NC}"
        npm install
    else
        echo -e "${GREEN}✓ NPM dependencies already installed.${NC}"
    fi
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
mkdir -p "$HOME/.local/share/spotsearch"

# Copy binary to local user directory (doesn't require sudo)
echo -e "${YELLOW}📦 Installing SpotSearch binary to ~/.local/bin/...${NC}"
rm -f "$HOME/.local/bin/spotsearch"
cp "$BINARY_SOURCE" "$HOME/.local/bin/spotsearch"
chmod +x "$HOME/.local/bin/spotsearch"

# System-wide installation (requires sudo, skipped if run non-interactively or denied)
if [ "$AUTO_UPDATE" = "true" ]; then
    echo -e "${GREEN}✓ Running in auto-update mode. Skipping system-wide root copy.${NC}"
else
    if confirm_action "Would you like to install the binary system-wide to /usr/local/bin? (Requires sudo)" "n"; then
        echo -e "${YELLOW}📦 Copying binary to /usr/local/bin/...${NC}"
        sudo rm -f "/usr/local/bin/spotsearch"
        sudo cp "$BINARY_SOURCE" "/usr/local/bin/spotsearch"
        sudo chmod +x "/usr/local/bin/spotsearch"
    else
        echo -e "${YELLOW}⚠️  Skipping system-wide install. SpotSearch is installed locally at ~/.local/bin/spotsearch.${NC}"
    fi
fi

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

# Save the version and repository path for auto-update feature
VERSION=$(node -e "console.log(require('./package.json').version)" 2>/dev/null || grep -m 1 -oP '"version":\s*"\K[^"]+' package.json || echo "0.1.0")
echo "$VERSION" > "$HOME/.local/share/spotsearch/version"
echo "$SCRIPT_DIR" > "$HOME/.local/share/spotsearch/repo_path"

echo -e "${BLUE}===============================================${NC}"
echo -e "${GREEN}🎉 SpotSearch v${VERSION} installed successfully!${NC}"
echo -e "${BLUE}===============================================${NC}"
echo -e "You can now launch SpotSearch from your desktop application launcher."
echo -e "Alternatively, run it in the background using:"
echo -e "    ${GREEN}spotsearch &${NC}"
echo -e ""
echo -e "• Try ${YELLOW}Super+Space${NC} to toggle search overlay once running!"
echo -e "• Access the system tray icon to hide, show, or quit the app."
echo -e "${BLUE}===============================================${NC}"
