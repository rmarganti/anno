use ratatui::{
    style::Style,
    text::{Line, Span},
};

use crate::highlight::StyledSpan;
use crate::tui::theme::Theme;
use crate::tui::viewport::CursorPosition;

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

/// Apply width-dependent styling to a visible slice of styled lines
/// and convert them to ratatui `Line`s for rendering.
pub fn prepare_visible_lines(
    styled_slice: &[Vec<StyledSpan>],
    plain_slice: &[String],
    visible_start: usize,
    cursor_row: usize,
    cursor_col: usize,
    theme: &Theme,
    selection: Option<(CursorPosition, CursorPosition)>,
) -> Vec<Line<'static>> {
    let mut result: Vec<Line<'static>> = Vec::with_capacity(styled_slice.len());

    for (i, styled_spans) in styled_slice.iter().enumerate() {
        let doc_row = visible_start + i;

        let line = {
            let spans: Vec<Span> = styled_spans
                .iter()
                .map(|ss| Span::styled(ss.text.clone(), ss.style))
                .collect();
            Line::from(spans)
        };

        // Apply selection overlay (before cursor, so cursor appears on top).
        let line = if let Some((sel_start, sel_end)) = selection {
            if doc_row >= sel_start.row && doc_row <= sel_end.row {
                let col_start = if doc_row == sel_start.row {
                    sel_start.col
                } else {
                    0
                };
                let col_end = if doc_row == sel_end.row {
                    sel_end.col
                } else {
                    // Full line selected — use plain line length as upper bound.
                    plain_slice[i].chars().count().saturating_sub(1)
                };
                apply_selection_to_line(line, col_start, col_end, theme)
            } else {
                line
            }
        } else {
            line
        };

        if doc_row == cursor_row {
            result.push(apply_cursor_to_line(line, cursor_col, theme));
        } else {
            result.push(line);
        }
    }

    result
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
