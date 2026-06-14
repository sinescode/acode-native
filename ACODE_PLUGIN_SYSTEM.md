# Acode Plugin System — Deep Dive Analysis

> **Generated**: Sat Jun 13 2026  
> **Scope**: Plugin loading, installation, lifecycle, API surface, and management

---

## 1. Plugin Architecture Overview

Acode's plugin system has two distinct layers:

1. **Cordova Native Plugins** — Java/Kotlin plugins bundled at build time (15 plugins in `src/plugins/`)
2. **User-Installed Plugins** — JavaScript/TypeScript plugins downloaded from the Acode registry at runtime, stored in the device's external data directory (`PLUGIN_DIR`)

This analysis covers **both layers**, with emphasis on the user-installed plugin system.

```
┌──────────────────────────────────────────────────────────┐
│                     Plugin Lifecycle                      │
│                                                          │
│  Registry ──► Download ──► Unzip ──► Write Files ──►    │
│  (API)        (ZIP)        (JSZip)   (fsOperation)      │
│                                                          │
│  ──► Load Script ──► initPlugin() ──► Running Plugin     │
│      (inject <script>)  (call init)    (active)          │
│                                                          │
│  On Update: unmountPlugin() ──► loadPlugin() ──► ...    │
│  On Error: markPluginBroken() ──► auto-disable          │
│  On Timeout: 15s warning ──► 60s auto-disable           │
└──────────────────────────────────────────────────────────┘
```

---

## 2. Plugin Directory Structure

### User-Installed Plugins
Stored at `PLUGIN_DIR` (external data directory + `/plugins/`):

```
PLUGIN_DIR/
├── <plugin-id>/
│   ├── plugin.json        # Plugin manifest (required)
│   ├── main.js            # Entry point (or specified in plugin.json)
│   ├── icon.png           # Plugin icon
│   ├── readme.md          # Documentation
│   └── ...                # Additional files
```

### Cordova Native Plugins (15 bundled)
Located in `src/plugins/`:

| Plugin | Package Name | Native Code | Purpose |
|--------|-------------|-------------|---------|
| terminal | `com.foxdebug.acode.rk.exec.terminal` | Java | Terminal emulator |
| system | `cordova-plugin-system` | Java | System utilities |
| sftp | `cordova-plugin-sftp` | Java | SFTP file transfer |
| ftp | `cordova-plugin-ftp` | Java | FTP file transfer |
| server | `cordova-plugin-server` | Java | Embedded HTTP server |
| sdcard | `cordova-plugin-sdcard` | Java | SD card access |
| browser | `cordova-plugin-browser` | Java | In-app browser |
| auth | `com.foxdebug.acode.rk.auth` | Java | Authentication |
| websocket | `cordova-plugin-websocket` | Java | WebSocket client |
| iap | `cordova-plugin-iap` | Java | In-app purchases |
| custom-tabs | `com.foxdebug.acode.rk.customtabs` | Java | Chrome Custom Tabs |
| pluginContext | `com.foxdebug.acode.rk.plugin.plugincontext` | Java | Plugin context |
| buildinfo | `cordova-plugin-buildinfo` | Java | Build information |
| proot | `com.foxdebug.acode.rk.exec.proot` | Java | PRoot Linux emulation |
| admob | `admob-plus-cordova` | Kotlin | Google AdMob ads |

---

## 3. Plugin Manifest (`plugin.json`)

Every user-installed plugin must have a `plugin.json`:

```json
{
  "id": "com.example.my-plugin",
  "name": "My Plugin",
  "version": "1.0.0",
  "description": "Description here",
  "main": "main.js",
  "icon": "icon.png",
  "readme": "readme.md",
  "dependencies": ["other-plugin-id"],
  "price": 0,
  "sku": "product-sku"
}
```

| Field | Required | Description |
|-------|----------|-------------|
| `id` | Yes | Unique plugin identifier |
| `name` | Yes | Display name |
| `version` | Yes | Semver version |
| `main` | No | Entry point (defaults to `main.js`) |
| `icon` | No | Icon filename (defaults to `icon.png`) |
| `readme` | No | Readme filename (defaults to `readme.md`) |
| `dependencies` | No | Array of required plugin IDs |
| `price` | No | Price in microcurrency (0 = free) |
| `sku` | No | In-app purchase SKU for paid plugins |

---

## 4. Plugin Installation Flow

### 4.1 Entry Point: `installPlugin()` (`src/lib/installPlugin.js`)

```
User clicks "Install" in plugin marketplace
    │
    ▼
installPlugin(id, name, purchaseToken, isDependency)
    │
    ├── 1. Ensure PLUGIN_DIR exists
    │
    ├── 2. Build download URL
    │   ├── From registry: {API_BASE}/plugin/download/{id}?device={uuid}&package={pkg}&version={ver}
    │   └── From URL: Use provided URL directly
    │
    ├── 3. Download plugin ZIP
    │   ├── fsOperation for registry/file URLs
    │   └── cordova.plugin.http for other URLs
    │
    ├── 4. Validate ZIP contents
    │   ├── Must contain plugin.json
    │   ├── Must contain main entry file
    │   └── Patch missing icon/readme defaults
    │
    ├── 5. Resolve dependencies
    │   ├── Fetch each dependency manifest from registry
    │   ├── Check if already installed at correct version
    │   ├── Recursively resolve nested dependencies
    │   ├── Prompt user for confirmation
    │   └── Install each dependency
    │
    ├── 6. Create InstallState (SHA-256 checksums)
    │
    ├── 7. Extract files to plugin directory
    │   ├── Sanitize paths (prevent path traversal)
    │   ├── Reject unsafe absolute paths (/, C:/, /data, etc.)
    │   ├── Create directories recursively
    │   ├── Write files with progress callback
    │   └── Skip unchanged files (checksum comparison)
    │
    ├── 8. Load the plugin
    │   └── loadPluginWithTimeout(id, justInstalled=true)
    │
    ├── 9. Save install state
    │
    └── 10. Delete redundant files (files removed in new version)
```

### 4.2 Security: Path Sanitization

The installer performs aggressive path sanitization to prevent zip-slip attacks:

```javascript
// installPlugin.js:339-364
function sanitizeZipPath(p, isDir) {
    path = path.replace(/\\/g, "/");           // Normalize separators
    path = path.replace(/^[a-zA-Z]+:\/\//, ""); // Remove URL schemes
    path = path.replace(/^\/+/, "");            // Strip leading slashes
    path = path.replace(/^[A-Za-z]:\//, "");    // Strip Windows drive letters
    // Resolve . and .. segments
    // Result: always a relative path under pluginDir
}

// installPlugin.js:372-389
function isUnsafeAbsolutePath(p) {
    // Rejects: /data, /system, /vendor, /storage, /sdcard, /root
    // Rejects: C:\, //network paths
    // Rejects: ANY path starting with /
}
```

### 4.3 Dependency Resolution

Dependencies are resolved recursively from the Acode registry:

```javascript
// installPlugin.js:395-436
async function resolveDepsManifest(deps) {
    for (const dependency of deps) {
        const remoteDependency = await fetch(`${API_BASE}/plugin/${dependency}`);
        const installed = await getInstalledPluginVersion(remoteDependency.id);
        
        // Skip if same version already installed
        if (remoteDependency.version === installed) continue;
        
        // Recursively resolve nested dependencies
        if (remoteDependency.dependencies) {
            resolved.push(...await resolveDepsManifest(remoteDependency.dependencies));
        }
        resolved.push(remoteDependency);
    }
    return resolved;
}
```

Paid dependencies trigger IAP flow before installation.

### 4.4 InstallState: Change Detection

`InstallState` (`src/lib/installState.js`) tracks file checksums to avoid unnecessary writes:

```javascript
class InstallState {
    // SHA-256 checksums stored in DATA_STORAGE/.install-state/<checksum-of-id>
    async isUpdated(url, content) {
        const current = this.store[url];
        const update = await checksum(content); // SHA-256 via crypto.subtle
        this.updatedStore[url] = update;
        return current !== update; // false = file unchanged, skip write
    }
}
```

This means plugin updates only write files that actually changed.

---

## 5. Plugin Loading Flow

### 5.1 Batch Loading: `loadPlugins()` (`src/lib/loadPlugins.js`)

Called twice during app startup:
1. **Theme plugins first** — `loadPlugins(true)` loads only theme-related plugins
2. **All other plugins** — `loadPlugins(false)` loads everything else

```javascript
async function loadPlugins(loadOnlyTheme = false) {
    const plugins = await fsOperation(PLUGIN_DIR).lsDir();
    
    const pluginsToLoad = plugins.filter((pluginDir) => {
        const pluginId = Url.basename(pluginDir.url);
        return (
            isThemePlugin(pluginId) === loadOnlyTheme &&  // Theme filter
            !LOADED_PLUGINS.has(pluginId) &&              // Not already loaded
            enabledMap[pluginId] !== true &&              // Not disabled
            !BROKEN_PLUGINS.has(pluginId)                 // Not broken
        );
    });
    
    // Load all plugins concurrently with individual timeouts
    await Promise.allSettled(
        pluginsToLoad.map(p => loadPluginWithTimeout(Url.basename(p.url)))
    );
    
    // Signal completion
    acode[onPluginsLoadCompleteCallback]();
}
```

### 5.2 Theme Plugin Detection

Theme plugins are identified by keyword matching against the plugin ID:

```javascript
const THEME_IDENTIFIERS = new Set([
    "theme", "catppuccin", "pine", "githubdark", "radiant",
    "rdtheme", "ayumirage", "dust", "synthwave", "dragon",
    "mint", "monokai", "lumina_code", "sweet", "moonlight",
    "bluloco", "acode.plugin.extra_syntax_highlights", "documentsviewer"
]);

function isThemePlugin(pluginId) {
    const id = pluginId.toLowerCase();
    return Array.from(THEME_IDENTIFIERS).some(theme => id.includes(theme));
}
```

### 5.3 Individual Plugin Loading: `loadPluginWithTimeout()`

Each plugin has a **15-second timeout** to complete loading:

```javascript
async function loadPluginWithTimeout(pluginId, justInstalled = false) {
    const pluginState = { settled: false };
    
    const pluginLoadPromise = loadPlugin(pluginId, justInstalled)
        .catch(async (error) => {
            pluginState.settled = true;
            await markPluginBroken(pluginId, error);  // Auto-disable
            throw error;
        })
        .then(async () => {
            pluginState.settled = true;
            await markPluginLoaded(pluginId, justInstalled);
        });
    
    try {
        await Promise.race([
            pluginLoadPromise,
            new Promise((_, rej) => setTimeout(
                () => rej(new PluginLoadTimeoutError()),
                15000  // 15s timeout
            ))
        ]);
    } catch (error) {
        if (error instanceof PluginLoadTimeoutError) {
            // Don't immediately disable — wait up to 60s total
            markPluginTimedOut(pluginId, pluginState);
        }
    }
}
```

### 5.4 Script Injection: `loadPlugin()` (`src/lib/loadPlugin.js`)

The actual loading mechanism:

```javascript
async function loadPlugin(pluginId, justInstalled = false) {
    const baseUrl = await helpers.toInternalUri(Url.join(PLUGIN_DIR, pluginId));
    
    // 1. Unmount old version first (important for hot-reload)
    try {
        acode.unmountPlugin(pluginId);
    } catch (e) { /* Old plugin's destroy() may throw — ignore */ }
    
    // 2. Remove old script tag
    const oldScript = document.getElementById(`${pluginId}-mainScript`);
    if (oldScript) oldScript.remove();
    
    // 3. Read plugin manifest
    const pluginJson = await fsOperation(
        Url.join(PLUGIN_DIR, pluginId, "plugin.json")
    ).readFile("json");
    
    // 4. Determine entry point
    let mainUrl;
    if (await fsOperation(Url.join(PLUGIN_DIR, pluginId, pluginJson.main)).exists()) {
        mainUrl = Url.join(baseUrl, pluginJson.main);
    } else {
        mainUrl = Url.join(baseUrl, "main.js");
    }
    
    // 5. Inject <script> tag into document head
    return new Promise((resolve, reject) => {
        const $script = (
            <script id={`${pluginId}-mainScript`} src={mainUrl}></script>
        );
        
        $script.onerror = () => reject(new Error(`Failed to load script`));
        
        $script.onload = async () => {
            // 6. Create plugin page
            const $page = Page("Plugin");
            
            // 7. Initialize plugin via Acode API
            await acode.initPlugin(pluginId, baseUrl, $page, {
                cacheFileUrl: ...,
                cacheFile: fsOperation(cacheFile),
                firstInit: justInstalled,
                ctx: await PluginContext.generate(pluginId, pluginJson),
            });
            
            resolve();
        };
        
        document.head.append($script);
    });
}
```

**Key detail**: Plugins are loaded by injecting a `<script>` tag. The plugin's JavaScript runs in the global scope and interacts with Acode through the `window.acode` API.

---

## 6. Plugin Lifecycle

### 6.1 Registration Phase

When a plugin's script loads, it calls `acode.setPluginInit()` to register its initialization function:

```javascript
// Plugin's main.js
window.acode.setPluginInit('my-plugin', (baseUrl, $page, options) => {
    // options.cacheFileUrl - path to plugin's cache file
    // options.cacheFile - fsOperation for cache file
    // options.firstInit - true if just installed
    // options.ctx - PluginContext
    
    // Register commands, sidebar apps, editor extensions, etc.
    acode.addCommand({
        name: 'my-command',
        description: 'My custom command',
        exec: () => { /* ... */ }
    });
    
    sidebarApps.add('mySidebar', 'My Sidebar', 'icon_name', (app) => {
        // Return sidebar app content
        return <div>Hello from my plugin!</div>;
    });
}, {
    // Optional: plugin settings page
    list: [
        { text: 'Setting 1', key: 'setting1', value: 'default' }
    ],
    cb: (key, value) => { /* Handle setting change */ }
});
```

### 6.2 Initialization Phase

`acode.initPlugin()` calls the registered init function:

```javascript
// acode.js:689
async initPlugin(id, baseUrl, $page, options) {
    if (id in this.#pluginsInit) {
        await this.#pluginsInit[id](baseUrl, $page, options);
    }
}
```

### 6.3 Unmount Phase

When a plugin is removed or reloaded, `acode.unmountPlugin()` calls its cleanup function:

```javascript
// acode.js:695
unmountPlugin(id) {
    if (id in this.#pluginUnmount) {
        this.#pluginUnmount[id]();  // Call plugin's destroy() callback
        fsOperation(Url.join(CACHE_STORAGE, id)).delete();
    }
    delete appSettings.uiSettings[`plugin-${id}`];
}
```

Plugins register their cleanup via:

```javascript
window.acode.setPluginUnmount('my-plugin', () => {
    // Remove event listeners, sidebar apps, commands, etc.
});
```

### 6.4 Broken Plugin Handling

```javascript
async function markPluginBroken(pluginId, error) {
    BROKEN_PLUGINS.set(pluginId, {
        error: String(error.message || error),
        timestamp: Date.now(),
    });
    AUTO_DISABLED_PLUGINS.add(pluginId);
    await updatePluginDisabled(pluginId, true);  // Persist to settings
}

// Timed-out plugins get a second chance (45s grace period)
function markPluginTimedOut(pluginId, pluginState) {
    BROKEN_PLUGINS.set(pluginId, { error: "Plugin load timeout", ... });
    setTimeout(async () => {
        if (pluginState.settled || LOADED_PLUGINS.has(pluginId)) return;
        await markPluginBroken(pluginId, new Error("Plugin load timeout"));
    }, 45000);  // 60s total - 15s initial timeout
}
```

Broken plugins are auto-disabled in `settings.value.pluginsDisabled` and skipped on subsequent loads.

---

## 7. Plugin API Surface

Plugins interact with Acode through `window.acode` (instance of `Acode` class).

### 7.1 Module System (`acode.define()` / `acode.require()`)

The `Acode` class maintains a module registry:

```javascript
// acode.js:79
#modules = {};

define(name, module) {
    this.#modules[name.toLowerCase()] = module;
}

require(module) {
    return this.#modules[module.toLowerCase()];
}
```

**Registered modules** (available to plugins via `acode.require()`):

| Module Name | Description |
|------------|-------------|
| `config` | App configuration |
| `Url` | URL utilities |
| `page` | Page component |
| `Color` | Color utilities |
| `fonts` | Font management |
| `toast` | Toast notifications |
| `alert` | Alert dialog |
| `select` | Selection dialog |
| `loader` | Loading dialog |
| `dialogBox` | Base dialog |
| `prompt` | Prompt dialog |
| `confirm` | Confirmation dialog |
| `intent` | Intent handlers |
| `fileList` | File list management |
| `fs` | File system operations |
| `helpers` | Utility helpers |
| `palette` | Command palette |
| `projects` | Project management |
| `tutorial` | Tutorial component |
| `aceModes` | Language mode management (legacy) |
| `themes` | Theme management |
| `editorLanguages` | Language mode management (preferred) |
| `editorThemes` | Editor theme management |
| `lsp` | Language Server Protocol API |
| `settings` | App settings |
| `sideButton` | Side button component |
| `EditorFile` | Editor file class |
| `inputhints` | Input hints component |
| `openfolder` | Folder opener |
| `colorPicker` | Color picker dialog |
| `actionStack` | Action stack (back navigation) |
| `multiPrompt` | Multi-field prompt |
| `addedfolder` | Added folders tracking |
| `contextMenu` | Context menu |
| `fileBrowser` | File browser |
| `fsOperation` | File system operations |
| `keyboard` | Keyboard handler |
| `windowResize` | Window resize handler |
| `encodings` | Character encodings |
| `themeBuilder` | Theme builder |
| `selectionMenu` | Selection menu |
| `sidebarApps` | Sidebar app management |
| `terminal` | Terminal management |
| `codemirror` | CodeMirror 6 modules |
| `@codemirror/autocomplete` | CM6 autocomplete |
| `@codemirror/commands` | CM6 commands |
| `@codemirror/language` | CM6 language |
| `@codemirror/lint` | CM6 lint |
| `@codemirror/search` | CM6 search |
| `@codemirror/state` | CM6 state |
| `@codemirror/view` | CM6 view |
| `@lezer/highlight` | Lezer highlight tags |
| `createKeyboardEvent` | Keyboard event factory |
| `toInternalUrl` | Internal URL converter |
| `commands` | Command registration API |

### 7.2 Command System

```javascript
// Register a command
acode.addCommand({
    name: 'my-command',
    description: 'My custom command',
    bindKey: { win: 'Ctrl-Shift-M', mac: 'Cmd-Shift-M' },
    exec: () => { /* ... */ }
});

// Execute a command
acode.exec('my-command');

// Remove a command
acode.removeCommand('my-command');

// List all commands
acode.listCommands();
```

### 7.3 Sidebar Apps

```javascript
const { sidebarApps } = acode.require('sidebarApps');

// Add a sidebar app
sidebarApps.add('myApp', 'My App', 'icon_name', (app) => {
    // Return UI content
    return <div>My sidebar content</div>;
});

// Get a sidebar app
const app = sidebarApps.get('myApp');

// Remove a sidebar app
sidebarApps.remove('myApp');
```

### 7.4 Editor Themes

```javascript
const { editorThemes } = acode.require('editorThemes');

// Register a theme
editorThemes.register({
    id: 'my-theme',
    caption: 'My Theme',
    dark: true,
    getExtension: () => [
        EditorView.theme({ /* ... */ }, { dark: true }),
        syntaxHighlighting(HighlightStyle.define([/* ... */]))
    ]
});

// Apply a theme
editorThemes.apply('my-theme');

// List themes
editorThemes.list();
```

### 7.5 Language Modes

```javascript
const { editorLanguages } = acode.require('editorLanguages');

// Register a language mode
editorLanguages.register(
    'mylang',           // name
    ['mylang', 'ml'],   // extensions
    'My Language',       // caption
    () => import('./mylang-mode')  // lazy loader
);

// List modes
editorLanguages.list();

// Get mode by path
const mode = editorLanguages.getForPath('file.mylang');
```

### 7.6 Terminal Integration

```javascript
const { terminal } = acode.require('terminal');

// Create a terminal
const term = terminal.create({
    name: 'My Terminal',
    cwd: '/data/data/com.foxdebug.acode/files'
});

// Write to terminal
terminal.write(term.id, 'echo "Hello from plugin"\n');

// Close terminal
terminal.close(term.id);

// Register terminal theme
terminal.themes.register('my-term-theme', { /* xterm theme */ });
```

### 7.7 LSP (Language Server Protocol)

```javascript
const { lsp } = acode.require('lsp');

// Register a language server
lsp.registerServer({
    name: 'my-lsp',
    languages: ['mylang'],
    serverOptions: { /* ... */ },
    clientOptions: { /* ... */ }
});
```

### 7.8 Formatter Registration

```javascript
acode.registerFormatter(
    'my-formatter',           // id
    ['js', 'ts'],             // file extensions
    async () => {             // format function
        const editor = editorManager.editor;
        const code = editor.state.doc.toString();
        const formatted = await myFormatter.format(code);
        editor.dispatch({
            changes: { from: 0, to: code.length, insert: formatted }
        });
    },
    'My Formatter'            // display name
);
```

### 7.9 File Type Handlers

```javascript
acode.registerFileHandler('my-handler', {
    extensions: ['myext'],
    handleFile: async (fileInfo) => {
        // fileInfo = { name, uri, stats, readOnly, options }
        // Custom file opening logic
    }
});
```

### 7.10 File System Operations

```javascript
const fs = acode.require('fs');

// Read a file
const content = await fs('/path/to/file').readFile('utf-8');

// Write a file
await fs('/path/to/file').writeFile('content');

// List directory
const entries = await fs('/path/to/dir').lsDir();

// Check existence
const exists = await fs('/path/to/file').exists();

// Delete
await fs('/path/to/file').delete();
```

### 7.11 Dialogs

```javascript
const { alert, confirm, prompt, select, colorPicker, loader } = acode;

// Alert
await alert('Title', 'Message');

// Confirm
const yes = await confirm('Title', 'Are you sure?');

// Prompt
const value = await prompt('Enter value:', 'default');

// Select
const choice = await select('Pick one:', ['Option 1', 'Option 2']);

// Color picker
const hex = await colorPicker();

// Loading dialog
const l = loader.create('Loading', 'Please wait...');
l.show();
l.setMessage('90%');
l.destroy();
```

---

## 8. Plugin Update System

### 8.1 Update Check: `checkPluginsUpdate()` (`src/lib/checkPluginsUpdate.js`)

```javascript
async function checkPluginsUpdate() {
    const plugins = await fsOperation(PLUGIN_DIR).lsDir();
    const updates = [];
    
    for (const pluginDir of plugins) {
        const plugin = await fsOperation(
            Url.join(pluginDir.url, "plugin.json")
        ).readFile("json");
        
        const res = await fetch(
            `${API_BASE}/plugin/check-update/${plugin.id}/${plugin.version}`
        );
        
        if (res.ok) {
            const json = await res.json();
            if (json.update) updates.push(plugin.id);
        }
    }
    
    return updates;
}
```

Called on app resume and at startup. Shows a notification if updates are available.

### 8.2 Plugin Marketplace UI

Lazy-loaded from `src/pages/plugins/plugins.js`:

```javascript
function plugins(updates) {
    import("./plugins").then(res => res.default(updates));
}
```

---

## 9. Plugin State Management

### 9.1 State Tracking

| Data Structure | Location | Purpose |
|---------------|----------|---------|
| `LOADED_PLUGINS` | `loadPlugins.js` | Set of successfully loaded plugin IDs |
| `BROKEN_PLUGINS` | `loadPlugins.js` | Map of plugin ID → {error, timestamp} |
| `AUTO_DISABLED_PLUGINS` | `loadPlugins.js` | Set of auto-disabled plugin IDs |
| `settings.value.pluginsDisabled` | `settings.js` | Persisted map of disabled plugin IDs |
| `InstallState` | `installState.js` | SHA-256 checksums of installed files |
| `#pluginsInit` | `acode.js` | Map of plugin ID → init function |
| `#pluginUnmount` | `acode.js` | Map of plugin ID → cleanup function |
| `#formatter` | `acode.js` | Array of registered formatters |

### 9.2 Plugin Enable/Disable

Plugins can be disabled by users via settings or automatically when broken:

```javascript
async function updatePluginDisabled(pluginId, disabled) {
    const disabledMap = { ...(settings.value.pluginsDisabled || {}) };
    if (disabled) {
        disabledMap[pluginId] = true;
    } else {
        delete disabledMap[pluginId];
    }
    await settings.update({ pluginsDisabled: disabledMap }, false);
}
```

Disabled plugins are skipped during loading:
```javascript
enabledMap[pluginId] !== true  // true = disabled
```

### 9.3 Plugin Waiting

Plugins can wait for other plugins to load:

```javascript
// In plugin A
await acode.waitForPlugin('plugin-B');
// Resolves when plugin-B loads, or rejects after all plugins finish loading
```

---

## 10. Plugin Security

### 10.1 Path Traversal Protection
- All zip paths are sanitized to prevent `../` traversal
- Absolute paths (/, C:/, /data, /system, etc.) are rejected
- Files are always written under `PLUGIN_DIR/<plugin-id>/`

### 10.2 Terminal Write Security
The `Acode` class includes a `#secureTerminalWrite()` method that blocks dangerous commands:

```javascript
const dangerousPatterns = [
    /^\s*rm\s+-rf?\s+\//m,        // rm -rf /
    /^\s*:(){ :|:& };:/m,          // Fork bomb
    /^\s*curl\s+.*\|\s*sh/m,       // curl | sh
    /^\s*sudo\s+dd\s+if=\//m,      // sudo dd
    /\x00/g,                        // Null bytes
    // ... more patterns
];
```

### 10.3 Plugin Context
Each plugin gets an isolated `PluginContext`:
```javascript
ctx: await PluginContext.generate(pluginId, JSON.stringify(pluginJson))
```

### 10.4 Sandbox
Plugins are loaded via script injection but have access to the full Acode API. The security model relies on:
- Plugin review on the Acode registry
- User consent for installation
- Auto-disable on errors/timeouts
- Path sanitization for file operations

---

## 11. Error Handling & Recovery

### 11.1 Error Flow

```
Plugin throws error during load()
    │
    ▼
markPluginBroken(pluginId, error)
    ├── Add to BROKEN_PLUGINS map
    ├── Add to AUTO_DISABLED_PLUGINS set
    └── Persist to settings.pluginsDisabled
    │
    ▼
Plugin skipped on subsequent loads
    │
    ▼
User can manually re-enable via Settings > Plugins
    │
    ▼
acode.clearBrokenPluginMark(pluginId)  // Clears BROKEN_PLUGINS entry
    │
    ▼
Plugin retried on next load
```

### 11.2 Timeout Handling

```
Plugin script loads but initPlugin() takes > 15s
    │
    ▼
PluginLoadTimeoutError thrown
    │
    ▼
markPluginTimedOut() called
    ├── Add to BROKEN_PLUGINS (tentative)
    └── Set 45s grace period timer
    │
    ▼
If plugin completes within 45s → markPluginLoaded()
If not → markPluginBroken()
```

### 11.3 Cleanup on Failed Install

```javascript
// installPlugin.js:265-282
} catch (err) {
    try {
        if (state) await state.clear();
        if (pluginDir && (await fsOperation(pluginDir).exists())) {
            await fsOperation(pluginDir).delete();
        }
    } catch (cleanupError) {
        console.error("Cleanup failed:", cleanupError);
    }
    throw err;
}
```

---

## 12. Plugin Settings Integration

Plugins can register their own settings page:

```javascript
acode.setPluginInit('my-plugin', initFn, {
    list: [
        {
            text: 'Enable Feature X',
            key: 'featureX',
            value: true,
            checkbox: true,
        },
        {
            text: 'Server URL',
            key: 'serverUrl',
            value: 'https://api.example.com',
        },
    ],
    cb: (key, value) => {
        console.log(`Setting ${key} changed to ${value}`);
    }
});
```

This creates a settings page accessible via Settings > Plugin Settings > My Plugin.

---

## 13. Summary: Complete Plugin Lifecycle

```
1. REGISTRY: Plugin published to acode.app/api
2. DOWNLOAD: User taps "Install" in marketplace
3. INSTALL: ZIP downloaded → extracted → files written
4. LOAD: <script> tag injected → initPlugin() called
5. ACTIVE: Plugin registers commands, sidebar apps, etc.
6. UPDATE: checkPluginsUpdate() detects new version
7. RELOAD: unmountPlugin() → loadPlugin() → initPlugin()
8. DISABLE: User toggles off OR auto-disable on error
9. REMOVE: Plugin directory deleted
```

---

*This analysis covers the complete plugin system including installation, loading, lifecycle management, API surface, security measures, and error handling.*
