/// Document viewport managing scroll offset, cursor position, and visible line range.
///
/// The viewport operates on "document lines" — the final rendered lines that appear
/// on screen. Each parsed [`Block`] expands into one or more document lines (e.g. a
/// multi-line paragraph or code block). The mapping from blocks to document lines is
/// provided externally via [`DisplayLayout`].

/// Cursor position in the document (0-indexed row and column).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CursorPosition {
    pub row: usize,
    pub col: usize,
}

/// A single display row: a slice of a document line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DisplayRow {
    /// Index of the source document line.
    pub doc_row: usize,
    /// Inclusive start char index in the doc line.
    pub start_col: usize,
    /// Exclusive end char index in the doc line.
    pub end_col: usize,
}

/// A render slice passed to the renderer — one per visible display row.
#[derive(Debug, Clone)]
pub struct RenderSlice {
    pub doc_row: usize,
    pub start_col: usize,
    pub end_col: usize,
}

/// Maps document lines to display rows, handling word-wrap layout.
pub struct DisplayLayout {
    /// Flattened list of display rows.
    pub rows: Vec<DisplayRow>,
    /// For each doc row, the range of indices into `rows`.
    pub doc_row_ranges: Vec<std::ops::Range<usize>>,
}

impl DisplayLayout {
    /// Build a display layout from document line lengths and viewport width.
    /// When `wrap` is true, lines longer than `width` are split at word boundaries
    /// (falling back to hard breaks for long unbroken tokens).
    /// When `wrap` is false, each doc line maps to exactly one display row.
    pub fn build(doc_lines: &[String], width: usize, wrap: bool) -> Self {
        let mut rows = Vec::new();
        let mut doc_row_ranges = Vec::with_capacity(doc_lines.len());

        for (doc_row, line) in doc_lines.iter().enumerate() {
            let start_idx = rows.len();
            let char_count = line.chars().count();

            if !wrap || width == 0 || char_count <= width {
                // Single display row for this doc line.
                rows.push(DisplayRow {
                    doc_row,
                    start_col: 0,
                    end_col: char_count,
                });
            } else {
                // Word-wrap: split into chunks of at most `width` chars.
                // Use char_indices to avoid allocating a Vec<char>.
                let mut col = 0;
                while col < char_count {
                    let chunk_end = (col + width).min(char_count);
                    let break_at = if chunk_end < char_count {
                        // Search backward from chunk_end for a whitespace boundary.
                        // Walk the char_indices to find positions.
                        let mut last_ws = None;
                        let mut idx = 0;
                        for (_, ch) in line.chars().enumerate().skip(col).take(chunk_end - col) {
                            let abs_idx = col + idx;
                            if ch.is_whitespace() {
                                last_ws = Some(abs_idx + 1); // break after whitespace
                            }
                            idx += 1;
                        }
                        match last_ws {
                            Some(b) if b > col => b,
                            _ => chunk_end, // No whitespace found — hard break.
                        }
                    } else {
                        chunk_end
                    };
                    rows.push(DisplayRow {
                        doc_row,
                        start_col: col,
                        end_col: break_at,
                    });
                    col = break_at;
                }
            }

            let end_idx = rows.len();
            doc_row_ranges.push(start_idx..end_idx);
        }

        // Handle empty document.
        if rows.is_empty() {
            rows.push(DisplayRow {
                doc_row: 0,
                start_col: 0,
                end_col: 0,
            });
            doc_row_ranges.push(0..1);
        }

        Self {
            rows,
            doc_row_ranges,
        }
    }

    pub fn total_display_rows(&self) -> usize {
        self.rows.len()
    }

    /// Total number of document lines represented in this layout.
    pub fn total_doc_lines(&self) -> usize {
        self.doc_row_ranges.len()
    }

    /// Character length of the document line at the given row, or 0 if out of bounds.
    pub fn doc_line_length(&self, row: usize) -> usize {
        if row >= self.doc_row_ranges.len() {
            return 0;
        }
        let range = &self.doc_row_ranges[row];
        if range.is_empty() {
            return 0;
        }
        self.rows[range.end - 1].end_col
    }

    /// Convert a document cursor position to a (display_row_index, local_col) pair.
    pub fn display_pos_of_doc_pos(&self, pos: CursorPosition) -> (usize, usize) {
        if pos.row >= self.doc_row_ranges.len() {
            let last = self.rows.len().saturating_sub(1);
            return (last, 0);
        }
        let range = &self.doc_row_ranges[pos.row];
        for idx in range.clone() {
            let dr = &self.rows[idx];
            if pos.col < dr.end_col || idx + 1 == range.end {
                return (idx, pos.col.saturating_sub(dr.start_col));
            }
        }
        // Fallback: last display row for this doc row.
        let last = range.end.saturating_sub(1);
        let dr = &self.rows[last];
        (last, pos.col.saturating_sub(dr.start_col))
    }

    /// Convert a display row index and local column to document coordinates.
    pub fn doc_pos_of_display_pos(&self, display_row: usize, local_col: usize) -> CursorPosition {
        let dr = &self.rows[display_row.min(self.rows.len().saturating_sub(1))];
        CursorPosition {
            row: dr.doc_row,
            col: dr.start_col + local_col,
        }
    }
}

/// Viewport state: scroll offset, cursor, and dimensions.
pub struct Viewport {
    /// Vertical scroll offset (first visible display row).
    pub scroll_offset: usize,
    /// Cursor position in document coordinates.
    pub cursor: CursorPosition,
    /// Number of visible rows in the document area (excludes status bar, borders).
    pub height: usize,
    /// Number of visible columns in the document area.
    pub width: usize,
    /// Whether word-wrap is enabled.
    pub word_wrap: bool,
    /// Horizontal scroll offset (only used when word_wrap is false).
    horizontal_offset: usize,
}

impl Viewport {
    pub fn new() -> Self {
        Self {
            scroll_offset: 0,
            cursor: CursorPosition { row: 0, col: 0 },
            height: 0,
            width: 0,
            word_wrap: false,
            horizontal_offset: 0,
        }
    }

    /// Update the terminal dimensions available for the document area.
    pub fn set_dimensions(&mut self, width: usize, height: usize) {
        self.width = width;
        self.height = height;
    }

    // ── Movement ──────────────────────────────────────────────────

    pub fn move_up(&mut self, layout: &DisplayLayout) {
        let (disp_row, local_col) = layout.display_pos_of_doc_pos(self.cursor);
        if disp_row > 0 {
            let target = disp_row - 1;
            let dr = &layout.rows[target];
            let slice_len = dr.end_col.saturating_sub(dr.start_col);
            let clamped_local = local_col.min(slice_len.saturating_sub(1));
            self.cursor = layout.doc_pos_of_display_pos(target, clamped_local);
            self.clamp_col(layout);
            self.ensure_cursor_visible(layout);
        }
    }

    pub fn move_down(&mut self, layout: &DisplayLayout) {
        let (disp_row, local_col) = layout.display_pos_of_doc_pos(self.cursor);
        if disp_row + 1 < layout.total_display_rows() {
            let target = disp_row + 1;
            let dr = &layout.rows[target];
            let slice_len = dr.end_col.saturating_sub(dr.start_col);
            let clamped_local = local_col.min(slice_len.saturating_sub(1));
            self.cursor = layout.doc_pos_of_display_pos(target, clamped_local);
            self.clamp_col(layout);
            self.ensure_cursor_visible(layout);
        }
    }

    pub fn move_left(&mut self, layout: &DisplayLayout) {
        if self.cursor.col > 0 {
            self.cursor.col -= 1;
            self.ensure_cursor_visible(layout);
            self.ensure_horizontal_visible();
        }
    }

    pub fn move_right(&mut self, layout: &DisplayLayout) {
        let max_col = self.current_line_max_col(layout);
        if self.cursor.col < max_col {
            self.cursor.col += 1;
            self.ensure_cursor_visible(layout);
            self.ensure_horizontal_visible();
        }
    }

    pub fn move_line_start(&mut self, layout: &DisplayLayout) {
        self.cursor.col = 0;
        self.ensure_cursor_visible(layout);
        self.ensure_horizontal_visible();
    }

    pub fn move_line_end(&mut self, layout: &DisplayLayout) {
        self.cursor.col = self.current_line_max_col(layout);
        self.ensure_cursor_visible(layout);
        self.ensure_horizontal_visible();
    }

    pub fn move_word_forward(&mut self, lines: &[&str], layout: &DisplayLayout) {
        let total_lines = layout.total_doc_lines();
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

        if col >= chars.len() && self.cursor.row + 1 < total_lines {
            // Wrap to the start of the next line.
            self.cursor.row += 1;
            self.cursor.col = 0;
            // Skip leading whitespace on next line.
            if self.cursor.row < lines.len() {
                let mut nc = 0;
                for ch in lines[self.cursor.row].chars() {
                    if !ch.is_whitespace() {
                        break;
                    }
                    nc += 1;
                }
                self.cursor.col = nc;
            }
        } else {
            self.cursor.col = col.min(layout.doc_line_length(self.cursor.row));
        }

        self.clamp_col(layout);
        self.ensure_cursor_visible(layout);
        self.ensure_horizontal_visible();
    }

    pub fn move_word_end(&mut self, lines: &[&str], layout: &DisplayLayout) {
        let total_lines = layout.total_doc_lines();
        if lines.is_empty() || self.cursor.row >= lines.len() {
            return;
        }

        let line = lines[self.cursor.row];
        let chars: Vec<char> = line.chars().collect();
        let mut col = self.cursor.col;

        // Advance at least one character so we leave the current position.
        if col < chars.len() {
            col += 1;
        }

        // Skip whitespace.
        while col < chars.len() && chars[col].is_whitespace() {
            col += 1;
        }

        if col >= chars.len() {
            // Wrap to the next line if possible.
            if self.cursor.row + 1 < total_lines {
                self.cursor.row += 1;
                self.cursor.col = 0;
                if self.cursor.row < lines.len() {
                    let next_chars: Vec<char> = lines[self.cursor.row].chars().collect();
                    let mut nc = 0;
                    // Skip leading whitespace.
                    while nc < next_chars.len() && next_chars[nc].is_whitespace() {
                        nc += 1;
                    }
                    // Advance to end of word.
                    while nc < next_chars.len() && !next_chars[nc].is_whitespace() {
                        nc += 1;
                    }
                    self.cursor.col = if nc > 0 { nc - 1 } else { 0 };
                }
            } else {
                // Last line — stay at end.
                self.cursor.col = if chars.is_empty() {
                    0
                } else {
                    chars.len() - 1
                };
            }
        } else {
            // Advance to end of word.
            while col < chars.len() && !chars[col].is_whitespace() {
                col += 1;
            }
            self.cursor.col = col - 1;
        }

        self.clamp_col(layout);
        self.ensure_cursor_visible(layout);
        self.ensure_horizontal_visible();
    }

    pub fn move_word_backward(&mut self, lines: &[&str], layout: &DisplayLayout) {
        if lines.is_empty() || self.cursor.row >= lines.len() {
            return;
        }

        let line = lines[self.cursor.row];
        let chars: Vec<char> = line.chars().collect();

        if self.cursor.col == 0 {
            // Wrap to end of previous line.
            if self.cursor.row > 0 {
                self.cursor.row -= 1;
                self.cursor.col = self.current_line_max_col(layout);
            }
            self.ensure_cursor_visible(layout);
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
        self.ensure_cursor_visible(layout);
        self.ensure_horizontal_visible();
    }

    pub fn move_document_top(&mut self, layout: &DisplayLayout) {
        self.cursor.row = 0;
        self.cursor.col = 0;
        self.ensure_cursor_visible(layout);
    }

    pub fn move_document_bottom(&mut self, layout: &DisplayLayout) {
        let total_lines = layout.total_doc_lines();
        if total_lines > 0 {
            self.cursor.row = total_lines - 1;
        }
        self.clamp_col(layout);
        self.ensure_cursor_visible(layout);
    }

    pub fn half_page_down(&mut self, layout: &DisplayLayout) {
        let delta = self.height / 2;
        let (disp_row, local_col) = layout.display_pos_of_doc_pos(self.cursor);
        let target = (disp_row + delta).min(layout.total_display_rows().saturating_sub(1));
        let dr = &layout.rows[target];
        let slice_len = dr.end_col.saturating_sub(dr.start_col);
        let clamped_local = local_col.min(slice_len.saturating_sub(1));
        self.cursor = layout.doc_pos_of_display_pos(target, clamped_local);
        self.clamp_col(layout);
        self.ensure_cursor_visible(layout);
    }

    pub fn half_page_up(&mut self, layout: &DisplayLayout) {
        let delta = self.height / 2;
        let (disp_row, local_col) = layout.display_pos_of_doc_pos(self.cursor);
        let target = disp_row.saturating_sub(delta);
        let dr = &layout.rows[target];
        let slice_len = dr.end_col.saturating_sub(dr.start_col);
        let clamped_local = local_col.min(slice_len.saturating_sub(1));
        self.cursor = layout.doc_pos_of_display_pos(target, clamped_local);
        self.clamp_col(layout);
        self.ensure_cursor_visible(layout);
    }

    pub fn full_page_down(&mut self, layout: &DisplayLayout) {
        let (disp_row, local_col) = layout.display_pos_of_doc_pos(self.cursor);
        let target = (disp_row + self.height).min(layout.total_display_rows().saturating_sub(1));
        let dr = &layout.rows[target];
        let slice_len = dr.end_col.saturating_sub(dr.start_col);
        let clamped_local = local_col.min(slice_len.saturating_sub(1));
        self.cursor = layout.doc_pos_of_display_pos(target, clamped_local);
        self.clamp_col(layout);
        self.ensure_cursor_visible(layout);
    }

    pub fn full_page_up(&mut self, layout: &DisplayLayout) {
        let (disp_row, local_col) = layout.display_pos_of_doc_pos(self.cursor);
        let target = disp_row.saturating_sub(self.height);
        let dr = &layout.rows[target];
        let slice_len = dr.end_col.saturating_sub(dr.start_col);
        let clamped_local = local_col.min(slice_len.saturating_sub(1));
        self.cursor = layout.doc_pos_of_display_pos(target, clamped_local);
        self.clamp_col(layout);
        self.ensure_cursor_visible(layout);
    }

    // ── Visible range ─────────────────────────────────────────────

    /// Build the list of render slices for visible display rows.
    pub fn visible_render_slices(&self, layout: &DisplayLayout) -> Vec<RenderSlice> {
        let total = layout.total_display_rows();
        let start = self.scroll_offset.min(total);
        let end = (start + self.height).min(total);
        layout.rows[start..end]
            .iter()
            .map(|dr| {
                if self.word_wrap {
                    RenderSlice {
                        doc_row: dr.doc_row,
                        start_col: dr.start_col,
                        end_col: dr.end_col,
                    }
                } else {
                    let line_len = dr.end_col;
                    let start_col = self.horizontal_offset.min(line_len);
                    let end_col = (self.horizontal_offset + self.width).min(line_len);
                    RenderSlice {
                        doc_row: dr.doc_row,
                        start_col,
                        end_col,
                    }
                }
            })
            .collect()
    }

    /// Returns `true` if the terminal is too small to render the UI.
    pub fn is_too_small(&self) -> bool {
        self.width < 40 || self.height < 5
    }

    /// Row of the cursor relative to the viewport (for rendering).
    #[allow(dead_code)] // TODO: used when annotation gutter rendering is added
    pub fn cursor_viewport_row(&self, layout: &DisplayLayout) -> usize {
        let (disp_row, _) = layout.display_pos_of_doc_pos(self.cursor);
        disp_row.saturating_sub(self.scroll_offset)
    }

    /// Toggle word wrap on/off and reset horizontal offset.
    pub fn toggle_word_wrap(&mut self) {
        self.word_wrap = !self.word_wrap;
        if self.word_wrap {
            self.horizontal_offset = 0;
        }
    }

    // ── Internal helpers ──────────────────────────────────────────

    /// Clamp column to current line length.
    fn clamp_col(&mut self, layout: &DisplayLayout) {
        self.cursor.col = self.cursor.col.min(self.current_line_max_col(layout));
    }

    /// The maximum column index for the current line (0 for empty lines).
    fn current_line_max_col(&self, layout: &DisplayLayout) -> usize {
        layout
            .doc_line_length(self.cursor.row)
            .saturating_sub(1)
            .max(0)
    }

    /// Adjust scroll offset so the cursor is within the visible area (display-row-based).
    fn ensure_cursor_visible(&mut self, layout: &DisplayLayout) {
        if self.height == 0 {
            return;
        }
        let (disp_row, _) = layout.display_pos_of_doc_pos(self.cursor);
        if disp_row < self.scroll_offset {
            self.scroll_offset = disp_row;
        } else if disp_row >= self.scroll_offset + self.height {
            self.scroll_offset = disp_row - self.height + 1;
        }
    }

    /// Adjust horizontal offset so the cursor column is visible (non-wrap mode).
    fn ensure_horizontal_visible(&mut self) {
        if self.word_wrap || self.width == 0 {
            return;
        }
        if self.cursor.col < self.horizontal_offset {
            self.horizontal_offset = self.cursor.col;
        } else if self.cursor.col >= self.horizontal_offset + self.width {
            self.horizontal_offset = self.cursor.col - self.width + 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a layout for `n` lines of `line_len` characters each (no wrap).
    fn make_lines(n: usize, line_len: usize) -> (Vec<String>, DisplayLayout) {
        let lines: Vec<String> = (0..n).map(|_| "x".repeat(line_len)).collect();
        let layout = DisplayLayout::build(&lines, 80, false);
        (lines, layout)
    }

    fn make_viewport(total_lines: usize, height: usize) -> (Viewport, DisplayLayout) {
        let (_, layout) = make_lines(total_lines, 20);
        let mut v = Viewport::new();
        v.set_dimensions(80, height);
        (v, layout)
    }

    // ── Basic movement ────────────────────────────────────────────

    #[test]
    fn initial_cursor_at_origin() {
        let (v, _) = make_viewport(10, 5);
        assert_eq!(v.cursor, CursorPosition { row: 0, col: 0 });
        assert_eq!(v.scroll_offset, 0);
    }

    #[test]
    fn move_down_and_up() {
        let (mut v, layout) = make_viewport(10, 5);
        v.move_down(&layout);
        assert_eq!(v.cursor.row, 1);
        v.move_down(&layout);
        assert_eq!(v.cursor.row, 2);
        v.move_up(&layout);
        assert_eq!(v.cursor.row, 1);
    }

    #[test]
    fn move_down_stops_at_last_line() {
        let (mut v, layout) = make_viewport(3, 5);
        v.move_down(&layout);
        v.move_down(&layout);
        v.move_down(&layout); // should not go past 2
        assert_eq!(v.cursor.row, 2);
    }

    #[test]
    fn move_up_stops_at_zero() {
        let (mut v, layout) = make_viewport(5, 5);
        v.move_up(&layout); // already at 0
        assert_eq!(v.cursor.row, 0);
    }

    #[test]
    fn move_left_right() {
        let (mut v, layout) = make_viewport(5, 5);
        v.move_right(&layout);
        assert_eq!(v.cursor.col, 1);
        v.move_right(&layout);
        assert_eq!(v.cursor.col, 2);
        v.move_left(&layout);
        assert_eq!(v.cursor.col, 1);
    }

    #[test]
    fn move_left_stops_at_zero() {
        let (mut v, layout) = make_viewport(5, 5);
        v.move_left(&layout);
        assert_eq!(v.cursor.col, 0);
    }

    #[test]
    fn move_right_stops_at_line_end() {
        let lines = vec!["hello".to_string()];
        let layout = DisplayLayout::build(&lines, 80, false);
        let mut v = Viewport::new();
        v.set_dimensions(80, 10);
        for _ in 0..10 {
            v.move_right(&layout);
        }
        assert_eq!(v.cursor.col, 4); // max col = len - 1
    }

    #[test]
    fn move_line_start_end() {
        let lines = vec!["0123456789".to_string()];
        let layout = DisplayLayout::build(&lines, 80, false);
        let mut v = Viewport::new();
        v.set_dimensions(80, 10);
        v.move_right(&layout);
        v.move_right(&layout);
        v.move_right(&layout);
        assert_eq!(v.cursor.col, 3);
        v.move_line_end(&layout);
        assert_eq!(v.cursor.col, 9);
        v.move_line_start(&layout);
        assert_eq!(v.cursor.col, 0);
    }

    // ── Document top/bottom ───────────────────────────────────────

    #[test]
    fn move_document_top_bottom() {
        let (mut v, layout) = make_viewport(20, 5);
        v.move_document_bottom(&layout);
        assert_eq!(v.cursor.row, 19);
        v.move_document_top(&layout);
        assert_eq!(v.cursor.row, 0);
        assert_eq!(v.cursor.col, 0);
    }

    // ── Half/full page ────────────────────────────────────────────

    #[test]
    fn half_page_down_up() {
        let (mut v, layout) = make_viewport(20, 10);
        v.half_page_down(&layout);
        assert_eq!(v.cursor.row, 5);
        v.half_page_up(&layout);
        assert_eq!(v.cursor.row, 0);
    }

    #[test]
    fn full_page_down_up() {
        let (mut v, layout) = make_viewport(30, 10);
        v.full_page_down(&layout);
        assert_eq!(v.cursor.row, 10);
        v.full_page_up(&layout);
        assert_eq!(v.cursor.row, 0);
    }

    #[test]
    fn page_down_clamps_to_last_line() {
        let (mut v, layout) = make_viewport(5, 10);
        v.full_page_down(&layout);
        assert_eq!(v.cursor.row, 4);
    }

    // ── Scrolling ─────────────────────────────────────────────────

    #[test]
    fn scroll_follows_cursor_down() {
        let (mut v, layout) = make_viewport(20, 5);
        // Move cursor past the visible area.
        for _ in 0..7 {
            v.move_down(&layout);
        }
        assert_eq!(v.cursor.row, 7);
        // Scroll should have adjusted so cursor is visible.
        assert!(v.scroll_offset <= 7);
        let (disp_row, _) = layout.display_pos_of_doc_pos(v.cursor);
        assert!(disp_row < v.scroll_offset + v.height);
    }

    #[test]
    fn scroll_follows_cursor_up() {
        let (mut v, layout) = make_viewport(20, 5);
        // Go to bottom first.
        v.move_document_bottom(&layout);
        // Now move up past the visible area.
        for _ in 0..7 {
            v.move_up(&layout);
        }
        let (disp_row, _) = layout.display_pos_of_doc_pos(v.cursor);
        assert!(v.scroll_offset <= disp_row);
        assert!(disp_row < v.scroll_offset + v.height);
    }

    // ── Column clamping on row change ─────────────────────────────

    #[test]
    fn col_clamped_when_moving_to_shorter_line() {
        let lines = vec!["x".repeat(20), "x".repeat(5), "x".repeat(20)];
        let layout = DisplayLayout::build(&lines, 80, false);
        let mut v = Viewport::new();
        v.set_dimensions(80, 10);
        v.cursor.col = 15;
        v.move_down(&layout); // row 0→1, col should clamp to 4
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
        let lines_s = vec!["hello world".to_string()];
        let layout = DisplayLayout::build(&lines_s, 80, false);
        let mut v = Viewport::new();
        v.set_dimensions(80, 10);
        let lines = vec!["hello world"];
        v.move_word_forward(&lines, &layout);
        assert_eq!(v.cursor.col, 6); // start of "world"
    }

    #[test]
    fn word_forward_wraps_to_next_line() {
        let lines_s = vec!["hello".to_string(), "world".to_string()];
        let layout = DisplayLayout::build(&lines_s, 80, false);
        let mut v = Viewport::new();
        v.set_dimensions(80, 10);
        let lines = vec!["hello", "world"];
        v.cursor.col = 3;
        v.move_word_forward(&lines, &layout);
        // At end of first word, wrap to next line.
        assert_eq!(v.cursor.row, 1);
        assert_eq!(v.cursor.col, 0);
    }

    #[test]
    fn word_backward_within_line() {
        let lines_s = vec!["hello world".to_string()];
        let layout = DisplayLayout::build(&lines_s, 80, false);
        let mut v = Viewport::new();
        v.set_dimensions(80, 10);
        let lines = vec!["hello world"];
        v.cursor.col = 8;
        v.move_word_backward(&lines, &layout);
        assert_eq!(v.cursor.col, 6); // start of "world"
    }

    #[test]
    fn word_backward_wraps_to_prev_line() {
        let lines_s = vec!["hello".to_string(), "world".to_string()];
        let layout = DisplayLayout::build(&lines_s, 80, false);
        let mut v = Viewport::new();
        v.set_dimensions(80, 10);
        let lines = vec!["hello", "world"];
        v.cursor.row = 1;
        v.cursor.col = 0;
        v.move_word_backward(&lines, &layout);
        assert_eq!(v.cursor.row, 0);
        assert_eq!(v.cursor.col, 4); // end of "hello"
    }

    // ── Empty document ────────────────────────────────────────────

    #[test]
    fn empty_document_stays_at_origin() {
        let lines: Vec<String> = vec![];
        let layout = DisplayLayout::build(&lines, 80, false);
        let mut v = Viewport::new();
        v.set_dimensions(80, 10);
        v.move_down(&layout);
        v.move_right(&layout);
        assert_eq!(v.cursor, CursorPosition { row: 0, col: 0 });
    }

    // ── Cursor viewport row ───────────────────────────────────────

    #[test]
    fn cursor_viewport_row_with_scroll() {
        let (mut v, layout) = make_viewport(20, 5);
        v.move_document_bottom(&layout); // row 19, scroll_offset should be 15
        assert_eq!(v.cursor_viewport_row(&layout), 4); // 19 - 15 = 4
    }

    // ── DisplayLayout ─────────────────────────────────────────────

    #[test]
    fn layout_no_wrap_one_row_per_line() {
        let lines = vec!["short".to_string(), "another".to_string()];
        let layout = DisplayLayout::build(&lines, 80, false);
        assert_eq!(layout.total_display_rows(), 2);
        assert_eq!(
            layout.rows[0],
            DisplayRow {
                doc_row: 0,
                start_col: 0,
                end_col: 5
            }
        );
        assert_eq!(
            layout.rows[1],
            DisplayRow {
                doc_row: 1,
                start_col: 0,
                end_col: 7
            }
        );
    }

    #[test]
    fn layout_wrap_splits_long_line() {
        // "hello world foo" is 15 chars, width 10 should wrap.
        let lines = vec!["hello world foo".to_string()];
        let layout = DisplayLayout::build(&lines, 10, true);
        assert!(layout.total_display_rows() >= 2);
        // First row should break at a word boundary.
        assert_eq!(layout.rows[0].start_col, 0);
        // All rows combined should cover full line.
        let last = layout.rows.last().unwrap();
        assert_eq!(last.end_col, 15);
    }

    #[test]
    fn layout_display_pos_roundtrip() {
        let lines = vec!["hello world foo bar".to_string()];
        let layout = DisplayLayout::build(&lines, 10, true);
        let pos = CursorPosition { row: 0, col: 12 };
        let (disp_row, local_col) = layout.display_pos_of_doc_pos(pos);
        let back = layout.doc_pos_of_display_pos(disp_row, local_col);
        assert_eq!(back, pos);
    }

    #[test]
    fn layout_empty_doc() {
        let lines: Vec<String> = vec![];
        let layout = DisplayLayout::build(&lines, 80, true);
        assert_eq!(layout.total_display_rows(), 1);
    }
}
