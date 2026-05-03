//! Additional indent/dedent claims, distinct from the existing
//! `indent_dedent.rs`. Captures intents from
//! `tests/e2e/auto_indent.rs` and `tests/e2e/tab_indent_selection.rs`.

use crate::common::scenario::buffer_scenario::{
    assert_buffer_scenario, BehaviorFlags, BufferScenario, CursorExpect,
};
use fresh::test_api::Action;

#[test]
fn migrated_tab_at_line_start_inserts_indent() {
    assert_buffer_scenario(BufferScenario {
        description: "Tab at column 0 inserts the configured indent".into(),
        initial_text: "line".into(),
        actions: vec![Action::InsertTab],
        expected_text: "    line".into(),
        // InsertTab adds 4 spaces and advances the cursor past
        // them, landing at column 4.
        expected_primary: CursorExpect::at(4),
        ..Default::default()
    });
}

#[test]
fn migrated_dedent_removes_one_level_of_indent() {
    assert_buffer_scenario(BufferScenario {
        description: "Dedent on '    line' removes 4 spaces".into(),
        initial_text: "    line".into(),
        actions: vec![Action::DedentSelection],
        expected_text: "line".into(),
        expected_primary: CursorExpect::at(0),
        ..Default::default()
    });
}

#[test]
fn migrated_auto_indent_after_newline_preserves_leading_whitespace() {
    // With auto_indent on, InsertNewline at end of an indented
    // line carries over the indentation.
    assert_buffer_scenario(BufferScenario {
        description: "InsertNewline mid-indented-line carries indentation".into(),
        initial_text: "    fn x()".into(),
        behavior: BehaviorFlags {
            auto_indent: true,
            ..Default::default()
        },
        actions: vec![Action::MoveDocumentEnd, Action::InsertNewline],
        expected_text: "    fn x()\n    ".into(),
        expected_primary: CursorExpect::at(15),
        ..Default::default()
    });
}
