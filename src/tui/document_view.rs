use ratatui::{
    Frame,
    layout::{Constraint, Flex, Layout},
    text::Line,
    widgets::{Block, Paragraph},
};

use crate::annotation::types::TextRange;
use crate::highlight::StyledSpan;
use crate::keybinds::handler::Action;
use crate::tui::renderer;
use crate::tui::selection::{self, Selection};
use crate::tui::theme::UiTheme;
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
            Action::MoveWordEnd => {
                let lines: Vec<&str> = self.doc_lines.iter().map(|s| s.as_str()).collect();
                self.viewport.move_word_end(&lines, &self.display_layout);
            }
            Action::MoveLineStart => self.viewport.move_line_start(&self.display_layout),
            Action::MoveLineEnd => self.viewport.move_line_end(&self.display_layout),
            Action::MoveDocumentTop => self.viewport.move_document_top(&self.display_layout),
            Action::MoveDocumentBottom => self.viewport.move_document_bottom(&self.display_layout),
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

    /// Move the cursor to the given document row and column, clamping to valid bounds.
    pub fn set_cursor(&mut self, row: usize, col: usize) {
        let max_row = self.doc_lines.len().saturating_sub(1);
        let clamped_row = row.min(max_row);
        self.viewport.cursor.row = clamped_row;
        let max_col = self.doc_lines[clamped_row]
            .chars()
            .count()
            .saturating_sub(1);
        self.viewport.cursor.col = col.min(max_col);
        self.viewport.ensure_cursor_visible(&self.display_layout);
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
        theme: &UiTheme,
        is_visual: bool,
        annotation_ranges: &[TextRange],
        selected_annotation_range: Option<&TextRange>,
    ) {
        frame.render_widget(Block::default().style(theme.document), area);

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

        let visible_lines: Vec<Line<'static>> =
            renderer::prepare_visible_lines_from_slices(&renderer::PrepareVisibleLinesParams {
                slices: &render_slices,
                styled_lines: &self.styled_lines,
                plain_lines: &self.doc_lines,
                cursor_row: self.viewport.cursor.row,
                cursor_col: self.viewport.cursor.col,
                theme,
                selection,
                annotation_ranges,
                selected_annotation_range,
            });

        let doc = Paragraph::new(visible_lines)
            .style(theme.document)
            .block(Block::default().style(theme.document));
        frame.render_widget(doc, main_area);
    }

    /// Update the viewport dimensions (e.g. from terminal size) so that
    /// `is_too_small()` works correctly before the first `render()` call.
    pub fn update_dimensions(&mut self, width: usize, height: usize) {
        let old_width = self.viewport.width;
        self.viewport.set_dimensions(width, height);
        if width != old_width {
            self.rebuild_display_layout();
        }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::highlight::StyledSpan;
    use crate::keybinds::handler::Action;
    use crate::tui::viewport::CursorPosition;
    use ratatui::{Terminal, backend::TestBackend, layout::Rect, style::Color};

    // ── Helpers ───────────────────────────────────────────────────────

    /// Build a `DocumentView` from plain text lines with no syntax highlighting.
    fn make_view(lines: &[&str]) -> DocumentView {
        let doc_lines: Vec<String> = lines.iter().map(|s| s.to_string()).collect();
        let styled_lines: Vec<Vec<StyledSpan>> = doc_lines
            .iter()
            .map(|l| {
                if l.is_empty() {
                    vec![]
                } else {
                    vec![StyledSpan::plain(l.clone())]
                }
            })
            .collect();
        let mut view = DocumentView::new(doc_lines, styled_lines);
        // Give it a non-zero size so movement works.
        view.update_dimensions(80, 24);
        view
    }

    fn pos(row: usize, col: usize) -> CursorPosition {
        CursorPosition { row, col }
    }

    // ── Initial state ─────────────────────────────────────────────────

    #[test]
    fn initial_cursor_at_origin() {
        let view = make_view(&["hello"]);
        assert_eq!(view.cursor(), pos(0, 0));
    }

    #[test]
    fn initial_word_wrap_disabled() {
        let view = make_view(&["hello"]);
        assert!(!view.word_wrap());
    }

    // ── Movement ──────────────────────────────────────────────────────

    #[test]
    fn move_down_advances_row() {
        let mut view = make_view(&["first", "second"]);
        let consumed = view.handle_action(&Action::MoveDown);
        assert!(consumed);
        assert_eq!(view.cursor().row, 1);
    }

    #[test]
    fn move_down_stops_at_last_line() {
        let mut view = make_view(&["only"]);
        view.handle_action(&Action::MoveDown);
        assert_eq!(view.cursor().row, 0);
    }

    #[test]
    fn move_up_decrements_row() {
        let mut view = make_view(&["first", "second"]);
        view.handle_action(&Action::MoveDown);
        let consumed = view.handle_action(&Action::MoveUp);
        assert!(consumed);
        assert_eq!(view.cursor().row, 0);
    }

    #[test]
    fn move_right_advances_col() {
        let mut view = make_view(&["hello"]);
        let consumed = view.handle_action(&Action::MoveRight);
        assert!(consumed);
        assert_eq!(view.cursor().col, 1);
    }

    #[test]
    fn move_left_decrements_col() {
        let mut view = make_view(&["hello"]);
        view.handle_action(&Action::MoveRight);
        view.handle_action(&Action::MoveRight);
        let consumed = view.handle_action(&Action::MoveLeft);
        assert!(consumed);
        assert_eq!(view.cursor().col, 1);
    }

    #[test]
    fn move_line_end_goes_to_last_col() {
        let mut view = make_view(&["hello"]);
        let consumed = view.handle_action(&Action::MoveLineEnd);
        assert!(consumed);
        assert_eq!(view.cursor().col, 4); // "hello" has 5 chars, last col = 4
    }

    #[test]
    fn move_line_start_goes_to_col_zero() {
        let mut view = make_view(&["hello"]);
        view.handle_action(&Action::MoveLineEnd);
        let consumed = view.handle_action(&Action::MoveLineStart);
        assert!(consumed);
        assert_eq!(view.cursor().col, 0);
    }

    #[test]
    fn move_document_top_goes_to_row_zero() {
        let mut view = make_view(&["a", "b", "c"]);
        view.handle_action(&Action::MoveDown);
        view.handle_action(&Action::MoveDown);
        let consumed = view.handle_action(&Action::MoveDocumentTop);
        assert!(consumed);
        assert_eq!(view.cursor().row, 0);
    }

    #[test]
    fn render_fills_background_across_full_area_width() {
        let backend = TestBackend::new(160, 12);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut view = make_view(&["hello"]);
        let theme = UiTheme::default();

        terminal
            .draw(|frame| {
                view.render(
                    frame,
                    Rect {
                        x: 0,
                        y: 0,
                        width: 160,
                        height: 12,
                    },
                    &theme,
                    false,
                    &[],
                    None,
                );
            })
            .unwrap();

        let buffer = terminal.backend().buffer().clone();
        let right_edge_cell = buffer.cell((159, 0)).unwrap();
        assert_eq!(right_edge_cell.style().bg, theme.document.bg,);
        assert_ne!(right_edge_cell.style().bg, Some(Color::Reset));
    }

    #[test]
    fn move_document_bottom_goes_to_last_row() {
        let mut view = make_view(&["a", "b", "c"]);
        let consumed = view.handle_action(&Action::MoveDocumentBottom);
        assert!(consumed);
        assert_eq!(view.cursor().row, 2);
    }

    // ── Unhandled actions return false ────────────────────────────────

    #[test]
    fn unhandled_action_returns_false() {
        let mut view = make_view(&["hello"]);
        let consumed = view.handle_action(&Action::CreateDeletion);
        assert!(!consumed);
    }

    // ── Visual selection ──────────────────────────────────────────────

    #[test]
    fn enter_visual_mode_sets_anchor() {
        let mut view = make_view(&["hello world"]);
        let consumed = view.handle_action(&Action::EnterVisualMode);
        assert!(consumed);
        // Move right then take selection — should be "h".
        view.handle_action(&Action::MoveRight);
        let result = view.take_visual_selection();
        assert!(result.is_some());
        let (range, text) = result.unwrap();
        assert_eq!(range.start.line, 0);
        assert_eq!(range.start.column, 0);
        assert_eq!(text, "he"); // anchor=0, cursor=1 → chars 0..=1
    }

    #[test]
    fn take_visual_selection_none_without_anchor() {
        let mut view = make_view(&["hello"]);
        let result = view.take_visual_selection();
        assert!(result.is_none());
    }

    #[test]
    fn take_visual_selection_clears_anchor() {
        let mut view = make_view(&["hello"]);
        view.handle_action(&Action::EnterVisualMode);
        view.handle_action(&Action::MoveRight);
        view.take_visual_selection();
        // Second call should return None (anchor was consumed).
        assert!(view.take_visual_selection().is_none());
    }

    #[test]
    fn clear_visual_removes_anchor() {
        let mut view = make_view(&["hello"]);
        view.handle_action(&Action::EnterVisualMode);
        view.clear_visual();
        assert!(view.take_visual_selection().is_none());
    }

    // ── Word wrap toggle ──────────────────────────────────────────────

    #[test]
    fn toggle_word_wrap_enables() {
        let mut view = make_view(&["hello world"]);
        view.handle_action(&Action::ToggleWordWrap);
        assert!(view.word_wrap());
    }

    #[test]
    fn toggle_word_wrap_disables_after_second_toggle() {
        let mut view = make_view(&["hello world"]);
        view.handle_action(&Action::ToggleWordWrap);
        view.handle_action(&Action::ToggleWordWrap);
        assert!(!view.word_wrap());
    }

    // ── is_too_small ──────────────────────────────────────────────────

    #[test]
    fn too_small_when_dimensions_zero() {
        // DocumentView::new() starts at 0×0 before update_dimensions is called.
        let mut raw_view = {
            let doc_lines = vec!["hello".to_string()];
            let styled_lines = vec![vec![StyledSpan::plain("hello")]];
            DocumentView::new(doc_lines, styled_lines)
        };
        assert!(raw_view.is_too_small());
        raw_view.update_dimensions(80, 24);
        assert!(!raw_view.is_too_small());
    }

    #[test]
    fn not_too_small_with_adequate_dimensions() {
        let view = make_view(&["hello"]);
        assert!(!view.is_too_small());
    }
}
