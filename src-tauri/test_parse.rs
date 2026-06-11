use std::str::FromStr;

fn main() {
    let s = tauri_plugin_global_shortcut::Shortcut::from_str("super+a");
    println!("{:?}", s);
}
