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
use crate::tui::viewport::CursorPosition;

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

    pub fn mode(&self) -> Mode {
        self.mode
    }

    pub fn should_quit(&self) -> bool {
        self.should_quit
    }

    pub fn take_exit_result(&mut self) -> Option<ExitResult> {
        self.exit_result.take()
    }

    pub fn cursor(&self) -> CursorPosition {
        self.document_view.cursor()
    }

    pub fn annotation_count(&self) -> usize {
        self.annotations.len()
    }

    pub fn annotations(&self) -> &AnnotationStore {
        &self.annotations
    }

    pub fn has_confirm_dialog(&self) -> bool {
        self.confirm_dialog.is_some()
    }

    pub fn is_panel_visible(&self) -> bool {
        self.annotation_list_panel.is_visible()
    }

    pub fn command_buffer(&self) -> &str {
        self.command_line.buffer()
    }

    pub fn word_wrap(&self) -> bool {
        self.document_view.word_wrap()
    }

    pub fn source_name(&self) -> &str {
        &self.source_name
    }

    pub fn document_view(&self) -> &DocumentView {
        &self.document_view
    }

    pub fn document_view_mut(&mut self) -> &mut DocumentView {
        &mut self.document_view
    }

    pub fn annotation_controller(&self) -> &AnnotationController {
        &self.annotation_controller
    }

    pub fn confirm_dialog(&self) -> Option<&ConfirmDialog> {
        self.confirm_dialog.as_ref()
    }

    pub fn annotation_list_panel(&self) -> &AnnotationListPanel {
        &self.annotation_list_panel
    }

    pub fn handle_key(&mut self, key_event: KeyEvent) {
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
        while !self.state.should_quit() {
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

        Ok(self
            .state
            .take_exit_result()
            .unwrap_or(ExitResult::QuitSilent))
    }

    fn render(&mut self, frame: &mut Frame) {
        let area = frame.area();

        // Decide whether the panel should actually be shown: it must be
        // toggled visible AND the terminal must be wide enough.
        let show_panel = self.state.is_panel_visible() && area.width >= MIN_WIDTH_FOR_PANEL;

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

        // Collect annotation ranges for visual indicators.
        let annotation_ranges: Vec<_> = self
            .state
            .annotations()
            .all()
            .iter()
            .filter_map(|a| a.range)
            .collect();

        // Resolve the selected annotation's text range (if any) for document highlighting.
        let selected_annotation_range = if show_panel {
            self.state
                .annotation_list_panel()
                .selected_annotation_id()
                .and_then(|id| self.state.annotations().get(id))
                .and_then(|a| a.range)
        } else {
            None
        };

        // -- Annotation list panel --
        if let Some(panel_area) = panel_area {
            self.state.annotation_list_panel().render(
                frame,
                panel_area,
                self.state.annotations(),
                &self.theme,
            );
        }

        // -- Main document area --
        let is_visual = self.state.mode() == Mode::Visual;
        self.state.document_view_mut().render(
            frame,
            doc_area,
            &self.theme,
            is_visual,
            &annotation_ranges,
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
                source_name: self.state.source_name(),
                annotation_count: self.state.annotation_count(),
                cursor_row: cursor.row,
                cursor_col: cursor.col,
                word_wrap: self.state.word_wrap(),
                command_buffer: self.state.command_buffer(),
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
    }
}

#[cfg(test)]
#[allow(dead_code)]
pub(crate) mod test_harness {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    use super::AppState;
    use crate::keybinds::mode::Mode;

    pub struct AppTestHarness {
        state: AppState,
    }

    impl AppTestHarness {
        pub fn new(content: &str) -> Self {
            let mut state = AppState::new_plain("[test]".to_string(), content.to_string());
            state.document_view_mut().update_dimensions(80, 24);

            Self { state }
        }

        pub fn key(&mut self, code: KeyCode) -> &mut Self {
            self.key_mod(code, KeyModifiers::NONE)
        }

        pub fn key_mod(&mut self, code: KeyCode, modifiers: KeyModifiers) -> &mut Self {
            self.state.handle_key(KeyEvent::new(code, modifiers));
            self
        }

        pub fn keys(&mut self, sequence: &str) -> &mut Self {
            for key_event in parse_key_sequence(sequence) {
                self.state.handle_key(key_event);
            }
            self
        }

        pub fn state(&self) -> &AppState {
            &self.state
        }

        pub fn state_mut(&mut self) -> &mut AppState {
            &mut self.state
        }

        pub fn assert_mode(&mut self, expected: Mode) -> &mut Self {
            assert_eq!(self.state.mode(), expected);
            self
        }

        pub fn assert_annotation_count(&mut self, expected: usize) -> &mut Self {
            assert_eq!(self.state.annotation_count(), expected);
            self
        }

        pub fn assert_cursor(&mut self, row: usize, col: usize) -> &mut Self {
            let cursor = self.state.cursor();
            assert_eq!(cursor.row, row);
            assert_eq!(cursor.col, col);
            self
        }

        pub fn assert_should_quit(&mut self) -> &mut Self {
            assert!(self.state.should_quit());
            self
        }

        pub fn assert_not_quit(&mut self) -> &mut Self {
            assert!(!self.state.should_quit());
            self
        }

        pub fn assert_has_confirm_dialog(&mut self) -> &mut Self {
            assert!(self.state.has_confirm_dialog());
            self
        }

        pub fn assert_no_confirm_dialog(&mut self) -> &mut Self {
            assert!(!self.state.has_confirm_dialog());
            self
        }

        pub fn assert_panel_visible(&mut self) -> &mut Self {
            assert!(self.state.is_panel_visible());
            self
        }

        pub fn assert_panel_hidden(&mut self) -> &mut Self {
            assert!(!self.state.is_panel_visible());
            self
        }
    }

    fn parse_key_sequence(sequence: &str) -> Vec<KeyEvent> {
        let mut events = Vec::new();
        let mut chars = sequence.chars().peekable();

        while let Some(ch) = chars.next() {
            if ch == '<' {
                let mut token = String::new();

                loop {
                    let next = chars
                        .next()
                        .expect("special key token must terminate with '>'");
                    if next == '>' {
                        break;
                    }
                    token.push(next);
                }

                events.push(parse_special_token(&token));
                continue;
            }

            events.push(parse_char_key(ch));
        }

        events
    }

    fn parse_special_token(token: &str) -> KeyEvent {
        match token {
            "Esc" => KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE),
            "Enter" => KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE),
            "Tab" => KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE),
            "BS" => KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE),
            "C-s" => KeyEvent::new(KeyCode::Char('s'), KeyModifiers::CONTROL),
            "C-c" => KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL),
            _ => panic!("unsupported special key token: <{token}>"),
        }
    }

    fn parse_char_key(ch: char) -> KeyEvent {
        let modifiers = if ch.is_ascii_uppercase() {
            KeyModifiers::SHIFT
        } else {
            KeyModifiers::NONE
        };

        KeyEvent::new(KeyCode::Char(ch), modifiers)
    }
}

#[cfg(test)]
mod tests {
    use super::test_harness::AppTestHarness;
    use super::{AppState, ExitResult};
    use crate::annotation::types::AnnotationType;
    use crate::keybinds::mode::Mode;

    fn harness(content: &str) -> AppTestHarness {
        AppTestHarness::new(content)
    }

    fn create_two_deletions(harness: &mut AppTestHarness) {
        harness.keys("vldjvld").assert_annotation_count(2);
    }

    #[test]
    fn new_plain_builds_terminal_independent_default_state() {
        let state = AppState::new_plain("[stdin]".to_string(), "first\nsecond".to_string());

        assert_eq!(state.source_name(), "[stdin]");
        assert_eq!(state.mode(), Mode::Normal);
        assert!(state.annotations().is_empty());
        assert_eq!(state.annotation_count(), 0);
        assert!(!state.should_quit());
        assert!(!state.has_confirm_dialog());
        assert!(!state.is_panel_visible());
        assert_eq!(state.command_buffer(), "");
        assert!(!state.word_wrap());
        assert!(state.confirm_dialog().is_none());

        let cursor = state.cursor();
        assert_eq!(cursor.row, 0);
        assert_eq!(cursor.col, 0);

        assert_eq!(state.document_view().cursor(), cursor);
        assert!(state.annotation_controller().input_box().is_none());
        let _ = state.annotation_list_panel();

        let _ = ExitResult::QuitSilent;
    }

    #[test]
    fn take_exit_result_returns_once() {
        let mut state = AppState::new_plain("[stdin]".to_string(), String::new());
        state.exit_result = Some(ExitResult::QuitSilent);

        assert!(matches!(
            state.take_exit_result(),
            Some(ExitResult::QuitSilent)
        ));
        assert!(state.take_exit_result().is_none());
    }

    #[test]
    fn test_harness_basic() {
        AppTestHarness::new("hello")
            .assert_mode(Mode::Normal)
            .assert_cursor(0, 0);
    }

    #[test]
    fn normal_can_enter_visual_mode_and_escape_back() {
        harness("first\nsecond")
            .keys("v")
            .assert_mode(Mode::Visual)
            .keys("<Esc>")
            .assert_mode(Mode::Normal);
    }

    #[test]
    fn normal_can_enter_command_mode_and_escape_back() {
        harness("first")
            .keys(":")
            .assert_mode(Mode::Command)
            .keys("<Esc>")
            .assert_mode(Mode::Normal);
    }

    #[test]
    fn tab_enters_annotation_list_mode_when_annotations_exist() {
        let mut harness = harness("first\nsecond");
        harness.keys("vld<Tab>").assert_mode(Mode::AnnotationList);
    }

    #[test]
    fn escape_leaves_annotation_list_mode() {
        let mut harness = harness("first\nsecond");
        harness
            .keys("vld<Tab><Esc>")
            .assert_mode(Mode::Normal)
            .assert_panel_visible();
    }

    #[test]
    fn insertion_enters_insert_mode_and_escape_cancels_back_to_normal() {
        harness("first")
            .keys("i")
            .assert_mode(Mode::Insert)
            .keys("<Esc>")
            .assert_mode(Mode::Normal)
            .assert_annotation_count(0);
    }

    #[test]
    fn visual_deletion_returns_to_normal_mode() {
        harness("first\nsecond")
            .keys("vld")
            .assert_mode(Mode::Normal)
            .assert_annotation_count(1);
    }

    #[test]
    fn normal_movement_keys_update_cursor_position() {
        harness("abcd\nefgh")
            .keys("ljh")
            .assert_cursor(1, 0)
            .keys("k")
            .assert_cursor(0, 0);
    }

    #[test]
    fn gg_moves_to_top_of_document() {
        harness("one\ntwo\nthree").keys("jjgg").assert_cursor(0, 0);
    }

    #[test]
    fn shift_g_moves_to_bottom_of_document() {
        harness("one\ntwo\nthree").keys("G").assert_cursor(2, 0);
    }

    #[test]
    fn zero_and_dollar_move_to_line_start_and_end() {
        harness("abcd\nefgh")
            .keys("jl0")
            .assert_cursor(1, 0)
            .keys("$")
            .assert_cursor(1, 3);
    }

    #[test]
    fn visual_deletion_creates_deletion_annotation() {
        let mut harness = harness("hello");
        harness.keys("vld");
        let annotation = harness.state().annotations().ordered()[0];

        assert_eq!(annotation.annotation_type, AnnotationType::Deletion);
        assert_eq!(annotation.selected_text, "he");
        assert_eq!(annotation.text, "");
    }

    #[test]
    fn visual_comment_opens_input_and_commits_comment() {
        let mut harness = harness("hello");
        harness.keys("vlcnote<C-s>");
        let annotation = harness.state().annotations().ordered()[0];

        assert_eq!(harness.state().mode(), Mode::Normal);
        assert_eq!(annotation.annotation_type, AnnotationType::Comment);
        assert_eq!(annotation.selected_text, "he");
        assert_eq!(annotation.text, "note");
    }

    #[test]
    fn visual_replacement_opens_input_and_commits_replacement() {
        let mut harness = harness("hello");
        harness.keys("vlrnew<C-s>");
        let annotation = harness.state().annotations().ordered()[0];

        assert_eq!(harness.state().mode(), Mode::Normal);
        assert_eq!(annotation.annotation_type, AnnotationType::Replacement);
        assert_eq!(annotation.selected_text, "he");
        assert_eq!(annotation.text, "new");
    }

    #[test]
    fn insertion_creates_annotation_at_cursor() {
        let mut harness = harness("hello\nworld");
        harness.keys("jliadd<C-s>");
        let annotation = harness.state().annotations().ordered()[0];
        let range = annotation.range.expect("insertion should have a range");

        assert_eq!(annotation.annotation_type, AnnotationType::Insertion);
        assert_eq!(annotation.text, "add");
        assert_eq!((range.start.line, range.start.column), (1, 1));
        assert_eq!((range.end.line, range.end.column), (1, 1));
    }

    #[test]
    fn global_comment_creates_unanchored_annotation() {
        let mut harness = harness("hello");
        harness.keys("gcoverall<C-s>");
        let annotation = harness.state().annotations().ordered()[0];

        assert_eq!(annotation.annotation_type, AnnotationType::GlobalComment);
        assert!(annotation.range.is_none());
        assert_eq!(annotation.text, "overall");
    }

    #[test]
    fn escape_during_input_cancels_annotation_creation() {
        harness("hello")
            .keys("vlcnote<Esc>")
            .assert_mode(Mode::Normal)
            .assert_annotation_count(0);
    }

    #[test]
    fn next_annotation_jumps_forward() {
        let mut harness = harness("alpha\nbeta\ngamma");
        create_two_deletions(&mut harness);

        harness.keys("gg]a").assert_cursor(1, 1);
    }

    #[test]
    fn prev_annotation_jumps_backward() {
        let mut harness = harness("alpha\nbeta\ngamma");
        create_two_deletions(&mut harness);

        harness.keys("G[a").assert_cursor(1, 1);
    }

    #[test]
    fn annotation_navigation_stops_at_boundaries() {
        let mut harness = harness("alpha\nbeta\ngamma");
        create_two_deletions(&mut harness);

        harness.keys("gg[a").assert_cursor(0, 0);
        harness.keys("G]a").assert_cursor(2, 0);
    }

    #[test]
    fn tab_toggles_annotation_panel_visibility() {
        harness("hello")
            .keys("vld<Tab>")
            .assert_panel_visible()
            .keys("<Tab>")
            .assert_panel_hidden();
    }

    #[test]
    fn annotation_list_navigation_updates_selection() {
        let mut harness = harness("alpha\nbeta\ngamma");
        create_two_deletions(&mut harness);

        harness.keys("<Tab>k");
        let first = harness
            .state()
            .annotation_list_panel()
            .selected_annotation_id();

        harness.keys("j");
        let second = harness
            .state()
            .annotation_list_panel()
            .selected_annotation_id();

        harness.keys("k");
        let back_to_first = harness
            .state()
            .annotation_list_panel()
            .selected_annotation_id();

        assert_ne!(first, second);
        assert_eq!(first, back_to_first);
    }

    #[test]
    fn enter_in_annotation_list_jumps_to_selected_annotation() {
        let mut harness = harness("alpha\nbeta\ngamma");
        create_two_deletions(&mut harness);

        harness.keys("gg<Tab>j<Enter>").assert_cursor(1, 1);
    }

    #[test]
    fn confirm_dialog_can_delete_annotation_from_list() {
        let mut harness = harness("alpha\nbeta");
        harness.keys("vld<Tab>kdd").assert_has_confirm_dialog();
        harness
            .keys("y")
            .assert_annotation_count(0)
            .assert_no_confirm_dialog();
    }

    #[test]
    fn confirm_dialog_cancel_keeps_annotation() {
        harness("alpha\nbeta")
            .keys("vld<Tab>kddn")
            .assert_annotation_count(1)
            .assert_no_confirm_dialog();
    }

    #[test]
    fn command_q_sets_quit_with_output() {
        let mut harness = harness("hello");
        harness.keys(":q<Enter>").assert_should_quit();

        match harness.state_mut().take_exit_result() {
            Some(ExitResult::QuitWithOutput(output)) => {
                assert_eq!(output, "No changes detected.");
            }
            _ => panic!("expected quit with output"),
        }
    }

    #[test]
    fn command_q_bang_sets_silent_quit() {
        let mut harness = harness("hello");
        harness.keys(":q!<Enter>").assert_should_quit();

        assert!(matches!(
            harness.state_mut().take_exit_result(),
            Some(ExitResult::QuitSilent)
        ));
    }

    #[test]
    fn backspace_on_empty_command_exits_command_mode() {
        harness("hello")
            .keys(":<BS>")
            .assert_mode(Mode::Normal)
            .assert_not_quit();
    }

    #[test]
    fn ctrl_c_quits_from_normal_mode() {
        harness("hello").keys("<C-c>").assert_should_quit();
    }

    #[test]
    fn ctrl_c_quits_from_visual_mode() {
        harness("hello").keys("v<C-c>").assert_should_quit();
    }

    #[test]
    fn ctrl_c_quits_from_insert_mode() {
        harness("hello").keys("i<C-c>").assert_should_quit();
    }

    #[test]
    fn ctrl_c_quits_from_command_mode() {
        harness("hello").keys(":<C-c>").assert_should_quit();
    }

    #[test]
    fn ctrl_c_quits_from_annotation_list_mode() {
        harness("hello").keys("vld<Tab><C-c>").assert_should_quit();
    }
}
