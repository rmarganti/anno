use ratatui::{
    layout::Rect,
    style::Modifier,
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use crate::keybinds::mode::Mode;
use crate::tui::theme::Theme;

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
pub fn render(frame: &mut Frame, area: Rect, theme: &Theme, props: &StatusBarProps) {
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
            theme
                .status_mode
                .add_modifier(Modifier::BOLD)
                .remove_modifier(Modifier::REVERSED),
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

    let status_bar = Paragraph::new(Line::from(status_spans)).style(theme.status_bar);
    frame.render_widget(status_bar, area);
}

#[cfg(test)]
mod tests {
    use ratatui::{backend::TestBackend, layout::Rect, Terminal};

    use super::*;

    /// Render StatusBarProps into a 80-wide single-row buffer and return the
    /// trimmed string of all visible characters.
    fn render_to_string(props: &StatusBarProps) -> String {
        let backend = TestBackend::new(80, 1);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|frame| {
                let area = Rect {
                    x: 0,
                    y: 0,
                    width: 80,
                    height: 1,
                };
                render(frame, area, &Theme::default(), props);
            })
            .unwrap();
        let buffer = terminal.backend().buffer().clone();
        let content: String = (0..80)
            .map(|x| {
                buffer
                    .cell((x, 0))
                    .map(|c| c.symbol().chars().next().unwrap_or(' '))
                    .unwrap_or(' ')
            })
            .collect();
        content.trim_end().to_string()
    }

    fn base_props(mode: Mode) -> StatusBarProps<'static> {
        StatusBarProps {
            mode,
            source_name: "test.md",
            annotation_count: 0,
            cursor_row: 0,
            cursor_col: 0,
            word_wrap: false,
            command_buffer: "",
        }
    }

    // ── Mode labels ───────────────────────────────────────────────────

    #[test]
    fn normal_mode_label() {
        let props = base_props(Mode::Normal);
        let output = render_to_string(&props);
        assert!(output.contains("NORMAL"), "Expected NORMAL in: {output}");
    }

    #[test]
    fn visual_mode_label() {
        let props = base_props(Mode::Visual);
        let output = render_to_string(&props);
        assert!(output.contains("VISUAL"), "Expected VISUAL in: {output}");
    }

    #[test]
    fn insert_mode_label() {
        let props = base_props(Mode::Insert);
        let output = render_to_string(&props);
        assert!(output.contains("INSERT"), "Expected INSERT in: {output}");
    }

    #[test]
    fn annotation_list_mode_label() {
        let props = base_props(Mode::AnnotationList);
        let output = render_to_string(&props);
        assert!(
            output.contains("ANNOTATIONS"),
            "Expected ANNOTATIONS in: {output}"
        );
    }

    #[test]
    fn command_mode_label() {
        let props = base_props(Mode::Command);
        let output = render_to_string(&props);
        assert!(output.contains("COMMAND"), "Expected COMMAND in: {output}");
    }

    // ── Annotation count ──────────────────────────────────────────────

    #[test]
    fn annotation_count_zero() {
        let props = StatusBarProps {
            annotation_count: 0,
            ..base_props(Mode::Normal)
        };
        let output = render_to_string(&props);
        assert!(
            output.contains("0 annotation(s)"),
            "Expected annotation count in: {output}"
        );
    }

    #[test]
    fn annotation_count_nonzero() {
        let props = StatusBarProps {
            annotation_count: 3,
            ..base_props(Mode::Normal)
        };
        let output = render_to_string(&props);
        assert!(
            output.contains("3 annotation(s)"),
            "Expected 3 annotation(s) in: {output}"
        );
    }

    // ── Cursor position ───────────────────────────────────────────────

    #[test]
    fn cursor_position_displayed_one_indexed() {
        let props = StatusBarProps {
            cursor_row: 4,
            cursor_col: 9,
            ..base_props(Mode::Normal)
        };
        let output = render_to_string(&props);
        // row+1=5, col+1=10
        assert!(output.contains("5:10"), "Expected 5:10 in: {output}");
    }

    #[test]
    fn cursor_at_origin_shows_1_1() {
        let props = StatusBarProps {
            cursor_row: 0,
            cursor_col: 0,
            ..base_props(Mode::Normal)
        };
        let output = render_to_string(&props);
        assert!(output.contains("1:1"), "Expected 1:1 in: {output}");
    }

    // ── Word wrap indicator ───────────────────────────────────────────

    #[test]
    fn word_wrap_on_shows_indicator() {
        let props = StatusBarProps {
            word_wrap: true,
            ..base_props(Mode::Normal)
        };
        let output = render_to_string(&props);
        assert!(output.contains("wrap"), "Expected 'wrap' in: {output}");
    }

    #[test]
    fn word_wrap_off_no_indicator() {
        let props = StatusBarProps {
            word_wrap: false,
            ..base_props(Mode::Normal)
        };
        let output = render_to_string(&props);
        assert!(
            !output.contains("wrap"),
            "Did not expect 'wrap' in: {output}"
        );
    }

    // ── Mode-specific hints ───────────────────────────────────────────

    #[test]
    fn insert_mode_shows_confirm_hint() {
        let props = base_props(Mode::Insert);
        let output = render_to_string(&props);
        assert!(
            output.contains("Ctrl+S confirm"),
            "Expected confirm hint in: {output}"
        );
    }

    #[test]
    fn normal_mode_shows_help_hint() {
        let props = base_props(Mode::Normal);
        let output = render_to_string(&props);
        assert!(output.contains("? help"), "Expected '? help' in: {output}");
    }

    #[test]
    fn command_mode_shows_command_buffer() {
        let props = StatusBarProps {
            command_buffer: "q!",
            ..base_props(Mode::Command)
        };
        let output = render_to_string(&props);
        assert!(output.contains(":q!"), "Expected ':q!' in: {output}");
    }

    #[test]
    fn source_name_shown() {
        let props = base_props(Mode::Normal);
        let output = render_to_string(&props);
        assert!(
            output.contains("test.md"),
            "Expected source name in: {output}"
        );
    }
}
