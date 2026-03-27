use crate::annotation::store::AnnotationStore;
use crate::annotation::types::{Annotation, TextPosition, TextRange};
use crate::keybinds::mode::Mode;
use crate::tui::document_view::DocumentView;
use crate::tui::input_box::{InputBox, InputBoxEvent};

use crossterm::event::KeyEvent;

/// Tracks the kind of annotation being created via the input box.
#[derive(Debug, Clone)]
enum PendingAnnotation {
    /// Comment on a selection — stores the selection range and original text.
    Comment {
        range: TextRange,
        selected_text: String,
    },
    /// Replacement for a selection — stores the selection range and original text.
    Replacement {
        range: TextRange,
        selected_text: String,
    },
    /// Insertion at a cursor position.
    Insertion { position: TextPosition },
    /// Global comment (not anchored to text).
    GlobalComment,
}

/// The result of an annotation controller action, telling the caller what mode
/// to switch to (if any).
pub enum AnnotationAction {
    /// Switch to the given mode.
    SwitchMode(Mode),
    /// No mode change needed.
    None,
}

/// Manages the annotation-creation state machine: pending annotation tracking,
/// input box lifecycle, and committing annotations to the store.
pub struct AnnotationController {
    /// Active input box (shown in Insert mode for annotation text entry).
    input_box: Option<InputBox<'static>>,
    /// The pending annotation being created (set when entering Insert mode).
    pending_annotation: Option<PendingAnnotation>,
}

impl AnnotationController {
    pub fn new() -> Self {
        Self {
            input_box: None,
            pending_annotation: None,
        }
    }

    /// Returns a reference to the active input box, if any.
    pub fn input_box(&self) -> Option<&InputBox<'static>> {
        self.input_box.as_ref()
    }

    /// Cancel any in-progress annotation creation and clear the input box.
    pub fn cancel(&mut self) {
        self.input_box = None;
        self.pending_annotation = None;
    }

    /// Create a Deletion annotation from the current visual selection.
    pub fn create_deletion(
        &mut self,
        document_view: &mut DocumentView,
        annotations: &mut AnnotationStore,
    ) -> AnnotationAction {
        if let Some((range, text)) = document_view.take_visual_selection() {
            annotations.add(Annotation::deletion(range, text));
        }
        AnnotationAction::SwitchMode(Mode::Normal)
    }

    /// Begin input for a Comment or Replacement from visual mode.
    pub fn start_input_for_visual_annotation(
        &mut self,
        kind: &str,
        document_view: &mut DocumentView,
    ) -> AnnotationAction {
        if let Some((range, selected_text)) = document_view.take_visual_selection() {
            let pending = if kind == "Comment" {
                PendingAnnotation::Comment {
                    range,
                    selected_text,
                }
            } else {
                PendingAnnotation::Replacement {
                    range,
                    selected_text,
                }
            };
            self.pending_annotation = Some(pending);
            self.input_box = Some(InputBox::new(kind));
            AnnotationAction::SwitchMode(Mode::Insert)
        } else {
            AnnotationAction::SwitchMode(Mode::Normal)
        }
    }

    /// Begin input for an Insertion annotation at the current cursor position.
    pub fn start_insertion(&mut self, document_view: &DocumentView) -> AnnotationAction {
        let cursor = document_view.cursor();
        let position = TextPosition {
            line: cursor.row,
            column: cursor.col,
        };
        self.pending_annotation = Some(PendingAnnotation::Insertion { position });
        self.input_box = Some(InputBox::new("Insertion"));
        AnnotationAction::SwitchMode(Mode::Insert)
    }

    /// Begin input for a GlobalComment annotation.
    pub fn start_global_comment(&mut self) -> AnnotationAction {
        self.pending_annotation = Some(PendingAnnotation::GlobalComment);
        self.input_box = Some(InputBox::new("Global Comment"));
        AnnotationAction::SwitchMode(Mode::Insert)
    }

    /// Forward a key event to the active input box. Returns the resulting mode
    /// change (if any).
    pub fn handle_input_key(
        &mut self,
        key_event: KeyEvent,
        annotations: &mut AnnotationStore,
    ) -> AnnotationAction {
        if let Some(ref mut ib) = self.input_box {
            match ib.handle_key(key_event) {
                InputBoxEvent::Confirm => {
                    self.confirm_input(annotations);
                    AnnotationAction::SwitchMode(Mode::Normal)
                }
                InputBoxEvent::Cancel => {
                    self.cancel();
                    AnnotationAction::SwitchMode(Mode::Normal)
                }
                InputBoxEvent::Consumed => AnnotationAction::None,
            }
        } else {
            AnnotationAction::None
        }
    }

    /// Confirm input and create the pending annotation.
    fn confirm_input(&mut self, annotations: &mut AnnotationStore) {
        let text = self
            .input_box
            .as_ref()
            .map(|ib| ib.text())
            .unwrap_or_default();

        if let Some(pending) = self.pending_annotation.take()
            && !text.is_empty()
        {
            let annotation = match pending {
                PendingAnnotation::Comment {
                    range,
                    selected_text,
                } => Annotation::comment(range, selected_text, text),
                PendingAnnotation::Replacement {
                    range,
                    selected_text,
                } => Annotation::replacement(range, selected_text, text),
                PendingAnnotation::Insertion { position } => Annotation::insertion(position, text),
                PendingAnnotation::GlobalComment => Annotation::global_comment(text),
            };
            annotations.add(annotation);
        }

        self.input_box = None;
    }
}
