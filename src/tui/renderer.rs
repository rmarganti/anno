use ratatui::{
    style::Style,
    text::{Line, Span},
};

use crate::annotation::types::TextRange;
use crate::highlight::StyledSpan;
use crate::tui::theme::UiTheme;
use crate::tui::viewport::{CursorPosition, RenderSlice};

/// Intermediate result of text-to-lines conversion.
/// Stores width-independent plain and styled lines.
pub struct DocumentLines {
    pub plain: Vec<String>,
    pub styled: Vec<Vec<StyledSpan>>,
}

/// Convert raw text into flat document lines.
///
/// This is width-independent — called once at startup.
/// Width-dependent adjustments happen in `prepare_visible_lines()`.
pub fn text_to_lines(text: &str, highlighter: &dyn crate::highlight::Highlighter) -> DocumentLines {
    // Empty text produces a single empty line so the cursor has a valid position.
    let plain: Vec<String> = if text.is_empty() {
        vec![String::new()]
    } else {
        text.split('\n').map(str::to_owned).collect()
    };

    let styled = highlighter.highlight_document(&plain);

    DocumentLines { plain, styled }
}

/// Parameters for [`prepare_visible_lines_from_slices`].
pub struct PrepareVisibleLinesParams<'a> {
    pub slices: &'a [RenderSlice],
    pub styled_lines: &'a [Vec<StyledSpan>],
    pub plain_lines: &'a [String],
    pub cursor_row: usize,
    pub cursor_col: usize,
    pub theme: &'a UiTheme,
    pub selection: Option<(CursorPosition, CursorPosition)>,
    pub annotation_ranges: &'a [TextRange],
    pub selected_annotation_range: Option<&'a TextRange>,
}

/// Prepare visible lines from render slices (supports both word-wrap and horizontal scroll).
///
/// Each `RenderSlice` maps to one display row, referencing a sub-range of a document line.
/// Styled spans are sliced to the column range, and cursor/selection overlays are applied
/// using intersection with the slice's column range.
pub fn prepare_visible_lines_from_slices(
    params: &PrepareVisibleLinesParams<'_>,
) -> Vec<Line<'static>> {
    let mut result: Vec<Line<'static>> = Vec::with_capacity(params.slices.len());

    for slice in params.slices {
        let doc_row = slice.doc_row;
        let start_col = slice.start_col;
        let end_col = slice.end_col;

        // Slice styled spans to the column range.
        let line = if doc_row < params.styled_lines.len() {
            slice_styled_spans(&params.styled_lines[doc_row], start_col, end_col)
        } else {
            Line::from(Span::raw(""))
        };

        // Apply annotation highlight overlay.
        let line = apply_annotation_overlays(
            line,
            slice,
            params.plain_lines,
            params.annotation_ranges,
            params.selected_annotation_range,
            params.theme,
        );

        // Apply selection overlay.
        let line = if let Some((sel_start, sel_end)) = params.selection {
            if doc_row >= sel_start.row && doc_row <= sel_end.row {
                let doc_col_start = if doc_row == sel_start.row {
                    sel_start.col
                } else {
                    0
                };
                let doc_col_end = if doc_row == sel_end.row {
                    sel_end.col
                } else {
                    params
                        .plain_lines
                        .get(doc_row)
                        .map(|l| l.chars().count().saturating_sub(1))
                        .unwrap_or(0)
                };

                // Intersect selection with slice range.
                let lo = doc_col_start.max(start_col);
                let hi = doc_col_end.min(end_col.saturating_sub(1));
                if lo <= hi {
                    apply_selection_to_line(
                        line,
                        lo.saturating_sub(start_col),
                        hi.saturating_sub(start_col),
                        params.theme,
                    )
                } else {
                    line
                }
            } else {
                line
            }
        } else {
            line
        };

        // Apply cursor overlay.
        if doc_row == params.cursor_row
            && params.cursor_col >= start_col
            && params.cursor_col < end_col
        {
            result.push(apply_cursor_to_line(
                line,
                params.cursor_col - start_col,
                params.theme,
            ));
        } else if doc_row == params.cursor_row
            && params.cursor_col == start_col
            && start_col == end_col
        {
            // Empty line with cursor.
            result.push(apply_cursor_to_line(line, 0, params.theme));
        } else {
            result.push(line);
        }
    }

    result
}

/// Slice styled spans to a character column range [start_col, end_col).
fn slice_styled_spans(spans: &[StyledSpan], start_col: usize, end_col: usize) -> Line<'static> {
    let mut result_spans: Vec<Span> = Vec::new();
    let mut offset = 0usize;

    for ss in spans {
        let span_len = ss.text.chars().count();
        let span_start = offset;
        let span_end = offset + span_len;

        // Check overlap with [start_col, end_col).
        let lo = start_col.max(span_start);
        let hi = end_col.min(span_end);

        if lo < hi {
            // Extract the overlapping portion without intermediate Vec allocation.
            let local_lo = lo - span_start;
            let local_hi = hi - span_start;
            let text: String = ss
                .text
                .chars()
                .skip(local_lo)
                .take(local_hi - local_lo)
                .collect();
            result_spans.push(Span::styled(text, ss.style));
        }

        offset += span_len;
        if offset >= end_col {
            break;
        }
    }

    if result_spans.is_empty() {
        Line::from(Span::raw(""))
    } else {
        Line::from(result_spans)
    }
}

/// Apply a block cursor overlay to a pre-styled `Line` at the given column.
pub fn apply_cursor_to_line(line: Line<'_>, cursor_col: usize, theme: &UiTheme) -> Line<'static> {
    // Flatten all spans into chars with their original style.
    let mut chars_with_style: Vec<(char, Style)> = Vec::new();
    for span in line.spans.iter() {
        for c in span.content.chars() {
            chars_with_style.push((c, span.style));
        }
    }

    if chars_with_style.is_empty() {
        return Line::from(Span::styled(" ", theme.cursor));
    }

    let col = cursor_col.min(chars_with_style.len().saturating_sub(1));

    // Rebuild spans, applying cursor style to the character at `col`.
    let mut spans: Vec<Span> = Vec::new();
    let mut current_text = String::new();
    let mut current_style: Option<Style> = None;

    for (i, &(ch, style)) in chars_with_style.iter().enumerate() {
        let effective_style = if i == col { theme.cursor } else { style };

        match current_style {
            Some(s) if s == effective_style => {
                current_text.push(ch);
            }
            _ => {
                if let Some(s) = current_style {
                    spans.push(Span::styled(std::mem::take(&mut current_text), s));
                }
                current_text.push(ch);
                current_style = Some(effective_style);
            }
        }
    }
    if let Some(s) = current_style {
        spans.push(Span::styled(current_text, s));
    }

    Line::from(spans)
}

/// Apply annotation underline overlays to a line for all annotation ranges that intersect it.
fn apply_annotation_overlays(
    line: Line<'static>,
    slice: &RenderSlice,
    plain_lines: &[String],
    annotation_ranges: &[TextRange],
    selected_annotation_range: Option<&TextRange>,
    theme: &UiTheme,
) -> Line<'static> {
    let doc_row = slice.doc_row;
    let start_col = slice.start_col;
    let end_col = slice.end_col;

    // Collect all column ranges within this display slice that need highlighting.
    let mut highlight_cols: Vec<(usize, usize)> = Vec::new();
    // Separate tracking for the selected annotation range.
    let mut selected_cols: Vec<(usize, usize)> = Vec::new();

    let line_len = plain_lines
        .get(doc_row)
        .map(|l| l.chars().count())
        .unwrap_or(0);

    let intersect_range = |range: &TextRange| -> Option<(usize, usize)> {
        if doc_row < range.start.line || doc_row > range.end.line {
            return None;
        }
        if line_len == 0 {
            return None;
        }

        let doc_col_start = if doc_row == range.start.line {
            range.start.column
        } else {
            0
        };
        let doc_col_end = if doc_row == range.end.line {
            range.end.column
        } else {
            line_len.saturating_sub(1)
        };

        let lo = doc_col_start.max(start_col);
        let hi = doc_col_end.min(end_col.saturating_sub(1));
        if lo <= hi {
            Some((lo.saturating_sub(start_col), hi.saturating_sub(start_col)))
        } else {
            None
        }
    };

    for range in annotation_ranges {
        if let Some(cols) = intersect_range(range) {
            highlight_cols.push(cols);
        }
    }

    if let Some(range) = selected_annotation_range
        && let Some(cols) = intersect_range(range)
    {
        selected_cols.push(cols);
    }

    if highlight_cols.is_empty() && selected_cols.is_empty() {
        return line;
    }

    // Flatten line into chars with styles, then apply overlays to highlighted columns.
    let mut chars_with_style: Vec<(char, Style)> = Vec::new();
    for span in line.spans.iter() {
        for c in span.content.chars() {
            chars_with_style.push((c, span.style));
        }
    }

    if chars_with_style.is_empty() {
        return line;
    }

    // Mark which char positions need the annotation style.
    let len = chars_with_style.len();
    let mut marked = vec![false; len];
    for (lo, hi) in &highlight_cols {
        let lo = (*lo).min(len.saturating_sub(1));
        let hi = (*hi).min(len.saturating_sub(1));
        for m in &mut marked[lo..=hi] {
            *m = true;
        }
    }

    // Mark which char positions need the selected-annotation style.
    let mut selected = vec![false; len];
    for (lo, hi) in &selected_cols {
        let lo = (*lo).min(len.saturating_sub(1));
        let hi = (*hi).min(len.saturating_sub(1));
        for m in &mut selected[lo..=hi] {
            *m = true;
        }
    }

    // Rebuild spans.
    let mut spans: Vec<Span> = Vec::new();
    let mut current_text = String::new();
    let mut current_style: Option<Style> = None;

    for (i, &(ch, style)) in chars_with_style.iter().enumerate() {
        let mut effective_style = style;
        if marked[i] {
            effective_style = effective_style.patch(theme.annotation_highlight);
        }
        if selected[i] {
            effective_style = effective_style.patch(theme.selected_annotation_highlight);
        }

        match current_style {
            Some(s) if s == effective_style => current_text.push(ch),
            _ => {
                if let Some(s) = current_style {
                    spans.push(Span::styled(std::mem::take(&mut current_text), s));
                }
                current_text.push(ch);
                current_style = Some(effective_style);
            }
        }
    }
    if let Some(s) = current_style {
        spans.push(Span::styled(current_text, s));
    }

    Line::from(spans)
}

/// Apply a selection highlight overlay to a pre-styled `Line` over the given column range.
pub fn apply_selection_to_line(
    line: Line<'_>,
    col_start: usize,
    col_end: usize,
    theme: &UiTheme,
) -> Line<'static> {
    let mut chars_with_style: Vec<(char, Style)> = Vec::new();
    for span in &line.spans {
        for c in span.content.chars() {
            chars_with_style.push((c, span.style));
        }
    }

    if chars_with_style.is_empty() {
        return Line::from(Span::styled(" ", theme.selection_highlight));
    }

    let end = col_end.min(chars_with_style.len().saturating_sub(1));

    let mut spans: Vec<Span> = Vec::new();
    let mut current_text = String::new();
    let mut current_style: Option<Style> = None;

    for (i, &(ch, style)) in chars_with_style.iter().enumerate() {
        let effective_style = if i >= col_start && i <= end {
            style.patch(theme.selection_highlight)
        } else {
            style
        };

        match current_style {
            Some(s) if s == effective_style => current_text.push(ch),
            _ => {
                if let Some(s) = current_style {
                    spans.push(Span::styled(std::mem::take(&mut current_text), s));
                }
                current_text.push(ch);
                current_style = Some(effective_style);
            }
        }
    }
    if let Some(s) = current_style {
        spans.push(Span::styled(current_text, s));
    }

    Line::from(spans)
}

#[cfg(test)]
mod tests {
    use ratatui::text::{Line, Span};

    use super::*;
    use crate::highlight::{Highlighter, StyledSpan};
    use crate::tui::theme::UiTheme;
    use crate::tui::viewport::RenderSlice;

    // ── Helpers ───────────────────────────────────────────────────────

    fn default_theme() -> UiTheme {
        UiTheme::new()
    }

    /// A no-op highlighter that returns unstyled spans for every line.
    struct PlainHighlighter;

    impl Highlighter for PlainHighlighter {
        fn highlight_document(&self, lines: &[String]) -> Vec<Vec<StyledSpan>> {
            lines
                .iter()
                .map(|l| {
                    if l.is_empty() {
                        vec![]
                    } else {
                        vec![StyledSpan::plain(l.clone())]
                    }
                })
                .collect()
        }
    }

    fn plain_line(text: &str) -> Line<'static> {
        Line::from(Span::raw(text.to_string()))
    }

    fn collect_text(line: &Line<'_>) -> String {
        line.spans.iter().map(|s| s.content.as_ref()).collect()
    }

    // ── text_to_lines ─────────────────────────────────────────────────

    #[test]
    fn text_to_lines_empty_produces_single_empty_line() {
        let result = text_to_lines("", &PlainHighlighter);
        assert_eq!(result.plain, vec!["".to_string()]);
        assert_eq!(result.styled.len(), 1);
    }

    #[test]
    fn text_to_lines_single_line() {
        let result = text_to_lines("hello", &PlainHighlighter);
        assert_eq!(result.plain, vec!["hello".to_string()]);
        assert_eq!(result.styled.len(), 1);
        assert_eq!(result.styled[0][0].text, "hello");
    }

    #[test]
    fn text_to_lines_splits_on_newline() {
        let result = text_to_lines("foo\nbar\nbaz", &PlainHighlighter);
        assert_eq!(
            result.plain,
            vec!["foo".to_string(), "bar".to_string(), "baz".to_string()]
        );
        assert_eq!(result.styled.len(), 3);
    }

    #[test]
    fn text_to_lines_count_matches_styled_count() {
        let text = "line one\nline two\nline three";
        let result = text_to_lines(text, &PlainHighlighter);
        assert_eq!(result.plain.len(), result.styled.len());
    }

    // ── apply_cursor_to_line ──────────────────────────────────────────

    #[test]
    fn apply_cursor_empty_line_produces_space_with_cursor_style() {
        let theme = default_theme();
        let line = plain_line("");
        let result = apply_cursor_to_line(line, 0, &theme);
        assert_eq!(collect_text(&result), " ");
        assert_eq!(result.spans[0].style, theme.cursor);
    }

    #[test]
    fn apply_cursor_at_col_zero() {
        let theme = default_theme();
        let line = plain_line("hello");
        let result = apply_cursor_to_line(line, 0, &theme);
        // First char should have cursor style.
        assert_eq!(result.spans[0].style, theme.cursor);
        assert!(result.spans[0].content.starts_with('h'));
    }

    #[test]
    fn apply_cursor_at_middle_col() {
        let theme = default_theme();
        let line = plain_line("abcde");
        let result = apply_cursor_to_line(line, 2, &theme);
        let text = collect_text(&result);
        assert_eq!(text, "abcde");
        // Find the span containing 'c' (col 2) and verify its style.
        let cursor_span = result
            .spans
            .iter()
            .find(|s| s.content.contains('c'))
            .unwrap();
        assert_eq!(cursor_span.style, theme.cursor);
    }

    #[test]
    fn apply_cursor_clamps_to_last_char() {
        let theme = default_theme();
        let line = plain_line("ab");
        // cursor_col beyond line length should clamp to last char.
        let result = apply_cursor_to_line(line, 99, &theme);
        let text = collect_text(&result);
        assert_eq!(text, "ab");
        // The last char 'b' should have cursor style.
        let last_span = result.spans.last().unwrap();
        assert_eq!(last_span.style, theme.cursor);
    }

    #[test]
    fn apply_cursor_preserves_text_content() {
        let theme = default_theme();
        let line = plain_line("hello world");
        let result = apply_cursor_to_line(line, 6, &theme);
        assert_eq!(collect_text(&result), "hello world");
    }

    // ── apply_selection_to_line ───────────────────────────────────────

    #[test]
    fn apply_selection_empty_line_produces_space_with_selection_style() {
        let theme = default_theme();
        let line = plain_line("");
        let result = apply_selection_to_line(line, 0, 0, &theme);
        assert_eq!(collect_text(&result), " ");
        assert_eq!(result.spans[0].style, theme.selection_highlight);
    }

    #[test]
    fn apply_selection_full_line() {
        let theme = default_theme();
        let line = plain_line("hello");
        let result = apply_selection_to_line(line, 0, 4, &theme);
        let text = collect_text(&result);
        assert_eq!(text, "hello");
        // All chars should use selection highlight (merged into one or more spans).
        for span in &result.spans {
            assert_eq!(span.style, theme.selection_highlight);
        }
    }

    #[test]
    fn apply_selection_preserves_text_content() {
        let theme = default_theme();
        let line = plain_line("abcde");
        let result = apply_selection_to_line(line, 1, 3, &theme);
        assert_eq!(collect_text(&result), "abcde");
    }

    #[test]
    fn apply_selection_partial_range_styles_only_selected_chars() {
        let theme = default_theme();
        let line = plain_line("abcde");
        let result = apply_selection_to_line(line, 1, 3, &theme);
        // 'a' (col 0) should NOT have selection style.
        let first_span = &result.spans[0];
        assert_ne!(first_span.style, theme.selection_highlight);
        assert!(first_span.content.starts_with('a'));
    }

    // ── prepare_visible_lines_from_slices ─────────────────────────────

    #[test]
    fn prepare_visible_lines_basic_no_cursor_no_selection() {
        let theme = default_theme();
        let doc_lines = vec!["hello".to_string(), "world".to_string()];
        let styled_lines: Vec<Vec<StyledSpan>> = doc_lines
            .iter()
            .map(|l| vec![StyledSpan::plain(l.clone())])
            .collect();
        let slices = vec![
            RenderSlice {
                doc_row: 0,
                start_col: 0,
                end_col: 5,
            },
            RenderSlice {
                doc_row: 1,
                start_col: 0,
                end_col: 5,
            },
        ];

        // Cursor is off-screen (row 99).
        let lines = prepare_visible_lines_from_slices(&PrepareVisibleLinesParams {
            slices: &slices,
            styled_lines: &styled_lines,
            plain_lines: &doc_lines,
            cursor_row: 99,
            cursor_col: 0,
            theme: &theme,
            selection: None,
            annotation_ranges: &[],
            selected_annotation_range: None,
        });

        assert_eq!(lines.len(), 2);
        assert_eq!(collect_text(&lines[0]), "hello");
        assert_eq!(collect_text(&lines[1]), "world");
    }

    #[test]
    fn prepare_visible_lines_cursor_applied_to_correct_row() {
        let theme = default_theme();
        let doc_lines = vec!["hello".to_string()];
        let styled_lines = vec![vec![StyledSpan::plain("hello")]];
        let slices = vec![RenderSlice {
            doc_row: 0,
            start_col: 0,
            end_col: 5,
        }];

        let lines = prepare_visible_lines_from_slices(&PrepareVisibleLinesParams {
            slices: &slices,
            styled_lines: &styled_lines,
            plain_lines: &doc_lines,
            cursor_row: 0,
            cursor_col: 2,
            theme: &theme,
            selection: None,
            annotation_ranges: &[],
            selected_annotation_range: None,
        });

        assert_eq!(lines.len(), 1);
        let text = collect_text(&lines[0]);
        assert_eq!(text, "hello");
        // Col 2 ('l') should carry cursor style.
        let cursor_span = lines[0]
            .spans
            .iter()
            .find(|s| s.style == theme.cursor)
            .unwrap();
        assert!(cursor_span.content.starts_with('l'));
    }

    #[test]
    fn prepare_visible_lines_out_of_bounds_doc_row_returns_empty() {
        let theme = default_theme();
        let doc_lines: Vec<String> = vec![];
        let styled_lines: Vec<Vec<StyledSpan>> = vec![];
        // Slice referencing a doc_row that doesn't exist.
        let slices = vec![RenderSlice {
            doc_row: 5,
            start_col: 0,
            end_col: 10,
        }];

        let lines = prepare_visible_lines_from_slices(&PrepareVisibleLinesParams {
            slices: &slices,
            styled_lines: &styled_lines,
            plain_lines: &doc_lines,
            cursor_row: 99,
            cursor_col: 0,
            theme: &theme,
            selection: None,
            annotation_ranges: &[],
            selected_annotation_range: None,
        });

        assert_eq!(lines.len(), 1);
        assert_eq!(collect_text(&lines[0]), "");
    }

    #[test]
    fn prepare_visible_lines_horizontal_slice() {
        let theme = default_theme();
        let doc_lines = vec!["abcdefgh".to_string()];
        let styled_lines = vec![vec![StyledSpan::plain("abcdefgh")]];
        // Slice columns 2..5 → "cde"
        let slices = vec![RenderSlice {
            doc_row: 0,
            start_col: 2,
            end_col: 5,
        }];

        let lines = prepare_visible_lines_from_slices(&PrepareVisibleLinesParams {
            slices: &slices,
            styled_lines: &styled_lines,
            plain_lines: &doc_lines,
            cursor_row: 99,
            cursor_col: 0,
            theme: &theme,
            selection: None,
            annotation_ranges: &[],
            selected_annotation_range: None,
        });

        assert_eq!(collect_text(&lines[0]), "cde");
    }
}
