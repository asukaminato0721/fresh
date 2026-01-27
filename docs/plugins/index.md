# Plugins

Fresh's functionality can be extended with plugins written in TypeScript. Fresh comes with a few useful plugins out of the box:

*   **TODO Highlighter:** Highlights `TODO`, `FIXME`, and other keywords in your comments.
*   **Git Grep:** Interactively search through your Git repository.
*   **Git Find File:** Quickly find and open files in your Git repository.

> On macOS, plugins folder needs to live either in the same directory as the binary OR in the directory that fresh is run from. If installed via homebrew, the binary lives in ```/opt/homebrew/bin/fresh```. The simplest, cleanest way to to create a symbolic link in that folder pointing to your plugins. i.e. ``` ln -s /Users/username/freshplugins /opt/homebrew/bin/plugins```

### Package Manager

Fresh includes a built-in package manager for installing plugins and themes from git repositories.

#### Installing Packages

Use the command palette (`Ctrl+P`) and search for:

| Command | Description |
|---------|-------------|
| `pkg: Install Plugin` | Browse and install plugins from the registry |
| `pkg: Install Theme` | Browse and install themes from the registry |
| `pkg: Install from URL` | Install directly from any git repository URL |
| `pkg: List Installed` | Show all installed packages |
| `pkg: Update All` | Update all installed packages |
| `pkg: Remove Package` | Remove an installed package |

#### Installing from Git URL

Any git repository can be installed directly:

1. Open command palette (`Ctrl+P`)
2. Search for `pkg: Install from URL`
3. Enter the git repository URL (e.g., `https://github.com/user/fresh-plugin`)
4. Restart Fresh to activate the plugin

**Monorepo support:** For repositories containing multiple plugins, use a URL fragment to specify the subdirectory:

```
https://github.com/user/fresh-plugins#packages/rainbow-brackets
```

This installs only the `packages/rainbow-brackets` directory from the repository.

#### Package Locations

Installed packages are stored in:
- **Plugins:** `~/.config/fresh/plugins/packages/`
- **Themes:** `~/.config/fresh/themes/packages/`

Each package is a git repository, so you can update manually with `git pull` if needed.

#### Registry Sources

By default, Fresh uses the official package registry. You can add additional registries in your config:

```json
{
  "packages": {
    "sources": [
      "https://github.com/sinelaw/fresh-plugins-registry",
      "https://github.com/my-org/private-plugins"
    ]
  }
}
```

Run `pkg: Sync Registry` to fetch the latest package lists.

### Clangd helper plugin

Fresh ships `plugins/clangd_support.ts` with the source tree; see `plugins/clangd_support.md` for an overview of the plugin commands and how it surfaces clangd-specific notifications in the status bar.