use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::test_harness::AppTestHarness;
use super::test_support::{
    add_mixed_annotations, create_three_deletions, create_two_deletions,
    ordered_anchored_positions, ordered_panel_positions, range, reverse_panel_positions,
};
use super::{ANNOTATION_INSPECT_PAGE_SCROLL_LINES, AppState};
use crate::annotation::types::{Annotation, AnnotationType};
use crate::app::ExitResult;
use crate::keybinds::mode::Mode;
use crate::startup::ExportFormat;

mod annotations;
mod commands;
mod construction;
mod modes;
mod navigation;
mod overlays;
mod panel;
mod search;
mod visual_line;

fn harness(content: &str) -> AppTestHarness {
    AppTestHarness::new(content)
}
