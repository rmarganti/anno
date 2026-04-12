use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Flex, Layout, Rect},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
};

use crate::keybinds::help_content::{HelpSection, help_sections};
use crate::tui::theme::UiTheme;

const MIN_WIDTH: u16 = 36;
const MIN_HEIGHT: u16 = 8;
const MIN_TWO_COL_WIDTH: u16 = 110;
const COL_GAP: u16 = 3;
const DISMISS_HINT: &str = "Press H or Esc to close";

const SECTION_ORDER: &[&str] = &[
    "Global",
    "Normal Mode",
    "Insert Mode",
    "Visual Mode",
    "Annotation List",
    "Command Mode",
];

const LEFT_COL_TITLES: &[&str] = &["Global", "Normal Mode", "Insert Mode"];
const RIGHT_COL_TITLES: &[&str] = &["Visual Mode", "Annotation List", "Command Mode"];

pub fn max_scroll_offset(width: u16, height: u16) -> u16 {
    HelpOverlay::new(help_sections()).max_scroll_offset(&UiTheme::default(), width, height)
}

/// Modal help overlay rendered on top of the document view.
#[derive(Debug, Clone)]
pub struct HelpOverlay {
    sections: Vec<HelpSection>,
}

impl HelpOverlay {
    pub fn new(sections: Vec<HelpSection>) -> Self {
        Self { sections }
    }

    pub fn max_scroll_offset(&self, theme: &UiTheme, width: u16, height: u16) -> u16 {
        let box_width = ((width as usize * 4) / 5)
            .max(MIN_WIDTH as usize)
            .min(width as usize) as u16;
        let box_height = ((height as usize * 4) / 5)
            .max(MIN_HEIGHT as usize)
            .min(height as usize) as u16;
        let content_width = box_width.saturating_sub(2) as usize;
        let content_height = box_height.saturating_sub(3) as usize;

        if content_width == 0 || content_height == 0 {
            return 0;
        }

        self.content_lines(theme, content_width)
            .len()
            .saturating_sub(content_height) as u16
    }

    /// Render the help overlay centered in the given area.
    pub fn render(&self, frame: &mut Frame, area: Rect, theme: &UiTheme, scroll_offset: u16) {
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

        let content_height = inner.height.saturating_sub(1);
        if content_height == 0 {
            return;
        }

        let content_lines = self.content_lines(theme, inner.width as usize);
        let visible_height = content_height as usize;
        let max_offset = self.max_scroll_offset(theme, area.width, area.height) as usize;
        let offset = (scroll_offset as usize).min(max_offset);
        let has_lines_above = offset > 0;
        let has_lines_below = offset + visible_height < content_lines.len();

        for (index, line) in content_lines
            .iter()
            .skip(offset)
            .take(visible_height)
            .enumerate()
        {
            frame.render_widget(
                Paragraph::new(line.clone()).style(theme.input_box),
                Rect::new(inner.x, inner.y + index as u16, inner.width, 1),
            );
        }

        // Build the dismiss hint line with optional scroll indicators.
        let hint_y = inner.y + inner.height.saturating_sub(1);
        let w = inner.width as usize;
        let arrow_up = if has_lines_above { "▲" } else { " " };
        let arrow_down = if has_lines_below { "▼" } else { " " };
        let center_text = truncate_to_width(DISMISS_HINT, w.saturating_sub(2));
        let center_width = w.saturating_sub(2);
        let hint_line = Line::from(vec![
            Span::styled(arrow_up.to_string(), theme.panel_border),
            Span::styled(format!("{center_text:^center_width$}"), theme.panel_border),
            Span::styled(arrow_down.to_string(), theme.panel_border),
        ]);
        frame.render_widget(
            Paragraph::new(hint_line),
            Rect::new(inner.x, hint_y, inner.width, 1),
        );
    }

    fn sections_in_order(&self, titles: &[&str]) -> Vec<&HelpSection> {
        titles
            .iter()
            .filter_map(|title| self.sections.iter().find(|s| s.title == *title))
            .collect()
    }

    fn content_lines(&self, theme: &UiTheme, width: usize) -> Vec<Line<'static>> {
        if width >= MIN_TWO_COL_WIDTH as usize {
            self.two_column_lines(theme, width)
        } else {
            let sections = self.sections_in_order(SECTION_ORDER);
            Self::section_lines(&sections, theme, width)
        }
    }

    fn section_lines(
        sections: &[&HelpSection],
        theme: &UiTheme,
        width: usize,
    ) -> Vec<Line<'static>> {
        let key_width = width.min(18);

        let mut lines = Vec::new();
        for (i, section) in sections.iter().enumerate() {
            if i > 0 {
                lines.push(Line::default());
            }
            lines.push(Line::from(Span::styled(
                truncate_to_width(section.title, width),
                theme.input_box_title,
            )));
            for entry in &section.entries {
                lines.push(help_entry_line(entry.keys, entry.action, key_width, width));
            }
        }
        lines
    }

    fn two_column_lines(&self, theme: &UiTheme, width: usize) -> Vec<Line<'static>> {
        let col_width = (width.saturating_sub(COL_GAP as usize)) / 2;
        let left_sections = self.sections_in_order(LEFT_COL_TITLES);
        let right_sections = self.sections_in_order(RIGHT_COL_TITLES);

        let left_lines = Self::section_lines(&left_sections, theme, col_width);
        let right_lines = Self::section_lines(&right_sections, theme, col_width);

        let max_len = left_lines.len().max(right_lines.len());
        let gap: String = " ".repeat(COL_GAP as usize);

        (0..max_len)
            .map(|i| {
                let left = left_lines.get(i);
                let right = right_lines.get(i);
                let left_spans = match left {
                    Some(line) => pad_spans(line.spans.clone(), col_width),
                    None => vec![Span::raw(" ".repeat(col_width))],
                };
                let right_spans = match right {
                    Some(line) => pad_spans(line.spans.clone(), col_width),
                    None => vec![Span::raw(" ".repeat(col_width))],
                };
                let mut spans = left_spans;
                spans.push(Span::raw(gap.clone()));
                spans.extend(right_spans);
                Line::from(spans)
            })
            .collect()
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

fn pad_spans(spans: Vec<Span<'static>>, width: usize) -> Vec<Span<'static>> {
    let current: usize = spans.iter().map(|s| s.content.chars().count()).sum();
    if current >= width {
        return spans;
    }
    let mut result = spans;
    result.push(Span::raw(" ".repeat(width - current)));
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::keybinds::help_content::help_sections;
    use ratatui::{Terminal, backend::TestBackend};

    fn render_to_lines_with_offset(width: u16, height: u16, scroll_offset: u16) -> Vec<String> {
        let backend = TestBackend::new(width, height);
        let mut terminal = Terminal::new(backend).unwrap();
        let overlay = HelpOverlay::new(help_sections());

        terminal
            .draw(|frame| {
                overlay.render(
                    frame,
                    Rect::new(0, 0, width, height),
                    &UiTheme::default(),
                    scroll_offset,
                );
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

    fn render_to_lines(width: u16, height: u16) -> Vec<String> {
        render_to_lines_with_offset(width, height, 0)
    }

    #[test]
    fn renders_section_titles() {
        let output = render_to_lines(80, 24).join("\n");
        assert!(
            output.contains("Normal Mode"),
            "Expected section title in: {output}"
        );
        assert!(
            output.contains("Global"),
            "Expected global section title in: {output}"
        );
    }

    #[test]
    fn renders_key_descriptions() {
        let output = render_to_lines(80, 40).join("\n");
        assert!(
            output.contains("Toggle help"),
            "Expected key description in: {output}"
        );
        assert!(
            output.contains("Move cursor"),
            "Expected mode help in: {output}"
        );
    }

    #[test]
    fn renders_on_small_terminals_without_panicking() {
        let output = render_to_lines(24, 8).join("\n");
        assert!(!output.is_empty());
    }

    #[test]
    fn renders_scroll_down_indicator_when_truncated() {
        let output = render_to_lines(24, 8).join("\n");
        assert!(
            output.contains('▼'),
            "Expected ▼ scroll indicator in: {output}"
        );
    }

    #[test]
    fn omits_scroll_indicators_when_help_fits() {
        let output = render_to_lines(120, 96).join("\n");
        assert!(
            !output.contains('▼'),
            "Did not expect ▼ indicator in: {output}"
        );
        assert!(
            !output.contains('▲'),
            "Did not expect ▲ indicator in: {output}"
        );
    }

    #[test]
    fn scroll_offset_changes_visible_content() {
        let at_top = render_to_lines(80, 24).join("\n");
        let scrolled = render_to_lines_with_offset(80, 24, 3).join("\n");
        assert_ne!(at_top, scrolled, "Expected different content when scrolled");
    }

    #[test]
    fn excessive_scroll_offset_is_clamped() {
        let output = render_to_lines_with_offset(80, 24, 9999).join("\n");
        assert!(
            !output.is_empty(),
            "Expected content to render after clamping"
        );
    }

    #[test]
    fn scroll_indicators_appear_when_scrolled() {
        let output = render_to_lines_with_offset(24, 8, 3).join("\n");
        assert!(
            output.contains('▲'),
            "Expected ▲ when scrolled down in: {output}"
        );
        assert!(
            output.contains('▼'),
            "Expected ▼ when more content below in: {output}"
        );
    }

    #[test]
    fn two_column_layout_at_wide_width() {
        // Inner width must reach MIN_TWO_COL_WIDTH (110).
        // box_width = (width * 4) / 5, inner = box_width - 2 (borders).
        // So width = 140 gives box_width = 112, inner = 110.
        let output = render_to_lines(140, 30);
        let has_side_by_side = output
            .iter()
            .any(|line| line.contains("Global") && line.contains("Visual Mode"));
        assert!(
            has_side_by_side,
            "Expected left and right column sections on same row in wide layout: {output:?}",
        );
    }

    #[test]
    fn single_column_layout_at_narrow_width() {
        let output = render_to_lines(80, 40).join("\n");
        let has_side_by_side = output
            .lines()
            .any(|line| line.contains("Normal Mode") && line.contains("Visual Mode"));
        assert!(
            !has_side_by_side,
            "Expected single-column layout at narrow width: {output}"
        );
    }

    #[test]
    fn sections_follow_hardcoded_order() {
        let output = render_to_lines(80, 50);
        let global_row = output.iter().position(|l| l.contains("Global"));
        let normal_row = output.iter().position(|l| l.contains("Normal Mode"));
        let insert_row = output.iter().position(|l| l.contains("Insert Mode"));
        assert!(
            global_row < normal_row && normal_row < insert_row,
            "Expected Global < Normal Mode < Insert Mode: {output:?}",
        );
    }
}
