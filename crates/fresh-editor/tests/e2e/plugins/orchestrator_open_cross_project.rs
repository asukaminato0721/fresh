//! The Orchestrator Open dialog scopes to the current project by
//! default, and reveals every project on the scope toggle (Alt+P).
//!
//! Sessions are inherently cross-project — each row can have its own
//! `project_path` — but surfacing all of them at once means launching
//! the editor in project B buries the user under project A's history
//! (the orchestration bug this scoping fixes). So the default view
//! lists only the active window's project, with an "N in other
//! projects" affordance, and the scope toggle brings the rest into a
//! single grouped list. Nothing is hidden; it's just not foregrounded.

use crate::common::harness::EditorTestHarness;
use crossterm::event::{KeyCode, KeyModifiers};
use fresh_core::api::PluginCommand;
use fresh_core::WindowId;
use serde_json::Value;
use std::path::Path;

const WIDTH: u16 = 160;
const HEIGHT: u16 = 40;

const LABEL_B: &str = "zebra-beta-xr";

fn run_palette(harness: &mut EditorTestHarness, command_name: &str) {
    harness
        .send_key(KeyCode::Char('p'), KeyModifiers::CONTROL)
        .unwrap();
    harness.render().unwrap();
    harness.type_text(command_name).unwrap();
    harness.render().unwrap();
    harness
        .send_key(KeyCode::Enter, KeyModifiers::NONE)
        .unwrap();
    harness.render().unwrap();
}

fn set_orch_project_path(harness: &mut EditorTestHarness, project_path: &Path) {
    harness
        .editor_mut()
        .handle_plugin_command(PluginCommand::SetWindowState {
            plugin_name: "orchestrator".into(),
            key: "project_path".into(),
            value: Some(Value::String(project_path.to_string_lossy().into_owned())),
        })
        .unwrap();
}

#[test]
fn open_dialog_scopes_to_current_project_then_reveals_all() {
    let mut harness = EditorTestHarness::with_temp_project(WIDTH, HEIGHT).unwrap();

    // Project A: the harness's temp project root, owned by the base
    // window (id 1, active at boot).
    let proj_a = harness.project_dir().unwrap().canonicalize().unwrap();
    set_orch_project_path(&mut harness, &proj_a);

    // Project B: a separate tempdir, owned by a second window we
    // create explicitly. Per-session plugin state always writes to
    // the *active* window, so we set B active before tagging.
    let proj_b_dir = tempfile::TempDir::new().unwrap();
    let proj_b = proj_b_dir.path().canonicalize().unwrap();
    let win_b = harness
        .editor_mut()
        .create_window_at(proj_b.clone(), LABEL_B.into());
    harness.editor_mut().set_active_window(win_b);
    set_orch_project_path(&mut harness, &proj_b);

    // Active window stays in Project B — the dialog should default to
    // B's sessions only.
    harness.render().unwrap();

    run_palette(&mut harness, "Orchestrator: Open");
    harness
        .wait_until(|h| h.screen_to_string().contains("this project"))
        .expect("Orchestrator Open dialog should appear scoped to the current project");

    let screen = harness.screen_to_string();
    assert!(
        screen.contains(LABEL_B),
        "Project B's session must be listed — it's the current project.\nScreen:\n{}",
        screen,
    );
    assert!(
        screen.contains("this project"),
        "Scoped section must be captioned as the current project.\nScreen:\n{}",
        screen,
    );
    assert!(
        screen.contains("in other projects"),
        "Scoped view must advertise the cross-project sessions it isn't \
         foregrounding so nothing feels hidden.\nScreen:\n{}",
        screen,
    );

    // Toggle scope (Alt+P) → every session, across every project.
    harness
        .send_key(KeyCode::Char('p'), KeyModifiers::ALT)
        .unwrap();
    harness.render().unwrap();
    harness
        .wait_until(|h| h.screen_to_string().contains("all projects"))
        .expect("scope toggle should switch the dialog to the all-projects view");

    let screen = harness.screen_to_string();
    assert!(
        screen.contains("Sessions (2)"),
        "All-projects view must count every session.\nScreen:\n{}",
        screen,
    );
    assert!(
        screen.contains(LABEL_B),
        "Project B's session must still be listed in the all-projects view.\nScreen:\n{}",
        screen,
    );
}
