use super::AppState;
use super::test_harness::AppTestHarness;
use crate::annotation::types::{Annotation, TextPosition, TextRange};

pub(super) fn create_two_deletions(harness: &mut AppTestHarness) {
    harness.keys("vldjvld").assert_annotation_count(2);
}

pub(super) fn create_three_deletions(harness: &mut AppTestHarness) {
    harness.keys("vldjvldjvld").assert_annotation_count(3);
}

pub(super) fn range(sl: usize, sc: usize, el: usize, ec: usize) -> TextRange {
    TextRange {
        start: TextPosition {
            line: sl,
            column: sc,
        },
        end: TextPosition {
            line: el,
            column: ec,
        },
    }
}

pub(super) fn add_mixed_annotations(harness: &mut AppTestHarness) {
    let annotations = [
        Annotation::deletion(range(0, 1, 0, 4), "lph".into()),
        Annotation::insertion(TextPosition { line: 1, column: 2 }, "inserted".into()),
        Annotation::comment(range(1, 2, 1, 5), "ta ".into(), "note".into()),
        Annotation::deletion(range(2, 0, 2, 2), "ga".into()),
        Annotation::global_comment("overall".into()),
    ];

    for annotation in annotations {
        harness.state_mut().annotations_mut().add(annotation);
    }
}

pub(super) fn ordered_anchored_positions(state: &AppState) -> Vec<(usize, usize)> {
    state
        .annotations()
        .ordered()
        .into_iter()
        .filter_map(|annotation| {
            annotation
                .range
                .map(|range| (range.start.line, range.start.column))
        })
        .collect()
}

pub(super) fn ordered_panel_positions(
    harness: &mut AppTestHarness,
    steps: usize,
) -> Vec<(usize, usize)> {
    harness.keys("<Tab>k");

    let mut positions = Vec::with_capacity(steps + 1);
    positions.push(
        harness
            .state()
            .selected_annotation_range()
            .map(|range| (range.start.line, range.start.column))
            .expect("panel should start on the first anchored annotation"),
    );

    for _ in 0..steps {
        harness.keys("j");
        let position = harness
            .state()
            .selected_annotation_range()
            .map(|range| (range.start.line, range.start.column));
        if let Some(position) = position {
            positions.push(position);
        }
    }

    positions
}

pub(super) fn reverse_panel_positions(
    harness: &mut AppTestHarness,
    steps: usize,
) -> Vec<(usize, usize)> {
    harness.keys("<Tab>k");
    for _ in 0..steps {
        harness.keys("j");
    }

    let mut positions = Vec::with_capacity(steps + 1);
    positions.push(
        harness
            .state()
            .selected_annotation_range()
            .map(|range| (range.start.line, range.start.column))
            .expect("panel should land on an anchored annotation before reversing"),
    );

    for _ in 0..steps {
        harness.keys("k");
        positions.push(
            harness
                .state()
                .selected_annotation_range()
                .map(|range| (range.start.line, range.start.column))
                .expect("reverse panel step should stay on anchored annotations"),
        );
    }

    positions
}
