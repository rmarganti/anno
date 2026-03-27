use std::io;

use crossterm::event::{self, Event, KeyEvent};
use ratatui::{
    DefaultTerminal, Frame,
    layout::{Alignment, Constraint, Layout},
    widgets::Paragraph,
};

use crate::annotation::export::{AnnotationExporter, PlannotatorExporter};
use crate::annotation::store::AnnotationStore;
use crate::highlight::syntect::SyntectHighlighter;
use crate::keybinds::handler::{Action, KeybindHandler};
use crate::keybinds::mode::Mode;
use crate::startup::{StartupError, StartupSettings};
use crate::tui::annotation_controller::{AnnotationAction, AnnotationController};
use crate::tui::app_command::{AppCommand, QuitKind};
use crate::tui::command_line::{CommandLine, CommandLineEvent};
use crate::tui::document_view::DocumentView;
use crate::tui::renderer;
use crate::tui::status_bar::{self, StatusBarProps};
use crate::tui::theme::UiTheme;

/// The result of running the application: whether to print annotations on exit.
pub enum ExitResult {
    /// Quit and print annotations to stdout.
    QuitWithOutput(String),
    /// Quit without printing.
    QuitSilent,
}

/// Top-level application state.
pub struct App {
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
    /// Centralized theme styles.
    theme: UiTheme,
    /// Document view component (viewport, cursor, rendering).
    document_view: DocumentView,
    /// Annotation creation state machine.
    annotation_controller: AnnotationController,
}

impl App {
    pub fn new(
        source_name: String,
        content: String,
        startup: StartupSettings,
    ) -> Result<Self, StartupError> {
        let highlighter = SyntectHighlighter::from_startup(&startup)?;
        let theme =
            UiTheme::from_syntect_theme(highlighter.theme(), Some(&startup.app_theme_overlays));
        let doc_lines_result = renderer::text_to_lines(&content, &highlighter);

        let document_view = DocumentView::new(doc_lines_result.plain, doc_lines_result.styled);

        Ok(Self {
            source_name,
            mode: Mode::Normal,
            keybinds: KeybindHandler::new(),
            annotations: AnnotationStore::new(),
            command_line: CommandLine::new(),
            should_quit: false,
            exit_result: None,
            theme,
            document_view,
            annotation_controller: AnnotationController::new(),
        })
    }

    /// Run the application main loop. Returns the exit result.
    pub fn run(mut self, terminal: &mut DefaultTerminal) -> io::Result<ExitResult> {
        while !self.should_quit {
            terminal.draw(|frame| {
                self.render(frame);
            })?;

            if let Event::Key(key_event) = event::read()? {
                self.handle_key(key_event);
            }
        }

        Ok(self.exit_result.unwrap_or(ExitResult::QuitSilent))
    }

    fn handle_key(&mut self, key_event: KeyEvent) {
        let action = self.keybinds.handle(self.mode, key_event);

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
            Action::EnterAnnotationListMode => self.mode = Mode::AnnotationList,
            Action::ExitToNormal => {
                self.mode = Mode::Normal;
                self.document_view.clear_visual();
                self.annotation_controller.cancel();
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

    fn render(&mut self, frame: &mut Frame) {
        let area = frame.area();

        // Sync viewport dimensions before the size check so is_too_small()
        // reflects the actual terminal size (viewport starts at 0×0).
        self.document_view
            .update_dimensions(area.width as usize, area.height.saturating_sub(1) as usize);

        // Minimum terminal size check.
        if self.document_view.is_too_small() {
            let msg = Paragraph::new("Terminal too small.\nPlease resize to at least 80×24.")
                .alignment(Alignment::Center);
            frame.render_widget(msg, area);
            return;
        }

        let [main_area, status_area] =
            Layout::vertical([Constraint::Min(1), Constraint::Length(1)]).areas(area);

        // Collect annotation ranges for visual indicators.
        let annotation_ranges: Vec<_> = self
            .annotations
            .all()
            .iter()
            .filter_map(|a| a.range)
            .collect();

        // -- Main document area --
        self.document_view.render(
            frame,
            main_area,
            &self.theme,
            self.mode == Mode::Visual,
            &annotation_ranges,
        );

        // -- Status bar --
        let cursor = self.document_view.cursor();
        status_bar::render(
            frame,
            status_area,
            &self.theme,
            &StatusBarProps {
                mode: self.mode,
                source_name: &self.source_name,
                annotation_count: self.annotations.len(),
                cursor_row: cursor.row,
                cursor_col: cursor.col,
                word_wrap: self.document_view.word_wrap(),
                command_buffer: self.command_line.buffer(),
            },
        );

        // -- Input box overlay --
        if let Some(ib) = self.annotation_controller.input_box() {
            ib.render(frame, main_area, &self.theme);
        }
    }
}
