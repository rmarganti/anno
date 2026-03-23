use std::io;

use crossterm::event::{self, Event, KeyEvent};
use ratatui::{
    layout::{Alignment, Constraint, Flex, Layout},
    widgets::Paragraph,
    DefaultTerminal, Frame,
};

use crate::annotation::export::{AnnotationExporter, PlannotatorExporter};
use crate::annotation::store::AnnotationStore;
use crate::annotation::types::{Annotation, TextPosition, TextRange};
use crate::document::Document;
use crate::highlight::syntect::SyntectHighlighter;
use crate::keybinds::handler::{Action, KeybindHandler};
use crate::keybinds::mode::Mode;
use crate::tui::annotation_list::{AnnotationList, AnnotationListEvent};
use crate::tui::command_line::{CommandLine, CommandLineEvent, CommandResult};
use crate::tui::document_viewer::{DocumentViewer, DocumentViewerEvent};
use crate::tui::input_box::{InputBox, InputBoxEvent};

const MAX_DOC_WIDTH: u16 = 120;
/// Width of the annotation list sidebar (in terminal columns).
const ANNOTATION_SIDEBAR_WIDTH: u16 = 40;

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

impl PendingAnnotation {
    /// Human-readable label used as the title of the input box.
    fn label(&self) -> &'static str {
        match self {
            PendingAnnotation::Comment { .. } => "Comment",
            PendingAnnotation::Replacement { .. } => "Replacement",
            PendingAnnotation::Insertion { .. } => "Insertion",
            PendingAnnotation::GlobalComment => "Global Comment",
        }
    }
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
    /// Command-line component (buffer + execution).
    command_line: CommandLine,
    /// Whether the app should quit.
    should_quit: bool,
    /// The exit result to return.
    exit_result: Option<ExitResult>,
    /// Document viewer component (viewport, layout, visual selection, theme).
    doc_viewer: DocumentViewer,
    /// Active input box (shown in Insert mode for annotation text entry).
    input_box: Option<InputBox<'static>>,
    /// The pending annotation being created (set when entering Insert mode).
    pending_annotation: Option<PendingAnnotation>,
    /// Annotation list sidebar component.
    annotation_list: AnnotationList,
}

impl App {
    pub fn new(source_name: String, content: String) -> Self {
        let highlighter = SyntectHighlighter::new();
        let document = Document::new(source_name, &content, &highlighter);
        let doc_viewer = DocumentViewer::new(&document);

        Self {
            document,
            mode: Mode::Normal,
            keybinds: KeybindHandler::new(),
            annotations: AnnotationStore::new(),
            command_line: CommandLine::new(),
            should_quit: false,
            exit_result: None,
            doc_viewer,
            input_box: None,
            pending_annotation: None,
            annotation_list: AnnotationList::new(),
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

        // Delegate movement, visual mode, and word-wrap actions to the document viewer
        // when we're in a mode where it makes sense.
        match self.mode {
            Mode::Normal | Mode::Visual => {
                let event = self.doc_viewer.handle_action(&action, &self.document);
                match event {
                    DocumentViewerEvent::EnteredVisual => {
                        self.mode = Mode::Visual;
                        return;
                    }
                    DocumentViewerEvent::ExitedVisual => {
                        self.mode = Mode::Normal;
                        self.input_box = None;
                        self.pending_annotation = None;
                        return;
                    }
                    DocumentViewerEvent::VisualSelection {
                        range,
                        selected_text,
                    } => {
                        // The visual selection was taken; determine what annotation to create.
                        match &action {
                            Action::CreateDeletion => {
                                self.annotations
                                    .add(Annotation::deletion(range, selected_text));
                                self.mode = Mode::Normal;
                            }
                            Action::CreateComment => {
                                self.start_annotation_input(PendingAnnotation::Comment {
                                    range,
                                    selected_text,
                                });
                            }
                            Action::CreateReplacement => {
                                self.start_annotation_input(PendingAnnotation::Replacement {
                                    range,
                                    selected_text,
                                });
                            }
                            _ => {}
                        }
                        return;
                    }
                    DocumentViewerEvent::Consumed => {
                        return;
                    }
                    DocumentViewerEvent::Unhandled => {
                        // Fall through to handle remaining actions below.
                    }
                }
            }
            Mode::AnnotationList => {
                let event = self
                    .annotation_list
                    .handle_action(&action, &self.annotations);
                match event {
                    AnnotationListEvent::JumpTo { line } => {
                        self.doc_viewer
                            .viewport
                            .jump_to_line(line, &self.doc_viewer.display_layout);
                        self.mode = Mode::Normal;
                        return;
                    }
                    AnnotationListEvent::Delete { id } => {
                        self.annotations.delete(id);
                        self.annotation_list.clamp(self.annotations.len());
                        return;
                    }
                    AnnotationListEvent::Exit => {
                        self.mode = Mode::Normal;
                        return;
                    }
                    AnnotationListEvent::Consumed => {
                        return;
                    }
                }
            }
            _ => {}
        }

        // Handle actions not delegated to the document viewer.
        match action {
            // -- Mode transitions (not handled by viewer) --
            Action::EnterCommandMode => {
                self.mode = Mode::Command;
                self.command_line.clear();
            }
            Action::EnterAnnotationListMode => self.mode = Mode::AnnotationList,
            Action::ExitToNormal => {
                self.mode = Mode::Normal;
                self.command_line.clear();
                self.input_box = None;
                self.pending_annotation = None;
            }

            // -- Command mode: forward all command actions to CommandLine --
            Action::CommandChar(_) | Action::CommandBackspace | Action::CommandConfirm => {
                match self.command_line.handle_action(&action) {
                    CommandLineEvent::Consumed => {}
                    CommandLineEvent::ExitToNormal => {
                        self.mode = Mode::Normal;
                    }
                    CommandLineEvent::Cancelled => {
                        self.mode = Mode::Normal;
                    }
                    CommandLineEvent::Executed(result) => {
                        self.handle_command_result(result);
                    }
                }
            }

            // -- Annotation navigation (]a / [a) --
            Action::NextAnnotation => {
                let ordered = self.annotations.ordered();
                let current_line = self.doc_viewer.viewport.cursor.row;
                // Find the first annotation whose start line is strictly after the current line.
                if let Some(annotation) = ordered.iter().find(|a| {
                    a.range
                        .map(|r| r.start.line > current_line)
                        .unwrap_or(false)
                }) {
                    let line = annotation.range.unwrap().start.line;
                    self.doc_viewer
                        .viewport
                        .jump_to_line(line, &self.doc_viewer.display_layout);
                }
            }
            Action::PrevAnnotation => {
                let ordered = self.annotations.ordered();
                let current_line = self.doc_viewer.viewport.cursor.row;
                // Find the last annotation whose start line is strictly before the current line.
                if let Some(annotation) = ordered.iter().rev().find(|a| {
                    a.range
                        .map(|r| r.start.line < current_line)
                        .unwrap_or(false)
                }) {
                    let line = annotation.range.unwrap().start.line;
                    self.doc_viewer
                        .viewport
                        .jump_to_line(line, &self.doc_viewer.display_layout);
                }
            }

            // -- Annotation creation from Normal mode --
            Action::CreateInsertion => {
                let position: TextPosition = self.doc_viewer.viewport.cursor.into();
                self.start_annotation_input(PendingAnnotation::Insertion { position });
            }
            Action::CreateGlobalComment => {
                self.start_annotation_input(PendingAnnotation::GlobalComment);
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

    /// Begin collecting text input for the given annotation kind.
    ///
    /// Sets `pending_annotation`, opens the input box, and transitions to Insert mode.
    fn start_annotation_input(&mut self, pending: PendingAnnotation) {
        self.input_box = Some(InputBox::new(pending.label()));
        self.pending_annotation = Some(pending);
        self.mode = Mode::Insert;
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

    /// Act on a parsed command result from `CommandLine`.
    fn handle_command_result(&mut self, result: CommandResult) {
        match result {
            CommandResult::Quit => {
                let output = PlannotatorExporter.export(&self.annotations);
                self.exit_result = Some(ExitResult::QuitWithOutput(output));
                self.should_quit = true;
            }
            CommandResult::QuitForce => {
                self.exit_result = Some(ExitResult::QuitSilent);
                self.should_quit = true;
            }
            CommandResult::Write => {
                let output = PlannotatorExporter.export(&self.annotations);
                // Print to stdout immediately (terminal is in alternate screen,
                // so this goes to the real stdout which the caller captures).
                // For now, store the output; actual piping is handled by the caller.
                // We write to stderr as a workaround since stdout is the TUI.
                // The proper `:w` mid-session write will be refined in later tasks.
                eprintln!("{output}");
            }
            CommandResult::Unknown => {}
        }
        if !self.should_quit {
            self.mode = Mode::Normal;
        }
    }

    fn update(&mut self, area: ratatui::layout::Rect) {
        // Account for status bar (leave room for status row).
        let doc_height = area.height.saturating_sub(1) as usize;
        // Cap content width and center it — same as the layout constraint below.
        let max_width = MAX_DOC_WIDTH as usize;

        // Compute the centered area width (mirrors Layout::horizontal below).
        let available_width = area.width as usize;
        let doc_width = available_width.min(max_width);

        let old_width = self.doc_viewer.viewport.width;
        self.doc_viewer
            .viewport
            .set_dimensions(doc_width, doc_height);

        if doc_width != old_width {
            self.doc_viewer.rebuild_display_layout(&self.document);
        }
    }

    fn render(&self, frame: &mut Frame) {
        let area = frame.area();

        // Minimum terminal size check.
        if self.doc_viewer.viewport.is_too_small() {
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

        // When in AnnotationList mode (and there are annotations), split the main
        // area into a document pane (left) and an annotation sidebar (right).
        let show_sidebar = self.mode == Mode::AnnotationList && !self.annotations.is_empty();

        let (doc_area, sidebar_area_opt) = if show_sidebar {
            let [doc, sidebar] = Layout::horizontal([
                Constraint::Min(40),
                Constraint::Length(ANNOTATION_SIDEBAR_WIDTH),
            ])
            .areas(main_area);
            (doc, Some(sidebar))
        } else {
            (main_area, None)
        };

        // -- Main document area --
        // Collect annotation ranges for visual indicators.
        let annotation_ranges: Vec<_> = self
            .annotations
            .all()
            .iter()
            .filter_map(|a| a.range)
            .collect();

        self.doc_viewer.render(
            frame,
            doc_area,
            &self.document,
            self.mode == Mode::Visual,
            &annotation_ranges,
        );

        // -- Annotation list sidebar --
        if let Some(sidebar_area) = sidebar_area_opt {
            self.annotation_list
                .render(frame, sidebar_area, &self.annotations);
        }

        // -- Status bar --
        let status_bar = crate::tui::status_bar::render(
            self.mode,
            &self.document.source_name,
            self.annotations.len(),
            self.doc_viewer.viewport.cursor.row,
            self.doc_viewer.viewport.cursor.col,
            self.doc_viewer.viewport.word_wrap,
            &self.command_line.buffer,
        );
        frame.render_widget(status_bar, status_area);

        // -- Input box overlay --
        if let Some(ref ib) = self.input_box {
            ib.render(frame, main_area);
        }
    }
}
