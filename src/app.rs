use std::io;

use crossterm::event::{self, Event, KeyEvent};
use ratatui::{
    DefaultTerminal, Frame,
    layout::{Alignment, Constraint, Layout},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Paragraph},
};

use crate::annotation::export::{AnnotationExporter, PlannotatorExporter};
use crate::annotation::store::AnnotationStore;
use crate::highlight::StyledSpan;
use crate::highlight::syntect::SyntectHighlighter;
use crate::keybinds::handler::{Action, KeybindHandler};
use crate::keybinds::mode::Mode;
use crate::tui::renderer;
use crate::tui::selection::Selection;
use crate::tui::theme::Theme;
use crate::tui::viewport::{CursorPosition, Viewport};

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
    /// Plain-text document lines (for cursor movement / word logic).
    doc_lines: Vec<String>,
    /// Highlighted document lines (for rendering with syntax highlighting).
    styled_lines: Vec<Vec<StyledSpan>>,
    /// Viewport state (scroll, cursor, dimensions).
    viewport: Viewport,
    /// Centralized theme styles.
    theme: Theme,
    /// Anchor position when in Visual mode. Set when entering Visual, cleared on exit.
    visual_anchor: Option<CursorPosition>,
}

impl App {
    pub fn new(source_name: String, content: String) -> Self {
        let highlighter = SyntectHighlighter::new();
        let theme = Theme::new();
        let doc_lines_result = renderer::text_to_lines(&content, &highlighter);
        let line_lengths: Vec<usize> = doc_lines_result
            .plain
            .iter()
            .map(|l| l.chars().count())
            .collect();

        let mut viewport = Viewport::new();
        viewport.set_line_info(line_lengths);

        Self {
            source_name,
            mode: Mode::Normal,
            keybinds: KeybindHandler::new(),
            annotations: AnnotationStore::new(),
            command_buffer: String::new(),
            should_quit: false,
            exit_result: None,
            doc_lines: doc_lines_result.plain,
            styled_lines: doc_lines_result.styled,
            viewport,
            theme,
            visual_anchor: None,
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

        match action {
            // -- Movement --
            Action::MoveUp => self.viewport.move_up(),
            Action::MoveDown => self.viewport.move_down(),
            Action::MoveLeft => self.viewport.move_left(),
            Action::MoveRight => self.viewport.move_right(),
            Action::MoveWordForward => {
                let lines: Vec<&str> = self.doc_lines.iter().map(|s| s.as_str()).collect();
                self.viewport.move_word_forward(&lines);
            }
            Action::MoveWordBackward => {
                let lines: Vec<&str> = self.doc_lines.iter().map(|s| s.as_str()).collect();
                self.viewport.move_word_backward(&lines);
            }
            Action::MoveLineStart => self.viewport.move_line_start(),
            Action::MoveLineEnd => self.viewport.move_line_end(),
            Action::MoveDocumentTop => self.viewport.move_document_top(),
            Action::MoveDocumentBottom => self.viewport.move_document_bottom(),
            Action::HalfPageDown => self.viewport.half_page_down(),
            Action::HalfPageUp => self.viewport.half_page_up(),
            Action::FullPageDown => self.viewport.full_page_down(),
            Action::FullPageUp => self.viewport.full_page_up(),

            // -- Mode transitions --
            Action::EnterVisualMode => {
                self.mode = Mode::Visual;
                self.visual_anchor = Some(self.viewport.cursor);
            }
            Action::EnterCommandMode => {
                self.mode = Mode::Command;
                self.command_buffer.clear();
            }
            Action::EnterAnnotationListMode => self.mode = Mode::AnnotationList,
            Action::ExitToNormal => {
                self.mode = Mode::Normal;
                self.visual_anchor = None;
            }

            // -- Command mode --
            Action::CommandChar(c) => self.command_buffer.push(c),
            Action::CommandBackspace => {
                self.command_buffer.pop();
                if self.command_buffer.is_empty() {
                    self.mode = Mode::Normal;
                }
            }
            Action::CommandConfirm => self.execute_command(),

            // -- Annotation creation from Visual mode (stubs — implemented in step 12) --
            Action::CreateDeletion | Action::CreateComment | Action::CreateReplacement => {
                self.mode = Mode::Normal;
                self.visual_anchor = None;
            }

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
                eprintln!("{output}");
            }
            _ => {}
        }
        self.command_buffer.clear();
        if !self.should_quit {
            self.mode = Mode::Normal;
        }
    }

    fn render(&mut self, frame: &mut Frame) {
        let area = frame.area();

        // Update viewport dimensions (account for borders + status bar).
        let doc_height = area.height.saturating_sub(1) as usize; // leave room for status row
        let doc_width = area.width as usize; // 2 border columns
        self.viewport.set_dimensions(doc_width, doc_height);

        // Minimum terminal size check.
        if self.viewport.is_too_small() {
            let msg = Paragraph::new("Terminal too small.\nPlease resize to at least 80×24.")
                .alignment(Alignment::Center);
            frame.render_widget(msg, area);
            return;
        }

        let [main_area, status_area] =
            Layout::vertical([Constraint::Min(1), Constraint::Length(1)]).areas(area);

        // -- Main document area --
        let visible = self.viewport.visible_range();

        // Compute normalized selection range when in Visual mode.
        let selection = if self.mode == Mode::Visual {
            self.visual_anchor.map(|anchor| {
                let sel = Selection { anchor };
                sel.range(self.viewport.cursor)
            })
        } else {
            None
        };

        let visible_lines = renderer::prepare_visible_lines(
            &self.styled_lines[visible.clone()],
            &self.doc_lines[visible.clone()],
            visible.start,
            self.viewport.cursor.row,
            self.viewport.cursor.col,
            &self.theme,
            selection,
        );

        let doc = Paragraph::new(visible_lines).block(Block::default());
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
        let cursor_pos = format!(
            "{}:{}",
            self.viewport.cursor.row + 1,
            self.viewport.cursor.col + 1
        );

        let mut status_spans = vec![
            Span::styled(
                mode_label,
                Style::default().add_modifier(Modifier::BOLD | Modifier::REVERSED),
            ),
            Span::raw(format!(" {}  ", self.source_name)),
            Span::raw(format!("{annotation_count} annotation(s)  ")),
            Span::raw(format!("{cursor_pos}  ")),
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
