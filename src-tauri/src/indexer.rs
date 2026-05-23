use directories::BaseDirs;
use rusqlite::{params, Connection, Result};
use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use walkdir::WalkDir;
use crate::SearchResult;

#[derive(Clone)]
pub struct Indexer {
    db_path: PathBuf,
    read_conn: Arc<Mutex<Connection>>,
}

/// Excluded directory names — junk that should never be indexed.
const EXCLUDED_DIRS: &[&str] = &[
    "node_modules", ".git", ".cache", ".cargo", ".npm", ".rustup",
    "target", "Trash", ".local", ".config", ".mozilla", ".thunderbird",
    "__pycache__", ".pycache", ".venv", "venv", "env", ".env",
    ".tox", ".mypy_cache", ".pytest_cache", ".ruff_cache",
    "dist", "build", ".next", ".nuxt", ".svelte-kit",
    ".gradle", ".m2", ".ivy2",
    "vendor", "bower_components",
    ".steam", ".wine", "snap",
    ".thumbnails", ".Trash-1000",
];

/// File extensions to completely skip (binary junk, compiled output, etc.)
const EXCLUDED_EXTENSIONS: &[&str] = &[
    "o", "so", "a", "dylib", "pyc", "pyo", "class", "jar",
    "lock", "log", "tmp", "swp", "swo",
    "min.js", "min.css", "map",
    "whl", "egg-info",
];

impl Indexer {
    pub fn new() -> Result<Self> {
        let base_dirs = BaseDirs::new().expect("Could not get base dirs");
        let config_dir = base_dirs.config_dir().join("launcher");
        if !config_dir.exists() {
            fs::create_dir_all(&config_dir).unwrap();
        }

        let db_path = config_dir.join("index.db");
        let conn = Connection::open(&db_path)?;

        // Enable WAL mode so reads and writes can happen concurrently
        conn.pragma_update(None, "journal_mode", "WAL")?;

        // Create the table if it doesn't exist
        conn.execute(
            "CREATE VIRTUAL TABLE IF NOT EXISTS files USING fts5(
                name,
                path UNINDEXED,
                extension UNINDEXED,
                modified UNINDEXED
            );",
            [],
        )?;

        Ok(Self {
            db_path: db_path.clone(),
            read_conn: Arc::new(Mutex::new(conn)),
        })
    }

    /// Build the index using a SEPARATE write connection so search queries
    /// are never blocked. WAL mode allows concurrent readers + single writer.
    pub fn build_index(&self) {
        // Open a dedicated write connection — does NOT touch self.read_conn
        let write_conn = match Connection::open(&self.db_path) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("[SpotSearch] Failed to open write connection: {}", e);
                return;
            }
        };
        let _ = write_conn.pragma_update(None, "journal_mode", "WAL");

        let base_dirs = BaseDirs::new().unwrap();
        let home_dir = base_dirs.home_dir();

        // Read XDG user directories from ~/.config/user-dirs.dirs
        let mut deep_dirs = parse_xdg_user_dirs(home_dir);

        // Also add common dev directories as extras (these aren't in the XDG spec)
        for extra in &["Projects", "Code", "dev", "work", "src"] {
            let p = home_dir.join(extra);
            if p.exists() && !deep_dirs.contains(&p) {
                deep_dirs.push(p);
            }
        }

        // Deduplicate and remove $HOME itself (we handle that separately with depth 1)
        deep_dirs.retain(|d| d != home_dir && d.exists());
        deep_dirs.sort();
        deep_dirs.dedup();

        let _ = write_conn.execute("DELETE FROM files", []);

        let mut stmt = write_conn
            .prepare("INSERT INTO files (name, path, extension, modified) VALUES (?1, ?2, ?3, ?4)")
            .unwrap();

        let mut count: usize = 0;

        // Index files directly in $HOME (depth 1 only — no recursion)
        if let Ok(entries) = fs::read_dir(home_dir) {
            for entry in entries.flatten() {
                if let Ok(ft) = entry.file_type() {
                    if ft.is_file() {
                        let path = entry.path();
                        let fname = path.file_name().unwrap_or_default().to_string_lossy();
                        if fname.starts_with('.') {
                            continue;
                        }
                        let ext = path.extension().unwrap_or_default().to_string_lossy();
                        if EXCLUDED_EXTENSIONS.contains(&ext.as_ref()) {
                            continue;
                        }
                        let _ = stmt.execute(params![fname, path.to_string_lossy(), ext, ""]);
                        count += 1;
                    }
                }
            }
        }

        // Deep scan known directories (up to depth 6)
        for dir in deep_dirs {
            if !dir.exists() {
                continue;
            }

            for entry in WalkDir::new(&dir)
                .max_depth(6)
                .follow_links(false)
                .into_iter()
                .filter_entry(|e| {
                    let file_name = e.file_name().to_string_lossy();
                    if file_name.starts_with('.') {
                        return false;
                    }
                    !EXCLUDED_DIRS.contains(&file_name.as_ref())
                })
            {
                if let Ok(entry) = entry {
                    if entry.file_type().is_file() {
                        let path = entry.path();
                        let name = path.file_name().unwrap_or_default().to_string_lossy();
                        let ext = path.extension().unwrap_or_default().to_string_lossy();

                        if EXCLUDED_EXTENSIONS.contains(&ext.as_ref()) {
                            continue;
                        }

                        let path_str = path.to_string_lossy();
                        let _ = stmt.execute(params![name, path_str, ext, ""]);
                        count += 1;
                    }
                }
            }
        }

        eprintln!("[SpotSearch] Indexed {} files", count);
    }

    /// Fuzzy search: pull candidates from FTS5 prefix match + LIKE fallback,
    /// then re-rank with a fuzzy scoring algorithm.
    pub fn search(&self, query: &str) -> Vec<SearchResult> {
        let query = query.trim();
        if query.is_empty() {
            return Vec::new();
        }

        let mut results = Vec::new();
        let lower_query = query.to_lowercase();

        if let Ok(conn) = self.read_conn.lock() {
            let first_term = lower_query.split_whitespace().next().unwrap_or(&lower_query);
            let fts_query = format!("{}*", first_term);

            // Pull up to 50 candidates from FTS5
            if let Ok(mut stmt) =
                conn.prepare("SELECT name, path, extension FROM files WHERE name MATCH ?1 LIMIT 50")
            {
                let rows = stmt.query_map(params![fts_query], |row| {
                    let name: String = row.get(0)?;
                    let path: String = row.get(1)?;
                    let ext: String = row.get(2)?;
                    Ok((name, path, ext))
                });

                if let Ok(rows) = rows {
                    for row in rows.flatten() {
                        let (name, path, ext) = row;
                        let score = fuzzy_score(&lower_query, &name.to_lowercase(), &ext);
                        if score > 0 {
                            results.push((
                                SearchResult {
                                    name,
                                    path: Some(path),
                                    icon_data: None,
                                    is_app: false,
                                    exec: None,
                                    subtitle: None,
                                },
                                score,
                            ));
                        }
                    }
                }
            }

            // LIKE fallback for terms that FTS5 prefix might miss
            let like_pattern = format!("%{}%", lower_query.replace(' ', "%"));
            if let Ok(mut stmt) = conn.prepare(
                "SELECT name, path, extension FROM files WHERE lower(name) LIKE ?1 LIMIT 30",
            ) {
                let rows = stmt.query_map(params![like_pattern], |row| {
                    let name: String = row.get(0)?;
                    let path: String = row.get(1)?;
                    let ext: String = row.get(2)?;
                    Ok((name, path, ext))
                });

                if let Ok(rows) = rows {
                    for row in rows.flatten() {
                        let (name, path, ext) = row;
                        if results.iter().any(|(r, _)| r.path.as_deref() == Some(&path)) {
                            continue;
                        }
                        let score = fuzzy_score(&lower_query, &name.to_lowercase(), &ext);
                        if score > 0 {
                            results.push((
                                SearchResult {
                                    name,
                                    path: Some(path),
                                    icon_data: None,
                                    is_app: false,
                                    exec: None,
                                    subtitle: None,
                                },
                                score,
                            ));
                        }
                    }
                }
            }
        }

        // Sort by score descending
        results.sort_by(|a, b| b.1.cmp(&a.1));
        results.into_iter().map(|(r, _)| r).take(10).collect()
    }
}

/// Simple fuzzy scoring
fn fuzzy_score(query: &str, name: &str, ext: &str) -> i32 {
    let mut score: i32 = 0;

    if name == query {
        return 1000;
    }
    if name.starts_with(query) {
        score += 500;
    }
    if name.contains(query) {
        score += 200;
    }
    if is_subsequence(query, name) {
        score += 50;
    }

    let terms: Vec<&str> = query.split_whitespace().collect();
    if terms.len() > 1 {
        let all_match = terms.iter().all(|t| name.contains(t));
        if all_match {
            score += 150;
        } else {
            return 0;
        }
    }

    if score == 0 {
        return 0;
    }

    // Media bonus
    let media_exts = [
        "png", "jpg", "jpeg", "gif", "webp", "svg", "bmp", "ico", "tiff",
        "mp4", "mkv", "avi", "mov", "webm", "flv", "wmv",
        "mp3", "flac", "wav", "ogg", "aac", "m4a", "opus",
        "pdf", "doc", "docx", "xls", "xlsx", "ppt", "pptx", "odt", "ods",
    ];
    if media_exts.contains(&ext) {
        score += 100;
    }

    score += (100_i32).saturating_sub(name.len() as i32);
    score
}

fn is_subsequence(needle: &str, haystack: &str) -> bool {
    let mut it = haystack.chars();
    for c in needle.chars() {
        loop {
            match it.next() {
                Some(h) if h == c => break,
                Some(_) => continue,
                None => return false,
            }
        }
    }
    true
}

/// Parse ~/.config/user-dirs.dirs to get the user's configured XDG directories.
fn parse_xdg_user_dirs(home: &std::path::Path) -> Vec<std::path::PathBuf> {
    let xdg_file = home.join(".config/user-dirs.dirs");
    let mut dirs = Vec::new();

    if let Ok(content) = fs::read_to_string(&xdg_file) {
        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            if let Some((_key, value)) = line.split_once('=') {
                let value = value.trim().trim_matches('"');
                let resolved = value.replace("$HOME", &home.to_string_lossy());
                let path = std::path::PathBuf::from(&resolved);
                if !dirs.contains(&path) {
                    dirs.push(path);
                }
            }
        }
    }

    if dirs.is_empty() {
        let defaults = ["Documents", "Downloads", "Desktop", "Pictures", "Videos", "Music"];
        for d in &defaults {
            dirs.push(home.join(d));
        }
    }

    dirs
}
