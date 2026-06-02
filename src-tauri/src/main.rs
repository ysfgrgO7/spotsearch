// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use directories::BaseDirs;
use std::env;
use std::io::Write;
use std::os::unix::net::UnixStream;

fn try_toggle_via_ipc() -> bool {
    let args: Vec<String> = env::args().collect();
    let has_toggle = args.iter().any(|arg| arg == "--toggle" || arg == "-t");
    if !has_toggle {
        return false;
    }

    if let Some(base_dirs) = BaseDirs::new() {
        let socket_path = base_dirs
            .config_dir()
            .join("launcher")
            .join("spotsearch.sock");
        if socket_path.exists() {
            if let Ok(mut stream) = UnixStream::connect(&socket_path) {
                if stream.write_all(b"toggle").is_ok() {
                    return true;
                }
            }
        }
    }
    false
}

fn main() {
    if try_toggle_via_ipc() {
        std::process::exit(0);
    }
    tauri_app_lib::run()
}
