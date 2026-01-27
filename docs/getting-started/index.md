# Getting Started

## Installation

See the [Installation section in the README](../README.md#installation) for all available installation methods, including Homebrew, AUR, .deb/.rpm packages, npm, crates.io, and building from source.





## Running Fresh

To run Fresh, you can either open it without a file, or specify a file to open:

```bash
# Open an empty buffer
fresh

# Open a file
fresh src/main.rs

# Open a file at a specific line
fresh src/main.rs:42

# Open a file at a specific line and column
fresh src/main.rs:42:10

# Open multiple files (with optional line:col)
fresh Cargo.toml src/lib.rs:100:5
```

The `file:line:col` syntax is useful for jumping directly to compiler errors or search results.

## Core Concepts

*   **The Command Palette:** The command palette is your central hub for accessing all of Fresh's features. Press `Ctrl+P` to open it, and then start typing to search for commands.
*   **Buffers:** Each open file is represented as a buffer. You can have multiple buffers open at once and switch between them.
*   **Splits:** You can split your editor view horizontally or vertically to view multiple buffers at once.
*   **The Status Bar:** The status bar at the bottom of the screen displays information about the current buffer, including the file name, cursor position, and Git branch.
