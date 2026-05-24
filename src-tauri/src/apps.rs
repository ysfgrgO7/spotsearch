use base64::Engine;
use std::fs;
use std::path::Path;
use std::sync::OnceLock;

#[derive(serde::Serialize, Clone)]
pub struct AppResult {
    pub name: String,
    pub exec: String,
    pub icon_data: Option<String>,
    pub desktop_file: String,
    pub categories: Option<String>,
}

static APPS_CACHE: OnceLock<Vec<AppResult>> = OnceLock::new();

pub fn get_apps() -> Vec<AppResult> {
    APPS_CACHE
        .get_or_init(|| {
            let mut apps = Vec::new();
            let dirs = vec!["/usr/share/applications", "/usr/local/share/applications"];

            // Also check ~/.local/share/applications
            let home_apps = dirs::home_dir()
                .map(|h| h.join(".local/share/applications"))
                .filter(|p| p.exists());

            for dir_path in dirs
                .iter()
                .map(|d| std::path::PathBuf::from(d))
                .chain(home_apps)
            {
                if !dir_path.exists() {
                    continue;
                }
                if let Ok(entries) = fs::read_dir(&dir_path) {
                    for entry in entries.flatten() {
                        if entry.path().extension().and_then(|s| s.to_str()) == Some("desktop") {
                            if let Some(app) = parse_desktop_file(&entry.path()) {
                                apps.push(app);
                            }
                        }
                    }
                }
            }
            apps
        })
        .clone()
}

fn parse_desktop_file(path: &Path) -> Option<AppResult> {
    let content = fs::read_to_string(path).ok()?;
    let mut name = String::new();
    let mut exec = String::new();
    let mut icon_name = None;
    let mut hidden = false;
    let mut nodisplay = false;
    let mut categories = None;

    let mut in_desktop_entry = false;
    for line in content.lines() {
        if line == "[Desktop Entry]" {
            in_desktop_entry = true;
            continue;
        } else if line.starts_with('[') {
            in_desktop_entry = false;
        }

        if !in_desktop_entry {
            continue;
        }

        if line.starts_with("Name=") && name.is_empty() {
            name = line["Name=".len()..].to_string();
        } else if line.starts_with("Exec=") && exec.is_empty() {
            exec = line["Exec=".len()..].to_string();
            exec = exec
                .replace("%u", "")
                .replace("%U", "")
                .replace("%f", "")
                .replace("%F", "")
                .replace("%c", "")
                .replace("%k", "");
            exec = exec.trim().to_string();
        } else if line.starts_with("Icon=") && icon_name.is_none() {
            icon_name = Some(line["Icon=".len()..].to_string());
        } else if line.starts_with("Hidden=true") {
            hidden = true;
        } else if line.starts_with("NoDisplay=true") {
            nodisplay = true;
        } else if line.starts_with("Categories=") && categories.is_none() {
            let raw = line["Categories=".len()..].to_string();
            // Pick the first meaningful category
            let cat = raw
                .split(';')
                .find(|c| {
                    !c.is_empty()
                        && *c != "Application"
                        && *c != "GNOME"
                        && *c != "GTK"
                        && *c != "KDE"
                        && *c != "Qt"
                })
                .unwrap_or("")
                .to_string();
            if !cat.is_empty() {
                categories = Some(cat);
            }
        }
    }

    if name.is_empty() || exec.is_empty() || hidden || nodisplay {
        return None;
    }

    // Resolve icon to a data URI
    let icon_data = icon_name.as_deref().and_then(resolve_icon_to_data_uri);

    Some(AppResult {
        name,
        exec,
        icon_data,
        desktop_file: path.to_string_lossy().to_string(),
        categories,
    })
}

/// Resolve an icon name (e.g. "firefox") to an absolute file path.
fn resolve_icon_path(icon_name: &str) -> Option<String> {
    // Already an absolute path
    if icon_name.starts_with('/') {
        if Path::new(icon_name).exists() {
            return Some(icon_name.to_string());
        }
        return None;
    }

    // If the name already has an extension, check if it exists directly
    if icon_name.contains('.') {
        let p = Path::new(icon_name);
        if p.exists() {
            return Some(icon_name.to_string());
        }
    }

    let search_dirs = [
        // Prefer 48x48 and 64x64 PNGs
        "/usr/share/icons/hicolor/48x48/apps",
        "/usr/share/icons/hicolor/64x64/apps",
        "/usr/share/icons/hicolor/128x128/apps",
        "/usr/share/icons/hicolor/256x256/apps",
        "/usr/share/icons/hicolor/32x32/apps",
        "/usr/share/icons/hicolor/scalable/apps",
        "/usr/share/pixmaps",
    ];

    let extensions = ["png", "svg", "xpm"];

    for dir in &search_dirs {
        for ext in &extensions {
            let candidate = format!("{}/{}.{}", dir, icon_name, ext);
            if Path::new(&candidate).exists() {
                return Some(candidate);
            }
        }
    }

    None
}

/// Read an icon file and convert it to a data URI (base64 for binary, utf8 for SVG).
fn resolve_icon_to_data_uri(icon_name: &str) -> Option<String> {
    let path = resolve_icon_path(icon_name)?;
    let data = fs::read(&path).ok()?;

    if path.ends_with(".svg") {
        Some(format!(
            "data:image/svg+xml;base64,{}",
            base64::engine::general_purpose::STANDARD.encode(&data)
        ))
    } else if path.ends_with(".png") {
        Some(format!(
            "data:image/png;base64,{}",
            base64::engine::general_purpose::STANDARD.encode(&data)
        ))
    } else {
        // For anything else (xpm, etc.), skip
        None
    }
}

mod dirs {
    pub fn home_dir() -> Option<std::path::PathBuf> {
        std::env::var("HOME").ok().map(std::path::PathBuf::from)
    }
}
