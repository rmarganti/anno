use ratatui::{
    Frame,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
};

use crate::keybinds::mode::Mode;

/// Data needed to render the status bar.
pub struct StatusBarProps<'a> {
    pub mode: Mode,
    pub source_name: &'a str,
    pub annotation_count: usize,
    pub cursor_row: usize,
    pub cursor_col: usize,
    pub word_wrap: bool,
    pub command_buffer: &'a str,
}

/// Render the status bar into the given area.
pub fn render(frame: &mut Frame, area: Rect, props: &StatusBarProps) {
    let mode_label = match props.mode {
        Mode::Normal => " NORMAL ",
        Mode::Visual => " VISUAL ",
        Mode::Insert => " INSERT ",
        Mode::AnnotationList => " ANNOTATIONS ",
        Mode::Command => " COMMAND ",
    };

    let cursor_pos = format!("{}:{}", props.cursor_row + 1, props.cursor_col + 1);
    let wrap_indicator = if props.word_wrap { "wrap " } else { "" };

    let mut status_spans = vec![
        Span::styled(
            mode_label,
            Style::default().add_modifier(Modifier::BOLD | Modifier::REVERSED),
        ),
        Span::raw(format!(" {}  ", props.source_name)),
        Span::raw(format!("{} annotation(s)  ", props.annotation_count)),
        Span::raw(format!("{cursor_pos}  ")),
        Span::raw(wrap_indicator),
    ];

    if props.mode == Mode::Command {
        status_spans.push(Span::raw(format!(":{}", props.command_buffer)));
    } else if props.mode == Mode::Insert {
        status_spans.push(Span::raw("Ctrl+S confirm  Esc cancel"));
    } else {
        status_spans.push(Span::raw("? help"));
    }

    let status_bar = Paragraph::new(Line::from(status_spans));
    frame.render_widget(status_bar, area);
}
