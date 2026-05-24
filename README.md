# SpotSearch Launcher

A lightweight, fast, floating search bar for Linux, built with Tauri v2 and Vite.

SpotSearch provides instant access to application launching and file indexing, resembling a sleek Spotlight-like or Raycast-like experience.

---

## 🚀 Features

- **Global Hotkey:** Press `Super+Space` to instantly toggle the launcher overlay.
- **Sleek Floating UI:** A gorgeous, glassmorphic, and borderless search bar centered on your screen.
- **Application Discovery:** Automatically indexes and runs desktop applications from `/usr/share/applications` and local user applications.
- **File Search:** Fast indexer backend in Rust for finding files instantly as you type.
- **Hyprland Optimization:** Natively supports window floating rules for the Hyprland Wayland compositor.

---

## 🛠️ System Prerequisites & Dependencies

To build or run SpotSearch on any Linux distribution, you need **Rust**, **Node.js**, and several core system libraries (development headers for `webkit2gtk`, `gtk3`, `openssl`, etc.).

Find the installation commands for your specific Linux distribution below:

### 1. Install Rust & Cargo

Regardless of your Linux distro, install Rust via `rustup` (the official installer):

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

Make sure to restart your shell or run `source "$HOME/.cargo/env"` to apply the PATH changes.

### 2. Install Node.js & npm

Ensure you have **Node.js** (v18 or higher recommended) and **npm** installed. Use your package manager or tools like [fnm](https://github.com/Schniz/fnm) or [nvm](https://github.com/nvm-sh/nvm).

### 3. Install Distro-Specific Dependencies

#### 📦 Ubuntu / Debian / Pop!\_OS / Linux Mint

```bash
sudo apt update
sudo apt install -y build-essential curl wget file libssl-dev libgtk-3-dev libwebkit2gtk-4.1-dev libayatana-appindicator3-dev librsvg2-dev
```

#### 📦 Fedora / RHEL

```bash
sudo dnf check-update
sudo dnf groupinstall -y "C Development Tools and Libraries"
sudo dnf install -y webkitgtk4.1-devel openssl-devel curl wget libappindicator-gtk3-devel librsvg2-devel
```

#### 📦 Arch Linux / Manjaro

```bash
sudo pacman -Syu --needed base-devel curl wget openssl webkit2gtk-4.1 libappindicator-gtk3 librsvg
```

#### 📦 openSUSE

```bash
sudo zypper refresh
sudo zypper install -y -t pattern devel_C_C++
sudo zypper install -y webkit2gtk3-soup2-devel gtk3-devel libappindicator3-devel librsvg2-devel libopenssl-devel curl wget
```

#### ❄️ NixOS

You can use a Nix development shell. Create a `shell.nix` in the project root with the following contents, then run `nix-shell` to enter the environment:

```nix
{ pkgs ? import <nixpkgs> { } }:

pkgs.mkShell {
  nativeBuildInputs = with pkgs; [
    pkg-config
    gobject-introspection
    cargo
    cargo-tauri
    nodejs
  ];
  buildInputs = with pkgs; [
    at-spi2-atk
    atkmm
    cairo
    gdk-pixbuf
    glib
    gtk3
    harfbuzz
    librsvg
    libsoup_3
    pango
    webkitgtk_4_1
    openssl
  ];
}
```

---

## 💻 Development

Once system dependencies are ready, clone this repository and run the local development server:

1. **Install Node dependencies:**

   ```bash
   npm install
   ```

2. **Run in development mode:**
   ```bash
   npm run tauri dev
   ```

---

## 🏗️ Building and Packaging

To compile a highly optimized release build of the application:

```bash
npm run tauri build
```

This compiles the Rust backend and packages the frontend. The build outputs will be created under:

- **`.deb` package:** `src-tauri/target/release/bundle/deb/tauri-app_*.deb` (for Debian-based distros)
- **`AppImage`:** `src-tauri/target/release/bundle/appimage/tauri-app_*.AppImage` (portable binary for any distro)
- **Raw Executable:** `src-tauri/target/release/tauri-app`

---

## 📦 How to Install

Depending on your distribution, choose one of the following installation methods:

### Method 1: Using the Debian Package (`.deb`)

For Ubuntu, Debian, Pop!\_OS, etc., install the built `.deb` using `apt`:

```bash
sudo apt install ./src-tauri/target/release/bundle/deb/tauri-app_*.deb
```

This installs the app system-wide and automatically integrates it into your desktop application menus.

### Method 2: Manual Binary Installation (Any Distro)

You can manually install the raw executable as a system-wide CLI tool:

```bash
sudo cp src-tauri/target/release/tauri-app /usr/local/bin/spotsearch
sudo chmod +x /usr/local/bin/spotsearch
```

Or a user-specific installation (does not require root):

```bash
mkdir -p ~/.local/bin
cp src-tauri/target/release/tauri-app ~/.local/bin/spotsearch
chmod +x ~/.local/bin/spotsearch
```

_(Make sure `~/.local/bin` is in your system's `PATH`!)_

### Method 3: Using the Portable AppImage (Any Distro)

Copy the `AppImage` to your user binary directory:

```bash
mkdir -p ~/.local/bin
cp src-tauri/target/release/bundle/appimage/tauri-app_*.AppImage ~/.local/bin/spotsearch
chmod +x ~/.local/bin/spotsearch
```

---

## 🖥️ Wayland & Hotkey Workaround

Wayland-based desktop environments (GNOME, KDE Plasma, Hyprland, etc.) restrict global hotkey captures for security reasons.

While SpotSearch uses Tauri's global shortcut listener (which works out of the box on X11), it may not be able to bind `Super+Space` on strict Wayland setups.

### Wayland Workaround:

You can register a custom global hotkey directly through your desktop environment settings to trigger SpotSearch's native toggle function.

1. Open your **Desktop Environment Settings** -> **Keyboard Shortcuts** (or Custom Shortcuts).
2. Create a new custom shortcut:
   - **Name:** Toggle SpotSearch
   - **Trigger/Hotkey:** `Super+Space` (or your preferred key combination)
   - **Command:** `spotsearch --toggle` (or `tauri-app --toggle` depending on your installation name)

---

## ⚙️ Desktop Integration & Autostart (Optional)

If you manually installed the binary/AppImage, you can create a desktop entry to launch SpotSearch from your application menus or add it to startup.

### 1. Create a Desktop Entry file

Create a file at `~/.local/share/applications/spotsearch.desktop` and add:

```ini
[Desktop Entry]
Type=Application
Name=SpotSearch
Exec=spotsearch
Icon=spotsearch
Comment=A lightweight, fast floating search bar for Linux
Terminal=false
Categories=Utility;
```

### 2. Copy the App Icon

To display the correct icon in application menus:

```bash
mkdir -p ~/.local/share/icons
cp src-tauri/icons/128x128.png ~/.local/share/icons/spotsearch.png
```

### 3. Add to Autostart (Optional)

To have SpotSearch run quietly in the background when your system starts:

```bash
mkdir -p ~/.config/autostart
cp ~/.local/share/applications/spotsearch.desktop ~/.config/autostart/
```

## Additional Settings

### For Hyprland We set these windows rules

```lua
----------------------
---- Window Rules ----
----------------------

hl.window_rule({
    name = "spotsearch-float",
    match = {
        title = "^(SpotSearch)$",
    },
    float = true,
})

hl.window_rule({
    name = "spotsearch-pin",
    match = {
        title = "^(SpotSearch)$",
    },
    pin = true,
})

hl.window_rule({
    name = "spotsearch-noborder",
    match = {
        title = "^(SpotSearch)$",
    },
    border_size = 0,
})

hl.window_rule({
    name = "spotsearch-center",
    match = {
        title = "^(SpotSearch)$",
    },
    center = true,
})
```
