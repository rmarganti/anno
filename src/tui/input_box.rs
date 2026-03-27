use crossterm::event::KeyEvent;
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Flex, Layout, Rect},
    style::Style,
    text::Span,
    widgets::{Block, Borders, Clear},
};
use tui_textarea::{Input, Key, TextArea};

use crate::tui::theme::Theme;

/// State for the modal input box widget, backed by `tui-textarea`.
#[derive(Debug, Clone)]
pub struct InputBox<'a> {
    /// The prompt label (e.g. "Comment", "Replacement", "Insertion", "Global Comment").
    prompt: String,
    /// The underlying textarea editor.
    textarea: TextArea<'a>,
}

/// Result of forwarding a key event to the input box.
pub enum InputBoxEvent {
    /// The key was consumed by the textarea (normal editing).
    Consumed,
    /// The user confirmed input (Ctrl+S).
    Confirm,
    /// The user cancelled input (Esc).
    Cancel,
}

impl<'a> InputBox<'a> {
    pub fn new(prompt: impl Into<String>) -> Self {
        let mut textarea = TextArea::default();
        textarea.set_cursor_line_style(Style::default());
        textarea.set_tab_length(4);
        Self {
            prompt: prompt.into(),
            textarea,
        }
    }

    /// Forward a crossterm key event to the textarea.
    /// Returns the semantic result of the key press.
    pub fn handle_key(&mut self, key_event: KeyEvent) -> InputBoxEvent {
        let input = Input::from(key_event);
        match input {
            Input { key: Key::Esc, .. } => InputBoxEvent::Cancel,
            Input {
                key: Key::Char('s'),
                ctrl: true,
                ..
            } => InputBoxEvent::Confirm,
            _ => {
                self.textarea.input(input);
                InputBoxEvent::Consumed
            }
        }
    }

    /// Return the current buffer contents as a single string (lines joined by newlines).
    pub fn text(&self) -> String {
        self.textarea.lines().join("\n")
    }

    /// Render the input box centered in the given area.
    pub fn render(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let box_width = (area.width as usize * 2 / 3)
            .max(30)
            .min(area.width as usize) as u16;
        let box_height: u16 = 8;

        let [vert_area] = Layout::vertical([Constraint::Length(box_height)])
            .flex(Flex::Center)
            .areas(area);

        let [horiz_area] = Layout::horizontal([Constraint::Length(box_width)])
            .flex(Flex::Center)
            .areas(vert_area);

        // Clear the area behind the input box.
        frame.render_widget(Clear, horiz_area);

        let block = Block::default()
            .borders(Borders::ALL)
            .style(theme.input_box)
            .border_style(theme.input_box_border)
            .title(Span::styled(
                format!(" {} ", self.prompt),
                theme.input_box_title,
            ))
            .title_alignment(Alignment::Left);

        // Clone the textarea so we can set the block on it for rendering.
        let mut ta = self.textarea.clone();
        ta.set_style(theme.input_box);
        ta.set_cursor_style(theme.cursor);
        ta.set_selection_style(theme.selection_highlight);
        ta.set_block(block);
        frame.render_widget(&ta, horiz_area);
    }
}
