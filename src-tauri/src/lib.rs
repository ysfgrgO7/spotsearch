use std::process::Command;
use std::thread;
use tauri::{command, AppHandle, Manager, State};
mod apps;
mod indexer;
mod config;
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

        let matches = terms.iter().all(|term| {
            app_name.contains(term) || app_exec.contains(term) || app_file.contains(term)
        });

        if matches {
            app_results.push(SearchResult {
                name: app.name,
                path: None, // Don't expose .desktop path to UI
                icon_data: app.icon_data,
                is_app: true,
                exec: Some(app.exec.clone()),
                subtitle: app.categories,
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
    indexer: State<'_, indexer::Indexer>
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

#[cfg(target_os = "linux")]
fn is_wayland() -> bool {
    std::env::var("WAYLAND_DISPLAY").is_ok()
}

#[cfg(target_os = "linux")]
fn setup_wayland_grab(window: &tauri::WebviewWindow) {
    use gtk_layer_shell::{Edge, Layer, KeyboardMode, LayerShell};

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
            open_settings
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
