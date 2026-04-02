use super::*;

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
