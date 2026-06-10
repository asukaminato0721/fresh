/// <reference path="../../plugins/lib/fresh.d.ts" />
const editor = getEditor();

/**
 * One half of the panel-ownership e2e pair (see
 * `test_panel_owner_beta.ts` and `widget_panel_ownership.rs`).
 *
 * Both plugins deliberately mount a panel with the SAME plugin-local
 * id (1). The host keys panels by (plugin, id), so the two panels must
 * coexist, and each plugin must receive ONLY its own `widget_event`s —
 * the `e.panel_id === 1` check here cannot tell the two panels apart,
 * so any cross-plugin broadcast would tick both counters.
 *
 * Alpha mounts into the left DOCK slot and renders an `ALPHA=<n>`
 * counter that ticks once per received `activate` event.
 */

const PANEL_ID = 1; // plugin-local; beta uses the same value on purpose
let activates = 0;
let mounted = false;

// deno-lint-ignore no-explicit-any
function spec(): any {
  return {
    kind: "col",
    children: [
      {
        kind: "raw",
        entries: [{ text: `ALPHA=${activates}\n`, properties: {} }],
      },
      {
        kind: "button",
        label: "AlphaGo",
        focused: false,
        intent: "normal",
        key: "alpha-go",
        disabled: false,
      },
    ],
  };
}

function owner_alpha_mount(): void {
  mounted = true;
  activates = 0;
  editor.mountFloatingWidget(PANEL_ID, spec(), 40, 40, true);
}
registerHandler("owner_alpha_mount", owner_alpha_mount);

editor.on("widget_event", (e) => {
  if (!mounted || e.panel_id !== PANEL_ID) return;
  if (e.event_type !== "activate") return;
  activates += 1;
  editor.updateFloatingWidget(PANEL_ID, spec());
});

editor.registerCommand(
  "OwnerAlpha: Mount",
  "Mount alpha's dock panel (local id 1)",
  "owner_alpha_mount",
  null,
);
