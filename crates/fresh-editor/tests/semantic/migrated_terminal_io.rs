//! Migrated terminal-IO scenarios — the kinds of vt100 / ANSI
//! claims `tests/e2e/rendering.rs`,
//! `tests/e2e/redraw_screen.rs`, and `tests/e2e/ansi_cursor.rs`
//! make.

use crate::common::harness::EditorTestHarness;
use crate::common::scenario::observable::{Observable, RoundTripGrid};
use fresh::test_api::Action;

#[test]
fn migrated_buffer_text_round_trips_through_ansi_emit() {
    let mut h = EditorTestHarness::with_temp_project(60, 12).unwrap();
    let _f = h.load_buffer_from_text("hello world").unwrap();
    let grid = RoundTripGrid::extract(&mut h);
    assert!(
        grid.rows.iter().any(|r| r.contains("hello world")),
        "vt100 grid lacks 'hello world'; rows: {:#?}",
        grid.rows
    );
}

#[test]
fn migrated_typing_appears_in_grid_after_render_real() {
    let mut h = EditorTestHarness::with_temp_project(60, 12).unwrap();
    let _f = h.load_buffer_from_text("").unwrap();
    h.api_mut().dispatch(Action::InsertChar('A'));
    h.api_mut().dispatch(Action::InsertChar('B'));
    h.api_mut().dispatch(Action::InsertChar('C'));
    let grid = RoundTripGrid::extract(&mut h);
    assert!(
        grid.rows.iter().any(|r| r.contains("ABC")),
        "vt100 grid lacks typed 'ABC'; rows: {:#?}",
        grid.rows
    );
}

#[test]
fn migrated_grid_dimensions_match_terminal() {
    let mut h = EditorTestHarness::with_temp_project(40, 8).unwrap();
    let _f = h.load_buffer_from_text("hello").unwrap();
    let grid = RoundTripGrid::extract(&mut h);
    assert_eq!(grid.height, 8, "grid height should equal terminal height");
}
