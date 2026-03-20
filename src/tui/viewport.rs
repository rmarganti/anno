/// Document viewport managing scroll offset, cursor position, and visible line range.
///
/// The viewport operates on "document lines" — the final rendered lines that appear
/// on screen. Each parsed [`Block`] expands into one or more document lines (e.g. a
/// multi-line paragraph or code block). The mapping from blocks to document lines is
/// provided externally via [`Viewport::set_total_lines`].

/// Cursor position in the document (0-indexed row and column).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CursorPosition {
    pub row: usize,
    pub col: usize,
}

/// Viewport state: scroll offset, cursor, and dimensions.
pub struct Viewport {
    /// Vertical scroll offset (first visible document line).
    pub scroll_offset: usize,
    /// Cursor position in document coordinates.
    pub cursor: CursorPosition,
    /// Number of visible rows in the document area (excludes status bar, borders).
    pub height: usize,
    /// Number of visible columns in the document area.
    pub width: usize,
    /// Total number of document lines.
    pub total_lines: usize,
    /// Lengths (in characters) of each document line — used for horizontal clamping.
    pub line_lengths: Vec<usize>,
}

impl Viewport {
    pub fn new() -> Self {
        Self {
            scroll_offset: 0,
            cursor: CursorPosition { row: 0, col: 0 },
            height: 0,
            width: 0,
            total_lines: 0,
            line_lengths: Vec::new(),
        }
    }

    /// Update the terminal dimensions available for the document area.
    pub fn set_dimensions(&mut self, width: usize, height: usize) {
        self.width = width;
        self.height = height;
        self.ensure_cursor_visible();
    }

    /// Update the total number of document lines and their lengths.
    pub fn set_line_info(&mut self, line_lengths: Vec<usize>) {
        self.total_lines = line_lengths.len();
        self.line_lengths = line_lengths;
        self.clamp_cursor();
        self.ensure_cursor_visible();
    }

    // ── Movement ──────────────────────────────────────────────────

    pub fn move_up(&mut self) {
        if self.cursor.row > 0 {
            self.cursor.row -= 1;
            self.clamp_col();
            self.ensure_cursor_visible();
        }
    }

    pub fn move_down(&mut self) {
        if self.total_lines > 0 && self.cursor.row < self.total_lines - 1 {
            self.cursor.row += 1;
            self.clamp_col();
            self.ensure_cursor_visible();
        }
    }

    pub fn move_left(&mut self) {
        if self.cursor.col > 0 {
            self.cursor.col -= 1;
        }
    }

    pub fn move_right(&mut self) {
        let max_col = self.current_line_max_col();
        if self.cursor.col < max_col {
            self.cursor.col += 1;
        }
    }

    pub fn move_line_start(&mut self) {
        self.cursor.col = 0;
    }

    pub fn move_line_end(&mut self) {
        self.cursor.col = self.current_line_max_col();
    }

    pub fn move_word_forward(&mut self, lines: &[&str]) {
        if lines.is_empty() || self.cursor.row >= lines.len() {
            return;
        }

        let line = lines[self.cursor.row];
        let chars: Vec<char> = line.chars().collect();
        let mut col = self.cursor.col;

        // Skip current word characters.
        while col < chars.len() && !chars[col].is_whitespace() {
            col += 1;
        }
        // Skip whitespace.
        while col < chars.len() && chars[col].is_whitespace() {
            col += 1;
        }

        if col >= chars.len() && self.cursor.row + 1 < self.total_lines {
            // Wrap to the start of the next line.
            self.cursor.row += 1;
            self.cursor.col = 0;
            // Skip leading whitespace on next line.
            if self.cursor.row < lines.len() {
                let next_chars: Vec<char> = lines[self.cursor.row].chars().collect();
                let mut nc = 0;
                while nc < next_chars.len() && next_chars[nc].is_whitespace() {
                    nc += 1;
                }
                self.cursor.col = nc;
            }
        } else {
            self.cursor.col = col.min(self.line_len_at(self.cursor.row));
        }

        self.clamp_col();
        self.ensure_cursor_visible();
    }

    pub fn move_word_backward(&mut self, lines: &[&str]) {
        if lines.is_empty() || self.cursor.row >= lines.len() {
            return;
        }

        let line = lines[self.cursor.row];
        let chars: Vec<char> = line.chars().collect();

        if self.cursor.col == 0 {
            // Wrap to end of previous line.
            if self.cursor.row > 0 {
                self.cursor.row -= 1;
                self.cursor.col = self.current_line_max_col();
            }
            self.ensure_cursor_visible();
            return;
        }

        let mut col = self.cursor.col;

        // Move back past whitespace.
        while col > 0 && chars[col - 1].is_whitespace() {
            col -= 1;
        }
        // Move back past word characters.
        while col > 0 && !chars[col - 1].is_whitespace() {
            col -= 1;
        }

        self.cursor.col = col;
        self.ensure_cursor_visible();
    }

    pub fn move_document_top(&mut self) {
        self.cursor.row = 0;
        self.cursor.col = 0;
        self.ensure_cursor_visible();
    }

    pub fn move_document_bottom(&mut self) {
        if self.total_lines > 0 {
            self.cursor.row = self.total_lines - 1;
        }
        self.clamp_col();
        self.ensure_cursor_visible();
    }

    pub fn half_page_down(&mut self) {
        let delta = self.height / 2;
        self.cursor.row = (self.cursor.row + delta).min(self.total_lines.saturating_sub(1));
        self.clamp_col();
        self.ensure_cursor_visible();
    }

    pub fn half_page_up(&mut self) {
        let delta = self.height / 2;
        self.cursor.row = self.cursor.row.saturating_sub(delta);
        self.clamp_col();
        self.ensure_cursor_visible();
    }

    pub fn full_page_down(&mut self) {
        self.cursor.row =
            (self.cursor.row + self.height).min(self.total_lines.saturating_sub(1));
        self.clamp_col();
        self.ensure_cursor_visible();
    }

    pub fn full_page_up(&mut self) {
        self.cursor.row = self.cursor.row.saturating_sub(self.height);
        self.clamp_col();
        self.ensure_cursor_visible();
    }

    // ── Visible range ─────────────────────────────────────────────

    /// The range of document lines currently visible: `start..end` (exclusive end).
    pub fn visible_range(&self) -> std::ops::Range<usize> {
        let start = self.scroll_offset;
        let end = (start + self.height).min(self.total_lines);
        start..end
    }

    /// Returns `true` if the terminal is too small to render the UI.
    pub fn is_too_small(&self) -> bool {
        self.width < 40 || self.height < 5
    }

    /// Row of the cursor relative to the viewport (for rendering).
    #[allow(dead_code)] // TODO: used when annotation gutter rendering is added
    pub fn cursor_viewport_row(&self) -> usize {
        self.cursor.row.saturating_sub(self.scroll_offset)
    }

    // ── Internal helpers ──────────────────────────────────────────

    /// Ensure the cursor stays within document bounds.
    fn clamp_cursor(&mut self) {
        if self.total_lines == 0 {
            self.cursor.row = 0;
            self.cursor.col = 0;
            return;
        }
        if self.cursor.row >= self.total_lines {
            self.cursor.row = self.total_lines - 1;
        }
        self.clamp_col();
    }

    /// Clamp column to current line length.
    fn clamp_col(&mut self) {
        self.cursor.col = self.cursor.col.min(self.current_line_max_col());
    }

    /// The maximum column index for the current line (0 for empty lines).
    fn current_line_max_col(&self) -> usize {
        self.line_len_at(self.cursor.row).saturating_sub(1).max(0)
    }

    /// Character length of the line at the given row, or 0 if out of bounds.
    fn line_len_at(&self, row: usize) -> usize {
        self.line_lengths.get(row).copied().unwrap_or(0)
    }

    /// Adjust scroll offset so the cursor is within the visible area.
    fn ensure_cursor_visible(&mut self) {
        if self.height == 0 {
            return;
        }
        if self.cursor.row < self.scroll_offset {
            self.scroll_offset = self.cursor.row;
        } else if self.cursor.row >= self.scroll_offset + self.height {
            self.scroll_offset = self.cursor.row - self.height + 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_viewport(total_lines: usize, height: usize) -> Viewport {
        let mut v = Viewport::new();
        let lengths = vec![20usize; total_lines];
        v.set_dimensions(80, height);
        v.set_line_info(lengths);
        v
    }

    // ── Basic movement ────────────────────────────────────────────

    #[test]
    fn initial_cursor_at_origin() {
        let v = make_viewport(10, 5);
        assert_eq!(v.cursor, CursorPosition { row: 0, col: 0 });
        assert_eq!(v.scroll_offset, 0);
    }

    #[test]
    fn move_down_and_up() {
        let mut v = make_viewport(10, 5);
        v.move_down();
        assert_eq!(v.cursor.row, 1);
        v.move_down();
        assert_eq!(v.cursor.row, 2);
        v.move_up();
        assert_eq!(v.cursor.row, 1);
    }

    #[test]
    fn move_down_stops_at_last_line() {
        let mut v = make_viewport(3, 5);
        v.move_down();
        v.move_down();
        v.move_down(); // should not go past 2
        assert_eq!(v.cursor.row, 2);
    }

    #[test]
    fn move_up_stops_at_zero() {
        let mut v = make_viewport(5, 5);
        v.move_up(); // already at 0
        assert_eq!(v.cursor.row, 0);
    }

    #[test]
    fn move_left_right() {
        let mut v = make_viewport(5, 5);
        v.move_right();
        assert_eq!(v.cursor.col, 1);
        v.move_right();
        assert_eq!(v.cursor.col, 2);
        v.move_left();
        assert_eq!(v.cursor.col, 1);
    }

    #[test]
    fn move_left_stops_at_zero() {
        let mut v = make_viewport(5, 5);
        v.move_left();
        assert_eq!(v.cursor.col, 0);
    }

    #[test]
    fn move_right_stops_at_line_end() {
        let mut v = Viewport::new();
        v.set_dimensions(80, 10);
        v.set_line_info(vec![5]); // line of length 5
        for _ in 0..10 {
            v.move_right();
        }
        assert_eq!(v.cursor.col, 4); // max col = len - 1
    }

    #[test]
    fn move_line_start_end() {
        let mut v = Viewport::new();
        v.set_dimensions(80, 10);
        v.set_line_info(vec![10]);
        v.move_right();
        v.move_right();
        v.move_right();
        assert_eq!(v.cursor.col, 3);
        v.move_line_end();
        assert_eq!(v.cursor.col, 9);
        v.move_line_start();
        assert_eq!(v.cursor.col, 0);
    }

    // ── Document top/bottom ───────────────────────────────────────

    #[test]
    fn move_document_top_bottom() {
        let mut v = make_viewport(20, 5);
        v.move_document_bottom();
        assert_eq!(v.cursor.row, 19);
        v.move_document_top();
        assert_eq!(v.cursor.row, 0);
        assert_eq!(v.cursor.col, 0);
    }

    // ── Half/full page ────────────────────────────────────────────

    #[test]
    fn half_page_down_up() {
        let mut v = make_viewport(20, 10);
        v.half_page_down();
        assert_eq!(v.cursor.row, 5);
        v.half_page_up();
        assert_eq!(v.cursor.row, 0);
    }

    #[test]
    fn full_page_down_up() {
        let mut v = make_viewport(30, 10);
        v.full_page_down();
        assert_eq!(v.cursor.row, 10);
        v.full_page_up();
        assert_eq!(v.cursor.row, 0);
    }

    #[test]
    fn page_down_clamps_to_last_line() {
        let mut v = make_viewport(5, 10);
        v.full_page_down();
        assert_eq!(v.cursor.row, 4);
    }

    // ── Scrolling ─────────────────────────────────────────────────

    #[test]
    fn scroll_follows_cursor_down() {
        let mut v = make_viewport(20, 5);
        // Move cursor past the visible area.
        for _ in 0..7 {
            v.move_down();
        }
        assert_eq!(v.cursor.row, 7);
        // Scroll should have adjusted so cursor is visible.
        assert!(v.scroll_offset <= v.cursor.row);
        assert!(v.cursor.row < v.scroll_offset + v.height);
    }

    #[test]
    fn scroll_follows_cursor_up() {
        let mut v = make_viewport(20, 5);
        // Go to bottom first.
        v.move_document_bottom();
        // Now move up past the visible area.
        for _ in 0..7 {
            v.move_up();
        }
        assert!(v.scroll_offset <= v.cursor.row);
        assert!(v.cursor.row < v.scroll_offset + v.height);
    }

    // ── Visible range ─────────────────────────────────────────────

    #[test]
    fn visible_range_basic() {
        let v = make_viewport(20, 5);
        assert_eq!(v.visible_range(), 0..5);
    }

    #[test]
    fn visible_range_short_document() {
        let v = make_viewport(3, 10);
        assert_eq!(v.visible_range(), 0..3);
    }

    // ── Column clamping on row change ─────────────────────────────

    #[test]
    fn col_clamped_when_moving_to_shorter_line() {
        let mut v = Viewport::new();
        v.set_dimensions(80, 10);
        v.set_line_info(vec![20, 5, 20]); // line 1 is short
        v.cursor.col = 15;
        v.move_down(); // row 0→1, col should clamp to 4
        assert_eq!(v.cursor.col, 4);
    }

    // ── Too-small detection ───────────────────────────────────────

    #[test]
    fn too_small_detection() {
        let mut v = Viewport::new();
        v.set_dimensions(39, 10);
        assert!(v.is_too_small());
        v.set_dimensions(40, 4);
        assert!(v.is_too_small());
        v.set_dimensions(40, 5);
        assert!(!v.is_too_small());
    }

    // ── Word movement ─────────────────────────────────────────────

    #[test]
    fn word_forward_within_line() {
        let mut v = Viewport::new();
        v.set_dimensions(80, 10);
        v.set_line_info(vec![11]); // "hello world"
        let lines = vec!["hello world"];
        v.move_word_forward(&lines);
        assert_eq!(v.cursor.col, 6); // start of "world"
    }

    #[test]
    fn word_forward_wraps_to_next_line() {
        let mut v = Viewport::new();
        v.set_dimensions(80, 10);
        v.set_line_info(vec![5, 5]); // "hello", "world"
        let lines = vec!["hello", "world"];
        v.cursor.col = 3;
        v.move_word_forward(&lines);
        // At end of first word, wrap to next line.
        assert_eq!(v.cursor.row, 1);
        assert_eq!(v.cursor.col, 0);
    }

    #[test]
    fn word_backward_within_line() {
        let mut v = Viewport::new();
        v.set_dimensions(80, 10);
        v.set_line_info(vec![11]); // "hello world"
        let lines = vec!["hello world"];
        v.cursor.col = 8;
        v.move_word_backward(&lines);
        assert_eq!(v.cursor.col, 6); // start of "world"
    }

    #[test]
    fn word_backward_wraps_to_prev_line() {
        let mut v = Viewport::new();
        v.set_dimensions(80, 10);
        v.set_line_info(vec![5, 5]);
        let lines = vec!["hello", "world"];
        v.cursor.row = 1;
        v.cursor.col = 0;
        v.move_word_backward(&lines);
        assert_eq!(v.cursor.row, 0);
        assert_eq!(v.cursor.col, 4); // end of "hello"
    }

    // ── Empty document ────────────────────────────────────────────

    #[test]
    fn empty_document_stays_at_origin() {
        let mut v = Viewport::new();
        v.set_dimensions(80, 10);
        v.set_line_info(vec![]);
        v.move_down();
        v.move_right();
        assert_eq!(v.cursor, CursorPosition { row: 0, col: 0 });
    }

    // ── Cursor viewport row ───────────────────────────────────────

    #[test]
    fn cursor_viewport_row_with_scroll() {
        let mut v = make_viewport(20, 5);
        v.move_document_bottom(); // row 19, scroll_offset should be 15
        assert_eq!(v.cursor_viewport_row(), 4); // 19 - 15 = 4
    }
}
