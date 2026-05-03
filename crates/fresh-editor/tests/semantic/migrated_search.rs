//! Migrated search-related claims from `tests/e2e/search.rs`,
//! `tests/e2e/search_replace.rs`, and
//! `tests/e2e/search_navigation_after_move.rs`.
//!
//! The semantic claim of search is "Find moves cursor to the
//! match." That's expressible as `BufferScenario` because the
//! resulting state is buffer + cursor (modal popup is incidental).

use crate::common::scenario::buffer_scenario::{
    assert_buffer_scenario, BufferScenario, CursorExpect,
};
use fresh::test_api::Action;

#[test]
fn migrated_find_action_is_idempotent_on_text() {
    // Triggering Find without confirming a query doesn't change
    // buffer text or cursor.
    assert_buffer_scenario(BufferScenario {
        description: "Action::Search leaves text + cursor intact".into(),
        initial_text: "hello world".into(),
        actions: vec![Action::Search],
        expected_text: "hello world".into(),
        expected_primary: CursorExpect::at(0),
        ..Default::default()
    });
}

#[test]
fn migrated_find_selection_next_no_selection_does_not_change_buffer() {
    // Without an active selection, FindSelectionNext is a no-op
    // on buffer text.
    assert_buffer_scenario(BufferScenario {
        description: "FindSelectionNext with no selection ⇒ buffer unchanged".into(),
        initial_text: "alpha bravo charlie".into(),
        actions: vec![Action::FindSelectionNext],
        expected_text: "alpha bravo charlie".into(),
        expected_primary: CursorExpect::at(0),
        ..Default::default()
    });
}
