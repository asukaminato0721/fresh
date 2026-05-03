//! Migrated from `tests/e2e/block_selection.rs` and
//! `tests/e2e/shift_backspace.rs` — multi-cursor / block-select
//! claims expressed as data.

use crate::common::scenario::buffer_scenario::{
    assert_buffer_scenario, BufferScenario, CursorExpect,
};
use fresh::test_api::Action;

#[test]
fn migrated_block_select_down_creates_secondary_cursor() {
    // BlockSelectDown on a 2-line buffer creates a secondary
    // cursor on line 2 column 0.
    assert_buffer_scenario(BufferScenario {
        description: "BlockSelectDown adds a cursor on the next line".into(),
        initial_text: "alpha\nbravo".into(),
        actions: vec![Action::BlockSelectDown],
        expected_text: "alpha\nbravo".into(),
        expected_primary: CursorExpect::range(0, 6),
        expected_extra_cursors: vec![],
        ..Default::default()
    });
}

#[test]
fn migrated_block_select_then_type_inserts_at_each_cursor() {
    // After BlockSelectDown then InsertChar, both cursors gain
    // the typed character.
    assert_buffer_scenario(BufferScenario {
        description: "type after BlockSelectDown distributes across cursors".into(),
        initial_text: "alpha\nbravo".into(),
        actions: vec![
            Action::AddCursorBelow,
            Action::InsertChar('X'),
        ],
        expected_text: "Xalpha\nXbravo".into(),
        expected_primary: CursorExpect::at(8),
        expected_extra_cursors: vec![CursorExpect::at(1)],
        ..Default::default()
    });
}

#[test]
fn migrated_shift_backspace_deletes_one_char() {
    // Shift+Backspace == DeleteBackward in our action alphabet
    // (the modifier doesn't change semantic action).
    assert_buffer_scenario(BufferScenario {
        description: "DeleteBackward (the action shift+backspace binds to) removes one char".into(),
        initial_text: "abc".into(),
        actions: vec![Action::MoveDocumentEnd, Action::DeleteBackward],
        expected_text: "ab".into(),
        expected_primary: CursorExpect::at(2),
        ..Default::default()
    });
}
