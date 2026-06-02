use crate::SearchResult;
use directories::BaseDirs;
use rusqlite::{params, Connection, Result};
use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use walkdir::WalkDir;

#[derive(Clone)]
pub struct Indexer {
    db_path: PathBuf,
    read_conn: Arc<Mutex<Connection>>,
}

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

        // Recreate the FTS5 table with 'path' as an indexed column.
        // We drop it and recreate it to ensure the schema matches exactly.
        if let Err(e) = conn.execute("DROP TABLE IF EXISTS files", []) {
            eprintln!("[SpotSearch] Warning: failed to drop table files: {}", e);
        }
        conn.execute(
            "CREATE VIRTUAL TABLE IF NOT EXISTS files USING fts5(
                name,
                path,
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
    /// are never blocked. SQLite Transaction ensures batched inserts take <200ms.
    #[allow(dead_code)]
    pub fn build_index(&self) {
        let config = crate::config::AppConfig::load();
        self.build_index_with_config(&config);
    }

    pub fn build_index_with_config(&self, config: &crate::config::AppConfig) {
        let mut write_conn = match Connection::open(&self.db_path) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("[SpotSearch] Failed to open write connection: {}", e);
                return;
            }
        };
        let _ = write_conn.pragma_update(None, "journal_mode", "WAL");
        let _ = write_conn.pragma_update(None, "synchronous", "NORMAL");

        let base_dirs = BaseDirs::new().unwrap();
        let home_dir = base_dirs.home_dir();

        let mut scan_dirs = Vec::new();
        for path_str in &config.search_paths {
            let path = PathBuf::from(path_str);
            if path.exists() {
                scan_dirs.push(path);
            }
        }

        // If no valid scan dirs configured, fall back to home dir
        if scan_dirs.is_empty() {
            scan_dirs.push(home_dir.to_path_buf());
        }

        // Start transaction for lightning fast batched inserts
        let tx = match write_conn.transaction() {
            Ok(t) => t,
            Err(e) => {
                eprintln!("[SpotSearch] Failed to start transaction: {}", e);
                return;
            }
        };

        let _ = tx.execute("DELETE FROM files", []);

        let mut count: usize = 0;
        {
            let mut stmt = match tx.prepare(
                "INSERT INTO files (name, path, extension, modified) VALUES (?1, ?2, ?3, ?4)",
            ) {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("[SpotSearch] Failed to prepare statement: {}", e);
                    return;
                }
            };

            for dir in scan_dirs {
                for entry in WalkDir::new(&dir)
                    .max_depth(config.max_depth)
                    .follow_links(false)
                    .into_iter()
                    .filter_entry(|e| {
                        let file_name = e.file_name().to_string_lossy();
                        if file_name.starts_with('.') {
                            return false;
                        }
                        !config.excluded_dirs.iter().any(|d| d == file_name.as_ref())
                    })
                {
                    if let Ok(entry) = entry {
                        if entry.file_type().is_file() {
                            let path = entry.path();
                            let name = path.file_name().unwrap_or_default().to_string_lossy();
                            let ext = path.extension().unwrap_or_default().to_string_lossy();

                            if config
                                .excluded_extensions
                                .iter()
                                .any(|ex| ex == ext.as_ref())
                            {
                                continue;
                            }

                            let path_str = path.to_string_lossy();
                            let _ = stmt.execute(params![name, path_str, ext, ""]);
                            count += 1;
                        }
                    }
                }
            }
        }

        if let Err(e) = tx.commit() {
            eprintln!("[SpotSearch] Failed to commit transaction: {}", e);
        } else {
            eprintln!("[SpotSearch] Indexed {} files in transaction", count);
        }
    }

    /// Fuzzy search: query matches against FTS5 table (indexing both name and path),
    /// supplement with substring LIKE fallback, and rank using advanced fuzzy scoring.
    pub fn search(&self, query: &str) -> Vec<SearchResult> {
        let query = query.trim();
        if query.is_empty() {
            return Vec::new();
        }

        let mut results = Vec::new();
        let lower_query = query.to_lowercase();

        // 1. Sanitize the query terms for FTS5
        let terms: Vec<String> = lower_query
            .split_whitespace()
            .map(|s| {
                s.chars()
                    .filter(|c| c.is_alphanumeric())
                    .collect::<String>()
            })
            .filter(|s| !s.is_empty())
            .collect();

        if terms.is_empty() {
            return Vec::new();
        }

        // Keep track of added paths to prevent duplicate candidates
        let mut seen_paths = std::collections::HashSet::new();

        if let Ok(conn) = self.read_conn.lock() {
            // Construct safe FTS5 query: e.g. "spot* AND lib*"
            let fts_query = terms
                .iter()
                .map(|t| format!("{}*", t))
                .collect::<Vec<_>>()
                .join(" AND ");

            // Query FTS5 table - matching against the virtual table name searches both indexed columns: name and path
            if let Ok(mut stmt) = conn
                .prepare("SELECT name, path, extension FROM files WHERE files MATCH ?1 LIMIT 300")
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
                        let score = fuzzy_score(query, &name, &path, &ext);
                        if score > 0 {
                            seen_paths.insert(path.clone());
                            results.push((
                                SearchResult {
                                    name,
                                    path: Some(path),
                                    icon_data: None,
                                    is_app: false,
                                    exec: None,
                                    subtitle: None,
                                    is_websearch: Some(false),
                                    url: None,
                                    is_terminal: Some(false),
                                },
                                score,
                            ));
                        }
                    }
                }
            }

            // 2. Substring LIKE fallback if we didn't get enough candidates or to cover non-word-boundary matches
            if results.len() < 150 {
                // Construct a LIKE pattern: %term1%term2%
                let like_pattern = format!("%{}%", terms.join("%"));
                if let Ok(mut stmt) = conn.prepare(
                    "SELECT name, path, extension FROM files WHERE lower(name) LIKE ?1 OR lower(path) LIKE ?1 LIMIT 150",
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
                            if seen_paths.contains(&path) {
                                continue;
                            }
                            let score = fuzzy_score(query, &name, &path, &ext);
                            if score > 0 {
                                seen_paths.insert(path.clone());
                                results.push((
                                    SearchResult {
                                        name,
                                        path: Some(path),
                                        icon_data: None,
                                        is_app: false,
                                        exec: None,
                                        subtitle: None,
                                        is_websearch: Some(false),
                                        url: None,
                                        is_terminal: Some(false),
                                    },
                                    score,
                                ));
                            }
                        }
                    }
                }
            }
        }

        // Sort by score descending, then alphabetically by name if scores are equal
        results.sort_by(|a, b| {
            b.1.cmp(&a.1)
                .then_with(|| a.0.name.len().cmp(&b.0.name.len()))
                .then_with(|| a.0.name.cmp(&b.0.name))
        });

        results.into_iter().map(|(r, _)| r).take(40).collect()
    }
}

/// Advanced fuzzy scoring evaluating filename similarity, subsequence clustering, directory depth, and priority paths.
fn fuzzy_score(query: &str, name: &str, path: &str, ext: &str) -> i32 {
    let query_lower = query.to_lowercase();
    let name_lower = name.to_lowercase();
    let path_lower = path.to_lowercase();

    if name_lower == query_lower {
        return 10000;
    }

    let mut score = 0;

    // 1. Evaluate filename match strength
    if name_lower.starts_with(&query_lower) {
        score += 5000;
    } else if let Some(idx) = name_lower.find(&query_lower) {
        // High bonus if substring starts at a word boundary
        if idx == 0
            || !name_lower
                .chars()
                .nth(idx - 1)
                .map_or(false, |c| c.is_alphanumeric())
        {
            score += 3000;
        } else {
            score += 1500;
        }
    } else if is_subsequence(&query_lower, &name_lower) {
        score += 800;
        score += score_subsequence_clustering(&query_lower, &name_lower);
    } else if path_lower.contains(&query_lower) {
        // Matches path, but not filename directly
        score += 400;
    } else if is_subsequence(&query_lower, &path_lower) {
        score += 100;
    }

    // 2. Evaluate multi-word term matches
    let terms: Vec<&str> = query_lower.split_whitespace().collect();
    if terms.len() > 1 {
        let name_matches = terms.iter().filter(|&&t| name_lower.contains(t)).count();
        let path_matches = terms.iter().filter(|&&t| path_lower.contains(t)).count();

        if name_matches == terms.len() {
            // All terms match the filename
            score += 2000;
        } else if name_matches + path_matches >= terms.len() {
            // Terms are distributed across filename and directory paths
            score += 1000 + (name_matches as i32 * 300);
        } else {
            // Not all terms matched, query is mismatch
            return 0;
        }
    } else if terms.len() == 1 {
        if score == 0 {
            return 0;
        }
    }

    // 3. Folder depth penalty (prefer shallow files)
    let slash_count = path.chars().filter(|&c| c == '/').count() as i32;
    score -= slash_count * 50;

    // 4. Name and path length penalty (prefer exact fits over huge strings)
    score -= (name.len() as i32) * 5;
    score -= (path.len() as i32) * 2;

    // 5. High-priority directory bonuses
    if path_lower.contains("/projects/") || path_lower.contains("/src/") {
        score += 400;
    }
    if path_lower.contains("/desktop/")
        || path_lower.contains("/documents/")
        || path_lower.contains("/downloads/")
    {
        score += 250;
    }

    // 6. Common code, document, and media formats bonus
    let important_exts = [
        "rs", "js", "ts", "py", "sh", "json", "html", "css", "md", "txt", // Code / text
        "pdf", "doc", "docx", "xls", "xlsx", "ppt", "pptx", // Documents
        "png", "jpg", "jpeg", "svg", "webp", // Graphics
        "mp3", "mp4", "mkv", "wav", // Media
    ];
    if important_exts.contains(&ext.to_lowercase().as_str()) {
        score += 150;
    }

    score
}

fn score_subsequence_clustering(needle: &str, haystack: &str) -> i32 {
    let mut score = 0;
    let mut needle_chars = needle.chars().peekable();
    let mut last_idx = None;
    let mut total_distance = 0;

    for (h_idx, h_char) in haystack.chars().enumerate() {
        if let Some(&n_char) = needle_chars.peek() {
            if h_char == n_char {
                needle_chars.next();
                if let Some(prev) = last_idx {
                    let dist = h_idx - prev;
                    total_distance += dist;
                }
                last_idx = Some(h_idx);
            }
        } else {
            break;
        }
    }

    if needle_chars.peek().is_none() && last_idx.is_some() {
        let needle_len = needle.len();
        if needle_len > 1 {
            let ideal_dist = needle_len - 1;
            let excess = total_distance - ideal_dist;
            score += (250_i32).saturating_sub((excess as i32) * 15);
        }
    }
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
#[allow(dead_code)]
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
        let defaults = [
            "Documents",
            "Downloads",
            "Desktop",
            "Pictures",
            "Videos",
            "Music",
        ];
        for d in &defaults {
            dirs.push(home.join(d));
        }
    }

    dirs
}
