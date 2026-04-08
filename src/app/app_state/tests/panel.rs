use super::*;
use crate::tui::annotation_list_panel::{PANEL_WIDTH, visible_content_height};
use ratatui::layout::Rect;

#[test]
fn tab_focuses_annotation_list_mode() {
    let mut harness = harness("first\nsecond");
    harness.keys("vld<Tab>").assert_mode(Mode::AnnotationList);
}

#[test]
fn tab_keeps_panel_hidden_from_annotation_list_mode_when_terminal_is_narrow() {
    let mut harness = harness("first\nsecond");
    harness
        .set_panel_available(false)
        .keys("vld<Tab>")
        .assert_mode(Mode::Normal)
        .assert_panel_visible();

    assert!(harness.state().is_panel_hidden_due_to_width());
}

#[test]
fn narrow_terminal_forces_annotation_list_mode_back_to_normal() {
    let mut harness = harness("first\nsecond");
    harness.keys("vld<Tab>").assert_mode(Mode::AnnotationList);

    harness.set_panel_available(false).assert_mode(Mode::Normal);
    assert!(harness.state().is_panel_hidden_due_to_width());
}

#[test]
fn escape_hides_annotation_list_panel() {
    let mut harness = harness("first\nsecond");
    harness
        .keys("vld<Tab><Esc>")
        .assert_mode(Mode::Normal)
        .assert_panel_hidden();
}

#[test]
fn tab_toggles_annotation_panel_focus_without_hiding_visible_panel() {
    harness("hello")
        .keys("vld<Tab>")
        .assert_panel_visible()
        .assert_mode(Mode::AnnotationList)
        .keys("<Tab>")
        .assert_panel_visible()
        .assert_mode(Mode::Normal);
}

#[test]
fn escape_hides_visible_unfocused_panel_from_normal_mode() {
    harness("hello")
        .assert_panel_visible()
        .assert_mode(Mode::Normal)
        .keys("<Esc>")
        .assert_panel_hidden()
        .assert_mode(Mode::Normal);
}

#[test]
fn tab_reopens_hidden_panel_and_focuses_it() {
    harness("hello")
        .keys("<Esc>")
        .assert_panel_hidden()
        .keys("<Tab>")
        .assert_panel_visible()
        .assert_mode(Mode::AnnotationList);
}

#[test]
fn annotation_list_navigation_updates_selection() {
    let mut harness = harness("alpha\nbeta\ngamma");
    create_two_deletions(&mut harness);

    harness.keys("<Tab>");
    let first = harness
        .state()
        .annotation_list_panel()
        .selected_annotation_id();

    harness.keys("j");
    let second = harness
        .state()
        .annotation_list_panel()
        .selected_annotation_id();

    harness.keys("k");
    let back_to_first = harness
        .state()
        .annotation_list_panel()
        .selected_annotation_id();

    assert_ne!(first, second);
    assert_eq!(first, back_to_first);
}

#[test]
fn counted_annotation_list_navigation_repeats_selection_movement() {
    let mut harness = harness("alpha\nbeta\ngamma\ndelta");
    create_three_deletions(&mut harness);

    harness.keys("<Tab>2j");
    let third = harness
        .state()
        .annotation_list_panel()
        .selected_annotation_id();

    harness.keys("2k");
    let first = harness
        .state()
        .annotation_list_panel()
        .selected_annotation_id();

    assert_eq!(third, Some(harness.state().annotations().ordered()[2].id));
    assert_eq!(first, Some(harness.state().annotations().ordered()[0].id));
}

#[test]
fn counted_annotation_list_enter_reuses_selected_jump_behavior() {
    let mut harness = harness("alpha\nbeta\ngamma");
    create_two_deletions(&mut harness);

    harness.keys("gg<Tab>j2<Enter>").assert_cursor(1, 1);
}

#[test]
fn counted_annotation_list_delete_is_rejected() {
    harness("alpha\nbeta")
        .keys("vld<Tab>4dd")
        .assert_annotation_count(1)
        .assert_no_confirm_dialog();
}

#[test]
fn selected_annotation_range_returns_selected_panel_annotation_range() {
    let mut harness = harness("alpha\nbeta\ngamma");
    create_two_deletions(&mut harness);

    harness.keys("<Tab>j");

    let expected_range = harness.state().annotations().ordered()[1]
        .range
        .expect("selected panel annotation should have a range");

    let selected_range = harness
        .state()
        .selected_annotation_range()
        .expect("selected panel annotation should have a range");

    assert_eq!(selected_range, expected_range);
}

#[test]
fn enter_in_annotation_list_jumps_to_selected_annotation() {
    let mut harness = harness("alpha\nbeta\ngamma");
    create_two_deletions(&mut harness);

    harness.keys("gg<Tab>j<Enter>").assert_cursor(1, 1);
}

#[test]
fn tab_initializes_selection_to_first_annotation() {
    let mut harness = harness("alpha\nbeta\ngamma");
    create_two_deletions(&mut harness);

    harness.keys("<Tab>");

    let expected_id = harness.state().annotations().ordered()[0].id;
    assert_eq!(
        harness
            .state()
            .annotation_list_panel()
            .selected_annotation_id(),
        Some(expected_id)
    );
}

#[test]
fn first_j_after_tab_selects_second_annotation() {
    let mut harness = harness("alpha\nbeta\ngamma");
    create_two_deletions(&mut harness);

    harness.keys("<Tab>j");

    let expected_id = harness.state().annotations().ordered()[1].id;
    assert_eq!(
        harness
            .state()
            .annotation_list_panel()
            .selected_annotation_id(),
        Some(expected_id)
    );
}

#[test]
fn confirm_dialog_can_delete_annotation_from_list() {
    let mut harness = harness("alpha\nbeta");
    harness.keys("vld<Tab>dd").assert_has_confirm_dialog();
    harness
        .keys("y")
        .assert_annotation_count(0)
        .assert_no_confirm_dialog();
}

#[test]
fn confirm_dialog_cancel_keeps_annotation() {
    harness("alpha\nbeta")
        .keys("vld<Tab>ddn")
        .assert_annotation_count(1)
        .assert_no_confirm_dialog();
}

#[test]
fn enter_after_first_focus_jumps_to_first_annotation() {
    let mut harness = harness("alpha\nbeta\ngamma");
    create_two_deletions(&mut harness);

    let expected = harness.state().annotations().ordered()[0]
        .range
        .expect("first annotation should be anchored")
        .start;

    harness
        .keys("gg<Tab><Enter>")
        .assert_cursor(expected.line, expected.column);
}

#[test]
fn space_after_first_focus_opens_annotation_inspect() {
    let mut harness = harness("alpha\nbeta");

    harness
        .keys("vld<Tab> ")
        .assert_mode(Mode::AnnotationList)
        .assert_annotation_inspect_visible();
}

#[test]
fn confirm_dialog_delete_keeps_selection_on_same_list_index() {
    let mut harness = harness("alpha\nbeta\ngamma\ndelta");
    create_three_deletions(&mut harness);

    let ordered = harness.state().annotations().ordered();
    let expected_id = ordered[2].id;
    let _ = ordered;

    harness
        .keys("<Tab>kjdd")
        .assert_has_confirm_dialog()
        .keys("y")
        .assert_annotation_count(2)
        .assert_no_confirm_dialog();

    assert_eq!(
        harness
            .state()
            .annotation_list_panel()
            .selected_annotation_id(),
        Some(expected_id)
    );
}

#[test]
fn annotation_list_navigation_uses_layout_derived_visible_height() {
    let mut harness = harness("a\nb\nc\nd\ne\nf\ng");
    harness
        .state_mut()
        .set_annotation_list_visible_height(visible_content_height(Rect::new(
            0,
            0,
            PANEL_WIDTH,
            4,
        )));
    harness.keys("vldjvldjvldjvldjvld");

    harness.keys("<Tab>jjj");

    assert_eq!(harness.state().annotation_list_visible_height(), 2);
    assert_eq!(harness.state().annotation_list_panel().scroll_offset, 2);
}
