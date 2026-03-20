use ratatui::style::{Color, Style};

/// Centralized style definitions for the document renderer.
pub struct Theme {
    pub cursor: Style,
    pub selection_highlight: Style,
}

impl Theme {
    pub fn new() -> Self {
        Self {
            cursor: Style::default().bg(Color::White).fg(Color::Black),
            selection_highlight: Style::default()
                .bg(Color::Black)
                .fg(Color::White),
        }
    }
}

impl Default for Theme {
    fn default() -> Self {
        Self::new()
    }
}
