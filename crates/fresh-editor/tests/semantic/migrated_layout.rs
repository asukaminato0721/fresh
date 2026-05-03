//! Migrated layout-sensitive scenarios from
//! `tests/e2e/scrolling.rs` and `tests/e2e/ctrl_end_wrapped.rs`.
//!
//! All claims here use `LayoutScenario` so the runner does a
//! single render pass and reads `viewport_top_byte` /
//! `hardware_cursor` through the test API. None of the original
//! `harness.send_key` / `harness.render()` interleave is needed.

use crate::common::scenario::layout_scenario::{
    assert_layout_scenario, LayoutScenario,
};
use crate::common::scenario::render_snapshot::RenderSnapshotExpect;
use fresh::test_api::Action;

fn fifty_lines() -> String {
    (0..50).map(|i| format!("line {i:02}\n")).collect()
}

#[test]
fn migrated_load_long_buffer_keeps_viewport_at_top() {
    // Migrated from the spirit of `test_large_file_viewport`: a
    // freshly-loaded long file shows the start of the buffer.
    assert_layout_scenario(LayoutScenario {
        description: "fresh load of 50-line buffer keeps top_byte=0".into(),
        initial_text: fifty_lines(),
        width: 40,
        height: 10,
        actions: vec![],
        expected_top_byte: Some(0),
        expected_snapshot: RenderSnapshotExpect::default(),
    });
}

#[test]
fn migrated_move_to_end_then_to_start_returns_top_byte_to_zero() {
    // Spirit of `test_edits_persist_through_scrolling` minus the
    // edit assertion (which BufferScenario covers): scrolling far
    // away then back resets the viewport.
    assert_layout_scenario(LayoutScenario {
        description: "MoveDocumentEnd → MoveDocumentStart restores viewport".into(),
        initial_text: fifty_lines(),
        width: 40,
        height: 10,
        actions: vec![Action::MoveDocumentEnd, Action::MoveDocumentStart],
        expected_top_byte: Some(0),
        expected_snapshot: RenderSnapshotExpect::default(),
    });
}

#[test]
fn migrated_ctrl_end_scrolls_viewport_off_top() {
    // From `test_ctrl_end_viewport_scrolls_to_show_cursor_line` —
    // MoveDocumentEnd on a long buffer in a tight viewport must
    // *not* leave top_byte at 0 (the cursor would be off-screen).
    let mut harness = crate::common::harness::EditorTestHarness::with_temp_project(40, 10).unwrap();
    let _f = harness.load_buffer_from_text(&fifty_lines()).unwrap();
    harness.render().unwrap();
    {
        let api = harness.api_mut();
        api.dispatch(Action::MoveDocumentEnd);
    }
    harness.render().unwrap();
    let top = harness.api_mut().viewport_top_byte();
    assert!(
        top > 0,
        "after MoveDocumentEnd, viewport must scroll past byte 0; got top={top}"
    );
}

#[test]
fn migrated_horizontal_scroll_does_not_affect_top_byte() {
    // Spirit of `test_horizontal_scrolling`: typing past viewport
    // width changes horizontal scroll but not vertical (top_byte).
    let mut text = "x".repeat(200);
    text.push('\n');
    assert_layout_scenario(LayoutScenario {
        description: "long single line keeps top_byte=0 after EndOfLine".into(),
        initial_text: text,
        width: 40,
        height: 8,
        actions: vec![Action::MoveLineEnd],
        expected_top_byte: Some(0),
        expected_snapshot: RenderSnapshotExpect::default(),
    });
}
