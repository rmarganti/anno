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

        let ordered_sections = self.ordered_sections();
        let mut y = inner.y;
        let end_y = inner.y + content_height;

        for section in ordered_sections {
            if y >= end_y {
                break;
            }

            let is_active = Some(section.title) == mode_title(self.mode);
            let title_style = if is_active {
                theme.input_box_title.add_modifier(Modifier::REVERSED)
            } else {
                theme.input_box_title
            };

            frame.render_widget(
                Paragraph::new(Line::from(Span::styled(
                    truncate_to_width(section.title, inner.width as usize),
                    title_style,
                ))),
                Rect::new(inner.x, y, inner.width, 1),
            );
            y += 1;

            for entry in &section.entries {
                if y >= end_y {
                    break;
                }

                let key_width = inner.width.min(18) as usize;
                let line =
                    help_entry_line(entry.keys, entry.action, key_width, inner.width as usize);
                frame.render_widget(
                    Paragraph::new(line).style(theme.input_box),
                    Rect::new(inner.x, y, inner.width, 1),
                );
                y += 1;
            }
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
}
