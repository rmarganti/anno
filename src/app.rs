use std::io;

use crossterm::event::{self, Event, KeyEvent};
use ratatui::{
    DefaultTerminal, Frame,
    layout::{Alignment, Constraint, Layout},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};

use crate::annotation::export::{AnnotationExporter, PlannotatorExporter};
use crate::annotation::store::AnnotationStore;
use crate::keybinds::handler::{Action, KeybindHandler};
use crate::keybinds::mode::Mode;

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
    /// Command-mode buffer (characters typed after `:`).
    command_buffer: String,
    /// Whether the app should quit.
    should_quit: bool,
    /// The exit result to return.
    exit_result: Option<ExitResult>,
}

impl App {
    pub fn new(source_name: String, _content: String) -> Self {
        Self {
            source_name,
            mode: Mode::Normal,
            keybinds: KeybindHandler::new(),
            annotations: AnnotationStore::new(),
            command_buffer: String::new(),
            should_quit: false,
            exit_result: None,
        }
    }

    /// Run the application main loop. Returns the exit result.
    pub fn run(mut self, terminal: &mut DefaultTerminal) -> io::Result<ExitResult> {
        while !self.should_quit {
            terminal.draw(|frame| self.render(frame))?;

            if let Event::Key(key_event) = event::read()? {
                self.handle_key(key_event);
            }
        }

        Ok(self.exit_result.unwrap_or(ExitResult::QuitSilent))
    }

    fn handle_key(&mut self, key_event: KeyEvent) {
        let action = self.keybinds.handle(self.mode, key_event);

        match action {
            // -- Mode transitions --
            Action::EnterVisualMode => self.mode = Mode::Visual,
            Action::EnterCommandMode => {
                self.mode = Mode::Command;
                self.command_buffer.clear();
            }
            Action::EnterAnnotationListMode => self.mode = Mode::AnnotationList,
            Action::ExitToNormal => self.mode = Mode::Normal,

            // -- Command mode --
            Action::CommandChar(c) => self.command_buffer.push(c),
            Action::CommandBackspace => {
                self.command_buffer.pop();
                if self.command_buffer.is_empty() {
                    self.mode = Mode::Normal;
                }
            }
            Action::CommandConfirm => self.execute_command(),

            // All other actions are no-ops for now (implemented in later tasks).
            _ => {}
        }
    }

    fn execute_command(&mut self) {
        match self.command_buffer.as_str() {
            "q" => {
                let output = PlannotatorExporter.export(&self.annotations);
                self.exit_result = Some(ExitResult::QuitWithOutput(output));
                self.should_quit = true;
            }
            "q!" => {
                self.exit_result = Some(ExitResult::QuitSilent);
                self.should_quit = true;
            }
            "w" => {
                let output = PlannotatorExporter.export(&self.annotations);
                // Print to stdout immediately (terminal is in alternate screen,
                // so this goes to the real stdout which the caller captures).
                // For now, store the output; actual piping is handled by the caller.
                // We write to stderr as a workaround since stdout is the TUI.
                // The proper `:w` mid-session write will be refined in later tasks.
                let _ = eprintln!("{output}");
            }
            _ => {}
        }
        self.command_buffer.clear();
        if !self.should_quit {
            self.mode = Mode::Normal;
        }
    }

    fn render(&self, frame: &mut Frame) {
        let area = frame.area();

        // Minimum terminal size check.
        if area.width < 40 || area.height < 10 {
            let msg = Paragraph::new("Terminal too small.\nPlease resize to at least 80×24.")
                .alignment(Alignment::Center);
            frame.render_widget(msg, area);
            return;
        }

        let [main_area, status_area] =
            Layout::vertical([Constraint::Min(1), Constraint::Length(1)]).areas(area);

        // -- Main document area (placeholder) --
        let doc = Paragraph::new("(document view — coming soon)")
            .block(Block::default().borders(Borders::ALL).title(format!(
                " {} ",
                self.source_name
            )));
        frame.render_widget(doc, main_area);

        // -- Status bar --
        let mode_label = match self.mode {
            Mode::Normal => " NORMAL ",
            Mode::Visual => " VISUAL ",
            Mode::Insert => " INSERT ",
            Mode::AnnotationList => " ANNOTATIONS ",
            Mode::Command => " COMMAND ",
        };

        let annotation_count = self.annotations.len();

        let mut status_spans = vec![
            Span::styled(
                mode_label,
                Style::default().add_modifier(Modifier::BOLD | Modifier::REVERSED),
            ),
            Span::raw(format!(" {}  ", self.source_name)),
            Span::raw(format!("{annotation_count} annotation(s)  ")),
        ];

        if self.mode == Mode::Command {
            status_spans.push(Span::raw(format!(":{}", self.command_buffer)));
        } else {
            status_spans.push(Span::raw("? help"));
        }

        let status_bar = Paragraph::new(Line::from(status_spans));
        frame.render_widget(status_bar, status_area);
    }
}
