//! E2E tests for vertical centering of search matches when Find Next must scroll.
//!
//! Covers <https://github.com/sinelaw/fresh/issues/1251>:
//! When Find Next navigates to a match that is off-screen, the viewport is
//! scrolled so the match is vertically centered — providing surrounding
//! context above and below. Matches that were already visible are not
//! re-scrolled.

use crate::common::harness::EditorTestHarness;
use crossterm::event::{KeyCode, KeyModifiers};

/// When Find Next jumps to a match far below the viewport, the viewport
/// should end up with the match vertically centered.
#[test]
fn test_find_next_centers_match_when_scrolling() {
    let temp_dir = tempfile::TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test.txt");

    let mut content = String::new();
    for i in 0..100 {
        if i == 2 || i == 60 {
            content.push_str(&format!("line {} NEEDLE here\n", i));
        } else {
            content.push_str(&format!("line {} filler text\n", i));
        }
    }
    std::fs::write(&file_path, &content).unwrap();

    let viewport_rows: u16 = 24;
    let mut harness = EditorTestHarness::new(100, viewport_rows).unwrap();
    harness.open_file(&file_path).unwrap();
    harness.render().unwrap();

    harness
        .send_key(KeyCode::Char('f'), KeyModifiers::CONTROL)
        .unwrap();
    harness.render().unwrap();
    harness.type_text("NEEDLE").unwrap();
    harness.render().unwrap();
    harness
        .send_key(KeyCode::Enter, KeyModifiers::NONE)
        .unwrap();
    harness.process_async_and_render().unwrap();

    // Move to the next match (line 60, off-screen below).
    harness.send_key(KeyCode::F(3), KeyModifiers::NONE).unwrap();
    harness.process_async_and_render().unwrap();

    // The viewport height available for content is less than the terminal
    // height (status bar, search panel, etc.).  Use the editor's own reported
    // viewport height and top line to verify centering.
    let viewport_height = harness.viewport_height();
    let top_line = harness.top_line_number();

    // The match is on line 60 (0-indexed).  Centering places the match
    // roughly at viewport_height / 2 rows from the top.
    let expected_top_line = 60usize.saturating_sub(viewport_height / 2);
    assert_eq!(
        top_line, expected_top_line,
        "Find Next should center the match vertically when scrolling; \
         viewport_height={}, expected_top_line={}, got top_line={}",
        viewport_height, expected_top_line, top_line
    );
}

/// When the next match is already fully visible in the viewport, Find Next
/// should not scroll or recenter — that would be surprising and discard
/// the user's reading context.
#[test]
fn test_find_next_does_not_recenter_visible_match() {
    let temp_dir = tempfile::TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test.txt");

    let mut content = String::new();
    for i in 0..40 {
        if i == 2 || i == 5 {
            content.push_str(&format!("line {} NEEDLE here\n", i));
        } else {
            content.push_str(&format!("line {} filler text\n", i));
        }
    }
    std::fs::write(&file_path, &content).unwrap();

    let mut harness = EditorTestHarness::new(100, 24).unwrap();
    harness.open_file(&file_path).unwrap();
    harness.render().unwrap();

    harness
        .send_key(KeyCode::Char('f'), KeyModifiers::CONTROL)
        .unwrap();
    harness.render().unwrap();
    harness.type_text("NEEDLE").unwrap();
    harness.render().unwrap();
    harness
        .send_key(KeyCode::Enter, KeyModifiers::NONE)
        .unwrap();
    harness.process_async_and_render().unwrap();

    let top_line_before = harness.top_line_number();

    // Both matches are within the initial viewport, so Find Next should not scroll.
    harness.send_key(KeyCode::F(3), KeyModifiers::NONE).unwrap();
    harness.process_async_and_render().unwrap();

    let top_line_after = harness.top_line_number();
    assert_eq!(
        top_line_before, top_line_after,
        "Find Next should not scroll when the next match is already visible"
    );
}
