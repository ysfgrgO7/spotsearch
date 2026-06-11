use base64::Engine;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::time::SystemTime;

#[derive(serde::Serialize, Clone)]
pub struct AppResult {
    pub name: String,
    pub exec: String,
    pub icon_data: Option<String>,
    pub desktop_file: String,
    pub categories: Option<String>,
    pub keywords: Option<String>,
    pub generic_name: Option<String>,
    pub terminal: bool,
    #[serde(skip)]
    pub name_lower: String,
    #[serde(skip)]
    pub exec_lower: String,
    #[serde(skip)]
    pub desktop_file_lower: String,
    #[serde(skip)]
    pub keywords_lower: String,
    #[serde(skip)]
    pub generic_name_lower: String,
}

struct CacheData {
    apps: Vec<AppResult>,
    dir_modified_times: HashMap<PathBuf, Option<SystemTime>>,
}

static APPS_CACHE: Mutex<Option<CacheData>> = Mutex::new(None);
static ICON_SEARCH_DIRS: OnceLock<Vec<String>> = OnceLock::new();

/// Get standard, flatpak, and user application directories.
fn get_application_dirs() -> Vec<PathBuf> {
    let mut paths = Vec::new();

    // 1. Read directories from $XDG_DATA_DIRS environment variable
    if let Ok(xdg_data_dirs) = std::env::var("XDG_DATA_DIRS") {
        for dir in xdg_data_dirs.split(':') {
            if !dir.is_empty() {
                let p = PathBuf::from(dir).join("applications");
                if p.exists() && !paths.contains(&p) {
                    paths.push(p);
                }
            }
        }
    }

    // 2. Add standard/fallback paths
    let standard_dirs = vec![
        "/usr/share/applications",
        "/usr/local/share/applications",
        "/var/lib/flatpak/exports/share/applications",
    ];

    for dir in standard_dirs {
        let p = PathBuf::from(dir);
        if p.exists() && !paths.contains(&p) {
            paths.push(p);
        }
    }

    // 3. Add home paths
    if let Some(home) = dirs::home_dir() {
        let home_paths = vec![
            home.join(".local/share/applications"),
            home.join(".local/share/flatpak/exports/share/applications"),
        ];
        for p in home_paths {
            if p.exists() && !paths.contains(&p) {
                paths.push(p);
            }
        }
    }

    paths
}

/// Dynamically returns or caches hicolor apps icon directories of all sizes.
fn get_icon_search_dirs() -> &'static [String] {
    ICON_SEARCH_DIRS.get_or_init(|| {
        let mut dirs = Vec::new();
        let sizes = ["48x48", "64x64", "128x128", "256x256", "32x32", "scalable"];

        // 1. Check directories from XDG_DATA_DIRS
        if let Ok(xdg_data_dirs) = std::env::var("XDG_DATA_DIRS") {
            for base_dir in xdg_data_dirs.split(':') {
                if base_dir.is_empty() {
                    continue;
                }
                let base_path = PathBuf::from(base_dir);
                for size in &sizes {
                    let p = base_path.join(format!("icons/hicolor/{}/apps", size));
                    if p.exists() {
                        if let Some(s) = p.to_str() {
                            dirs.push(s.to_string());
                        }
                    }
                }
            }
        }

        // 2. Add system fallbacks
        let fallback_bases = vec![
            "/usr/share",
            "/usr/local/share",
            "/var/lib/flatpak/exports/share",
        ];

        for base in fallback_bases {
            let base_path = PathBuf::from(base);
            for size in &sizes {
                let p = base_path.join(format!("icons/hicolor/{}/apps", size));
                if p.exists() {
                    if let Some(s) = p.to_str() {
                        let s_str = s.to_string();
                        if !dirs.contains(&s_str) {
                            dirs.push(s_str);
                        }
                    }
                }
            }
        }

        // 3. Add home paths
        if let Some(home) = dirs::home_dir() {
            let home_bases = vec![
                home.join(".local/share"),
                home.join(".local/share/flatpak/exports/share"),
            ];
            for base_path in home_bases {
                for size in &sizes {
                    let p = base_path.join(format!("icons/hicolor/{}/apps", size));
                    if p.exists() {
                        if let Some(s) = p.to_str() {
                            let s_str = s.to_string();
                            if !dirs.contains(&s_str) {
                                dirs.push(s_str);
                            }
                        }
                    }
                }
            }

            // Also check ~/.icons
            let p = home.join(".icons");
            if p.exists() {
                if let Some(s) = p.to_str() {
                    let s_str = s.to_string();
                    if !dirs.contains(&s_str) {
                        dirs.push(s_str);
                    }
                }
            }
        }

        // Also include /usr/share/pixmaps
        let pixmaps = "/usr/share/pixmaps".to_string();
        if Path::new(&pixmaps).exists() && !dirs.contains(&pixmaps) {
            dirs.push(pixmaps);
        }

        dirs
    })
}

pub fn get_apps() -> Vec<AppResult> {
    let mut cache_lock = match APPS_CACHE.lock() {
        Ok(guard) => guard,
        Err(_) => return Vec::new(),
    };

    let dirs = get_application_dirs();

    // Check if we need to reload
    let mut needs_reload = cache_lock.is_none();

    if let Some(ref data) = *cache_lock {
        // Compare folder modification times
        for dir in &dirs {
            let current_mtime = fs::metadata(dir).and_then(|m| m.modified()).ok();

            let cached_mtime = data.dir_modified_times.get(dir).cloned().flatten();

            if current_mtime != cached_mtime {
                needs_reload = true;
                break;
            }
        }
    }

    if needs_reload {
        let mut apps = Vec::new();
        let mut dir_modified_times = HashMap::new();
        let mut seen_names = std::collections::HashSet::new();

        for dir in &dirs {
            let mtime = fs::metadata(dir).and_then(|m| m.modified()).ok();
            dir_modified_times.insert(dir.clone(), mtime);

            if let Ok(entries) = fs::read_dir(dir) {
                let mut sorted_entries: Vec<_> = entries.flatten().collect();
                sorted_entries.sort_by_key(|e| e.path());
                
                for entry in sorted_entries {
                    if entry.path().extension().and_then(|s| s.to_str()) == Some("desktop") {
                        if let Some(app) = parse_desktop_file(&entry.path()) {
                            if seen_names.insert(app.name.clone()) {
                                apps.push(app);
                            }
                        }
                    }
                }
            }
        }

        *cache_lock = Some(CacheData {
            apps: apps.clone(),
            dir_modified_times,
        });

        apps
    } else {
        cache_lock.as_ref().unwrap().apps.clone()
    }
}

fn parse_desktop_file(path: &Path) -> Option<AppResult> {
    let content = fs::read_to_string(path).ok()?;
    let mut name = String::new();
    let mut exec = String::new();
    let mut icon_name = None;
    let mut hidden = false;
    let mut nodisplay = false;
    let mut categories = None;
    let mut keywords = None;
    let mut generic_name = None;
    let mut terminal = false;

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
        } else if line.starts_with("Terminal=") {
            let val = line["Terminal=".len()..].trim().to_lowercase();
            terminal = val == "true" || val == "1";
        } else if line.starts_with("Categories=") && categories.is_none() {
            let raw = line["Categories=".len()..].to_string();
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
        } else if (line.starts_with("Keywords=") || line.starts_with("Keywords["))
            && keywords.is_none()
        {
            if let Some((_, val)) = line.split_once('=') {
                keywords = Some(val.to_string());
            }
        } else if (line.starts_with("GenericName=") || line.starts_with("GenericName["))
            && generic_name.is_none()
        {
            if let Some((_, val)) = line.split_once('=') {
                generic_name = Some(val.to_string());
            }
        }
    }

    if name.is_empty() || exec.is_empty() || hidden || nodisplay {
        return None;
    }

    // Resolve icon to a data URI
    let icon_data = icon_name.as_deref().and_then(resolve_icon_to_data_uri);

    let name_lower = name.to_lowercase();
    let exec_lower = exec.to_lowercase();
    let desktop_file_lower = path.to_string_lossy().to_string().to_lowercase();
    let keywords_lower = keywords.as_deref().unwrap_or("").to_lowercase();
    let generic_name_lower = generic_name.as_deref().unwrap_or("").to_lowercase();

    Some(AppResult {
        name,
        exec,
        icon_data,
        desktop_file: path.to_string_lossy().to_string(),
        categories,
        keywords,
        generic_name,
        terminal,
        name_lower,
        exec_lower,
        desktop_file_lower,
        keywords_lower,
        generic_name_lower,
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

    let search_dirs = get_icon_search_dirs();
    let extensions = ["png", "svg", "xpm"];

    for dir in search_dirs {
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
        None
    }
}

mod dirs {
    pub fn home_dir() -> Option<std::path::PathBuf> {
        std::env::var("HOME").ok().map(std::path::PathBuf::from)
    }
}
