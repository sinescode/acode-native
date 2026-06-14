//! Fast ZIP extract/compress — drop-in replacement for JSZip in Acode.
//!
//! Matches the exact usage patterns in `installPlugin.js`:
//! - Load ZIP from bytes and inspect entries (zip.files[...])
//! - Extract to a directory with path traversal protection (zip-slip defense)
//! - Sanitize entry paths (matches JS `sanitizeZipPath`)
//! - Progress callback on extraction
//! - Compress a directory into a ZIP archive

use serde::Serialize;
use std::fs;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use zip::read::ZipArchive;
use zip::write::{SimpleFileOptions, ZipWriter};
use zip::CompressionMethod;

// ---------------------------------------------------------------------------
// Types — match JSZip .files[] and installPlugin.js data flow
// ---------------------------------------------------------------------------

/// A single entry in the ZIP, mirroring JSZip's file object.
#[derive(Debug, Clone, Serialize)]
pub struct ZipEntry {
    /// Full path within the archive (normalized to / separators)
    pub name: String,
    /// Whether this is a directory entry
    pub is_dir: bool,
    /// Uncompressed size in bytes
    pub size: u64,
    /// Compressed size in bytes
    pub compressed_size: u64,
    /// Last modified date (ISO 8601)
    pub last_modified: Option<String>,
    /// Comment on this entry, if any
    pub comment: String,
}

/// Result of a ZIP extraction operation.
#[derive(Debug, Clone, Serialize)]
pub struct ExtractResult {
    /// Files that were successfully extracted
    pub extracted: Vec<String>,
    /// Files that were skipped (unsafe paths)
    pub skipped: Vec<String>,
    /// Count of extracted entries
    pub count: usize,
}

/// Progress info sent to the callback during extract/compress.
#[derive(Debug, Clone, Serialize)]
pub struct ZipProgress {
    pub current_file: String,
    pub files_processed: usize,
    pub files_total: usize,
    pub bytes_processed: u64,
    pub bytes_total: u64,
    pub percent: u32,
}

// ---------------------------------------------------------------------------
// Path sanitization — matches JS sanitizeZipPath + isUnsafeAbsolutePath
// ---------------------------------------------------------------------------

/// Sanitize a ZIP entry path to prevent zip-slip attacks.
/// Matches the logic in `installPlugin.js` lines 339-364 exactly.
pub fn sanitize_zip_path(path: &str) -> Option<String> {
    if path.is_empty() {
        return Some(String::new());
    }

    let mut s = path.replace('\\', "/");

    // Strip leading slashes
    while s.starts_with('/') {
        s = s[1..].to_string();
    }

    // Strip Windows drive letter (e.g., C:/)
    if s.len() >= 2
        && s.as_bytes().get(1) == Some(&b':')
        && s.as_bytes()[0].is_ascii_alphabetic()
        && s.as_bytes().get(2) == Some(&b'/')
    {
        s = s[3..].to_string();
    }

    // Resolve . and .. components
    let parts: Vec<&str> = s.split('/').collect();
    let mut stack: Vec<&str> = Vec::new();
    for part in parts {
        match part {
            "" | "." => continue,
            ".." => {
                stack.pop();
            }
            _ => stack.push(part),
        }
    }

    let safe = stack.join("/");
    if safe.is_empty() {
        return None;
    }
    Some(safe)
}

/// Check if a path is unsafe (absolute, escapes root).
/// Matches `isUnsafeAbsolutePath` in installPlugin.js.
pub fn is_unsafe_path(path: &str) -> bool {
    let s = path.trim();
    if s.is_empty() {
        return false;
    }

    // Windows drive root
    if s.len() >= 2
        && s.as_bytes()[1] == b':'
        && (s.as_bytes().get(2) == Some(&b'/') || s.as_bytes().get(2) == Some(&b'\\'))
    {
        return true;
    }

    // Network path
    if s.starts_with("//") {
        return true;
    }

    // Leading slash → unsafe
    if s.starts_with('/') {
        return true;
    }

    // Dot-dot segments
    let normalized = s.replace('\\', "/");
    let parts: Vec<&str> = normalized.split('/').collect();
    parts.contains(&"..")
}

// ---------------------------------------------------------------------------
// Extract ZIP to directory
// ---------------------------------------------------------------------------

/// Extract a ZIP archive (from bytes) to a target directory.
///
/// Path traversal is prevented by sanitizing every entry path and
/// skipping entries that try to escape the target directory.
///
/// Calls `on_progress` periodically with extraction status.
pub fn extract_zip(
    zip_bytes: &[u8],
    target_dir: &Path,
    on_progress: Option<&dyn Fn(ZipProgress)>,
) -> Result<ExtractResult, String> {
    let cursor = io::Cursor::new(zip_bytes);
    let mut archive = ZipArchive::new(cursor)
        .map_err(|e| format!("Failed to open ZIP: {}", e))?;

    let total = archive.len();
    let mut extracted = Vec::new();
    let mut skipped = Vec::new();

    // Ensure target directory exists
    fs::create_dir_all(target_dir)
        .map_err(|e| format!("Failed to create target dir: {}", e))?;

    for i in 0..total {
        let mut entry = archive.by_index(i)
            .map_err(|e| format!("Failed to read entry {}: {}", i, e))?;

        let raw_name = entry.name().to_string();
        let is_dir = entry.is_dir() || raw_name.ends_with('/');

        // Safety check
        if is_unsafe_path(&raw_name) {
            if let Some(ref cb) = on_progress {
                cb(ZipProgress {
                    current_file: raw_name.clone(),
                    files_processed: i + 1,
                    files_total: total,
                    bytes_processed: 0,
                    bytes_total: 0,
                    percent: ((i + 1) as f64 / total as f64 * 100.0) as u32,
                });
            }
            skipped.push(raw_name);
            continue;
        }

        let sanitized = sanitize_zip_path(&raw_name);
        let Some(safe_path) = sanitized else {
            skipped.push(raw_name);
            continue;
        };

        let output_path = target_dir.join(&safe_path);

        // Double-check: ensure output is within target dir
        if !output_path.starts_with(target_dir) {
            skipped.push(raw_name);
            continue;
        }

        if is_dir {
            fs::create_dir_all(&output_path)
                .map_err(|e| format!("Failed to create dir {}: {}", safe_path, e))?;
            extracted.push(safe_path);
        } else {
            // Ensure parent directory exists
            if let Some(parent) = output_path.parent() {
                fs::create_dir_all(parent)
                    .map_err(|e| format!("Failed to create parent dir: {}", e))?;
            }

            let mut output_file = fs::File::create(&output_path)
                .map_err(|e| format!("Failed to create file {}: {}", safe_path, e))?;

            io::copy(&mut entry, &mut output_file)
                .map_err(|e| format!("Failed to extract {}: {}", safe_path, e))?;

            extracted.push(safe_path);
        }

        if let Some(ref cb) = on_progress {
            cb(ZipProgress {
                current_file: raw_name,
                files_processed: i + 1,
                files_total: total,
                bytes_processed: 0, // size tracking would require a wrapping reader
                bytes_total: 0,
                percent: ((i + 1) as f64 / total as f64 * 100.0) as u32,
            });
        }
    }

    Ok(ExtractResult {
        count: extracted.len(),
        extracted,
        skipped,
    })
}

// ---------------------------------------------------------------------------
// List ZIP entries (read-only inspection — matches JSZip zip.files keys)
// ---------------------------------------------------------------------------

/// List all entries in a ZIP archive, returning metadata for each.
pub fn list_zip_entries(zip_bytes: &[u8]) -> Result<Vec<ZipEntry>, String> {
    let cursor = io::Cursor::new(zip_bytes);
    let mut archive = ZipArchive::new(cursor)
        .map_err(|e| format!("Failed to open ZIP: {}", e))?;

    let mut entries = Vec::with_capacity(archive.len());
    for i in 0..archive.len() {
        let entry = archive.by_index(i)
            .map_err(|e| format!("Failed to read entry {}: {}", i, e))?;

        entries.push(ZipEntry {
            name: entry.name().to_string(),
            is_dir: entry.is_dir(),
            size: entry.size(),
            compressed_size: entry.compressed_size(),
            last_modified: entry.last_modified()
                .map(|dt| dt.to_string()),
            comment: entry.comment().to_string(),
        });
    }

    Ok(entries)
}

/// Read a single file's content from a ZIP archive by its entry name.
/// Equivalent to JSZip's `zip.files[name].async("text")` or `async("ArrayBuffer")`.
pub fn read_zip_entry(zip_bytes: &[u8], entry_name: &str) -> Result<Vec<u8>, String> {
    let cursor = io::Cursor::new(zip_bytes);
    let mut archive = ZipArchive::new(cursor)
        .map_err(|e| format!("Failed to open ZIP: {}", e))?;

    let mut entry = archive.by_name(entry_name)
        .map_err(|e| format!("Entry '{}' not found: {}", entry_name, e))?;

    let mut buf = Vec::with_capacity(entry.size() as usize);
    entry.read_to_end(&mut buf)
        .map_err(|e| format!("Failed to read entry '{}': {}", entry_name, e))?;

    Ok(buf)
}

/// Read a single file's content as a UTF-8 string.
pub fn read_zip_entry_text(zip_bytes: &[u8], entry_name: &str) -> Result<String, String> {
    let bytes = read_zip_entry(zip_bytes, entry_name)?;
    String::from_utf8(bytes)
        .map_err(|e| format!("Entry '{}' is not valid UTF-8: {}", entry_name, e))
}

// ---------------------------------------------------------------------------
// Compress directory to ZIP
// ---------------------------------------------------------------------------

/// Compress a directory into a ZIP archive (in-memory bytes).
///
/// Uses Deflate compression. Can be called with progress tracking.
pub fn compress_dir(
    source_dir: &Path,
    compression: CompressionLevel,
    on_progress: Option<&dyn Fn(ZipProgress)>,
) -> Result<Vec<u8>, String> {
    let mut buf = Vec::new();
    let mut zip = ZipWriter::new(io::Cursor::new(&mut buf));

    let options = SimpleFileOptions::default()
        .compression_method(match compression {
            CompressionLevel::Store => CompressionMethod::Stored,
            CompressionLevel::Deflate => CompressionMethod::Deflated,
            CompressionLevel::Bzip2 => CompressionMethod::Bzip2,
            CompressionLevel::Zstd => CompressionMethod::Zstd,
        })
        .unix_permissions(0o644);

    // Walk the directory
    let mut files: Vec<PathBuf> = Vec::new();
    walk_dir(source_dir, source_dir, &mut files)
        .map_err(|e| format!("Failed to walk directory: {}", e))?;

    let total = files.len();

    for (i, path) in files.iter().enumerate() {
        let relative = path.strip_prefix(source_dir)
            .map_err(|e| format!("Path error: {}", e))?;
        let name = relative.to_string_lossy().replace('\\', "/");

        if path.is_dir() {
            zip.add_directory(&name, options)
                .map_err(|e| format!("Failed to add directory {}: {}", name, e))?;
        } else {
            zip.start_file(&name, options)
                .map_err(|e| format!("Failed to start file {}: {}", name, e))?;
            let content = fs::read(path)
                .map_err(|e| format!("Failed to read {}: {}", name, e))?;
            zip.write_all(&content)
                .map_err(|e| format!("Failed to write {}: {}", name, e))?;
        }

        if let Some(ref cb) = on_progress {
            cb(ZipProgress {
                current_file: name,
                files_processed: i + 1,
                files_total: total,
                bytes_processed: 0,
                bytes_total: 0,
                percent: ((i + 1) as f64 / total as f64 * 100.0) as u32,
            });
        }
    }

    zip.finish()
        .map_err(|e| format!("Failed to finalize ZIP: {}", e))?;

    Ok(buf)
}

/// Compression level selector.
#[derive(Debug, Clone, Copy)]
pub enum CompressionLevel {
    /// No compression — fastest, largest output
    Store,
    /// Standard Deflate (zlib) — good balance
    Deflate,
    /// Bzip2 — better compression, slower
    Bzip2,
    /// Zstandard — modern, fast, good compression
    Zstd,
}

fn walk_dir(base: &Path, dir: &Path, files: &mut Vec<PathBuf>) -> io::Result<()> {
    if dir.is_dir() {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            files.push(path.clone());
            if path.is_dir() {
                walk_dir(base, &path, files)?;
            }
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn make_test_zip() -> Vec<u8> {
        let mut buf = Vec::new();
        let mut zip = ZipWriter::new(io::Cursor::new(&mut buf));
        let opts = SimpleFileOptions::default()
            .compression_method(CompressionMethod::Deflated);
        zip.start_file("plugin.json", opts).unwrap();
        zip.write_all(b"{\"id\":\"test\"}").unwrap();
        zip.start_file("main.js", opts).unwrap();
        zip.write_all(b"console.log(1)").unwrap();
        zip.add_directory("lib/", opts).unwrap();
        zip.finish().unwrap();
        buf
    }

    #[test]
    fn test_sanitize_zip_path_normal() {
        let result = sanitize_zip_path("src/main.js");
        assert_eq!(result, Some("src/main.js".to_string()));
    }

    #[test]
    fn test_sanitize_zip_path_leading_slash() {
        let result = sanitize_zip_path("/etc/passwd");
        assert!(result.is_some());
        assert!(!result.unwrap().starts_with('/'));
    }

    #[test]
    fn test_sanitize_zip_path_dotdot() {
        let result = sanitize_zip_path("../outside");
        assert_eq!(result, Some("outside".to_string()));
    }

    #[test]
    fn test_is_unsafe_absolute() {
        assert!(is_unsafe_path("/data/app"));
        assert!(is_unsafe_path("C:/windows"));
        assert!(is_unsafe_path("//network/share"));
        assert!(is_unsafe_path("../escape"));
    }

    #[test]
    fn test_is_unsafe_safe() {
        assert!(!is_unsafe_path("plugin/main.js"));
        assert!(!is_unsafe_path("lib/"));
    }

    #[test]
    fn test_list_zip_entries() {
        let zip_bytes = make_test_zip();
        let entries = list_zip_entries(&zip_bytes).unwrap();
        assert_eq!(entries.len(), 3); // plugin.json, main.js, lib/ (dir)
        let names: Vec<&str> = entries.iter().map(|e| e.name.as_str()).collect();
        assert!(names.contains(&"plugin.json"));
        assert!(names.contains(&"main.js"));
    }

    #[test]
    fn test_read_zip_entry_text() {
        let zip_bytes = make_test_zip();
        let text = read_zip_entry_text(&zip_bytes, "plugin.json").unwrap();
        assert!(text.contains("\"id\""));
    }

    #[test]
    fn test_extract_and_compress_roundtrip() {
        let tmp = std::env::temp_dir().join("acode_zip_test");
        fs::create_dir_all(&tmp).ok();

        // Create a small dir
        let sub = tmp.join("testpkg");
        fs::create_dir_all(&sub).unwrap();
        fs::write(sub.join("manifest.json"), "{}").unwrap();
        fs::write(sub.join("code.js"), "x=1").unwrap();

        let zip_bytes = compress_dir(&sub, CompressionLevel::Deflate, None).unwrap();
        assert!(!zip_bytes.is_empty());

        // Extract to another dir
        let extract_to = tmp.join("extracted");
        let result = extract_zip(&zip_bytes, &extract_to, None).unwrap();
        assert!(result.count >= 2);

        // Cleanup
        fs::remove_dir_all(&tmp).ok();
    }
}
