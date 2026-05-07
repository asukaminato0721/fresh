# Conductor & Sessions Design

> **Status**: Design Document
> **Date**: May 2026
> **Branch**: `claude/plan-conductor-architecture-6YsJt`
> **Driving feature**: "Conductor" multi-agent orchestration UI (PRD external).
> **Core change required**: first-class `Session` abstraction in the editor.

## Motivation

The "Conductor" feature lets a developer run multiple AI coding agents
(`aider`, `claude -p`, `opencode`, …) in parallel, each in its own git
worktree, and switch between them from a single Fresh process. The PRD
calls for two modes:

1. A full-screen **Control Room** that lists every active agent, its
   parsed state (running / awaiting input / ready / errored), live
   terminal preview, diff stats, and a file-collision radar across
   worktrees.
2. A standard **Session IDE** (file explorer, LSP, quick-open, splits,
   buffers) scoped to one worktree at a time, that the user "dives" into
   from the Control Room.

The user-facing requirement that drives this design is:

> Switching sessions from Conductor should feel like swapping the
> entire Fresh state. File explorer, LSP, quick-open scope, ignore
> rules, buffer set, splits — all of it retargets atomically. Conductor
> itself stays anchored above the swap, with its session list,
> collision matrix, and agent PTY handles untouched.

Today, Fresh's editor state is built around a single implicit project
root. The cwd is read in dozens of places (`getCwd()` on the plugin
API, file explorer init, LSP root URI, ignore-matcher construction,
quick-open scoping, plugin path resolution). There is no abstraction
that bundles "everything rooted at one project" so that several can
coexist and one can be made active. A Conductor plugin alone cannot
deliver the required UX, because the things that need to retarget
(file explorer, quick-open, LSP set) are core-owned and scoped
implicitly to whatever `getCwd()` returns.

This document specifies the smallest core abstraction that makes the
required UX possible — a first-class `Session` — and the plugin-API
surface a Conductor plugin needs on top of it. It deliberately does
not specify the Conductor plugin itself; that is a follow-up doc once
this design is settled.

## Non-goals

- Multi-process isolation. Crash isolation between worktrees is not a
  requirement (`§ Trade-off discussion`). One Fresh process, one
  plugin runtime, one editor instance.
- Remote / SSH / devcontainer worktrees. The authority model
  (`AUTHORITY_DESIGN.md`) is orthogonal; sessions and authorities
  compose, but this doc only specifies sessions on the local
  authority. Remote sessions are a follow-up.
- Replacing the existing `panelId` / `utility_dock` machinery. This
  design composes on top of it (`§ Control Room placement`).
- Hot-reload of the Conductor plugin itself. Standard plugin reload
  semantics apply.

## Background: the primitives we already have

### Project root is implicit and editor-wide

Fresh has no `project` or `workspace` struct. The cwd of the Fresh
process is the project root, surfaced to plugins via
`editor.getCwd()` and read directly in many places:

- File explorer (`crates/fresh-editor/src/app/file_explorer.rs`)
  walks from cwd.
- Quick-open / file finder
  (`crates/fresh-editor/src/input/quick_open/providers.rs`) is scoped
  to cwd.
- Ignore rules (`crates/fresh-editor/src/view/file_tree/ignore.rs`)
  load `.gitignore` from cwd upward.
- LSP root URIs derive from cwd or per-buffer file paths
  (`crates/fresh-editor/src/services/lsp/manager.rs`).
- Plugin runtime exposes cwd as a JS string read on demand.

There is no central registry; each subsystem reads cwd when it needs
it. Changing cwd at runtime today would race against any of these
readers and would not retroactively rebuild file-tree or LSP state.

### Buffers and splits live on the Editor struct

`crates/fresh-editor/src/app/mod.rs` (the `Editor` struct) owns:

- `buffers: HashMap<BufferId, Buffer>` — every open buffer.
- `split_manager: SplitManager` — the pane tree.
- `split_view_states: HashMap<SplitId, SplitViewState>` — per-split
  scroll/cursor state.
- `terminal_manager` — every PTY.
- `plugin_manager` — single plugin runtime, single QuickJS instance.
- `file_mod_times: HashMap<PathBuf, _>` — polling-based change
  detection.
- `panel_ids: HashMap<String, BufferId>` — utility-dock occupancy.

None of these are scoped by project root. There is one of each, for
the whole Fresh process.

### Plugins are editor-scoped, not session-scoped

The plugin runtime lives on the Editor (singleton). Plugin state in
JS is whatever the plugin module's top-level scope holds, which
persists for the lifetime of the editor (or until plugin reload). No
plugin state is currently scoped narrower than that. This is
fortunate: it is exactly the property that lets Conductor "live above"
sessions for free, once sessions exist.

### `utility_dock` and virtual buffers

`createVirtualBufferInSplit({ role: "utility_dock", … })` (handled at
`crates/fresh-editor/src/app/plugin_dispatch.rs:2167` onward)
implements a one-leaf-per-role dock for diagnostics, file explorer,
search/replace, finder. Conductor's Control Room will use this same
dock with its own role tag.

`defineMode(name, bindings, …)`
(`crates/fresh-plugin-runtime/src/backend/quickjs_backend.rs:3196`)
binds keys to commands within a named mode that virtual buffers can
opt into via the `mode` field. This is how Conductor binds its own
hotkeys.

### Terminal manager already emits the events we need

`AsyncMessage::TerminalOutput { terminal_id }` and
`AsyncMessage::TerminalExited { terminal_id }` are emitted from
`crates/fresh-editor/src/services/terminal/manager.rs:407,433` and
consumed internally at
`crates/fresh-editor/src/app/async_dispatch.rs:427,453`. They are not
exposed to plugins today. Surfacing them is one of the changes this
design requires (§ Plugin API surface).

### Daemon / IPC

Fresh's client/server (`crates/fresh-editor/src/server/`) is already
robust and used for persistence-across-disconnect. This design does
**not** introduce a second server or a new RPC channel. The daemon
hosts one Editor with N sessions; the client renders whichever
session is active plus the editor-level chrome.

## The `Session` abstraction

A `Session` owns the per-project-root state that today is implicit on
the Editor.

```rust
pub struct Session {
    pub id: SessionId,
    pub label: String,                   // user-visible
    pub root: PathBuf,                   // canonical absolute path

    // What used to be "the editor's"
    pub buffers: HashSet<BufferId>,      // ids; storage stays Editor-global
    pub split_layout: SplitTree,
    pub view_states: HashMap<SplitId, SplitViewState>,
    pub active_split: SplitId,
    pub panel_ids: HashMap<String, BufferId>,  // utility-dock occupancy
    pub file_tree: FileTreeState,
    pub ignore_matcher: IgnoreMatcher,
    pub lsp_clients: LspClientSet,       // keyed by language, rooted at `root`
    pub watch_handles: Vec<WatchHandle>,
    pub plugin_state: HashMap<PluginId, JsValue>,  // session-scoped, opt-in

    // Persistence
    pub layout_snapshot: Option<LayoutSnapshot>,   // for save/restore
    pub created_at: SystemTime,
}

pub struct Editor {
    sessions: HashMap<SessionId, Session>,
    active_session: SessionId,

    // Editor-global (one per process):
    buffers: HashMap<BufferId, Buffer>,            // owned here; sessions hold ids
    terminal_manager: TerminalManager,             // PTYs survive session swaps
    plugin_manager: PluginManager,                 // one runtime
    plugin_global_state: HashMap<PluginId, JsValue>,
    theme: Theme,
    config: Arc<Config>,
    keybindings: KeyBindings,
    // ...
}
```

### Why buffer storage stays Editor-global

Buffers are owned by `Editor.buffers`; sessions hold a `HashSet<BufferId>` of
which buffers belong to them. Three reasons:

1. The same physical file can in principle be open in two sessions
   (e.g. a shared header outside both worktrees). De-duplicating at
   the buffer-storage level keeps undo, mtime tracking, and LSP
   text-sync coherent.
2. Conductor's terminal buffers (one per agent) need to be
   addressable from the Control Room, which lives editor-globally. If
   buffers were owned by sessions, the Control Room would either need
   to peek into other sessions' storage or duplicate.
3. Migration cost: keeping `Editor.buffers` flat means every existing
   `BufferId` lookup keeps working unchanged. Only the question
   "which session is this buffer attached to?" is new.

### Active session is a single pointer

`active_session: SessionId` is the only piece of session state read
on every render. Switching is atomic from the renderer's perspective:
update the pointer, redraw. All cached state — file tree expansion,
LSP clients, watchers — already lives on the (now-active) session
and was kept warm while inactive.

### Session-global vs session-scoped plugin state

Two storage namespaces exposed to plugins:

```ts
// Editor-global (default).
editor.setGlobalState("conductor.sessions", JSON.stringify(state));
editor.getGlobalState("conductor.sessions"): string | null;

// Session-scoped (opt-in).
editor.setSessionState("my-plugin.foo", value);
editor.getSessionState("my-plugin.foo"): unknown;  // current active session
```

Conductor uses **only** the global namespace. Plugins that genuinely
want per-project state (per-language helpers, per-repo lint configs)
opt in to session scope.

The default is global because that's the *current* behavior — plugin
top-level scope persists for the lifetime of the editor — and we do
not want to silently change the meaning of existing plugins' module
state.

## Dive: the atomic swap

`editor.setActiveSession(id)` performs:

1. **Snapshot** the outgoing session's last-active split, scroll
   positions, file-tree expansion, prompt state. Persist to
   `Session.layout_snapshot`.
2. **Update** `Editor.active_session = id`.
3. **Restore** the incoming session's snapshot to the live view
   state.
4. **Emit** `active_session_changed` to plugins.

LSPs, watchers, and plugin global state are never touched. The
inactive session's LSPs continue running; if a tool finishes
indexing while the user is in another session, it is ready
immediately on the next dive.

The renderer reads `editor.active_session()` once at the top of each
frame. There is no per-subsystem "switch" call — the switch is the
pointer write, and every read from then on routes through the
session.

## Lifecycle

| Event | Effect |
|---|---|
| `createSession({ root, label })` | Construct a new `Session`, walk file tree, build ignore matcher, lazily start LSPs on first buffer open. Return `SessionId`. Does not switch active. |
| `setActiveSession(id)` | Atomic swap (above). |
| `closeSession(id)` | Shut down LSPs, drop watchers, free per-session caches. If `id == active_session`, refuse with error (caller must switch first). Buffers attached to this session and not to any other are closed. |
| Editor shutdown | Persist session list (root, label, layout snapshot) to `.fresh/sessions.json`. Terminal PTYs and agent processes are torn down per existing rules. |
| Editor startup | Rehydrate session list. **Inactive sessions are lazy** — LSPs and file watchers do not start until the session is first activated. Only the active session is fully spun up. |

## Control Room placement

The Control Room is a virtual buffer that must render identically
regardless of which session is active. Two options:

- **(A) Editor-global virtual buffer.** A new buffer-attachment kind
  that is not in any `Session.buffers` set; the renderer treats it
  as part of editor chrome. Drawn over the active session's UI.
- **(B) Mirrored across all sessions.** Every session's `panel_ids`
  contains the Control Room buffer, so it stays addressable after
  switches.

(A) is cleaner: one buffer, one panel id, no per-session
bookkeeping. It requires a small new affordance in
`virtual_buffers.rs` — an "editor-global" flag — but the rendering
path already special-cases dock leaves, so this is local.

(B) reuses existing machinery but means every `closeSession` has to
remember not to evict the Control Room. Strictly more error-prone.

This design picks **(A)**.

## Plugin API surface

Additions only. Nothing existing is removed or changed shape.

### Sessions

```ts
type SessionId = number;
type SessionInfo = { id: SessionId; label: string; root: string; createdAt: number };

editor.listSessions(): SessionInfo[];
editor.activeSession(): SessionId;
editor.createSession(opts: { root: string; label: string }): Promise<SessionId>;
editor.setActiveSession(id: SessionId): void;
editor.closeSession(id: SessionId): Promise<void>;

// Events
editor.on("session_created",        handler: string): void;
editor.on("session_closed",         handler: string): void;
editor.on("active_session_changed", handler: string): void;
// payload: { previousId: SessionId | null; activeId: SessionId }
```

### Buffer/terminal scoping

Most buffer APIs gain an optional `sessionId` (defaults to active):

```ts
editor.createTerminal({ sessionId?: SessionId, cwd?: string, ... }): Promise<TerminalResult>;
editor.openFile(path: string, opts?: { sessionId?: SessionId }): Promise<BufferId>;
```

Existing call sites without `sessionId` get the active session, so
existing plugins keep working.

### Terminal output and exit events (the small core change)

```ts
editor.on("terminal_output", handler: string): void;
// payload: { terminalId: number; recentBytes: string; lastLine: string }

editor.on("terminal_exit", handler: string): void;
// payload: { terminalId: number; code: number | null }
```

Wired by firing plugin events at
`crates/fresh-editor/src/app/async_dispatch.rs:427,453`.

### File watching

```ts
editor.watchPath(path: string, opts?: {
  recursive?: boolean;
  sessionId?: SessionId;     // tag for collision matrix; not for scoping
}): Promise<WatchHandle>;

editor.unwatchPath(handle: WatchHandle): void;

editor.on("path_changed", handler: string): void;
// payload: { handle: WatchHandle; path: string; kind: "modify"|"create"|"delete" }
```

Backed by the `notify` crate. The `sessionId` field is informational
(passed back in the event payload) so Conductor can build a
`Map<path, Set<SessionId>>` collision matrix without juggling its
own handle-to-session map.

### Plugin state scopes

```ts
editor.setGlobalState(key: string, value: string): void;
editor.getGlobalState(key: string): string | null;

editor.setSessionState(key: string, value: unknown): void;
editor.getSessionState(key: string): unknown;
```

Persistence is editor-driven: the global namespace is flushed to
`.fresh/state/<plugin>.json`, the session namespace to the session's
record in `.fresh/sessions.json`.

### Diff rendering (optional, v2)

```ts
editor.openDiffView(opts: {
  oldText: string; newText: string;
  title: string;
  mode?: string;
  sessionId?: SessionId;
}): Promise<{ bufferId: BufferId }>;
```

V1 fallback: shell out `git diff --color` into a session terminal.

## Migration sequence

The work is large (`§ Risks`) but factorable. Each step is a
reviewable PR.

### Step 1 — `Session` struct, single forced session

- Introduce `Session` with the fields above.
- Construct exactly one session at startup, rooted at process cwd.
  Active forever.
- Move project-root reads to flow through
  `editor.active_session().root` *without changing behavior*.
- File tree, ignore matcher, LSP clients, watchers move to the
  session. Buffer storage stays on `Editor`; add the
  `Session.buffers: HashSet<BufferId>` membership field.
- Existing plugin APIs (`getCwd`, etc.) read from the active session.
- All existing tests must pass unchanged.

This is the bulk of the refactor and the riskiest step. It is purely
a rearrangement: behavior is identical to today's editor.

### Step 2 — multiple sessions, manual switching

- Add `createSession`, `setActiveSession`, `closeSession`.
- Implement the atomic swap (`§ Dive`).
- Add `editor.listSessions()` / `activeSession()` plugin APIs and
  the `active_session_changed` event.
- A test plugin that calls `createSession` + `setActiveSession`
  exercises the swap end-to-end.

### Step 3 — terminal events to plugins

Smallest core change. Add `terminal_output` / `terminal_exit` events
at the two `async_dispatch.rs` arms.

### Step 4 — `watchPath` plugin API

Wrap `notify` crate. Surface `path_changed` event.

### Step 5 — plugin state scopes

Add `setGlobalState`/`getGlobalState`/`setSessionState`/`getSessionState`
with persistence to `.fresh/`.

### Step 6 — Conductor plugin (separate doc)

A first-party plugin shipping in `crates/fresh-editor/plugins/conductor/`.
Drives the whole feature. Uses only the APIs introduced above.

### Step 7 — diff renderer (optional)

Native vertical diff. Falls back to `git diff` in a terminal until
this lands.

### Step 8 — session persistence across restart

Lazy rehydration: only the active session boots LSPs / watchers on
startup; others spin up on first activation.

## Risks

1. **Step 1 is invasive.** Every place that today reads cwd or
   project-root state must be re-routed through
   `editor.active_session()`. Compiler enforcement is the mitigation:
   move the field off `Editor` and onto `Session` early so the
   compiler errors point at every call site.

2. **LSP teardown on `closeSession`.** Today LSPs mostly key on
   project root, but the manager has assumed-singleton ergonomics in
   places. Audit `services/lsp/manager.rs` before Step 2.

3. **Buffer-to-session attribution edge cases.** A buffer opened
   from a path that lies under no session's root: which session
   owns it? Proposal: editor-global, attached to no session, opens
   in a "scratch" surface. Surfaced as a separate concept so it
   doesn't muddy session semantics.

4. **Plugin reload during a session swap.** If the plugin runtime
   reloads mid-swap, in-flight events are lost. Mitigation: drain
   the plugin event queue before the swap commits.

5. **Lazy LSP startup may surprise users.** First-time activation of
   an inactive session has the usual "rust-analyzer is indexing"
   pause. Document explicitly. A pre-warm hint
   (`editor.prewarmSession(id)`) could be added later if needed.

6. **Cross-session cursor jumps.** "Go to definition" landing in a
   file under a different session's root is undefined under this
   design. Proposal: open the target buffer in the *current* session
   (attaching the buffer id to its `buffers` set) rather than
   switching sessions — the alternative is a surprise dive.

7. **Memory growth with many warm sessions.** N rust-analyzers at
   500MB+ each adds up. This is intrinsic to "warm LSPs across
   sessions" and acceptable per `§ Trade-off discussion`. A future
   `editor.suspendSession(id)` (kill LSPs, keep buffer text) is a
   reasonable escape hatch but not part of v1.

## Trade-off discussion

(Carried over from the design conversation that produced this doc;
recorded here so the rationale is reviewable.)

Three architectures were considered:

- **(A) Plugin-driven workspace switching.** One Fresh process; a
  plugin asks core to mutate `cwd` and rebuild file-tree / LSP /
  ignore in place. Smallest core change but most fragile UX: every
  subsystem rebuild is a separate event the user can see seams in.
- **(B) First-class `Session` in core.** This document. Larger core
  change but the swap is atomic and inactive sessions are warm.
- **(C) Multi-process: one Fresh server per worktree, client
  multiplexes.** Best crash isolation, biggest architectural lift,
  new IPC, two plugin runtimes (or a coordinator). Roughly N×60MB
  fixed-cost-per-server overhead beyond the N×LSP cost that
  dominates either way.

(C) was rejected because crash isolation is not a requirement and
the per-process overhead, while not free, is small relative to LSP
cost. (A) was rejected because "Conductor lives above sessions" is a
load-bearing UX claim that (A) cannot honor — under (A), Conductor
*is* the editor reaching into its own root, and every glitch in the
in-place rebuild is a Conductor glitch. (B) is the architecture
that makes the UX claim true by construction.

## Open questions

1. **Should sessions persist across restarts by default?** Two
   schools: VS Code reopens last workspace; vim opens fresh. Default
   to "rehydrate session list, do not auto-dive into one of them"
   for now; user lands in a scratch session and picks. Configurable.

2. **Maximum sessions.** N=20 worktrees with N rust-analyzers will
   melt a laptop. A soft cap (configurable, default 8?) with a warning
   would be friendly. Out of scope for the core abstraction; can be
   enforced in the Conductor plugin.

3. **Session-aware command palette.** Should the palette show
   commands from all sessions, or just the active one? Default:
   active only, since commands tend to be buffer-scoped.

4. **Cross-session search.** Quick-open today scopes to cwd; under
   sessions, default is active session's root. A "search across all
   sessions" mode is desirable but post-v1.

5. **Authority composition.** A future remote session would carry an
   authority alongside its root. The fields nest cleanly
   (`Session.authority: AuthorityHandle`), but the spawning/teardown
   sequence interacts with `AUTHORITY_DESIGN.md` and is deferred.

## Appendix: a Conductor plugin sketch (illustrative only)

This is *not* a spec — the Conductor plugin gets its own design doc.
Included here only to illustrate that the API surface above is
sufficient.

```ts
const sessions = new Map<SessionId, AgentSession>();
const collisions = new Map<string, Set<SessionId>>();

editor.registerCommand("conductor.new", async () => {
  const branch = await editor.startPrompt("Branch");
  const cmd    = await editor.startPrompt("Agent command");
  const wt     = await git.worktreeAdd(branch);
  const id     = await editor.createSession({ root: wt.path, label: branch });
  const term   = await editor.createTerminal({ sessionId: id, cwd: wt.path });
  editor.sendTerminalInput(term.terminalId, cmd + "\n");
  await editor.watchPath(wt.path, { recursive: true, sessionId: id });
  sessions.set(id, { id, branch, terminal: term, state: "running" });
  rerenderControlRoom();
});

editor.registerCommand("conductor.dive", () => {
  editor.setActiveSession(selectedSessionId);
  // file tree, LSP, quick-open, splits all retarget. Conductor state untouched.
});

editor.on("terminal_output", e => stateMachine.observe(e));
editor.on("terminal_exit",   e => stateMachine.observe(e));
editor.on("path_changed",    e => collisionMatrix.observe(e));
editor.on("active_session_changed", () => rerenderControlRoom());
```

The plugin's `sessions` map and `collisions` map live in the plugin
module's top-level scope, which under this design is editor-global
and is not affected by `setActiveSession`. That is the property the
PRD asks for.
