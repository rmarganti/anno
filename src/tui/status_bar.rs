use ratatui::{
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
};

use crate::keybinds::mode::Mode;

/// Render the status bar as a single-line `Paragraph`.
///
/// This is a stateless rendering function — all state is passed in as parameters.
pub fn render(
    mode: Mode,
    source_name: &str,
    annotation_count: usize,
    cursor_row: usize,
    cursor_col: usize,
    word_wrap: bool,
    command_buffer: &str,
) -> Paragraph<'static> {
    let mode_label = match mode {
        Mode::Normal => " NORMAL ",
        Mode::Visual => " VISUAL ",
        Mode::Insert => " INSERT ",
        Mode::AnnotationList => " ANNOTATIONS ",
        Mode::Command => " COMMAND ",
    };

    let cursor_pos = format!("{}:{}", cursor_row + 1, cursor_col + 1);
    let wrap_indicator = if word_wrap { "wrap " } else { "" };

    let mut status_spans = vec![
        Span::styled(
            mode_label,
            Style::default().add_modifier(Modifier::BOLD | Modifier::REVERSED),
        ),
        Span::raw(format!(" {}  ", source_name)),
        Span::raw(format!("{annotation_count} annotation(s)  ")),
        Span::raw(format!("{cursor_pos}  ")),
        Span::raw(wrap_indicator.to_string()),
    ];

    if mode == Mode::Command {
        status_spans.push(Span::raw(format!(":{command_buffer}")));
    } else if mode == Mode::Insert {
        status_spans.push(Span::raw("Ctrl+S confirm  Esc cancel"));
    } else {
        status_spans.push(Span::raw("? help"));
    }

    Paragraph::new(Line::from(status_spans))
}
