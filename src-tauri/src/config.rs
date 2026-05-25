use serde::{Deserialize, Serialize};
use std::fs;
use directories::BaseDirs;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ThemeColors {
    pub bg_color: String,
    pub text_color: String,
    pub text_dim: String,
    pub accent_bg: String,
    pub accent_bar: String,
    pub glow_color: String,
}

impl Default for ThemeColors {
    fn default() -> Self {
        Self {
            bg_color: "#2b2b2b".to_string(),
            text_color: "#f4f4f5".to_string(),
            text_dim: "#a1a1aa".to_string(),
            accent_bg: "rgba(139, 92, 246, 0.15)".to_string(),
            accent_bar: "#8560f6".to_string(),
            glow_color: "rgba(139, 92, 246, 0.12)".to_string(),
        }
    }
}

fn default_hide_on_blur() -> bool {
    true
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AppConfig {
    pub theme: ThemeColors,
    pub search_paths: Vec<String>,
    pub excluded_dirs: Vec<String>,
    pub excluded_extensions: Vec<String>,
    pub max_depth: usize,
    #[serde(default = "default_hide_on_blur")]
    pub hide_on_blur: bool,
}

impl Default for AppConfig {
    fn default() -> Self {
        let base_dirs = BaseDirs::new().expect("Failed to get base dirs");
        let home_dir = base_dirs.home_dir().to_string_lossy().to_string();

        Self {
            theme: ThemeColors::default(),
            search_paths: vec![home_dir],
            excluded_dirs: vec![
                "node_modules", ".git", ".cache", ".cargo", ".npm", ".rustup",
                "target", "Trash", ".local", ".config", ".mozilla", ".thunderbird",
                "__pycache__", ".pycache", ".venv", "venv", "env", ".env",
                ".tox", ".mypy_cache", ".pytest_cache", ".ruff_cache",
                "dist", "build", ".next", ".nuxt", ".svelte-kit",
                ".gradle", ".m2", ".ivy2",
                "vendor", "bower_components",
                ".steam", ".wine", "snap",
                ".thumbnails", ".Trash-1000",
            ]
            .into_iter()
            .map(String::from)
            .collect(),
            excluded_extensions: vec![
                "o", "so", "a", "dylib", "pyc", "pyo", "class", "jar",
                "lock", "log", "tmp", "swp", "swo",
                "min.js", "min.css", "map",
                "whl", "egg-info",
            ]
            .into_iter()
            .map(String::from)
            .collect(),
            max_depth: 7,
            hide_on_blur: true,
        }
    }
}

impl AppConfig {
    pub fn load() -> Self {
        let base_dirs = match BaseDirs::new() {
            Some(dirs) => dirs,
            None => return Self::default(),
        };

        let config_dir = base_dirs.config_dir().join("spotsearch");
        let config_path = config_dir.join("config.json");

        if config_path.exists() {
            if let Ok(content) = fs::read_to_string(&config_path) {
                if let Ok(config) = serde_json::from_str::<AppConfig>(&content) {
                    return config;
                }
            }
        }

        // If load fails or file doesn't exist, save and return default
        let default_config = Self::default();
        let _ = default_config.save();
        default_config
    }

    pub fn save(&self) -> Result<(), std::io::Error> {
        let base_dirs = match BaseDirs::new() {
            Some(dirs) => dirs,
            None => return Err(std::io::Error::new(std::io::ErrorKind::NotFound, "Could not find home directory")),
        };

        let config_dir = base_dirs.config_dir().join("spotsearch");
        if !config_dir.exists() {
            fs::create_dir_all(&config_dir)?;
        }

        let config_path = config_dir.join("config.json");
        let content = serde_json::to_string_pretty(self).unwrap();
        fs::write(config_path, content)?;
        Ok(())
    }
}
