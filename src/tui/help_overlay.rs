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
const MIN_TWO_COL_WIDTH: u16 = 110;
const COL_GAP: u16 = 3;
const DISMISS_HINT: &str = "Press ? or Esc to close";

const LEFT_COL_TITLES: &[&str] = &["Global", "Normal Mode", "Insert Mode"];
const RIGHT_COL_TITLES: &[&str] = &["Visual Mode", "Annotation List", "Command Mode"];

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
    pub fn render(&self, frame: &mut Frame, area: Rect, theme: &UiTheme, scroll_offset: &mut u16) {
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

        // Clamp scroll offset so it never exceeds scrollable range.
        let max_offset = content_lines.len().saturating_sub(visible_height);
        *scroll_offset = (*scroll_offset as usize).min(max_offset) as u16;

        let offset = *scroll_offset as usize;
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
        if width >= MIN_TWO_COL_WIDTH as usize {
            self.two_column_lines(theme, width)
        } else {
            let sections = self.ordered_sections();
            Self::section_lines(&sections, self.mode, theme, width)
        }
    }

    fn section_lines(
        sections: &[&HelpSection],
        mode: Mode,
        theme: &UiTheme,
        width: usize,
    ) -> Vec<Line<'static>> {
        let key_width = width.min(18);
        let active_title = mode_title(mode);

        sections
            .iter()
            .flat_map(|section| {
                let is_active = Some(section.title) == active_title;
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

    fn column_sections<'a>(&'a self, titles: &[&str]) -> Vec<&'a HelpSection> {
        let active_title = mode_title(self.mode);
        let mut col: Vec<&HelpSection> = self
            .sections
            .iter()
            .filter(|s| titles.contains(&s.title))
            .collect();
        col.sort_by_key(|s| {
            if Some(s.title) == active_title {
                0
            } else if s.title == "Global" {
                1
            } else {
                2
            }
        });
        col
    }

    fn two_column_lines(&self, theme: &UiTheme, width: usize) -> Vec<Line<'static>> {
        let col_width = (width.saturating_sub(COL_GAP as usize)) / 2;
        let left_sections = self.column_sections(LEFT_COL_TITLES);
        let right_sections = self.column_sections(RIGHT_COL_TITLES);

        let left_lines = Self::section_lines(&left_sections, self.mode, theme, col_width);
        let right_lines = Self::section_lines(&right_sections, self.mode, theme, col_width);

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

    fn render_to_lines_with_offset(
        width: u16,
        height: u16,
        mode: Mode,
        scroll_offset: &mut u16,
    ) -> Vec<String> {
        let backend = TestBackend::new(width, height);
        let mut terminal = Terminal::new(backend).unwrap();
        let overlay = HelpOverlay::new(mode, help_sections());

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

    fn render_to_lines(width: u16, height: u16, mode: Mode) -> Vec<String> {
        render_to_lines_with_offset(width, height, mode, &mut 0)
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
    fn renders_scroll_down_indicator_when_truncated() {
        let output = render_to_lines(24, 8, Mode::Normal).join("\n");
        assert!(
            output.contains('▼'),
            "Expected ▼ scroll indicator in: {output}"
        );
    }

    #[test]
    fn omits_scroll_indicators_when_help_fits() {
        let output = render_to_lines(120, 60, Mode::Normal).join("\n");
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
        let at_top = render_to_lines(80, 24, Mode::Normal).join("\n");
        let scrolled = render_to_lines_with_offset(80, 24, Mode::Normal, &mut 3).join("\n");
        assert_ne!(at_top, scrolled, "Expected different content when scrolled");
    }

    #[test]
    fn excessive_scroll_offset_is_clamped() {
        let mut offset = 9999u16;
        let output = render_to_lines_with_offset(80, 24, Mode::Normal, &mut offset).join("\n");
        assert!(offset < 9999, "Expected scroll offset to be clamped");
        assert!(
            !output.is_empty(),
            "Expected content to render after clamping"
        );
    }

    #[test]
    fn scroll_indicators_appear_when_scrolled() {
        let mut offset = 3u16;
        let output = render_to_lines_with_offset(24, 8, Mode::Normal, &mut offset).join("\n");
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
        let output = render_to_lines(140, 30, Mode::Normal);
        let has_side_by_side = output
            .iter()
            .any(|line| line.contains("Normal Mode") && line.contains("Visual Mode"));
        assert!(
            has_side_by_side,
            "Expected left and right column sections on same row in wide layout: {output:?}",
        );
    }

    #[test]
    fn single_column_layout_at_narrow_width() {
        let output = render_to_lines(80, 40, Mode::Normal).join("\n");
        let has_side_by_side = output
            .lines()
            .any(|line| line.contains("Normal Mode") && line.contains("Visual Mode"));
        assert!(
            !has_side_by_side,
            "Expected single-column layout at narrow width: {output}"
        );
    }

    #[test]
    fn active_mode_first_in_column() {
        // In Visual mode, "Visual Mode" should appear before "Annotation List" in right column
        let output = render_to_lines(140, 30, Mode::Visual);
        let visual_row = output.iter().position(|l| l.contains("Visual Mode"));
        let annot_row = output.iter().position(|l| l.contains("Annotation List"));
        assert!(
            visual_row < annot_row,
            "Expected Visual Mode before Annotation List in right column: {output:?}",
        );
    }
}
