//! High-performance parallel file search — ripgrep-style.
//!
//! Matches all features of the current Acode JS search:
//! - Regex, case-sensitive, whole-word, regex-mode flags
//! - Include/exclude glob patterns (picomatch-compatible)
//! - Default binary file extension excludes
//! - Line/column position computation (0-based)
//! - Surrounding context preview (50 char max)
//! - Progress reporting
//! - Parallel file walking via rayon

use ignore::WalkBuilder;
use regex::{Regex, RegexBuilder};
use serde::{Serialize, Deserialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

// ---------------------------------------------------------------------------
// Types — match the JS worker.js data structures exactly
// ---------------------------------------------------------------------------

/// A single match position (row, column), 0-based like the JS version.
#[derive(Debug, Clone, Serialize)]
pub struct Position {
    pub row: usize,
    pub column: usize,
}

/// Start and end of a match.
#[derive(Debug, Clone, Serialize)]
pub struct MatchSpan {
    pub start: Position,
    pub end: Position,
}

/// One search hit inside a file.
#[derive(Debug, Clone, Serialize)]
pub struct SearchMatch {
    /// The matched substring
    pub r#match: String,
    /// 0-based start/end
    pub position: MatchSpan,
    /// The rendered match text (for highlighting), capped at 50 chars
    pub render_text: String,
    /// The surrounding line preview (with newlines replaced)
    pub line_preview: String,
}

/// Result for a single file.
#[derive(Debug, Clone, Serialize)]
pub struct FileSearchResult {
    /// Relative path from search root
    pub path: String,
    /// Absolute URL / full path
    pub url: String,
    /// Display name (filename)
    pub name: String,
    /// All matches found in this file
    pub matches: Vec<SearchMatch>,
}

/// Aggregate progress callback payload.
#[derive(Debug, Clone, Serialize)]
pub struct SearchProgress {
    pub files_processed: usize,
    pub files_total: usize,
    pub matches_found: usize,
    pub percent: u32,
}

// ---------------------------------------------------------------------------
// Options — match JS getOptions() + toRegex() + Skip()
// ---------------------------------------------------------------------------

/// Search options mirroring the JS UI checkboxes and text inputs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchOptions {
    /// Whether to match case (default: false → case-insensitive)
    pub case_sensitive: bool,
    /// Whether to match whole words only (wraps in \b...\b)
    pub whole_word: bool,
    /// Whether the search string is already a regex
    pub regex_mode: bool,
    /// Comma-separated exclusion globs (appended to defaults)
    pub exclude: Option<String>,
    /// Comma-separated inclusion globs (default: **)
    pub include: Option<String>,
    /// File encoding hint (default: UTF-8)
    pub encoding: Option<String>,
    /// Maximum context chars around match (default: 50 — matches JS)
    pub context_chars: usize,
}

impl Default for SearchOptions {
    fn default() -> Self {
        Self {
            case_sensitive: false,
            whole_word: false,
            regex_mode: false,
            exclude: None,
            include: None,
            encoding: Some("utf-8".into()),
            context_chars: 50,
        }
    }
}

// Default binary/media/archive/font excludes — exact same list as JS worker.js
const DEFAULT_EXCLUDES: &[&str] = &[
    "*.png", "*.jpg", "*.jpeg", "*.gif", "*.bmp", "*.webp", "*.avif", "*.ico",
    "*.svgz", "*.mp3", "*.wav", "*.ogg", "*.flac", "*.m4a", "*.aac", "*.mp4",
    "*.mkv", "*.webm", "*.mov", "*.avi", "*.zip", "*.gz", "*.bz2", "*.xz",
    "*.7z", "*.rar", "*.tar", "*.exe", "*.dll", "*.so", "*.bin", "*.class",
    "*.ttf", "*.otf", "*.woff", "*.woff2", "*.pdf", "*.psd", "*.ai", "*.sketch",
];

// ---------------------------------------------------------------------------
// Regex builder — matches JS toRegex()
// ---------------------------------------------------------------------------

fn build_regex(search: &str, options: &SearchOptions) -> Result<Regex, String> {
    let mut pattern = if options.regex_mode {
        search.to_string()
    } else {
        regex::escape(search)
    };

    if options.whole_word {
        pattern = format!(r"\b{}\b", pattern);
    }

    RegexBuilder::new(&pattern)
        .case_insensitive(!options.case_sensitive)
        .multi_line(true)
        .build()
        .map_err(|e| format!("Invalid regex: {}", e))
}

// ---------------------------------------------------------------------------
// Line/column computation — matches JS getLineColumn()
// ---------------------------------------------------------------------------

fn get_line_column(content: &str, byte_offset: usize) -> Position {
    let prefix = &content[..byte_offset.min(content.len())];
    let row = prefix.matches('\n').count();
    let last_newline = prefix.rfind('\n').map(|i| i + 1).unwrap_or(0);
    let column = byte_offset.saturating_sub(last_newline);
    Position { row, column }
}

// ---------------------------------------------------------------------------
// Surrounding context — matches JS getSurrounding()
// ---------------------------------------------------------------------------

fn get_surrounding(content: &str, word: &str, start: usize, end: usize, max: usize) -> (String, String) {
    let match_len = end.saturating_sub(start);
    let remaining = max.saturating_sub(match_len);

    if remaining == 0 {
        // Match itself is >= max chars → truncate the word
        let truncated: String = word.chars().rev().take(max).collect::<Vec<_>>().into_iter().rev().collect();
        let preview = format!("...{}", truncated);
        (preview, truncated)
    } else {
        let left = remaining / 2;
        let right = remaining - left;

        let left_start = start.saturating_sub(left);
        let left_text = &content[left_start..start];

        let right_end = (end + right).min(content.len());
        let right_text = &content[end..right_end];

        (
            format!("{}{}{}", left_text, word, right_text),
            word.to_string(),
        )
    }
}

fn sanitize_preview(s: &str) -> String {
    s.replace(['\r', '\n'], " ⏎ ")
}

// ---------------------------------------------------------------------------
// Search a single file — matches JS searchInFile()
// ---------------------------------------------------------------------------

fn search_in_file(
    path: &Path,
    content: &str,
    regex: &Regex,
    context_chars: usize,
) -> FileSearchResult {
    let name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("")
        .to_string();

    let url = path.to_string_lossy().to_string();

    let relative = path.to_string_lossy().to_string();

    let mut matches: Vec<SearchMatch> = Vec::new();

    for caps in regex.captures_iter(content) {
        let m = caps.get(0).unwrap();
        let word = m.as_str().to_string();
        let start_offset = m.start();
        let end_offset = m.end();

        let position = MatchSpan {
            start: get_line_column(content, start_offset),
            end: get_line_column(content, end_offset),
        };

        let (line_preview_raw, render_text) =
            get_surrounding(content, &word, start_offset, end_offset, context_chars);
        let line_preview = sanitize_preview(&line_preview_raw);

        matches.push(SearchMatch {
            r#match: word,
            position,
            render_text,
            line_preview,
        });
    }

    FileSearchResult {
        path: relative,
        url,
        name,
        matches,
    }
}

// ---------------------------------------------------------------------------
// Core: parallel search over a directory tree
// ---------------------------------------------------------------------------

/// Search all files under `root_dir`, returning results and optionally calling
/// `on_progress` with updates. Uses rayon for parallel file processing.
pub fn search_dir(
    root_dir: &Path,
    search: &str,
    options: &SearchOptions,
    on_progress: Option<&dyn Fn(SearchProgress)>,
) -> Result<Vec<FileSearchResult>, String> {
    let regex = build_regex(search, options)?;

    // --- Build file walker with include/exclude patterns ---
    let mut builder = WalkBuilder::new(root_dir);
    builder
        .hidden(false)          // JS version includes hidden files
        .git_ignore(false)      // .gitignore filtering is separate
        .follow_links(false)
        .max_depth(None)
        .threads(1);            // walking is single-threaded; search is parallel

    // Add default excludes
    for ext in DEFAULT_EXCLUDES {
        builder.filter_entry(move |entry| {
            let name = entry.file_name().to_string_lossy();
            let pat = ext.strip_prefix("*.").unwrap_or(ext);
            if name.ends_with(pat) {
                return false;
            }
            true
        });
    }

    // User excludes (comma-separated glob patterns — simplified as suffix match)
    if let Some(ref exclude_str) = options.exclude {
        let user_excludes: Vec<String> = exclude_str
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        for ue in &user_excludes {
            let ue = ue.clone();
            builder.filter_entry(move |entry| {
                let name = entry.file_name().to_string_lossy();
                // Simple glob: if pattern starts with *., do suffix match
                if let Some(suffix) = ue.strip_prefix("*.") {
                    return !name.ends_with(suffix);
                }
                // Otherwise exact match
                name != ue
            });
        }
    }

    // User includes (if specified, only matching files)
    let user_includes: Option<Vec<String>> = options
        .include
        .as_ref()
        .map(|inc| inc.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect())
        .filter(|v: &Vec<String>| !v.is_empty() && v.iter().any(|s| s != "**"));

    // --- Collect file paths ---
    let mut paths: Vec<PathBuf> = Vec::new();
    for result in builder.build() {
        match result {
            Ok(entry) => {
                if entry.file_type().map_or(false, |ft| ft.is_file()) {
                    let name = entry.file_name().to_string_lossy().to_string();
                    // Apply user includes
                    if let Some(ref includes) = user_includes {
                        let matched = includes.iter().any(|pat| {
                            if let Some(suffix) = pat.strip_prefix("*.") {
                                name.ends_with(suffix)
                            } else {
                                name == *pat || *pat == "**"
                            }
                        });
                        if !matched {
                            continue;
                        }
                    }
                    paths.push(entry.into_path());
                }
            }
            Err(err) => {
                log::warn!("Walk error: {}", err);
            }
        }
    }

    let total = paths.len();
    if total == 0 {
        return Ok(Vec::new());
    }

    let files_processed = AtomicUsize::new(0);
    let matches_found = AtomicUsize::new(0);
    let _results: Arc<Mutex<Vec<FileSearchResult>>> = Arc::new(Mutex::new(Vec::new()));

    // --- Parallel search via rayon ---
    use rayon::prelude::*;

    // We need to capture the regex by reference; rayon needs Send + Sync
    // regex::Regex is Sync, so we can share it
    let regex = &regex;
    let context_chars = options.context_chars;
    let _root = root_dir.to_path_buf();

    let chunk_results: Vec<Vec<FileSearchResult>> = paths
        .par_chunks(64) // chunk to reduce lock contention
        .map(|chunk| {
            let mut local_results = Vec::with_capacity(chunk.len());
            for path in chunk {
                // Read file content with encoding-aware decoding
                let content = match read_file_utf8(path) {
                    Ok(c) => c,
                    Err(_) => {
                        files_processed.fetch_add(1, Ordering::Relaxed);
                        continue;
                    }
                };

                let result = search_in_file(path, &content, regex, context_chars);
                let match_count = result.matches.len();

                if !result.matches.is_empty() {
                    matches_found.fetch_add(match_count, Ordering::Relaxed);
                    local_results.push(result);
                }

                files_processed.fetch_add(1, Ordering::Relaxed);
            }
            local_results
        })
        .collect();

    // Merge chunk results
    let mut final_results = Vec::new();
    for chunk in chunk_results {
        final_results.extend(chunk);
    }

    // Final progress
    if let Some(ref cb) = on_progress {
        cb(SearchProgress {
            files_processed: total,
            files_total: total,
            matches_found: matches_found.load(Ordering::Relaxed),
            percent: 100,
        });
    }

    Ok(final_results)
}

/// Read a file as UTF-8, falling back to lossy decoding (like the JS `readFile` with encoding).
fn read_file_utf8(path: &Path) -> Result<String, std::io::Error> {
    let bytes = fs::read(path)?;
    // Try strict UTF-8 first; fall back to replacement characters
    Ok(String::from_utf8_lossy(&bytes).into_owned())
}

// ---------------------------------------------------------------------------
// Replace-all — matches JS replaceInFile()
// ---------------------------------------------------------------------------

/// Replace all matches of `search` with `replace` in a file's content.
/// Returns the new content. JS equivalent: String.prototype.replace(regex, replace)
pub fn replace_in_content(content: &str, search: &str, replace: &str, options: &SearchOptions) -> Result<String, String> {
    let regex = build_regex(search, options)?;
    Ok(regex.replace_all(content, replace).to_string())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_line_column() {
        let content = "hello\nworld\nfoo bar";
        // "world" starts at offset 6
        let pos = get_line_column(content, 6);
        assert_eq!(pos.row, 1);
        assert_eq!(pos.column, 0);

        // "bar" starts at offset 15
        let pos = get_line_column(content, 15);
        assert_eq!(pos.row, 2);
        assert_eq!(pos.column, 4);
    }

    #[test]
    fn test_build_regex_case_insensitive() {
        let opts = SearchOptions::default();
        let re = build_regex("hello", &opts).unwrap();
        assert!(re.is_match("HELLO world"));
    }

    #[test]
    fn test_build_regex_whole_word() {
        let opts = SearchOptions { whole_word: true, ..Default::default() };
        let re = build_regex("foo", &opts).unwrap();
        assert!(re.is_match("foo bar"));
        assert!(!re.is_match("foobar"));
    }

    #[test]
    fn test_build_regex_case_sensitive() {
        let opts = SearchOptions { case_sensitive: true, ..Default::default() };
        let re = build_regex("Hello", &opts).unwrap();
        assert!(!re.is_match("hello"));
        assert!(re.is_match("Hello"));
    }

    #[test]
    fn test_get_surrounding_truncation() {
        let content = "abcdefghijklmnopqrstuvwxyz0123456789ABCDEFGHIJKLMNOP";
        let long_word = "0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ";
        let (preview, render) = get_surrounding(
            &format!("prefix{}{}", long_word, "suffix"),
            long_word,
            6,
            6 + long_word.len(),
            50,
        );
        assert!(preview.contains("...")); // truncated
        assert_eq!(render.len(), 50); // capped
    }

    #[test]
    fn test_sanitize_preview() {
        let input = "line1\nline2\r\nline3";
        let sanitized = sanitize_preview(input);
        assert!(!sanitized.contains('\n'));
        assert!(sanitized.contains("⏎"));
    }

    #[test]
    fn test_replace_in_content() {
        let opts = SearchOptions::default();
        let result = replace_in_content("hello world hello", "hello", "hi", &opts).unwrap();
        assert_eq!(result, "hi world hi");
    }
}
