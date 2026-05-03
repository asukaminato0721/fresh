//! Migrated workspace scenarios — buffer-list and active-buffer
//! claims from `tests/e2e/multi_file_opening.rs` and
//! `tests/e2e/buffer_lifecycle.rs`.

use crate::common::scenario::context::{NamedBuffer, WorkspaceContext};
use crate::common::scenario::input_event::InputEvent;
use crate::common::scenario::observable::WorkspaceState;
use crate::common::scenario::workspace_scenario::{
    assert_workspace_scenario, WorkspaceScenario,
};
use fresh::test_api::Action;

#[test]
fn migrated_one_buffer_yields_count_one() {
    assert_workspace_scenario(WorkspaceScenario {
        description: "one initial buffer ⇒ buffer_count == 1".into(),
        workspace: WorkspaceContext {
            initial_buffers: vec![NamedBuffer {
                filename: "lonely.txt".into(),
                content: "hi".into(),
            }],
            initial_splits: None,
        },
        events: vec![],
        expected: WorkspaceState {
            buffer_count: 1,
            active_buffer_path: None,
            buffer_paths: Vec::new(),
        },
    });
}

#[test]
fn migrated_three_buffers_yield_count_three() {
    assert_workspace_scenario(WorkspaceScenario {
        description: "three initial buffers ⇒ buffer_count == 3".into(),
        workspace: WorkspaceContext {
            initial_buffers: vec![
                NamedBuffer { filename: "a.txt".into(), content: "alpha".into() },
                NamedBuffer { filename: "b.txt".into(), content: "bravo".into() },
                NamedBuffer { filename: "c.txt".into(), content: "charlie".into() },
            ],
            initial_splits: None,
        },
        events: vec![],
        expected: WorkspaceState {
            buffer_count: 3,
            active_buffer_path: None,
            buffer_paths: Vec::new(),
        },
    });
}

#[test]
fn migrated_typing_leaves_buffer_count_unchanged() {
    // Editing doesn't change the workspace topology.
    assert_workspace_scenario(WorkspaceScenario {
        description: "typing inside a buffer doesn't alter buffer_count".into(),
        workspace: WorkspaceContext {
            initial_buffers: vec![NamedBuffer {
                filename: "x.txt".into(),
                content: "hi".into(),
            }],
            initial_splits: None,
        },
        events: vec![
            InputEvent::Action(Action::MoveDocumentEnd),
            InputEvent::Action(Action::InsertChar('!')),
        ],
        expected: WorkspaceState {
            buffer_count: 1,
            active_buffer_path: None,
            buffer_paths: Vec::new(),
        },
    });
}
