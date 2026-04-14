use ratatui::{
    Frame,
    layout::{Constraint, Flex, Layout},
    text::{Line, Span},
    widgets::{Block, Paragraph},
};

use crate::annotation::types::{AnnotationIndicator, AnnotationType, TextRange};
use crate::highlight::StyledSpan;
use crate::keybinds::handler::{Action, CharSearchDirection, RepeatDirection, SearchDirection};
use crate::startup::LineNumberMode;
use crate::tui::renderer;
use crate::tui::selection::{self, Selection};
use crate::tui::theme::UiTheme;
use crate::tui::viewport::{CharSearch, CursorPosition, DisplayLayout, Viewport};

const MAX_DOC_WIDTH: u16 = 120;
const ANNOTATION_GUTTER_WIDTH: usize = 1;
const GUTTER_SEPARATOR_WIDTH: usize = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct CharSearchState {
    target: char,
    direction: CharSearchDirection,
    until: bool,
}

/// Manages the document content display state: viewport, cursor movement,
/// word wrap, and visual selection.
pub struct DocumentViewState {
    /// Viewport state (scroll, cursor, dimensions).
    viewport: Viewport,
    /// Display layout mapping document lines → display rows.
    display_layout: DisplayLayout,
    /// Anchor position when in Visual mode.
    visual_anchor: Option<CursorPosition>,
    /// Last successful f/F/t/T search, used by ; and , repeat motions.
    last_char_search: Option<CharSearchState>,
    /// Plain-text document lines.
    doc_lines: Vec<String>,
    /// Highlighted document lines (for rendering with syntax highlighting).
    styled_lines: Vec<Vec<StyledSpan>>,
    /// Configured line number mode for gutter rendering.
    line_number_mode: LineNumberMode,
}

impl DocumentViewState {
    pub fn new(
        doc_lines: Vec<String>,
        styled_lines: Vec<Vec<StyledSpan>>,
        line_number_mode: LineNumberMode,
    ) -> Self {
        let mut viewport = Viewport::new();
        let display_layout = DisplayLayout::build(&doc_lines, 0, false);
        viewport.set_dimensions(0, 0);

        Self {
            viewport,
            display_layout,
            visual_anchor: None,
            last_char_search: None,
            doc_lines,
            styled_lines,
            line_number_mode,
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

    pub fn line_number_mode(&self) -> LineNumberMode {
        self.line_number_mode
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
            Action::MoveToChar {
                target,
                direction,
                until,
                count,
            } => {
                let state = CharSearchState {
                    target: *target,
                    direction: *direction,
                    until: *until,
                };
                let moved = self.execute_char_search(state, *count, false);
                if moved {
                    self.last_char_search = Some(state);
                }
            }
            Action::RepeatLastCharSearch { direction, count } => {
                let Some(search) = self.last_char_search else {
                    return true;
                };

                let repeated = CharSearchState {
                    direction: match direction {
                        RepeatDirection::Same => search.direction,
                        RepeatDirection::Opposite => search.direction.reversed(),
                    },
                    ..search
                };
                self.execute_char_search(repeated, *count, true);
            }
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

    fn execute_char_search(
        &mut self,
        search: CharSearchState,
        count: usize,
        is_repeat: bool,
    ) -> bool {
        let line = self
            .doc_lines
            .get(self.viewport.cursor.row)
            .map(String::as_str)
            .unwrap_or("");
        self.viewport.move_to_char(
            line,
            &self.display_layout,
            CharSearch {
                target: search.target,
                direction: search.direction,
                until: search.until,
                is_repeat,
            },
            count,
        )
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

    /// Search for the next/previous occurrence of `pattern` and jump to its start.
    /// Returns `true` when a match is found.
    pub fn search_text(&mut self, pattern: &str, direction: SearchDirection) -> bool {
        if pattern.is_empty() || self.doc_lines.is_empty() {
            return false;
        }

        let start = self.viewport.cursor;
        let found = match direction {
            SearchDirection::Forward => self.search_forward(pattern, start),
            SearchDirection::Backward => self.search_backward(pattern, start),
        };

        if let Some((row, col)) = found {
            self.set_cursor(row, col);
            return true;
        }

        false
    }

    fn search_forward(&self, pattern: &str, start: CursorPosition) -> Option<(usize, usize)> {
        let total_lines = self.doc_lines.len();
        for line_offset in 0..=total_lines {
            let row = (start.row + line_offset) % total_lines;
            let line = self.doc_lines[row].as_str();
            let line_len = line.chars().count();
            let (start_col, end_col) = if line_offset == 0 {
                (start.col.saturating_add(1), line_len)
            } else if row == start.row {
                (0, start.col)
            } else {
                (0, line_len)
            };

            if let Some(col) = Self::find_forward_in_line(line, pattern, start_col, end_col) {
                return Some((row, col));
            }
        }

        None
    }

    fn search_backward(&self, pattern: &str, start: CursorPosition) -> Option<(usize, usize)> {
        let total_lines = self.doc_lines.len();
        for line_offset in 0..=total_lines {
            let row = (start.row + total_lines - (line_offset % total_lines)) % total_lines;
            let line = self.doc_lines[row].as_str();
            let line_len = line.chars().count();
            let (start_col, end_col) = if line_offset == 0 {
                (0, start.col)
            } else if row == start.row {
                (start.col.saturating_add(1), line_len)
            } else {
                (0, line_len)
            };

            if let Some(col) = Self::find_backward_in_line(line, pattern, start_col, end_col) {
                return Some((row, col));
            }
        }

        None
    }

    fn find_forward_in_line(
        line: &str,
        pattern: &str,
        start_col: usize,
        end_col: usize,
    ) -> Option<usize> {
        if start_col > end_col {
            return None;
        }

        let start_byte = Self::char_to_byte_idx(line, start_col)?;
        let end_byte = Self::char_to_byte_idx(line, end_col)?;
        let haystack = line.get(start_byte..end_byte)?;
        let match_byte = haystack.find(pattern)?;

        Some(line[..start_byte + match_byte].chars().count())
    }

    fn find_backward_in_line(
        line: &str,
        pattern: &str,
        start_col: usize,
        end_col: usize,
    ) -> Option<usize> {
        if start_col > end_col {
            return None;
        }

        let start_byte = Self::char_to_byte_idx(line, start_col)?;
        let end_byte = Self::char_to_byte_idx(line, end_col)?;
        let haystack = line.get(start_byte..end_byte)?;
        let match_byte = haystack.rfind(pattern)?;

        Some(line[..start_byte + match_byte].chars().count())
    }

    fn char_to_byte_idx(line: &str, char_idx: usize) -> Option<usize> {
        line.char_indices()
            .map(|(idx, _)| idx)
            .chain(std::iter::once(line.len()))
            .nth(char_idx)
    }

    /// Clear the visual anchor (e.g. when exiting Visual mode).
    pub fn clear_visual(&mut self) {
        self.visual_anchor = None;
    }

    /// Update the viewport dimensions (e.g. from terminal size) so that
    /// `is_too_small()` works correctly before the first `render()` call.
    pub fn update_dimensions(&mut self, width: usize, height: usize) {
        let width = Self::text_width(width, self.total_doc_lines());
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

    fn total_doc_lines(&self) -> usize {
        self.doc_lines.len().max(1)
    }

    fn line_number_gutter_width(total_doc_lines: usize) -> usize {
        total_doc_lines.max(1).to_string().len()
    }

    fn gutter_width(total_doc_lines: usize) -> usize {
        ANNOTATION_GUTTER_WIDTH
            + Self::line_number_gutter_width(total_doc_lines)
            + GUTTER_SEPARATOR_WIDTH
    }

    fn main_area_width(width: usize) -> usize {
        width.min(MAX_DOC_WIDTH as usize)
    }

    fn text_width(width: usize, total_doc_lines: usize) -> usize {
        Self::main_area_width(width).saturating_sub(Self::gutter_width(total_doc_lines))
    }

    fn prepare_gutter_lines(
        render_slices: &[crate::tui::viewport::RenderSlice],
        annotation_indicators: &[AnnotationIndicator],
        total_doc_lines: usize,
        cursor_row: usize,
        _line_number_mode: LineNumberMode,
        theme: &UiTheme,
    ) -> Vec<Line<'static>> {
        let line_number_width = Self::line_number_gutter_width(total_doc_lines);
        let separator = " ".to_string();

        render_slices
            .iter()
            .zip(Self::compute_gutter_annotation_types(
                render_slices,
                annotation_indicators,
            ))
            .map(|(slice, annotation_type)| {
                let line_number_style = if slice.doc_row == cursor_row {
                    theme.current_line_number
                } else {
                    theme.line_number
                };
                let symbol = annotation_type
                    .map(|annotation_type| {
                        Span::styled(
                            "▌",
                            theme
                                .document
                                .fg(theme.annotation_type_color(&annotation_type)),
                        )
                    })
                    .unwrap_or_else(|| Span::styled(" ", theme.document));

                Line::from(vec![
                    symbol,
                    Span::styled(" ".repeat(line_number_width), line_number_style),
                    Span::styled(separator.clone(), line_number_style),
                ])
            })
            .collect()
    }

    fn compute_gutter_annotation_types(
        render_slices: &[crate::tui::viewport::RenderSlice],
        annotation_indicators: &[AnnotationIndicator],
    ) -> Vec<Option<AnnotationType>> {
        render_slices
            .iter()
            .map(|slice| Self::gutter_annotation_type(slice.doc_row, annotation_indicators))
            .collect()
    }

    fn gutter_annotation_type(
        doc_row: usize,
        annotation_indicators: &[AnnotationIndicator],
    ) -> Option<AnnotationType> {
        annotation_indicators
            .iter()
            .filter(|indicator| {
                indicator.annotation_type != AnnotationType::GlobalComment
                    && doc_row >= indicator.range.start.line
                    && doc_row <= indicator.range.end.line
            })
            .min_by_key(|indicator| indicator.annotation_type.priority())
            .map(|indicator| indicator.annotation_type)
    }
}

/// Render the document area into the frame.
/// `is_visual` should be true when in Visual mode (to show selection highlighting).
pub fn render_document_view(
    frame: &mut Frame,
    area: ratatui::layout::Rect,
    state: &DocumentViewState,
    theme: &UiTheme,
    is_visual: bool,
    annotation_indicators: &[AnnotationIndicator],
    selected_annotation_range: Option<&TextRange>,
) {
    frame.render_widget(Block::default().style(theme.document), area);

    // Cap the main content width at MAX_DOC_WIDTH columns and center it.
    let main_area = Layout::horizontal([Constraint::Max(MAX_DOC_WIDTH)])
        .flex(Flex::Center)
        .areas::<1>(area)[0];
    let gutter_width = DocumentViewState::gutter_width(state.total_doc_lines()) as u16;
    let [gutter_area, text_area] =
        Layout::horizontal([Constraint::Length(gutter_width), Constraint::Min(0)]).areas(main_area);

    let render_slices = state.viewport.visible_render_slices(&state.display_layout);

    let selection = if is_visual {
        state.visual_anchor.map(|anchor| {
            let sel = Selection { anchor };
            sel.range(state.viewport.cursor)
        })
    } else {
        None
    };

    let annotation_ranges: Vec<TextRange> = annotation_indicators
        .iter()
        .map(|indicator| indicator.range)
        .collect();

    let visible_lines: Vec<Line<'static>> =
        renderer::prepare_visible_lines_from_slices(&renderer::PrepareVisibleLinesParams {
            slices: &render_slices,
            styled_lines: &state.styled_lines,
            plain_lines: &state.doc_lines,
            cursor_row: state.viewport.cursor.row,
            cursor_col: state.viewport.cursor.col,
            theme,
            selection,
            annotation_ranges: &annotation_ranges,
            selected_annotation_range,
        });

    let gutter_lines = DocumentViewState::prepare_gutter_lines(
        &render_slices,
        annotation_indicators,
        state.total_doc_lines(),
        state.viewport.cursor.row,
        state.line_number_mode(),
        theme,
    );

    let gutter = Paragraph::new(gutter_lines)
        .style(theme.document)
        .block(Block::default().style(theme.document));
    frame.render_widget(gutter, gutter_area);

    let doc = Paragraph::new(visible_lines)
        .style(theme.document)
        .block(Block::default().style(theme.document));
    frame.render_widget(doc, text_area);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::annotation::types::{AnnotationType, TextPosition};
    use crate::highlight::StyledSpan;
    use crate::keybinds::handler::{Action, CharSearchDirection, RepeatDirection};
    use crate::startup::LineNumberMode;
    use crate::tui::viewport::{CursorPosition, RenderSlice};
    use ratatui::{Terminal, backend::TestBackend, layout::Rect, style::Color};

    // ── Helpers ───────────────────────────────────────────────────────

    /// Build a `DocumentViewState` from plain text lines with no syntax highlighting.
    fn make_view(lines: &[&str]) -> DocumentViewState {
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
        let mut view = DocumentViewState::new(doc_lines, styled_lines, LineNumberMode::Relative);
        // Give it a non-zero size so movement works.
        view.update_dimensions(80, 24);
        view
    }

    fn pos(row: usize, col: usize) -> CursorPosition {
        CursorPosition { row, col }
    }

    fn indicator(
        annotation_type: AnnotationType,
        start_line: usize,
        end_line: usize,
    ) -> AnnotationIndicator {
        AnnotationIndicator {
            range: TextRange {
                start: TextPosition {
                    line: start_line,
                    column: 0,
                },
                end: TextPosition {
                    line: end_line,
                    column: 0,
                },
            },
            annotation_type,
        }
    }

    fn render_buffer(
        view: &mut DocumentViewState,
        width: u16,
        height: u16,
        annotation_indicators: &[AnnotationIndicator],
    ) -> ratatui::buffer::Buffer {
        let backend = TestBackend::new(width, height);
        let mut terminal = Terminal::new(backend).unwrap();
        let theme = UiTheme::default();

        terminal
            .draw(|frame| {
                view.update_dimensions(width as usize, height as usize);
                render_document_view(
                    frame,
                    Rect {
                        x: 0,
                        y: 0,
                        width,
                        height,
                    },
                    view,
                    &theme,
                    false,
                    annotation_indicators,
                    None,
                );
            })
            .unwrap();

        terminal.backend().buffer().clone()
    }

    fn slice(doc_row: usize) -> RenderSlice {
        RenderSlice {
            doc_row,
            start_col: 0,
            end_col: 1,
        }
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

    #[test]
    fn char_to_byte_idx_handles_unicode_and_end_of_line() {
        let line = "aé🙂z";

        assert_eq!(DocumentViewState::char_to_byte_idx(line, 0), Some(0));
        assert_eq!(DocumentViewState::char_to_byte_idx(line, 1), Some(1));
        assert_eq!(DocumentViewState::char_to_byte_idx(line, 2), Some(3));
        assert_eq!(DocumentViewState::char_to_byte_idx(line, 3), Some(7));
        assert_eq!(
            DocumentViewState::char_to_byte_idx(line, 4),
            Some(line.len())
        );
        assert_eq!(DocumentViewState::char_to_byte_idx(line, 5), None);
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
    fn move_to_char_searches_within_logical_line_when_wrapped() {
        let mut view = make_view(&["abcd efgh ijkl mnop"]);
        view.handle_action(&Action::ToggleWordWrap);
        view.update_dimensions(8, 24);

        let consumed = view.handle_action(&Action::MoveToChar {
            target: 'm',
            direction: CharSearchDirection::Forward,
            until: false,
            count: 1,
        });

        assert!(consumed);
        assert_eq!(view.cursor(), pos(0, 15));
    }

    #[test]
    fn move_to_char_in_visual_mode_extends_selection() {
        let mut view = make_view(&["alpha beta gamma"]);

        assert!(view.handle_action(&Action::EnterVisualMode));
        assert!(view.handle_action(&Action::MoveToChar {
            target: 'b',
            direction: CharSearchDirection::Forward,
            until: false,
            count: 1,
        }));

        let (range, text) = view.take_visual_selection().unwrap();
        assert_eq!(range.start.line, 0);
        assert_eq!(range.start.column, 0);
        assert_eq!(range.end.line, 0);
        assert_eq!(range.end.column, 6);
        assert_eq!(text, "alpha b");
    }

    #[test]
    fn repeat_last_char_search_uses_same_or_opposite_direction() {
        let mut view = make_view(&["abcabc"]);

        assert!(view.handle_action(&Action::MoveToChar {
            target: 'c',
            direction: CharSearchDirection::Forward,
            until: false,
            count: 1,
        }));
        assert_eq!(view.cursor(), pos(0, 2));

        assert!(view.handle_action(&Action::RepeatLastCharSearch {
            direction: RepeatDirection::Same,
            count: 1,
        }));
        assert_eq!(view.cursor(), pos(0, 5));

        assert!(view.handle_action(&Action::RepeatLastCharSearch {
            direction: RepeatDirection::Opposite,
            count: 1,
        }));
        assert_eq!(view.cursor(), pos(0, 2));
    }

    #[test]
    fn render_fills_background_across_full_area_width() {
        let mut view = make_view(&["hello"]);
        let theme = UiTheme::default();

        let buffer = render_buffer(&mut view, 160, 12, &[]);
        let right_edge_cell = buffer.cell((159, 0)).unwrap();
        assert_eq!(right_edge_cell.style().bg, theme.document.bg,);
        assert_ne!(right_edge_cell.style().bg, Some(Color::Reset));
    }

    #[test]
    fn render_draws_colored_gutter_indicator_for_annotated_rows() {
        let mut view = make_view(&["first", "second", "third"]);
        let theme = UiTheme::default();
        let buffer = render_buffer(
            &mut view,
            80,
            5,
            &[indicator(AnnotationType::Comment, 1, 1)],
        );

        let gutter_x = 0;
        assert_eq!(buffer.cell((gutter_x, 0)).unwrap().symbol(), " ");
        assert_eq!(buffer.cell((gutter_x, 1)).unwrap().symbol(), "▌");
        assert_eq!(
            buffer.cell((gutter_x, 1)).unwrap().style().fg,
            Some(theme.annotation_type_color(&AnnotationType::Comment))
        );
        assert_eq!(buffer.cell((gutter_x, 2)).unwrap().symbol(), " ");
    }

    #[test]
    fn render_uses_highest_priority_annotation_type_in_gutter() {
        let mut view = make_view(&["first", "second"]);
        let theme = UiTheme::default();
        let buffer = render_buffer(
            &mut view,
            80,
            5,
            &[
                indicator(AnnotationType::Comment, 1, 1),
                indicator(AnnotationType::Deletion, 1, 1),
            ],
        );

        let gutter_cell = buffer.cell((0, 1)).unwrap();
        assert_eq!(gutter_cell.symbol(), "▌");
        assert_eq!(
            gutter_cell.style().fg,
            Some(theme.annotation_type_color(&AnnotationType::Deletion))
        );
    }

    #[test]
    fn render_draws_gutter_indicator_for_all_wrapped_rows_of_annotated_line() {
        let mut view = make_view(&["abcdefghijklmnopqrstuvwxyz"]);
        view.handle_action(&Action::ToggleWordWrap);

        let buffer = render_buffer(
            &mut view,
            10,
            5,
            &[indicator(AnnotationType::Insertion, 0, 0)],
        );

        assert_eq!(buffer.cell((0, 0)).unwrap().symbol(), "▌");
        assert_eq!(buffer.cell((0, 1)).unwrap().symbol(), "▌");
        assert_eq!(buffer.cell((0, 2)).unwrap().symbol(), "▌");
    }

    #[test]
    fn compute_gutter_annotation_types_marks_every_line_in_single_range() {
        let gutter_types = DocumentViewState::compute_gutter_annotation_types(
            &[slice(0), slice(1), slice(2)],
            &[indicator(AnnotationType::Comment, 0, 2)],
        );

        assert_eq!(
            gutter_types,
            vec![
                Some(AnnotationType::Comment),
                Some(AnnotationType::Comment),
                Some(AnnotationType::Comment)
            ]
        );
    }

    #[test]
    fn compute_gutter_annotation_types_uses_highest_priority_type_per_row() {
        let gutter_types = DocumentViewState::compute_gutter_annotation_types(
            &[slice(1)],
            &[
                indicator(AnnotationType::Comment, 1, 1),
                indicator(AnnotationType::Insertion, 1, 1),
                indicator(AnnotationType::Replacement, 1, 1),
                indicator(AnnotationType::Deletion, 1, 1),
            ],
        );

        assert_eq!(gutter_types, vec![Some(AnnotationType::Deletion)]);
    }

    #[test]
    fn compute_gutter_annotation_types_leaves_unannotated_rows_empty() {
        let gutter_types = DocumentViewState::compute_gutter_annotation_types(
            &[slice(0), slice(1), slice(2)],
            &[indicator(AnnotationType::Comment, 1, 1)],
        );

        assert_eq!(
            gutter_types,
            vec![None, Some(AnnotationType::Comment), None]
        );
    }

    #[test]
    fn compute_gutter_annotation_types_marks_every_line_in_multiline_range() {
        let gutter_types = DocumentViewState::compute_gutter_annotation_types(
            &[slice(2), slice(3), slice(4)],
            &[indicator(AnnotationType::Replacement, 2, 4)],
        );

        assert_eq!(
            gutter_types,
            vec![
                Some(AnnotationType::Replacement),
                Some(AnnotationType::Replacement),
                Some(AnnotationType::Replacement)
            ]
        );
    }

    #[test]
    fn compute_gutter_annotation_types_marks_all_wrapped_rows_for_same_doc_line() {
        let gutter_types = DocumentViewState::compute_gutter_annotation_types(
            &[slice(0), slice(0), slice(0)],
            &[indicator(AnnotationType::Insertion, 0, 0)],
        );

        assert_eq!(
            gutter_types,
            vec![
                Some(AnnotationType::Insertion),
                Some(AnnotationType::Insertion),
                Some(AnnotationType::Insertion)
            ]
        );
    }

    #[test]
    fn compute_gutter_annotation_types_ignores_global_comments() {
        let gutter_types = DocumentViewState::compute_gutter_annotation_types(
            &[slice(1)],
            &[indicator(AnnotationType::GlobalComment, 1, 1)],
        );

        assert_eq!(gutter_types, vec![None]);
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
        // DocumentViewState::new() starts at 0×0 before update_dimensions is called.
        let mut raw_view = {
            let doc_lines = vec!["hello".to_string()];
            let styled_lines = vec![vec![StyledSpan::plain("hello")]];
            DocumentViewState::new(doc_lines, styled_lines, LineNumberMode::Relative)
        };
        assert!(raw_view.is_too_small());
        raw_view.update_dimensions(80, 24);
        assert!(!raw_view.is_too_small());
    }

    #[test]
    fn document_view_state_keeps_configured_line_number_mode() {
        let view = DocumentViewState::new(
            vec!["hello".to_string()],
            vec![vec![StyledSpan::plain("hello")]],
            LineNumberMode::Absolute,
        );

        assert_eq!(view.line_number_mode(), LineNumberMode::Absolute);
    }

    #[test]
    fn not_too_small_with_adequate_dimensions() {
        let view = make_view(&["hello"]);
        assert!(!view.is_too_small());
    }

    #[test]
    fn gutter_width_includes_annotation_strip_line_numbers_and_separator() {
        assert_eq!(DocumentViewState::gutter_width(0), 3);
        assert_eq!(DocumentViewState::gutter_width(9), 3);
        assert_eq!(DocumentViewState::gutter_width(10), 4);
        assert_eq!(DocumentViewState::gutter_width(100), 5);
    }

    #[test]
    fn update_dimensions_caps_text_width_after_gutter_budget() {
        let mut view = make_view(&[&"x".repeat(200)]);

        view.update_dimensions(160, 5);

        assert_eq!(view.viewport.width, 117);
    }

    #[test]
    fn update_dimensions_wraps_to_rendered_text_width_after_gutter() {
        let mut view = make_view(&["abcdefghij"]);
        view.handle_action(&Action::ToggleWordWrap);

        view.update_dimensions(10, 5);

        assert_eq!(view.viewport.width, 7);
        assert_eq!(view.display_layout.rows.len(), 2);
        assert_eq!(view.display_layout.rows[0].start_col, 0);
        assert_eq!(view.display_layout.rows[0].end_col, 7);
        assert_eq!(view.display_layout.rows[1].start_col, 7);
        assert_eq!(view.display_layout.rows[1].end_col, 10);
    }

    #[test]
    fn render_keeps_annotation_strip_at_far_left_with_composed_gutter() {
        let mut view = make_view(&["abcdefghij"]);
        view.handle_action(&Action::ToggleWordWrap);

        let buffer = render_buffer(&mut view, 10, 5, &[]);

        assert_eq!(buffer.cell((0, 0)).unwrap().symbol(), " ");
        assert_eq!(buffer.cell((1, 0)).unwrap().symbol(), " ");
        assert_eq!(buffer.cell((2, 0)).unwrap().symbol(), " ");
        assert_eq!(buffer.cell((3, 0)).unwrap().symbol(), "a");
        assert_eq!(buffer.cell((9, 0)).unwrap().symbol(), "g");
        assert_eq!(buffer.cell((3, 1)).unwrap().symbol(), "h");
    }

    #[test]
    fn update_dimensions_reserves_composed_gutter_width() {
        let mut view = make_view(&["hello"]);

        view.update_dimensions(43, 5);
        assert!(!view.is_too_small());

        view.update_dimensions(42, 5);
        assert!(view.is_too_small());
    }
}
