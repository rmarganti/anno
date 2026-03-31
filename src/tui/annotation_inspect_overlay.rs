#![allow(dead_code)]

use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Flex, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
};

use crate::{
    annotation::types::{Annotation, AnnotationType, TextRange},
    tui::theme::UiTheme,
};

const MIN_WIDTH: u16 = 36;
const MIN_HEIGHT: u16 = 8;
const FOOTER_HINT: &str = "j/k Select  Up/Down Scroll  Enter Jump  Esc Close";

/// Modal overlay for read-only inspection of a single annotation.
#[derive(Debug, Clone)]
pub struct AnnotationInspectOverlay {
    annotation: Annotation,
}

impl AnnotationInspectOverlay {
    pub fn new(annotation: Annotation) -> Self {
        Self { annotation }
    }

    /// Render the inspect overlay centered in the given area.
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
            .title(Span::styled(
                format!(" {} ", self.title()),
                theme.input_box_title,
            ))
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

        let footer_y = inner.y + inner.height.saturating_sub(1);
        let width = inner.width as usize;
        let arrow_up = if has_lines_above { "▲" } else { " " };
        let arrow_down = if has_lines_below { "▼" } else { " " };
        let center_text = truncate_to_width(FOOTER_HINT, width.saturating_sub(2));
        let center_width = width.saturating_sub(2);
        let footer = Line::from(vec![
            Span::styled(arrow_up.to_string(), theme.panel_border),
            Span::styled(format!("{center_text:^center_width$}"), theme.panel_border),
            Span::styled(arrow_down.to_string(), theme.panel_border),
        ]);
        frame.render_widget(
            Paragraph::new(footer),
            Rect::new(inner.x, footer_y, inner.width, 1),
        );
    }

    fn title(&self) -> String {
        match self.annotation.range {
            Some(range) => format!(
                "{}: {}",
                type_name(&self.annotation.annotation_type),
                location_label(range)
            ),
            None => type_name(&self.annotation.annotation_type).to_string(),
        }
    }

    fn content_lines(&self, theme: &UiTheme, width: usize) -> Vec<Line<'static>> {
        let mut lines = Vec::new();
        let section_style = Style::default()
            .fg(theme.annotation_type_color(&self.annotation.annotation_type))
            .add_modifier(Modifier::BOLD);

        if let Some(range) = self.annotation.range {
            push_section(
                &mut lines,
                "Location",
                &location_label(range),
                width,
                section_style,
            );
        } else {
            push_section(
                &mut lines,
                "Location",
                "Global comment",
                width,
                section_style,
            );
        }

        match self.annotation.annotation_type {
            AnnotationType::Deletion => {
                push_section(
                    &mut lines,
                    "Selected Text",
                    &self.annotation.selected_text,
                    width,
                    section_style,
                );
            }
            AnnotationType::Comment => {
                if !self.annotation.selected_text.is_empty() {
                    push_section(
                        &mut lines,
                        "Selected Text",
                        &self.annotation.selected_text,
                        width,
                        section_style,
                    );
                }
                push_section(
                    &mut lines,
                    "Comment",
                    &self.annotation.text,
                    width,
                    section_style,
                );
            }
            AnnotationType::Replacement => {
                push_section(
                    &mut lines,
                    "Original Text",
                    &self.annotation.selected_text,
                    width,
                    section_style,
                );
                push_section(
                    &mut lines,
                    "Replacement Text",
                    &self.annotation.text,
                    width,
                    section_style,
                );
            }
            AnnotationType::Insertion => {
                push_section(
                    &mut lines,
                    "Inserted Text",
                    &self.annotation.text,
                    width,
                    section_style,
                );
            }
            AnnotationType::GlobalComment => {
                push_section(
                    &mut lines,
                    "Comment",
                    &self.annotation.text,
                    width,
                    section_style,
                );
            }
        }

        lines
    }
}

fn type_name(annotation_type: &AnnotationType) -> &'static str {
    match annotation_type {
        AnnotationType::Deletion => "Deletion",
        AnnotationType::Comment => "Comment",
        AnnotationType::Replacement => "Replacement",
        AnnotationType::Insertion => "Insertion",
        AnnotationType::GlobalComment => "Global Comment",
    }
}

fn location_label(range: TextRange) -> String {
    let start_line = range.start.line + 1;
    let start_col = range.start.column + 1;
    let end_line = range.end.line + 1;
    let end_col = range.end.column + 1;

    if range.start == range.end {
        format!("L{start_line}:C{start_col}")
    } else {
        format!("L{start_line}:C{start_col}-L{end_line}:C{end_col}")
    }
}

fn push_section(
    lines: &mut Vec<Line<'static>>,
    title: &str,
    body: &str,
    width: usize,
    title_style: Style,
) {
    if !lines.is_empty() {
        lines.push(Line::default());
    }

    lines.push(Line::from(Span::styled(title.to_string(), title_style)));

    let body = if body.is_empty() { "(empty)" } else { body };
    let indent = if width > 2 { "  " } else { "" };
    let wrap_width = width.saturating_sub(indent.chars().count()).max(1);
    for wrapped in wrap_text(body, wrap_width) {
        lines.push(Line::from(format!("{indent}{wrapped}")));
    }
}

fn wrap_text(text: &str, width: usize) -> Vec<String> {
    let width = width.max(1);
    let mut lines = Vec::new();

    for raw_line in text.split('\n') {
        if raw_line.is_empty() {
            lines.push(String::new());
            continue;
        }

        let mut current = String::new();
        let mut current_len = 0;
        for ch in raw_line.chars() {
            if current_len == width {
                lines.push(current);
                current = String::new();
                current_len = 0;
            }
            current.push(ch);
            current_len += 1;
        }

        if current.is_empty() {
            lines.push(String::new());
        } else {
            lines.push(current);
        }
    }

    lines
}

fn truncate_to_width(text: &str, width: usize) -> String {
    text.chars().take(width).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::annotation::types::{TextPosition, TextRange};
    use ratatui::{Terminal, backend::TestBackend};

    fn range(sl: usize, sc: usize, el: usize, ec: usize) -> TextRange {
        TextRange {
            start: TextPosition {
                line: sl,
                column: sc,
            },
            end: TextPosition {
                line: el,
                column: ec,
            },
        }
    }

    fn render_to_lines_with_offset(
        width: u16,
        height: u16,
        overlay: &AnnotationInspectOverlay,
        scroll_offset: &mut u16,
    ) -> Vec<String> {
        let backend = TestBackend::new(width, height);
        let mut terminal = Terminal::new(backend).unwrap();

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

    fn render_to_lines(width: u16, height: u16, overlay: &AnnotationInspectOverlay) -> Vec<String> {
        render_to_lines_with_offset(width, height, overlay, &mut 0)
    }

    #[test]
    fn title_includes_type_and_location_for_anchored_annotations() {
        let overlay = AnnotationInspectOverlay::new(Annotation::replacement(
            range(1, 2, 1, 7),
            String::from("old"),
            String::from("new"),
        ));

        let output = render_to_lines(80, 20, &overlay).join("\n");
        assert!(
            output.contains("Replacement"),
            "Expected type in title: {output}"
        );
        assert!(
            output.contains("L2:C3-L2:C8"),
            "Expected location in title: {output}"
        );
    }

    #[test]
    fn renders_replacement_with_structured_original_and_new_sections() {
        let overlay = AnnotationInspectOverlay::new(Annotation::replacement(
            range(2, 0, 2, 5),
            String::from("alpha"),
            String::from("beta"),
        ));

        let output = render_to_lines(80, 20, &overlay).join("\n");
        assert!(
            output.contains("Original Text"),
            "Expected original section: {output}"
        );
        assert!(output.contains("alpha"), "Expected original text: {output}");
        assert!(
            output.contains("Replacement Text"),
            "Expected replacement section: {output}"
        );
        assert!(
            output.contains("beta"),
            "Expected replacement text: {output}"
        );
    }

    #[test]
    fn renders_global_comment_without_anchor_location() {
        let overlay = AnnotationInspectOverlay::new(Annotation::global_comment(String::from(
            "top-level note",
        )));

        let output = render_to_lines(80, 20, &overlay).join("\n");
        assert!(
            output.contains("Global Comment"),
            "Expected global comment title: {output}"
        );
        assert!(
            output.contains("Location\n  Global comment")
                || output.contains("Location") && output.contains("Global comment"),
            "Expected global location copy: {output}"
        );
        assert!(
            output.contains("top-level note"),
            "Expected global comment body: {output}"
        );
    }

    #[test]
    fn wraps_long_content_and_shows_scroll_indicators() {
        let overlay = AnnotationInspectOverlay::new(Annotation::comment(
            range(0, 0, 0, 4),
            String::from("mark"),
            String::from("abcdefghijklmnopqrstuvwxyz0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ"),
        ));

        let top = render_to_lines(40, 12, &overlay).join("\n");
        let scrolled = render_to_lines_with_offset(40, 12, &overlay, &mut 4).join("\n");

        assert!(top.contains('▼'), "Expected down indicator at top: {top}");
        assert!(
            scrolled.contains('▲'),
            "Expected up indicator when scrolled: {scrolled}"
        );
        assert_ne!(top, scrolled, "Expected different content after scrolling");
        assert!(
            top.contains("abcdefghijklmnopqrstuv") || scrolled.contains("abcdefghijklmnopqrstuv"),
            "Expected wrapped comment content in top or scrolled view:\nTOP:\n{top}\nSCROLLED:\n{scrolled}"
        );
    }

    #[test]
    fn renders_on_small_terminals_without_panicking() {
        let overlay = AnnotationInspectOverlay::new(Annotation::deletion(
            range(0, 0, 0, 3),
            String::from("xyz"),
        ));

        let output = render_to_lines(24, 8, &overlay).join("\n");
        assert!(!output.is_empty());
    }

    #[test]
    fn clamps_excessive_scroll_offsets() {
        let overlay = AnnotationInspectOverlay::new(Annotation::comment(
            range(0, 0, 0, 4),
            String::from("mark"),
            "line one\nline two\nline three\nline four\nline five\nline six".to_string(),
        ));

        let mut offset = 999u16;
        let output = render_to_lines_with_offset(40, 10, &overlay, &mut offset).join("\n");

        assert!(offset < 999, "Expected offset to be clamped");
        assert!(!output.is_empty(), "Expected output after clamping");
    }
}
