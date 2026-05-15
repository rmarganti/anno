use super::*;
use crossterm::event::MouseEventKind;

#[test]
fn vertical_wheel_events_are_injected_without_using_keyboard_paths() {
    let mut harness = harness("alpha\nbeta\ngamma");

    harness.keys("g");
    assert!(harness.state().keybinds().has_pending());

    assert!(harness.mouse_scroll_down());
    assert!(harness.mouse_scroll_up());
    assert!(harness.state().keybinds().has_pending());
    harness.assert_cursor(0, 0);
}

#[test]
fn unsupported_mouse_events_are_ignored() {
    let mut harness = harness("alpha\nbeta");

    harness.keys("g");
    assert!(harness.state().keybinds().has_pending());

    assert!(!harness.mouse(MouseEventKind::Moved));
    assert!(!harness.mouse(MouseEventKind::Down(crossterm::event::MouseButton::Left)));
    assert!(!harness.mouse(MouseEventKind::ScrollLeft));
    assert!(!harness.mouse(MouseEventKind::ScrollRight));
    assert!(harness.state().keybinds().has_pending());
    harness.assert_cursor(0, 0);
}

#[test]
fn normal_mode_mouse_wheel_matches_existing_document_navigation() {
    let mut keyboard = harness("alpha\nbeta\ngamma");
    keyboard.keys("jjk");

    let mut mouse = harness("alpha\nbeta\ngamma");
    assert!(mouse.mouse_scroll_down());
    assert!(mouse.mouse_scroll_down());
    assert!(mouse.mouse_scroll_up());

    assert_eq!(mouse.state().cursor(), keyboard.state().cursor());
}

#[test]
fn visual_mode_mouse_wheel_matches_existing_j_selection_behavior() {
    let mut keyboard = harness("abcd\nefgh\nijkl");
    keyboard.keys("lvjd").assert_mode(Mode::Normal);

    let mut mouse = harness("abcd\nefgh\nijkl");
    mouse.keys("lv").assert_mode(Mode::Visual);
    assert!(mouse.mouse_scroll_down());
    mouse.keys("d").assert_mode(Mode::Normal);

    assert_matching_first_annotation(&mouse, &keyboard);
}

#[test]
fn visual_line_mode_mouse_wheel_matches_existing_j_selection_behavior() {
    let mut keyboard = harness("one\ntwo\nthree");
    keyboard.keys("Vjd").assert_mode(Mode::Normal);

    let mut mouse = harness("one\ntwo\nthree");
    mouse.keys("V").assert_mode(Mode::VisualLine);
    assert!(mouse.mouse_scroll_down());
    mouse.keys("d").assert_mode(Mode::Normal);

    assert_matching_first_annotation(&mouse, &keyboard);
}

#[test]
fn annotation_list_mouse_wheel_matches_existing_selection_motion() {
    let mut keyboard = harness("alpha\nbeta\ngamma\ndelta");
    create_three_deletions(&mut keyboard);
    keyboard.keys("<Tab>2j");

    let mut mouse = harness("alpha\nbeta\ngamma\ndelta");
    create_three_deletions(&mut mouse);
    mouse.keys("<Tab>");
    assert!(mouse.mouse_scroll_down());
    assert!(mouse.mouse_scroll_down());

    assert_eq!(
        mouse.state().selected_annotation_range(),
        keyboard.state().selected_annotation_range()
    );
}

#[test]
fn annotation_list_mouse_wheel_noops_at_boundaries_without_fallthrough() {
    let mut harness = harness("alpha\nbeta\ngamma");
    create_two_deletions(&mut harness);

    harness.keys("gg<Tab>");
    let first = harness.state().selected_annotation_range();
    assert!(harness.mouse_scroll_up());
    assert_eq!(harness.state().selected_annotation_range(), first);
    harness.assert_cursor(0, 0);

    assert!(harness.mouse_scroll_down());
    let last = harness.state().selected_annotation_range();
    assert!(harness.mouse_scroll_down());
    assert_eq!(harness.state().selected_annotation_range(), last);
    harness.assert_cursor(0, 0);
}

fn assert_matching_first_annotation(actual: &AppTestHarness, expected: &AppTestHarness) {
    let actual = actual.state().annotations().ordered()[0];
    let expected = expected.state().annotations().ordered()[0];

    assert_eq!(actual.annotation_type, expected.annotation_type);
    assert_eq!(actual.range, expected.range);
    assert_eq!(actual.selected_text, expected.selected_text);
    assert_eq!(actual.text, expected.text);
}
