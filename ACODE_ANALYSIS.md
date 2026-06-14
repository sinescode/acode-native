# Acode Project Analysis Report

> **Generated**: Sat Jun 13 2026  
> **Location**: `/home/kali/gitaction/acodeper/Acode/`  
> **Repository**: https://github.com/Acode-Foundation/Acode

---

## 1. Project Overview

**Acode** is a full-featured code editor for Android built with Apache Cordova. It's a hybrid mobile application using JavaScript/TypeScript for the UI layer and Java/Kotlin for native Android plugins. The app supports 100+ programming languages via CodeMirror 6, includes LSP (Language Server Protocol) support, a built-in terminal, file browser, plugin system, and theme engine.

| Property | Value |
|----------|-------|
| **Package ID** | `com.foxdebug.acode` |
| **Display Name** | Acode |
| **Version** | 1.12.3 (npm) / 1.12.5 (Cordova config.xml) |
| **Author** | Foxdebug (Ajit Kumar) |
| **License** | MIT |
| **Platform** | Android (Cordova) |
| **Min SDK** | 26 (Android 8.0) |
| **Target SDK** | 36 |

---

## 2. Directory Structure

```
Acode/
├── src/                          # Core application source
│   ├── main.js                   # App entry point (917 lines)
│   ├── main.scss                 # Main stylesheet
│   ├── index.d.ts                # TypeScript type declarations
│   ├── cordova-custom.d.ts       # Cordova type declarations
│   ├── cm/                       # CodeMirror 6 integration (17 files)
│   │   ├── baseExtensions.ts     # Core editor extensions
│   │   ├── colorView.ts          # Color preview
│   │   ├── commandRegistry.js    # Command registry
│   │   ├── editorUtils.ts        # Editor utilities
│   │   ├── indentGuides.ts       # Indent guides
│   │   ├── lineBreakMarker.ts    # Line break markers
│   │   ├── lineNumberSelection.ts # Line number selection
│   │   ├── localWordCompletions.ts # Local word completions
│   │   ├── mainEditorExtensions.ts # Main editor extensions bundle
│   │   ├── modelist.ts           # Language mode detection
│   │   ├── rainbowBrackets.ts    # Rainbow brackets
│   │   ├── supportedModes.ts     # Supported language modes
│   │   ├── tagAutoRename.ts      # HTML tag auto-rename
│   │   ├── touchSelectionMenu.js # Touch selection menu
│   │   ├── lsp/                  # Language Server Protocol client (22 files)
│   │   │   ├── api.ts            # LSP API
│   │   │   ├── clientManager.ts  # Client lifecycle management
│   │   │   ├── codeActions.ts    # Code actions (quick fixes)
│   │   │   ├── diagnostics.ts    # Diagnostics (errors/warnings)
│   │   │   ├── documentSymbols.ts # Document symbols
│   │   │   ├── formatter.ts      # Code formatter
│   │   │   ├── formattingSupport.ts # Formatting support
│   │   │   ├── index.ts          # LSP entry point
│   │   │   ├── inlayHints.ts     # Inlay hints
│   │   │   ├── installerUtils.ts # Server installer utilities
│   │   │   ├── installRuntime.ts # Runtime installer
│   │   │   ├── providerUtils.ts  # Provider utilities
│   │   │   ├── references.ts     # Find references
│   │   │   ├── rename.ts         # Rename support
│   │   │   ├── serverCatalog.ts  # Server catalog
│   │   │   ├── serverLauncher.ts # Server launcher
│   │   │   ├── serverRegistry.ts # Server registry
│   │   │   ├── tooltipExtensions.ts # Tooltips
│   │   │   ├── transport.ts      # LSP transport layer
│   │   │   ├── types.ts          # LSP types
│   │   │   ├── workspace.ts      # Workspace management
│   │   │   └── servers/          # Language-specific server configs
│   │   │       ├── javascript.ts
│   │   │       ├── luau.ts
│   │   │       ├── python.ts
│   │   │       ├── shared.ts
│   │   │       ├── systems.ts
│   │   │       └── web.ts
│   │   ├── modes/                # Custom language modes
│   │   │   └── luau/index.ts     # Luau mode
│   │   └── themes/               # Editor themes
│   ├── components/               # UI Components (27 items)
│   │   ├── audioPlayer/          # Audio player widget
│   │   ├── checkbox/             # Custom checkbox
│   │   ├── collapsableList.js    # Collapsible list
│   │   ├── contextmenu/          # Context menu
│   │   ├── fileTree/             # File tree view
│   │   ├── inputhints/           # Input hints
│   │   ├── logo/                 # App logo
│   │   ├── lspInfoDialog/        # LSP info dialog
│   │   ├── lspStatusBar/         # LSP status bar
│   │   ├── page.js               # Page component
│   │   ├── palette/              # Command palette
│   │   ├── quickTools/           # Quick tools bar
│   │   ├── referencesPanel/      # References panel
│   │   ├── scrollbar/            # Custom scrollbar
│   │   ├── searchbar/            # Search bar
│   │   ├── settingsPage.js       # Settings page
│   │   ├── sidebar/              # Sidebar
│   │   ├── sideButton/           # Side button
│   │   ├── symbolsPanel/         # Symbols panel
│   │   ├── tabView.js            # Tab view
│   │   ├── terminal/             # Terminal emulator (xterm.js)
│   │   ├── tile/                 # Tile component
│   │   ├── toast/                # Toast notifications
│   │   ├── tutorial.js           # Tutorial component
│   │   ├── virtualList/          # Virtual list (performance)
│   │   └── WebComponents/        # Web Components
│   ├── dialogs/                  # Dialog components (10 files)
│   │   ├── alert.js
│   │   ├── color.js
│   │   ├── confirm.js
│   │   ├── dialog.js             # Base dialog class
│   │   ├── loader.js
│   │   ├── multiPrompt.js
│   │   ├── prompt.js
│   │   ├── rateBox.js
│   │   ├── select.js
│   │   └── style.scss
│   ├── fileSystem/               # File system abstraction (5 files)
│   │   ├── index.js              # File system factory
│   │   ├── internalFs.js         # Internal storage
│   │   ├── externalFs.js         # External storage (SD card)
│   │   ├── ftp.js                # FTP protocol
│   │   └── sftp.js               # SFTP protocol
│   ├── handlers/                 # Event handlers (7 files)
│   │   ├── editorFileTab.js
│   │   ├── intent.js             # Android intent handler
│   │   ├── keyboard.js           # Keyboard shortcuts
│   │   ├── purchase.js           # In-app purchase
│   │   ├── quickTools.js
│   │   ├── quickToolsInit.js
│   │   └── windowResize.js
│   ├── lang/                     # Internationalization (50+ locales)
│   ├── lib/                      # Core libraries (46 files)
│   │   ├── acode.js              # Main Acode class
│   │   ├── actionStack.js        # Action stack (back navigation)
│   │   ├── adRewards.js          # Ad rewards
│   │   ├── ajax.js               # HTTP client
│   │   ├── applySettings.js      # Settings application
│   │   ├── auth.js               # Authentication
│   │   ├── checkFiles.js         # File change detection
│   │   ├── checkPluginsUpdate.js # Plugin update checker
│   │   ├── commands.js           # Command definitions
│   │   ├── config.js             # App configuration
│   │   ├── console.js            # JavaScript console
│   │   ├── customTab.ts          # Custom tab
│   │   ├── devTools.js           # Developer tools
│   │   ├── editorFile.js         # Editor file model
│   │   ├── editorManager.js      # Editor lifecycle manager
│   │   ├── fileList.js           # File list management
│   │   ├── fileTypeHandler.js    # File type detection
│   │   ├── fonts.js              # Font management
│   │   ├── installPlugin.js      # Plugin installer
│   │   ├── installState.js       # Installation state
│   │   ├── keyBindings.js        # Key bindings
│   │   ├── lang.js               # Language/i18n manager
│   │   ├── loadPlugin.js         # Plugin loader
│   │   ├── loadPlugins.js        # Batch plugin loader
│   │   ├── logger.js             # Logger
│   │   ├── notificationManager.js # Notification system
│   │   ├── openFile.js           # File opener
│   │   ├── openFolder.js         # Folder opener
│   │   ├── polyfill.js           # Polyfills
│   │   ├── prettierFormatter.js  # Prettier integration
│   │   ├── projects.js           # Project management
│   │   ├── recents.js            # Recent files
│   │   ├── remoteStorage.js      # Remote storage
│   │   ├── removeAds.js          # Ad removal
│   │   ├── restoreFiles.js       # File restoration
│   │   ├── restoreTheme.js       # Theme restoration
│   │   ├── run.js                # Code runner
│   │   ├── saveFile.js           # File saver
│   │   ├── saveState.js          # State persistence
│   │   ├── searchHistory.js      # Search history
│   │   ├── secureAdRewardState.js # Ad reward state
│   │   ├── selectionMenu.js      # Selection menu
│   │   ├── settings.js           # Settings manager
│   │   ├── showFileInfo.js       # File info display
│   │   ├── startAd.js            # Ad initialization
│   │   └── systemConfiguration.js # System config
│   ├── pages/                    # Full-page views (15 pages)
│   │   ├── about/
│   │   ├── adRewards/
│   │   ├── changelog/
│   │   ├── customTheme/
│   │   ├── fileBrowser/
│   │   ├── fontManager/
│   │   ├── markdownPreview/
│   │   ├── plugin/
│   │   ├── plugins/              # Plugin marketplace
│   │   ├── problems/
│   │   ├── quickTools/
│   │   ├── sponsor/
│   │   ├── sponsors/
│   │   ├── themeSetting/
│   │   └── welcome/
│   ├── palettes/                 # Command palettes
│   │   └── changeTheme/
│   ├── plugins/                  # Cordova native plugins (15 plugins)
│   │   ├── admob/                # AdMob (Google Ads) - TypeScript + Kotlin
│   │   ├── auth/                 # Authentication - Java
│   │   ├── browser/              # In-app browser - Java
│   │   ├── cordova-plugin-buildinfo/ # Build info
│   │   ├── custom-tabs/          # Chrome Custom Tabs - Java
│   │   ├── ftp/                  # FTP client - Java
│   │   ├── iap/                  # In-app purchases - Java
│   │   ├── pluginContext/        # Plugin context - Java
│   │   ├── proot/                # PRoot (Linux userland)
│   │   ├── sdcard/               # SD card access - Java
│   │   ├── server/               # Embedded HTTP server - Java
│   │   ├── sftp/                 # SFTP client - Java
│   │   ├── system/               # System utilities - Java
│   │   ├── terminal/             # Terminal emulator - Java
│   │   └── websocket/            # WebSocket client - Java
│   ├── res/                      # Static resources
│   ├── settings/                 # Settings UI (14 files)
│   ├── sidebarApps/              # Sidebar applications
│   │   ├── extensions/           # Extensions sidebar
│   │   ├── files/                # Files sidebar
│   │   ├── notification/         # Notifications sidebar
│   │   └── searchInFiles/        # Search in files
│   ├── styles/                   # Global styles
│   ├── theme/                    # Theme system
│   ├── utils/                    # Utility functions (10 files)
│   └── views/                    # Handlebars templates
├── utils/                        # Build utilities
│   ├── config.js
│   ├── setup.js
│   ├── loadStyles.js
│   ├── lang.js
│   ├── updateAce.js
│   ├── storage_manager.mjs
│   ├── custom-loaders/
│   │   └── html-tag-jsx-loader.js
│   └── scripts/
│       ├── build.sh
│       ├── clean.sh
│       ├── dev.js
│       ├── generate-release-notes.js
│       ├── plugin.sh
│       ├── setup.sh
│       └── start.sh
├── hooks/                        # Cordova lifecycle hooks
│   ├── move-files.js
│   ├── post-process.js
│   ├── modify-java-files.js
│   └── restore-cordova-resources.js
├── www/                          # Web output directory
│   └── index.html
├── .devcontainer/                # Dev container config
├── .github/                      # GitHub Actions & templates
│   ├── workflows/
│   │   ├── ci.yml
│   │   ├── nightly-build.yml
│   │   ├── on-demand-preview-releases-PR.yml
│   │   ├── community-release-notifier.yml
│   │   ├── close-inactive-issues.yml
│   │   └── add-pr-labels.yml
│   ├── dependabot.yml
│   └── ISSUE_TEMPLATE/
├── package.json
├── config.xml                    # Cordova configuration
├── webpack.config.js             # Webpack config
├── rspack.config.js              # Rspack config (alternative bundler)
├── tsconfig.json
├── biome.json                    # Biome linter/formatter config
├── postcss.config.js
├── jsconfig.json
├── _typos.toml                   # Typo checker config
├── .babelrc
├── .prettierrc
├── .hintrc
├── .dockerignore
├── .gitignore
├── .gitattributes
├── build-extras.gradle           # Android Gradle extras
├── license.txt
├── readme.md
├── CHANGELOG.md
├── CONTRIBUTING.md
└── CODE_OF_CONDUCT.md
```

---

## 3. Technology Stack

### Frontend
| Technology | Purpose |
|------------|---------|
| **CodeMirror 6** | Core editor engine (20+ language packages) |
| **xterm.js** | Terminal emulator (`@xterm/xterm` v5.5.0) |
| **SCSS** | Styling (50+ SCSS files) |
| **Handlebars** | Templating (`.hbs` files) |
| **html-tag-js** | JSX-like syntax for DOM creation |
| **Day.js** | Date/time manipulation |
| **markdown-it** | Markdown rendering (with plugins: anchor, emoji, footnote, task-lists, texmath) |
| **KaTeX** | Math rendering in markdown |
| **Mermaid** | Diagram rendering |
| **JSZip** | ZIP file handling |
| **DOMPurify** | HTML sanitization |
| **mustache** | Template rendering |
| **vanilla-picker** | Color picker |
| **picomatch** | Glob matching |
| **acorn** | JavaScript parser |
| **esprima** | JavaScript parser (alternative) |
| **Emmet** | HTML/CSS abbreviation expansion |

### Build System
| Technology | Purpose |
|------------|---------|
| **Rspack** | Primary bundler (Rust-based, faster) |
| **Webpack** | Alternative bundler |
| **Babel** | JavaScript transpilation (ES2015+) |
| **SWC** | TypeScript/JS transpilation (via Rspack) |
| **TypeScript** | Type checking (`tsc --noEmit`) |
| **Sass** | SCSS compilation |
| **PostCSS** | CSS post-processing |
| **Autoprefixer** | CSS vendor prefixes |
| **Biome** | Linting & formatting |

### Native (Android)
| Technology | Purpose |
|------------|---------|
| **Cordova** | Hybrid app framework (v13.0.0) |
| **cordova-android** | Android platform (v15.0.0) |
| **Java** | Native plugins (12 plugins) |
| **Kotlin** | AdMob plugin |
| **Gradle** | Android build system |

---

## 4. Entry Points & Boot Sequence

### Main Entry: `src/main.js` (917 lines)

The application boots through Cordova's `deviceready` event:

1. **Initialization** (lines 101-157):
   - Initialize character encodings
   - Detect free vs pro package
   - Set up file system paths (`DATA_STORAGE`, `CACHE_STORAGE`, `PLUGIN_DIR`)
   - Install global error handlers
   - Detect app install source (Play Store vs F-Droid)

2. **IAP & Auth** (lines 159-183):
   - Connect to in-app purchase service
   - Verify pro status via IAP purchases
   - Check `localStorage.acode_pro` flag

3. **System Info** (lines 185-208):
   - Get Android SDK version
   - Test CSS variable support (`DOES_SUPPORT_THEME`)

4. **Settings & Theme** (lines 239-263):
   - Initialize settings
   - Load theme system
   - Initialize syntax highlighting
   - Inject terminal font (MesloLGS NF)
   - Register Prettier formatter

5. **Language** (lines 265-275):
   - Load i18n language strings
   - Optionally load dev tools in developer mode

6. **App Load** (`loadApp()`, lines 481-815):
   - Create editor manager (`EditorManager`)
   - Create sidebar with file tree
   - Create main menu and file menu (context menus)
   - Set up header with navigation toggler
   - Initialize code modes
   - Initialize quick tools
   - Load sidebar apps
   - Restore previously open files
   - Restore terminal sessions
   - Load plugins (theme plugins first, then all plugins)
   - Open welcome tab if no files

7. **Post-Load** (lines 283-393):
   - Ensure sidebar has active app
   - Re-emit file events for plugins
   - Check login status
   - Fetch promotions
   - Start ads
   - Check for app updates (GitHub releases)
   - Check for plugin updates

### Secondary Entry Points
- `src/lib/console.js` - JavaScript console (separate bundle)
- `src/sidebarApps/searchInFiles/worker.js` - Web Worker for file search
- `src/boot.js` - Boot sequence (Rspack only)

---

## 5. Core Architecture Patterns

### 5.1 Editor Management
- **EditorManager** (`src/lib/editorManager.js`) - Singleton managing all editor instances
- **EditorFile** (`src/lib/editorFile.js`) - Represents a single open file with its editor state
- **CodeMirror 6** - Each file gets its own CM6 editor instance with shared extensions

### 5.2 Plugin System
- Plugins are Cordova plugins installed to `PLUGIN_DIR` (external data directory)
- Each plugin has its own `package.json` with metadata
- Plugin lifecycle: `installPlugin.js` → `loadPlugin.js` → `loadPlugins.js`
- Plugins can register:
  - Editor extensions
  - Theme definitions
  - File type handlers
  - Keyboard shortcuts
  - Sidebar apps
  - Settings panels

### 5.3 File System Abstraction
- **Unified API** via `src/fileSystem/index.js`
- **Internal FS** - App's sandboxed storage
- **External FS** - SD card / external storage
- **FTP/SFTP** - Remote file access via Cordova plugins
- All operations return promises for async consistency

### 5.4 Settings System
- **Settings Manager** (`src/lib/settings.js`) - Event-driven settings
- **Settings Files**: 14 specialized settings modules
- Persistence via `localStorage` and file system
- Settings emit events on change for reactive updates

### 5.5 Command System
- **Action Stack** (`src/lib/actionStack.js`) - Stack-based navigation
- **Command Registry** (`src/cm/commandRegistry.js`) - Keyboard shortcut binding
- **Commands** (`src/lib/commands.js`) - All available commands

### 5.6 LSP (Language Server Protocol)
- Full LSP client implementation in `src/cm/lsp/`
- **Client Manager** - Manages multiple language server connections
- **Server Registry** - Registry of available servers
- **Server Launcher** - Spawns language server processes
- **Server Catalog** - Pre-configured server configurations
- Supported languages: JavaScript/TypeScript, Python, Luau, and more
- Features: diagnostics, code actions, hover, references, rename, formatting, inlay hints, document symbols

---

## 6. Native Plugins

| Plugin | Language | Purpose |
|--------|----------|---------|
| **terminal** | Java | Terminal emulator with process management, Alpine Linux support |
| **system** | Java | System utilities (UI, input, permissions, rewards) |
| **sftp** | Java | SFTP file transfer |
| **ftp** | Java | FTP file transfer |
| **server** | Java | Embedded HTTP server (NanoHTTPD-based) |
| **sdcard** | Java | SD card access and file watching |
| **browser** | Java | In-app browser with emulator support |
| **auth** | Java | Authentication with encrypted preferences |
| **websocket** | Java | WebSocket client |
| **iap** | Java | In-app purchases |
| **custom-tabs** | Java | Chrome Custom Tabs |
| **pluginContext** | Java | Plugin context management |
| **cordova-plugin-buildinfo** | Java | Build information provider |
| **proot** | Java | PRoot (Linux userland emulation) |
| **admob** | Kotlin | Google AdMob integration (banner, interstitial, rewarded, native ads) |

---

## 7. Supported Languages (CodeMirror 6)

The editor supports 20+ languages via `@codemirror/lang-*` packages:

| Language | Package |
|----------|---------|
| Angular | `@codemirror/lang-angular` |
| C/C++ | `@codemirror/lang-cpp` |
| CSS | `@codemirror/lang-css` |
| Go | `@codemirror/lang-go` |
| HTML | `@codemirror/lang-html` |
| Java | `@codemirror/lang-java` |
| JavaScript/TypeScript | `@codemirror/lang-javascript` |
| Jinja | `@codemirror/lang-jinja` |
| JSON | `@codemirror/lang-json` |
| Less | `@codemirror/lang-less` |
| Liquid | `@codemirror/lang-liquid` |
| Markdown | `@codemirror/lang-markdown` |
| PHP | `@codemirror/lang-php` |
| Python | `@codemirror/lang-python` |
| Rust | `@codemirror/lang-rust` |
| Sass | `@codemirror/lang-sass` |
| SQL | `@codemirror/lang-sql` |
| Vue | `@codemirror/lang-vue` |
| WAT | `@codemirror/lang-wast` |
| XML | `@codemirror/lang-xml` |
| YAML | `@codemirror/lang-yaml` |
| Luau | Custom mode (`src/cm/modes/luau/`) |

Plus legacy modes via `@codemirror/legacy-modes`.

---

## 8. Internationalization

50+ language files in `src/lang/`:

```
ar-ye, be-by, bn-bd, cs-cz, de-de, en-us, es-sv, fr-fr,
he-il, hi-in, hu-hu, id-id, ir-fa, it-it, ja-jp, ko-kr,
ml-in, mm-unicode, mm-zawgyi, pl-pl, pt-br, pu-in, ru-ru,
tl-ph, tr-tr, uk-ua, uz-uz, vi-vn, zh-cn, zh-hant, zh-tw
```

Language management via `src/lib/lang.js` and CLI tool `utils/lang.js`:
```bash
pnpm run lang add      # Add new language
pnpm run lang remove   # Remove language
pnpm run lang search   # Search strings
pnpm run lang update   # Update translations
```

---

## 9. Build Configuration

### Rspack Config (`rspack.config.js`)
- **Entry points**: `boot.js`, `main.js`, `console.js`, `searchInFilesWorker.js`
- **Output**: `www/build/`
- **Loaders**: SWC for TS/JS, custom JSX loader, SCSS, assets
- **Dev mode**: Supports remote dev server with configurable host/port
- **Production**: Clean builds, code splitting

### Webpack Config (`webpack.config.js`)
- Same entry points (minus `boot.js`)
- Babel for transpilation
- `html-tag-js/jsx/tag-loader.js` for JSX
- Separate handling for CodeMirror files (no JSX loader)

### Biome Config (`biome.json`)
- Formatter: tab indentation
- Linter: selective rules (complexity, style, suspicious)
- Includes: `src/**/*.js`, `utils/**/*.js`, `src/lang/**/*.json`
- Excludes: `src/plugins/`, `www/`, `hooks/`, `platforms/`

---

## 10. CI/CD & DevOps

### GitHub Actions Workflows
| Workflow | Trigger |
|----------|---------|
| `ci.yml` | Push/PR to main |
| `nightly-build.yml` | Scheduled nightly |
| `on-demand-preview-releases-PR.yml` | PR preview releases |
| `community-release-notifier.yml` | Release notifications |
| `close-inactive-issues.yml` | Auto-close stale issues |
| `add-pr-labels.yml` | Auto-label PRs |

### Dev Container
- Docker-based development environment
- Pre-configured for VS Code

---

## 11. Key Libraries & Dependencies

### Core Dependencies (50+)
| Package | Version | Purpose |
|---------|---------|---------|
| `@codemirror/*` | ^6.x | Editor engine (20 packages) |
| `@xterm/xterm` | ^5.5.0 | Terminal emulator |
| `@xterm/addon-*` | ^0.x | Terminal addons (fit, search, image, webgl, etc.) |
| `html-tag-js` | ^2.4.16 | JSX-like DOM creation |
| `dayjs` | ^1.11.20 | Date/time |
| `markdown-it` | ^14.1.1 | Markdown rendering |
| `katex` | ^0.16.45 | Math rendering |
| `mermaid` | ^11.14.0 | Diagram rendering |
| `jszip` | ^3.10.1 | ZIP handling |
| `dompurify` | ^3.4.2 | HTML sanitization |
| `vanilla-picker` | ^2.12.3 | Color picker |
| `picomatch` | ^4.0.4 | Glob matching |
| `acorn` | ^8.16.0 | JS parser |
| `esprima` | ^4.0.1 | JS parser |
| `@emmetio/codemirror6-plugin` | ^0.4.0 | Emmet abbreviations |
| `url-parse` | ^1.5.10 | URL parsing |
| `mime-types` | ^3.0.2 | MIME type detection |
| `filesize` | ^11.0.17 | File size formatting |
| `escape-string-regexp` | ^5.0.0 | Regex escaping |
| `yargs` | ^18.0.0 | CLI argument parsing |

### Dev Dependencies (40+)
| Package | Version | Purpose |
|---------|---------|---------|
| `@rspack/core` | ^2.0.0 | Rspack bundler |
| `@rspack/cli` | ^2.0.0 | Rspack CLI |
| `@biomejs/biome` | 2.4.11 | Linter/formatter |
| `typescript` | ^5.9.3 | Type checking |
| `sass` | ^1.99.0 | SCSS compilation |
| `prettier` | ^3.8.3 | Code formatting |
| `@babel/*` | ^7.29.x | Transpilation (6 packages) |
| `chokidar` | ^4.0.3 | File watching |

---

## 12. Settings & Configuration

14 settings modules in `src/settings/`:

| Module | Purpose |
|--------|---------|
| `appSettings.js` | General app settings |
| `editorSettings.js` | Editor preferences (font, size, theme) |
| `terminalSettings.js` | Terminal configuration |
| `searchSettings.js` | Search behavior |
| `scrollSettings.js` | Scroll preferences |
| `previewSettings.js` | Preview settings |
| `lspSettings.js` | LSP configuration |
| `lspServerDetail.js` | LSP server details |
| `lspConfigUtils.js` | LSP config utilities |
| `helpSettings.js` | Help & about |
| `formatterSettings.js` | Code formatter settings |
| `filesSettings.js` | File handling settings |
| `backupRestore.js` | Backup & restore |
| `mainSettings.js` | Main settings page |

---

## 13. Key Features

1. **CodeMirror 6 Editor** - Modern, extensible editor with 100+ language support
2. **LSP Support** - Language Server Protocol for intelligent code completion, diagnostics, and refactoring
3. **Built-in Terminal** - xterm.js-based terminal with PRoot/Alpine Linux support
4. **Plugin System** - Extensible via community plugins
5. **Theme Engine** - Customizable editor themes with preview
6. **File Browser** - Native file system access (internal, external, FTP, SFTP)
7. **Search in Files** - Web Worker-powered search across project files
8. **Markdown Preview** - Live markdown preview with KaTeX, Mermaid, emojis
9. **In-App Browser** - Preview web pages directly in the editor
10. **Emmet Support** - HTML/CSS abbreviation expansion
11. **Rainbow Brackets** - Visual bracket matching
12. **Tag Auto-Rename** - Auto-rename paired HTML tags
13. **Touch Selection** - Mobile-optimized text selection
14. **Quick Tools** - Bottom toolbar for common actions
15. **Code Formatting** - Prettier integration
16. **JavaScript Console** - Built-in JS console for debugging
17. **Backup & Restore** - Settings and file backup
18. **Internationalization** - 50+ languages
19. **AdMob Integration** - Banner, interstitial, rewarded, native ads
20. **In-App Purchases** - Pro version unlock

---

## 14. Code Statistics

| Category | Count |
|----------|-------|
| JavaScript files (src/) | ~120 |
| TypeScript files (src/) | ~35 |
| SCSS files | 50+ |
| Java files (plugins/) | ~30 |
| Kotlin files (plugins/) | ~12 |
| Language JSON files | 50+ |
| Handlebars templates | 6 |
| Shell scripts | 8 |
| Build config files | 10+ |
| GitHub workflows | 6 |

---

## 15. Architecture Diagram

```
┌─────────────────────────────────────────────────┐
│                   www/index.html                 │
├─────────────────────────────────────────────────┤
│                    main.js                       │
│  ┌──────────┐ ┌──────────┐ ┌────────────────┐  │
│  │ Editor   │ │ Terminal │ │ File Browser   │  │
│  │ Manager  │ │ Manager  │ │ (sidebar)      │  │
│  └────┬─────┘ └────┬─────┘ └───────┬────────┘  │
│       │            │               │            │
│  ┌────▼─────┐ ┌────▼─────┐ ┌──────▼────────┐  │
│  │ CodeMirror│ │ xterm.js │ │ fileSystem/   │  │
│  │    6     │ │          │ │  index.js     │  │
│  └────┬─────┘ └──────────┘ └──────┬────────┘  │
│       │                           │            │
│  ┌────▼─────┐              ┌──────▼────────┐  │
│  │ LSP Client│              │ Internal FS   │  │
│  │  (22 files)│             │ External FS   │  │
│  └──────────┘              │ FTP / SFTP    │  │
│                            └───────────────┘  │
│  ┌──────────┐ ┌──────────┐ ┌────────────────┐  │
│  │ Plugin   │ │ Settings │ │ Theme System   │  │
│  │ System   │ │ System   │ │                │  │
│  └────┬─────┘ └──────────┘ └────────────────┘  │
│       │                                        │
├───────▼────────────────────────────────────────┤
│              Cordova Plugin Bridge              │
├────────────────────────────────────────────────┤
│  ┌──────┐ ┌──────┐ ┌──────┐ ┌──────┐ ┌─────┐ │
│  │termi-│ │sftp/ │ │sdcard│ │auth  │ │iAP  │ │
│  │nal   │ │ftp   │ │      │ │      │ │     │ │
│  └──────┘ └──────┘ └──────┘ └──────┘ └─────┘ │
│  ┌──────┐ ┌──────┐ ┌──────┐ ┌──────┐ ┌─────┐ │
│  │server│ │brows-│ │system│ │admob │ │ws   │ │
│  │      │ │er    │ │      │ │      │ │     │ │
│  └──────┘ └──────┘ └──────┘ └──────┘ └─────┘ │
├────────────────────────────────────────────────┤
│              Android (Java/Kotlin)              │
└────────────────────────────────────────────────┘
```

---

## 16. Notable Design Decisions

1. **Dual Bundler Support** - Both Webpack and Rspack configs maintained for flexibility
2. **Custom JSX via html-tag-js** - Instead of React, uses lightweight JSX for DOM creation
3. **Web Worker for Search** - File search runs in a separate thread for UI responsiveness
4. **Event-Driven Settings** - Settings emit events for reactive UI updates
5. **Plugin Isolation** - Plugins run in their own context with controlled API access
6. **LSP Auto-Installation** - Language servers can be automatically downloaded and installed
7. **State Persistence** - Editor state, open files, and terminal sessions persist across restarts
8. **Ace Editor Compatibility** - Maintains `window.ace` compat API for plugin backward compatibility
9. **Modular LSP** - LSP client is split into 22 focused modules for maintainability
10. **Theme System** - Separate theme plugins loaded before main plugins for immediate visual feedback

---

*This report was generated by analyzing the full Acode directory structure, source files, configuration, and dependencies.*
