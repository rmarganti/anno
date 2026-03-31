use super::*;

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
fn question_mark_toggles_help_on_from_normal_mode() {
    harness("hello").keys("?").assert_help_visible();
}

#[test]
fn configured_help_shortcut_toggles_help_off_when_already_visible() {
    harness("hello")
        .keys("??")
        .assert_help_hidden()
        .assert_mode(Mode::Normal);
}

#[test]
fn escape_dismisses_help() {
    harness("hello")
        .keys("?<Esc>")
        .assert_help_hidden()
        .assert_mode(Mode::Normal);
}

#[test]
fn q_dismisses_help() {
    harness("hello")
        .keys("?q")
        .assert_help_hidden()
        .assert_mode(Mode::Normal);
}

#[test]
fn other_keys_are_consumed_while_help_is_visible() {
    let mut harness = harness("hello");
    harness
        .keys("?j")
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

    harness.keys("?").assert_help_visible();
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
    harness.keys("?jjj");
    assert_eq!(harness.state().help_scroll_offset(), 3);

    harness.keys("k");
    assert_eq!(harness.state().help_scroll_offset(), 2);
}

#[test]
fn help_scroll_offset_saturates_at_zero() {
    let mut harness = harness("hello");
    harness.keys("?kk");
    assert_eq!(harness.state().help_scroll_offset(), 0);
}

#[test]
fn help_scroll_offset_resets_on_reopen() {
    let mut harness = harness("hello");
    harness.keys("?jjj");
    assert_eq!(harness.state().help_scroll_offset(), 3);

    harness.keys("?");
    harness.assert_help_hidden();

    harness.keys("?");
    harness.assert_help_visible();
    assert_eq!(harness.state().help_scroll_offset(), 0);
}

#[test]
fn dismiss_keys_work_regardless_of_scroll_position() {
    harness("hello")
        .keys("?jjj<Esc>")
        .assert_help_hidden()
        .assert_mode(Mode::Normal);

    harness("hello")
        .keys("?jjjq")
        .assert_help_hidden()
        .assert_mode(Mode::Normal);

    harness("hello")
        .keys("?jjj?")
        .assert_help_hidden()
        .assert_mode(Mode::Normal);
}
