//! Migrated from `tests/e2e/movement.rs`.
//!
//! The originals drive `KeyCode::Char/Left/Right/Up/Down/Home/End`
//! through the harness and assert with `harness.cursor_position()`.
//! The scenarios below state the same claims as data: action
//! sequence in, expected text + cursor out.
//!
//! What's gained:
//! - keymap-independent (Alt+U vs Cmd+U vs Vi binding doesn't change
//!   any of these),
//! - render-independent (no `harness.render()` calls),
//! - faster (single-digit ms per scenario),
//! - shrinkable as proptest seeds (the corpus dump emits these).

use crate::common::scenario::buffer_scenario::{
    assert_buffer_scenario, repeat, BufferScenario, CursorExpect,
};
use fresh::test_api::Action;

#[test]
fn migrated_typing_and_cursor_movement_basic() {
    // Cleaned-up version of the first half of
    // `test_typing_and_cursor_movement` — type "Hello", end at
    // cursor 5.
    assert_buffer_scenario(BufferScenario {
        description: "type 'Hello' from empty buffer leaves cursor at 5".into(),
        initial_text: String::new(),
        actions: vec![
            Action::InsertChar('H'),
            Action::InsertChar('e'),
            Action::InsertChar('l'),
            Action::InsertChar('l'),
            Action::InsertChar('o'),
        ],
        expected_text: "Hello".into(),
        expected_primary: CursorExpect::at(5),
        ..Default::default()
    });
}

#[test]
fn migrated_type_then_arrow_left_then_insert_in_middle() {
    assert_buffer_scenario(BufferScenario {
        description: "type 'abcd', MoveLeft 2, insert 'X' produces 'abXcd' with cursor at 3"
            .into(),
        initial_text: String::new(),
        actions: vec![
            Action::InsertChar('a'),
            Action::InsertChar('b'),
            Action::InsertChar('c'),
            Action::InsertChar('d'),
            Action::MoveLeft,
            Action::MoveLeft,
            Action::InsertChar('X'),
        ],
        expected_text: "abXcd".into(),
        expected_primary: CursorExpect::at(3),
        ..Default::default()
    });
}

#[test]
fn migrated_home_end_navigation() {
    // Home jumps to line start; End jumps to line end. On a single
    // line, Home → 0, End → length.
    assert_buffer_scenario(BufferScenario {
        description: "MoveLineStart on 'hello world' parks cursor at 0".into(),
        initial_text: "hello world".into(),
        actions: vec![Action::MoveDocumentEnd, Action::MoveLineStart],
        expected_text: "hello world".into(),
        expected_primary: CursorExpect::at(0),
        ..Default::default()
    });
}

#[test]
fn migrated_multiline_navigation_up_down() {
    // 3-line buffer; from end, MoveUp lands on line 2 end.
    assert_buffer_scenario(BufferScenario {
        description: "MoveUp from end of line 3 jumps to end of line 2".into(),
        initial_text: "Line 1\nLine 2\nLine 3".into(),
        actions: vec![Action::MoveDocumentEnd, Action::MoveUp],
        expected_text: "Line 1\nLine 2\nLine 3".into(),
        // "Line 1\n" (7) + "Line 2" (6) = 13.
        expected_primary: CursorExpect::at(13),
        ..Default::default()
    });
}

#[test]
fn migrated_backspace_deletes_previous_char() {
    assert_buffer_scenario(BufferScenario {
        description: "DeleteBackward at position 5 removes the previous char".into(),
        initial_text: String::new(),
        actions: vec![
            Action::InsertChar('a'),
            Action::InsertChar('b'),
            Action::InsertChar('c'),
            Action::DeleteBackward,
        ],
        expected_text: "ab".into(),
        expected_primary: CursorExpect::at(2),
        ..Default::default()
    });
}

#[test]
fn migrated_movement_across_empty_lines() {
    // Empty lines between content shouldn't trap the cursor.
    assert_buffer_scenario(BufferScenario {
        description: "MoveDown twice from line 1 walks past an empty middle line".into(),
        initial_text: "alpha\n\ncharlie".into(),
        actions: vec![Action::MoveDown, Action::MoveDown],
        expected_text: "alpha\n\ncharlie".into(),
        // alpha\n (6) + \n (1) = 7, which is line-3 col-0.
        expected_primary: CursorExpect::at(7),
        ..Default::default()
    });
}

#[test]
fn migrated_repeated_select_right_grows_selection() {
    // Lift the imperative `for _ in 0..N` selection-extension
    // pattern into one declarative `repeat`.
    let mut actions: Vec<Action> = repeat(Action::SelectRight, 5).collect();
    actions.push(Action::SelectRight);
    assert_buffer_scenario(BufferScenario {
        description: "6 SelectRight steps on 'hello world' yield range 0..6".into(),
        initial_text: "hello world".into(),
        actions,
        expected_text: "hello world".into(),
        expected_primary: CursorExpect::range(0, 6),
        expected_selection_text: Some("hello ".into()),
        ..Default::default()
    });
}

#[test]
fn migrated_select_all_then_delete_clears_buffer() {
    assert_buffer_scenario(BufferScenario {
        description: "SelectAll + DeleteBackward empties the buffer".into(),
        initial_text: "non-empty content".into(),
        actions: vec![Action::SelectAll, Action::DeleteBackward],
        expected_text: String::new(),
        expected_primary: CursorExpect::at(0),
        expected_selection_text: Some(String::new()),
        ..Default::default()
    });
}
