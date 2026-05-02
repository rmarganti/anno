use ratatui::{
    Frame,
    layout::Rect,
    text::{Line, Span},
    widgets::Paragraph,
};

use crate::keybinds::{handler::SearchDirection, mode::Mode};
use crate::tui::theme::UiTheme;

/// Data needed to render the status bar.
pub struct StatusBarProps<'a> {
    pub mode: Mode,
    pub annotation_inspect_visible: bool,
    pub panel_visible: bool,
    pub title: Option<&'a str>,
    pub source_name: &'a str,
    pub annotation_count: usize,
    pub cursor_row: usize,
    pub cursor_col: usize,
    pub word_wrap: bool,
    pub command_buffer: &'a str,
    pub search_buffer: &'a str,
    pub search_direction: Option<SearchDirection>,
    pub panel_hidden_due_to_width: bool,
}

/// Render the status bar into the given area.
pub fn render(frame: &mut Frame, area: Rect, theme: &UiTheme, props: &StatusBarProps) {
    let mode_label = match props.mode {
        Mode::Normal => " NORMAL ",
        // VisualLine reuses the VISUAL pill until the UI-polish ish refines it.
        Mode::Visual | Mode::VisualLine => " VISUAL ",
        Mode::Insert => " INSERT ",
        Mode::AnnotationList => " ANNOTATIONS ",
        Mode::Command => " COMMAND ",
        Mode::Search => " SEARCH ",
    };

    let cursor_pos = format!("{}:{}", props.cursor_row + 1, props.cursor_col + 1);
    let wrap_indicator = if props.word_wrap { "wrap " } else { "" };
    let display_name = props.title.unwrap_or(props.source_name);

    let mut status_spans = vec![
        Span::styled(mode_label, theme.status_mode),
        Span::raw(format!(" {display_name}  ")),
        Span::raw(format!("{} annotation(s)  ", props.annotation_count)),
        Span::raw(format!("{cursor_pos}  ")),
        Span::raw(wrap_indicator),
    ];

    let hint = if props.panel_hidden_due_to_width {
        "[panel hidden: terminal too narrow]".to_string()
    } else {
        match props.mode {
            Mode::Normal if props.panel_visible => {
                "count+nav  Tab focus  Esc hide  H help".to_string()
            }
            Mode::Normal => "count+nav  Tab panel  H help".to_string(),
            Mode::Visual | Mode::VisualLine => "count+nav  d/c/r annotate  Esc".to_string(),
            Mode::Insert => "Ctrl+S confirm  Esc cancel".to_string(),
            Mode::AnnotationList if props.annotation_inspect_visible => {
                "count+nav  Up/Down  Enter  Esc".to_string()
            }
            Mode::AnnotationList => "count+nav  Space  Enter  Esc hide".to_string(),
            Mode::Command => format!(":{}", props.command_buffer),
            Mode::Search => {
                let prefix = match props.search_direction.unwrap_or(SearchDirection::Forward) {
                    SearchDirection::Forward => '/',
                    SearchDirection::Backward => '?',
                };
                format!("{prefix}{}", props.search_buffer)
            }
        }
    };
    status_spans.push(Span::raw(hint));

    let status_bar = Paragraph::new(Line::from(status_spans)).style(theme.status_bar);
    frame.render_widget(status_bar, area);
}

#[cfg(test)]
mod tests {
    use ratatui::{Terminal, backend::TestBackend, layout::Rect};

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
                render(frame, area, &UiTheme::default(), props);
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
            annotation_inspect_visible: false,
            panel_visible: true,
            title: None,
            source_name: "test.md",
            annotation_count: 0,
            cursor_row: 0,
            cursor_col: 0,
            word_wrap: false,
            command_buffer: "",
            search_buffer: "",
            search_direction: None,
            panel_hidden_due_to_width: false,
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

    #[test]
    fn search_mode_label() {
        let props = base_props(Mode::Search);
        let output = render_to_string(&props);
        assert!(output.contains("SEARCH"), "Expected SEARCH in: {output}");
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
        assert!(
            output.contains("count+nav  Tab focus  Esc hide  H help"),
            "Expected normal panel hint in: {output}"
        );
    }

    #[test]
    fn visual_mode_shows_selection_actions() {
        let props = base_props(Mode::Visual);
        let output = render_to_string(&props);
        assert!(
            output.contains("count+nav  d/c/r annotate  Esc"),
            "Expected visual hint in: {output}"
        );
    }

    #[test]
    fn annotation_list_mode_shows_navigation_actions() {
        let props = base_props(Mode::AnnotationList);
        let output = render_to_string(&props);
        assert!(
            output.contains("count+nav  Space  Enter  Esc hide"),
            "Expected annotation list hint in: {output}"
        );
    }

    #[test]
    fn annotation_inspect_open_shows_dismissal_actions() {
        let props = StatusBarProps {
            annotation_inspect_visible: true,
            ..base_props(Mode::AnnotationList)
        };
        let output = render_to_string(&props);
        assert!(
            output.contains("count+nav  Up/Down  Enter  Esc"),
            "Expected inspect hint in: {output}"
        );
        assert!(
            !output.contains("dd"),
            "Did not expect delete hint while inspect is open: {output}"
        );
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
    fn search_mode_shows_forward_search_buffer() {
        let props = StatusBarProps {
            search_buffer: "pattern",
            search_direction: Some(SearchDirection::Forward),
            ..base_props(Mode::Search)
        };
        let output = render_to_string(&props);
        assert!(
            output.contains("/pattern"),
            "Expected '/pattern' in: {output}"
        );
    }

    #[test]
    fn search_mode_shows_backward_search_buffer() {
        let props = StatusBarProps {
            search_buffer: "pattern",
            search_direction: Some(SearchDirection::Backward),
            ..base_props(Mode::Search)
        };
        let output = render_to_string(&props);
        assert!(
            output.contains("?pattern"),
            "Expected '?pattern' in: {output}"
        );
    }

    #[test]
    fn narrow_terminal_panel_hint_overrides_default_hint() {
        let props = StatusBarProps {
            panel_visible: false,
            panel_hidden_due_to_width: true,
            ..base_props(Mode::Normal)
        };
        let output = render_to_string(&props);
        assert!(
            output.contains("[panel hidden: terminal too narrow]"),
            "Expected narrow terminal hint in: {output}"
        );
        assert!(
            !output.contains("H help"),
            "Did not expect default help hint in: {output}"
        );
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

    #[test]
    fn title_overrides_source_name() {
        let props = StatusBarProps {
            title: Some("Reviewing: Implementation Plan"),
            ..base_props(Mode::Normal)
        };
        let output = render_to_string(&props);
        assert!(
            output.contains("Reviewing: Implementation Plan"),
            "Expected title in: {output}"
        );
        assert!(
            !output.contains("test.md"),
            "Did not expect source name in: {output}"
        );
    }
}
