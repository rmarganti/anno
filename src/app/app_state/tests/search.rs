use super::*;
use crate::keybinds::handler::SearchDirection;

#[test]
fn search_state_defaults_are_initialized() {
    let state = AppState::new_plain("[stdin]".to_string(), "first\nsecond".to_string());

    assert_eq!(state.search_buffer(), "");
    assert_eq!(state.last_search_pattern(), None);
    assert_eq!(
        state.last_search_direction_for_test(),
        SearchDirection::Forward
    );
}

#[test]
fn enter_search_mode_sets_mode_direction_and_clears_buffer() {
    let mut state = AppState::new_plain("[stdin]".to_string(), "first".to_string());
    state.handle_search_char('x');

    state.enter_search_mode(SearchDirection::Backward);

    assert_eq!(state.mode(), Mode::Search);
    assert_eq!(state.search_buffer(), "");
    assert_eq!(
        state.last_search_direction_for_test(),
        SearchDirection::Backward
    );
}

#[test]
fn search_chars_append_to_buffer() {
    let mut state = AppState::new_plain("[stdin]".to_string(), "first".to_string());
    state.enter_search_mode(SearchDirection::Forward);

    state.handle_search_char('f');
    state.handle_search_char('o');
    state.handle_search_char('o');

    assert_eq!(state.search_buffer(), "foo");
}

#[test]
fn search_backspace_removes_last_char() {
    let mut state = AppState::new_plain("[stdin]".to_string(), "first".to_string());
    state.enter_search_mode(SearchDirection::Forward);
    state.handle_search_char('f');
    state.handle_search_char('o');

    state.handle_search_backspace();

    assert_eq!(state.mode(), Mode::Search);
    assert_eq!(state.search_buffer(), "f");
}

#[test]
fn search_backspace_on_empty_buffer_exits_search_mode() {
    let mut state = AppState::new_plain("[stdin]".to_string(), "first".to_string());
    state.enter_search_mode(SearchDirection::Forward);
    state.handle_search_char('f');

    state.handle_search_backspace();

    assert_eq!(state.mode(), Mode::Normal);
    assert_eq!(state.search_buffer(), "");
}

#[test]
fn search_confirm_stores_pattern_clears_buffer_and_exits_mode() {
    let mut state = AppState::new_plain("[stdin]".to_string(), "first".to_string());
    state.enter_search_mode(SearchDirection::Backward);
    state.handle_search_char('f');
    state.handle_search_char('i');

    state.handle_search_confirm();

    assert_eq!(state.mode(), Mode::Normal);
    assert_eq!(state.search_buffer(), "");
    assert_eq!(state.last_search_pattern(), Some("fi"));
    assert_eq!(
        state.last_search_direction_for_test(),
        SearchDirection::Backward
    );
}

#[test]
fn search_confirm_with_empty_buffer_preserves_last_pattern() {
    let mut state = AppState::new_plain("[stdin]".to_string(), "first".to_string());
    state.enter_search_mode(SearchDirection::Forward);
    state.handle_search_char('f');
    state.handle_search_confirm();

    state.enter_search_mode(SearchDirection::Backward);
    state.handle_search_confirm();

    assert_eq!(state.last_search_pattern(), Some("f"));
    assert_eq!(
        state.last_search_direction_for_test(),
        SearchDirection::Backward
    );
}

#[test]
fn search_prev_uses_opposite_direction_without_changing_stored_direction() {
    let mut harness = harness("x target here\nx target there\nx target everywhere");

    harness.keys("/target<Enter>").assert_cursor(0, 2);
    harness.keys("N").assert_cursor(2, 2);

    assert_eq!(
        harness.state().last_search_direction_for_test(),
        SearchDirection::Forward
    );

    harness.keys("n").assert_cursor(0, 2);
}

#[test]
fn search_confirm_jumps_to_same_line_match_after_cursor() {
    harness("alpha beta alpha")
        .keys("l/beta<Enter>")
        .assert_cursor(0, 6);
}

#[test]
fn search_confirm_jumps_to_subsequent_line_match() {
    harness("alpha\nbeta alpha")
        .keys("/beta<Enter>")
        .assert_cursor(1, 0);
}

#[test]
fn forward_search_wraps_around_document() {
    harness("alpha\nbeta\ngamma alpha")
        .keys("G$/beta<Enter>")
        .assert_cursor(1, 0);
}

#[test]
fn forward_search_with_no_match_leaves_cursor_unchanged() {
    harness("alpha\nbeta")
        .keys("jll/missing<Enter>")
        .assert_cursor(1, 2);
}

#[test]
fn backward_search_finds_same_line_match_before_cursor() {
    harness("alpha beta alpha")
        .keys("$?beta<Enter>")
        .assert_cursor(0, 6);
}

#[test]
fn backward_search_wraps_around_document() {
    harness("alpha\nbeta\ngamma")
        .keys("gg?beta<Enter>")
        .assert_cursor(1, 0);
}

#[test]
fn search_next_and_prev_follow_vim_direction_rules() {
    harness("x target one\nx target two\nx target three")
        .keys("/target<Enter>nN")
        .assert_cursor(0, 2);
}

#[test]
fn backward_confirmed_search_makes_n_continue_backward() {
    harness("x target one\nx target two\nx target three")
        .keys("G$?target<Enter>n")
        .assert_cursor(1, 2);
}

#[test]
fn counted_search_repeat_advances_multiple_matches() {
    harness("x target a\nx target b\nx target c\nx target d")
        .keys("/target<Enter>2n")
        .assert_cursor(2, 2);
}

#[test]
fn search_works_on_single_line_document() {
    harness("alpha beta gamma beta")
        .keys("/beta<Enter>n")
        .assert_cursor(0, 17);
}

#[test]
fn search_skips_match_starting_at_cursor_position() {
    harness("cat dog cat")
        .keys("/cat<Enter>")
        .assert_cursor(0, 8);
}
