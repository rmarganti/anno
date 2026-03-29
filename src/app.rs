use std::io;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use crossterm::event::{self, Event, KeyEvent};
use ratatui::{
    DefaultTerminal, Frame,
    layout::{Alignment, Constraint, Layout},
    widgets::Paragraph,
};

use crate::annotation::export::{AnnotationExporter, PlannotatorExporter};
use crate::annotation::store::AnnotationStore;
#[cfg(any(test, doctest))]
use crate::highlight::StyledSpan;
use crate::highlight::syntect::SyntectHighlighter;
use crate::keybinds::handler::{Action, KeybindHandler};
use crate::keybinds::mode::Mode;
use crate::startup::{StartupError, StartupSettings};
use crate::tui::annotation_controller::{AnnotationAction, AnnotationController};
use crate::tui::annotation_list_panel::{AnnotationListPanel, PANEL_WIDTH};
use crate::tui::app_command::{AppCommand, QuitKind};
use crate::tui::command_line::{CommandLine, CommandLineEvent};
use crate::tui::confirm_dialog::{ConfirmDialog, ConfirmDialogEvent};
use crate::tui::document_view::DocumentView;
use crate::tui::renderer;
use crate::tui::status_bar::{self, StatusBarProps};
use crate::tui::theme::UiTheme;

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

/// Terminal-independent application state.
pub struct AppState {
    /// The source name (filename or "[stdin]").
    source_name: String,
    /// Current input mode.
    mode: Mode,
    /// Key event dispatcher.
    keybinds: KeybindHandler,
    /// Annotation storage.
    annotations: AnnotationStore,
    /// Command-line component (handles `:` command input).
    command_line: CommandLine,
    /// Whether the app should quit.
    should_quit: bool,
    /// The exit result to return.
    exit_result: Option<ExitResult>,
    /// Document view component (viewport, cursor, rendering).
    document_view: DocumentView,
    /// Annotation creation state machine.
    annotation_controller: AnnotationController,
    /// Annotation list sidebar panel.
    annotation_list_panel: AnnotationListPanel,
    /// Active confirmation dialog overlay, if any.
    confirm_dialog: Option<ConfirmDialog>,
}

impl AppState {
    pub fn new(
        source_name: String,
        content: String,
        startup: StartupSettings,
    ) -> Result<Self, StartupError> {
        let highlighter = SyntectHighlighter::from_startup(&startup)?;
        let doc_lines_result = renderer::text_to_lines(&content, &highlighter);

        Ok(Self::from_document_lines(source_name, doc_lines_result))
    }

    #[cfg(any(test, doctest))]
    pub fn new_plain(source_name: String, content: String) -> Self {
        let plain = if content.is_empty() {
            vec![String::new()]
        } else {
            content.split('\n').map(str::to_owned).collect::<Vec<_>>()
        };
        let styled = plain
            .iter()
            .map(|line| vec![StyledSpan::plain(line.clone())])
            .collect();

        Self::from_document_lines(source_name, renderer::DocumentLines { plain, styled })
    }

    fn from_document_lines(source_name: String, doc_lines_result: renderer::DocumentLines) -> Self {
        let document_view = DocumentView::new(doc_lines_result.plain, doc_lines_result.styled);

        Self {
            source_name,
            mode: Mode::Normal,
            keybinds: KeybindHandler::new(),
            annotations: AnnotationStore::new(),
            command_line: CommandLine::new(),
            should_quit: false,
            exit_result: None,
            document_view,
            annotation_controller: AnnotationController::new(),
            annotation_list_panel: AnnotationListPanel::new(),
            confirm_dialog: None,
        }
    }

    fn handle_key(&mut self, key_event: KeyEvent) {
        // If a confirm dialog is active, route all input to it.
        if let Some(dialog) = self.confirm_dialog.take() {
            match dialog.handle_key(key_event) {
                ConfirmDialogEvent::Confirm => {
                    if let Some(id) = self.annotation_list_panel.selected_annotation_id() {
                        self.annotations.delete(id);
                    }
                }
                ConfirmDialogEvent::Cancel => {}
                ConfirmDialogEvent::Consumed => {
                    self.confirm_dialog = Some(dialog);
                }
            }
            return;
        }

        let action = self.keybinds.handle(self.mode, key_event);

        // In AnnotationList mode, MoveDown/MoveUp control panel selection,
        // not document cursor movement.
        if self.mode == Mode::AnnotationList {
            match action {
                Action::MoveDown => {
                    self.annotation_list_panel
                        .move_selection_down(&self.annotations);
                    return;
                }
                Action::MoveUp => {
                    self.annotation_list_panel
                        .move_selection_up(&self.annotations);
                    return;
                }
                Action::JumpToAnnotation => {
                    if let Some(id) = self.annotation_list_panel.selected_annotation_id()
                        && let Some(annotation) = self.annotations.get(id)
                        && let Some(range) = annotation.range
                    {
                        self.document_view
                            .set_cursor(range.start.line, range.start.column);
                    }
                    return;
                }
                Action::DeleteAnnotation => {
                    self.confirm_dialog = Some(ConfirmDialog::new("Delete annotation? (y/n)"));
                    return;
                }
                Action::ExitToNormal => {
                    self.mode = Mode::Normal;
                    return;
                }
                _ => {}
            }
        }

        // Let DocumentView handle movement and visual-mode actions first.
        if self.document_view.handle_action(&action) {
            // EnterVisualMode is handled by DocumentView (sets anchor),
            // but we also need to update the mode.
            if matches!(action, Action::EnterVisualMode) {
                self.mode = Mode::Visual;
            }
            return;
        }

        match action {
            // -- Mode transitions --
            Action::EnterCommandMode => {
                self.mode = Mode::Command;
                self.command_line.clear();
            }
            Action::EnterAnnotationListMode => {
                self.annotation_list_panel.toggle();
                if self.annotation_list_panel.is_visible() && !self.annotations.is_empty() {
                    self.mode = Mode::AnnotationList;
                } else {
                    self.mode = Mode::Normal;
                }
            }
            Action::ExitToNormal => {
                self.mode = Mode::Normal;
                self.document_view.clear_visual();
                self.annotation_controller.cancel();
            }

            // -- Annotation navigation (Normal mode) --
            Action::NextAnnotation => {
                self.jump_to_adjacent_annotation(true);
            }
            Action::PrevAnnotation => {
                self.jump_to_adjacent_annotation(false);
            }

            // -- Command mode --
            Action::CommandChar(c) => {
                let event = self.command_line.handle_char(c);
                self.handle_command_line_event(event);
            }
            Action::CommandBackspace => {
                let event = self.command_line.handle_backspace();
                self.handle_command_line_event(event);
            }
            Action::CommandConfirm => {
                let event = self.command_line.handle_confirm();
                self.handle_command_line_event(event);
            }

            // -- Annotation creation from Visual mode --
            Action::CreateDeletion => {
                let action = self
                    .annotation_controller
                    .create_deletion(&mut self.document_view, &mut self.annotations);
                self.apply_annotation_action(action);
            }
            Action::CreateComment => {
                let action = self
                    .annotation_controller
                    .start_input_for_visual_annotation("Comment", &mut self.document_view);
                self.apply_annotation_action(action);
            }
            Action::CreateReplacement => {
                let action = self
                    .annotation_controller
                    .start_input_for_visual_annotation("Replacement", &mut self.document_view);
                self.apply_annotation_action(action);
            }

            // -- Annotation creation from Normal mode --
            Action::CreateInsertion => {
                let action = self
                    .annotation_controller
                    .start_insertion(&self.document_view);
                self.apply_annotation_action(action);
            }
            Action::CreateGlobalComment => {
                let action = self.annotation_controller.start_global_comment();
                self.apply_annotation_action(action);
            }

            // -- Input mode --
            Action::InputForward(key_event) => {
                let action = self
                    .annotation_controller
                    .handle_input_key(key_event, &mut self.annotations);
                self.apply_annotation_action(action);
            }

            // -- Force quit (Ctrl-C) --
            Action::ForceQuit => {
                self.should_quit = true;
            }

            _ => {}
        }
    }

    fn apply_annotation_action(&mut self, action: AnnotationAction) {
        if let AnnotationAction::SwitchMode(mode) = action {
            self.mode = mode;
        }
    }

    fn handle_command_line_event(&mut self, event: CommandLineEvent) {
        match event {
            CommandLineEvent::Command(cmd) => self.process_app_command(cmd),
            CommandLineEvent::ExitToNormal => self.mode = Mode::Normal,
            CommandLineEvent::Consumed => {}
        }
    }

    fn process_app_command(&mut self, cmd: AppCommand) {
        match cmd {
            AppCommand::Quit(QuitKind::WithOutput) => {
                let output = PlannotatorExporter.export(&self.annotations);
                self.exit_result = Some(ExitResult::QuitWithOutput(output));
                self.should_quit = true;
            }
            AppCommand::Quit(QuitKind::Silent) => {
                self.exit_result = Some(ExitResult::QuitSilent);
                self.should_quit = true;
            }
        }
    }

    fn jump_to_adjacent_annotation(&mut self, forward: bool) {
        let cursor = self.document_view.cursor();
        let cursor_pos = (cursor.row, cursor.col);
        let ordered = self.annotations.ordered();

        if ordered.is_empty() {
            return;
        }

        let target = if forward {
            ordered.iter().find(|a| {
                if let Some(r) = a.range {
                    (r.start.line, r.start.column) > cursor_pos
                } else {
                    false
                }
            })
        } else {
            ordered.iter().rev().find(|a| {
                if let Some(r) = a.range {
                    (r.start.line, r.start.column) < cursor_pos
                } else {
                    false
                }
            })
        };

        if let Some(annotation) = target
            && let Some(range) = annotation.range
        {
            self.document_view
                .set_cursor(range.start.line, range.start.column);
        }
    }
}

/// Top-level application shell.
pub struct App {
    /// Centralized theme styles.
    theme: UiTheme,
    /// Terminal-independent application state.
    state: AppState,
}

impl App {
    pub fn new(
        source_name: String,
        content: String,
        startup: StartupSettings,
    ) -> Result<Self, StartupError> {
        let theme = Self::theme_from_startup(&startup)?;

        Ok(Self {
            theme,
            state: AppState::new(source_name, content, startup)?,
        })
    }

    fn theme_from_startup(startup: &StartupSettings) -> Result<UiTheme, StartupError> {
        let highlighter = SyntectHighlighter::from_startup(startup)?;

        Ok(UiTheme::from_syntect_theme(
            highlighter.theme(),
            Some(&startup.app_theme_overlays),
            startup.document_background,
        ))
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
        while !self.state.should_quit {
            if signal_flag.load(Ordering::Relaxed) {
                break;
            }

            terminal.draw(|frame| {
                self.render(frame);
            })?;

            if event::poll(Duration::from_millis(100))? {
                if let Event::Key(key_event) = event::read()? {
                    self.state.handle_key(key_event);
                }
            }
        }

        Ok(self.state.exit_result.unwrap_or(ExitResult::QuitSilent))
    }

    fn render(&mut self, frame: &mut Frame) {
        let area = frame.area();

        // Decide whether the panel should actually be shown: it must be
        // toggled visible AND the terminal must be wide enough.
        let show_panel =
            self.state.annotation_list_panel.is_visible() && area.width >= MIN_WIDTH_FOR_PANEL;

        // Compute the document area width for dimension checks.
        let doc_area_width = if show_panel {
            area.width.saturating_sub(PANEL_WIDTH)
        } else {
            area.width
        };

        // Sync viewport dimensions before the size check so is_too_small()
        // reflects the actual terminal size (viewport starts at 0×0).
        self.state.document_view.update_dimensions(
            doc_area_width as usize,
            area.height.saturating_sub(1) as usize,
        );

        // Minimum terminal size check.
        if self.state.document_view.is_too_small() {
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

        // Collect annotation ranges for visual indicators.
        let annotation_ranges: Vec<_> = self
            .state
            .annotations
            .all()
            .iter()
            .filter_map(|a| a.range)
            .collect();

        // Resolve the selected annotation's text range (if any) for document highlighting.
        let selected_annotation_range = if show_panel {
            self.state
                .annotation_list_panel
                .selected_annotation_id()
                .and_then(|id| self.state.annotations.get(id))
                .and_then(|a| a.range)
        } else {
            None
        };

        // -- Annotation list panel --
        if let Some(panel_area) = panel_area {
            self.state.annotation_list_panel.render(
                frame,
                panel_area,
                &self.state.annotations,
                &self.theme,
            );
        }

        // -- Main document area --
        self.state.document_view.render(
            frame,
            doc_area,
            &self.theme,
            self.state.mode == Mode::Visual,
            &annotation_ranges,
            selected_annotation_range.as_ref(),
        );

        // -- Status bar --
        let cursor = self.state.document_view.cursor();
        status_bar::render(
            frame,
            status_area,
            &self.theme,
            &StatusBarProps {
                mode: self.state.mode,
                source_name: &self.state.source_name,
                annotation_count: self.state.annotations.len(),
                cursor_row: cursor.row,
                cursor_col: cursor.col,
                word_wrap: self.state.document_view.word_wrap(),
                command_buffer: self.state.command_line.buffer(),
            },
        );

        // -- Input box overlay --
        if let Some(ib) = self.state.annotation_controller.input_box() {
            ib.render(frame, main_area, &self.theme);
        }

        // -- Confirm dialog overlay --
        if let Some(ref dialog) = self.state.confirm_dialog {
            dialog.render(frame, main_area);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{AppState, ExitResult};
    use crate::keybinds::mode::Mode;

    #[test]
    fn new_plain_builds_terminal_independent_default_state() {
        let state = AppState::new_plain("[stdin]".to_string(), "first\nsecond".to_string());

        assert_eq!(state.source_name, "[stdin]");
        assert_eq!(state.mode, Mode::Normal);
        assert!(state.annotations.is_empty());
        assert!(!state.should_quit);
        assert!(state.exit_result.is_none());
        assert!(state.confirm_dialog.is_none());

        let cursor = state.document_view.cursor();
        assert_eq!(cursor.row, 0);
        assert_eq!(cursor.col, 0);

        let _ = ExitResult::QuitSilent;
    }
}
