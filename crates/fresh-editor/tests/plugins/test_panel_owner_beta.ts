/// <reference path="../../plugins/lib/fresh.d.ts" />
const editor = getEditor();

/**
 * The other half of the panel-ownership e2e pair (see
 * `test_panel_owner_alpha.ts` and `widget_panel_ownership.rs`).
 *
 * Beta mounts a CENTERED modal with the same plugin-local panel id (1)
 * alpha's dock uses, and renders a `BETA=<n>` counter that ticks once
 * per received `activate` event.
 */

const PANEL_ID = 1; // plugin-local; alpha uses the same value on purpose
let activates = 0;
let mounted = false;

// deno-lint-ignore no-explicit-any
function spec(): any {
  return {
    kind: "col",
    children: [
      {
        kind: "raw",
        entries: [{ text: `BETA=${activates}\n`, properties: {} }],
      },
      {
        kind: "button",
        label: "BetaGo",
        focused: false,
        intent: "normal",
        key: "beta-go",
        disabled: false,
      },
    ],
  };
}

function owner_beta_mount(): void {
  mounted = true;
  activates = 0;
  editor.mountFloatingWidget(PANEL_ID, spec(), 50, 40);
}
registerHandler("owner_beta_mount", owner_beta_mount);

editor.on("widget_event", (e) => {
  if (!mounted || e.panel_id !== PANEL_ID) return;
  if (e.event_type !== "activate") return;
  activates += 1;
  editor.updateFloatingWidget(PANEL_ID, spec());
});

editor.registerCommand(
  "OwnerBeta: Mount",
  "Mount beta's centered panel (local id 1)",
  "owner_beta_mount",
  null,
);
