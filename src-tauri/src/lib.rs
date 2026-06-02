use std::process::Command;
use std::thread;
use tauri::{command, AppHandle, Manager, State};
mod apps;
mod config;
mod indexer;
use config::AppConfig;
use std::sync::Mutex;

#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub struct SearchResult {
    pub name: String,
    pub path: Option<String>,
    pub icon_data: Option<String>,
    pub is_app: bool,
    pub exec: Option<String>,
    pub subtitle: Option<String>,
}

fn is_acronym_match(query: &str, name: &str) -> bool {
    let name_lower = name.to_lowercase();
    let query_lower = query.to_lowercase();

    // Extract first letters of all alphanumeric words
    let initials: String = name_lower
        .split(|c: char| !c.is_alphanumeric())
        .filter(|word| !word.is_empty())
        .filter_map(|word| word.chars().next())
        .collect();

    if initials.is_empty() {
        return false;
    }

    initials.starts_with(&query_lower) || query_lower.starts_with(&initials)
}

#[command]
fn search(query: &str, indexer: State<'_, indexer::Indexer>) -> Vec<SearchResult> {
    let lower_query = query.to_lowercase();
    let terms: Vec<&str> = lower_query.split_whitespace().collect();

    if terms.is_empty() {
        return Vec::new();
    }

    // --- App results ---
    let mut app_results = Vec::new();
    let apps_list = apps::get_apps();
    for app in apps_list {
        let app_name = app.name.to_lowercase();
        let app_exec = app.exec.to_lowercase();
        let app_file = app.desktop_file.to_lowercase();
        let app_keywords = app.keywords.as_deref().unwrap_or("").to_lowercase();
        let app_generic = app.generic_name.as_deref().unwrap_or("").to_lowercase();

        let matches = terms.iter().all(|term| {
            app_name.contains(term)
                || app_exec.contains(term)
                || app_file.contains(term)
                || app_keywords.contains(term)
                || app_generic.contains(term)
                || is_acronym_match(term, &app.name)
        });

        if matches {
            app_results.push(SearchResult {
                name: app.name,
                path: None, // Don't expose .desktop path to UI
                icon_data: app.icon_data,
                is_app: true,
                exec: Some(app.exec.clone()),
                subtitle: app.generic_name.clone().or(app.categories.clone()),
            });
        }
    }
    // Sort apps: shorter names first (better match)
    app_results.sort_by_key(|r| r.name.len());
    app_results.truncate(5);

    // --- File results ---
    let mut file_results = indexer.search(query);
    // Set subtitle to the file path for files
    for fr in &mut file_results {
        fr.subtitle = fr.path.clone();
    }
    file_results.truncate(30);

    // Combine: apps first, then files
    let mut results = app_results;
    results.append(&mut file_results);
    results
}

#[command]
fn hide_window(app: AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.hide();
    }
}

#[command]
fn open_result(app: AppHandle, result: SearchResult) {
    if result.is_app {
        if let Some(exec) = result.exec {
            let parts: Vec<&str> = exec.split_whitespace().collect();
            if !parts.is_empty() {
                let mut cmd = Command::new(parts[0]);
                cmd.args(&parts[1..]);
                let _ = cmd.spawn();
            }
        }
    } else {
        if let Some(path) = result.path {
            let _ = Command::new("xdg-open").arg(path).spawn();
        }
    }
    hide_window(app);
}

#[command]
fn get_config(config_state: State<'_, Mutex<AppConfig>>) -> AppConfig {
    config_state.lock().unwrap().clone()
}

#[command]
fn save_config(
    new_config: AppConfig,
    config_state: State<'_, Mutex<AppConfig>>,
    indexer: State<'_, indexer::Indexer>,
) -> Result<(), String> {
    let mut config = config_state.lock().unwrap();

    // 1. Save config to disk
    new_config.save().map_err(|e| e.to_string())?;

    // 2. Update memory state
    *config = new_config.clone();

    // 3. Trigger indexer rebuild in background
    let indexer_clone = indexer.inner().clone();
    let new_config_clone = new_config.clone();
    thread::spawn(move || {
        indexer_clone.build_index_with_config(&new_config_clone);
    });

    Ok(())
}

#[command]
fn open_settings(app: AppHandle) {
    if let Some(main_window) = app.get_webview_window("main") {
        let _ = main_window.hide();
    }

    if let Some(settings_window) = app.get_webview_window("settings") {
        let _ = settings_window.show();
        let _ = settings_window.set_focus();
    } else {
        let _settings_window = tauri::WebviewWindowBuilder::new(
            &app,
            "settings",
            tauri::WebviewUrl::App("settings.html".into()),
        )
        .title("SpotSearch Settings")
        .inner_size(680.0, 560.0)
        .resizable(true)
        .decorations(true)
        .center()
        .build();
    }
}

#[derive(serde::Serialize, Clone)]
pub struct UpdateInfo {
    pub has_update: bool,
    pub current_version: String,
    pub latest_version: String,
    pub repo_path: Option<String>,
}

#[command]
fn check_for_updates() -> Result<UpdateInfo, String> {
    use std::fs;
    use std::process::Command;
    let base_dirs = directories::BaseDirs::new()
        .ok_or_else(|| "Could not determine home directory".to_string())?;
    let share_dir = base_dirs.data_local_dir().join("spotsearch");
    let version_file = share_dir.join("version");
    let repo_path_file = share_dir.join("repo_path");

    let current_version = if version_file.exists() {
        fs::read_to_string(&version_file)
            .map(|s| s.trim().to_string())
            .unwrap_or_else(|_| "1.0.1".to_string())
    } else {
        "1.0.1".to_string()
    };

    let repo_path = if repo_path_file.exists() {
        fs::read_to_string(&repo_path_file)
            .map(|s| s.trim().to_string())
            .ok()
    } else {
        None
    };

    let mut latest_version = current_version.clone();
    let mut has_update = false;

    if let Some(ref path) = repo_path {
        // Try to fetch latest changes from git remote to know if there's any update
        let _ = Command::new("git").arg("fetch").current_dir(path).status();

        // Try origin/main package.json first, then origin/master as fallback
        let mut got_remote_version = false;
        for branch in &["origin/main:package.json", "origin/master:package.json"] {
            if let Ok(output) = Command::new("git")
                .args(["show", branch])
                .current_dir(path)
                .output()
            {
                if output.status.success() {
                    if let Ok(content) = String::from_utf8(output.stdout) {
                        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
                            if let Some(ver) = json.get("version").and_then(|v| v.as_str()) {
                                latest_version = ver.to_string();
                                got_remote_version = true;
                                break;
                            }
                        }
                    }
                }
            }
        }

        // If we couldn't get it from the remote (e.g. offline), fall back to the local repository package.json
        if !got_remote_version {
            let package_json_path = std::path::Path::new(path).join("package.json");
            if package_json_path.exists() {
                if let Ok(content) = fs::read_to_string(&package_json_path) {
                    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
                        if let Some(ver) = json.get("version").and_then(|v| v.as_str()) {
                            latest_version = ver.to_string();
                        }
                    }
                }
            }
        }

        if latest_version != current_version {
            has_update = true;
        }
    }

    Ok(UpdateInfo {
        has_update,
        current_version,
        latest_version,
        repo_path,
    })
}

fn strip_ansi_codes(input: &str) -> String {
    let mut result = String::new();
    let mut chars = input.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\x1b' || c == '\u{1b}' {
            if let Some('[') = chars.peek() {
                let _ = chars.next(); // consume '['
                while let Some(&next_c) = chars.peek() {
                    let _ = chars.next();
                    if next_c.is_ascii_alphabetic() {
                        break;
                    }
                }
            }
        } else {
            result.push(c);
        }
    }
    result
}

#[command]
fn apply_update(app: AppHandle) -> Result<(), String> {
    use std::fs;
    use std::io::{BufRead, BufReader};
    use std::process::{Command, Stdio};
    use tauri::Emitter;

    let base_dirs = directories::BaseDirs::new()
        .ok_or_else(|| "Could not determine home directory".to_string())?;
    let share_dir = base_dirs.data_local_dir().join("spotsearch");
    let repo_path_file = share_dir.join("repo_path");

    if !repo_path_file.exists() {
        return Err("Source repository path not found. Cannot auto-update.".to_string());
    }

    let repo_path = fs::read_to_string(&repo_path_file)
        .map(|s| s.trim().to_string())
        .map_err(|e| format!("Failed to read repo path: {}", e))?;

    let install_script = std::path::Path::new(&repo_path).join("install.sh");
    if !install_script.exists() {
        return Err(format!(
            "Installer script not found at {:?}",
            install_script
        ));
    }

    let repo_path_clone = repo_path.clone();
    let app_clone = app.clone();

    std::thread::spawn(move || {
        let emit_log = |msg: &str, is_system: bool| {
            #[derive(serde::Serialize, Clone)]
            struct LogPayload {
                message: String,
                is_system: bool,
            }
            let _ = app_clone.emit(
                "update-log",
                LogPayload {
                    message: msg.to_string(),
                    is_system,
                },
            );
        };

        emit_log("Starting update process...", true);
        emit_log("Pulling latest changes from git repository...", true);

        // Run git pull
        let git_status = Command::new("git")
            .arg("pull")
            .current_dir(&repo_path_clone)
            .status();

        match git_status {
            Ok(s) if s.success() => {
                emit_log("Git pull completed successfully.", true);
            }
            _ => {
                emit_log("Warning: Git pull failed or repository has no remote.", true);
            }
        }

        emit_log("Running install script...", true);

        // Run bash install.sh --auto-update and capture stdout/stderr merged (using bash -c)
        let mut child = match Command::new("bash")
            .arg("-c")
            .arg("bash install.sh --auto-update 2>&1")
            .current_dir(&repo_path_clone)
            .stdout(Stdio::piped())
            .spawn()
        {
            Ok(c) => c,
            Err(e) => {
                let err_msg = format!("Failed to start installer script: {}", e);
                emit_log(&err_msg, true);
                let _ = app_clone.emit("update-error", err_msg);
                return;
            }
        };

        if let Some(stdout) = child.stdout.take() {
            let reader = BufReader::new(stdout);
            for line in reader.lines() {
                if let Ok(line_content) = line {
                    let cleaned_line = strip_ansi_codes(&line_content);
                    emit_log(&cleaned_line, false);
                }
            }
        }

        let status = child.wait();

        match status {
            Ok(s) if s.success() => {
                emit_log("Auto-update successfully built and installed!", true);
                emit_log("Launching updated SpotSearch and exiting in 2 seconds...", true);
                
                let _ = app_clone.emit("update-complete", ());

                // Sleep to allow the user to see the success message
                std::thread::sleep(std::time::Duration::from_secs(2));

                if let Some(base_dirs) = directories::BaseDirs::new() {
                    let bin_path = base_dirs.home_dir().join(".local/bin/spotsearch");
                    if bin_path.exists() {
                        let _ = Command::new(bin_path).spawn();
                    }
                }

                std::process::exit(0);
            }
            _ => {
                emit_log("Error: Auto-update failed to build or install.", true);
                let _ = app_clone.emit("update-error", "Auto-update failed to build or install.".to_string());
            }
        }
    });

    Ok(())
}

#[cfg(target_os = "linux")]
fn is_wayland() -> bool {
    std::env::var("WAYLAND_DISPLAY").is_ok()
}

#[cfg(target_os = "linux")]
fn setup_wayland_grab(window: &tauri::WebviewWindow) {
    use gtk_layer_shell::{Edge, KeyboardMode, Layer, LayerShell};

    if !gtk_layer_shell::is_supported() {
        println!("Wayland compositor does not support Layer Shell protocol. Falling back to normal window.");
        return;
    }

    if let Ok(gtk_window) = window.gtk_window() {
        // Initialize the window as a Wayland Layer Surface
        gtk_window.init_layer_shell();

        // Place it in the overlay layer (above everything, including panels)
        gtk_window.set_layer(Layer::Overlay);

        // Force exclusive keyboard grab (Compositor redirects all keys to our app)
        gtk_window.set_keyboard_mode(KeyboardMode::Exclusive);

        // Disable anchoring so it floats centered
        gtk_window.set_anchor(Edge::Top, false);
        gtk_window.set_anchor(Edge::Bottom, false);
        gtk_window.set_anchor(Edge::Left, false);
        gtk_window.set_anchor(Edge::Right, false);

        println!("Wayland overlay & exclusive keyboard grab initialized successfully.");
    }
}

#[cfg(target_os = "linux")]
fn grab_keyboard_x11(window: &tauri::Window) {
    use gtk::prelude::*;

    if let Ok(gtk_window) = window.gtk_window() {
        if let Some(gdk_window) = gtk_window.window() {
            let display = gdk_window.display();
            if let Some(seat) = display.default_seat() {
                // Grab the entire keyboard exclusively
                let grab_status = seat.grab(
                    &gdk_window,
                    gdk::SeatCapabilities::KEYBOARD,
                    true,
                    None,
                    None,
                    None,
                );

                if grab_status == gdk::GrabStatus::Success {
                    println!("X11 keyboard grab successfully established.");
                } else {
                    eprintln!("Failed to establish X11 keyboard grab: {:?}", grab_status);
                }
            }
        }
    }
}

#[cfg(target_os = "linux")]
fn ungrab_keyboard_x11(window: &tauri::Window) {
    use gtk::prelude::*;

    if let Ok(gtk_window) = window.gtk_window() {
        if let Some(gdk_window) = gtk_window.window() {
            let display = gdk_window.display();
            if let Some(seat) = display.default_seat() {
                seat.ungrab();
                println!("X11 keyboard grab released.");
            }
        }
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_cli::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_single_instance::init(|app, argv, _cwd| {
            if let Some(window) = app.get_webview_window("main") {
                let has_toggle = argv.iter().any(|arg| arg == "--toggle" || arg == "-t");
                if has_toggle {
                    if window.is_visible().unwrap_or(false) {
                        let _ = window.hide();
                    } else {
                        let _ = window.show();
                        let _ = window.set_focus();
                    }
                } else {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }
        }))
        .invoke_handler(tauri::generate_handler![
            search,
            hide_window,
            open_result,
            get_config,
            save_config,
            open_settings,
            check_for_updates,
            apply_update
        ])
        .setup(|app| {
            // Load AppConfig and manage state
            let config = AppConfig::load();
            app.manage(Mutex::new(config.clone()));

            // Setup indexer state
            let indexer_instance = indexer::Indexer::new().expect("Failed to init indexer");
            let indexer_clone = indexer_instance.clone();

            // Build index on a background thread — does NOT block the UI
            let config_for_index = config.clone();
            thread::spawn(move || {
                indexer_clone.build_index_with_config(&config_for_index);
            });

            app.manage(indexer_instance);

            if let Some(window) = app.get_webview_window("main") {
                let _ = window.set_always_on_top(true);
                #[cfg(target_os = "linux")]
                {
                    if is_wayland() {
                        setup_wayland_grab(&window);
                    }
                }
            }

            // Setup IPC socket listener for super-fast toggling
            let app_handle = app.handle().clone();
            thread::spawn(move || {
                let base_dirs = directories::BaseDirs::new().expect("Failed to get base dirs");
                let config_dir = base_dirs.config_dir().join("launcher");
                let socket_path = config_dir.join("spotsearch.sock");

                // Ensure the parent directory exists
                if !config_dir.exists() {
                    let _ = std::fs::create_dir_all(&config_dir);
                }

                // Remove existing socket file if it exists
                let _ = std::fs::remove_file(&socket_path);

                if let Ok(listener) = std::os::unix::net::UnixListener::bind(&socket_path) {
                    use std::io::Read;
                    for stream in listener.incoming() {
                        if let Ok(mut stream) = stream {
                            let mut buf = [0; 6];
                            if let Ok(n) = stream.read(&mut buf) {
                                if &buf[..n] == b"toggle" {
                                    if let Some(window) = app_handle.get_webview_window("main") {
                                        let is_visible = window.is_visible().unwrap_or(false);
                                        if is_visible {
                                            let _ = window.hide();
                                        } else {
                                            let _ = window.show();
                                            let _ = window.set_focus();
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            });

            // Set Hyprland-specific window rules so the window floats as an overlay
            if std::env::var("HYPRLAND_INSTANCE_SIGNATURE").is_ok() {
                let _ = Command::new("hyprctl")
                    .args(["keyword", "windowrulev2", "float,title:SpotSearch"])
                    .output();
                let _ = Command::new("hyprctl")
                    .args(["keyword", "windowrulev2", "pin,title:SpotSearch"])
                    .output();
                let _ = Command::new("hyprctl")
                    .args(["keyword", "windowrulev2", "stayfocused,title:SpotSearch"])
                    .output();
                let _ = Command::new("hyprctl")
                    .args(["keyword", "windowrulev2", "noborder,title:SpotSearch"])
                    .output();
                let _ = Command::new("hyprctl")
                    .args(["keyword", "windowrulev2", "noshadow,title:SpotSearch"])
                    .output();
                let _ = Command::new("hyprctl")
                    .args(["keyword", "windowrulev2", "center,title:SpotSearch"])
                    .output();
            }

            // Check for --toggle flag
            use tauri_plugin_cli::CliExt;
            if let Ok(matches) = app.cli().matches() {
                if let Some(arg) = matches.args.get("toggle") {
                    if arg.occurrences > 0 || arg.value.as_bool().unwrap_or(false) {
                        if let Some(window) = app.get_webview_window("main") {
                            if window.is_visible().unwrap_or(false) {
                                let _ = window.hide();
                            } else {
                                let _ = window.show();
                                let _ = window.set_focus();
                            }
                        }
                    }
                }
            }

            // System Tray Setup
            use tauri::menu::{Menu, MenuItem};
            use tauri::tray::{TrayIconBuilder, TrayIconEvent};

            let toggle_i =
                MenuItem::with_id(app, "toggle", "Show/Hide SpotSearch", true, None::<&str>)?;
            let quit_i = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&toggle_i, &quit_i])?;

            let _tray = TrayIconBuilder::new()
                .icon(app.default_window_icon().cloned().unwrap())
                .menu(&menu)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "toggle" => {
                        if let Some(window) = app.get_webview_window("main") {
                            if window.is_visible().unwrap_or(false) {
                                let _ = window.hide();
                            } else {
                                let _ = window.show();
                                let _ = window.set_focus();
                            }
                        }
                    }
                    "quit" => {
                        app.exit(0);
                    }
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: tauri::tray::MouseButton::Left,
                        ..
                    } = event
                    {
                        let app = tray.app_handle();
                        if let Some(window) = app.get_webview_window("main") {
                            if window.is_visible().unwrap_or(false) {
                                let _ = window.hide();
                            } else {
                                let _ = window.show();
                                let _ = window.set_focus();
                            }
                        }
                    }
                })
                .build(app)?;

            Ok(())
        })
        .on_window_event(|window, event| match event {
            tauri::WindowEvent::CloseRequested { api, .. } => {
                let _ = window.hide();
                api.prevent_close();
            }
            tauri::WindowEvent::Focused(focused) => {
                if window.label() == "main" {
                    #[cfg(target_os = "linux")]
                    {
                        if !is_wayland() {
                            if *focused {
                                grab_keyboard_x11(window);
                            } else {
                                ungrab_keyboard_x11(window);
                            }
                        }
                    }

                    if !focused {
                        let hide_on_blur = {
                            let config_state = window.state::<Mutex<AppConfig>>();
                            let config = config_state.lock().unwrap();
                            config.hide_on_blur
                        };
                        if hide_on_blur {
                            let _ = window.hide();
                        } else {
                            #[cfg(target_os = "linux")]
                            {
                                if !is_wayland() {
                                    let _ = window.set_focus();
                                }
                            }
                        }
                    }
                }
            }
            _ => {}
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
