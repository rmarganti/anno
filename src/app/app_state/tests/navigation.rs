use super::*;
use crate::keybinds::handler::Action;

#[test]
fn next_annotation_jumps_forward() {
    let mut harness = harness("alpha\nbeta\ngamma");
    create_two_deletions(&mut harness);

    harness.keys("gg]a").assert_cursor(1, 1);
}

#[test]
fn prev_annotation_jumps_backward() {
    let mut harness = harness("alpha\nbeta\ngamma");
    create_two_deletions(&mut harness);

    harness.keys("G[a").assert_cursor(1, 1);
}

#[test]
fn annotation_navigation_stops_at_boundaries() {
    let mut harness = harness("alpha\nbeta\ngamma");
    create_two_deletions(&mut harness);

    harness.keys("gg[a").assert_cursor(0, 0);
    harness.keys("G]a").assert_cursor(2, 0);
}

#[test]
fn next_annotation_matches_panel_order_for_mixed_annotations() {
    let mut panel_harness = harness("alpha\nbeta\ngamma");
    add_mixed_annotations(&mut panel_harness);
    let expected = ordered_panel_positions(&mut panel_harness, 4);

    let mut navigation_harness = harness("alpha\nbeta\ngamma");
    add_mixed_annotations(&mut navigation_harness);

    let anchored = ordered_anchored_positions(navigation_harness.state());
    let mut visited = Vec::with_capacity(anchored.len());
    for _ in 0..anchored.len() {
        navigation_harness.keys("]a");
        let cursor = navigation_harness.state().cursor();
        visited.push((cursor.row, cursor.col));
    }

    assert_eq!(visited, expected);
    assert_eq!(visited, anchored);
}

#[test]
fn prev_annotation_matches_panel_order_for_mixed_annotations() {
    let mut panel_harness = harness("alpha\nbeta\ngamma");
    add_mixed_annotations(&mut panel_harness);
    let anchored = ordered_anchored_positions(panel_harness.state());
    let expected = reverse_panel_positions(&mut panel_harness, anchored.len() - 1);

    let mut navigation_harness = harness("alpha\nbeta\ngamma");
    add_mixed_annotations(&mut navigation_harness);
    navigation_harness.keys("G$");

    let mut visited = Vec::with_capacity(anchored.len());
    for _ in 0..anchored.len() {
        navigation_harness.keys("[a");
        let cursor = navigation_harness.state().cursor();
        visited.push((cursor.row, cursor.col));
    }

    assert_eq!(visited, expected);
    assert_eq!(visited, anchored.into_iter().rev().collect::<Vec<_>>());
}

#[test]
fn counted_normal_motion_repeats_existing_document_navigation() {
    harness("alpha\nbeta\ngamma\ndelta\nepsilon")
        .keys("3j")
        .assert_cursor(3, 0);
}

#[test]
fn counted_visual_motion_repeats_existing_selection_navigation() {
    let mut harness = harness("alpha\nbeta\ngamma\ndelta");

    harness.keys("v2jld").assert_annotation_count(1);

    let annotation = harness.state().annotations().ordered()[0];
    let range = annotation
        .range
        .expect("visual deletion should have a range");
    assert_eq!((range.start.line, range.start.column), (0, 0));
    assert_eq!((range.end.line, range.end.column), (2, 1));
}

#[test]
fn counted_wrapped_motion_uses_display_rows() {
    let mut harness = harness("abcdefghijklmnopqrst");
    harness
        .state_mut()
        .document_view_mut()
        .update_dimensions(5, 24);
    harness
        .state_mut()
        .document_view_mut()
        .handle_action(&Action::ToggleWordWrap);

    harness.keys("4j").assert_cursor(0, 16);
}

#[test]
fn counted_annotation_navigation_repeats_adjacent_jumps() {
    let mut harness = harness("alpha\nbeta\ngamma\ndelta");
    create_three_deletions(&mut harness);

    harness.keys("gg2]a").assert_cursor(2, 2);
    harness.keys("2[a").assert_cursor(0, 0);
}

#[test]
fn repeated_char_search_uses_last_successful_motion() {
    harness("abacadaba").keys("fa;").assert_cursor(0, 4);
}

#[test]
fn reverse_repeated_char_search_uses_opposite_direction() {
    harness("abacadaba").keys("fa;,").assert_cursor(0, 2);
}

#[test]
fn counted_repeated_char_search_uses_repeat_count() {
    harness("abacadaba").keys("fa2;").assert_cursor(0, 6);
}

#[test]
fn repeated_forward_until_skips_adjacent_target() {
    harness("abacadaba").keys("ta;").assert_cursor(0, 3);
}

#[test]
fn repeated_backward_until_skips_adjacent_target() {
    harness("abacadaba").keys("$Ta;").assert_cursor(0, 5);
}

#[test]
fn counted_char_search_targets_nth_occurrence() {
    harness("abacadaba").keys("2fa").assert_cursor(0, 4);
}

#[test]
fn char_search_is_noop_when_target_is_absent() {
    harness("abacadaba").keys("fz").assert_cursor(0, 0);
}

#[test]
fn char_search_does_not_cross_logical_lines() {
    harness("ab\nca").keys("lfa").assert_cursor(0, 1);
}

#[test]
fn repeated_char_search_is_noop_without_prior_success() {
    let mut harness = harness("abacadaba");

    harness.keys(";,").assert_cursor(0, 0);
}

#[test]
fn failed_char_search_does_not_replace_remembered_motion() {
    harness("abacadaba").keys("fa$fa0;").assert_cursor(0, 2);
}

#[test]
fn bare_zero_keeps_line_start_behavior_outside_char_search_targets() {
    harness("ab0c").keys("$0").assert_cursor(0, 0);
}

#[test]
fn char_search_accepts_digit_and_punctuation_targets() {
    harness("1a2;3").keys("f2t;").assert_cursor(0, 2);
}

#[test]
fn backward_char_search_accepts_digit_targets() {
    harness("1a2b3").keys("$F2T1").assert_cursor(0, 1);
}

#[test]
fn repeated_char_search_extends_visual_selection() {
    let mut harness = harness("abacadaba");

    harness.keys("vfa;d").assert_annotation_count(1);

    let annotation = harness.state().annotations().ordered()[0];
    let range = annotation
        .range
        .expect("visual deletion should have a range");
    assert_eq!((range.start.line, range.start.column), (0, 0));
    assert_eq!((range.end.line, range.end.column), (0, 4));
}
