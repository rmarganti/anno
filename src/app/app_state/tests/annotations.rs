use super::*;

#[test]
fn visual_deletion_creates_deletion_annotation() {
    let mut harness = harness("hello");
    harness.keys("vld");
    let annotation = harness.state().annotations().ordered()[0];

    assert_eq!(annotation.annotation_type, AnnotationType::Deletion);
    assert_eq!(annotation.selected_text, "he");
    assert_eq!(annotation.text, "");
}

#[test]
fn visual_comment_opens_input_and_commits_comment() {
    let mut harness = harness("hello");
    harness.keys("vlcnote<C-s>");
    let annotation = harness.state().annotations().ordered()[0];

    assert_eq!(harness.state().mode(), Mode::Normal);
    assert_eq!(annotation.annotation_type, AnnotationType::Comment);
    assert_eq!(annotation.selected_text, "he");
    assert_eq!(annotation.text, "note");
}

#[test]
fn visual_replacement_opens_input_and_commits_replacement() {
    let mut harness = harness("hello");
    harness.keys("vlrnew<C-s>");
    let annotation = harness.state().annotations().ordered()[0];

    assert_eq!(harness.state().mode(), Mode::Normal);
    assert_eq!(annotation.annotation_type, AnnotationType::Replacement);
    assert_eq!(annotation.selected_text, "he");
    assert_eq!(annotation.text, "new");
}

#[test]
fn insertion_creates_annotation_at_cursor() {
    let mut harness = harness("hello\nworld");
    harness.keys("jliadd<C-s>");
    let annotation = harness.state().annotations().ordered()[0];
    let range = annotation.range.expect("insertion should have a range");

    assert_eq!(annotation.annotation_type, AnnotationType::Insertion);
    assert_eq!(annotation.text, "add");
    assert_eq!((range.start.line, range.start.column), (1, 1));
    assert_eq!((range.end.line, range.end.column), (1, 1));
}

#[test]
fn global_comment_creates_unanchored_annotation() {
    let mut harness = harness("hello");
    harness.keys("gcoverall<C-s>");
    let annotation = harness.state().annotations().ordered()[0];

    assert_eq!(annotation.annotation_type, AnnotationType::GlobalComment);
    assert!(annotation.range.is_none());
    assert_eq!(annotation.text, "overall");
}

#[test]
fn escape_during_input_cancels_annotation_creation() {
    harness("hello")
        .keys("vlcnote<Esc>")
        .assert_mode(Mode::Normal)
        .assert_annotation_count(0);
}

#[test]
fn counted_global_comment_does_not_create_multiple_annotations() {
    harness("hello")
        .keys("2gcoverall<C-s>")
        .assert_mode(Mode::Normal)
        .assert_annotation_count(1);
}
