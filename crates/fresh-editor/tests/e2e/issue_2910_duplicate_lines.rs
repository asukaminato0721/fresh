use crate::common::harness::EditorTestHarness;
use crossterm::event::{KeyCode, KeyModifiers};

const DUPLICATE_MODIFIERS: KeyModifiers = KeyModifiers::ALT.union(KeyModifiers::SHIFT);

#[test]
fn shift_alt_up_duplicates_the_current_line_above() {
    let mut harness = EditorTestHarness::new(80, 24).unwrap();
    harness.load_buffer_from_text("A\nB\nC").unwrap();

    harness.send_key(KeyCode::Down, KeyModifiers::NONE).unwrap();
    harness.send_key(KeyCode::Up, DUPLICATE_MODIFIERS).unwrap();
    harness.type_text("X").unwrap();

    // Typing lands in the new upper copy, not the original line.
    harness.assert_buffer_content("A\nXB\nB\nC");
}

#[test]
fn shift_alt_down_duplicates_the_current_line_below() {
    let mut harness = EditorTestHarness::new(80, 24).unwrap();
    harness.load_buffer_from_text("A\nB\nC").unwrap();

    harness.send_key(KeyCode::Down, KeyModifiers::NONE).unwrap();
    harness
        .send_key(KeyCode::Down, DUPLICATE_MODIFIERS)
        .unwrap();
    harness.type_text("X").unwrap();

    // Typing lands in the new lower copy, not the original line.
    harness.assert_buffer_content("A\nB\nXB\nC");
}
