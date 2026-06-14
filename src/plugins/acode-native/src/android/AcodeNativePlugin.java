package com.foxdebug.acode.nativeplugin;

import org.apache.cordova.CordovaPlugin;
import org.apache.cordova.CallbackContext;
import org.apache.cordova.PluginResult;
import org.apache.cordova.CordovaInterface;
import org.apache.cordova.CordovaWebView;
import org.json.JSONArray;
import org.json.JSONException;
import org.json.JSONObject;
import android.util.Base64;
import android.util.Log;

/**
 * Cordova plugin that bridges JavaScript to the acode_native Rust library via JNI.
 *
 * Data flow: JS (cordova.exec) → Java (this class + JNI) → Rust FFI → JSON
 *
 * All blocking native calls are dispatched to cordova.getThreadPool() to
 * avoid blocking the Cordova bridge thread.
 */
public class AcodeNativePlugin extends CordovaPlugin {

    private static final String TAG = "AcodeNativePlugin";
    private static volatile boolean libraryLoaded = false;
    private static final Object loadLock = new Object();

    // ---------------------------------------------------------------
    //  JNI native method declarations — match Rust #[no_mangle] FFI
    // ---------------------------------------------------------------

    /** SHA-256 hash a file at the given absolute path. Returns JSON. */
    private static native String nativeHashFile(String path);

    /** SHA-256 hash raw bytes. Returns JSON: { hex, input_size }. */
    private static native String nativeHashBytes(byte[] data, int len);

    /** List entries in a ZIP archive. Returns JSON array. */
    private static native String nativeZipList(byte[] data, int len);

    /** Extract a ZIP archive to a directory. Returns JSON: { extracted, skipped, count }. */
    private static native String nativeZipExtract(byte[] data, int len, String targetDir);

    /** Read a single entry from a ZIP. Returns JSON: { data_base64, size }. */
    private static native String nativeZipReadEntry(byte[] data, int len, String entryName);

    /** Compute text diff between two strings. Returns JSON: { changes, additions, deletions, similarity, unified }. */
    private static native String nativeDiff(String oldText, String newText, int contextLines);

    /** Sanitize a ZIP entry path to prevent zip-slip. Returns JSON. */
    private static native String nativeSanitizeZipPath(String path);

    /** Search files in a directory tree. Returns JSON array of FileSearchResult. */
    private static native String nativeSearchFiles(String rootDir, String search, String optionsJson);

    // --- Filesystem (v0.3) ---

    /** List directory entries. Returns JSON array of DirEntry. */
    private static native String nativeFsLsDir(String path);

    /** Read a file with encoding detection. Returns JSON: { data_base64, size, encoding }. */
    private static native String nativeFsReadFile(String path, String encoding);

    /** Write content (base64) to a file. Returns JSON: { ok: true }. */
    private static native String nativeFsWriteFile(String path, String dataBase64);

    /** Recursively walk a directory tree. Returns JSON FileTree. */
    private static native String nativeFsWalkTree(String path);

    /** Get file/directory metadata. Returns JSON FileStat. */
    private static native String nativeFsStat(String path);

    /** Create a directory. Returns JSON: { url }. */
    private static native String nativeFsCreateDir(String parent, String name);

    /** Delete a file or directory (recursive). Returns JSON: { ok: true }. */
    private static native String nativeFsDelete(String path);

    // --- Encoding (v0.3) ---

    /** Detect encoding of base64-encoded bytes. Returns JSON: { encoding, confidence, language? }. */
    private static native String nativeDetectEncoding(String dataBase64);

    /** Decode base64 bytes with a given encoding. Returns JSON: { text, encoding, has_errors, error_count }. */
    private static native String nativeDecode(String dataBase64, String encoding);

    /** Encode text to bytes (returned as base64). Returns JSON: { data_base64, byte_length }. */
    private static native String nativeEncode(String text, String encoding);

    /** Get the full encoding catalog. Returns JSON array of EncodingInfo. */
    private static native String nativeGetEncodings();

    // --- Archive (v0.3) ---

    /** List entries in an archive (base64 bytes). Returns JSON array of ArchiveEntry. */
    private static native String nativeArchiveList(String dataBase64, String format);

    /** Extract an archive (base64 bytes) to a target directory. Returns JSON ArchiveExtractResult. */
    private static native String nativeArchiveExtract(String dataBase64, String format, String targetDir);

    /** Compress a directory to an archive. Returns JSON: { data_base64, byte_length }. */
    private static native String nativeArchiveCompress(String sourceDir, String format);

    // --- Color / Markdown / Highlight / Sanitize (v0.4) ---

    /** Parse a CSS color string to RGBA. Returns JSON: { r, g, b, a, hex }. */
    private static native String nativeParseColor(String colorStr);

    /** Highlight source code with syntax coloring. Returns JSON: { html, language }. */
    private static native String nativeHighlightCode(String code, String language, String theme);

    /** List all available syntax highlighting languages. Returns JSON array. */
    private static native String nativeListHighlightLanguages();

    /** Render GFM markdown to HTML. Returns JSON: { html, has_math, has_mermaid }. */
    private static native String nativeRenderMarkdown(String text, String optionsJson);

    /** Sanitize HTML to prevent XSS. Returns JSON: { html }. */
    private static native String nativeSanitizeHtml(String html, String profile);

    /** Free a Rust-allocated C string. Called after consuming native return values. */
    private static native void nativeFreeString(long ptr);

    // ---------------------------------------------------------------
    //  Lifecycle
    // ---------------------------------------------------------------

    @Override
    public void initialize(CordovaInterface cordova, CordovaWebView webView) {
        super.initialize(cordova, webView);
        ensureLibraryLoaded();
    }

    @Override
    public void onDestroy() {
        super.onDestroy();
        Log.d(TAG, "Plugin destroyed");
    }

    /**
     * Attempt to load libacode_native.so exactly once.
     * Failure is non-fatal — callers check {@link #isAvailable()} before use.
     */
    private static void ensureLibraryLoaded() {
        if (libraryLoaded) return;
        synchronized (loadLock) {
            if (libraryLoaded) return;
            try {
                System.loadLibrary("acode_native");
                libraryLoaded = true;
                Log.i(TAG, "libacode_native.so loaded successfully");
            } catch (UnsatisfiedLinkError e) {
                Log.e(TAG, "Failed to load libacode_native.so: " + e.getMessage());
                // libraryLoaded stays false; all actions return errors gracefully
            }
        }
    }

    /** Whether the native library is loaded and operational. */
    public static boolean isAvailable() {
        return libraryLoaded;
    }

    // ---------------------------------------------------------------
    //  CordovaPlugin dispatch
    // ---------------------------------------------------------------

    @Override
    public boolean execute(String action, final JSONArray args,
                           final CallbackContext callbackContext) throws JSONException {

        // Fast-path: library availability check without thread pool
        if ("isAvailable".equals(action)) {
            callbackContext.success(isAvailable() ? "true" : "false");
            return true;
        }

        if (!libraryLoaded) {
            callbackContext.error("{\"error\":\"Native library not loaded on this device/ABI\"}");
            return true;
        }

        // All other actions run on the thread pool (they involve JNI calls)
        cordova.getThreadPool().execute(new Runnable() {
            @Override
            public void run() {
                try {
                    String result = dispatchAction(action, args);
                    callbackContext.success(result);
                } catch (Exception e) {
                    Log.e(TAG, "Error in action '" + action + "'", e);
                    String escaped = escapeJson(e.getMessage());
                    callbackContext.error("{\"error\":\"" + escaped + "\"}");
                }
            }
        });
        // Return true immediately — callback will be invoked from thread pool
        return true;
    }

    /**
     * Dispatch an action to the correct JNI native method.
     * All methods return JSON strings from the Rust layer.
     */
    private String dispatchAction(String action, JSONArray args) throws JSONException {
        switch (action) {
            // --- Checksum ---
            case "hashFile": {
                String path = args.getString(0);
                return nativeHashFile(path);
            }
            case "hashBytes": {
                byte[] data = args.getString(0).getBytes("UTF-8");
                return nativeHashBytes(data, data.length);
            }

            // --- Diff ---
            case "diff": {
                String oldText = args.getString(0);
                String newText = args.getString(1);
                int contextLines = args.optInt(2, 3);
                return nativeDiff(oldText, newText, contextLines);
            }

            // --- ZIP ---
            case "zipList": {
                byte[] data = decodeBase64Arg(args, 0);
                return nativeZipList(data, data.length);
            }
            case "zipExtract": {
                byte[] data = decodeBase64Arg(args, 0);
                String targetDir = args.getString(1);
                return nativeZipExtract(data, data.length, targetDir);
            }
            case "zipReadEntry": {
                byte[] data = decodeBase64Arg(args, 0);
                String entryName = args.getString(1);
                return nativeZipReadEntry(data, data.length, entryName);
            }
            case "sanitizeZipPath": {
                String path = args.getString(0);
                return nativeSanitizeZipPath(path);
            }

            // --- Search ---
            case "searchFiles": {
                String rootDir = args.getString(0);
                String search = args.getString(1);
                String optionsJson = args.optString(2, "{}");
                return nativeSearchFiles(rootDir, search, optionsJson);
            }

            // --- Filesystem (v0.3) ---
            case "fsLsDir": {
                String path = args.getString(0);
                return nativeFsLsDir(path);
            }
            case "fsReadFile": {
                String path = args.getString(0);
                String encoding = args.optString(1, null);
                return nativeFsReadFile(path, encoding);
            }
            case "fsWriteFile": {
                String path = args.getString(0);
                String dataBase64 = args.getString(1);
                return nativeFsWriteFile(path, dataBase64);
            }
            case "fsWalkTree": {
                String path = args.getString(0);
                return nativeFsWalkTree(path);
            }
            case "fsStat": {
                String path = args.getString(0);
                return nativeFsStat(path);
            }
            case "fsCreateDir": {
                String parent = args.getString(0);
                String name = args.getString(1);
                return nativeFsCreateDir(parent, name);
            }
            case "fsDelete": {
                String path = args.getString(0);
                return nativeFsDelete(path);
            }

            // --- Encoding (v0.3) ---
            case "detectEncoding": {
                String dataBase64 = args.getString(0);
                return nativeDetectEncoding(dataBase64);
            }
            case "decode": {
                String dataBase64 = args.getString(0);
                String encoding = args.getString(1);
                return nativeDecode(dataBase64, encoding);
            }
            case "encode": {
                String text = args.getString(0);
                String encoding = args.getString(1);
                return nativeEncode(text, encoding);
            }
            case "getEncodings": {
                return nativeGetEncodings();
            }

            // --- Archive (v0.3) ---
            case "archiveList": {
                String dataBase64 = args.getString(0);
                String format = args.getString(1);
                return nativeArchiveList(dataBase64, format);
            }
            case "archiveExtract": {
                String dataBase64 = args.getString(0);
                String format = args.getString(1);
                String targetDir = args.getString(2);
                return nativeArchiveExtract(dataBase64, format, targetDir);
            }
            case "archiveCompress": {
                String sourceDir = args.getString(0);
                String format = args.getString(1);
                return nativeArchiveCompress(sourceDir, format);
            }

            // --- Color / Markdown / Highlight / Sanitize (v0.4) ---
            case "parseColor": {
                String colorStr = args.getString(0);
                return nativeParseColor(colorStr);
            }
            case "highlightCode": {
                String code = args.getString(0);
                String language = args.getString(1);
                String theme = args.optString(2, "dark");
                return nativeHighlightCode(code, language, theme);
            }
            case "listHighlightLanguages": {
                return nativeListHighlightLanguages();
            }
            case "renderMarkdown": {
                String text = args.getString(0);
                String optionsJson = args.optString(1, "{}");
                return nativeRenderMarkdown(text, optionsJson);
            }
            case "sanitizeHtml": {
                String html = args.getString(0);
                String profile = args.optString(1, "standard");
                return nativeSanitizeHtml(html, profile);
            }

            default:
                return "{\"error\":\"Unknown action: " + escapeJson(action) + "\"}";
        }
    }

    // ---------------------------------------------------------------
    //  Helpers
    // ---------------------------------------------------------------

    /**
     * Decode a base64-encoded argument from a JSONArray.
     * Used for passing binary ZIP data across the Cordova bridge.
     */
    private static byte[] decodeBase64Arg(JSONArray args, int index) throws JSONException {
        String b64 = args.getString(index);
        return Base64.decode(b64, Base64.DEFAULT);
    }

    /** Minimal JSON string escaping for error messages. */
    private static String escapeJson(String s) {
        if (s == null) return "null";
        return s.replace("\\", "\\\\")
                .replace("\"", "\\\"")
                .replace("\n", "\\n")
                .replace("\r", "\\r")
                .replace("\t", "\\t");
    }
}
