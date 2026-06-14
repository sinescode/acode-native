# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Overview

This directory contains two related projects for studying and extending the **Acode** Android code editor:

1. **`Acode/`** — Fork/clone of the [Acode editor](https://github.com/Acode-Foundation/Acode) (upstream by Foxdebug/Ajit Kumar). A full-featured, MIT-licensed code editor for Android built with Apache Cordova + CodeMirror 6.
2. **`acode-rs/`** — A Rust native module companion crate (`acode_native`) that provides high-performance filesystem, search, diff, zip, archive, encoding, checksum, markdown, syntax highlighting, color, and sanitize functionality via FFI, replacing JS bottlenecks in Acode.

Analysis docs (`ACODE_ANALYSIS.md`, `ACODE_PLUGIN_SYSTEM.md`) contain deep-dives generated from the codebase.

---

## Acode (JavaScript/TypeScript — Cordova Android App)

| Property | Value |
|----------|-------|
| Package ID | `com.foxdebug.acode` |
| Version | 1.12.3 (npm) / 1.12.5 (config.xml) |
| Platform | Android (Cordova 13, cordova-android 15) |
| Min SDK | 26 (Android 8.0) |
| Target SDK | 36 |
| License | MIT |
| Bundler | Rspack (primary) + Webpack (legacy fallback) |
| Linter | Biome 2.4 |
| TypeScript | 5.9 (type-check only, no emit) |
| Editor engine | CodeMirror 6 (migrated from Ace) |

### Source Structure

```
Acode/src/
├── main.js                 # App entry point (917 lines) — onDeviceReady, plugin loading
├── boot.js                 # Boot/init — dev vs prod routing, WebSocket reload
├── cm/                     # CodeMirror 6 integration (17 files)
│   ├── lsp/                # LSP client (22 files + servers/)
│   ├── modes/              # Custom language modes (luau)
│   └── themes/             # Editor themes
├── components/             # UI components (27 items)
│   ├── terminal/           # xterm.js terminal
│   ├── sidebar/            # Sidebar panel
│   ├── palette/            # Command palette
│   ├── fileTree/           # File tree view
│   └── ...
├── dialogs/                # Modal dialogs (alert, confirm, prompt, select, color)
├── fileSystem/             # FS abstraction (internal, external, FTP, SFTP)
├── handlers/               # Event handlers (keyboard, intent, purchase, resize)
├── lib/                    # Core logic (46 files)
│   ├── acode.js            # Global API surface — Acode class (~1000 lines)
│   ├── editorManager.js    # Editor lifecycle
│   ├── editorFile.js       # File model
│   ├── loadPlugin.js       # Plugin loader
│   ├── settings.js         # Settings system
│   └── ...
├── pages/                  # Full-page views (plugin store, file browser, themes, about)
├── palettes/               # Palette UIs (command, mode, theme, encoding, find-file)
├── plugins/                # 15 bundled Cordova native plugins (Java/Kotlin)
├── settings/               # Settings panels (14 files: editor, LSP, terminal, etc.)
├── sidebarApps/            # Sidebar apps (files, extensions, search-in-files, notifications)
├── styles/                 # SCSS stylesheets (53 total)
├── utils/                  # Helpers (Path, Uri, Url, color, encodings, keyboardEvent)
├── views/                  # Handlebars templates (console, file-info, menu, markdown, rating)
├── res/                    # Static resources (fonts, icons, file-icons)
└── theme/                  # Theme engine
```

**File counts**: ~288 JS/TS, ~53 SCSS, ~30 Java, ~12 Kotlin, 511 total source files.

### UI Framework: html-tag-js (NOT React)

The entire UI uses **`html-tag-js`** (`tag()` calls), not React. A custom Rspack loader (`utils/custom-loaders/html-tag-jsx-loader.js`) transforms JSX syntax in `.js`/`.tsx` files at build time using Babel's parser — JSX elements become `tag('div', ...)` calls:

```js
// Source:
const $el = <div className="foo" id="bar" onClick={handler}>Hello</div>;
// Compiled to:
import tag from 'html-tag-js';
const $el = tag('div', 'foo', 'bar', ['Hello'], { on: { click: handler } });
```

**Key rules:**
- Lowercase JSX tags → `tag('tagname', ...)` (DOM elements)
- Uppercase/namespaced JSX tags → `tag(Component, ...)` (component calls)
- `class`/`className` → second argument to `tag()`
- `id` → third argument to `tag()`
- Event handlers (`onClick`, `onChange`) → `on` object in options
- `attr-*` prefix → `attr` object in options
- Spread attributes on lowercase tags go to options; on uppercase tags go to attrs

The loader only processes files that contain JSX-like syntax. Files without JSX pass through unchanged.

### Build Pipeline

```
Source (.js/.ts/.tsx)
  → html-tag-jsx-loader (Babel parse → JSX→tag() transform → Babel generate)
  → builtin:swc-loader (ES2015 target transpilation)
  → Rspack bundle output → www/build/
```

**Entry points** (4 separate bundles):
- `boot.js` — tiny loader (no JSX), loaded by `index.html`
- `main.js` — the entire app
- `console.js` — Eruda debug console (lazy loaded)
- `searchInFilesWorker.js` — Web Worker for file search

**Module resolution**: `src/` is in the resolve path, so imports are absolute from `src/`:
```js
import Acode from "lib/acode";        // → src/lib/acode.js
import toast from "components/toast"; // → src/components/toast.js
```

### App Initialization Flow (in order)

1. `boot.js` — detects `DEV_MODE` (compile-time define); in dev → loads bundles from dev server over HTTP + WebSocket reload; in prod → loads local `www/build/` bundles
2. `main.js` — `deviceready` event fires `onDeviceReady()`:
   - Init encodings
   - Set global paths (`DATA_STORAGE`, `CACHE_STORAGE`, `PLUGIN_DIR`, etc.)
   - Detect pro/free, Android SDK version, theme support
   - Create `window.acode = new Acode()` (the global plugin API)
   - Init ad rewards, request permissions
   - Ensure plugin directory exists
   - Load settings → load themes → init syntax highlighting
   - Inject terminal font
   - `loadApp()` — restore files, open folder, init editor
   - `loadPlugins()` — load user-installed plugins
   - `applySettings.afterRender()` — post-render settings
   - Check auth, fetch promotions, start ads

### Global Plugin API (`window.acode` / `lib/acode.js`)

The `Acode` class (~1000 lines) is **the** extension API. Every user-installed plugin interacts with it:

- **`acode.define(name, module)`** — register a module (case-insensitive)
- **`acode.require(name)`** — get a registered module
- **`acode.exec(key, val)`** — execute a built-in command
- **`acode.setPluginInit(id, initFn, settings)`** — register plugin init + optional settings UI
- **`acode.setPluginUnmount(id, unmountFn)`** — register cleanup
- **`acode.initPlugin(id, baseUrl, $page, options)`** — called by plugin loader
- **`acode.unmountPlugin(id)`** — called on plugin removal

**Built-in modules** exposed to plugins (via `acode.define`): `config`, `Url`, `Color`, `fonts`, `toast`, `alert`, `select`, `loader`, `dialogBox`, `prompt`, `confirm`, `helpers`, `palette`, `tutorial`, `aceModes` (language registration), `themes`, `editorLanguages`, `editorThemes`, `lsp`, `settings`, `terminal`, `codemirror` (all CM packages), `@codemirror/*` (individual packages), `fs` (filesystem), `sidebarApps`, `encodings`, `keyboard`, `commands`, `contextMenu`, `fileBrowser`, `selectionMenu`, `createKeyboardEvent`, and more.

### Settings System (`lib/settings.js`)

Event-driven settings with named update channels:
- `settings.on('update', callback)` — fires on any setting change
- `settings.on('update:after', callback)` — fires after UI update
- `settings.on('update:<key>', callback)` — fires when specific key changes (e.g. `'update:editorTheme'`)
- `settings.on('reset', callback)` — fires on settings reset
- `settings.value` — current settings object
- `settings.uiSettings` — registry of settings UI pages (plugins add via `acode.setPluginInit`)
- `settings.update(render)` — persist + notify

### Translation System (`lib/lang.js`)

Languages loaded dynamically from `src/lang/<locale>.json` files. The `strings` global holds all translated strings. CI checks translation consistency on PRs touching `src/lang/`:
```bash
npm run lang check
```

### Key Dependencies

- **Editor**: `codemirror` 6 + 20+ `@codemirror/*` language/extension packages
- **Terminal**: `@xterm/xterm` 5.5 with WebGL, search, fit, image addons
- **Markdown**: `markdown-it` with plugins (emoji, footnote, task-lists, KaTeX math, mermaid)
- **HTML/JSX**: `html-tag-js` 2.4 (custom JSX-like DOM library)
- **UI**: `vanilla-picker` (color), `autosize`, `dompurify`
- **Build**: `@rspack/core` 2 + Babel 7.29 (SWC for TS, Babel for JS with polyfills)

### Build & Dev

```bash
cd Acode
npm run setup      # Initial setup (npm install + cordova prepare)
npm run build      # Production build (Rspack + Cordova APK)
npm run dev        # Dev mode: HTTP server + WebSocket reload + rspack --watch + cordova run
npm run dev:android # Dev mode, Android target
npm run lint       # Biome lint --write
npm run format     # Biome format --write
npm run check      # Biome check --write (lint + format)
npm run typecheck  # tsc --noEmit
npm run lang       # Translation check
```

**Dev mode architecture** (`utils/scripts/dev.js`):
1. Self-signed HTTPS server (or HTTP fallback) serving `www/` + WebSocket for reload signaling
2. Rspack in `--watch` mode with `DEV_MODE=true` — recompiles on change, broadcasts "reload" via WebSocket
3. `cordova run android` — launches APK on device; the APK's embedded `boot.js` fetches fresh bundles from the dev server
4. File watcher on `src/plugins/` — auto-reinstalls changed Cordova plugins and rebuilds

### Biome Configuration (`biome.json`)

- Indent: tabs
- Organize imports on save
- Lint rules: `noForEach: off`, `noDoubleEquals: error`, `useForOf: error`, `useIsArray: error`, `noStaticOnlyClass: error`, `useNodejsImportProtocol: error`
- Files: includes `src/**/*.js`, `utils/**/*.js`, `src/lang/**/*.json`; excludes `src/plugins/**/*.js`, `www/**`, `platforms/**`

### Runtime Config (`lib/config.js`)

Static constants for the app: `BASE_URL` (acode.app), `API_BASE`, server ports (8158, 8159), scroll speed presets, file name regex, default file name, etc. Accessed by plugins via `acode.require('config')`.

### No Test Infrastructure

There are no test files, no test runner configuration, and no test scripts in `package.json`. The project relies on manual testing via `npm run dev` (device/emulator) and CI linting/type-checking.

---

## acode-rs (Rust Native Module — v0.4)

| Property | Value |
|----------|-------|
| Crate name | `acode_native` |
| Version | 0.4.0 |
| Edition | 2021 |
| Crate types | `cdylib`, `staticlib` (FFI) |
| Status | Feature-complete — targets JS-side bottlenecks across 11 modules |

### Design Decision

v0.1 targeted Java/Cordova plugins (server, SFTP, FTP, terminal, websocket) — these already work fine and the Cordova `JS → Java` bridge is the real bottleneck. Adding Rust between JS and Java adds JNI overhead with no user-perceptible gain.

v0.2+ targets the **actual Acode bottlenecks** — operations running in pure JS on the main thread: search, zip, diff, checksum.

v0.4 expands to cover additional JS-side operations: filesystem bulk ops, archive extraction, encoding detection, syntax highlighting, markdown rendering, color manipulation, and HTML sanitization.

### Modules (12 source files)

| Module | Purpose | Key deps |
|--------|---------|----------|
| `fs` | Bulk filesystem ops (walk, mime detect) | walkdir, mime_guess |
| `search` | Parallel regex file search (ripgrep-style) | regex, ignore, memchr, rayon |
| `diff` | Fast text diff (Myers algorithm, unified output) | similar |
| `zip` | ZIP extract/compress + zip-slip protection | zip (deflate, bzip2, zstd) |
| `archive` | tar, tar.gz, tar.bz2, tar.xz, tar.zst | tar, flate2, bzip2, xz2, zstd |
| `encoding` | Charset detection (replaces JS BOM+heuristic) | chardetng |
| `checksum` | SHA-256 file hashing (SIMD) | sha2, hex |
| `markdown` | Markdown→HTML (replaces markdown-it) | comrak |
| `highlight` | Syntax highlighting (replaces Lezer) | syntect |
| `color` | Color manipulation utilities | — |
| `sanitize` | HTML sanitization (replaces DOMPurify) | ammonia |
| `lib` | JNI bridge + JSON/FFI exports + module wiring | libc, base64, serde_json, tokio |

### FFI Exports (C-compatible, callable from Java/Cordova via JNI)

All return values are JSON strings (owned by Rust, freed by `acode_free_string`).

### Features (all default-on)

`fs`, `search`, `diff`, `zip`, `archive`, `encoding`, `checksum`, `markdown`, `highlight`, `color`, `sanitize`

### Feature mapping: Rust → replaces JS

| Rust module | Replaces JS code |
|-------------|-----------------|
| `fs::walk_dir` | `fileSystem` + `fileList` directory listing |
| `search::search_dir` | `sidebarApps/searchInFiles/worker.js:searchInFile()` + `processFiles()` |
| `search::replace_in_content` | `worker.js:replaceInFile()` |
| `zip::extract_zip` | `lib/installPlugin.js:installPlugin()` ZIP extraction loop |
| `zip::sanitize_zip_path` | `installPlugin.js:sanitizeZipPath()` + `isUnsafeAbsolutePath()` |
| `archive::extract_archive` | JSZip tar/compressed format handling |
| `encoding::detect` | `utils/encodings.js` BOM+heuristic detection |
| `diff::text_diff` | No JS equivalent — new capability |
| `checksum::hash_file` | No JS equivalent — new capability |
| `markdown::render` | `markdown-it` + plugin chain |
| `highlight::highlight` | `utils/codeHighlight.js` Lezer-based highlighting |
| `sanitize::clean` | `dompurify` HTML sanitization |

### Profile (release)

```toml
lto = true           # Link-time optimization
codegen-units = 1    # Maximize inlining
opt-level = 3        # Aggressive optimization
strip = true         # Strip symbols → smaller .so
```

### Important

**Do NOT run `cargo build/check/test/clippy` locally** — all Rust compilation happens in CI only. See global CLAUDE.md disk space rules.

---

## Cordova Plugin: acode-native (`Acode/src/plugins/acode-native/`)

Bridges the Rust `.so` into Acode's Cordova build. Follows the exact `proot` plugin pattern for native library bundling.

### Files

```
src/plugins/acode-native/
├── package.json              # npm metadata
├── plugin.xml                # Feature reg + .so source-file declarations (3 ABIs)
├── www/
│   └── acode-native.js       # JS bridge: Promise-based methods (cordova.exec)
├── src/android/
│   └── AcodeNativePlugin.java # Extends CordovaPlugin, JNI native methods, thread-pool dispatch
├── libs/
│   ├── .gitignore            # *.so — never commit native libs
│   ├── arm32/.gitkeep        # armeabi-v7a .so placed here by CI
│   ├── arm64/.gitkeep        # arm64-v8a
│   └── x64/.gitkeep          # x86_64
```

### Architecture

```
JS:  acode.native.hashFile(path) → Promise<{hex, input_size}>
         ↓ cordova.exec(success, error, 'AcodeNativePlugin', 'hashFile', [path])
Java:   AcodeNativePlugin.execute("hashFile", ...) → cordova.getThreadPool()
         ↓ nativeHashFile(path) [JNI]
Rust:   acode_hash_file(path) → JSON string via CString::into_raw()
```

### JS API (`acode.native.*`)

| Method | Args | Returns |
|--------|------|---------|
| `isAvailable()` | — | `Promise<boolean>` |
| `hashFile(path)` | string | `Promise<{hex, input_size}>` |
| `hashBytes(data)` | string | `Promise<{hex, input_size}>` |
| `diff(old, new, context?)` | string×2, number | `Promise<DiffResult>` |
| `zipList(zipData)` | ArrayBuffer | `Promise<ZipEntry[]>` |
| `zipExtract(zipData, targetDir)` | ArrayBuffer, string | `Promise<ExtractResult>` |
| `zipReadEntry(zipData, entryName)` | ArrayBuffer, string | `Promise<{data_base64, size}>` |
| `sanitizeZipPath(path)` | string | `Promise<{safe\|unsafe}>` |
| `searchFiles(rootDir, query, opts?)` | string×2, object | `Promise<FileSearchResult[]>` |

### Fallback

`acode.native.isAvailable()` probes whether the `.so` loaded. If false (wrong ABI, missing lib), the existing JS implementations (JSZip, web workers) remain in place as fallback.

---

## CI/CD

| Workflow | Trigger | What it does |
|----------|---------|-------------|
| `ci.yml` | push, PR | Spell check (typos), lint/format (Biome), translation check (PRs touching `src/lang/`) |
| `build-rust-android.yml` | `workflow_dispatch`, push to `acode-rs/**` | Cross-compiles Rust for 3 ABIs via cargo-ndk, uploads .so artifacts |
| `build-acode-with-rust.yml` | `workflow_dispatch`, push to `acode-rs/**` or plugin | Full pipeline: Rust build → download .so → Cordova APK build |
| `nightly-build.yml` | schedule (7am UTC), push tag `nightly` | Upstream nightly APK builds |
| Other | — | PR preview releases, community release notifier, inactive issue closer |

### Rust cross-compilation

```bash
# In CI (GitHub Actions, ubuntu-latest):
cargo install cargo-ndk
cargo ndk -t aarch64-linux-android -p 26 -o ./libs/arm64 build --release
# Produces: libs/arm64/libacode_native.so
```

ABI targets: `aarch64-linux-android` → `arm64-v8a`, `armv7-linux-androideabi` → `armeabi-v7a`, `x86_64-linux-android` → `x86_64`

Platform 26 matches Acode's `minSdkVersion`.

---

## Working Notes

- The Acode fork is read-only study material — upstream is `deadlyjack/acode` (now `Acode-Foundation/Acode`)
- The two existing analysis docs (`ACODE_ANALYSIS.md`, `ACODE_PLUGIN_SYSTEM.md`) are reference material, not living docs
- `acode-rs` v0.4 covers 11 modules targeting JS-side bottlenecks — the modules that Java Cordova plugins already handle (server, SFTP, FTP, terminal, websocket) are intentionally excluded
