use crate::tui::renderer::LineInfo;
use crate::tui::viewport::CursorPosition;

/// Active visual-mode selection. The selection spans from `anchor` to the
/// current viewport cursor position (inclusive), in whichever order they
/// appear in the document.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Selection {
    /// The position where `v` was pressed.
    pub anchor: CursorPosition,
}

impl Selection {
    /// Returns `(start, end)` in document order (start ≤ end by row, then col).
    pub fn range(&self, cursor: CursorPosition) -> (CursorPosition, CursorPosition) {
        let a = self.anchor;
        let b = cursor;
        if (a.row, a.col) <= (b.row, b.col) {
            (a, b)
        } else {
            (b, a)
        }
    }
}

/// Extract the plain text of the selection, stripping block prefixes.
/// `start` and `end` must already be in document order (`start ≤ end`).
pub fn selected_text(
    start: CursorPosition,
    end: CursorPosition,
    doc_lines: &[String],
    line_info: &[LineInfo],
) -> String {
    let mut result = String::new();

    for row in start.row..=end.row {
        if row >= doc_lines.len() {
            break;
        }
        let line = &doc_lines[row];
        let prefix_len = line_info.get(row).map_or(0, |i| i.prefix_len);
        let chars: Vec<char> = line.chars().collect();

        let col_start = if row == start.row { start.col } else { 0 };
        let col_end = if row == end.row {
            end.col
        } else {
            chars.len().saturating_sub(1)
        };

        // Map column positions to content area (skip prefix chars).
        let cs = col_start.saturating_sub(prefix_len);
        let ce = col_end.saturating_sub(prefix_len);

        let content_chars: Vec<char> = chars.iter().skip(prefix_len).cloned().collect();
        if !content_chars.is_empty() {
            let lo = cs.min(content_chars.len().saturating_sub(1));
            let hi = ce.min(content_chars.len().saturating_sub(1));
            result.push_str(&content_chars[lo..=hi].iter().collect::<String>());
        }
        if row < end.row {
            result.push('\n');
        }
    }

    result
}

/// Returns the list of `(block_id, start_offset, end_offset)` spans covered by
/// the selection, one entry per block. Synthetic lines (None block_id) are skipped.
///
/// Used for annotation creation (step 12), but defined here to keep the module complete.
pub fn block_ranges(
    start: CursorPosition,
    end: CursorPosition,
    doc_lines: &[String],
    line_info: &[LineInfo],
) -> Vec<(String, usize, usize)> {
    // We collect (block_id, start_offset, end_offset) for each block spanned.
    // Multiple rows can belong to the same block (e.g. paragraphs with embedded newlines).
    // We track the first and last encountered offsets per block in order.
    let mut result: Vec<(String, usize, usize)> = Vec::new();

    // Keep track of which block_id we last emitted, to merge adjacent rows of same block.
    let mut current_block: Option<(String, usize, usize)> = None;

    for row in start.row..=end.row {
        if row >= doc_lines.len() {
            break;
        }
        let info = match line_info.get(row) {
            Some(i) => i,
            None => continue,
        };
        let block_id = match &info.block_id {
            Some(id) => id.clone(),
            None => {
                // Blank separator line — flush current block if any.
                if let Some(blk) = current_block.take() {
                    result.push(blk);
                }
                continue;
            }
        };

        let line = &doc_lines[row];
        let chars: Vec<char> = line.chars().collect();
        let prefix_len = info.prefix_len;

        let col_start = if row == start.row { start.col } else { 0 };
        let col_end = if row == end.row {
            end.col
        } else {
            chars.len().saturating_sub(1)
        };

        // Convert column → content offset.
        let cs = col_start.saturating_sub(prefix_len);
        let ce = col_end.saturating_sub(prefix_len);

        let row_start_offset = info.content_start + cs;
        let row_end_offset = info.content_start + ce;

        match current_block.as_mut() {
            Some(blk) if blk.0 == block_id => {
                // Extend the end offset within the same block.
                blk.2 = row_end_offset;
            }
            _ => {
                // New block — flush previous if any.
                if let Some(blk) = current_block.take() {
                    result.push(blk);
                }
                current_block = Some((block_id, row_start_offset, row_end_offset));
            }
        }
    }

    // Flush the last block.
    if let Some(blk) = current_block {
        result.push(blk);
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pos(row: usize, col: usize) -> CursorPosition {
        CursorPosition { row, col }
    }

    fn info(block_id: Option<&str>, content_start: usize, prefix_len: usize) -> LineInfo {
        LineInfo {
            block_id: block_id.map(|s| s.to_string()),
            content_start,
            prefix_len,
        }
    }

    // ── Selection::range ─────────────────────────────────────────────

    #[test]
    fn range_anchor_before_cursor() {
        let sel = Selection { anchor: pos(2, 3) };
        assert_eq!(sel.range(pos(4, 1)), (pos(2, 3), pos(4, 1)));
    }

    #[test]
    fn range_cursor_before_anchor() {
        let sel = Selection { anchor: pos(4, 1) };
        assert_eq!(sel.range(pos(2, 3)), (pos(2, 3), pos(4, 1)));
    }

    #[test]
    fn range_same_row_anchor_before() {
        let sel = Selection { anchor: pos(1, 2) };
        assert_eq!(sel.range(pos(1, 5)), (pos(1, 2), pos(1, 5)));
    }

    #[test]
    fn range_same_row_cursor_before() {
        let sel = Selection { anchor: pos(1, 5) };
        assert_eq!(sel.range(pos(1, 2)), (pos(1, 2), pos(1, 5)));
    }

    // ── selected_text ────────────────────────────────────────────────

    #[test]
    fn single_line_no_prefix() {
        let doc_lines = vec!["hello world".to_string()];
        let line_info = vec![info(Some("b1"), 0, 0)];
        let text = selected_text(pos(0, 0), pos(0, 4), &doc_lines, &line_info);
        assert_eq!(text, "hello");
    }

    #[test]
    fn single_line_with_prefix() {
        // "▎ blockquote text" — prefix_len = 2
        let doc_lines = vec!["▎ blockquote".to_string()];
        let line_info = vec![info(Some("b1"), 0, 2)];
        // Column 2 is 'b', column 11 is 'e'
        let text = selected_text(pos(0, 2), pos(0, 11), &doc_lines, &line_info);
        assert_eq!(text, "blockquote");
    }

    #[test]
    fn single_line_cursor_inside_prefix() {
        // If cursor starts inside the prefix, we clamp to content start.
        let doc_lines = vec!["▎ hello".to_string()];
        let line_info = vec![info(Some("b1"), 0, 2)];
        let text = selected_text(pos(0, 0), pos(0, 6), &doc_lines, &line_info);
        // prefix_len=2, col_start=0 → cs=0, col_end=6 → ce=4; content="hello"[0..=4]
        assert_eq!(text, "hello");
    }

    #[test]
    fn multi_line_selection_no_prefix() {
        let doc_lines = vec!["first line".to_string(), "second line".to_string()];
        let line_info = vec![info(Some("b1"), 0, 0), info(Some("b1"), 11, 0)];
        // row=0 col 6..=9 → "line", row=1 col 0..=5 → "second"
        let text = selected_text(pos(0, 6), pos(1, 5), &doc_lines, &line_info);
        assert_eq!(text, "line\nsecond");
    }

    #[test]
    fn selection_on_blank_separator_line() {
        let doc_lines = vec!["".to_string()];
        let line_info = vec![info(None, 0, 0)];
        let text = selected_text(pos(0, 0), pos(0, 0), &doc_lines, &line_info);
        // Empty content — result should be empty.
        assert_eq!(text, "");
    }

    // ── block_ranges ─────────────────────────────────────────────────

    #[test]
    fn block_ranges_single_block() {
        let doc_lines = vec!["hello world".to_string()];
        let line_info = vec![info(Some("block1"), 0, 0)];
        let ranges = block_ranges(pos(0, 0), pos(0, 4), &doc_lines, &line_info);
        assert_eq!(ranges, vec![("block1".to_string(), 0, 4)]);
    }

    #[test]
    fn block_ranges_skips_blank_separator() {
        let doc_lines = vec![
            "para one".to_string(),
            "".to_string(),
            "para two".to_string(),
        ];
        let line_info = vec![
            info(Some("b1"), 0, 0),
            info(None, 0, 0),
            info(Some("b2"), 0, 0),
        ];
        let ranges = block_ranges(pos(0, 0), pos(2, 4), &doc_lines, &line_info);
        assert_eq!(
            ranges,
            vec![
                ("b1".to_string(), 0, 7), // "para one" last col = len-1 = 7
                ("b2".to_string(), 0, 4),
            ]
        );
    }

    #[test]
    fn block_ranges_with_prefix() {
        // List item: "- hello", prefix_len=2
        let doc_lines = vec!["- hello".to_string()];
        let line_info = vec![info(Some("list1"), 0, 2)];
        // col_start=2 → cs=0, col_end=6 → ce=4
        let ranges = block_ranges(pos(0, 2), pos(0, 6), &doc_lines, &line_info);
        assert_eq!(ranges, vec![("list1".to_string(), 0, 4)]);
    }
}
