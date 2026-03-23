use std::io;

use crossterm::event::{self, Event, KeyEvent};
use ratatui::{
    DefaultTerminal, Frame,
    layout::{Alignment, Constraint, Flex, Layout},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Paragraph},
};

use crate::annotation::export::{AnnotationExporter, PlannotatorExporter};
use crate::annotation::store::AnnotationStore;
use crate::annotation::types::{Annotation, TextPosition, TextRange};
use crate::document::Document;
use crate::highlight::syntect::SyntectHighlighter;
use crate::keybinds::handler::{Action, KeybindHandler};
use crate::keybinds::mode::Mode;
use crate::tui::input_box::{InputBox, InputBoxEvent};
use crate::tui::renderer;
use crate::tui::selection::{self, Selection};
use crate::tui::theme::Theme;
use crate::tui::viewport::{CursorPosition, DisplayLayout, Viewport};

const MAX_DOC_WIDTH: u16 = 120;

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
    /// The document being annotated.
    document: Document,
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
    /// Viewport state (scroll, cursor, dimensions).
    viewport: Viewport,
    /// Centralized theme styles.
    theme: Theme,
    /// Anchor position when in Visual mode. Set when entering Visual, cleared on exit.
    visual_anchor: Option<CursorPosition>,
    /// Display layout mapping document lines → display rows.
    display_layout: DisplayLayout,
    /// Active input box (shown in Insert mode for annotation text entry).
    input_box: Option<InputBox<'static>>,
    /// The pending annotation being created (set when entering Insert mode).
    pending_annotation: Option<PendingAnnotation>,
}

impl App {
    pub fn new(source_name: String, content: String) -> Self {
        let highlighter = SyntectHighlighter::new();
        let theme = Theme::new();
        let document = Document::new(source_name, &content, &highlighter);

        let mut viewport = Viewport::new();

        // Initial layout (width 0 until first render sets dimensions).
        let display_layout = DisplayLayout::build(&document.lines, 0, false);

        // Ensure viewport knows about initial dimensions (0×0 is fine; render will update).
        viewport.set_dimensions(0, 0);

        Self {
            document,
            mode: Mode::Normal,
            keybinds: KeybindHandler::new(),
            annotations: AnnotationStore::new(),
            command_buffer: String::new(),
            should_quit: false,
            exit_result: None,
            viewport,
            theme,
            visual_anchor: None,
            display_layout,
            input_box: None,
            pending_annotation: None,
        }
    }

    /// Run the application main loop. Returns the exit result.
    pub fn run(mut self, terminal: &mut DefaultTerminal) -> io::Result<ExitResult> {
        while !self.should_quit {
            terminal.draw(|frame| {
                self.update(frame.area());
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
            Action::MoveUp => self.viewport.move_up(&self.display_layout),
            Action::MoveDown => self.viewport.move_down(&self.display_layout),
            Action::MoveLeft => self.viewport.move_left(&self.display_layout),
            Action::MoveRight => self.viewport.move_right(&self.display_layout),
            Action::MoveWordForward => {
                let lines: Vec<&str> = self.document.lines.iter().map(|s| s.as_str()).collect();
                self.viewport
                    .move_word_forward(&lines, &self.display_layout);
            }
            Action::MoveWordBackward => {
                let lines: Vec<&str> = self.document.lines.iter().map(|s| s.as_str()).collect();
                self.viewport
                    .move_word_backward(&lines, &self.display_layout);
            }
            Action::MoveLineStart => self.viewport.move_line_start(&self.display_layout),
            Action::MoveLineEnd => self.viewport.move_line_end(&self.display_layout),
            Action::MoveDocumentTop => self.viewport.move_document_top(&self.display_layout),
            Action::MoveDocumentBottom => self.viewport.move_document_bottom(&self.display_layout),
            Action::HalfPageDown => self.viewport.half_page_down(&self.display_layout),
            Action::HalfPageUp => self.viewport.half_page_up(&self.display_layout),
            Action::FullPageDown => self.viewport.full_page_down(&self.display_layout),
            Action::FullPageUp => self.viewport.full_page_up(&self.display_layout),

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
                self.input_box = None;
                self.pending_annotation = None;
            }

            // -- Word wrap --
            Action::ToggleWordWrap => {
                self.viewport.toggle_word_wrap();
                self.rebuild_display_layout();
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

            // -- Annotation creation from Visual mode --
            Action::CreateDeletion => self.create_deletion(),
            Action::CreateComment => self.start_input_for_visual_annotation("Comment"),
            Action::CreateReplacement => self.start_input_for_visual_annotation("Replacement"),

            // -- Annotation creation from Normal mode --
            Action::CreateInsertion => {
                let position = TextPosition {
                    line: self.viewport.cursor.row,
                    column: self.viewport.cursor.col,
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

    /// Extract the current visual selection as a TextRange and selected text.
    /// Returns `None` if there is no active visual anchor.
    fn take_visual_selection(&mut self) -> Option<(TextRange, String)> {
        let anchor = self.visual_anchor.take()?;
        let sel = Selection { anchor };
        let (start, end) = sel.range(self.viewport.cursor);
        let range = TextRange {
            start: TextPosition {
                line: start.row,
                column: start.col,
            },
            end: TextPosition {
                line: end.row,
                column: end.col,
            },
        };
        let text = selection::selected_text(start, end, &self.document.lines);
        Some((range, text))
    }

    /// Create a Deletion annotation from the current visual selection.
    fn create_deletion(&mut self) {
        if let Some((range, text)) = self.take_visual_selection() {
            self.annotations.add(Annotation::deletion(range, text));
        }
        self.mode = Mode::Normal;
    }

    /// Begin input for a Comment or Replacement from visual mode.
    fn start_input_for_visual_annotation(&mut self, kind: &str) {
        if let Some((range, selected_text)) = self.take_visual_selection() {
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

    fn rebuild_display_layout(&mut self) {
        self.display_layout = DisplayLayout::build(
            &self.document.lines,
            self.viewport.width,
            self.viewport.word_wrap,
        );
    }

    fn update(&mut self, area: ratatui::layout::Rect) {
        // Update viewport dimensions (account for borders + status bar).
        let doc_height = area.height.saturating_sub(1) as usize; // leave room for status row
        let doc_width = (area.width as usize).min(MAX_DOC_WIDTH as usize);
        let old_width = self.viewport.width;
        self.viewport.set_dimensions(doc_width, doc_height);

        // Rebuild display layout when width changes (affects word-wrap).
        if doc_width != old_width {
            self.rebuild_display_layout();
        }
    }

    fn render(&self, frame: &mut Frame) {
        let area = frame.area();

        // Minimum terminal size check.
        if self.viewport.is_too_small() {
            let msg = Paragraph::new("Terminal too small.\nPlease resize to at least 80×24.")
                .alignment(Alignment::Center);
            frame.render_widget(msg, area);
            return;
        }

        let [main_area, status_area] =
            Layout::vertical([Constraint::Min(1), Constraint::Length(1)]).areas(area);

        // Cap the main content width at MAX_DOC_WIDTH columns and center it.
        let main_area = Layout::horizontal([Constraint::Max(MAX_DOC_WIDTH)])
            .flex(Flex::Center)
            .areas::<1>(main_area)[0];

        // -- Main document area --
        let render_slices = self.viewport.visible_render_slices(&self.display_layout);

        // Compute normalized selection range when in Visual mode.
        let selection = if self.mode == Mode::Visual {
            self.visual_anchor.map(|anchor| {
                let sel = Selection { anchor };
                sel.range(self.viewport.cursor)
            })
        } else {
            None
        };

        // Collect annotation ranges for visual indicators.
        let annotation_ranges: Vec<_> = self
            .annotations
            .all()
            .iter()
            .filter_map(|a| a.range)
            .collect();

        let visible_lines = renderer::prepare_visible_lines_from_slices(
            &render_slices,
            &self.document.styled_lines,
            &self.document.lines,
            self.viewport.cursor.row,
            self.viewport.cursor.col,
            &self.theme,
            selection,
            &annotation_ranges,
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

        let wrap_indicator = if self.viewport.word_wrap { "wrap " } else { "" };

        let mut status_spans = vec![
            Span::styled(
                mode_label,
                Style::default().add_modifier(Modifier::BOLD | Modifier::REVERSED),
            ),
            Span::raw(format!(" {}  ", self.document.source_name)),
            Span::raw(format!("{annotation_count} annotation(s)  ")),
            Span::raw(format!("{cursor_pos}  ")),
            Span::raw(wrap_indicator),
        ];

        if self.mode == Mode::Command {
            status_spans.push(Span::raw(format!(":{}", self.command_buffer)));
        } else if self.mode == Mode::Insert {
            status_spans.push(Span::raw("Ctrl+S confirm  Esc cancel"));
        } else {
            status_spans.push(Span::raw("? help"));
        }

        let status_bar = Paragraph::new(Line::from(status_spans));
        frame.render_widget(status_bar, status_area);

        // -- Input box overlay --
        if let Some(ref ib) = self.input_box {
            ib.render(frame, main_area);
        }
    }
}
