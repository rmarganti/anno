use ratatui::{
    layout::{Alignment, Constraint, Flex, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

/// Events produced by the `HelpOverlay` component.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HelpOverlayEvent {
    /// The overlay was dismissed (any key press).
    Dismissed,
    /// The key was consumed (no state change needed beyond dismissal).
    Consumed,
}

/// A full-screen help overlay that displays keybind reference information.
/// Any key press dismisses it.
pub struct HelpOverlay;

impl HelpOverlay {
    /// Handle any key event. Any key dismisses the overlay.
    pub fn handle_any_key(&self) -> HelpOverlayEvent {
        HelpOverlayEvent::Dismissed
    }

    /// Render the help overlay centered on the given area.
    pub fn render(&self, frame: &mut Frame, area: Rect) {
        let sections: &[(&str, &[(&str, &str)])] = &[
            (
                "Navigation",
                &[
                    ("j / ↓", "Move down"),
                    ("k / ↑", "Move up"),
                    ("h / ←", "Move left"),
                    ("l / →", "Move right"),
                    ("w / b", "Move word forward / backward"),
                    ("0 / $", "Line start / end"),
                    ("gg / G", "Document top / bottom"),
                    ("^d / ^u", "Half page down / up"),
                    ("^f / ^b", "Full page down / up"),
                ],
            ),
            (
                "Annotation Creation",
                &[
                    ("v", "Enter visual selection mode"),
                    ("d", "(visual) Mark as deletion"),
                    ("c", "(visual) Add comment"),
                    ("r", "(visual) Add replacement"),
                    ("i", "Insert at cursor position"),
                    ("gc", "Add global comment"),
                ],
            ),
            (
                "Other",
                &[
                    ("Tab", "Open annotation list"),
                    ("]a / [a", "Next / previous annotation"),
                    ("W", "Toggle word wrap"),
                    (":", "Enter command mode  (:q, :q!, :w)"),
                    ("?", "Toggle this help overlay"),
                ],
            ),
        ];

        // Build lines for the content.
        let mut lines: Vec<Line> = vec![Line::from("")];

        for (section_title, bindings) in sections {
            lines.push(Line::from(Span::styled(
                format!("  {section_title}"),
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )));
            for (key, desc) in *bindings {
                let key_span = Span::styled(
                    format!("  {key:<14}"),
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                );
                let desc_span = Span::styled(*desc, Style::default().fg(Color::White));
                lines.push(Line::from(vec![key_span, desc_span]));
            }
            lines.push(Line::from(""));
        }

        lines.push(Line::from(Span::styled(
            "  Press any key to close",
            Style::default()
                .fg(Color::DarkGray)
                .add_modifier(Modifier::ITALIC),
        )));

        let content_height = lines.len() as u16 + 2; // +2 for block borders
        let content_width: u16 = 54;

        let overlay_height = content_height.min(area.height);
        let overlay_width = content_width.min(area.width);

        let [vert_area] = Layout::vertical([Constraint::Length(overlay_height)])
            .flex(Flex::Center)
            .areas(area);

        let [horiz_area] = Layout::horizontal([Constraint::Length(overlay_width)])
            .flex(Flex::Center)
            .areas(vert_area);

        frame.render_widget(Clear, horiz_area);

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Yellow))
            .title(Span::styled(
                " Help ",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ))
            .title_alignment(Alignment::Center);

        let paragraph = Paragraph::new(lines).block(block);
        frame.render_widget(paragraph, horiz_area);
    }
}
