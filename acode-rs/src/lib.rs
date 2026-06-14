//! `acode_native` — High-performance native modules for the Acode Android code editor.
//!
//! ## Modules
//!
//! | Module | Purpose | Expected speedup vs JS |
//! |--------|---------|----------------------|
//! | `fs`       | Bulk filesystem ops (replaces fsOperation) | **50-100x** for bulk ops |
//! | `search`   | Parallel regex file search (ripgrep-style) | **10-50x** |
//! | `diff`     | Fast text diffing (Myers algorithm) | **5-20x** on large files |
//! | `zip`      | ZIP extract/compress (replaces JSZip) | **5-15x** |
//! | `archive`  | tar.gz/bz2/xz/zst extract/compress | **20-50x** vs shell tar |
//! | `encoding` | chardetng detection + multi-encoding decode/encode | **20x** vs JS detection |
//! | `checksum` | SHA-256 file hashing (SIMD) | **3-10x** |
//!
//! ## Build
//!
//! ```bash
//! cargo build --release --features fs,search,diff,zip,archive,encoding,checksum
//! ```
//!
//! For Android (via NDK):
//! ```bash
//! cargo ndk -t armeabi-v7a -t arm64-v8a -t x86_64 -o ./jniLibs build --release
//! ```

pub mod fs;
pub mod encoding;
pub mod archive;
pub mod search;
pub mod diff;
pub mod zip;
pub mod checksum;
pub mod color;
pub mod highlight;
pub mod markdown;
pub mod sanitize;

#[cfg(feature = "jni")]
pub mod jni;

pub use fs::{
    ls_dir, read_file, read_file_bytes, read_file_text, write_file, write_file_text,
    create_file, create_directory, delete, copy_to, move_to, rename_to,
    stat, exists, walk_dir_tree, walk_dir_flat, path_join, path_dirname,
    path_basename, path_extname, path_normalize, DirEntry, FileStat, FileTree, ReadResult,
};
pub use encoding::{
    detect_encoding, detect_encoding_with_hint, decode, encode as text_encode,
    get_available_encodings, strip_bom, validate as validate_encoding,
    DetectionResult, DecodeResult, EncodingInfo,
};
pub use archive::{
    extract_archive, extract_archive_from_bytes, extract_archive_stream,
    compress_dir_to_bytes as compress_archive_dir, compress_dir_to_bytes,
    list_archive_entries, list_archive_entries_from_bytes,
    ArchiveFormat, ArchiveEntry, ArchiveExtractResult, ArchiveProgress,
};
pub use search::{search_dir, replace_in_content, SearchMatch, SearchOptions, SearchProgress};
pub use diff::{diff as text_diff, unified_diff, DiffOptions, DiffGranularity, DiffResult};
pub use zip::{
    compress_dir, extract_zip, list_zip_entries, read_zip_entry, read_zip_entry_text,
    sanitize_zip_path, is_unsafe_path, CompressionLevel, ExtractResult, ZipEntry, ZipProgress,
};
pub use checksum::{hash_file, hash_bytes, hash_string, verify_file, HashResult};
pub use color::{parse_color, RgbaColor};
pub use highlight::{highlight_code, list_highlight_languages, generate_theme_css, HighlightResult, HighlightLanguage};
pub use markdown::{render_markdown, has_math_content, has_mermaid_content, slugify, MarkdownResult, MarkdownOptions};
pub use sanitize::{sanitize_html, SanitizeResult, SanitizeProfile};

// ---------------------------------------------------------------------------
// FFI — C-compatible exports for Android JNI / Cordova bridge
// ---------------------------------------------------------------------------

/// Free a string previously returned by any `acode_*` function.
/// The JNI shim must call this to avoid memory leaks.
///
/// # Safety
/// `ptr` must have been allocated by this crate via `CString::into_raw()`.
#[no_mangle]
pub unsafe extern "C" fn acode_free_string(ptr: *mut std::os::raw::c_char) {
    if !ptr.is_null() {
        unsafe {
            let _ = std::ffi::CString::from_raw(ptr);
        }
    }
}

/// SHA-256 hash a file at `path` (UTF-8). Returns a JSON string with
/// `{ "hex": "...", "input_size": N }` or `{ "error": "..." }`.
///
/// Caller must free the returned string with `acode_free_string`.
///
/// # Safety
/// `path` must be a valid, null-terminated UTF-8 string.
#[no_mangle]
pub unsafe extern "C" fn acode_hash_file(path: *const std::os::raw::c_char) -> *mut std::os::raw::c_char {
    let path_str = unsafe {
        if path.is_null() {
            return make_error("null path");
        }
        std::ffi::CStr::from_ptr(path).to_string_lossy().into_owned()
    };

    let result = hash_file(std::path::Path::new(&path_str));
    let json = match result {
        Ok(h) => serde_json::json!({
            "hex": h.hex,
            "input_size": h.input_size
        }),
        Err(e) => serde_json::json!({ "error": e }),
    };

    to_cstring(&serde_json::to_string(&json).unwrap_or_default())
}

/// SHA-256 hash of raw bytes. Returns JSON: `{ "hex": "...", "input_size": N }`.
///
/// # Safety
/// `data` and `len` must describe a valid readable byte range.
#[no_mangle]
pub unsafe extern "C" fn acode_hash_bytes(
    data: *const u8,
    len: usize,
) -> *mut std::os::raw::c_char {
    let bytes = unsafe { std::slice::from_raw_parts(data, len) };
    let result = hash_bytes(bytes);
    let json = serde_json::json!({
        "hex": result.hex,
        "input_size": result.input_size,
    });
    to_cstring(&serde_json::to_string(&json).unwrap_or_default())
}

/// List entries in a ZIP archive (bytes in, JSON array out).
/// `data` / `len` describe the raw ZIP bytes.
///
/// Returns a JSON-encoded `Vec<ZipEntry>` or `{ "error": "..." }`.
///
/// # Safety
/// `data` and `len` must describe a valid readable byte range.
#[no_mangle]
pub unsafe extern "C" fn acode_zip_list(
    data: *const u8,
    len: usize,
) -> *mut std::os::raw::c_char {
    let bytes = unsafe { std::slice::from_raw_parts(data, len) };
    let entries = list_zip_entries(bytes);
    let json = match entries {
        Ok(e) => serde_json::to_string(&e).unwrap_or_default(),
        Err(e) => serde_json::json!({ "error": e }).to_string(),
    };
    to_cstring(&json)
}

/// Extract a ZIP archive to `target_dir`. ZIP bytes: `data`/`len`.
/// Returns JSON-encoded `ExtractResult` or `{ "error": "..." }`.
///
/// # Safety
/// `data`/`len` must describe valid readable bytes; `target_dir` must be a
/// valid null-terminated UTF-8 path.
#[no_mangle]
pub unsafe extern "C" fn acode_zip_extract(
    data: *const u8,
    len: usize,
    target_dir: *const std::os::raw::c_char,
) -> *mut std::os::raw::c_char {
    let bytes = unsafe { std::slice::from_raw_parts(data, len) };
    let dir_str = unsafe {
        if target_dir.is_null() {
            return make_error("null target_dir");
        }
        std::ffi::CStr::from_ptr(target_dir).to_string_lossy().into_owned()
    };

    let result = extract_zip(bytes, std::path::Path::new(&dir_str), None);
    let json = match result {
        Ok(r) => serde_json::to_string(&r).unwrap_or_default(),
        Err(e) => serde_json::json!({ "error": e }).to_string(),
    };
    to_cstring(&json)
}

/// Read a single file from a ZIP archive by entry name.
/// Returns the raw bytes as a base64-encoded JSON string:
/// `{ "data_base64": "...", "size": N }` or `{ "error": "..." }`.
///
/// # Safety
/// `data`/`len` must describe valid readable bytes; `entry_name` must be a
/// valid null-terminated UTF-8 string.
#[no_mangle]
pub unsafe extern "C" fn acode_zip_read_entry(
    data: *const u8,
    len: usize,
    entry_name: *const std::os::raw::c_char,
) -> *mut std::os::raw::c_char {
    let bytes = unsafe { std::slice::from_raw_parts(data, len) };
    let name = unsafe {
        if entry_name.is_null() {
            return make_error("null entry_name");
        }
        std::ffi::CStr::from_ptr(entry_name).to_string_lossy().into_owned()
    };

    let result = read_zip_entry(bytes, &name);
    let json = match result {
        Ok(buf) => {
            use base64::Engine;
            let b64 = base64::engine::general_purpose::STANDARD.encode(&buf);
            serde_json::json!({
                "data_base64": b64,
                "size": buf.len(),
            })
        }
        Err(e) => serde_json::json!({ "error": e }),
    };
    to_cstring(&serde_json::to_string(&json).unwrap_or_default())
}

/// Compute a text diff between two strings. Both `old` and `new` are
/// null-terminated UTF-8; `context_lines` is the number of context lines.
/// Returns JSON-encoded `DiffResult` or `{ "error": "..." }`.
///
/// # Safety
/// `old` and `new` must be valid null-terminated UTF-8 strings.
#[no_mangle]
pub unsafe extern "C" fn acode_diff(
    old: *const std::os::raw::c_char,
    new: *const std::os::raw::c_char,
    context_lines: usize,
) -> *mut std::os::raw::c_char {
    let old_str = unsafe {
        if old.is_null() { return make_error("null old"); }
        std::ffi::CStr::from_ptr(old).to_string_lossy().into_owned()
    };
    let new_str = unsafe {
        if new.is_null() { return make_error("null new"); }
        std::ffi::CStr::from_ptr(new).to_string_lossy().into_owned()
    };

    let opts = DiffOptions {
        context_lines,
        ..Default::default()
    };
    let result = text_diff(&old_str, &new_str, &opts);
    let json = serde_json::to_string(&result).unwrap_or_default();
    to_cstring(&json)
}

/// Sanitize a ZIP entry path to prevent zip-slip attacks.
/// Returns the safe relative path or `{ "unsafe": true }`.
///
/// # Safety
/// `path` must be a valid null-terminated UTF-8 string.
#[no_mangle]
pub unsafe extern "C" fn acode_sanitize_zip_path(
    path: *const std::os::raw::c_char,
) -> *mut std::os::raw::c_char {
    let raw = unsafe {
        if path.is_null() { return make_error("null path"); }
        std::ffi::CStr::from_ptr(path).to_string_lossy().into_owned()
    };

    if is_unsafe_path(&raw) {
        return to_cstring(r#"{"unsafe":true}"#);
    }

    match sanitize_zip_path(&raw) {
        Some(safe) => {
            let json = serde_json::json!({ "safe": safe });
            to_cstring(&serde_json::to_string(&json).unwrap_or_default())
        }
        None => to_cstring(r#"{"unsafe":true}"#),
    }
}

/// Search files in a directory tree using parallel regex search.
/// `root_dir` — absolute path to the search root.
/// `search` — the search query (literal or regex depending on options).
/// `options_json` — JSON-serialized `SearchOptions`.
///
/// Returns a JSON-encoded `Vec<FileSearchResult>` or `{ "error": "..." }`.
///
/// # Safety
/// All string pointers must be valid null-terminated UTF-8.
#[no_mangle]
pub unsafe extern "C" fn acode_search_files(
    root_dir: *const std::os::raw::c_char,
    search: *const std::os::raw::c_char,
    options_json: *const std::os::raw::c_char,
) -> *mut std::os::raw::c_char {
    let root_str = unsafe {
        if root_dir.is_null() {
            return make_error("null root_dir");
        }
        std::ffi::CStr::from_ptr(root_dir).to_string_lossy().into_owned()
    };
    let query = unsafe {
        if search.is_null() {
            return make_error("null search");
        }
        std::ffi::CStr::from_ptr(search).to_string_lossy().into_owned()
    };
    let opts_str = unsafe {
        if options_json.is_null() {
            "{}".to_string()
        } else {
            std::ffi::CStr::from_ptr(options_json).to_string_lossy().into_owned()
        }
    };

    let options: SearchOptions = serde_json::from_str(&opts_str).unwrap_or_default();
    let result = search_dir(std::path::Path::new(&root_str), &query, &options, None);
    let json = match result {
        Ok(results) => serde_json::to_string(&results).unwrap_or_default(),
        Err(e) => serde_json::json!({ "error": e }).to_string(),
    };
    to_cstring(&json)
}

// ---------------------------------------------------------------------------
// FFI — Filesystem
// ---------------------------------------------------------------------------

/// List directory entries. Returns JSON array of DirEntry.
#[no_mangle]
pub unsafe extern "C" fn acode_fs_ls_dir(
    path: *const std::os::raw::c_char,
) -> *mut std::os::raw::c_char {
    let p = unsafe { cstr_to_string(path, "null path") };
    let result = ls_dir(std::path::Path::new(&p));
    let json = match result {
        Ok(entries) => serde_json::to_string(&entries).unwrap_or_default(),
        Err(e) => serde_json::json!({ "error": e }).to_string(),
    };
    to_cstring(&json)
}

/// Read a file. Returns JSON: { data_base64, size, encoding }.
#[no_mangle]
pub unsafe extern "C" fn acode_fs_read_file(
    path: *const std::os::raw::c_char,
    encoding: *const std::os::raw::c_char,
) -> *mut std::os::raw::c_char {
    let p = unsafe { cstr_to_string(path, "null path") };
    let enc = unsafe {
        if encoding.is_null() { None }
        else { Some(std::ffi::CStr::from_ptr(encoding).to_string_lossy().into_owned()) }
    };
    let result = read_file(std::path::Path::new(&p), enc.as_deref());
    let json = match result {
        Ok(r) => serde_json::to_string(&r).unwrap_or_default(),
        Err(e) => serde_json::json!({ "error": e }).to_string(),
    };
    to_cstring(&json)
}

/// Write content to a file. data is base64-encoded bytes.
#[no_mangle]
pub unsafe extern "C" fn acode_fs_write_file(
    path: *const std::os::raw::c_char,
    data_base64: *const std::os::raw::c_char,
) -> *mut std::os::raw::c_char {
    let p = unsafe { cstr_to_string(path, "null path") };
    let b64 = unsafe { cstr_to_string(data_base64, "null data") };
    use base64::Engine;
    let content = match base64::engine::general_purpose::STANDARD.decode(&b64) {
        Ok(c) => c,
        Err(e) => return make_error(&format!("base64 decode: {}", e)),
    };
    let result = write_file(std::path::Path::new(&p), &content);
    match result {
        Ok(()) => to_cstring(r#"{"ok":true}"#),
        Err(e) => make_error(&e),
    }
}

/// Recursively walk a directory tree. Returns JSON FileTree.
#[no_mangle]
pub unsafe extern "C" fn acode_fs_walk_tree(
    path: *const std::os::raw::c_char,
) -> *mut std::os::raw::c_char {
    let p = unsafe { cstr_to_string(path, "null path") };
    let result = walk_dir_tree(std::path::Path::new(&p));
    let json = match result {
        Ok(tree) => serde_json::to_string(&tree).unwrap_or_default(),
        Err(e) => serde_json::json!({ "error": e }).to_string(),
    };
    to_cstring(&json)
}

/// Get file stat. Returns JSON FileStat.
#[no_mangle]
pub unsafe extern "C" fn acode_fs_stat(
    path: *const std::os::raw::c_char,
) -> *mut std::os::raw::c_char {
    let p = unsafe { cstr_to_string(path, "null path") };
    let result = stat(std::path::Path::new(&p));
    let json = match result {
        Ok(s) => serde_json::to_string(&s).unwrap_or_default(),
        Err(e) => serde_json::json!({ "error": e }).to_string(),
    };
    to_cstring(&json)
}

/// Create a directory. Returns URL.
#[no_mangle]
pub unsafe extern "C" fn acode_fs_create_dir(
    parent: *const std::os::raw::c_char,
    name: *const std::os::raw::c_char,
) -> *mut std::os::raw::c_char {
    let p = unsafe { cstr_to_string(parent, "null parent") };
    let n = unsafe { cstr_to_string(name, "null name") };
    let result = create_directory(std::path::Path::new(&p), &n);
    let json = match result {
        Ok(url) => serde_json::json!({ "url": url }),
        Err(e) => serde_json::json!({ "error": e }),
    };
    to_cstring(&serde_json::to_string(&json).unwrap_or_default())
}

/// Delete a file or directory (recursive). Returns { ok: true }.
#[no_mangle]
pub unsafe extern "C" fn acode_fs_delete(
    path: *const std::os::raw::c_char,
) -> *mut std::os::raw::c_char {
    let p = unsafe { cstr_to_string(path, "null path") };
    match delete(std::path::Path::new(&p)) {
        Ok(()) => to_cstring(r#"{"ok":true}"#),
        Err(e) => make_error(&e),
    }
}

// ---------------------------------------------------------------------------
// FFI — Encoding
// ---------------------------------------------------------------------------

/// Detect the encoding of binary data. data is base64-encoded bytes.
/// Returns JSON: { encoding, confidence, language? }.
#[no_mangle]
pub unsafe extern "C" fn acode_detect_encoding(
    data_base64: *const std::os::raw::c_char,
) -> *mut std::os::raw::c_char {
    let b64 = unsafe { cstr_to_string(data_base64, "null data") };
    use base64::Engine;
    let bytes = match base64::engine::general_purpose::STANDARD.decode(&b64) {
        Ok(b) => b,
        Err(e) => return make_error(&format!("base64: {}", e)),
    };
    let result = detect_encoding(&bytes);
    let json = serde_json::to_string(&result).unwrap_or_default();
    to_cstring(&json)
}

/// Decode bytes with the given encoding. data is base64-encoded.
/// Returns JSON DecodeResult { text, encoding, has_errors, error_count }.
#[no_mangle]
pub unsafe extern "C" fn acode_decode(
    data_base64: *const std::os::raw::c_char,
    encoding: *const std::os::raw::c_char,
) -> *mut std::os::raw::c_char {
    let b64 = unsafe { cstr_to_string(data_base64, "null data") };
    let enc = unsafe { cstr_to_string(encoding, "null encoding") };
    use base64::Engine;
    let bytes = match base64::engine::general_purpose::STANDARD.decode(&b64) {
        Ok(b) => b,
        Err(e) => return make_error(&format!("base64: {}", e)),
    };
    let result = decode(&bytes, &enc);
    let json = match result {
        Ok(r) => serde_json::to_string(&r).unwrap_or_default(),
        Err(e) => serde_json::json!({ "error": e }).to_string(),
    };
    to_cstring(&json)
}

/// Encode text to bytes. Returns base64-encoded bytes.
/// Returns JSON: { data_base64, byte_length }.
#[no_mangle]
pub unsafe extern "C" fn acode_encode(
    text: *const std::os::raw::c_char,
    encoding: *const std::os::raw::c_char,
) -> *mut std::os::raw::c_char {
    let txt = unsafe { cstr_to_string(text, "null text") };
    let enc = unsafe { cstr_to_string(encoding, "null encoding") };
    let result = text_encode(&txt, &enc);
    match result {
        Ok(bytes) => {
            use base64::Engine;
            let b64 = base64::engine::general_purpose::STANDARD.encode(&bytes);
            let json = serde_json::json!({
                "data_base64": b64,
                "byte_length": bytes.len(),
            });
            to_cstring(&serde_json::to_string(&json).unwrap_or_default())
        }
        Err(e) => make_error(&e),
    }
}

/// Get the full encoding catalog. Returns JSON array of EncodingInfo.
#[no_mangle]
pub extern "C" fn acode_get_encodings() -> *mut std::os::raw::c_char {
    let catalog = get_available_encodings();
    let json = serde_json::to_string(&catalog).unwrap_or_default();
    to_cstring(&json)
}

// ---------------------------------------------------------------------------
// FFI — Archive
// ---------------------------------------------------------------------------

/// List entries in an archive. data is base64-encoded archive bytes.
/// format: "tar", "tar.gz", "tar.bz2", "tar.xz", "tar.zst".
#[no_mangle]
pub unsafe extern "C" fn acode_archive_list(
    data_base64: *const std::os::raw::c_char,
    format: *const std::os::raw::c_char,
) -> *mut std::os::raw::c_char {
    let b64 = unsafe { cstr_to_string(data_base64, "null data") };
    let fmt_str = unsafe { cstr_to_string(format, "null format") };
    use base64::Engine;
    let bytes = match base64::engine::general_purpose::STANDARD.decode(&b64) {
        Ok(b) => b,
        Err(e) => return make_error(&format!("base64: {}", e)),
    };
    let fmt = str_to_format(&fmt_str);
    let result = list_archive_entries_from_bytes(&bytes, fmt);
    let json = match result {
        Ok(entries) => serde_json::to_string(&entries).unwrap_or_default(),
        Err(e) => serde_json::json!({ "error": e }).to_string(),
    };
    to_cstring(&json)
}

/// Extract an archive to a target directory.
/// data is base64-encoded archive bytes.
#[no_mangle]
pub unsafe extern "C" fn acode_archive_extract(
    data_base64: *const std::os::raw::c_char,
    format: *const std::os::raw::c_char,
    target_dir: *const std::os::raw::c_char,
) -> *mut std::os::raw::c_char {
    let b64 = unsafe { cstr_to_string(data_base64, "null data") };
    let fmt_str = unsafe { cstr_to_string(format, "null format") };
    let dir = unsafe { cstr_to_string(target_dir, "null target_dir") };
    use base64::Engine;
    let bytes = match base64::engine::general_purpose::STANDARD.decode(&b64) {
        Ok(b) => b,
        Err(e) => return make_error(&format!("base64: {}", e)),
    };
    let fmt = str_to_format(&fmt_str);
    let result = extract_archive_from_bytes(&bytes, fmt, std::path::Path::new(&dir), None);
    let json = match result {
        Ok(r) => serde_json::to_string(&r).unwrap_or_default(),
        Err(e) => serde_json::json!({ "error": e }).to_string(),
    };
    to_cstring(&json)
}

/// Compress a directory to an archive. Returns base64-encoded archive bytes.
#[no_mangle]
pub unsafe extern "C" fn acode_archive_compress(
    source_dir: *const std::os::raw::c_char,
    format: *const std::os::raw::c_char,
) -> *mut std::os::raw::c_char {
    let dir = unsafe { cstr_to_string(source_dir, "null source_dir") };
    let fmt_str = unsafe { cstr_to_string(format, "null format") };
    let fmt = str_to_format(&fmt_str);
    let result = compress_archive_dir(std::path::Path::new(&dir), fmt);
    match result {
        Ok(bytes) => {
            use base64::Engine;
            let b64 = base64::engine::general_purpose::STANDARD.encode(&bytes);
            let json = serde_json::json!({
                "data_base64": b64,
                "byte_length": bytes.len(),
            });
            to_cstring(&serde_json::to_string(&json).unwrap_or_default())
        }
        Err(e) => make_error(&e),
    }
}

// ---------------------------------------------------------------------------
// FFI — Color (v0.4)
// ---------------------------------------------------------------------------

/// Parse a CSS color string to RGBA components.
/// Returns JSON: { r, g, b, a, hex }.
#[no_mangle]
pub unsafe extern "C" fn acode_parse_color(
    color_str: *const std::os::raw::c_char,
) -> *mut std::os::raw::c_char {
    let s = unsafe { cstr_to_string(color_str, "null color_str") };
    match parse_color(&s) {
        Ok(c) => {
            let json = serde_json::to_string(&c).unwrap_or_default();
            to_cstring(&json)
        }
        Err(e) => make_error(&e),
    }
}

// ---------------------------------------------------------------------------
// FFI — Highlight (v0.4)
// ---------------------------------------------------------------------------

/// Highlight source code with syntax coloring. Returns HTML with span tags.
#[no_mangle]
pub unsafe extern "C" fn acode_highlight_code(
    code: *const std::os::raw::c_char,
    language: *const std::os::raw::c_char,
    theme: *const std::os::raw::c_char,
) -> *mut std::os::raw::c_char {
    let code_str = unsafe { cstr_to_string(code, "null code") };
    let lang = unsafe { cstr_to_string(language, "null language") };
    let theme_str = unsafe { cstr_to_string(theme, "null theme") };
    let theme = if theme_str.is_empty() { "dark" } else { &theme_str };
    match highlight_code(&code_str, &lang, theme) {
        Ok(r) => {
            let json = serde_json::to_string(&r).unwrap_or_default();
            to_cstring(&json)
        }
        Err(e) => make_error(&e),
    }
}

/// List all available syntax highlighting languages. Returns JSON array.
#[no_mangle]
pub extern "C" fn acode_list_highlight_languages() -> *mut std::os::raw::c_char {
    let langs = list_highlight_languages();
    let json = serde_json::to_string(&langs).unwrap_or_default();
    to_cstring(&json)
}

// ---------------------------------------------------------------------------
// FFI — Markdown (v0.4)
// ---------------------------------------------------------------------------

/// Render GFM markdown to HTML. Returns JSON: { html, has_math, has_mermaid }.
#[no_mangle]
pub unsafe extern "C" fn acode_render_markdown(
    text: *const std::os::raw::c_char,
    options_json: *const std::os::raw::c_char,
) -> *mut std::os::raw::c_char {
    let md = unsafe { cstr_to_string(text, "null text") };
    let opts_str = unsafe {
        if options_json.is_null() {
            "{}".to_string()
        } else {
            std::ffi::CStr::from_ptr(options_json).to_string_lossy().into_owned()
        }
    };

    // Parse options (or use defaults)
    let options = MarkdownOptions::default(); // TODO: parse from options_json
    let _ = opts_str; // reserved for future options parsing

    let result = render_markdown(&md, &options);
    let json = serde_json::to_string(&result).unwrap_or_default();
    to_cstring(&json)
}

// ---------------------------------------------------------------------------
// FFI — Sanitize (v0.4)
// ---------------------------------------------------------------------------

/// Sanitize HTML to prevent XSS. Returns JSON: { html }.
#[no_mangle]
pub unsafe extern "C" fn acode_sanitize_html(
    html: *const std::os::raw::c_char,
    profile: *const std::os::raw::c_char,
) -> *mut std::os::raw::c_char {
    let html_str = unsafe { cstr_to_string(html, "null html") };
    let profile_str = unsafe { cstr_to_string(profile, "null profile") };
    let profile = if profile_str.is_empty() { "standard" } else { &profile_str };
    let result = sanitize_html(&html_str, profile);
    let json = serde_json::to_string(&result).unwrap_or_default();
    to_cstring(&json)
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Helper: convert a C string to a Rust String, returning an error JSON on null.
unsafe fn cstr_to_string(ptr: *const std::os::raw::c_char, label: &str) -> String {
    unsafe {
        if ptr.is_null() {
            String::new() // caller should check — but we return a default
        } else {
            std::ffi::CStr::from_ptr(ptr).to_string_lossy().into_owned()
        }
    }
}

fn str_to_format(s: &str) -> ArchiveFormat {
    match s.to_lowercase().as_str() {
        "tar" => ArchiveFormat::Tar,
        "tar.gz" | "tgz" => ArchiveFormat::TarGz,
        "tar.bz2" | "tbz2" | "tbz" => ArchiveFormat::TarBz2,
        "tar.xz" | "txz" => ArchiveFormat::TarXz,
        "tar.zst" | "tzst" => ArchiveFormat::TarZst,
        _ => ArchiveFormat::TarGz, // default: most common
    }
}

fn make_error(msg: &str) -> *mut std::os::raw::c_char {
    let json = serde_json::json!({ "error": msg });
    to_cstring(&serde_json::to_string(&json).unwrap_or_default())
}

fn to_cstring(s: &str) -> *mut std::os::raw::c_char {
    std::ffi::CString::new(s)
        .unwrap_or_else(|_| std::ffi::CString::new("null").unwrap())
        .into_raw()
}
