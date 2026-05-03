//! Additional undo/redo scenarios beyond the existing
//! `undo_redo.rs` set, capturing claims from
//! `tests/e2e/undo_bulk_edit_after_save.rs` and
//! `tests/e2e/undo_redo_marker_roundtrip.rs`.

use crate::common::scenario::buffer_scenario::{
    assert_buffer_scenario, BufferScenario, CursorExpect,
};
use crate::common::scenario::trace_scenario::{assert_trace_scenario, TraceScenario};
use fresh::test_api::Action;

#[test]
fn migrated_three_inserts_undo_three_times_restores_initial() {
    assert_trace_scenario(TraceScenario {
        description: "3 inserts + 3 Undos restore initial buffer".into(),
        initial_text: "stable".into(),
        actions: vec![
            Action::MoveDocumentEnd,
            Action::InsertChar('a'),
            Action::InsertChar('b'),
            Action::InsertChar('c'),
        ],
        expected_text: "stableabc".into(),
        undo_count: 3,
    });
}

#[test]
fn migrated_undo_after_select_replace_restores_pre_replace_text() {
    // SelectAll + InsertChar replaces the buffer with the typed
    // char. Undo should restore the original.
    assert_trace_scenario(TraceScenario {
        description: "SelectAll + insert + Undo restores the buffer".into(),
        initial_text: "original".into(),
        actions: vec![Action::SelectAll, Action::InsertChar('!')],
        expected_text: "!".into(),
        undo_count: 1,
    });
}

#[test]
fn migrated_redo_after_undo_reapplies_change() {
    assert_buffer_scenario(BufferScenario {
        description: "InsertChar then Undo then Redo lands at the post-insert state".into(),
        initial_text: String::new(),
        actions: vec![
            Action::InsertChar('x'),
            Action::Undo,
            Action::Redo,
        ],
        expected_text: "x".into(),
        // Redo replays both the insert *and* the cursor-advance
        // event, so we end at position 1 after one InsertChar('x').
        expected_primary: CursorExpect::at(1),
        ..Default::default()
    });
}
