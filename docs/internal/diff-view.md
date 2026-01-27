# Diff View Design

**Status**: In Development
**Last Updated**: 2025-01-01

This document consolidates all design documentation for the diff viewing and code review features in Fresh Editor.

---

# Part 1: UX Design

## Overview

The diff view feature transforms Fresh Editor into a "decision engine" for reviewing, annotating, and staging code changes. It supports two primary workflows:

1. **Review Diff** - Unified stream view for reviewing AI-generated or collaborator changes
2. **Side-by-Side Diff** - Traditional two-pane comparison view

## Quick Start

1. Open Review Diff: `Ctrl+P` → "Review Diff"
2. Navigate: Arrow keys to move, `n`/`p` to jump between hunks
3. Comment: `c` on any line to add feedback
4. Review: `a` approve, `x` reject, `!` needs changes
5. Drill down: `Enter` on a hunk for side-by-side view
6. Export: `E` to save feedback to `.review/session.md`

## Keyboard Shortcuts (review-mode)

### Navigation
| Key | Action |
|-----|--------|
| `n` | Next hunk |
| `p` | Previous hunk |
| `Enter` | Drill down to side-by-side view |
| `r` | Refresh diff |
| `q` | Close buffer |
| Arrow keys | Move cursor within buffer |

### Commenting
| Key | Action |
|-----|--------|
| `c` | Add comment at cursor position |
| `O` | Set overall session feedback |

### Review Status
| Key | Action |
|-----|--------|
| `a` | Approve hunk |
| `x` | Reject hunk |
| `!` | Mark as needs changes |
| `?` | Mark with question |
| `u` | Clear/undo status |

### Staging
| Key | Action |
|-----|--------|
| `s` | Stage hunk (accept change) |
| `d` | Discard hunk (reject change) |

### Export
| Key | Action |
|-----|--------|
| `E` | Export to `.review/session.md` |

## Visual Layout

### Unified Review Stream
```
┌─────────────────────────────────────────────────────┐
│ [Keybindings: n=next p=prev s=stage d=discard ...]  │
├─────────────────────────────────────────────────────┤
│ src/auth.ts                                         │
│ @@ -45,7 +45,9 @@ function validateToken()          │
│   const token = req.headers.auth;                   │  ← context (unchanged)
│ - const decoded = jwt.verify(token);                │  ← deletion (red bg)
│ + const decoded = jwt.verify(token, secret);        │  ← addition (green bg)
│ + if (!decoded) throw new AuthError();              │  ← addition (green bg)
│   return decoded.userId;                            │  ← context
│ » [+47] Consider adding validation here             │  ← inline comment
└─────────────────────────────────────────────────────┘
```

### Side-by-Side View
```
┌──────────────────────────┬──────────────────────────┐
│  OLD                     │  NEW                     │
├──────────────────────────┼──────────────────────────┤
│  45 │ const token = ...  │  45 │ const token = ...  │
│  46 │ jwt.verify(token); │  46 │ jwt.verify(token,  │
│     │                    │  47 │ if (!decoded) ...  │  ← gap alignment
│  47 │ return decoded...  │  48 │ return decoded...  │
└──────────────────────────┴──────────────────────────┘
```

## Comment System

Comments are attached to specific file line numbers (not hunk-relative), making them robust to rebases and squashes.

### Adding Comments
1. Navigate to a specific line in the diff
2. Press `c`
3. Enter your comment text
4. Press Enter to confirm

The prompt shows the line reference:
- `Comment on +42:` for added lines (new file line 42)
- `Comment on -38:` for removed lines (old file line 38)
- `Comment on hunk:` when on hunk header

### Export Format

**Markdown (`.review/session.md`)**:
```markdown
## File: src/auth.ts

### validateToken (line 45)
**Status**: NEEDS_CHANGES

**Comments:**
> » [+45] Consider adding validation here
> `const token = req.headers.authorization;`
```

**JSON (`.review/session.json`)**:
```json
{
  "files": {
    "src/auth.ts": {
      "hunks": [{
        "context": "validateToken",
        "new_lines": [45, 52],
        "status": "needs_changes",
        "comments": [{
          "text": "Consider adding validation here",
          "line_type": "add",
          "new_line": 45
        }]
      }]
    }
  }
}
```

## UX Requirements

### Must Have
- [x] Colorized diff output (red=removed, green=added, blue=context)
- [x] Hunk navigation (n/p keys)
- [x] Per-hunk staging (s/d keys)
- [x] Side-by-side drill-down
- [x] Line-level comments
- [x] Export to markdown/JSON
- [ ] Line-level alignment in side-by-side view (gaps for insertions)
- [ ] Syntax highlighting in diff view

### Nice to Have
- [ ] Original prompt display (what user asked the agent)
- [ ] Summary header (at-a-glance change statistics)
- [ ] Safety warnings (secrets, large deletions)
- [ ] 3-pane merge view for conflicts

## Known UX Issues

1. **No line alignment**: Side-by-side view doesn't align corresponding lines with gaps
2. **No syntax highlighting**: Diff content shows plain text colors
3. **Arrow key navigation only**: `j`/`k` don't work in review-mode
4. **Pane order reversed**: NEW|OLD instead of conventional OLD|NEW (API limitation)

---

# Part 2: Architecture & Implementation

## Design Decision: Dual-Mode Rendering (Option E)

After analyzing the rendering pipeline, we chose **dual-mode rendering** for aligned diff views:

### The Problem

The normal buffer rendering pipeline (`render_view_lines`) renders **consecutive lines** from a buffer. Aligned diff views require **non-consecutive rendering with gaps**:

```
Old        | New
Line 1     | Line 1
Line 2     | Line 2
Line 3     | Line 3
[gap]      | Line 4  ← insertion, old side shows gap
Line 4     | Line 5
```

Calling `render_buffer_in_split()` with a calculated `top_byte` would render consecutive lines without gaps.

### Solution: Dual-Mode Rendering

1. **Normal mode**: Existing `render_view_lines()` for regular buffers
2. **Aligned mode**: New `render_aligned_view_lines()` for composite buffers

Both modes share:
- `build_view_data()` - token building, syntax highlighting, line wrapping
- Extracted helper functions for gutter, character styling, cursor rendering

The aligned mode:
- Takes ViewData for each pane + alignment info
- For each display row, looks up the ViewLine for each pane (or renders a gap)
- Renders with shared helper functions

### Why Not Full Reuse?

We considered having CompositeBuffer call `render_buffer_in_split()` per pane, but:
- That renders consecutive lines, not aligned lines with gaps
- The alignment requires rendering specific source lines at specific display rows
- Gap rows have no source content to render

## High-Level Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                      CompositeBuffer                         │
│                                                              │
│  ┌──────────────────┐        ┌──────────────────┐           │
│  │     Pane 0       │        │     Pane 1       │           │
│  │  ┌────────────┐  │        │  ┌────────────┐  │           │
│  │  │EditorState │  │        │  │EditorState │  │           │
│  │  │  - buffer  │  │        │  │  - buffer  │  │           │
│  │  │  - cursors │  │        │  │  - cursors │  │           │
│  │  │  - highlight│ │        │  │  - highlight│ │           │
│  │  │  - overlays │  │        │  │  - overlays │  │           │
│  │  └────────────┘  │        │  └────────────┘  │           │
│  └──────────────────┘        └──────────────────┘           │
│                                                              │
│  ┌────────────────────────────────────────────────────────┐ │
│  │                   ChunkAlignment                        │ │
│  │  chunks: [Context, Hunk, Context, Hunk, Context, ...]  │ │
│  │  (markers at chunk boundaries for edit-robustness)     │ │
│  └────────────────────────────────────────────────────────┘ │
│                                                              │
│  scroll_display_row: usize   (unified scroll position)      │
│  focused_pane: usize         (which pane receives input)    │
└─────────────────────────────────────────────────────────────┘
```

## Core Data Structures

### CompositeBuffer

```rust
pub struct CompositeBuffer {
    pub id: BufferId,
    pub name: String,
    pub layout: CompositeLayout,
    pub sources: Vec<SourcePane>,
    pub alignment: LineAlignment,
    pub active_pane: usize,
    pub mode: String,
}

pub enum CompositeLayout {
    SideBySide { ratios: Vec<f32>, show_separator: bool },
    Stacked { spacing: u16 },
    Unified,
}

pub struct SourcePane {
    pub buffer_id: BufferId,
    pub label: String,
    pub editable: bool,
    pub style: PaneStyle,
    pub range: Option<Range<usize>>,
}
```

### ChunkAlignment (Edit-Robust)

Traditional alignment stores line numbers, which break on edit. We use **markers at chunk boundaries**:

```rust
struct ChunkAlignment {
    chunks: Vec<AlignmentChunk>,
}

struct AlignmentChunk {
    /// Marker at the START of this chunk in each pane
    /// None if this pane has no content (e.g., pure insertion)
    start_markers: Vec<Option<MarkerId>>,
    kind: ChunkKind,
    dirty: bool,  // Needs recomputation after edit
}

enum ChunkKind {
    Context { line_count: usize },
    Hunk { ops: Vec<(usize, usize)> },  // (old_lines, new_lines) pairs
}
```

**Example:**
```
  Line 1    |   Line 1      (context)
  Line 2    |   Line 2      (context)
- Line 3    |               (deletion)
- Line 4    |               (deletion)
            |+  New 3       (insertion)
  Line 5    |   Line 5      (context)
```

Becomes:
```rust
chunks: [
    AlignmentChunk {
        start_markers: [M0_old, M0_new],  // At "Line 1"
        kind: Context { line_count: 2 },
    },
    AlignmentChunk {
        start_markers: [M1_old, M1_new],  // At "Line 3" / "New 3"
        kind: Hunk { ops: [(1,0), (1,0), (0,1)] },  // del, del, ins
    },
    AlignmentChunk {
        start_markers: [M2_old, M2_new],  // At "Line 5"
        kind: Context { line_count: 1 },
    },
]
```

**Total: 6 markers** (2 per chunk) instead of one per line.

### Edit Handling

When a buffer is edited:
1. **Markers auto-adjust** their byte positions (handled by buffer's marker system)
2. **Context chunks**: Update `line_count` based on lines inserted/deleted
3. **Hunk chunks**: Mark as `dirty` for localized re-diffing

```rust
impl ChunkAlignment {
    fn on_buffer_edit(&mut self, pane_idx: usize, edit_line: usize, lines_delta: isize) {
        for chunk in &mut self.chunks {
            if chunk.contains_line(pane_idx, edit_line) {
                match &mut chunk.kind {
                    ChunkKind::Context { line_count } => {
                        *line_count = (*line_count as isize + lines_delta) as usize;
                    }
                    ChunkKind::Hunk { .. } => {
                        chunk.dirty = true;
                    }
                }
                return;
            }
        }
    }
}
```

## Rendering Pipeline

### Normal Buffer (Existing)
```
EditorState
    ↓ build_view_data()
ViewData { lines: Vec<ViewLine> }
    ↓ render_view_lines()
Screen
```

### Composite Buffer (New)
```
Per-pane EditorState
    ↓ build_view_data() (reused!)
Per-pane ViewData
    ↓ render_aligned_view_lines() (new!)
Screen with aligned panes
```

### render_aligned_view_lines

```rust
fn render_aligned_view_lines(
    frame: &mut Frame,
    pane_areas: &[Rect],
    pane_view_data: &[ViewData],
    alignment: &ChunkAlignment,
    view_state: &CompositeViewState,
    theme: &Theme,
) {
    let display_rows = alignment.to_display_rows();

    for (view_row, aligned_row) in display_rows.iter()
        .skip(view_state.scroll_row)
        .take(viewport_height)
        .enumerate()
    {
        for (pane_idx, pane_area) in pane_areas.iter().enumerate() {
            let row_rect = Rect { y: pane_area.y + view_row, height: 1, ..*pane_area };

            match aligned_row.get_pane_line(pane_idx) {
                Some(source_line) => {
                    // Find ViewLine for this source line
                    let view_line = find_view_line(&pane_view_data[pane_idx], source_line);
                    render_single_view_line(frame, row_rect, view_line, ...);
                }
                None => {
                    // Gap row - render empty with appropriate background
                    render_gap_row(frame, row_rect, aligned_row.row_type, theme);
                }
            }
        }
    }
}
```

## Scroll Synchronization

The composite buffer has a unified `scroll_display_row`. Each pane's viewport is derived:

```rust
impl CompositeBuffer {
    fn derive_pane_top_byte(&self, pane_idx: usize, display_row: usize) -> usize {
        let display_rows = self.alignment.to_display_rows();

        display_rows
            .get(display_row)
            .and_then(|row| row.pane_lines.get(pane_idx))
            .flatten()
            .and_then(|line| self.pane_buffer(pane_idx).line_start_offset(line))
            .unwrap_or(0)
    }
}
```

## Input Routing

Cursor and edit actions go to the focused pane's EditorState:

```rust
impl CompositeBuffer {
    fn handle_action(&mut self, action: Action) -> Option<Event> {
        match action {
            Action::FocusNextPane => {
                self.focused_pane = (self.focused_pane + 1) % self.panes.len();
                None
            }
            Action::CursorDown => {
                self.scroll_display_row += 1;
                self.focused_pane_state_mut().handle_cursor_down()
            }
            Action::Insert(_) | Action::Delete => {
                self.focused_pane_state_mut().handle_action(action)
            }
            _ => None
        }
    }
}
```

## Diff Highlighting via Overlays

Add overlays to each pane's EditorState based on alignment:

```rust
fn apply_diff_overlays(&mut self, theme: &Theme) {
    let display_rows = self.alignment.to_display_rows();

    for (pane_idx, pane_state) in self.pane_states.iter_mut().enumerate() {
        pane_state.overlays.clear_category("diff");

        for row in &display_rows {
            if let Some(source_line) = row.pane_lines.get(pane_idx).flatten() {
                let bg_color = match row.row_type {
                    RowType::Addition => Some(theme.diff_add_bg),
                    RowType::Deletion => Some(theme.diff_remove_bg),
                    RowType::Modification => Some(theme.diff_modify_bg),
                    _ => None,
                };

                if let Some(color) = bg_color {
                    let range = pane_state.buffer.line_byte_range(source_line);
                    pane_state.overlays.add(Overlay {
                        range,
                        face: OverlayFace::Background { color },
                        category: "diff".to_string(),
                    });
                }
            }
        }
    }
}
```

## File Structure

| File | Purpose |
|------|---------|
| `src/model/composite_buffer.rs` | CompositeBuffer, SourcePane, LineAlignment, ChunkAlignment |
| `src/view/composite_view.rs` | CompositeViewState, PaneViewport |
| `src/view/ui/split_rendering.rs` | render_aligned_view_lines, helper extraction |
| `src/input/composite_router.rs` | Input routing to focused pane |
| `src/app/composite_buffer_actions.rs` | Editor methods for composite buffers |
| `plugins/audit_mode.ts` | Review Diff plugin (TypeScript) |

## Implementation Phases

### Phase 1: Helper Extraction (Current)
- [ ] Extract gutter rendering from render_view_lines
- [ ] Extract character style computation
- [ ] Extract cursor rendering logic
- [ ] Create render_single_view_line helper

### Phase 2: Aligned Rendering
- [ ] Implement render_aligned_view_lines
- [ ] Add source_line → ViewLine lookup
- [ ] Implement gap row rendering
- [ ] Wire up to composite buffer path

### Phase 3: ChunkAlignment
- [ ] Implement ChunkAlignment with markers
- [ ] Add to_display_rows() conversion
- [ ] Implement on_buffer_edit() for live updates
- [ ] Add dirty chunk re-diffing

### Phase 4: Polish
- [ ] Syntax highlighting in diff view
- [ ] Cursor navigation within aligned view
- [ ] Selection across aligned rows
- [ ] Performance optimization

## Summary

| Aspect | Design |
|--------|--------|
| **Rendering approach** | Dual-mode: consecutive (normal) + aligned (composite) |
| **ViewLine building** | Fully reused via build_view_data() |
| **Alignment storage** | Chunks with markers at boundaries |
| **Edit robustness** | Markers auto-adjust; context updates count; hunks marked dirty |
| **Scroll sync** | Unified display_row → per-pane top_byte via alignment |
| **Diff highlighting** | Overlays on pane EditorStates |
| **Input handling** | Route to focused pane's EditorState |

## Benefits

1. **ViewLine reuse** - syntax highlighting, ANSI, virtual text all work
2. **Edit-robust alignment** - markers + chunks handle edits gracefully
3. **Minimal markers** - O(chunks) not O(lines)
4. **Localized recomputation** - only dirty chunks re-diffed
5. **Low risk** - existing render_view_lines unchanged
