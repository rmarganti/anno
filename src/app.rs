mod app_state;

use std::io;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use crossterm::event::{self, Event};
use ratatui::{
    DefaultTerminal, Frame,
    layout::{Alignment, Constraint, Layout},
    widgets::Paragraph,
};

use crate::annotation::types::{AnnotationIndicator, AnnotationType};
use crate::highlight::syntect::SyntectHighlighter;
use crate::keybinds::help_content::help_sections;
use crate::keybinds::mode::Mode;
use crate::startup::{StartupError, StartupSettings};
use crate::tui::annotation_inspect_overlay::AnnotationInspectOverlay;
use crate::tui::annotation_list_panel::PANEL_WIDTH;
use crate::tui::help_overlay::HelpOverlay;
use crate::tui::renderer;
use crate::tui::status_bar::{self, StatusBarProps};
use crate::tui::theme::UiTheme;
use app_state::AppState;

/// Minimum terminal width required to show the annotation list panel.
/// Below this width the panel is automatically hidden.
const MIN_WIDTH_FOR_PANEL: u16 = 116;

/// The result of running the application: whether to print annotations on exit.
pub enum ExitResult {
    /// Quit and print annotations to stdout.
    QuitWithOutput(String),
    /// Quit without printing.
    QuitSilent,
}

/// Top-level application shell.
pub struct App {
    /// Centralized theme styles.
    theme: UiTheme,
    /// Optional display-only title for the status bar.
    title: Option<String>,
    /// Terminal-independent application state.
    state: AppState,
}

impl App {
    pub fn new(
        source_name: String,
        content: String,
        startup: StartupSettings,
    ) -> Result<Self, StartupError> {
        let title = startup.title.clone();
        let export_format = startup.export_format;
        let highlighter = SyntectHighlighter::from_startup(&startup)?;
        let theme = UiTheme::from_syntect_theme(
            highlighter.theme(),
            Some(&startup.app_theme_overlays),
            startup.document_background,
        );
        let document_lines = renderer::text_to_lines(&content, &highlighter);

        Ok(Self {
            theme,
            title,
            state: AppState::new(source_name, document_lines, export_format),
        })
    }

    /// Run the application main loop. Returns the exit result.
    ///
    /// `signal_flag` is set to `true` by signal handlers registered in `main`
    /// when SIGINT, SIGTERM, or SIGHUP is received.
    pub fn run(
        mut self,
        terminal: &mut DefaultTerminal,
        signal_flag: &AtomicBool,
    ) -> io::Result<ExitResult> {
        while !self.state.should_quit() {
            if signal_flag.load(Ordering::Relaxed) {
                break;
            }

            terminal.draw(|frame| {
                self.render(frame);
            })?;

            if event::poll(Duration::from_millis(100))?
                && let Event::Key(key_event) = event::read()?
            {
                self.state.handle_key(key_event);
            }
        }

        Ok(self
            .state
            .take_exit_result()
            .unwrap_or(ExitResult::QuitSilent))
    }

    fn render(&mut self, frame: &mut Frame) {
        let area = frame.area();
        let panel_available = area.width >= MIN_WIDTH_FOR_PANEL;
        self.state.set_annotation_panel_available(panel_available);

        // Decide whether the panel should actually be shown: it must be
        // toggled visible AND the terminal must be wide enough.
        let show_panel = self.state.is_panel_visible() && panel_available;

        // Compute the document area width for dimension checks.
        let doc_area_width = if show_panel {
            area.width.saturating_sub(PANEL_WIDTH)
        } else {
            area.width
        };

        // Sync viewport dimensions before the size check so is_too_small()
        // reflects the actual terminal size (viewport starts at 0×0).
        self.state.document_view_mut().update_dimensions(
            doc_area_width as usize,
            area.height.saturating_sub(1) as usize,
        );

        // Minimum terminal size check.
        if self.state.document_view().is_too_small() {
            let msg = Paragraph::new("Terminal too small.\nPlease resize to at least 80×24.")
                .alignment(Alignment::Center);
            frame.render_widget(msg, area);
            return;
        }

        let [main_area, status_area] =
            Layout::vertical([Constraint::Min(1), Constraint::Length(1)]).areas(area);

        // Split main_area into panel + document when the panel is shown.
        let (panel_area, doc_area) = if show_panel {
            let [doc, panel] =
                Layout::horizontal([Constraint::Min(1), Constraint::Length(PANEL_WIDTH)])
                    .areas(main_area);
            (Some(panel), doc)
        } else {
            (None, main_area)
        };

        // Collect typed indicators for in-document rendering. Global comments are
        // excluded because they have no anchor range.
        let annotation_indicators: Vec<AnnotationIndicator> = self
            .state
            .annotations()
            .all()
            .iter()
            .filter_map(|annotation| {
                annotation.range.map(|range| AnnotationIndicator {
                    range,
                    annotation_type: annotation.annotation_type,
                })
            })
            .filter(|indicator| indicator.annotation_type != AnnotationType::GlobalComment)
            .collect();

        // Resolve the selected annotation's text range (if any) for document highlighting.
        let selected_annotation_range = if show_panel {
            self.state.selected_annotation_range()
        } else {
            None
        };

        // -- Annotation list panel --
        if let Some(panel_area) = panel_area {
            let is_focused = self.state.mode() == Mode::AnnotationList;
            self.state
                .render_annotation_list_panel(frame, panel_area, &self.theme, is_focused);
        }

        // -- Main document area --
        let is_visual = self.state.mode() == Mode::Visual;
        self.state.document_view_mut().render(
            frame,
            doc_area,
            &self.theme,
            is_visual,
            &annotation_indicators,
            selected_annotation_range.as_ref(),
        );

        // -- Status bar --
        let cursor = self.state.cursor();
        status_bar::render(
            frame,
            status_area,
            &self.theme,
            &StatusBarProps {
                mode: self.state.mode(),
                annotation_inspect_visible: self.state.is_annotation_inspect_visible(),
                title: self.title.as_deref(),
                source_name: self.state.source_name(),
                annotation_count: self.state.annotation_count(),
                cursor_row: cursor.row,
                cursor_col: cursor.col,
                word_wrap: self.state.word_wrap(),
                command_buffer: self.state.command_buffer(),
                panel_hidden_due_to_width: self.state.is_panel_hidden_due_to_width(),
            },
        );

        // -- Input box overlay --
        if let Some(ib) = self.state.annotation_controller().input_box() {
            ib.render(frame, main_area, &self.theme);
        }

        // -- Confirm dialog overlay --
        if self.state.has_confirm_dialog()
            && let Some(dialog) = self.state.confirm_dialog()
        {
            dialog.render(frame, main_area);
        }

        // -- Annotation inspect overlay --
        if self.state.is_annotation_inspect_visible()
            && let Some(annotation) = self.state.selected_annotation().cloned()
        {
            AnnotationInspectOverlay::new(annotation).render(
                frame,
                main_area,
                &self.theme,
                self.state.annotation_inspect_scroll_offset_mut(),
            );
        }

        // -- Help overlay --
        if self.state.is_help_visible() {
            HelpOverlay::new(help_sections()).render(
                frame,
                main_area,
                &self.theme,
                self.state.help_scroll_offset_mut(),
            );
        }
    }
}
