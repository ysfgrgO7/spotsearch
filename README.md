# SpotSearch Launcher

A lightweight, fast, floating search bar for Linux, built with Tauri v2 and Vite.

SpotSearch provides instant access to application launching and file indexing, resembling a sleek Spotlight-like or Raycast-like experience.

---

## 🚀 Features

- **Global Hotkey:** Press `Alt+Shift+Space` to instantly toggle the launcher overlay.
- **Sleek Floating UI:** A gorgeous, glassmorphic, and borderless search bar centered on your screen.
- **Application Discovery:** Automatically indexes and runs desktop applications from `/usr/share/applications` and local user applications.
- **File Search:** Fast indexer backend in Rust for finding files instantly as you type.
- **AI Assistant:** Instantly chat with Google's AI models. (Powered by the Antigravity CLI, `agy`). Just prefix your search with `>`
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

## 🖥️ Window Manager & Desktop Integrations

SpotSearch is designed to be fully desktop-agnostic. Rather than managing complex, fragile internal key capture libraries, SpotSearch relies entirely on your window manager (WM) or desktop environment (DE) for global hotkeys. 

When you configure a keyboard shortcut to call `spotsearch --toggle`, your system instantly sends an IPC message to the running daemon, toggling the launcher in less than **2ms**.

---

### 👑 1. Hyprland Configuration (Recommended)

Add the following rules to your `~/.config/hypr/hyprland.conf` to make SpotSearch overlay perfectly and toggle seamlessly:

#### 🪟 Window Rules
Make sure SpotSearch is registered to float, pin on top of all virtual desktops, gain focus immediately, and stay centered without window borders or drop-shadow clipping:

```ini
# SpotSearch Overlay Window Rules
windowrulev2 = float, class:^(spotsearch)$, title:^(SpotSearch)$
windowrulev2 = pin, class:^(spotsearch)$, title:^(SpotSearch)$
windowrulev2 = stayfocused, class:^(spotsearch)$, title:^(SpotSearch)$
windowrulev2 = noborder, class:^(spotsearch)$, title:^(SpotSearch)$
windowrulev2 = noshadow, class:^(spotsearch)$, title:^(SpotSearch)$
windowrulev2 = center, class:^(spotsearch)$, title:^(SpotSearch)$
```

#### 🎮 Keyboard Shortcut
Bind your preferred hotkey (e.g. `Alt+Shift+Space` or `Super+D`) to toggle the launcher:

```ini
# Toggle SpotSearch Overlay
bind = SUPER, Space, exec, spotsearch --toggle
```

#### ⚙️ Autostart Daemon
To have SpotSearch run quietly in the background on startup:

```ini
# Start SpotSearch Daemon
exec-once = spotsearch
```

---

### 🪵 2. i3 / Sway Configuration

Add these settings to your Sway configuration (`~/.config/sway/config`) or i3 configuration (`~/.config/i3/config`):

#### 🪟 Window Rules & Position
```ini
# Floating rules for SpotSearch
for_window [class="spotsearch" title="SpotSearch"] floating enable, border none, move position center, sticky enable
```

#### 🎮 Keyboard Shortcut
```ini
# Toggle shortcut
bindsym Mod4+space exec spotsearch --toggle
```

#### ⚙️ Autostart Daemon
```ini
# Autostart daemon
exec --no-startup-id spotsearch
```

---

### 🖥️ 3. GNOME / KDE / XFCE

Traditional desktop environments can bind the toggle command natively via system settings:

1. Open **System Settings** -> **Keyboard** -> **Keyboard Shortcuts** (or Custom Shortcuts).
2. Add a new custom shortcut:
   - **Name:** `Toggle SpotSearch`
   - **Command:** `spotsearch --toggle`
   - **Shortcut:** Press `Alt+Shift+Space` (or your preferred combination).

To add SpotSearch to autostart:
- Search for **Startup Applications** in your menu, click **Add**, and input `spotsearch` as the command.
- Alternatively, copy the `.desktop` launcher to your autostart directory:
  ```bash
  mkdir -p ~/.config/autostart
  cp ~/.local/share/applications/spotsearch.desktop ~/.config/autostart/
  ```

