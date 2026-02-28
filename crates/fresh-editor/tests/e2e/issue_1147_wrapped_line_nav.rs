//! Tests for issue #1147: Navigation bugs with wrapped lines at end of file.
//!
//! These tests reproduce three related navigation failures when line wrapping
//! is enabled and the file contains lines that wrap to multiple visual rows:
//!
//! 1. **Up-arrow scrolling bug**: Moving the cursor upward from the end of a
//!    file causes the buffer to scroll up unnecessarily (one scroll per
//!    Up press), even though the cursor is still far from the top of the
//!    viewport. The buffer should only scroll when the cursor actually reaches
//!    the top of the visible area.
//!
//! 2. **Down-arrow skipping wrapped lines**: Pressing Down on a line that wraps
//!    to multiple visual rows skips all the intermediate wrapped rows and jumps
//!    directly to the next logical line, instead of moving one visual row at a
//!    time within the same wrapped line.
//!
//! 3. **End key stuck on first visual segment**: Pressing End on a wrapped line
//!    moves the cursor to the end of the first visual segment, but pressing End
//!    again does nothing — it never advances to subsequent visual segments.
//!
//! Root cause (per the reporter): the editor miscalculates file length by
//! counting only unwrapped (logical) lines rather than accounting for the
//! display lines created by text wrapping.

use crate::common::harness::EditorTestHarness;
use crossterm::event::{KeyCode, KeyModifiers};

/// Build test content matching the issue's reproduction file:
/// - 20 short lines that don't wrap in 80 columns
/// - 3 lines that wrap once (each ~141 chars)
/// - 3 lines that wrap 3+ times (each ~249 chars)
/// No trailing newline.
fn make_issue_1147_content() -> String {
    let mut lines = Vec::new();

    // 20 short lines (won't wrap in 80 columns)
    for i in 1..=20 {
        lines.push(format!("Line {} - short line", i));
    }

    // 3 lines that wrap once (~141 chars each, wraps at ~66 text cols in an 80-wide terminal)
    for i in 21..=23 {
        lines.push(format!(
            "Line {} - this is a longer line that should wrap once in an \
             80-column terminal because it needs to exceed eighty characters \
             total length here",
            i
        ));
    }

    // 3 lines that wrap multiple times (~249 chars each)
    for i in 24..=26 {
        lines.push(format!(
            "Line {} - this line is extremely long and should wrap twice in \
             an 80-column terminal, because it has enough characters to fill \
             up more than two full rows of display output in the terminal \
             window making it an excellent test case for wrapping behavior",
            i
        ));
    }

    lines.join("\n") // no trailing newline
}

/// Issue #1147 Bug #1: Moving cursor up from end of file with wrapped lines
/// causes the viewport to scroll up on every Up press, even though the cursor
/// is nowhere near the top of the viewport.
///
/// Reproduction:
/// 1. Open the test file (26 lines, last 6 wrap) in an 80x25 terminal
/// 2. Press Ctrl+End to go to end of file
/// 3. Press Up arrow repeatedly
///
/// Expected: The cursor moves up through visual lines without the viewport
/// scrolling (since the cursor starts at the bottom and has plenty of room).
///
/// Actual (bug): The viewport scrolls up on every Up press, pushing the
/// bottom content off-screen while the cursor stays near the bottom.
#[test]
fn test_issue_1147_up_arrow_should_not_scroll_at_end_of_wrapped_file() {
    const WIDTH: u16 = 80;
    const HEIGHT: u16 = 25;

    let mut harness = EditorTestHarness::new(WIDTH, HEIGHT).unwrap();
    let content = make_issue_1147_content();
    let _fixture = harness.load_buffer_from_text(&content).unwrap();
    harness.render().unwrap();

    // Go to end of file
    harness
        .send_key(KeyCode::End, KeyModifiers::CONTROL)
        .unwrap();
    harness.render().unwrap();

    // Record the top line after Ctrl+End (viewport should be scrolled to show end of file)
    let top_line_at_end = harness.top_line_number();
    let initial_top_byte = harness.top_byte();

    eprintln!(
        "After Ctrl+End: cursor at byte {}, top_line={}, top_byte={}",
        harness.cursor_position(),
        top_line_at_end,
        initial_top_byte
    );

    // Press Up arrow 4 times — the cursor should move up through visual lines
    // of lines 26 and 25, but the viewport should NOT scroll because the cursor
    // is still well within the visible area.
    for i in 1..=4 {
        let top_byte_before = harness.top_byte();
        harness.send_key(KeyCode::Up, KeyModifiers::NONE).unwrap();
        harness.render().unwrap();
        let top_byte_after = harness.top_byte();

        eprintln!(
            "After Up #{}: cursor at byte {}, top_byte: {} -> {} (scrolled={})",
            i,
            harness.cursor_position(),
            top_byte_before,
            top_byte_after,
            top_byte_after != top_byte_before
        );
    }

    // After 4 Up presses from the end, the viewport should NOT have scrolled
    // significantly. The cursor was at the bottom and should have moved up
    // through visual lines without the viewport needing to scroll.
    let top_byte_after_ups = harness.top_byte();

    // The viewport should not have scrolled more than one line (at most).
    // With the bug, the viewport scrolls once per Up press, so after 4 presses
    // the top_byte moves significantly (by ~4 logical lines worth of content).
    //
    // We check that the top_byte hasn't decreased by more than 1 logical line.
    // A short line is ~20 chars, so if top_byte decreased by more than ~25 bytes
    // per Up press, that's the bug.
    let scroll_distance = if initial_top_byte > top_byte_after_ups {
        initial_top_byte - top_byte_after_ups
    } else {
        0
    };

    eprintln!(
        "Total scroll distance after 4 Up presses: {} bytes (initial_top_byte={}, final={})",
        scroll_distance, initial_top_byte, top_byte_after_ups
    );

    // With the bug, 4 Up presses scrolls the viewport up by ~4 logical lines
    // (approximately 80+ bytes). Without the bug, it should scroll at most
    // a small amount (0-1 visual lines, if the cursor was at the very last row).
    //
    // We allow up to one short line (~30 bytes) of scrolling as tolerance for
    // the initial cursor position being at the very bottom edge.
    assert!(
        scroll_distance <= 30,
        "Bug #1147: Viewport scrolled {} bytes after just 4 Up presses from end of file. \
         Expected minimal or no scrolling. The viewport is incorrectly scrolling on every \
         Up key press even though the cursor is well within the visible area. \
         Initial top_byte={}, after 4 Ups top_byte={}",
        scroll_distance,
        initial_top_byte,
        top_byte_after_ups
    );
}

/// Issue #1147 Bug #2: Down arrow skips wrapped visual lines within a long
/// logical line, jumping directly to the next logical line.
///
/// Reproduction:
/// 1. Open the test file in an 80x25 terminal
/// 2. Position cursor on line 24 (a line that wraps to ~4 visual rows)
/// 3. Press Down — cursor should move to the 2nd visual row of line 24
/// 4. Press Down — cursor should move to the 3rd visual row of line 24
///
/// Expected: Each Down press moves the cursor one visual row down within the
/// wrapped line before eventually reaching the next logical line.
///
/// Actual (bug): Down immediately jumps from line 24 to line 25, skipping all
/// intermediate visual rows of line 24.
#[test]
fn test_issue_1147_down_arrow_should_traverse_wrapped_visual_lines() {
    const WIDTH: u16 = 80;
    const HEIGHT: u16 = 25;

    let mut harness = EditorTestHarness::new(WIDTH, HEIGHT).unwrap();
    let content = make_issue_1147_content();
    let _fixture = harness.load_buffer_from_text(&content).unwrap();
    harness.render().unwrap();

    // Find byte offset of line 24 (0-indexed line 23 in the buffer)
    // Lines 1-20 are short (~20 chars each), lines 21-23 are ~141 chars each
    let line_24_start = content
        .match_indices('\n')
        .nth(22) // 23rd newline = start of line 24 (0-indexed line 23)
        .map(|(i, _)| i + 1)
        .unwrap();

    let line_25_start = content
        .match_indices('\n')
        .nth(23)
        .map(|(i, _)| i + 1)
        .unwrap();

    eprintln!(
        "Line 24 starts at byte {}, line 25 starts at byte {}",
        line_24_start, line_25_start
    );
    eprintln!(
        "Line 24 length: {} chars",
        line_25_start - line_24_start - 1
    );

    // Navigate to start of line 24 using Ctrl+G (Go to Line)
    // Use Ctrl+Home first, then move down to line 24
    harness
        .send_key(KeyCode::Home, KeyModifiers::CONTROL)
        .unwrap();
    harness.render().unwrap();

    // Move down 23 times to get to line 24 (from line 1)
    // Actually, let's use a different approach - go to a specific position
    // by navigating to line 24 col 1
    // Use Down key 23 times (since Down may or may not have the bug,
    // let's use a different approach: send Ctrl+G to go to line)
    //
    // We'll use the goto line action via Ctrl+G
    harness
        .send_key(KeyCode::Char('g'), KeyModifiers::CONTROL)
        .unwrap();
    harness.render().unwrap();
    // Type "24" and press Enter
    harness.type_text("24").unwrap();
    harness
        .send_key(KeyCode::Enter, KeyModifiers::NONE)
        .unwrap();
    harness.render().unwrap();

    let pos_at_line_24 = harness.cursor_position();
    eprintln!(
        "After Ctrl+G 24: cursor at byte {} (expected ~{})",
        pos_at_line_24, line_24_start
    );

    // Cursor should be at the start of line 24
    assert_eq!(
        pos_at_line_24, line_24_start,
        "Ctrl+G 24 should place cursor at byte {} (start of line 24), got {}",
        line_24_start, pos_at_line_24
    );

    // Line 24 is ~249 chars long. In an 80-column terminal with ~7 cols for
    // gutter ("   24 │ ") and 1 for scrollbar, the text area is ~72 chars wide.
    // So line 24 wraps to ceil(249/72) ≈ 4 visual rows.
    //
    // Pressing Down from col 1 of line 24 should move the cursor to the 2nd
    // visual row of line 24 (approximately byte offset line_24_start + 72),
    // NOT to line 25.

    harness.send_key(KeyCode::Down, KeyModifiers::NONE).unwrap();
    harness.render().unwrap();

    let pos_after_first_down = harness.cursor_position();
    eprintln!(
        "After 1st Down from line 24 col 1: cursor at byte {}",
        pos_after_first_down
    );

    // The cursor should still be within line 24 (between line_24_start and line_25_start)
    assert!(
        pos_after_first_down >= line_24_start && pos_after_first_down < line_25_start,
        "Bug #1147: After pressing Down from start of line 24, cursor jumped to byte {} \
         which is on line 25 (starts at {}). It should have moved to the 2nd visual row \
         of line 24 (staying within bytes {}..{})",
        pos_after_first_down,
        line_25_start,
        line_24_start,
        line_25_start
    );

    // Press Down again — should move to 3rd visual row of line 24
    harness.send_key(KeyCode::Down, KeyModifiers::NONE).unwrap();
    harness.render().unwrap();

    let pos_after_second_down = harness.cursor_position();
    eprintln!(
        "After 2nd Down: cursor at byte {}",
        pos_after_second_down
    );

    assert!(
        pos_after_second_down >= line_24_start && pos_after_second_down < line_25_start,
        "Bug #1147: After 2nd Down from line 24, cursor jumped to byte {} \
         which is beyond line 24 (ends at {}). It should still be within line 24's \
         wrapped visual rows.",
        pos_after_second_down,
        line_25_start
    );

    // Each Down press should make forward progress within the line
    assert!(
        pos_after_second_down > pos_after_first_down,
        "Down should advance cursor position within wrapped line. \
         First down: {}, second down: {}",
        pos_after_first_down,
        pos_after_second_down
    );
}

/// Issue #1147 Bug #3: End key gets stuck on the first visual segment of a
/// wrapped line. Pressing End goes to the end of the first visual row, but
/// pressing End again does nothing instead of advancing to the end of the next
/// visual row.
///
/// Reproduction:
/// 1. Open the test file in an 80x25 terminal
/// 2. Go to start of line 26 (Col 1)
/// 3. Press End — cursor goes to end of 1st visual segment (~Col 66)
/// 4. Press End — cursor should go to end of 2nd visual segment (~Col 132)
///
/// Expected: Each End press advances to the end of the next visual segment,
/// eventually reaching the physical end of the line.
///
/// Actual (bug): End stays stuck at the end of the first visual segment (~Col 66)
/// and never advances.
#[test]
fn test_issue_1147_end_key_should_advance_through_wrapped_segments() {
    const WIDTH: u16 = 80;
    const HEIGHT: u16 = 25;

    let mut harness = EditorTestHarness::new(WIDTH, HEIGHT).unwrap();
    let content = make_issue_1147_content();
    let _fixture = harness.load_buffer_from_text(&content).unwrap();
    harness.render().unwrap();

    // Navigate to start of line 26
    let line_26_start = content
        .match_indices('\n')
        .nth(24) // 25th newline = start of line 26 (0-indexed line 25)
        .map(|(i, _)| i + 1)
        .unwrap();

    let line_26_text = &content[line_26_start..];
    let line_26_len = line_26_text.len();

    eprintln!(
        "Line 26 starts at byte {}, length {} chars",
        line_26_start, line_26_len
    );

    // Go to line 26 via Ctrl+G
    harness
        .send_key(KeyCode::Char('g'), KeyModifiers::CONTROL)
        .unwrap();
    harness.render().unwrap();
    harness.type_text("26").unwrap();
    harness
        .send_key(KeyCode::Enter, KeyModifiers::NONE)
        .unwrap();
    harness.render().unwrap();

    let pos_at_line_26 = harness.cursor_position();
    assert_eq!(
        pos_at_line_26, line_26_start,
        "Should be at start of line 26"
    );

    // Press End — should go to end of 1st visual segment
    harness.send_key(KeyCode::End, KeyModifiers::NONE).unwrap();
    harness.render().unwrap();
    let pos_after_first_end = harness.cursor_position();
    eprintln!(
        "After 1st End: cursor at byte {} (offset {} within line 26)",
        pos_after_first_end,
        pos_after_first_end - line_26_start
    );

    // Should be at end of first visual segment (not at position 0, not at end of line)
    assert!(
        pos_after_first_end > line_26_start,
        "End should move cursor forward from start of line"
    );
    assert!(
        pos_after_first_end < line_26_start + line_26_len,
        "First End should go to end of visual segment, not end of physical line"
    );

    // Press End again — should advance to end of 2nd visual segment
    harness.send_key(KeyCode::End, KeyModifiers::NONE).unwrap();
    harness.render().unwrap();
    let pos_after_second_end = harness.cursor_position();
    eprintln!(
        "After 2nd End: cursor at byte {} (offset {} within line 26)",
        pos_after_second_end,
        pos_after_second_end - line_26_start
    );

    assert!(
        pos_after_second_end > pos_after_first_end,
        "Bug #1147: End key is stuck! Pressing End from byte {} (end of 1st visual \
         segment) did not advance. Cursor stayed at byte {}. Expected it to move to \
         the end of the 2nd visual segment. Line 26 is {} chars long and wraps to \
         multiple visual rows.",
        pos_after_first_end,
        pos_after_second_end,
        line_26_len
    );

    // Press End a third time — should advance further
    harness.send_key(KeyCode::End, KeyModifiers::NONE).unwrap();
    harness.render().unwrap();
    let pos_after_third_end = harness.cursor_position();
    eprintln!(
        "After 3rd End: cursor at byte {} (offset {} within line 26)",
        pos_after_third_end,
        pos_after_third_end - line_26_start
    );

    assert!(
        pos_after_third_end > pos_after_second_end,
        "Bug #1147: End key stopped advancing after 2nd press. \
         Cursor at byte {} should have moved past byte {}. \
         Line 26 is {} chars long.",
        pos_after_third_end,
        pos_after_second_end,
        line_26_len
    );

    // Eventually pressing End enough times should reach the physical end of line 26
    let mut pos = pos_after_third_end;
    let line_26_end = line_26_start + line_26_len;
    let mut attempts = 0;
    while pos < line_26_end && attempts < 10 {
        harness.send_key(KeyCode::End, KeyModifiers::NONE).unwrap();
        harness.render().unwrap();
        let new_pos = harness.cursor_position();
        if new_pos == pos {
            break; // Stuck — this is the bug
        }
        pos = new_pos;
        attempts += 1;
    }

    assert_eq!(
        pos, line_26_end,
        "Repeated End presses should eventually reach byte {} (physical end of line 26), \
         but got stuck at byte {} (offset {} within line 26)",
        line_26_end,
        pos,
        pos - line_26_start
    );
}

/// Combined test: verify that Down arrow correctly counts wrapped visual lines
/// for scrolling decisions near end of file.
///
/// This tests the root cause identified in issue #1147: the editor calculates
/// file length based on unwrapped line count rather than visual line count,
/// causing incorrect scrolling behavior when navigating near the end of a file
/// with wrapped lines.
#[test]
fn test_issue_1147_viewport_stable_while_navigating_up_through_wrapped_content() {
    const WIDTH: u16 = 80;
    const HEIGHT: u16 = 25;

    let mut harness = EditorTestHarness::new(WIDTH, HEIGHT).unwrap();
    let content = make_issue_1147_content();
    let _fixture = harness.load_buffer_from_text(&content).unwrap();
    harness.render().unwrap();

    // Go to end of file
    harness
        .send_key(KeyCode::End, KeyModifiers::CONTROL)
        .unwrap();
    harness.render().unwrap();

    // Record the initial viewport state
    let initial_top_byte = harness.top_byte();
    let (_, initial_cursor_y) = harness.screen_cursor_position();

    eprintln!(
        "At end of file: top_byte={}, cursor screen y={}",
        initial_top_byte, initial_cursor_y
    );

    // Press Up 8 times — each should move cursor up one visual row
    // The viewport should NOT scroll until cursor reaches the top of visible area
    let mut viewport_scrolled_count = 0;
    for i in 1..=8 {
        let top_byte_before = harness.top_byte();
        harness.send_key(KeyCode::Up, KeyModifiers::NONE).unwrap();
        harness.render().unwrap();
        let top_byte_after = harness.top_byte();

        if top_byte_after != top_byte_before {
            viewport_scrolled_count += 1;
        }

        let (_, cursor_y) = harness.screen_cursor_position();
        eprintln!(
            "Up #{}: cursor byte={}, screen y={}, top_byte: {} -> {} (scrolled={})",
            i,
            harness.cursor_position(),
            cursor_y,
            top_byte_before,
            top_byte_after,
            top_byte_after != top_byte_before
        );
    }

    // The content area is HEIGHT - 4 = 21 visual rows. The cursor starts near the
    // bottom. After 8 Up presses it should still be well within the visible area
    // (around row 13 or so). The viewport should NOT have scrolled more than once.
    assert!(
        viewport_scrolled_count <= 1,
        "Bug #1147: Viewport scrolled {} times during 8 Up presses from end of file. \
         Expected 0-1 scrolls (only if cursor was on the very last visible row). \
         The editor is incorrectly scrolling the viewport on every Up press.",
        viewport_scrolled_count
    );
}
