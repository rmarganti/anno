use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Flex, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
};

/// A reusable modal confirmation dialog that accepts y/n/Esc input.
#[derive(Debug, Clone)]
pub struct ConfirmDialog {
    /// The prompt message (e.g. "Delete annotation? (y/n)").
    prompt: String,
}

/// Result of forwarding a key event to the confirm dialog.
pub enum ConfirmDialogEvent {
    /// The user confirmed (y or Enter).
    Confirm,
    /// The user cancelled (n or Esc).
    Cancel,
    /// The key was consumed but produced no decision.
    Consumed,
}

impl ConfirmDialog {
    pub fn new(prompt: impl Into<String>) -> Self {
        Self {
            prompt: prompt.into(),
        }
    }

    /// Forward a crossterm key event to the dialog.
    /// Returns the semantic result of the key press.
    pub fn handle_key(&self, key_event: KeyEvent) -> ConfirmDialogEvent {
        match key_event.code {
            KeyCode::Char('y') | KeyCode::Char('Y') | KeyCode::Enter => ConfirmDialogEvent::Confirm,
            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => ConfirmDialogEvent::Cancel,
            _ => ConfirmDialogEvent::Consumed,
        }
    }

    /// Render the dialog centered in the given area.
    pub fn render(&self, frame: &mut Frame, area: Rect) {
        let box_width = (area.width as usize * 2 / 3)
            .max(30)
            .min(area.width as usize) as u16;
        let box_height: u16 = 5;

        let [vert_area] = Layout::vertical([Constraint::Length(box_height)])
            .flex(Flex::Center)
            .areas(area);

        let [horiz_area] = Layout::horizontal([Constraint::Length(box_width)])
            .flex(Flex::Center)
            .areas(vert_area);

        frame.render_widget(Clear, horiz_area);

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan))
            .title(Span::styled(
                " Confirm ",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ))
            .title_alignment(Alignment::Left);

        let paragraph = Paragraph::new(Line::from(self.prompt.as_str()))
            .alignment(Alignment::Center)
            .block(block);

        frame.render_widget(paragraph, horiz_area);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::empty())
    }

    #[test]
    fn confirm_on_y() {
        let dialog = ConfirmDialog::new("Delete?");
        assert!(matches!(
            dialog.handle_key(key(KeyCode::Char('y'))),
            ConfirmDialogEvent::Confirm
        ));
    }

    #[test]
    fn confirm_on_upper_y() {
        let dialog = ConfirmDialog::new("Delete?");
        assert!(matches!(
            dialog.handle_key(key(KeyCode::Char('Y'))),
            ConfirmDialogEvent::Confirm
        ));
    }

    #[test]
    fn confirm_on_enter() {
        let dialog = ConfirmDialog::new("Delete?");
        assert!(matches!(
            dialog.handle_key(key(KeyCode::Enter)),
            ConfirmDialogEvent::Confirm
        ));
    }

    #[test]
    fn cancel_on_n() {
        let dialog = ConfirmDialog::new("Delete?");
        assert!(matches!(
            dialog.handle_key(key(KeyCode::Char('n'))),
            ConfirmDialogEvent::Cancel
        ));
    }

    #[test]
    fn cancel_on_upper_n() {
        let dialog = ConfirmDialog::new("Delete?");
        assert!(matches!(
            dialog.handle_key(key(KeyCode::Char('N'))),
            ConfirmDialogEvent::Cancel
        ));
    }

    #[test]
    fn cancel_on_esc() {
        let dialog = ConfirmDialog::new("Delete?");
        assert!(matches!(
            dialog.handle_key(key(KeyCode::Esc)),
            ConfirmDialogEvent::Cancel
        ));
    }

    #[test]
    fn consumed_on_other_keys() {
        let dialog = ConfirmDialog::new("Delete?");
        assert!(matches!(
            dialog.handle_key(key(KeyCode::Char('x'))),
            ConfirmDialogEvent::Consumed
        ));
        assert!(matches!(
            dialog.handle_key(key(KeyCode::Tab)),
            ConfirmDialogEvent::Consumed
        ));
    }
}
