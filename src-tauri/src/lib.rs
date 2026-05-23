use tauri::{command, AppHandle, Manager, State};
use std::process::Command;
use std::thread;
mod apps;
mod indexer;

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
            app_name.contains(term)
                || app_exec.contains(term)
                || app_file.contains(term)
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
    file_results.truncate(5);

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
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, _shortcut, event| {
                    if event.state() == tauri_plugin_global_shortcut::ShortcutState::Pressed {
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
                .build()
        )
        .invoke_handler(tauri::generate_handler![search, hide_window, open_result])
        .setup(|app| {
            // Setup indexer state
            let indexer_instance = indexer::Indexer::new().expect("Failed to init indexer");
            let indexer_clone = indexer_instance.clone();

            // Build index on a background thread — does NOT block the UI
            thread::spawn(move || {
                indexer_clone.build_index();
            });

            app.manage(indexer_instance);

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

            // Register hotkey
            use std::str::FromStr;
            use tauri_plugin_global_shortcut::GlobalShortcutExt;

            if let Ok(shortcut) = tauri_plugin_global_shortcut::Shortcut::from_str("Super+Space") {
                if !app.global_shortcut().is_registered(shortcut.clone()) {
                    let _ = app.global_shortcut().register(shortcut);
                }
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
            use tauri::tray::{TrayIconBuilder, TrayIconEvent};
            use tauri::menu::{Menu, MenuItem};

            let toggle_i = MenuItem::with_id(app, "toggle", "Show/Hide SpotSearch", true, None::<&str>)?;
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
                    if let TrayIconEvent::Click { button: tauri::tray::MouseButton::Left, .. } = event {
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
            _ => {}
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
