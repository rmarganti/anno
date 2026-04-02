use super::*;

#[test]
fn new_plain_builds_terminal_independent_default_state() {
    let state = AppState::new_plain("[stdin]".to_string(), "first\nsecond".to_string());

    assert_eq!(state.source_name(), "[stdin]");
    assert_eq!(state.mode(), Mode::Normal);
    assert!(state.annotations().is_empty());
    assert_eq!(state.annotation_count(), 0);
    assert!(!state.should_quit());
    assert!(!state.has_confirm_dialog());
    assert!(!state.is_annotation_inspect_visible());
    assert!(!state.is_help_visible());
    assert!(state.is_panel_visible());
    assert_eq!(state.command_buffer(), "");
    assert!(!state.word_wrap());
    assert!(state.confirm_dialog().is_none());

    let cursor = state.cursor();
    assert_eq!(cursor.row, 0);
    assert_eq!(cursor.col, 0);

    assert_eq!(state.document_view().cursor(), cursor);
    assert!(state.input_box().is_none());
    let _ = state.annotation_list_panel();

    let _ = ExitResult::QuitSilent;
}

#[test]
fn take_exit_result_returns_once() {
    let mut state = AppState::new_plain("[stdin]".to_string(), String::new());
    state.exit_result = Some(ExitResult::QuitSilent);

    assert!(matches!(
        state.take_exit_result(),
        Some(ExitResult::QuitSilent)
    ));
    assert!(state.take_exit_result().is_none());
}

#[test]
fn test_harness_basic() {
    AppTestHarness::new("hello")
        .assert_mode(Mode::Normal)
        .assert_cursor(0, 0);
}
