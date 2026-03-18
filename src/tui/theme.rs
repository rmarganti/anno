use ratatui::style::{Color, Modifier, Style};

/// Centralized style definitions for the document renderer.
pub struct Theme {
    pub heading_styles: [Style; 6],
    pub blockquote_border: Style,
    pub blockquote_text: Style,
    pub hr: Style,
    pub code_fence: Style,
    pub code_bg: Style,
    pub list_marker: Style,
    pub checkbox: Style,
    pub table_header: Style,
    pub table_border: Style,
    pub cursor: Style,
}

impl Theme {
    pub fn new() -> Self {
        Self {
            heading_styles: [
                // H1: Bright Magenta, bold
                Style::default()
                    .fg(Color::Magenta)
                    .add_modifier(Modifier::BOLD),
                // H2: Cyan, bold
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
                // H3: Green, bold
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
                // H4: Yellow, bold
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
                // H5: Blue
                Style::default().fg(Color::Blue),
                // H6: DarkGray
                Style::default().fg(Color::DarkGray),
            ],
            blockquote_border: Style::default().fg(Color::Blue),
            blockquote_text: Style::default()
                .fg(Color::Gray)
                .add_modifier(Modifier::ITALIC),
            hr: Style::default().fg(Color::DarkGray),
            code_fence: Style::default().fg(Color::DarkGray),
            code_bg: Style::default().bg(Color::Rgb(43, 48, 59)),
            list_marker: Style::default().fg(Color::Yellow),
            checkbox: Style::default().fg(Color::Green),
            table_header: Style::default().add_modifier(Modifier::BOLD),
            table_border: Style::default().fg(Color::DarkGray),
            cursor: Style::default().bg(Color::White).fg(Color::Black),
        }
    }

    /// Get the heading style for the given level (1-indexed, clamped to 1..=6).
    pub fn heading(&self, level: usize) -> Style {
        let idx = level.clamp(1, 6) - 1;
        self.heading_styles[idx]
    }
}

impl Default for Theme {
    fn default() -> Self {
        Self::new()
    }
}
