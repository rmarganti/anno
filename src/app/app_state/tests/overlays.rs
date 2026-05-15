use super::*;
use crate::tui::annotation_inspect_overlay::max_scroll_offset;
use crate::tui::help_overlay::max_scroll_offset as help_overlay_max_scroll_offset;

#[test]
fn space_opens_annotation_inspect_from_annotation_list() {
    let mut harness = harness("alpha\nbeta");

    harness
        .keys("vldjvld<Tab>k ")
        .assert_mode(Mode::AnnotationList)
        .assert_annotation_inspect_visible();
}

#[test]
fn escape_dismisses_annotation_inspect_back_to_list_mode() {
    let mut harness = harness("alpha\nbeta");

    harness
        .keys("vld<Tab>k ")
        .assert_annotation_inspect_visible()
        .keys("<Esc>")
        .assert_annotation_inspect_hidden()
        .assert_mode(Mode::AnnotationList)
        .assert_panel_visible();
}

#[test]
fn annotation_inspect_arrow_keys_scroll_without_changing_selection() {
    let mut harness = harness("alpha\nbeta\ngamma");
    harness
        .state_mut()
        .annotations_mut()
        .add(Annotation::comment(
            range(0, 0, 0, 5),
            "alpha".into(),
            (0..24)
                .map(|idx| format!("detail line {idx}"))
                .collect::<Vec<_>>()
                .join("\n"),
        ));

    harness.keys("<Tab>k ");
    let selected_id = harness
        .state()
        .annotation_list_panel()
        .selected_annotation_id();

    harness
        .key(KeyCode::Down)
        .assert_annotation_inspect_visible();
    assert_eq!(harness.state().annotation_inspect_scroll_offset(), 1);
    assert_eq!(
        harness
            .state()
            .annotation_list_panel()
            .selected_annotation_id(),
        selected_id
    );

    harness.key(KeyCode::PageDown);
    assert_eq!(
        harness.state().annotation_inspect_scroll_offset(),
        1 + ANNOTATION_INSPECT_PAGE_SCROLL_LINES
    );
    assert_eq!(
        harness
            .state()
            .annotation_list_panel()
            .selected_annotation_id(),
        selected_id
    );

    harness.key(KeyCode::Up);
    assert_eq!(
        harness.state().annotation_inspect_scroll_offset(),
        ANNOTATION_INSPECT_PAGE_SCROLL_LINES
    );
}

#[test]
fn annotation_inspect_scroll_offset_clamps_at_content_end() {
    let mut harness = harness("alpha\nbeta\ngamma");
    harness
        .state_mut()
        .annotations_mut()
        .add(Annotation::comment(
            range(0, 0, 0, 5),
            "alpha".into(),
            (0..48)
                .map(|idx| format!("detail line {idx}"))
                .collect::<Vec<_>>()
                .join("\n"),
        ));

    harness.keys("<Tab>k ");
    let expected_max = max_scroll_offset(
        harness
            .state()
            .selected_annotation()
            .expect("inspect selection should exist"),
        80,
        23,
    );

    for _ in 0..20 {
        harness.key(KeyCode::PageDown);
    }
    assert_eq!(
        harness.state().annotation_inspect_scroll_offset(),
        expected_max
    );

    harness.key(KeyCode::Up);
    assert_eq!(
        harness.state().annotation_inspect_scroll_offset(),
        expected_max.saturating_sub(1)
    );
}

#[test]
fn mouse_wheel_scrolls_annotation_inspect_without_changing_selection() {
    let mut harness = harness("alpha\nbeta\ngamma");
    harness
        .state_mut()
        .annotations_mut()
        .add(Annotation::comment(
            range(0, 0, 0, 5),
            "alpha".into(),
            (0..24)
                .map(|idx| format!("detail line {idx}"))
                .collect::<Vec<_>>()
                .join("\n"),
        ));

    harness.keys("<Tab>k ");
    let selected_id = harness
        .state()
        .annotation_list_panel()
        .selected_annotation_id();

    assert!(harness.mouse_scroll_down());
    assert_eq!(harness.state().annotation_inspect_scroll_offset(), 1);
    assert_eq!(
        harness
            .state()
            .annotation_list_panel()
            .selected_annotation_id(),
        selected_id
    );

    assert!(harness.mouse_scroll_up());
    assert_eq!(harness.state().annotation_inspect_scroll_offset(), 0);
    assert_eq!(
        harness
            .state()
            .annotation_list_panel()
            .selected_annotation_id(),
        selected_id
    );
}

#[test]
fn shift_h_toggles_help_on_from_normal_mode() {
    harness("hello").keys("H").assert_help_visible();
}

#[test]
fn mouse_wheel_scrolls_help_overlay_without_moving_cursor() {
    let mut harness = harness("hello\nworld");
    harness.keys("jH").assert_help_visible();

    assert!(harness.mouse_scroll_down());
    assert_eq!(harness.state().help_scroll_offset(), 1);
    harness.assert_cursor(1, 0);

    assert!(harness.mouse_scroll_up());
    assert_eq!(harness.state().help_scroll_offset(), 0);
    harness.assert_cursor(1, 0);
}

#[test]
fn configured_help_shortcut_toggles_help_off_when_already_visible() {
    harness("hello")
        .keys("HH")
        .assert_help_hidden()
        .assert_mode(Mode::Normal);
}

#[test]
fn escape_dismisses_help() {
    harness("hello")
        .keys("H<Esc>")
        .assert_help_hidden()
        .assert_mode(Mode::Normal);
}

#[test]
fn q_dismisses_help() {
    harness("hello")
        .keys("Hq")
        .assert_help_hidden()
        .assert_mode(Mode::Normal);
}

#[test]
fn other_keys_are_consumed_while_help_is_visible() {
    let mut harness = harness("hello");
    harness
        .keys("Hj")
        .assert_help_visible()
        .assert_cursor(0, 0)
        .assert_mode(Mode::Normal)
        .assert_not_quit();
    assert_eq!(harness.state().help_scroll_offset(), 1);
}

#[test]
fn help_dismissal_preserves_the_active_mode() {
    let mut harness = harness("hello");
    harness.state_mut().set_mode_for_test(Mode::Visual);
    harness.state_mut().set_help_visible_for_test(true);

    harness
        .keys("q")
        .assert_help_hidden()
        .assert_mode(Mode::Visual);
}

#[test]
fn help_toggle_clears_pending_key_sequence() {
    let mut harness = harness("alpha\nbeta\ngamma");

    harness.keys("jg").assert_cursor(1, 0);
    assert!(harness.state().keybinds().has_pending());

    harness.keys("H").assert_help_visible();
    assert!(!harness.state().keybinds().has_pending());

    harness.keys("q").assert_help_hidden();
    assert!(!harness.state().keybinds().has_pending());

    harness.keys("g").assert_cursor(1, 0);
    assert!(harness.state().keybinds().has_pending());
}

#[test]
fn opening_annotation_inspect_clears_pending_delete_sequence() {
    let mut harness = harness("alpha\nbeta");

    harness.keys("vld<Tab>d");
    assert!(harness.state().keybinds().has_pending());

    harness
        .keys(" ")
        .assert_annotation_inspect_visible()
        .assert_mode(Mode::AnnotationList);
    assert!(!harness.state().keybinds().has_pending());

    harness
        .keys("<Esc>")
        .assert_annotation_inspect_hidden()
        .assert_mode(Mode::AnnotationList);
    assert!(!harness.state().keybinds().has_pending());
}

#[test]
fn opening_and_closing_overlays_clear_pending_count_state() {
    let mut harness = harness("alpha\nbeta\ngamma");

    harness.keys("3H").assert_help_visible();
    assert!(!harness.state().keybinds().has_pending());

    harness.keys("q").assert_help_hidden();
    assert!(!harness.state().keybinds().has_pending());

    harness
        .keys("vld<Tab>2 ")
        .assert_annotation_inspect_visible();
    assert!(!harness.state().keybinds().has_pending());

    harness.keys("<Esc>").assert_annotation_inspect_hidden();
    assert!(!harness.state().keybinds().has_pending());
}

#[test]
fn annotation_inspect_j_k_cycle_annotations_without_closing() {
    let mut harness = harness("alpha\nbeta\ngamma");
    create_two_deletions(&mut harness);

    harness.keys("<Tab> ");
    let first = harness
        .state()
        .annotation_list_panel()
        .selected_annotation_id();

    harness.keys("j").assert_annotation_inspect_visible();
    let second = harness
        .state()
        .annotation_list_panel()
        .selected_annotation_id();

    harness.keys("k").assert_annotation_inspect_visible();
    let back_to_first = harness
        .state()
        .annotation_list_panel()
        .selected_annotation_id();

    assert_ne!(first, second);
    assert_eq!(first, back_to_first);
}

#[test]
fn counted_help_scroll_repeats_existing_scroll_behavior() {
    let mut harness = harness("hello");

    harness.keys("H3j");
    assert_eq!(harness.state().help_scroll_offset(), 3);

    harness.keys("2k");
    assert_eq!(harness.state().help_scroll_offset(), 1);
}

#[test]
fn counted_annotation_inspect_j_k_repeats_selection_movement() {
    let mut harness = harness("alpha\nbeta\ngamma\ndelta");
    create_three_deletions(&mut harness);

    harness.keys("<Tab> ");
    let first = harness
        .state()
        .annotation_list_panel()
        .selected_annotation_id();

    harness.keys("2j").assert_annotation_inspect_visible();
    let third = harness
        .state()
        .annotation_list_panel()
        .selected_annotation_id();

    harness.keys("2k").assert_annotation_inspect_visible();
    let back_to_first = harness
        .state()
        .annotation_list_panel()
        .selected_annotation_id();

    assert_ne!(first, third);
    assert_eq!(first, back_to_first);
}

#[test]
fn counted_annotation_inspect_scroll_repeats_existing_scroll_behavior() {
    let mut harness = harness("alpha\nbeta\ngamma");
    harness
        .state_mut()
        .annotations_mut()
        .add(Annotation::comment(
            range(0, 0, 0, 5),
            "alpha".into(),
            (0..48)
                .map(|idx| format!("detail line {idx}"))
                .collect::<Vec<_>>()
                .join("\n"),
        ));

    harness.keys("<Tab> 3");
    harness.key(KeyCode::Down);
    assert_eq!(harness.state().annotation_inspect_scroll_offset(), 3);

    harness.keys("2");
    harness.key(KeyCode::PageDown);
    assert_eq!(
        harness.state().annotation_inspect_scroll_offset(),
        3 + 2 * ANNOTATION_INSPECT_PAGE_SCROLL_LINES
    );

    harness.keys("4");
    harness.key(KeyCode::Up);
    assert_eq!(
        harness.state().annotation_inspect_scroll_offset(),
        2 * ANNOTATION_INSPECT_PAGE_SCROLL_LINES - 1
    );
}

#[test]
fn enter_in_annotation_inspect_jumps_to_selected_annotation() {
    let mut harness = harness("alpha\nbeta\ngamma");
    create_two_deletions(&mut harness);

    harness.keys("<Tab>j ");
    let expected = harness
        .state()
        .selected_annotation_range()
        .expect("inspect selection should be anchored")
        .start;

    harness
        .keys("<Enter>")
        .assert_annotation_inspect_visible()
        .assert_cursor(expected.line, expected.column);
}

#[test]
fn enter_in_annotation_inspect_is_noop_for_global_comments() {
    let mut harness = harness("alpha\nbeta\ngamma");
    add_mixed_annotations(&mut harness);

    harness
        .keys("<Tab>kjjjj ")
        .assert_annotation_inspect_visible();
    assert!(harness.state().selected_annotation_range().is_none());

    let cursor = harness.state().cursor();
    harness
        .keys("<Enter>")
        .assert_annotation_inspect_visible()
        .assert_cursor(cursor.row, cursor.col);
}

#[test]
fn dd_is_disabled_while_annotation_inspect_is_open() {
    let mut harness = harness("alpha\nbeta");

    harness
        .keys("vld<Tab> dd")
        .assert_annotation_count(1)
        .assert_no_confirm_dialog()
        .assert_annotation_inspect_visible();
}

#[test]
fn mouse_wheel_is_ignored_while_confirm_dialog_is_open() {
    let mut harness = harness("alpha\nbeta");

    harness.keys("vld<Tab>dd").assert_has_confirm_dialog();

    assert!(harness.mouse_scroll_down());
    harness
        .assert_has_confirm_dialog()
        .assert_annotation_count(1)
        .assert_mode(Mode::AnnotationList);
}

#[test]
fn mouse_wheel_is_ignored_in_input_taking_modes() {
    for mode in [Mode::Insert, Mode::Command, Mode::Search] {
        let mut harness = harness("alpha\nbeta");
        harness.state_mut().set_mode_for_test(mode);

        assert!(harness.mouse_scroll_down());
        harness.assert_mode(mode).assert_cursor(0, 0);
    }
}

#[test]
fn narrow_terminal_closes_annotation_inspect_and_returns_to_normal() {
    let mut harness = harness("alpha\nbeta");

    harness
        .keys("vld<Tab> ")
        .assert_mode(Mode::AnnotationList)
        .assert_annotation_inspect_visible();

    harness
        .set_panel_available(false)
        .assert_mode(Mode::Normal)
        .assert_annotation_inspect_hidden();
}

#[test]
fn narrow_terminal_does_not_offer_annotation_inspect_entry() {
    let mut harness = harness("alpha\nbeta");

    harness
        .set_panel_available(false)
        .keys("vld<Tab> ")
        .assert_mode(Mode::Normal)
        .assert_annotation_inspect_hidden();

    assert!(harness.state().is_panel_hidden_due_to_width());
}

#[test]
fn j_k_adjust_help_scroll_offset() {
    let mut harness = harness("hello");
    harness.keys("Hjjj");
    assert_eq!(harness.state().help_scroll_offset(), 3);

    harness.keys("k");
    assert_eq!(harness.state().help_scroll_offset(), 2);
}

#[test]
fn help_scroll_offset_saturates_at_zero() {
    let mut harness = harness("hello");
    harness.keys("Hkk");
    assert_eq!(harness.state().help_scroll_offset(), 0);
}

#[test]
fn help_scroll_offset_resets_on_reopen() {
    let mut harness = harness("hello");
    harness.keys("Hjjj");
    assert_eq!(harness.state().help_scroll_offset(), 3);

    harness.keys("H");
    harness.assert_help_hidden();

    harness.keys("H");
    harness.assert_help_visible();
    assert_eq!(harness.state().help_scroll_offset(), 0);
}

#[test]
fn help_scroll_offset_clamps_at_content_end_and_recovers_immediately() {
    let mut harness = harness("hello");
    let expected_max = help_overlay_max_scroll_offset(80, 23);

    harness.keys("H");
    for _ in 0..200 {
        harness.key(KeyCode::Down);
    }
    assert_eq!(harness.state().help_scroll_offset(), expected_max);

    harness.key(KeyCode::Up);
    assert_eq!(
        harness.state().help_scroll_offset(),
        expected_max.saturating_sub(1)
    );
}

#[test]
fn mouse_wheel_at_help_boundary_does_not_fall_through() {
    let mut harness = harness("alpha\nbeta");
    let expected_max = help_overlay_max_scroll_offset(80, 23);

    harness.keys("jH");
    for _ in 0..200 {
        assert!(harness.mouse_scroll_down());
    }

    assert_eq!(harness.state().help_scroll_offset(), expected_max);
    harness.assert_cursor(1, 0);

    assert!(harness.mouse_scroll_down());
    assert_eq!(harness.state().help_scroll_offset(), expected_max);
    harness.assert_cursor(1, 0);
}

#[test]
fn dismiss_keys_work_regardless_of_scroll_position() {
    harness("hello")
        .keys("Hjjj<Esc>")
        .assert_help_hidden()
        .assert_mode(Mode::Normal);

    harness("hello")
        .keys("Hjjjq")
        .assert_help_hidden()
        .assert_mode(Mode::Normal);

    harness("hello")
        .keys("HjjjH")
        .assert_help_hidden()
        .assert_mode(Mode::Normal);
}

#[test]
fn mouse_wheel_at_annotation_inspect_boundary_does_not_fall_through() {
    let mut harness = harness("alpha\nbeta\ngamma");
    harness
        .state_mut()
        .annotations_mut()
        .add(Annotation::comment(
            range(0, 0, 0, 5),
            "alpha".into(),
            (0..48)
                .map(|idx| format!("detail line {idx}"))
                .collect::<Vec<_>>()
                .join("\n"),
        ));

    harness.keys("<Tab>k ");
    let expected_max = max_scroll_offset(
        harness
            .state()
            .selected_annotation()
            .expect("inspect selection should exist"),
        80,
        23,
    );
    let selected_id = harness
        .state()
        .annotation_list_panel()
        .selected_annotation_id();

    for _ in 0..200 {
        assert!(harness.mouse_scroll_down());
    }

    assert_eq!(
        harness.state().annotation_inspect_scroll_offset(),
        expected_max
    );
    assert_eq!(
        harness
            .state()
            .annotation_list_panel()
            .selected_annotation_id(),
        selected_id
    );

    assert!(harness.mouse_scroll_down());
    assert_eq!(
        harness.state().annotation_inspect_scroll_offset(),
        expected_max
    );
    assert_eq!(
        harness
            .state()
            .annotation_list_panel()
            .selected_annotation_id(),
        selected_id
    );
}
