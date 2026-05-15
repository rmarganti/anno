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
