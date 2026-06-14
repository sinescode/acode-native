/**
 * Acode Native — High-performance Rust-backed operations for Acode editor.
 *
 * Provides search, diff, zip, and checksum functionality via the
 * cordova.exec bridge to the Rust acode_native library.
 *
 * All methods return Promises. The native library is optional —
 * use {@link acodeNative.isAvailable} to check before calling.
 *
 * @namespace acode.native
 * @example
 *   if (await acode.native.isAvailable()) {
 *     const hash = await acode.native.hashFile('/sdcard/test.txt');
 *     console.log(hash.hex);
 *   }
 */

var exec = require('cordova/exec');

var SERVICE_NAME = 'AcodeNativePlugin';

// -------------------------------------------------------------------------
// Public API
// -------------------------------------------------------------------------

var acodeNative = {

    /**
     * Check whether the Rust native library is available on this device.
     * Returns false if the wrong ABI, library failed to load, or not installed.
     * @returns {Promise<boolean>}
     */
    isAvailable: function () {
        return new Promise(function (resolve) {
            exec(
                function (result) { resolve(result === 'true'); },
                function () { resolve(false); },
                SERVICE_NAME,
                'isAvailable',
                []
            );
        });
    },

    // =================================================================
    // Checksum
    // =================================================================

    /**
     * Compute the SHA-256 hash of a file.
     * @param {string} filePath - Absolute path to the file
     * @returns {Promise<{hex: string, input_size: number}>}
     */
    hashFile: function (filePath) {
        return new Promise(function (resolve, reject) {
            exec(
                function (result) {
                    var data = safeParse(result);
                    if (data.error) reject(new Error(data.error));
                    else resolve(data);
                },
                reject,
                SERVICE_NAME,
                'hashFile',
                [filePath]
            );
        });
    },

    /**
     * Compute SHA-256 hash of a string or text buffer.
     * @param {string} data - The text data to hash
     * @returns {Promise<{hex: string, input_size: number}>}
     */
    hashBytes: function (data) {
        return new Promise(function (resolve, reject) {
            exec(
                function (result) {
                    var data = safeParse(result);
                    if (data.error) reject(new Error(data.error));
                    else resolve(data);
                },
                reject,
                SERVICE_NAME,
                'hashBytes',
                [String(data)]
            );
        });
    },

    // =================================================================
    // Diff
    // =================================================================

    /**
     * Compute a text diff between two strings (Myers algorithm).
     * @param {string} oldText - Original text
     * @param {string} newText - Modified text
     * @param {number} [contextLines=3] - Lines of context around changes
     * @returns {Promise<{changes: Array<{tag:string, value:string}>, additions: number, deletions: number, similarity: number, unified: string}>}
     */
    diff: function (oldText, newText, contextLines) {
        return new Promise(function (resolve, reject) {
            exec(
                function (result) {
                    var data = safeParse(result);
                    if (data.error) reject(new Error(data.error));
                    else resolve(data);
                },
                reject,
                SERVICE_NAME,
                'diff',
                [oldText, newText, contextLines || 3]
            );
        });
    },

    // =================================================================
    // ZIP
    // =================================================================

    /**
     * List all entries in a ZIP archive.
     * @param {ArrayBuffer|Uint8Array} zipData - Raw ZIP bytes
     * @returns {Promise<Array<ZipEntry>>}
     *
     * @typedef {object} ZipEntry
     * @property {string} name - Entry path in archive
     * @property {boolean} is_dir
     * @property {number} size - Uncompressed size
     * @property {number} compressed_size
     * @property {string} last_modified - ISO 8601 date
     * @property {string} comment
     */
    zipList: function (zipData) {
        return new Promise(function (resolve, reject) {
            var b64 = _toBase64(zipData);
            exec(
                function (result) {
                    var data = safeParse(result);
                    if (data.error) reject(new Error(data.error));
                    else resolve(data);
                },
                reject,
                SERVICE_NAME,
                'zipList',
                [b64]
            );
        });
    },

    /**
     * Extract a ZIP archive to a target directory.
     * Includes zip-slip path traversal protection.
     * @param {ArrayBuffer|Uint8Array} zipData - Raw ZIP bytes
     * @param {string} targetDir - Absolute path to extraction target
     * @returns {Promise<{extracted: string[], skipped: string[], count: number}>}
     */
    zipExtract: function (zipData, targetDir) {
        return new Promise(function (resolve, reject) {
            var b64 = _toBase64(zipData);
            exec(
                function (result) {
                    var data = safeParse(result);
                    if (data.error) reject(new Error(data.error));
                    else resolve(data);
                },
                reject,
                SERVICE_NAME,
                'zipExtract',
                [b64, targetDir]
            );
        });
    },

    /**
     * Read a single entry's content from a ZIP archive.
     * @param {ArrayBuffer|Uint8Array} zipData - Raw ZIP bytes
     * @param {string} entryName - Path inside the ZIP (e.g. "plugin.json")
     * @returns {Promise<{data_base64: string, size: number}>}
     *   Decode data_base64 with atob() for text, or _base64ToArrayBuffer() for binary.
     */
    zipReadEntry: function (zipData, entryName) {
        return new Promise(function (resolve, reject) {
            var b64 = _toBase64(zipData);
            exec(
                function (result) {
                    var data = safeParse(result);
                    if (data.error) reject(new Error(data.error));
                    else resolve(data);
                },
                reject,
                SERVICE_NAME,
                'zipReadEntry',
                [b64, entryName]
            );
        });
    },

    /**
     * Sanitize a ZIP entry path to prevent zip-slip / path traversal attacks.
     * Same logic as installPlugin.js:sanitizeZipPath().
     * @param {string} path - Raw ZIP entry path
     * @returns {Promise<{safe?: string, unsafe?: boolean}>}
     */
    sanitizeZipPath: function (path) {
        return new Promise(function (resolve, reject) {
            exec(
                function (result) {
                    var data = safeParse(result);
                    if (data.error) reject(new Error(data.error));
                    else resolve(data);
                },
                reject,
                SERVICE_NAME,
                'sanitizeZipPath',
                [path]
            );
        });
    },

    // =================================================================
    // Search
    // =================================================================

    /**
     * Search all files in a directory tree (parallel regex search).
     * Replaces the Web Worker search in sidebarApps/searchInFiles/.
     *
     * @param {string} rootDir - Absolute path to the search root directory
     * @param {string} searchString - The search query (literal or regex)
     * @param {object} [options] - Search options
     * @param {boolean} [options.caseSensitive=false] - Case-sensitive matching
     * @param {boolean} [options.wholeWord=false] - Whole-word matching (adds \b)
     * @param {boolean} [options.regexMode=false] - Treat searchString as regex
     * @param {string} [options.exclude] - Comma-separated exclusion globs
     * @param {string} [options.include] - Comma-separated inclusion globs
     * @returns {Promise<Array<FileSearchResult>>}
     *
     * @typedef {object} FileSearchResult
     * @property {string} path - Relative path from root
     * @property {string} url - Absolute file path
     * @property {string} name - Display filename
     * @property {Array<SearchMatch>} matches
     *
     * @typedef {object} SearchMatch
     * @property {string} match - The matched substring
     * @property {{start:{row:number,column:number}, end:{row:number,column:number}}} position
     * @property {string} renderText - Highlight text (capped at 50 chars)
     * @property {string} linePreview - Surrounding line with newlines as ⏎
     */
    searchFiles: function (rootDir, searchString, options) {
        return new Promise(function (resolve, reject) {
            var optsJson = options ? JSON.stringify(options) : '{}';
            exec(
                function (result) {
                    var data = safeParse(result);
                    if (data.error) reject(new Error(data.error));
                    else resolve(data);
                },
                reject,
                SERVICE_NAME,
                'searchFiles',
                [rootDir, searchString, optsJson]
            );
        });
    },

    /**
     * Replace all matches in a string (convenience wrapper).
     * Uses diff-based approach: applies regex replacement in Rust.
     * @param {string} content - Text content to replace in
     * @param {string} search - Search pattern
     * @param {string} replace - Replacement text
     * @param {object} [options] - Same options as searchFiles
     * @returns {Promise<string>} - Text with all replacements applied
     */
    replaceInContent: function (content, search, replace, options) {
        return new Promise(function (resolve, reject) {
            var optsJson = options ? JSON.stringify(options) : '{}';
            exec(
                function (result) {
                    var data = safeParse(result);
                    if (data.error) reject(new Error(data.error));
                    else resolve(data);
                },
                reject,
                SERVICE_NAME,
                'replaceInContent',
                [content, search, replace, optsJson]
            );
        });
    },

    // =================================================================
    // Filesystem (v0.3)
    // =================================================================

    /**
     * List directory entries (faster than JS fsOperation.lsDir).
     * @param {string} path - Absolute directory path
     * @returns {Promise<Array<DirEntry>>}
     *
     * @typedef {object} DirEntry
     * @property {string} name
     * @property {string} url - file:// URL
     * @property {boolean} isFile
     * @property {boolean} isDirectory
     * @property {boolean} isLink
     * @property {string} [type] - MIME type
     */
    fsLsDir: function (path) {
        return _stringCall(SERVICE_NAME, 'fsLsDir', [path]);
    },

    /**
     * Read a file with automatic encoding detection.
     * @param {string} path - Absolute file path
     * @param {string} [encoding="auto"] - Encoding hint ("auto", "utf-8", "raw", etc.)
     * @returns {Promise<{data_base64: string, size: number, encoding: string}>}
     *   data_base64 is the file content as base64. Decode with atob() for text.
     */
    fsReadFile: function (path, encoding) {
        return new Promise(function (resolve, reject) {
            exec(
                function (result) {
                    var data = safeParse(result);
                    if (data.error) reject(new Error(data.error));
                    else resolve(data);
                },
                reject,
                SERVICE_NAME,
                'fsReadFile',
                [path, encoding || 'auto']
            );
        });
    },

    /**
     * Write content to a file (atomic write: temp + rename).
     * @param {string} path - Absolute file path
     * @param {string} dataBase64 - File content as base64 string
     * @returns {Promise<{ok: true}>}
     */
    fsWriteFile: function (path, dataBase64) {
        return _stringCall(SERVICE_NAME, 'fsWriteFile', [path, dataBase64]);
    },

    /**
     * Recursively walk a directory tree (replaces fileList.getAllFiles).
     * @param {string} path - Root directory path
     * @returns {Promise<FileTree>}
     *
     * @typedef {object} FileTree
     * @property {string} name
     * @property {string} url - file:// URL
     * @property {boolean} isDirectory
     * @property {boolean} isFile
     * @property {number} size
     * @property {number} modifiedDate - ms since epoch
     * @property {Array<FileTree>} children - Sorted: dirs first, then alpha
     */
    fsWalkTree: function (path) {
        return _stringCall(SERVICE_NAME, 'fsWalkTree', [path]);
    },

    /**
     * Get file/directory metadata.
     * @param {string} path - Absolute path
     * @returns {Promise<{name: string, url: string, isFile: boolean, isDirectory: boolean, isLink: boolean, size: number, modifiedDate: number, canRead: boolean, canWrite: boolean, exists: boolean}>}
     */
    fsStat: function (path) {
        return _stringCall(SERVICE_NAME, 'fsStat', [path]);
    },

    /**
     * Create a directory (with parents if needed).
     * @param {string} parent - Parent directory path
     * @param {string} name - New directory name
     * @returns {Promise<{url: string}>} - file:// URL of created directory
     */
    fsCreateDir: function (parent, name) {
        return _stringCall(SERVICE_NAME, 'fsCreateDir', [parent, name]);
    },

    /**
     * Delete a file or directory (recursive for directories).
     * @param {string} path - Absolute path
     * @returns {Promise<{ok: true}>}
     */
    fsDelete: function (path) {
        return _stringCall(SERVICE_NAME, 'fsDelete', [path]);
    },

    // =================================================================
    // Encoding (v0.3)
    // =================================================================

    /**
     * Detect the character encoding of binary data.
     * Uses chardetng (Firefox-grade statistical detection) with BOM priority.
     * @param {string} dataBase64 - File content as base64
     * @returns {Promise<{encoding: string, confidence: number, language?: string}>}
     */
    detectEncoding: function (dataBase64) {
        return _stringCall(SERVICE_NAME, 'detectEncoding', [dataBase64]);
    },

    /**
     * Decode bytes to text using the specified encoding.
     * Supports 20+ encodings: UTF-8, UTF-16LE/BE, windows-1252,
     * ISO-8859-1/2/5/7/15, Shift_JIS, EUC-JP/KR, GBK, Big5, KOI8-R/U,
     * IBM866, MacRoman, MacCyrillic.
     * @param {string} dataBase64 - Encoded bytes as base64
     * @param {string} encoding - Target encoding name
     * @returns {Promise<{text: string, encoding: string, has_errors: boolean, error_count: number}>}
     */
    decode: function (dataBase64, encoding) {
        return _stringCall(SERVICE_NAME, 'decode', [dataBase64, encoding]);
    },

    /**
     * Encode text to bytes using the specified encoding.
     * @param {string} text - The text to encode
     * @param {string} encoding - Target encoding name
     * @returns {Promise<{data_base64: string, byte_length: number}>}
     */
    encode: function (text, encoding) {
        return _stringCall(SERVICE_NAME, 'encode', [text, encoding]);
    },

    /**
     * Get the full encoding catalog with labels and descriptions.
     * @returns {Promise<Array<{name: string, labels: string[], bom: string|null}>>}
     */
    getEncodings: function () {
        return _stringCall(SERVICE_NAME, 'getEncodings', []);
    },

    // =================================================================
    // Archive (v0.3)
    // =================================================================

    /**
     * List entries in an archive (tar, tar.gz, tar.bz2, tar.xz, tar.zst).
     * @param {ArrayBuffer|Uint8Array} archiveData - Raw archive bytes
     * @param {string} format - "tar", "tar.gz", "tar.bz2", "tar.xz", "tar.zst"
     * @returns {Promise<Array<ArchiveEntry>>}
     *
     * @typedef {object} ArchiveEntry
     * @property {string} name - Entry path in archive
     * @property {boolean} is_dir
     * @property {number} size
     * @property {number} modified - Unix timestamp
     * @property {number} mode - Unix file mode
     */
    archiveList: function (archiveData, format) {
        return new Promise(function (resolve, reject) {
            var b64 = _toBase64(archiveData);
            exec(
                function (result) {
                    var data = safeParse(result);
                    if (data.error) reject(new Error(data.error));
                    else resolve(data);
                },
                reject,
                SERVICE_NAME,
                'archiveList',
                [b64, format]
            );
        });
    },

    /**
     * Extract an archive to a target directory (with path traversal protection).
     * @param {ArrayBuffer|Uint8Array} archiveData - Raw archive bytes
     * @param {string} format - Archive format string
     * @param {string} targetDir - Absolute path to extract into
     * @returns {Promise<{extracted: string[], errors: string[], count: number}>}
     */
    archiveExtract: function (archiveData, format, targetDir) {
        return new Promise(function (resolve, reject) {
            var b64 = _toBase64(archiveData);
            exec(
                function (result) {
                    var data = safeParse(result);
                    if (data.error) reject(new Error(data.error));
                    else resolve(data);
                },
                reject,
                SERVICE_NAME,
                'archiveExtract',
                [b64, format, targetDir]
            );
        });
    },

    /**
     * Compress a directory to an archive.
     * @param {string} sourceDir - Absolute path to the directory to compress
     * @param {string} format - Archive format string
     * @returns {Promise<{data_base64: string, byte_length: number}>}
     */
    archiveCompress: function (sourceDir, format) {
        return _stringCall(SERVICE_NAME, 'archiveCompress', [sourceDir, format]);
    },

    // =================================================================
    // Color (v0.4)
    // =================================================================

    /**
     * Parse a CSS color string to RGBA — replaces Canvas API round-trip.
     * Supports hex (#rgb, #rrggbb, #rrggbbaa), rgb(), rgba(), hsl(), hsla(),
     * and 148 named CSS colors.
     * @param {string} colorStr - CSS color string to parse
     * @returns {Promise<{r: number, g: number, b: number, a: number, hex: string}>}
     */
    parseColor: function (colorStr) {
        return _stringCall(SERVICE_NAME, 'parseColor', [String(colorStr)]);
    },

    // =================================================================
    // Highlight (v0.4)
    // =================================================================

    /**
     * Highlight source code using syntect (Sublime Text syntax definitions).
     * Replaces the Lezer-based codeHighlight.js for 3-6x speedup.
     * @param {string} code - Source code to highlight
     * @param {string} language - Language identifier (e.g. "javascript", "rust")
     * @param {string} [theme="dark"] - Theme name ("dark", "light")
     * @returns {Promise<{html: string, language: string}>}
     */
    highlightCode: function (code, language, theme) {
        return _stringCall(SERVICE_NAME, 'highlightCode', [code, language, theme || 'dark']);
    },

    /**
     * Get the catalog of supported syntax highlighting languages.
     * @returns {Promise<Array<{name: string, display: string, extensions: string[]}>>}
     */
    listHighlightLanguages: function () {
        return _stringCall(SERVICE_NAME, 'listHighlightLanguages', []);
    },

    // =================================================================
    // Markdown (v0.4)
    // =================================================================

    /**
     * Render GitHub Flavored Markdown to HTML using comrak.
     * Replaces markdown-it + 6 plugins for 10-20x speedup.
     * @param {string} text - Markdown source text
     * @param {object} [options] - Render options (all default to true)
     * @param {boolean} [options.gfm=true] - GitHub Flavored Markdown extensions
     * @param {boolean} [options.headerAnchors=true] - Auto-generate heading IDs
     * @param {boolean} [options.taskLists=true] - Task list checkboxes
     * @param {boolean} [options.footnotes=true] - Footnote support
     * @param {boolean} [options.strikethrough=true] - Strikethrough
     * @param {boolean} [options.emoji=true] - Emoji shortcodes
     * @param {boolean} [options.githubAlerts=true] - GitHub-style alerts
     * @param {boolean} [options.syntaxHighlighting=true] - Code block highlighting
     * @param {boolean} [options.math=true] - Math detection (for KaTeX)
     * @param {boolean} [options.mermaid=true] - Mermaid diagram detection
     * @param {boolean} [options.escapeHtml=false] - Escape raw HTML
     * @returns {Promise<{html: string, has_math: boolean, has_mermaid: boolean}>}
     */
    renderMarkdown: function (text, options) {
        return new Promise(function (resolve, reject) {
            var optsJson = options ? JSON.stringify(options) : '{}';
            exec(
                function (result) {
                    var data = safeParse(result);
                    if (data.error) reject(new Error(data.error));
                    else resolve(data);
                },
                reject,
                SERVICE_NAME,
                'renderMarkdown',
                [text, optsJson]
            );
        });
    },

    // =================================================================
    // Sanitize (v0.4)
    // =================================================================

    /**
     * Sanitize HTML to prevent XSS using ammonia.
     * Replaces DOMPurify for 3-10x speedup.
     * @param {string} html - Raw HTML to sanitize
     * @param {string} [profile="standard"] - Sanitization profile: "strict", "standard", "permissive"
     * @returns {Promise<{html: string}>}
     */
    sanitizeHtml: function (html, profile) {
        return _stringCall(SERVICE_NAME, 'sanitizeHtml', [html, profile || 'standard']);
    }
};

// -------------------------------------------------------------------------
// Internal helpers
// -------------------------------------------------------------------------

/**
 * Generic string-argument native call. Parses JSON result, rejects on error.
 * @param {string} service - Cordova service name
 * @param {string} action - Action to dispatch
 * @param {string[]} args - String arguments
 * @returns {Promise<any>}
 */
function _stringCall(service, action, args) {
    return new Promise(function (resolve, reject) {
        exec(
            function (result) {
                var data = safeParse(result);
                if (data.error) reject(new Error(data.error));
                else resolve(data);
            },
            reject,
            service,
            action,
            args
        );
    });
}

/**
 * Convert ArrayBuffer or Uint8Array to a base64 string for transmission
 * across the Cordova bridge (which only handles string arguments).
 */
function _toBase64(buffer) {
    var bytes;
    if (buffer instanceof ArrayBuffer) {
        bytes = new Uint8Array(buffer);
    } else if (buffer instanceof Uint8Array) {
        bytes = buffer;
    } else {
        throw new TypeError('Expected ArrayBuffer or Uint8Array, got ' + typeof buffer);
    }
    var binary = '';
    var len = bytes.length;
    for (var i = 0; i < len; i++) {
        binary += String.fromCharCode(bytes[i]);
    }
    return btoa(binary);
}

/**
 * Convert a base64 string back to an ArrayBuffer.
 */
function _base64ToArrayBuffer(b64) {
    var binary = atob(b64);
    var len = binary.length;
    var bytes = new Uint8Array(len);
    for (var i = 0; i < len; i++) {
        bytes[i] = binary.charCodeAt(i);
    }
    return bytes.buffer;
}

/**
 * Safely parse JSON. Returns { error: ... } on failure.
 */
function safeParse(json) {
    try {
        return JSON.parse(json);
    } catch (e) {
        return { error: 'Failed to parse native response: ' + e.message };
    }
}

// Retain for external consumers
acodeNative._toBase64 = _toBase64;
acodeNative._base64ToArrayBuffer = _base64ToArrayBuffer;

module.exports = acodeNative;
