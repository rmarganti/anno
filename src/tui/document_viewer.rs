use ratatui::{
    layout::Rect,
    widgets::{Block, Paragraph},
    Frame,
};

use crate::annotation::types::TextRange;
use crate::document::Document;
use crate::keybinds::handler::Action;
use crate::tui::renderer;
use crate::tui::selection::{self, Selection};
use crate::tui::theme::Theme;
use crate::tui::viewport::{CursorPosition, DisplayLayout, Viewport};

/// Events emitted by [`DocumentViewer`] after processing an action.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DocumentViewerEvent {
    /// The action was consumed with no state change visible to the parent.
    Consumed,
    /// The viewer entered Visual mode; the anchor is set.
    EnteredVisual,
    /// The viewer exited Visual mode (Esc pressed).
    ExitedVisual,
    /// Visual selection was confirmed; the caller should start annotation input.
    /// Contains the selected text range and the selected plain text.
    VisualSelection {
        range: TextRange,
        selected_text: String,
    },
    /// The action was not handled by the viewer.
    Unhandled,
}

/// Owns all document-display state: viewport, display layout, visual anchor, and theme.
///
/// Responsibilities:
/// - Handling movement actions and Visual-mode cursor actions.
/// - Rendering the document area (lines, cursor, selection, annotation highlights).
/// - Providing `take_visual_selection()` to the caller when starting annotation input.
pub struct DocumentViewer {
    /// Viewport state (scroll, cursor, dimensions).
    pub viewport: Viewport,
    /// Display layout mapping document lines → display rows.
    pub display_layout: DisplayLayout,
    /// Anchor position when in Visual mode.
    pub visual_anchor: Option<CursorPosition>,
    /// Centralized theme styles.
    pub theme: Theme,
}

impl DocumentViewer {
    /// Create a new `DocumentViewer` for the given document.
    pub fn new(document: &Document) -> Self {
        let theme = Theme::new();
        let mut viewport = Viewport::new();

        // Initial layout (width 0 until first render sets dimensions).
        let display_layout = DisplayLayout::build(&document.lines, 0, false);

        // Ensure viewport knows about initial dimensions (0×0 is fine; render will update).
        viewport.set_dimensions(0, 0);

        Self {
            viewport,
            display_layout,
            visual_anchor: None,
            theme,
        }
    }

    /// Update dimensions based on the available render area.
    ///
    /// Call this each frame before `render()`.
    #[allow(dead_code)]
    pub fn update(&mut self, area: Rect, max_doc_width: usize, document: &Document) {
        let doc_height = area.height.saturating_sub(1) as usize;
        let doc_width = (area.width as usize).min(max_doc_width);
        let old_width = self.viewport.width;
        self.viewport.set_dimensions(doc_width, doc_height);

        if doc_width != old_width {
            self.rebuild_display_layout(document);
        }
    }

    /// Render the document area.
    pub fn render(
        &self,
        frame: &mut Frame,
        area: Rect,
        document: &Document,
        in_visual_mode: bool,
        annotation_ranges: &[TextRange],
    ) {
        let render_slices = self.viewport.visible_render_slices(&self.display_layout);

        // Compute normalized selection range when in Visual mode.
        let selection = if in_visual_mode {
            self.visual_anchor.map(|anchor| {
                let sel = Selection { anchor };
                sel.range(self.viewport.cursor)
            })
        } else {
            None
        };

        let visible_lines = renderer::prepare_visible_lines_from_slices(
            &render_slices,
            &document.styled_lines,
            &document.lines,
            self.viewport.cursor.row,
            self.viewport.cursor.col,
            &self.theme,
            selection,
            annotation_ranges,
        );

        let doc = Paragraph::new(visible_lines).block(Block::default());
        frame.render_widget(doc, area);
    }

    /// Process a document-viewer action.
    ///
    /// Returns a [`DocumentViewerEvent`] describing what happened. Actions not
    /// related to document viewing are returned as `Unhandled` so the caller can
    /// process them.
    pub fn handle_action(&mut self, action: &Action, document: &Document) -> DocumentViewerEvent {
        match action {
            // -- Movement --
            Action::MoveUp => {
                self.viewport.move_up(&self.display_layout);
                DocumentViewerEvent::Consumed
            }
            Action::MoveDown => {
                self.viewport.move_down(&self.display_layout);
                DocumentViewerEvent::Consumed
            }
            Action::MoveLeft => {
                self.viewport.move_left(&self.display_layout);
                DocumentViewerEvent::Consumed
            }
            Action::MoveRight => {
                self.viewport.move_right(&self.display_layout);
                DocumentViewerEvent::Consumed
            }
            Action::MoveWordForward => {
                let lines: Vec<&str> = document.lines.iter().map(|s| s.as_str()).collect();
                self.viewport
                    .move_word_forward(&lines, &self.display_layout);
                DocumentViewerEvent::Consumed
            }
            Action::MoveWordBackward => {
                let lines: Vec<&str> = document.lines.iter().map(|s| s.as_str()).collect();
                self.viewport
                    .move_word_backward(&lines, &self.display_layout);
                DocumentViewerEvent::Consumed
            }
            Action::MoveLineStart => {
                self.viewport.move_line_start(&self.display_layout);
                DocumentViewerEvent::Consumed
            }
            Action::MoveLineEnd => {
                self.viewport.move_line_end(&self.display_layout);
                DocumentViewerEvent::Consumed
            }
            Action::MoveDocumentTop => {
                self.viewport.move_document_top(&self.display_layout);
                DocumentViewerEvent::Consumed
            }
            Action::MoveDocumentBottom => {
                self.viewport.move_document_bottom(&self.display_layout);
                DocumentViewerEvent::Consumed
            }
            Action::HalfPageDown => {
                self.viewport.half_page_down(&self.display_layout);
                DocumentViewerEvent::Consumed
            }
            Action::HalfPageUp => {
                self.viewport.half_page_up(&self.display_layout);
                DocumentViewerEvent::Consumed
            }
            Action::FullPageDown => {
                self.viewport.full_page_down(&self.display_layout);
                DocumentViewerEvent::Consumed
            }
            Action::FullPageUp => {
                self.viewport.full_page_up(&self.display_layout);
                DocumentViewerEvent::Consumed
            }

            // -- Visual mode: enter --
            Action::EnterVisualMode => {
                self.visual_anchor = Some(self.viewport.cursor);
                DocumentViewerEvent::EnteredVisual
            }

            // -- Visual mode: exit --
            Action::ExitToNormal => {
                self.visual_anchor = None;
                DocumentViewerEvent::ExitedVisual
            }

            // -- Word wrap --
            Action::ToggleWordWrap => {
                self.viewport.toggle_word_wrap();
                self.rebuild_display_layout(document);
                DocumentViewerEvent::Consumed
            }

            // -- Visual selection actions (CreateDeletion / CreateComment / CreateReplacement) --
            Action::CreateDeletion | Action::CreateComment | Action::CreateReplacement => {
                if let Some((range, selected_text)) = self.take_visual_selection(&document.lines) {
                    DocumentViewerEvent::VisualSelection {
                        range,
                        selected_text,
                    }
                } else {
                    // No active selection; treat as consuming the action.
                    DocumentViewerEvent::Consumed
                }
            }

            _ => DocumentViewerEvent::Unhandled,
        }
    }

    /// Extract the current visual selection as a `(TextRange, selected_text)` pair.
    ///
    /// Clears `visual_anchor`. Returns `None` if there is no active selection.
    pub fn take_visual_selection(&mut self, doc_lines: &[String]) -> Option<(TextRange, String)> {
        use crate::annotation::types::{TextPosition, TextRange};

        let anchor = self.visual_anchor.take()?;
        let sel = Selection { anchor };
        let (start, end) = sel.range(self.viewport.cursor);
        let range = TextRange {
            start: TextPosition::from(start),
            end: TextPosition::from(end),
        };
        let text = selection::selected_text(start, end, doc_lines);
        Some((range, text))
    }

    /// Rebuild the display layout after a width or word-wrap change.
    pub fn rebuild_display_layout(&mut self, document: &Document) {
        self.display_layout = DisplayLayout::build(
            &document.lines,
            self.viewport.width,
            self.viewport.word_wrap,
        );
    }
}
