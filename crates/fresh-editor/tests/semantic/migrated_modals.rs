//! Migrated modal-popup claims from `tests/e2e/command_palette.rs`,
//! `tests/e2e/file_browser.rs`, and `tests/e2e/popup_selection.rs`.
//!
//! Each scenario uses `OpenPrompt` / `CancelPrompt` /
//! `ConfirmPrompt` `InputEvent` variants to drive the popup
//! lifecycle and asserts depth + top-popup kind on
//! `ModalSnapshot`.

use crate::common::scenario::context::PromptKind;
use crate::common::scenario::input_event::InputEvent;
use crate::common::scenario::modal_scenario::{
    assert_modal_scenario, ModalScenario,
};
use crate::common::scenario::observable::ModalState;

#[test]
fn migrated_command_palette_opens_then_cancels() {
    assert_modal_scenario(ModalScenario {
        description: "OpenPrompt(CommandPalette) + CancelPrompt → no modal".into(),
        initial_text: String::new(),
        events: vec![
            InputEvent::OpenPrompt(PromptKind::CommandPalette),
            InputEvent::CancelPrompt,
        ],
        expected_modal: ModalState::default(),
    });
}

#[test]
fn migrated_quick_open_opens_then_cancels() {
    assert_modal_scenario(ModalScenario {
        description: "OpenPrompt(FileOpen) + CancelPrompt → no modal".into(),
        initial_text: String::new(),
        events: vec![
            InputEvent::OpenPrompt(PromptKind::FileOpen),
            InputEvent::CancelPrompt,
        ],
        expected_modal: ModalState::default(),
    });
}

#[test]
fn migrated_goto_line_opens_then_cancels() {
    assert_modal_scenario(ModalScenario {
        description: "OpenPrompt(Goto) + CancelPrompt → no modal".into(),
        initial_text: String::new(),
        events: vec![
            InputEvent::OpenPrompt(PromptKind::Goto),
            InputEvent::CancelPrompt,
        ],
        expected_modal: ModalState::default(),
    });
}

#[test]
fn migrated_no_events_means_no_modal() {
    assert_modal_scenario(ModalScenario {
        description: "fresh harness has empty modal stack".into(),
        initial_text: "hi".into(),
        events: vec![],
        expected_modal: ModalState::default(),
    });
}
