use ratatui::{
    style::Style,
    text::{Line, Span},
};

use crate::annotation::types::TextRange;
use crate::highlight::StyledSpan;
use crate::tui::theme::Theme;
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

/// Prepare visible lines from render slices (supports both word-wrap and horizontal scroll).
///
/// Each `RenderSlice` maps to one display row, referencing a sub-range of a document line.
/// Styled spans are sliced to the column range, and cursor/selection overlays are applied
/// using intersection with the slice's column range.
pub fn prepare_visible_lines_from_slices(
    slices: &[RenderSlice],
    styled_lines: &[Vec<StyledSpan>],
    plain_lines: &[String],
    cursor_row: usize,
    cursor_col: usize,
    theme: &Theme,
    selection: Option<(CursorPosition, CursorPosition)>,
    annotation_ranges: &[TextRange],
) -> Vec<Line<'static>> {
    let mut result: Vec<Line<'static>> = Vec::with_capacity(slices.len());

    for slice in slices {
        let doc_row = slice.doc_row;
        let start_col = slice.start_col;
        let end_col = slice.end_col;

        // Slice styled spans to the column range.
        let line = if doc_row < styled_lines.len() {
            slice_styled_spans(&styled_lines[doc_row], start_col, end_col)
        } else {
            Line::from(Span::raw(""))
        };

        // Apply annotation highlight overlay.
        let line = apply_annotation_overlays(line, doc_row, start_col, end_col, plain_lines, annotation_ranges, theme);

        // Apply selection overlay.
        let line = if let Some((sel_start, sel_end)) = selection {
            if doc_row >= sel_start.row && doc_row <= sel_end.row {
                let doc_col_start = if doc_row == sel_start.row {
                    sel_start.col
                } else {
                    0
                };
                let doc_col_end = if doc_row == sel_end.row {
                    sel_end.col
                } else {
                    plain_lines
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
                        theme,
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
        if doc_row == cursor_row && cursor_col >= start_col && cursor_col < end_col {
            result.push(apply_cursor_to_line(
                line,
                cursor_col - start_col,
                theme,
            ));
        } else if doc_row == cursor_row && cursor_col == start_col && start_col == end_col {
            // Empty line with cursor.
            result.push(apply_cursor_to_line(line, 0, theme));
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
            let text: String = ss.text.chars().skip(local_lo).take(local_hi - local_lo).collect();
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
pub fn apply_cursor_to_line(line: Line<'_>, cursor_col: usize, theme: &Theme) -> Line<'static> {
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
    doc_row: usize,
    start_col: usize,
    end_col: usize,
    plain_lines: &[String],
    annotation_ranges: &[TextRange],
    theme: &Theme,
) -> Line<'static> {
    // Collect all column ranges within this display slice that need highlighting.
    let mut highlight_cols: Vec<(usize, usize)> = Vec::new();

    for range in annotation_ranges {
        if doc_row < range.start.line || doc_row > range.end.line {
            continue;
        }

        let line_len = plain_lines
            .get(doc_row)
            .map(|l| l.chars().count())
            .unwrap_or(0);
        if line_len == 0 {
            continue;
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

        // Intersect with the display slice column range.
        let lo = doc_col_start.max(start_col);
        let hi = doc_col_end.min(end_col.saturating_sub(1));
        if lo <= hi {
            highlight_cols.push((lo.saturating_sub(start_col), hi.saturating_sub(start_col)));
        }
    }

    if highlight_cols.is_empty() {
        return line;
    }

    // Flatten line into chars with styles, then apply underline to highlighted columns.
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

    // Rebuild spans.
    let mut spans: Vec<Span> = Vec::new();
    let mut current_text = String::new();
    let mut current_style: Option<Style> = None;

    for (i, &(ch, style)) in chars_with_style.iter().enumerate() {
        let effective_style = if marked[i] {
            style.patch(theme.annotation_highlight)
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

/// Apply a selection highlight overlay to a pre-styled `Line` over the given column range.
pub fn apply_selection_to_line(
    line: Line<'_>,
    col_start: usize,
    col_end: usize,
    theme: &Theme,
) -> Line<'static> {
    let mut chars_with_style: Vec<(char, Style)> = Vec::new();
    for span in line.spans.iter() {
        for c in span.content.chars() {
            chars_with_style.push((c, span.style));
        }
    }

    if chars_with_style.is_empty() {
        // Empty line in selection: render a space with selection bg.
        return Line::from(Span::styled(" ", theme.selection_highlight));
    }

    let end = col_end.min(chars_with_style.len().saturating_sub(1));

    let mut spans: Vec<Span> = Vec::new();
    let mut current_text = String::new();
    let mut current_style: Option<Style> = None;

    for (i, &(ch, style)) in chars_with_style.iter().enumerate() {
        let effective_style = if i >= col_start && i <= end {
            // Merge: keep fg from original style if set, override bg with selection.
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
