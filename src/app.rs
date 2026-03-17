use std::io;

use crossterm::event::{self, Event, KeyEvent};
use ratatui::{
    DefaultTerminal, Frame,
    layout::{Alignment, Constraint, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};

use crate::annotation::export::{AnnotationExporter, PlannotatorExporter};
use crate::annotation::store::AnnotationStore;
use crate::highlight::syntect::SyntectHighlighter;
use crate::highlight::{Highlighter, StyledSpan};
use crate::keybinds::handler::{Action, KeybindHandler};
use crate::keybinds::mode::Mode;
use crate::markdown::block::BlockType;
use crate::markdown::parser::parse_markdown_to_blocks;
use crate::tui::viewport::Viewport;

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
}

impl App {
    pub fn new(source_name: String, content: String) -> Self {
        let blocks = parse_markdown_to_blocks(&content);
        let highlighter = SyntectHighlighter::new();
        let (doc_lines, styled_lines) = blocks_to_lines(&blocks, &highlighter);
        let line_lengths: Vec<usize> = doc_lines.iter().map(|l| l.len()).collect();

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
            doc_lines,
            styled_lines,
            viewport,
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

    fn render(&mut self, frame: &mut Frame) {
        let area = frame.area();

        // Update viewport dimensions (account for borders + status bar).
        let doc_height = area.height.saturating_sub(3) as usize; // 2 border rows + 1 status row
        let doc_width = area.width.saturating_sub(2) as usize; // 2 border columns
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
        let visible_lines: Vec<Line> = self.styled_lines[visible.clone()]
            .iter()
            .enumerate()
            .map(|(i, styled_spans)| {
                let doc_row = visible.start + i;
                let spans: Vec<Span> = styled_spans
                    .iter()
                    .map(|ss| Span::styled(ss.text.clone(), ss.style))
                    .collect();
                if doc_row == self.viewport.cursor.row {
                    apply_cursor_to_line(Line::from(spans), self.viewport.cursor.col)
                } else {
                    Line::from(spans)
                }
            })
            .collect();

        let doc = Paragraph::new(visible_lines).block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!(" {} ", self.source_name)),
        );
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

/// Apply a block cursor overlay to a pre-styled `Line` at the given column.
fn apply_cursor_to_line(line: Line<'_>, cursor_col: usize) -> Line<'_> {
    // Flatten all spans into chars with their original style.
    let mut chars_with_style: Vec<(char, Style)> = Vec::new();
    for span in line.spans.iter() {
        for c in span.content.chars() {
            chars_with_style.push((c, span.style));
        }
    }

    if chars_with_style.is_empty() {
        return Line::from(Span::styled(
            " ",
            Style::default().bg(Color::White).fg(Color::Black),
        ));
    }

    let col = cursor_col.min(chars_with_style.len().saturating_sub(1));
    let cursor_style = Style::default().bg(Color::White).fg(Color::Black);

    // Rebuild spans, applying cursor style to the character at `col`.
    let mut spans: Vec<Span> = Vec::new();
    let mut current_text = String::new();
    let mut current_style: Option<Style> = None;

    for (i, &(ch, style)) in chars_with_style.iter().enumerate() {
        let effective_style = if i == col { cursor_style } else { style };

        match current_style {
            Some(s) if s == effective_style => {
                current_text.push(ch);
            }
            _ => {
                if let Some(s) = current_style {
                    spans.push(Span::styled(std::mem::take(&mut current_text), s));
                }
                current_text.push(ch);
                current_style = Some(effective_style);
            }
        }
    }
    if let Some(s) = current_style {
        spans.push(Span::styled(current_text, s));
    }

    Line::from(spans)
}

/// Convert parsed markdown blocks into flat document lines.
///
/// Returns `(plain_lines, styled_lines)`:
/// - `plain_lines`: raw text per line (for cursor movement / word logic).
/// - `styled_lines`: highlighted spans per line (for rendering).
fn blocks_to_lines(
    blocks: &[crate::markdown::block::Block],
    highlighter: &dyn Highlighter,
) -> (Vec<String>, Vec<Vec<StyledSpan>>) {
    let mut plain: Vec<String> = Vec::new();
    let mut styled: Vec<Vec<StyledSpan>> = Vec::new();

    let heading_style = Style::default()
        .fg(Color::Cyan)
        .add_modifier(Modifier::BOLD);
    let blockquote_style = Style::default().fg(Color::DarkGray);
    let hr_style = Style::default().fg(Color::DarkGray);
    let code_fence_style = Style::default().fg(Color::DarkGray);
    let list_marker_style = Style::default().fg(Color::Yellow);
    let checkbox_style = Style::default().fg(Color::Green);

    for block in blocks {
        match &block.block_type {
            BlockType::Heading => {
                let prefix = "#".repeat(block.level);
                let text = format!("{prefix} {}", block.content);
                plain.push(text.clone());
                styled.push(vec![StyledSpan::new(text, heading_style)]);
            }
            BlockType::Paragraph => {
                for line in block.content.split('\n') {
                    plain.push(line.to_string());
                    styled.push(highlighter.highlight_line(line));
                }
            }
            BlockType::Code => {
                let lang = block.language.as_deref().unwrap_or("");
                let fence_open = format!("```{lang}");
                plain.push(fence_open.clone());
                styled.push(vec![StyledSpan::new(fence_open, code_fence_style)]);

                let highlighted = highlighter.highlight_code_block(
                    &block.content,
                    block.language.as_deref(),
                );
                for (code_line, spans) in block.content.split('\n').zip(highlighted) {
                    let indented = format!("  {code_line}");
                    plain.push(indented);
                    // Prepend indent to highlighted spans.
                    let mut line_spans = vec![StyledSpan::new("  ", Style::default())];
                    line_spans.extend(spans);
                    styled.push(line_spans);
                }

                plain.push("```".to_string());
                styled.push(vec![StyledSpan::new("```", code_fence_style)]);
            }
            BlockType::Blockquote => {
                let text = format!("> {}", block.content);
                plain.push(text.clone());
                styled.push(vec![StyledSpan::new(text, blockquote_style)]);
            }
            BlockType::ListItem => {
                let indent = "  ".repeat(block.level);
                let (marker, content_spans) = if let Some(checked) = block.checked {
                    let marker = if checked { "- [x] " } else { "- [ ] " };
                    (marker, vec![
                        StyledSpan::new(&indent, Style::default()),
                        StyledSpan::new(marker, checkbox_style),
                    ])
                } else {
                    ("- ", vec![
                        StyledSpan::new(&indent, Style::default()),
                        StyledSpan::new("- ", list_marker_style),
                    ])
                };
                let text = format!("{indent}{marker}{}", block.content);
                plain.push(text);
                let mut spans = content_spans;
                spans.extend(highlighter.highlight_line(&block.content));
                styled.push(spans);
            }
            BlockType::HorizontalRule => {
                plain.push("───".to_string());
                styled.push(vec![StyledSpan::new("───", hr_style)]);
            }
            BlockType::Table => {
                for line in block.content.split('\n') {
                    plain.push(line.to_string());
                    styled.push(vec![StyledSpan::plain(line)]);
                }
            }
        }
        // Blank line between blocks.
        plain.push(String::new());
        styled.push(vec![]);
    }

    // Remove trailing blank line.
    if plain.last().is_some_and(|l| l.is_empty()) {
        plain.pop();
        styled.pop();
    }

    (plain, styled)
}
