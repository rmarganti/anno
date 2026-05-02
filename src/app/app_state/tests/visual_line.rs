use super::*;

#[test]
fn normal_can_enter_visual_line_mode_and_escape_back() {
    harness("first\nsecond")
        .keys("V")
        .assert_mode(Mode::VisualLine)
        .keys("<Esc>")
        .assert_mode(Mode::Normal);
}

#[test]
fn visual_line_deletion_creates_full_line_range() {
    let mut harness = harness("first\nsecond\nthird");
    harness.keys("Vjd");
    let annotation = harness.state().annotations().ordered()[0];
    let range = annotation.range.expect("deletion should have a range");

    assert_eq!(annotation.annotation_type, AnnotationType::Deletion);
    assert_eq!(annotation.selected_text, "first\nsecond\n");
    assert_eq!(annotation.text, "");
    assert_eq!((range.start.line, range.start.column), (0, 0));
    assert_eq!(
        (range.end.line, range.end.column),
        (1, "second".chars().count() - 1)
    );
}

#[test]
fn visual_line_comment_and_replacement_use_linewise_selection() {
    let mut comment = harness("first\nsecond\nthird");
    comment.keys("Vjcnote<C-s>");
    let comment_annotation = comment.state().annotations().ordered()[0];
    assert_eq!(comment.state().mode(), Mode::Normal);
    assert_eq!(comment_annotation.annotation_type, AnnotationType::Comment);
    assert_eq!(comment_annotation.selected_text, "first\nsecond\n");
    assert_eq!(comment_annotation.text, "note");

    let mut replacement = harness("first\nsecond\nthird");
    replacement.keys("Vjrnew<C-s>");
    let replacement_annotation = replacement.state().annotations().ordered()[0];
    assert_eq!(replacement.state().mode(), Mode::Normal);
    assert_eq!(
        replacement_annotation.annotation_type,
        AnnotationType::Replacement
    );
    assert_eq!(replacement_annotation.selected_text, "first\nsecond\n");
    assert_eq!(replacement_annotation.text, "new");
}

#[test]
fn visual_line_search_confirm_preserves_mode_and_selection() {
    let mut harness = harness("alpha beta\ngamma beta");

    harness.keys("V/beta<Enter>d").assert_mode(Mode::Normal);

    let annotation = harness.state().annotations().ordered()[0];
    assert_eq!(annotation.annotation_type, AnnotationType::Deletion);
    assert_eq!(annotation.selected_text, "alpha beta\n");
}

#[test]
fn counted_search_repeat_works_from_visual_line_mode() {
    harness("x target one\nx target two\nx target three")
        .keys("V/target<Enter>2n")
        .assert_mode(Mode::VisualLine)
        .assert_cursor(2, 2);
}
