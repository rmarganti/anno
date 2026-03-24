use ratatui::{
    layout::{Constraint, Flex, Layout},
    text::Line,
    widgets::{Block, Paragraph},
    Frame,
};

use crate::annotation::types::TextRange;
use crate::highlight::StyledSpan;
use crate::keybinds::handler::Action;
use crate::tui::renderer;
use crate::tui::selection::{self, Selection};
use crate::tui::theme::Theme;
use crate::tui::viewport::{CursorPosition, DisplayLayout, Viewport};

const MAX_DOC_WIDTH: u16 = 120;

/// Manages the document content display: viewport, cursor movement, word wrap,
/// visual selection, and rendering of the main document area.
pub struct DocumentView {
    /// Viewport state (scroll, cursor, dimensions).
    viewport: Viewport,
    /// Display layout mapping document lines → display rows.
    display_layout: DisplayLayout,
    /// Anchor position when in Visual mode.
    visual_anchor: Option<CursorPosition>,
    /// Plain-text document lines.
    doc_lines: Vec<String>,
    /// Highlighted document lines (for rendering with syntax highlighting).
    styled_lines: Vec<Vec<StyledSpan>>,
}

impl DocumentView {
    pub fn new(doc_lines: Vec<String>, styled_lines: Vec<Vec<StyledSpan>>) -> Self {
        let mut viewport = Viewport::new();
        let display_layout = DisplayLayout::build(&doc_lines, 0, false);
        viewport.set_dimensions(0, 0);

        Self {
            viewport,
            display_layout,
            visual_anchor: None,
            doc_lines,
            styled_lines,
        }
    }

    /// Current cursor position in document coordinates.
    pub fn cursor(&self) -> CursorPosition {
        self.viewport.cursor
    }

    /// Whether word wrap is enabled.
    pub fn word_wrap(&self) -> bool {
        self.viewport.word_wrap
    }

    /// Handle a movement or visual-mode action. Returns `true` if consumed.
    pub fn handle_action(&mut self, action: &Action) -> bool {
        match action {
            Action::MoveUp => self.viewport.move_up(&self.display_layout),
            Action::MoveDown => self.viewport.move_down(&self.display_layout),
            Action::MoveLeft => self.viewport.move_left(&self.display_layout),
            Action::MoveRight => self.viewport.move_right(&self.display_layout),
            Action::MoveWordForward => {
                let lines: Vec<&str> = self.doc_lines.iter().map(|s| s.as_str()).collect();
                self.viewport
                    .move_word_forward(&lines, &self.display_layout);
            }
            Action::MoveWordBackward => {
                let lines: Vec<&str> = self.doc_lines.iter().map(|s| s.as_str()).collect();
                self.viewport
                    .move_word_backward(&lines, &self.display_layout);
            }
            Action::MoveLineStart => self.viewport.move_line_start(&self.display_layout),
            Action::MoveLineEnd => self.viewport.move_line_end(&self.display_layout),
            Action::MoveDocumentTop => self.viewport.move_document_top(&self.display_layout),
            Action::MoveDocumentBottom => {
                self.viewport.move_document_bottom(&self.display_layout)
            }
            Action::HalfPageDown => self.viewport.half_page_down(&self.display_layout),
            Action::HalfPageUp => self.viewport.half_page_up(&self.display_layout),
            Action::FullPageDown => self.viewport.full_page_down(&self.display_layout),
            Action::FullPageUp => self.viewport.full_page_up(&self.display_layout),

            Action::EnterVisualMode => {
                self.visual_anchor = Some(self.viewport.cursor);
            }

            Action::ToggleWordWrap => {
                self.viewport.toggle_word_wrap();
                self.rebuild_display_layout();
            }

            _ => return false,
        }
        true
    }

    /// Extract the current visual selection as a `TextRange` and selected text.
    /// Returns `None` if there is no active visual anchor.
    pub fn take_visual_selection(&mut self) -> Option<(TextRange, String)> {
        let anchor = self.visual_anchor.take()?;
        let sel = Selection { anchor };
        let (start, end) = sel.range(self.viewport.cursor);
        let range = TextRange {
            start: crate::annotation::types::TextPosition {
                line: start.row,
                column: start.col,
            },
            end: crate::annotation::types::TextPosition {
                line: end.row,
                column: end.col,
            },
        };
        let text = selection::selected_text(start, end, &self.doc_lines);
        Some((range, text))
    }

    /// Clear the visual anchor (e.g. when exiting Visual mode).
    pub fn clear_visual(&mut self) {
        self.visual_anchor = None;
    }

    /// Render the document area into the frame.
    /// `is_visual` should be true when in Visual mode (to show selection highlighting).
    pub fn render(
        &mut self,
        frame: &mut Frame,
        area: ratatui::layout::Rect,
        theme: &Theme,
        is_visual: bool,
        annotation_ranges: &[TextRange],
    ) {
        // Update viewport dimensions (account for status row handled by caller).
        let doc_height = area.height as usize;
        let doc_width = (area.width as usize).min(MAX_DOC_WIDTH as usize);
        let old_width = self.viewport.width;
        self.viewport.set_dimensions(doc_width, doc_height);

        if doc_width != old_width {
            self.rebuild_display_layout();
        }

        // Cap the main content width at MAX_DOC_WIDTH columns and center it.
        let main_area = Layout::horizontal([Constraint::Max(MAX_DOC_WIDTH)])
            .flex(Flex::Center)
            .areas::<1>(area)[0];

        let render_slices = self.viewport.visible_render_slices(&self.display_layout);

        let selection = if is_visual {
            self.visual_anchor.map(|anchor| {
                let sel = Selection { anchor };
                sel.range(self.viewport.cursor)
            })
        } else {
            None
        };

        let visible_lines: Vec<Line<'static>> = renderer::prepare_visible_lines_from_slices(
            &render_slices,
            &self.styled_lines,
            &self.doc_lines,
            self.viewport.cursor.row,
            self.viewport.cursor.col,
            theme,
            selection,
            annotation_ranges,
        );

        let doc = Paragraph::new(visible_lines).block(Block::default());
        frame.render_widget(doc, main_area);
    }

    /// Returns `true` if the terminal is too small to render the UI.
    pub fn is_too_small(&self) -> bool {
        self.viewport.is_too_small()
    }

    fn rebuild_display_layout(&mut self) {
        self.display_layout = DisplayLayout::build(
            &self.doc_lines,
            self.viewport.width,
            self.viewport.word_wrap,
        );
    }
}
