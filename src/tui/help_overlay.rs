use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Flex, Layout, Rect},
    style::Modifier,
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
};

use crate::keybinds::help_content::HelpSection;
use crate::keybinds::mode::Mode;
use crate::tui::theme::UiTheme;

const MIN_WIDTH: u16 = 36;
const MIN_HEIGHT: u16 = 8;
const DISMISS_HINT: &str = "Press ? or Esc to close";
const OVERFLOW_HINT: &str = "More help below";

/// Modal help overlay rendered on top of the document view.
#[derive(Debug, Clone)]
pub struct HelpOverlay {
    mode: Mode,
    sections: Vec<HelpSection>,
}

impl HelpOverlay {
    pub fn new(mode: Mode, sections: Vec<HelpSection>) -> Self {
        Self { mode, sections }
    }

    /// Render the help overlay centered in the given area.
    pub fn render(&self, frame: &mut Frame, area: Rect, theme: &UiTheme) {
        let box_width = ((area.width as usize * 4) / 5)
            .max(MIN_WIDTH as usize)
            .min(area.width as usize) as u16;
        let box_height = ((area.height as usize * 4) / 5)
            .max(MIN_HEIGHT as usize)
            .min(area.height as usize) as u16;

        let [vert_area] = Layout::vertical([Constraint::Length(box_height)])
            .flex(Flex::Center)
            .areas(area);
        let [overlay_area] = Layout::horizontal([Constraint::Length(box_width)])
            .flex(Flex::Center)
            .areas(vert_area);

        frame.render_widget(Clear, overlay_area);

        let block = Block::default()
            .borders(Borders::ALL)
            .style(theme.input_box)
            .border_style(theme.input_box_border)
            .title(Span::styled(" Help ", theme.input_box_title))
            .title_alignment(Alignment::Left);
        let inner = block.inner(overlay_area);
        frame.render_widget(block, overlay_area);

        if inner.width == 0 || inner.height == 0 {
            return;
        }

        let hint_y = inner.y + inner.height.saturating_sub(1);
        let hint = truncate_to_width(DISMISS_HINT, inner.width as usize);
        frame.render_widget(
            Paragraph::new(hint)
                .alignment(Alignment::Center)
                .style(theme.panel_border),
            Rect::new(inner.x, hint_y, inner.width, 1),
        );

        let content_height = inner.height.saturating_sub(1);
        if content_height == 0 {
            return;
        }

        let content_lines = self.content_lines(theme, inner.width as usize);
        let mut visible_line_count = content_height as usize;
        let is_overflowing = content_lines.len() > visible_line_count;
        if is_overflowing {
            visible_line_count = visible_line_count.saturating_sub(1);
        }

        for (index, line) in content_lines.iter().take(visible_line_count).enumerate() {
            frame.render_widget(
                Paragraph::new(line.clone()).style(theme.input_box),
                Rect::new(inner.x, inner.y + index as u16, inner.width, 1),
            );
        }

        if is_overflowing {
            let overflow_y = inner.y + content_height.saturating_sub(1);
            let overflow_hint = truncate_to_width(OVERFLOW_HINT, inner.width as usize);
            frame.render_widget(
                Paragraph::new(overflow_hint)
                    .alignment(Alignment::Center)
                    .style(theme.panel_border.add_modifier(Modifier::ITALIC)),
                Rect::new(inner.x, overflow_y, inner.width, 1),
            );
        }
    }

    fn ordered_sections(&self) -> Vec<&HelpSection> {
        let active_title = mode_title(self.mode);
        let mut ordered: Vec<&HelpSection> = self.sections.iter().collect();
        ordered.sort_by_key(|section| {
            if Some(section.title) == active_title {
                0
            } else if section.title == "Global" {
                1
            } else {
                2
            }
        });
        ordered
    }

    fn content_lines(&self, theme: &UiTheme, width: usize) -> Vec<Line<'static>> {
        let key_width = width.min(18);

        self.ordered_sections()
            .into_iter()
            .flat_map(|section| {
                let is_active = Some(section.title) == mode_title(self.mode);
                let title_style = if is_active {
                    theme.input_box_title.add_modifier(Modifier::REVERSED)
                } else {
                    theme.input_box_title
                };

                std::iter::once(Line::from(Span::styled(
                    truncate_to_width(section.title, width),
                    title_style,
                )))
                .chain(
                    section.entries.iter().map(move |entry| {
                        help_entry_line(entry.keys, entry.action, key_width, width)
                    }),
                )
            })
            .collect()
    }
}

fn mode_title(mode: Mode) -> Option<&'static str> {
    match mode {
        Mode::Normal => Some("Normal Mode"),
        Mode::Visual => Some("Visual Mode"),
        Mode::Insert => Some("Insert Mode"),
        Mode::AnnotationList => Some("Annotation List"),
        Mode::Command => Some("Command Mode"),
    }
}

fn help_entry_line(
    keys: &str,
    action: &str,
    key_width: usize,
    total_width: usize,
) -> Line<'static> {
    let keys = truncate_to_width(keys, key_width);
    let available = total_width.saturating_sub(keys.chars().count() + 2);
    let action = truncate_to_width(action, available);
    Line::from(format!("{keys:key_width$}  {action}"))
}

fn truncate_to_width(text: &str, width: usize) -> String {
    text.chars().take(width).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::keybinds::help_content::help_sections;
    use ratatui::{Terminal, backend::TestBackend};

    fn render_to_lines(width: u16, height: u16, mode: Mode) -> Vec<String> {
        let backend = TestBackend::new(width, height);
        let mut terminal = Terminal::new(backend).unwrap();
        let overlay = HelpOverlay::new(mode, help_sections());

        terminal
            .draw(|frame| {
                overlay.render(frame, Rect::new(0, 0, width, height), &UiTheme::default());
            })
            .unwrap();

        let buffer = terminal.backend().buffer().clone();
        (0..height)
            .map(|y| {
                (0..width)
                    .map(|x| {
                        buffer
                            .cell((x, y))
                            .map(|cell| cell.symbol().chars().next().unwrap_or(' '))
                            .unwrap_or(' ')
                    })
                    .collect::<String>()
                    .trim_end()
                    .to_string()
            })
            .collect()
    }

    #[test]
    fn renders_section_titles() {
        let output = render_to_lines(80, 24, Mode::Normal).join("\n");
        assert!(
            output.contains("Normal Mode"),
            "Expected active section title in: {output}"
        );
        assert!(
            output.contains("Global"),
            "Expected global section title in: {output}"
        );
    }

    #[test]
    fn renders_key_descriptions() {
        let output = render_to_lines(80, 24, Mode::Normal).join("\n");
        assert!(
            output.contains("Toggle help"),
            "Expected key description in: {output}"
        );
        assert!(
            output.contains("Create insertion annotation"),
            "Expected mode help in: {output}"
        );
    }

    #[test]
    fn renders_on_small_terminals_without_panicking() {
        let output = render_to_lines(24, 8, Mode::Normal).join("\n");
        assert!(!output.is_empty());
    }

    #[test]
    fn renders_overflow_hint_when_help_is_truncated() {
        let output = render_to_lines(24, 8, Mode::Normal).join("\n");
        assert!(
            output.contains("More help below"),
            "Expected overflow hint in: {output}"
        );
    }

    #[test]
    fn omits_overflow_hint_when_help_fits() {
        let output = render_to_lines(120, 60, Mode::Normal).join("\n");
        assert!(
            !output.contains("More help below"),
            "Did not expect overflow hint in: {output}"
        );
    }
}
