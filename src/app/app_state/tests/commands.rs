use super::*;

#[test]
fn command_q_sets_quit_with_output() {
    let mut harness = harness("hello");
    harness.keys(":q<Enter>").assert_should_quit();

    match harness.state_mut().take_exit_result() {
        Some(ExitResult::QuitWithOutput(output)) => {
            assert_eq!(output, "No annotations.");
        }
        _ => panic!("expected quit with output"),
    }
}

#[test]
fn command_q_bang_sets_silent_quit() {
    let mut harness = harness("hello");
    harness.keys(":q!<Enter>").assert_should_quit();

    assert!(matches!(
        harness.state_mut().take_exit_result(),
        Some(ExitResult::QuitSilent)
    ));
}

#[test]
fn command_q_uses_json_export_when_configured() {
    let mut state = AppState::new_plain_with_format(
        "demo.md".to_string(),
        "hello".to_string(),
        ExportFormat::Json,
    );

    state.handle_key(KeyEvent::new(KeyCode::Char(':'), KeyModifiers::NONE));
    state.handle_key(KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE));
    state.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

    match state.take_exit_result() {
        Some(ExitResult::QuitWithOutput(output)) => {
            assert_eq!(
                output,
                "{\"source\":\"demo.md\",\"total\":0,\"annotations\":[]}"
            );
        }
        _ => panic!("expected quit with output"),
    }
}

#[test]
fn backspace_on_empty_command_exits_command_mode() {
    harness("hello")
        .keys(":<BS>")
        .assert_mode(Mode::Normal)
        .assert_not_quit();
}

#[test]
fn command_chars_append_to_buffer() {
    let mut harness = harness("hello");
    harness.keys(":q!").assert_mode(Mode::Command);

    assert_eq!(harness.state().command_buffer(), "q!");
}

#[test]
fn command_backspace_removes_last_char() {
    let mut harness = harness("hello");
    harness.keys(":q!<BS>").assert_mode(Mode::Command);

    assert_eq!(harness.state().command_buffer(), "q");
}

#[test]
fn backspace_on_single_command_char_exits_and_clears_buffer() {
    let mut harness = harness("hello");
    harness.keys(":a<BS>").assert_mode(Mode::Normal);

    assert_eq!(harness.state().command_buffer(), "");
}

#[test]
fn confirm_unknown_command_exits_and_clears_buffer() {
    let mut harness = harness("hello");
    harness
        .keys(":x<Enter>")
        .assert_mode(Mode::Normal)
        .assert_not_quit();

    assert_eq!(harness.state().command_buffer(), "");
}

#[test]
fn entering_command_mode_clears_existing_buffer() {
    let mut harness = harness("hello");
    harness.keys(":q<Esc>:").assert_mode(Mode::Command);

    assert_eq!(harness.state().command_buffer(), "");
}

#[test]
fn ctrl_c_quits_from_normal_mode() {
    harness("hello").keys("<C-c>").assert_should_quit();
}

#[test]
fn ctrl_c_quits_from_visual_mode() {
    harness("hello").keys("v<C-c>").assert_should_quit();
}

#[test]
fn ctrl_c_quits_from_insert_mode() {
    harness("hello").keys("i<C-c>").assert_should_quit();
}

#[test]
fn ctrl_c_quits_from_command_mode() {
    harness("hello").keys(":<C-c>").assert_should_quit();
}

#[test]
fn ctrl_c_quits_from_annotation_list_mode() {
    harness("hello").keys("vld<Tab><C-c>").assert_should_quit();
}
