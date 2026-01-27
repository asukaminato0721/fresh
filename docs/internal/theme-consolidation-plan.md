# Theme Consolidation Plan

Currently, built-in themes in Fresh are defined in two ways: some are hardcoded in Rust source code (`src/view/theme.rs`), while others are loaded from JSON files. This creates a "split-brain" problem where the source of truth is divided, making maintenance difficult and leading to inconsistencies (e.g., the Theme Editor plugin not seeing Rust-defined themes).

This plan outlines a holistic approach to unify theme management.

## 1. Single Source of Truth: Embedded JSON

All built-in themes (Dark, Light, High Contrast, Nostalgia) will be moved to JSON files in the `themes/` directory.

- **Storage:** `themes/*.json`
- **Embedding:** Use `include_str!` in Rust to embed these JSON files directly into the binary.
- **Loading:** Themes will be deserialized from the embedded JSON strings at runtime.

## 2. Automated Schema Validation

To ensure that JSON themes stay in sync with the `ThemeFile` structure defined in Rust:

- **Build-time/Test-time Validation:** A Rust test will iterate over all embedded themes and attempt to deserialize them into the `ThemeFile` struct.
- **CI Enforcement:** If a developer adds a new field to the `Theme` schema but fails to update the JSON files, the CI test will fail.

## 3. API-Driven Theme Discovery

The Plugin API will be updated to allow plugins (like the Theme Editor) to discover built-in themes without needing to scan the filesystem.

- **New API Method:** `getBuiltinThemes()` will return a list of available built-in theme names and their JSON content (or a way to fetch the content).
- **Benefit:** This removes the requirement to distribute a `themes/` folder alongside the binary, as all built-in themes are part of the binary itself.

## 4. Implementation Steps

1. **Extraction:** Programmatically extract existing Rust-defined themes (`dark`, `light`, `high-contrast`, `nostalgia`) into `themes/*.json` files.
2. **Standardization:** Ensure all JSON files follow the latest `ThemeFile` schema.
3. **Refactoring Rust:**
    - Add `include_str!` for each theme in `src/view/theme.rs`.
    - Replace hardcoded color definitions in `Theme::dark()`, `Theme::light()`, etc., with logic that loads from the embedded JSON.
4. **Validation Test:** Implement the `test_builtin_themes_match_schema` test.
5. **Plugin Update:** Update `plugins/theme_editor.ts` to use the new discovery mechanism.
