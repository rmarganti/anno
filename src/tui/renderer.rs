use ratatui::{
    style::Style,
    text::{Line, Span},
};

use crate::highlight::{Highlighter, StyledSpan};
use crate::markdown::block::{Block as MdBlock, BlockType};
use crate::tui::theme::Theme;
use crate::tui::viewport::CursorPosition;

/// Metadata about what kind of block a document line came from.
/// Used by `prepare_visible_lines()` for width-dependent adjustments.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineKind {
    /// Normal line (paragraph, heading, list, blockquote, etc.) — no special padding.
    Normal,
    /// Code block content or fence line — pad with bg color to full width.
    Code,
    /// Horizontal rule — expand `─` to full width.
    HorizontalRule,
    /// Table row — handled by table formatting.
    TableRow,
}

/// Metadata mapping a document line back to its source block.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LineInfo {
    /// The block this line belongs to. `None` for blank separator lines.
    pub block_id: Option<String>,
    /// Character offset within the block's `content` string where this
    /// document line's content begins. Meaningless when `block_id` is None.
    pub content_start: usize,
    /// Number of prefix characters prepended to the document line that
    /// are NOT part of the block's `content` (heading markers, indentation,
    /// list markers, blockquote border). Needed to map column → content offset.
    pub prefix_len: usize,
}

/// Intermediate result of blocks-to-lines conversion.
/// Stores width-independent plain and styled lines.
pub struct DocumentLines {
    pub plain: Vec<String>,
    pub styled: Vec<Vec<StyledSpan>>,
    pub line_kinds: Vec<LineKind>,
    pub line_info: Vec<LineInfo>,
}

/// Convert parsed blocks into flat document lines.
///
/// This is width-independent — called once at startup.
/// Width-dependent adjustments happen in `prepare_visible_lines()`.
pub fn blocks_to_lines(
    blocks: &[MdBlock],
    highlighter: &dyn Highlighter,
    theme: &Theme,
) -> DocumentLines {
    let mut plain: Vec<String> = Vec::new();
    let mut styled: Vec<Vec<StyledSpan>> = Vec::new();
    let mut line_kinds: Vec<LineKind> = Vec::new();
    let mut line_info: Vec<LineInfo> = Vec::new();

    for block in blocks {
        match &block.block_type {
            BlockType::Heading => {
                let prefix = "#".repeat(block.level);
                let prefix_len = prefix.chars().count() + 1; // "### " → level + 1 chars
                let text = format!("{prefix} {}", block.content);
                plain.push(text.clone());
                styled.push(vec![StyledSpan::new(text, theme.heading(block.level))]);
                line_kinds.push(LineKind::Normal);
                line_info.push(LineInfo {
                    block_id: Some(block.id.clone()),
                    content_start: 0,
                    prefix_len,
                });
            }
            BlockType::Paragraph => {
                let mut offset = 0usize;
                for line in block.content.split('\n') {
                    plain.push(line.to_string());
                    styled.push(highlighter.highlight_line(line));
                    line_kinds.push(LineKind::Normal);
                    line_info.push(LineInfo {
                        block_id: Some(block.id.clone()),
                        content_start: offset,
                        prefix_len: 0,
                    });
                    offset += line.chars().count() + 1; // +1 for '\n'
                }
            }
            BlockType::Code => {
                let lang = block.language.as_deref().unwrap_or("");
                let fence_open = format!("```{lang}");
                plain.push(fence_open.clone());
                styled.push(vec![StyledSpan::new(fence_open, theme.code_fence)]);
                line_kinds.push(LineKind::Code);
                // fence-open: synthetic, no content
                line_info.push(LineInfo {
                    block_id: Some(block.id.clone()),
                    content_start: 0,
                    prefix_len: 0,
                });

                let highlighted =
                    highlighter.highlight_code_block(&block.content, block.language.as_deref());
                let mut offset = 0usize;
                for (code_line, spans) in block.content.split('\n').zip(highlighted) {
                    let indented = format!("  {code_line}");
                    plain.push(indented);
                    let mut line_spans = vec![StyledSpan::new("  ", Style::default())];
                    line_spans.extend(spans);
                    styled.push(line_spans);
                    line_kinds.push(LineKind::Code);
                    // prefix_len = 2 ("  " indentation added by renderer)
                    line_info.push(LineInfo {
                        block_id: Some(block.id.clone()),
                        content_start: offset,
                        prefix_len: 2,
                    });
                    offset += code_line.chars().count() + 1;
                }

                plain.push("```".to_string());
                styled.push(vec![StyledSpan::new("```", theme.code_fence)]);
                line_kinds.push(LineKind::Code);
                // fence-close: synthetic
                line_info.push(LineInfo {
                    block_id: Some(block.id.clone()),
                    content_start: 0,
                    prefix_len: 0,
                });
            }
            BlockType::Blockquote => {
                let text = format!("▎ {}", block.content);
                plain.push(text);
                styled.push(vec![
                    StyledSpan::new("▎ ", theme.blockquote_border),
                    StyledSpan::new(&block.content, theme.blockquote_text),
                ]);
                line_kinds.push(LineKind::Normal);
                // "▎ " prefix: 2 characters (▎ is a single char + space)
                line_info.push(LineInfo {
                    block_id: Some(block.id.clone()),
                    content_start: 0,
                    prefix_len: 2,
                });
            }
            BlockType::ListItem => {
                let indent = "  ".repeat(block.level);
                let indent_len = block.level * 2;
                let (marker_text, content_spans) = if let Some(checked) = block.checked {
                    let marker = if checked { "- [x] " } else { "- [ ] " };
                    (
                        marker.to_string(),
                        vec![
                            StyledSpan::new(&indent, Style::default()),
                            StyledSpan::new(marker, theme.checkbox),
                        ],
                    )
                } else if let Some(idx) = block.ordered_index {
                    let marker = format!("{idx}. ");
                    (
                        marker.clone(),
                        vec![
                            StyledSpan::new(&indent, Style::default()),
                            StyledSpan::new(&marker, theme.list_marker),
                        ],
                    )
                } else {
                    (
                        "- ".to_string(),
                        vec![
                            StyledSpan::new(&indent, Style::default()),
                            StyledSpan::new("- ", theme.list_marker),
                        ],
                    )
                };
                let marker_len = match (block.checked, block.ordered_index) {
                    (Some(_), _) => 6,                             // "- [x] " or "- [ ] "
                    (None, Some(idx)) => format!("{idx}. ").len(), // e.g. "1. "
                    (None, None) => 2,                             // "- "
                };
                let text = format!("{indent}{marker_text}{}", block.content);
                plain.push(text);
                let mut spans = content_spans;
                spans.extend(highlighter.highlight_line(&block.content));
                styled.push(spans);
                line_kinds.push(LineKind::Normal);
                line_info.push(LineInfo {
                    block_id: Some(block.id.clone()),
                    content_start: 0,
                    prefix_len: indent_len + marker_len,
                });
            }
            BlockType::HorizontalRule => {
                plain.push("───".to_string());
                styled.push(vec![StyledSpan::new("───", theme.hr)]);
                line_kinds.push(LineKind::HorizontalRule);
                line_info.push(LineInfo {
                    block_id: Some(block.id.clone()),
                    content_start: 0,
                    prefix_len: 0,
                });
            }
            BlockType::Table => {
                let mut offset = 0usize;
                for line in block.content.split('\n') {
                    plain.push(line.to_string());
                    styled.push(vec![StyledSpan::plain(line)]);
                    line_kinds.push(LineKind::TableRow);
                    line_info.push(LineInfo {
                        block_id: Some(block.id.clone()),
                        content_start: offset,
                        prefix_len: 0,
                    });
                    offset += line.chars().count() + 1;
                }
            }
        }
        // Blank line between blocks.
        plain.push(String::new());
        styled.push(vec![]);
        line_kinds.push(LineKind::Normal);
        line_info.push(LineInfo {
            block_id: None,
            content_start: 0,
            prefix_len: 0,
        });
    }

    // Remove trailing blank line.
    if plain.last().is_some_and(|l| l.is_empty()) {
        plain.pop();
        styled.pop();
        line_kinds.pop();
        line_info.pop();
    }

    DocumentLines {
        plain,
        styled,
        line_kinds,
        line_info,
    }
}

/// Apply width-dependent styling to a visible slice of styled lines
/// and convert them to ratatui `Line`s for rendering.
///
/// Handles: full-width HR, code block background padding, table formatting.
#[allow(clippy::too_many_arguments)]
pub fn prepare_visible_lines(
    styled_slice: &[Vec<StyledSpan>],
    plain_slice: &[String],
    line_types: &[LineKind],
    visible_start: usize,
    cursor_row: usize,
    cursor_col: usize,
    content_width: usize,
    theme: &Theme,
    selection: Option<(CursorPosition, CursorPosition)>,
) -> Vec<Line<'static>> {
    // Pre-compute table formatting for contiguous runs of TableRow lines.
    let table_lines = format_table_runs(plain_slice, line_types, content_width, theme);

    let mut result: Vec<Line<'static>> = Vec::with_capacity(styled_slice.len());

    for (i, (styled_spans, kind)) in styled_slice.iter().zip(line_types.iter()).enumerate() {
        let doc_row = visible_start + i;

        let line = match kind {
            LineKind::Normal => {
                let spans: Vec<Span> = styled_spans
                    .iter()
                    .map(|ss| Span::styled(ss.text.clone(), ss.style))
                    .collect();
                Line::from(spans)
            }
            LineKind::Code => {
                let mut spans: Vec<Span> = styled_spans
                    .iter()
                    .map(|ss| {
                        let mut style = ss.style;
                        style.bg = Some(theme.code_bg.bg.unwrap_or(ratatui::style::Color::Reset));
                        Span::styled(ss.text.clone(), style)
                    })
                    .collect();
                // Pad with spaces to fill content_width.
                let current_width: usize = styled_spans.iter().map(|ss| ss.text.len()).sum();
                if current_width < content_width {
                    spans.push(Span::styled(
                        " ".repeat(content_width - current_width),
                        theme.code_bg,
                    ));
                }
                Line::from(spans)
            }
            LineKind::HorizontalRule => {
                Line::from(Span::styled("─".repeat(content_width), theme.hr))
            }
            LineKind::TableRow => {
                if let Some(formatted_line) = table_lines.get(&i) {
                    formatted_line.clone()
                } else {
                    let spans: Vec<Span> = styled_spans
                        .iter()
                        .map(|ss| Span::styled(ss.text.clone(), ss.style))
                        .collect();
                    Line::from(spans)
                }
            }
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

/// Detect contiguous runs of `LineKind::TableRow` and format them.
/// Returns a map from slice index → formatted `Line`.
fn format_table_runs(
    plain_slice: &[String],
    line_types: &[LineKind],
    _content_width: usize,
    theme: &Theme,
) -> std::collections::HashMap<usize, Line<'static>> {
    let mut result = std::collections::HashMap::new();
    let mut i = 0;

    while i < line_types.len() {
        if line_types[i] != LineKind::TableRow {
            i += 1;
            continue;
        }

        // Collect contiguous table rows.
        let run_start = i;
        while i < line_types.len() && line_types[i] == LineKind::TableRow {
            i += 1;
        }
        let run_end = i;
        let run = &plain_slice[run_start..run_end];

        // Parse cells for each row.
        let parsed: Vec<Vec<String>> = run
            .iter()
            .map(|line| {
                let trimmed = line.trim();
                let inner = trimmed.strip_prefix('|').unwrap_or(trimmed);
                let inner = inner.strip_suffix('|').unwrap_or(inner);
                inner.split('|').map(|c| c.trim().to_string()).collect()
            })
            .collect();

        // Detect separator row and extract alignment.
        let separator_idx = parsed.iter().position(|row| {
            row.iter().all(|cell| {
                cell.chars().all(|c| c == '-' || c == ':' || c == ' ') && !cell.is_empty()
            })
        });

        let alignments: Vec<Alignment> = if let Some(sep_idx) = separator_idx {
            parsed[sep_idx]
                .iter()
                .map(|cell| {
                    let cell = cell.trim();
                    let starts_colon = cell.starts_with(':');
                    let ends_colon = cell.ends_with(':');
                    match (starts_colon, ends_colon) {
                        (true, true) => Alignment::Center,
                        (false, true) => Alignment::Right,
                        _ => Alignment::Left,
                    }
                })
                .collect()
        } else {
            vec![Alignment::Left; parsed.first().map_or(0, |r| r.len())]
        };

        // Calculate max column widths.
        let num_cols = parsed.iter().map(|r| r.len()).max().unwrap_or(0);
        let mut col_widths = vec![0usize; num_cols];
        for (row_idx, row) in parsed.iter().enumerate() {
            if Some(row_idx) == separator_idx {
                continue;
            }
            for (col_idx, cell) in row.iter().enumerate() {
                if col_idx < num_cols {
                    col_widths[col_idx] = col_widths[col_idx].max(cell.len());
                }
            }
        }

        // Render each row.
        for (row_idx, row) in parsed.iter().enumerate() {
            let slice_idx = run_start + row_idx;
            let is_header = row_idx == 0;
            let is_separator = Some(row_idx) == separator_idx;

            if is_separator {
                // Render separator with ─ characters and ┼ joints.
                let parts: Vec<String> = col_widths.iter().map(|&w| "─".repeat(w + 2)).collect();
                let sep_text = format!("│{}│", parts.join("┼"));
                result.insert(
                    slice_idx,
                    Line::from(Span::styled(sep_text, theme.table_border)),
                );
            } else {
                // Render data/header row with padded cells.
                let mut spans: Vec<Span<'static>> = Vec::new();
                let row_style = if is_header {
                    theme.table_header
                } else {
                    Style::default()
                };

                spans.push(Span::styled("│ ", theme.table_border));
                for (col_idx, col_width) in col_widths.iter().enumerate() {
                    let cell = row.get(col_idx).map(|s| s.as_str()).unwrap_or("");
                    let alignment = alignments.get(col_idx).copied().unwrap_or(Alignment::Left);
                    let padded = align_cell(cell, *col_width, alignment);

                    spans.push(Span::styled(padded, row_style));
                    if col_idx < num_cols - 1 {
                        spans.push(Span::styled(" │ ", theme.table_border));
                    }
                }
                spans.push(Span::styled(" │", theme.table_border));

                result.insert(slice_idx, Line::from(spans));
            }
        }
    }

    result
}

#[derive(Debug, Clone, Copy)]
enum Alignment {
    Left,
    Center,
    Right,
}

fn align_cell(text: &str, width: usize, alignment: Alignment) -> String {
    if text.len() >= width {
        return text.to_string();
    }
    let padding = width - text.len();
    match alignment {
        Alignment::Left => format!("{text}{}", " ".repeat(padding)),
        Alignment::Right => format!("{}{text}", " ".repeat(padding)),
        Alignment::Center => {
            let left_pad = padding / 2;
            let right_pad = padding - left_pad;
            format!("{}{text}{}", " ".repeat(left_pad), " ".repeat(right_pad))
        }
    }
}
