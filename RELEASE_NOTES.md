## Release Notes

### v0.1.10 (Unreleased)

#### Features

* **Session Persistence**: Automatically saves per-project state (open files, tabs, split layout, cursor/scroll positions, file explorer state, search/replace history and options, bookmarks) to the XDG data dir and restores it on launch. Session restore is skipped when opening a specific file; use `--no-session` to start fresh.

* **Unified Search & Replace**: Replace (Ctrl+H) and Query Replace (Ctrl+Shift+H) now share the same interface with a "Confirm each" toggle (Alt+E). Query Replace enables confirmation by default; Replace uses the toggle state. Confirmation prompt shows `(y)es (n)o (a)ll (c)ancel` options.

#### Bug Fixes

* **Session Restore Reliability**: Fixed session rehydration to reopen files/splits with the correct active buffer, cursor, and scroll position (including nested splits) instead of jumping back to the top on first render.

* **macOS Build**: Fixed Linux-specific `.init_array` by using cross-platform V8 initialization.

* **Syntax Highlighting**: Fixed invisible/hard-to-read highlighting in light and nostalgia themes by using theme-based color resolution instead of hardcoded colors.

* **Theme Colors**: Improved status bar and prompt colors across all themes (dark, high-contrast, light, nostalgia).

* **Search Prompt**: Search/replace prompts now cancel when focus leaves the editor (switching buffers or focusing file explorer).

---

### v0.1.9

#### Features

* **Native File Browser**: New built-in file browser for Open File command (Ctrl+O) that works without plugins. Features sortable columns (name, size, modified), navigation shortcuts (parent, home, root), filtering with grayed non-matches, mouse support with hover indicators, and async directory loading.

* **CRLF Line Ending Support**: Transparent handling of Windows-style line endings. Files are detected and normalized internally, then saved with their original line ending format preserved.

* **CLI Enhancements**: Added `--version`, `--no-plugins` (skip JS runtime for faster startup), `--log-file`, and `--config` flags.

* **UI Improvements**:
  - Tab hover effects with close button changing to red on hover
  - Menu hover-to-switch when a menu is open
  - Buffer name shown in modified buffer confirmation prompts
  - Fixed column widths in command palette for stable layout

#### Bug Fixes

* **V8 Segfault**: Fixed crash when creating multiple Editor instances (e.g., in tests) by initializing V8 platform once at library load.

* **Windows**: Fixed duplicate key presses caused by processing both Press and Release events.

---

### v0.1.8

#### Bug Fixes

* **Open File Prompt**: Fixed completions not showing immediately (issue #193) by enabling ICU support for Unicode functions.

* **Keyboard Shortcuts Help**: Fixed crash when reopening keyboard shortcuts buffer (issue #192).

* **Undo Save Points**: Fixed extra undo step at beginning of save history (issue #191).

* **Scroll Keybindings**: Fixed Ctrl+Up/Down scroll not working by syncing viewport between SplitViewState and EditorState.

---

### v0.1.7

#### Features

* **Select Theme Command**: New theme picker accessible from the command palette and View menu. Includes a new "nostalgia" theme inspired by Turbo Pascal 5 / WordPerfect 5.

* **Compose Mode Improvements**: Paper-on-desk visual effect with desk margin colors, and hanging indent support for markdown lists and blockquotes.

* **Binary File Detection**: Binary files are now detected and opened in read-only mode to prevent accidental corruption.

#### Bug Fixes

* **Light Theme**: Fixed colors for status bar, prompt, scrollbar, tabs, and file explorer to use proper light theme colors.

* **Mouse Performance**: Fixed slow mouse movement on large terminals by skipping redundant renders when hover target hasn't changed. Added mouse event coalescing to skip stale positions.

* **UTF-8 Truncation**: Fixed panic when truncating suggestion descriptions mid-character.

#### Internal Changes

* **Code Refactoring**: Major cleanup extracting helpers and reducing duplication across many modules including `process_async_messages`, `handle_plugin_command`, `render_view_lines`, `multi_cursor`, `highlight_color`, and more. Consolidated duplicate `hook_args_to_json` implementations.

* **Test Improvements**: Fixed flaky tests by removing timing assertions, made shortcut tests platform-aware for macOS.

* **Documentation**: Reorganized internal planning docs, updated plugin README from Lua to TypeScript, and added embedded help manual using `include_str!()`.
