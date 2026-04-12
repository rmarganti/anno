use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

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
fn search_prev_reverses_last_search_direction() {
    let mut state = AppState::new_plain("[stdin]".to_string(), "first".to_string());
    state.enter_search_mode(SearchDirection::Forward);
    state.handle_search_char('f');
    state.handle_search_confirm();

    state.handle_search_prev();
    assert_eq!(
        state.last_search_direction_for_test(),
        SearchDirection::Backward
    );

    state.handle_search_next();
    assert_eq!(
        state.last_search_direction_for_test(),
        SearchDirection::Backward
    );
}

#[test]
fn counted_search_repeats_dispatch_from_normal_mode() {
    let mut state = AppState::new_plain("[stdin]".to_string(), "first".to_string());
    state.enter_search_mode(SearchDirection::Forward);
    state.handle_search_char('f');
    state.handle_search_confirm();

    state.handle_key(KeyEvent::new(KeyCode::Char('2'), KeyModifiers::NONE));
    state.handle_key(KeyEvent::new(KeyCode::Char('N'), KeyModifiers::NONE));

    assert_eq!(
        state.last_search_direction_for_test(),
        SearchDirection::Forward
    );
}
