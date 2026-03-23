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

/// Extract the plain text of the selection.
/// `start` and `end` must already be in document order (`start ≤ end`).
pub fn selected_text(start: CursorPosition, end: CursorPosition, doc_lines: &[String]) -> String {
    let mut result = String::new();

    for row in start.row..=end.row {
        if row >= doc_lines.len() {
            break;
        }
        let line = &doc_lines[row];
        let chars: Vec<char> = line.chars().collect();

        let col_start = if row == start.row { start.col } else { 0 };
        let col_end = if row == end.row {
            end.col
        } else {
            chars.len().saturating_sub(1)
        };

        if !chars.is_empty() {
            let lo = col_start.min(chars.len().saturating_sub(1));
            let hi = col_end.min(chars.len().saturating_sub(1));
            result.push_str(&chars[lo..=hi].iter().collect::<String>());
        }
        if row < end.row {
            result.push('\n');
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pos(row: usize, col: usize) -> CursorPosition {
        CursorPosition { row, col }
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
    fn single_line_selection() {
        let doc_lines = vec!["hello world".to_string()];
        let text = selected_text(pos(0, 0), pos(0, 4), &doc_lines);
        assert_eq!(text, "hello");
    }

    #[test]
    fn multi_line_selection() {
        let doc_lines = vec!["first line".to_string(), "second line".to_string()];
        let text = selected_text(pos(0, 6), pos(1, 5), &doc_lines);
        assert_eq!(text, "line\nsecond");
    }

    #[test]
    fn selection_on_empty_line() {
        let doc_lines = vec!["".to_string()];
        let text = selected_text(pos(0, 0), pos(0, 0), &doc_lines);
        assert_eq!(text, "");
    }
}
