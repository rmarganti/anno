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

/// Whether an active visual selection is character-wise or line-wise.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum VisualKind {
    Char,
    Line,
}

/// Active visual-mode anchor: the position where Visual mode was entered
/// together with the kind (character-wise vs line-wise) of selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct VisualAnchor {
    pos: CursorPosition,
    kind: VisualKind,
}

/// Manages the document content display state: viewport, cursor movement,
/// word wrap, and visual selection.
pub struct DocumentViewState {
    /// Viewport state (scroll, cursor, dimensions).
    viewport: Viewport,
    /// Display layout mapping document lines → display rows.
    display_layout: DisplayLayout,
    /// Anchor position when in Visual or Visual Line mode.
    visual_anchor: Option<VisualAnchor>,
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
            Action::MoveScreenUp => self.viewport.move_screen_up(&self.display_layout),
            Action::MoveScreenDown => self.viewport.move_screen_down(&self.display_layout),
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
                self.visual_anchor = Some(VisualAnchor {
                    pos: self.viewport.cursor,
                    kind: VisualKind::Char,
                });
            }

            Action::EnterVisualLineMode => {
                self.visual_anchor = Some(VisualAnchor {
                    pos: self.viewport.cursor,
                    kind: VisualKind::Line,
                });
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
        let sel = Selection { anchor: anchor.pos };
        let (start, end) = sel.range(self.viewport.cursor);
        let (start, end) = match anchor.kind {
            VisualKind::Char => (start, end),
            VisualKind::Line => self.snap_linewise(start, end),
        };
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
        let mut text = selection::selected_text(start, end, &self.doc_lines);
        if anchor.kind == VisualKind::Line {
            text.push('\n');
        }
        Some((range, text))
    }

    /// Snap a (start, end) pair so the start sits at column 0 of its row and
    /// the end sits at the last char index of its row (clamped to 0 for
    /// empty lines, matching the existing charwise empty-line behavior).
    fn snap_linewise(
        &self,
        start: CursorPosition,
        end: CursorPosition,
    ) -> (CursorPosition, CursorPosition) {
        let last_col = self
            .doc_lines
            .get(end.row)
            .map(|line| line.chars().count().saturating_sub(1))
            .unwrap_or(0);
        (
            CursorPosition {
                row: start.row,
                col: 0,
            },
            CursorPosition {
                row: end.row,
                col: last_col,
            },
        )
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
        line_number_mode: LineNumberMode,
        theme: &UiTheme,
    ) -> Vec<Line<'static>> {
        let line_number_width = Self::line_number_gutter_width(total_doc_lines);
        let separator = " ".to_string();

        render_slices
            .iter()
            .enumerate()
            .zip(Self::compute_gutter_annotation_types(
                render_slices,
                annotation_indicators,
            ))
            .map(|((index, slice), annotation_type)| {
                let line_number_style = if slice.doc_row == cursor_row {
                    theme.current_line_number
                } else {
                    theme.line_number
                };
                let line_number = if Self::shows_line_number(render_slices, index, slice) {
                    Self::format_line_number(
                        slice.doc_row,
                        cursor_row,
                        line_number_mode,
                        line_number_width,
                    )
                } else {
                    " ".repeat(line_number_width)
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
                    Span::styled(line_number, line_number_style),
                    Span::styled(separator.clone(), line_number_style),
                ])
            })
            .collect()
    }

    fn shows_line_number(
        render_slices: &[crate::tui::viewport::RenderSlice],
        index: usize,
        slice: &crate::tui::viewport::RenderSlice,
    ) -> bool {
        slice.start_col == 0 || index == 0 || render_slices[index - 1].doc_row != slice.doc_row
    }

    fn format_line_number(
        doc_row: usize,
        cursor_row: usize,
        line_number_mode: LineNumberMode,
        width: usize,
    ) -> String {
        let line_number = match line_number_mode {
            LineNumberMode::Absolute => doc_row + 1,
            LineNumberMode::Relative => {
                if doc_row == cursor_row {
                    doc_row + 1
                } else {
                    doc_row.abs_diff(cursor_row)
                }
            }
        };

        format!("{line_number:>width$}")
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
            let sel = Selection { anchor: anchor.pos };
            let (start, end) = sel.range(state.viewport.cursor);
            match anchor.kind {
                VisualKind::Char => (start, end),
                VisualKind::Line => state.snap_linewise(start, end),
            }
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
        make_view_with_mode(lines, LineNumberMode::Relative)
    }

    fn make_view_with_mode(lines: &[&str], line_number_mode: LineNumberMode) -> DocumentViewState {
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
        let mut view = DocumentViewState::new(doc_lines, styled_lines, line_number_mode);
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

    fn buffer_line(buffer: &ratatui::buffer::Buffer, y: u16, width: u16) -> String {
        (0..width)
            .map(|x| buffer.cell((x, y)).unwrap().symbol())
            .collect()
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
    fn render_absolute_line_numbers_are_human_readable_and_right_aligned() {
        let mut view = make_view_with_mode(&["first", "second", "third"], LineNumberMode::Absolute);

        let buffer = render_buffer(&mut view, 80, 5, &[]);

        assert_eq!(&buffer_line(&buffer, 0, 4), " 1 f");
        assert_eq!(&buffer_line(&buffer, 1, 4), " 2 s");
        assert_eq!(&buffer_line(&buffer, 2, 4), " 3 t");
    }

    #[test]
    fn render_relative_line_numbers_show_current_line_absolute() {
        let mut view = make_view_with_mode(&["first", "second", "third"], LineNumberMode::Relative);
        view.set_cursor(1, 0);

        let buffer = render_buffer(&mut view, 80, 5, &[]);

        assert_eq!(&buffer_line(&buffer, 0, 4), " 1 f");
        assert_eq!(&buffer_line(&buffer, 1, 4), " 2 s");
        assert_eq!(&buffer_line(&buffer, 2, 4), " 1 t");
    }

    #[test]
    fn render_wrapped_continuation_rows_leave_line_number_cells_blank() {
        let mut view = make_view_with_mode(&["abcdefghij"], LineNumberMode::Absolute);
        view.handle_action(&Action::ToggleWordWrap);

        let buffer = render_buffer(&mut view, 10, 5, &[]);

        assert_eq!(&buffer_line(&buffer, 0, 4), " 1 a");
        assert_eq!(&buffer_line(&buffer, 1, 4), "   h");
    }

    #[test]
    fn render_mid_wrapped_line_shows_number_on_first_visible_slice() {
        let mut view =
            make_view_with_mode(&["abcdefghijklmnopqrstuvwxyz"], LineNumberMode::Absolute);
        view.handle_action(&Action::ToggleWordWrap);
        view.update_dimensions(10, 1);
        view.set_cursor(0, 7);

        let buffer = render_buffer(&mut view, 10, 1, &[]);

        assert_eq!(&buffer_line(&buffer, 0, 4), " 1 h");
    }

    #[test]
    fn render_empty_document_row_uses_line_number_one() {
        let mut view = make_view_with_mode(&[""], LineNumberMode::Absolute);

        let buffer = render_buffer(&mut view, 10, 3, &[]);

        assert_eq!(&buffer_line(&buffer, 0, 3), " 1 ");
    }

    #[test]
    fn render_current_line_number_uses_emphasis_style_in_both_modes() {
        let theme = UiTheme::default();

        let mut absolute_view = make_view_with_mode(&["first", "second"], LineNumberMode::Absolute);
        absolute_view.set_cursor(1, 0);
        let absolute_buffer = render_buffer(&mut absolute_view, 80, 5, &[]);

        let mut relative_view = make_view_with_mode(&["first", "second"], LineNumberMode::Relative);
        relative_view.set_cursor(1, 0);
        let relative_buffer = render_buffer(&mut relative_view, 80, 5, &[]);

        assert_eq!(
            absolute_buffer.cell((1, 1)).unwrap().style().fg,
            theme.current_line_number.fg
        );
        assert_eq!(
            relative_buffer.cell((1, 1)).unwrap().style().fg,
            theme.current_line_number.fg
        );
        assert_eq!(
            absolute_buffer.cell((1, 0)).unwrap().style().fg,
            theme.line_number.fg
        );
        assert_eq!(
            relative_buffer.cell((1, 0)).unwrap().style().fg,
            theme.line_number.fg
        );
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

    // ── Visual Line selection ─────────────────────────────────────────

    #[test]
    fn enter_visual_line_mode_sets_line_anchor_at_cursor() {
        let mut view = make_view(&["hello world", "second line"]);
        view.set_cursor(0, 4);
        let consumed = view.handle_action(&Action::EnterVisualLineMode);
        assert!(consumed);
        let anchor = view.visual_anchor.expect("anchor should be set");
        assert_eq!(anchor.kind, VisualKind::Line);
        assert_eq!(anchor.pos, pos(0, 4));
    }

    #[test]
    fn take_visual_line_selection_only_anchor_covers_full_row_with_newline() {
        let mut view = make_view(&["hello world"]);
        view.set_cursor(0, 4);
        view.handle_action(&Action::EnterVisualLineMode);
        let (range, text) = view.take_visual_selection().unwrap();
        assert_eq!(range.start, TextPosition { line: 0, column: 0 });
        assert_eq!(
            range.end,
            TextPosition {
                line: 0,
                column: "hello world".chars().count() - 1
            }
        );
        assert_eq!(text, "hello world\n");
    }

    #[test]
    fn take_visual_line_selection_after_move_down_covers_full_lines() {
        let mut view = make_view(&["first line", "second line", "third line"]);
        view.set_cursor(0, 3);
        view.handle_action(&Action::EnterVisualLineMode);
        view.handle_action(&Action::MoveDown);
        let (range, text) = view.take_visual_selection().unwrap();
        assert_eq!(range.start, TextPosition { line: 0, column: 0 });
        assert_eq!(
            range.end,
            TextPosition {
                line: 1,
                column: "second line".chars().count() - 1
            }
        );
        assert_eq!(text, "first line\nsecond line\n");
    }

    #[test]
    fn take_visual_line_selection_handles_anchor_below_cursor() {
        let mut view = make_view(&["first line", "second line"]);
        view.set_cursor(1, 3);
        view.handle_action(&Action::EnterVisualLineMode);
        view.handle_action(&Action::MoveUp);
        let (range, text) = view.take_visual_selection().unwrap();
        assert_eq!(range.start, TextPosition { line: 0, column: 0 });
        assert_eq!(
            range.end,
            TextPosition {
                line: 1,
                column: "second line".chars().count() - 1
            }
        );
        assert_eq!(text, "first line\nsecond line\n");
    }

    #[test]
    fn take_visual_line_selection_on_empty_line_clamps_end_col() {
        let mut view = make_view(&["", "second"]);
        view.set_cursor(0, 0);
        view.handle_action(&Action::EnterVisualLineMode);
        let (range, text) = view.take_visual_selection().unwrap();
        assert_eq!(range.start, TextPosition { line: 0, column: 0 });
        assert_eq!(range.end, TextPosition { line: 0, column: 0 });
        assert_eq!(text, "\n");
    }

    #[test]
    fn charwise_visual_selection_does_not_get_trailing_newline() {
        // Regression guard: charwise must keep ending at the cursor's exact
        // column with no trailing newline.
        let mut view = make_view(&["hello"]);
        view.handle_action(&Action::EnterVisualMode);
        view.handle_action(&Action::MoveRight);
        let (range, text) = view.take_visual_selection().unwrap();
        assert_eq!(range.start, TextPosition { line: 0, column: 0 });
        assert_eq!(range.end, TextPosition { line: 0, column: 1 });
        assert_eq!(text, "he");
        assert!(!text.ends_with('\n'));
    }

    #[test]
    fn clear_visual_clears_line_kind_anchor() {
        let mut view = make_view(&["hello"]);
        view.handle_action(&Action::EnterVisualLineMode);
        view.clear_visual();
        assert!(view.take_visual_selection().is_none());
    }

    #[test]
    fn render_visual_line_highlights_every_column_of_selected_rows() {
        let theme = UiTheme::default();
        let mut view = make_view(&["abc", "defgh", "ijk"]);
        view.set_cursor(0, 1);
        view.handle_action(&Action::EnterVisualLineMode);
        view.handle_action(&Action::MoveDown);

        let backend = TestBackend::new(40, 5);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|frame| {
                view.update_dimensions(40, 5);
                render_document_view(
                    frame,
                    Rect {
                        x: 0,
                        y: 0,
                        width: 40,
                        height: 5,
                    },
                    &view,
                    &theme,
                    true,
                    &[],
                    None,
                );
            })
            .unwrap();
        let buffer = terminal.backend().buffer().clone();

        // Locate the columns containing the document text on rows 0 and 1.
        let row0 = buffer_line(&buffer, 0, 40);
        let row1 = buffer_line(&buffer, 1, 40);
        let abc_x = row0.find("abc").expect("abc should be visible") as u16;
        let defgh_x = row1.find("defgh").expect("defgh should be visible") as u16;

        // Every char of "abc" is part of the selection (full first row).
        // The anchor was set at row 0 col 1 ('b'), so every cell must carry
        // the selection background.
        for i in 0..3u16 {
            let cell = buffer.cell((abc_x + i, 0)).unwrap();
            assert_eq!(cell.style().bg, theme.selection_highlight.bg);
        }

        // Every char of "defgh" is part of the selection. The cursor (after
        // MoveDown) lives at row 1 col 1 ('e'), which gets the cursor style
        // instead of selection.
        for i in 0..5u16 {
            let cell = buffer.cell((defgh_x + i, 1)).unwrap();
            if i == 1 {
                assert_eq!(cell.style().bg, theme.cursor.bg);
            } else {
                assert_eq!(cell.style().bg, theme.selection_highlight.bg);
            }
        }

        // Row 2 ("ijk") is outside the selection.
        let ijk_x = buffer_line(&buffer, 2, 40)
            .find("ijk")
            .expect("ijk should be visible") as u16;
        for i in 0..3u16 {
            let cell = buffer.cell((ijk_x + i, 2)).unwrap();
            assert_ne!(cell.style().bg, theme.selection_highlight.bg);
        }
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
        assert_eq!(buffer.cell((1, 0)).unwrap().symbol(), "1");
        assert_eq!(buffer.cell((2, 0)).unwrap().symbol(), " ");
        assert_eq!(buffer.cell((3, 0)).unwrap().symbol(), "a");
        assert_eq!(buffer.cell((9, 0)).unwrap().symbol(), "g");
        assert_eq!(buffer.cell((1, 1)).unwrap().symbol(), " ");
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
