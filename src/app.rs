use std::io;

use crossterm::event::{self, Event, KeyEvent};
use ratatui::{
    DefaultTerminal, Frame,
    layout::{Alignment, Constraint, Layout},
    widgets::Paragraph,
};

use crate::annotation::export::{AnnotationExporter, PlannotatorExporter};
use crate::annotation::store::AnnotationStore;
use crate::annotation::types::{Annotation, TextPosition, TextRange};
use crate::highlight::syntect::SyntectHighlighter;
use crate::keybinds::handler::{Action, KeybindHandler};
use crate::keybinds::mode::Mode;
use crate::tui::app_command::{AppCommand, QuitKind};
use crate::tui::command_line::{CommandLine, CommandLineEvent};
use crate::tui::document_view::DocumentView;
use crate::tui::input_box::{InputBox, InputBoxEvent};
use crate::tui::renderer;
use crate::tui::status_bar::{self, StatusBarProps};
use crate::tui::theme::Theme;

/// The result of running the application: whether to print annotations on exit.
pub enum ExitResult {
    /// Quit and print annotations to stdout.
    QuitWithOutput(String),
    /// Quit without printing.
    QuitSilent,
}

/// Tracks the kind of annotation being created via the input box.
#[derive(Debug, Clone)]
enum PendingAnnotation {
    /// Comment on a selection — stores the selection range and original text.
    Comment {
        range: TextRange,
        selected_text: String,
    },
    /// Replacement for a selection — stores the selection range and original text.
    Replacement {
        range: TextRange,
        selected_text: String,
    },
    /// Insertion at a cursor position.
    Insertion { position: TextPosition },
    /// Global comment (not anchored to text).
    GlobalComment,
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
    theme: Theme,
    /// Document view component (viewport, cursor, rendering).
    document_view: DocumentView,
    /// Active input box (shown in Insert mode for annotation text entry).
    input_box: Option<InputBox<'static>>,
    /// The pending annotation being created (set when entering Insert mode).
    pending_annotation: Option<PendingAnnotation>,
}

impl App {
    pub fn new(source_name: String, content: String) -> Self {
        let highlighter = SyntectHighlighter::new();
        let theme = Theme::new();
        let doc_lines_result = renderer::text_to_lines(&content, &highlighter);

        let document_view = DocumentView::new(doc_lines_result.plain, doc_lines_result.styled);

        Self {
            source_name,
            mode: Mode::Normal,
            keybinds: KeybindHandler::new(),
            annotations: AnnotationStore::new(),
            command_line: CommandLine::new(),
            should_quit: false,
            exit_result: None,
            theme,
            document_view,
            input_box: None,
            pending_annotation: None,
        }
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
                self.input_box = None;
                self.pending_annotation = None;
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
            Action::CreateDeletion => self.create_deletion(),
            Action::CreateComment => self.start_input_for_visual_annotation("Comment"),
            Action::CreateReplacement => self.start_input_for_visual_annotation("Replacement"),

            // -- Annotation creation from Normal mode --
            Action::CreateInsertion => {
                let cursor = self.document_view.cursor();
                let position = TextPosition {
                    line: cursor.row,
                    column: cursor.col,
                };
                self.pending_annotation = Some(PendingAnnotation::Insertion { position });
                self.input_box = Some(InputBox::new("Insertion"));
                self.mode = Mode::Insert;
            }
            Action::CreateGlobalComment => {
                self.pending_annotation = Some(PendingAnnotation::GlobalComment);
                self.input_box = Some(InputBox::new("Global Comment"));
                self.mode = Mode::Insert;
            }

            // -- Input mode --
            Action::InputForward(key_event) => {
                if let Some(ref mut ib) = self.input_box {
                    match ib.handle_key(key_event) {
                        InputBoxEvent::Confirm => self.confirm_input(),
                        InputBoxEvent::Cancel => {
                            self.input_box = None;
                            self.pending_annotation = None;
                            self.mode = Mode::Normal;
                        }
                        InputBoxEvent::Consumed => {}
                    }
                }
            }

            _ => {}
        }
    }

    /// Create a Deletion annotation from the current visual selection.
    fn create_deletion(&mut self) {
        if let Some((range, text)) = self.document_view.take_visual_selection() {
            self.annotations.add(Annotation::deletion(range, text));
        }
        self.mode = Mode::Normal;
    }

    /// Begin input for a Comment or Replacement from visual mode.
    fn start_input_for_visual_annotation(&mut self, kind: &str) {
        if let Some((range, selected_text)) = self.document_view.take_visual_selection() {
            let pending = if kind == "Comment" {
                PendingAnnotation::Comment {
                    range,
                    selected_text,
                }
            } else {
                PendingAnnotation::Replacement {
                    range,
                    selected_text,
                }
            };
            self.pending_annotation = Some(pending);
            self.input_box = Some(InputBox::new(kind));
            self.mode = Mode::Insert;
        } else {
            self.mode = Mode::Normal;
        }
    }

    /// Confirm input and create the pending annotation.
    fn confirm_input(&mut self) {
        let text = self
            .input_box
            .as_ref()
            .map(|ib| ib.text())
            .unwrap_or_default();

        if let Some(pending) = self.pending_annotation.take() {
            if !text.is_empty() {
                let annotation = match pending {
                    PendingAnnotation::Comment {
                        range,
                        selected_text,
                    } => Annotation::comment(range, selected_text, text),
                    PendingAnnotation::Replacement {
                        range,
                        selected_text,
                    } => Annotation::replacement(range, selected_text, text),
                    PendingAnnotation::Insertion { position } => {
                        Annotation::insertion(position, text)
                    }
                    PendingAnnotation::GlobalComment => Annotation::global_comment(text),
                };
                self.annotations.add(annotation);
            }
        }

        self.input_box = None;
        self.mode = Mode::Normal;
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
            AppCommand::Write => {
                let output = PlannotatorExporter.export(&self.annotations);
                eprintln!("{output}");
                self.mode = Mode::Normal;
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
        if let Some(ref ib) = self.input_box {
            ib.render(frame, main_area);
        }
    }
}
