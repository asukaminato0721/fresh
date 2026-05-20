# Orchestrator: Open Dialog Redesign + Session Lifecycle

> **Status**: Design Document
> **Date**: May 2026
> **Driving feature**: A picker UX for browsing/operating on
> Orchestrator sessions, plus a richer session lifecycle (Stop /
> Archive / Delete) with cross-machine recovery.

## Motivation

The current `Orchestrator: Open` picker is built on the legacy
`startPrompt` infrastructure. It works for "list + filter +
pick one", but it can't host the action surface Orchestrator
needs:

- No per-session actions besides "dive" — killing a session
  requires a separate command.
- No custom keystroke handling — typing collides with filter
  text, and we can't bind `Alt+D` or similar to "kill the
  highlighted session" without leaking the chord to global
  bindings.
- Pseudo-rows (`+ New`, `× Kill highlighted`) get filtered out
  by the prompt's fuzzy matcher, so they vanish exactly when
  the user has typed something they want to act on.

Separately, the term "kill" is too coarse. Users want three
distinct lifecycle operations:

- **Stop** — abort the session's running processes but keep
  the worktree and editor session around.
- **Archive** — declare "I'm done with this for now" and move
  the worktree out of the way, while keeping it recoverable.
- **Delete** — permanently remove the worktree and all
  metadata.

Finally, when a session represents real work-in-progress, the
user wants to recover it on a different machine — pulling a
git branch should be enough to resume.

## Wireframe

The new dialog is rendered through the existing
`FloatingWidgetPanel` infrastructure (the same primitive the
new-session form already uses), composed from `text` (filter
input), `list` (sessions), and `labeledSection` (chrome) widgets.

### Normal state — session highlighted, action menu in preview pane

```
╭─ ORCHESTRATOR :: Sessions ────────────────────────────────────────────╮
│ ╭─ Filter ───────────────────────────────────────────────────────╮ │
│ │ [filter text                                                  ]│ │
│ ╰────────────────────────────────────────────────────────────────╯ │
│ ╭─ Sessions ─────────────╮ ╭─ [2] moshiko ──────────────────────╮  │
│ │   [1] ACT  fresh       │ │ Root:  /home/noam/repos/fresh/     │  │
│ │ ▸ [2] RUN  moshiko     │ │        .fresh/orchestrator/moshiko    │  │
│ │   [3] RUN  session-1   │ │ Age:   3m       State: RUN         │  │
│ │   [4] RUN  session-2   │ │ pgid:  12345    pids: 12345, 12387 │  │
│ │                        │ │ ────────────────────────────────── │  │
│ │                        │ │ Last terminal lines:               │  │
│ │                        │ │   $ make build                     │  │
│ │                        │ │   compiling...                     │  │
│ │                        │ │ ────────────────────────────────── │  │
│ │                        │ │ ▸ Dive into session                │  │
│ │                        │ │   Stop processes      (Alt+S)      │  │
│ │                        │ │   Archive             (Alt+A)      │  │
│ │                        │ │   Delete permanently  (Alt+D)      │  │
│ ╰────────────────────────╯ ╰────────────────────────────────────╯  │
│                                                                    │
│  ↑↓ nav · Tab focus · Enter activate · Alt+N new · Esc close       │
╰────────────────────────────────────────────────────────────────────╯
```

### Confirmation state — `Delete` chosen

Only irreversible actions confirm. `Stop` and `Archive` are
both recoverable (relaunch the agent; unarchive the session),
so they fire immediately.

```
╭─ ORCHESTRATOR :: Sessions ────────────────────────────────────────────╮
│ ╭─ Filter ───────────────────────────────────────────────────────╮ │
│ │ [filter text                                                  ]│ │
│ ╰────────────────────────────────────────────────────────────────╯ │
│ ╭─ Sessions ─────────────╮ ╭─ Confirm Delete ───────────────────╮  │
│ │   [1] ACT  fresh       │ │                                    │  │
│ │ ▸ [2] RUN  moshiko     │ │ Delete session [2] moshiko?        │  │
│ │   [3] RUN  session-1   │ │                                    │  │
│ │   [4] RUN  session-2   │ │ This will:                         │  │
│ │                        │ │   • stop all session processes     │  │
│ │                        │ │   • run `git worktree remove`      │  │
│ │                        │ │   • drop the session record        │  │
│ │                        │ │                                    │  │
│ │                        │ │ Uncommitted changes will be lost.  │  │
│ │                        │ │                                    │  │
│ │                        │ │ [ Confirm Delete ]   [ Cancel ]    │  │
│ ╰────────────────────────╯ ╰────────────────────────────────────╯  │
│                                                                    │
│  Tab focus · Enter activate · Esc cancel                           │
╰────────────────────────────────────────────────────────────────────╯
```

### Archived row state — action menu swaps

When the highlighted row is an archived session, `Archive`
becomes `Unarchive` and `Stop` is hidden (no live processes):

```
                            │ ▸ Dive into session                │
                            │   Unarchive           (Alt+A)      │
                            │   Delete permanently  (Alt+D)      │
```

Diving into an archived session implicitly unarchives it first
(can't activate a closed editor window).

## Action semantics

| Action     | Touches processes              | Touches worktree                                | Touches editor session | Recoverable                | Needs confirm |
|------------|--------------------------------|-------------------------------------------------|------------------------|----------------------------|---------------|
| Dive       | no                             | no                                              | sets active            | n/a                        | no            |
| Stop       | SIGTERM → SIGKILL the pgid     | no                                              | no                     | yes (relaunch the agent)   | no            |
| Archive    | stops first                    | `git worktree move` to `.archived/` graveyard   | `closeWindow`          | yes (Unarchive)            | no            |
| Unarchive  | no                             | `git worktree move` back to active path         | `createWindow`         | yes (Archive again)        | no            |
| Delete     | stops first                    | `git worktree remove` + rmdir                   | `closeWindow`          | **no**                     | yes           |
| New        | spawns                         | creates                                         | `createWindow`         | n/a                        | no            |

## Focus model and key surface

- Default focus: the filter input.
- `Tab` cycles `filter → list → Dive → Stop → Archive → Delete`
  (skipping `Stop` for archived rows, swapping `Archive` →
  `Unarchive`).
- `↑` / `↓` on the focused filter input forwards to the list
  (smart-key tweak so the user can both type and navigate
  without leaving the filter).
- `Enter` activates whichever element has focus. On a focused
  list row, `Enter` dives. On a focused button, `Enter` fires
  the button's action.
- `Esc` closes the dialog (and cancels confirmation when one
  is open).
- `Alt+S` / `Alt+A` / `Alt+D` / `Alt+N` are chord shortcuts.
  See [Keybinding integration](#keybinding-integration) for
  how they're registered and rendered cross-platform.

## Infrastructure work

### Widget renderer: `row()` of multi-line children

The two-pane wireframe (sessions list next to preview pane)
is the natural composition `row(col(…), col(…))` — or
equivalently `row(labeledSection(…), labeledSection(…))`,
since `labeledSection` is multi-line by construction. The
current widget renderer doesn't realise that shape: `row()`'s
inline-collapse path only operates on single-line children;
when it sees multi-line children it flushes each as a block
**vertically**, so the panes stack instead of sitting
side-by-side.

Phase 1 shipped with a vertically-stacked layout to defer this
work; the rest of the design assumes the proper two-pane shape.
The fix is to extend `row()`'s second pass to zip multi-line
children per line:

1. Walk children, separate into inline pieces (single-line) and
   block pieces (multi-line).
2. Inline pieces collapse into a single line as today.
3. Block pieces, when there are ≥1 in the row, get
   horizontally zipped:
   - allocate each block a column width — equal split of the
     row's `panel_width` by default, with an optional explicit
     weight (a future `widthPct` field on `Col` / `Row` /
     `LabeledSection` if uneven splits become useful);
   - for each row-index up to `max(height(block_i))`, build
     a merged line by concatenating `block_left[i].text`
     padded to its column width + `sep` + `block_right[i].text`;
   - shift each non-first block's inline overlays right by
     the cumulative byte width of the blocks to its left
     (plus any separator bytes).
4. The merged lines then flow into the row's output the same
   way the inline-collapsed line does today, so callers like
   `col` see one block per `row()` rather than per child.

Heights don't have to match — short blocks are padded with
spaces on the missing rows; the column they were given stays
visually open. Overlays attached to those phantom rows aren't
needed because the renderer is generating fresh blank lines.

The change is local to `render.rs`'s Row arm. No new widget
kind, no new spec field for Phase 1's two-pane; widths are
implicit-equal-split. A widget-level `widthPct` parameter can
arrive later if a phase needs an explicit ratio (e.g. a 40/60
split for the preview).

### Process-group signal API

Stop and the stop-leg of Archive / Delete need to terminate
**every** process the session has spawned, including children
the agent forks itself. The terminal layer already runs each
session's command under a fresh pty, which gives us a
session-leader process with its own process-group id (pgid).

New host-side API surface:

```ts
// Plugin API (TypeScript binding via ts-rs).
//
// Sends `signal` to the *process group* led by the terminal's
// pty session leader. Defaults to a graceful escalation:
// SIGTERM, wait `gracePeriodMs`, then SIGKILL anything still
// alive.
editor.signalTerminal(
  terminalId: number,
  options?: {
    signal?: "SIGTERM" | "SIGKILL" | "SIGINT",
    gracePeriodMs?: number,
  },
): Promise<{ stopped: boolean }>;
```

Rust-side implementation lives in `services/terminal/manager.rs`
next to the existing `closeTerminal` path. It walks the pgid
via `kill(-pgid, signal)` on Unix; on Windows it walks the
job object that `portable_pty` already attaches the child to.

### Archive: worktree move + local manifest

`git worktree move` keeps git's internal bookkeeping
consistent with the on-disk move — the worktree still appears
in `git worktree list`, just under the new path, which is
fine because it's still a valid worktree the user could
inspect or fall back to manually.

Layout:

```
<XDG data dir>/orchestrator/<repo-slug>/
├── session-1/                ← active
├── session-2/                ← active
└── .archived/
    ├── session-3/            ← archived
    └── session-4/            ← archived
```

`<repo-slug>` is the slugified repository toplevel path
(`/home/noam/repos/fresh` → `home_noam_repos_fresh`), matching
what the new-session form already produces.

A local manifest at
`<XDG data dir>/orchestrator/<repo-slug>/archived.json` records
the archived sessions so the orchestrator plugin can show them
in the "Show archived" view without scanning the filesystem:

```json
{
  "version": 1,
  "sessions": [
    {
      "label": "session-3",
      "root": "<XDG>/orchestrator/<repo-slug>/.archived/session-3",
      "branch": "session-3",
      "archived_at": "2026-05-13T11:00:00Z",
      "last_state": "ready"
    }
  ]
}
```

"Show archived" is a toggle in the filter row (default off).
When on, archived rows are interleaved with active rows in
the list, rendered with a dim foreground and `ARCH` state
badge.

### Cross-machine recovery via a git branch

The local manifest is the source of truth on one machine. To
recover sessions on another machine the user pushes a special
git branch:

```
refs/heads/<user>/fresh-sessions
```

`<user>` is derived in this order:

1. `$FRESH_SESSIONS_USER` environment variable, if set.
2. The local-part of `git config user.email`
   (`noam@example.com` → `noam`).
3. The username from `gh auth status` when the `gh` CLI is
   configured.
4. `$USER` as a last resort.

The branch is an *orphan-ish* branch carrying only a single
file at its root, `sessions.json`:

```json
{
  "version": 1,
  "machine_id": "chunky.lan",
  "updated_at": "2026-05-13T11:00:00Z",
  "active": [
    {
      "label": "session-2",
      "branch": "session-2",
      "base_ref": "origin/master",
      "created_at": "2026-05-13T09:00:00Z"
    }
  ],
  "archived": [
    {
      "label": "session-3",
      "branch": "session-3",
      "base_ref": "origin/master",
      "archived_at": "2026-05-13T10:00:00Z"
    }
  ]
}
```

Sync behaviour:

- **Push** *(asynchronous, never blocks)*: any local lifecycle
  action (new session, archive, unarchive, delete) commits to
  the sessions branch and fires-and-forgets a
  `git push origin <user>/fresh-sessions`. The user-visible
  action returns immediately — the push runs in the
  background. Failures are non-fatal: the local manifest has
  already been updated, and the next successful push
  reconciles. A small *unobtrusive* indicator surfaces failure
  state — a `⤒` glyph (or similar) appended to the dialog's
  footer when there is unsynced state, plus a one-line
  hover/status-bar message naming the last error. The
  indicator clears as soon as a subsequent push succeeds.
  Users who care can run an explicit "Orchestrator: Sync Now"
  command from the palette to retry on demand.
- **Pull on open**: when `Orchestrator: Open` first loads in a
  fresh editor process, it tries
  `git fetch origin <user>/fresh-sessions` and merges any
  entries it doesn't already know about. Sessions whose
  `branch` is missing locally are shown as "remote" rows that
  resolve to "Dive" by first running `git fetch` for that
  branch and creating the worktree locally.
- **Merge strategy**: per-session `created_at` /
  `archived_at` timestamps decide which side wins on
  conflict. Two machines archiving the same session is
  idempotent.
- **Privacy**: the branch lives under
  `refs/heads/<user>/fresh-sessions` so it doesn't pollute
  the default `git branch` output (Git already hides
  namespaced refs in many UIs). Users who want full opt-out
  can set `fresh.orchestrator.sync = false` in their config.

This feature builds on top of the local manifest and ships
in a later phase (see [Implementation phases](#implementation-phases)).

### Keybinding integration

Shortcuts go through the existing keybinding pipeline rather
than being hardcoded in the plugin. The plugin registers
chord defaults under a `orchestrator-open` plugin mode:

```rust
keybindings.load_plugin_chord_default(
    KeyContext::Mode("orchestrator-open".into()),
    vec![(KeyCode::Char('s'), KeyModifiers::ALT)],
    Action::PluginCommand("orchestrator_stop".into()),
);
// …same for 'a' (archive), 'd' (delete), 'n' (new)
```

What this buys us:

- **User override**: any of these chords can be rebound in
  `~/.config/fresh/keybindings.json` — the resolution path
  in `KeyBindings::resolve_chord` checks user settings before
  the plugin defaults, identical to how built-in actions
  behave.
- **Cross-platform display**: footer hints and the
  Keybinding Editor render the chord through
  `format_keybinding`, which produces `Alt+D` on Linux /
  Windows and `⌥D` on macOS without the plugin caring.
- **Mode scoping**: the chord only fires while the
  `orchestrator-open` mode is active (i.e. the dialog is
  open), so `Alt+D` doesn't shadow anything global.

This requires one small host change: the floating-widget-panel
keystroke dispatcher (`dispatch_floating_widget_key` in
`app/input.rs`) currently swallows Ctrl/Alt chords. The new
behaviour is to *first* attempt a mode-chord resolution
against the active editor mode, then fall back to the
existing swallow-don't-leak rule.

## Implementation phases

### Phase 1 — Widget-based picker shell *(shipped)*

- Build the layout: header, filter input, list + preview
  panes, focus model, default keys (Up/Down/Tab/Enter/Esc).
- Plugin-side fuzzy filter over `orchestratorSessions` (small
  ranker, substring + prefix bonus). No external action
  surface yet — Dive is the only action.
- Smart-key forwarding: Up/Down/Enter on a focused single-line
  Text route to the panel's first List/Tree, so the filter
  input stays focused for typing while arrows navigate.
- Shipped with the panes stacked vertically because the
  widget renderer's `row()` doesn't yet do horizontal zip
  for multi-line children (see [Widget renderer:
  `row()` of multi-line children](#widget-renderer-row-of-multi-line-children)).
- Replaces today's `startPrompt`-based picker.

### Phase 1b — `row()` of multi-line children

- Extend `render.rs`'s Row arm so block children are zipped
  per line instead of flushed vertically.
- Flip the picker spec from a `col(filter, list, preview)`
  fallback back to the wireframed
  `row(labeledSection(list), labeledSection(preview))`.
- No new widget kind or spec field; equal-width split is the
  default. A `widthPct` field can arrive when a phase
  demands an explicit ratio.

### Phase 2 — Process-group signal API

- `editor.signalTerminal(terminalId, options)` host API in
  `services/terminal/manager.rs`.
- Unix: `kill(-pgid, signal)`. Windows: walk the
  `portable_pty` job object.
- TS-rs binding + plugin wrapper.

### Phase 3 — Stop action

- Wire `Stop` button + `Alt+S` shortcut to the new signal
  API.
- Preview pane renders pgid + pid list once the API is
  available.

### Phase 4 — Archive / Unarchive

- Local manifest at
  `<XDG>/orchestrator/<repo-slug>/archived.json`.
- `git worktree move` to / from the `.archived/` graveyard.
- "Show archived" toggle in the filter section; archived rows
  rendered with dim fg + `ARCH` badge.

### Phase 5 — Delete

- Confirmation panel in the preview pane.
- `git worktree remove` + rmdir for active sessions; manifest
  cleanup for archived sessions.

### Phase 6 — Cross-machine recovery

- Derive `<user>` from env / git config / gh / `$USER`.
- Commit + best-effort push of `<user>/fresh-sessions` branch
  on every lifecycle action.
- Fetch + merge on dialog open. Render remote-only sessions
  as a distinct row category, resolvable via Dive (which
  performs the worktree fetch / create lazily).
- `fresh.orchestrator.sync = false` config opt-out.

## Project scoping (cross-project confusion)

> **Status**: Design addition, May 2026
> **Driving bug**: After the v2 persistence change (one global
> `<data>/orchestrator/windows.json` instead of per-cwd files),
> launching Fresh in project B surfaces every session from every
> project the user ever created. The picker lists them in one
> flat, unlabeled list, and `pick_active_window_for_cwd` can boot
> straight into yesterday's session when its `project_path`
> matches today's cwd. Users read this as "it's combining
> yesterday's directories/tabs into today's project."

### Principles

1. **Scope by default, global on demand.** Open scoped to the
   current project. Cross-project is an explicit, always-visible
   gesture (`⌥P` / a Scope toggle), never the landing view.
2. **All sessions stay reachable.** The orchestrator is still the
   one place to reach every session everywhere — scoping changes
   what's *foregrounded*, not what's *reachable*. The scoped view
   always shows a `── N in other projects · ⌥P ──` affordance, and
   the filter searches globally even while scoped.
3. **Never silently inherit a session across projects.** Booting
   in project X lands on a clean base window for X. Do not
   auto-activate a persisted session just because its
   `project_path` matches the cwd; if we restore, make it a
   visible, dismissible "Resume last session?" affordance.
4. **Scope is legible.** Current project shows in the dialog
   title and the editor status bar. The list is grouped by
   project (never flat), with the current project marked. A dive
   into another project is labeled (`Dive (switches project)`).
5. **Consistent boundaries.** Workspaces (tabs/explorer) are
   per-cwd but orchestrator windows are global — that mismatch is
   what makes state feel "combined." Apply one boundary (project
   root) uniformly to sessions, per-window plugin state, and
   workspace.
6. **Migrations are visible and reversible.** When the storage
   model changes (per-cwd → global), show a one-time notice
   rather than silently folding everyone's history together on
   first launch.

### Wireframe — current (flat, unscoped)

```
╭─ ORCHESTRATOR :: Sessions ──────────────────────────────────────╮
│ ╭─ Sessions (4) ─────────╮ ╭─ [3] blog-redesign ──────────────╮ │
│ │ [ + New Session  Alt+N]│ │ [ Visit ] [Details][Stop][Arch…] │ │
│ │ [type to filter…     ] │ │                                  │ │
│ │ [1] RUN  fresh BASE ⇄  │ │        (preview of session)      │ │
│ │ [2] RUN  feature-login │ │                                  │ │
│ │ [3] ACT  blog-redesign │ │                                  │ │
│ │ [4] RUN  hotfix-2031 ⇄ │ │                                  │ │
│ ╰────────────────────────╯ ╰──────────────────────────────────╯ │
│  ↑↓ nav · Enter dive · Tab focus · Esc close                    │
╰──────────────────────────────────────────────────────────────────╯
```

Launched in projB, but `fresh` / `feature-login` (projA) are
mixed in with no project label and the count `(4)` is the global
total.

### Wireframe — new, default (scoped to current project)

```
╭─ ORCHESTRATOR :: Sessions ─ projB ──────────────────────────────╮
│ ╭ [type to filter…      ]  Scope: ‹ current › ⌥P  searches all ╮│
│ ╰───────────────────────────────────────────────────────────────╯│
│ ╭─ projB · this project (2) ─╮ ╭─ [3] blog-redesign ──────────╮ │
│ │ [ + New Session  ⌥N ]      │ │ Project: /…/projB            │ │
│ │ ▸ [3] ACT  blog-redesign   │ │ State:   ACT       Age: 3m   │ │
│ │   [4] RUN  hotfix-2031     │ │ ──────────────────────────── │ │
│ │ ── 2 in other projects ─── │ │ ▸ Dive · Stop⌥S · Arch⌥A …   │ │
│ │      press ⌥P to show      │ │                              │ │
│ ╰────────────────────────────╯ ╰──────────────────────────────╯ │
│  ↑↓ nav · Enter dive · ⌥P all projects · ⌥N new · Esc close     │
╰──────────────────────────────────────────────────────────────────╯
```

### Wireframe — new, all-projects view (`⌥P`, grouped)

```
╭─ ORCHESTRATOR :: Sessions ─ all projects ───────────────────────╮
│ ╭ [type to filter…      ]  Scope: ‹ all ›    ⌥P current only   ╮│
│ ╰───────────────────────────────────────────────────────────────╯│
│ ╭─ Sessions (4) ─────────────╮ ╭─ [1] fresh ──────────────────╮ │
│ │ [ + New Session  ⌥N ]      │ │ Project: /…/projA            │ │
│ │ ▾ projB  · current         │ │ ⚠ different project than this │ │
│ │   [3] ACT  blog-redesign   │ │   window                     │ │
│ │   [4] RUN  hotfix-2031     │ │ State:   RUN (BASE)  Age: 1d  │ │
│ │ ▾ projA                    │ │ ──────────────────────────── │ │
│ │   [1] RUN  fresh    BASE   │ │ ▸ Dive (switches project) ·… │ │
│ │   [2] RUN  feature-login   │ │                              │ │
│ ╰────────────────────────────╯ ╰──────────────────────────────╯ │
│  ↑↓ nav · Enter dive · ⌥P current only · ⌥N new · Esc close     │
╰──────────────────────────────────────────────────────────────────╯
```

### Interaction notes

- **Scope toggle** (`⌥P`) flips current ↔ all and is rendered in
  the filter row through `format_keybinding` (chord registered in
  the `orchestrator-open` mode, same pipeline as Stop/Archive/
  Delete). Persist the last-used scope per editor session.
- **Filter is always global**: typing in the scoped view still
  matches sessions in other projects and auto-reveals them under
  their project header (so search never hides a session the user
  is clearly looking for).
- **Grouping** reuses the existing list widget: project headers
  are non-selectable separator rows; the current project's group
  sorts first and is labeled `· current`.
- **Boot behavior** (separate from the dialog): replace the
  cross-project auto-activate in `pick_active_window_for_cwd` with
  "clean base window for the cwd + optional Resume affordance."
  This is the single highest-impact fix and is independent of the
  dialog redesign.

## Open questions

- **Project scope default**: per-project default scope is the
  recommendation, but should the very first open in a brand-new
  project (zero sessions) auto-expand to all-projects so the list
  isn't empty, or show an empty state with a clear `⌥P` hint?
  Leaning toward the empty state — an empty scoped list with a
  visible toggle teaches the model better than silently widening.
- **Stop with no live processes**: silent no-op or status-bar
  feedback? Leaning toward status-bar — surprise-no-op feels
  broken.
- **Diving into an archived session**: implicit unarchive
  (today's plan), or refuse and require the user to
  explicitly unarchive first? Implicit is more ergonomic but
  hides the worktree move under what reads as a navigation
  action.
- **Diving into an archived session**: implicit unarchive
  (today's plan), or refuse and require the user to
  explicitly unarchive first? Implicit is more ergonomic but
  hides the worktree move under what reads as a navigation
  action.
- **Push frequency**: every lifecycle action vs. batched on
  dialog close vs. an explicit "Sync" button. Every-action
  is simplest; the push is async and non-fatal so latency is
  hidden, but it does add network traffic.
- **`<user>` collision**: two contributors with the same
  `git config user.email` local-part would collide on the
  branch namespace. Possibly require the gh username or a
  config when ambiguous.
