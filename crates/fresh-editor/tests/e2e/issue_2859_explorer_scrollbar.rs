//! Regression test for issue #2859: a scrollable File Explorer did not draw
//! its scrollbar, even though the tree itself could be scrolled.

use crate::common::harness::EditorTestHarness;
use fresh::config::{Config, ExplorerWidth};
use std::fs;

#[test]
fn scrollable_file_explorer_draws_themed_scrollbar() {
    let mut config = Config {
        theme: "high-contrast".into(),
        ..Default::default()
    };
    config.file_explorer.width = ExplorerWidth::Columns(30);

    let mut harness = EditorTestHarness::with_temp_project_and_config(100, 30, config).unwrap();
    let project_root = harness.project_dir().unwrap();
    for i in 0..80 {
        fs::write(project_root.join(format!("file_{i:02}.txt")), "x").unwrap();
    }

    harness.editor_mut().focus_file_explorer();
    harness.wait_for_file_explorer().unwrap();
    harness.wait_for_file_explorer_item("file_00.txt").unwrap();
    harness.render().unwrap();

    let theme = harness.editor().theme();
    let expected_track = theme.scrollbar_track_fg;
    let expected_thumb = theme.scrollbar_thumb_fg;

    // The fixed-width, left-side explorer occupies columns 0..30. Its
    // right border is column 29 and the reserved inner scrollbar lane is 28.
    let scrollbar_col = 28;
    let backgrounds: Vec<_> = (0..harness.buffer().area.height)
        .filter_map(|row| harness.get_cell_style(scrollbar_col, row)?.bg)
        .collect();

    assert!(
        backgrounds.contains(&expected_track),
        "Explorer scrollbar should draw the themed track ({expected_track:?}); \
         saw {backgrounds:?}.\nScreen:\n{}",
        harness.screen_to_string()
    );
    assert!(
        backgrounds.contains(&expected_thumb),
        "Explorer scrollbar should draw the themed thumb ({expected_thumb:?}); \
         saw {backgrounds:?}.\nScreen:\n{}",
        harness.screen_to_string()
    );
}
