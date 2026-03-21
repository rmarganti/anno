use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Flex, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
};

/// State for the modal input box widget.
#[derive(Debug, Clone)]
pub struct InputBox {
    /// The prompt label (e.g. "Comment:", "Replacement:", "Insertion:", "Global Comment:").
    pub prompt: String,
    /// The current text buffer.
    pub buffer: String,
    /// Cursor position within the buffer (character index).
    pub cursor: usize,
}

impl InputBox {
    pub fn new(prompt: impl Into<String>) -> Self {
        Self {
            prompt: prompt.into(),
            buffer: String::new(),
            cursor: 0,
        }
    }

    /// Insert a character at the cursor position.
    pub fn insert_char(&mut self, c: char) {
        let byte_idx = self
            .buffer
            .char_indices()
            .nth(self.cursor)
            .map(|(i, _)| i)
            .unwrap_or(self.buffer.len());
        self.buffer.insert(byte_idx, c);
        self.cursor += 1;
    }

    /// Delete the character before the cursor.
    pub fn backspace(&mut self) {
        if self.cursor > 0 {
            self.cursor -= 1;
            let byte_idx = self
                .buffer
                .char_indices()
                .nth(self.cursor)
                .map(|(i, _)| i)
                .unwrap_or(self.buffer.len());
            let next_byte = self
                .buffer
                .char_indices()
                .nth(self.cursor + 1)
                .map(|(i, _)| i)
                .unwrap_or(self.buffer.len());
            self.buffer.drain(byte_idx..next_byte);
        }
    }

    /// Move the cursor one character to the left.
    pub fn move_left(&mut self) {
        self.cursor = self.cursor.saturating_sub(1);
    }

    /// Move the cursor one character to the right.
    pub fn move_right(&mut self) {
        let len = self.buffer.chars().count();
        if self.cursor < len {
            self.cursor += 1;
        }
    }

    /// Return the current buffer contents.
    pub fn text(&self) -> &str {
        &self.buffer
    }

    /// Render the input box centered in the given area.
    pub fn render(&self, frame: &mut Frame, area: Rect) {
        let box_width = (area.width as usize * 2 / 3).max(30).min(area.width as usize) as u16;
        let box_height: u16 = 3;

        let [vert_area] =
            Layout::vertical([Constraint::Length(box_height)])
                .flex(Flex::Center)
                .areas(area);

        let [horiz_area] =
            Layout::horizontal([Constraint::Length(box_width)])
                .flex(Flex::Center)
                .areas(vert_area);

        // Clear the area behind the input box.
        frame.render_widget(Clear, horiz_area);

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan))
            .title(Span::styled(
                format!(" {} ", self.prompt),
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ))
            .title_alignment(Alignment::Left);

        let inner = block.inner(horiz_area);

        // Build the text line with a cursor indicator.
        let chars: Vec<char> = self.buffer.chars().collect();
        let before: String = chars[..self.cursor].iter().collect();
        let cursor_char = chars.get(self.cursor).copied().unwrap_or(' ');
        let after: String = if self.cursor < chars.len() {
            chars[self.cursor + 1..].iter().collect()
        } else {
            String::new()
        };

        let line = Line::from(vec![
            Span::raw(before),
            Span::styled(
                cursor_char.to_string(),
                Style::default().bg(Color::White).fg(Color::Black),
            ),
            Span::raw(after),
        ]);

        frame.render_widget(block, horiz_area);
        frame.render_widget(Paragraph::new(line), inner);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn insert_and_backspace() {
        let mut ib = InputBox::new("Test:");
        ib.insert_char('a');
        ib.insert_char('b');
        ib.insert_char('c');
        assert_eq!(ib.text(), "abc");
        assert_eq!(ib.cursor, 3);

        ib.backspace();
        assert_eq!(ib.text(), "ab");
        assert_eq!(ib.cursor, 2);
    }

    #[test]
    fn cursor_movement() {
        let mut ib = InputBox::new("Test:");
        ib.insert_char('a');
        ib.insert_char('b');
        ib.insert_char('c');

        ib.move_left();
        assert_eq!(ib.cursor, 2);
        ib.move_left();
        assert_eq!(ib.cursor, 1);
        ib.move_left();
        assert_eq!(ib.cursor, 0);
        // Should not go below 0
        ib.move_left();
        assert_eq!(ib.cursor, 0);

        ib.move_right();
        assert_eq!(ib.cursor, 1);
    }

    #[test]
    fn insert_at_middle() {
        let mut ib = InputBox::new("Test:");
        ib.insert_char('a');
        ib.insert_char('c');
        ib.move_left();
        ib.insert_char('b');
        assert_eq!(ib.text(), "abc");
        assert_eq!(ib.cursor, 2);
    }

    #[test]
    fn backspace_at_start_is_noop() {
        let mut ib = InputBox::new("Test:");
        ib.backspace();
        assert_eq!(ib.text(), "");
        assert_eq!(ib.cursor, 0);
    }

    #[test]
    fn move_right_does_not_exceed_length() {
        let mut ib = InputBox::new("Test:");
        ib.insert_char('a');
        ib.move_right();
        assert_eq!(ib.cursor, 1); // stays at 1, length is 1
    }
}
