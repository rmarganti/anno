use crossterm::event::KeyEvent;
use ratatui::{Frame, layout::Rect};

use crate::annotation::export::{AgentExporter, AnnotationExporter, JsonExporter};
use crate::annotation::store::AnnotationStore;
use crate::annotation::types::{Annotation, TextRange};
use crate::app::ExitResult;
#[cfg(any(test, doctest))]
use crate::highlight::StyledSpan;
use crate::keybinds::handler::{Action, KeybindHandler};
use crate::keybinds::mode::Mode;
use crate::startup::ExportFormat;
use crate::tui::annotation_controller::{AnnotationAction, AnnotationController};
use crate::tui::annotation_list_panel::AnnotationListPanel;
use crate::tui::app_command::{AppCommand, QuitKind};
use crate::tui::command_line::{CommandLine, CommandLineEvent};
use crate::tui::confirm_dialog::{ConfirmDialog, ConfirmDialogEvent};
use crate::tui::document_view::DocumentView;
use crate::tui::renderer;
use crate::tui::theme::UiTheme;
use crate::tui::viewport::CursorPosition;

/// Terminal-independent application state.
pub struct AppState {
    /// The source name (filename or "[stdin]").
    source_name: String,
    /// Output format used when exporting annotations on quit.
    export_format: ExportFormat,
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
    /// Whether the annotation inspect overlay is visible.
    annotation_inspect_visible: bool,
    /// Scroll offset for the annotation inspect overlay content.
    annotation_inspect_scroll_offset: u16,
    /// Whether the help overlay is visible.
    help_visible: bool,
    /// Scroll offset for the help overlay content.
    help_scroll_offset: u16,
    /// Whether the current terminal width can show the annotation list panel.
    annotation_panel_available: bool,
}

impl AppState {
    pub fn new(
        source_name: String,
        doc_lines_result: renderer::DocumentLines,
        export_format: ExportFormat,
    ) -> Self {
        Self::from_document_lines(source_name, doc_lines_result, export_format)
    }

    #[cfg(any(test, doctest))]
    pub fn new_plain(source_name: String, content: String) -> Self {
        Self::new_plain_with_format(source_name, content, ExportFormat::Agent)
    }

    #[cfg(any(test, doctest))]
    pub fn new_plain_with_format(
        source_name: String,
        content: String,
        export_format: ExportFormat,
    ) -> Self {
        let plain = if content.is_empty() {
            vec![String::new()]
        } else {
            content.split('\n').map(str::to_owned).collect::<Vec<_>>()
        };
        let styled = plain
            .iter()
            .map(|line| vec![StyledSpan::plain(line.clone())])
            .collect();

        Self::from_document_lines(
            source_name,
            renderer::DocumentLines { plain, styled },
            export_format,
        )
    }

    fn from_document_lines(
        source_name: String,
        doc_lines_result: renderer::DocumentLines,
        export_format: ExportFormat,
    ) -> Self {
        let document_view = DocumentView::new(doc_lines_result.plain, doc_lines_result.styled);

        Self {
            source_name,
            export_format,
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
            annotation_inspect_visible: false,
            annotation_inspect_scroll_offset: 0,
            help_visible: false,
            help_scroll_offset: 0,
            annotation_panel_available: true,
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

    pub fn is_help_visible(&self) -> bool {
        self.help_visible
    }

    pub fn is_annotation_inspect_visible(&self) -> bool {
        self.annotation_inspect_visible
    }

    pub fn annotation_inspect_scroll_offset_mut(&mut self) -> &mut u16 {
        &mut self.annotation_inspect_scroll_offset
    }

    #[cfg_attr(not(test), allow(dead_code))]
    pub fn help_scroll_offset(&self) -> u16 {
        self.help_scroll_offset
    }

    #[allow(dead_code)]
    pub fn help_scroll_offset_mut(&mut self) -> &mut u16 {
        &mut self.help_scroll_offset
    }

    pub fn is_panel_visible(&self) -> bool {
        self.annotation_list_panel.is_visible()
    }

    pub fn is_panel_hidden_due_to_width(&self) -> bool {
        self.annotation_list_panel.is_visible() && !self.annotation_panel_available
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

    #[cfg(any(test, doctest))]
    pub fn annotation_list_panel(&self) -> &AnnotationListPanel {
        &self.annotation_list_panel
    }

    pub fn render_annotation_list_panel(
        &mut self,
        frame: &mut Frame,
        area: Rect,
        theme: &UiTheme,
        is_focused: bool,
    ) {
        self.annotation_list_panel
            .render(frame, area, &self.annotations, theme, is_focused);
    }

    pub fn selected_annotation_range(&self) -> Option<TextRange> {
        self.selected_annotation()
            .and_then(|annotation| annotation.range)
    }

    pub fn selected_annotation(&self) -> Option<&Annotation> {
        self.annotation_list_panel
            .selected_annotation_id()
            .and_then(|id| self.annotations.get(id))
    }

    pub fn set_annotation_panel_available(&mut self, available: bool) {
        self.annotation_panel_available = available;

        if !available && self.mode == Mode::AnnotationList {
            self.mode = Mode::Normal;
        }

        if !available {
            self.close_annotation_inspect();
        }
    }

    pub fn handle_key(&mut self, key_event: KeyEvent) {
        if self.help_visible {
            match self.keybinds.handle_help_overlay(self.mode, key_event) {
                Action::ToggleHelp => {
                    self.help_visible = false;
                    self.keybinds.clear_pending();
                }
                Action::MoveDown => {
                    self.help_scroll_offset = self.help_scroll_offset.saturating_add(1);
                }
                Action::MoveUp => {
                    self.help_scroll_offset = self.help_scroll_offset.saturating_sub(1);
                }
                _ => {}
            }
            return;
        }

        // If a confirm dialog is active, route all input to it.
        if let Some(dialog) = self.confirm_dialog.take() {
            match dialog.handle_key(key_event) {
                ConfirmDialogEvent::Confirm => {
                    if let Some(id) = self.annotation_list_panel.selected_annotation_id() {
                        let deleted_index = self
                            .annotations
                            .ordered()
                            .iter()
                            .position(|annotation| annotation.id == id);

                        if self.annotations.delete(id)
                            && let Some(deleted_index) = deleted_index
                        {
                            self.annotation_list_panel
                                .reconcile_after_deletion(&self.annotations, deleted_index);
                        }
                    }
                }
                ConfirmDialogEvent::Cancel => {}
                ConfirmDialogEvent::Consumed => {
                    self.confirm_dialog = Some(dialog);
                }
            }
            return;
        }

        if self.annotation_inspect_visible {
            match self.keybinds.handle_annotation_inspect(key_event) {
                Action::MoveDown => {
                    self.annotation_list_panel
                        .move_selection_down(&self.annotations);
                    self.annotation_inspect_scroll_offset = 0;
                }
                Action::MoveUp => {
                    self.annotation_list_panel
                        .move_selection_up(&self.annotations);
                    self.annotation_inspect_scroll_offset = 0;
                }
                Action::JumpToAnnotation => {
                    if let Some(annotation) = self.selected_annotation()
                        && let Some(range) = annotation.range
                    {
                        self.document_view
                            .set_cursor(range.start.line, range.start.column);
                    }
                }
                Action::ExitToNormal => {
                    self.close_annotation_inspect();
                }
                Action::ForceQuit => {
                    self.should_quit = true;
                }
                _ => {}
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
                Action::OpenAnnotationInspect => {
                    if self.selected_annotation().is_some() {
                        self.annotation_inspect_visible = true;
                        self.annotation_inspect_scroll_offset = 0;
                        self.keybinds.clear_pending();
                    }
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
                if self.annotation_list_panel.is_visible()
                    && self.annotation_panel_available
                    && !self.annotations.is_empty()
                {
                    self.mode = Mode::AnnotationList;
                } else {
                    self.mode = Mode::Normal;
                }
            }
            Action::ToggleHelp => {
                self.help_visible = !self.help_visible;
                if self.help_visible {
                    self.help_scroll_offset = 0;
                }
                self.keybinds.clear_pending();
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
                let output = match self.export_format {
                    ExportFormat::Agent => {
                        AgentExporter.export(&self.annotations, &self.source_name)
                    }
                    ExportFormat::Json => JsonExporter.export(&self.annotations, &self.source_name),
                };
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

        let current_idx = self.current_annotation_index(&ordered, cursor_pos, forward);

        let target = if forward {
            if let Some(current_idx) = current_idx {
                ordered
                    .iter()
                    .skip(current_idx + 1)
                    .find(|annotation| annotation.range.is_some())
            } else {
                ordered.iter().find(|annotation| {
                    annotation
                        .range
                        .map(|range| (range.start.line, range.start.column) > cursor_pos)
                        .unwrap_or(false)
                })
            }
        } else if let Some(current_idx) = current_idx {
            ordered[..current_idx]
                .iter()
                .rev()
                .find(|annotation| annotation.range.is_some())
        } else {
            ordered.iter().rev().find(|annotation| {
                annotation
                    .range
                    .map(|range| (range.start.line, range.start.column) < cursor_pos)
                    .unwrap_or(false)
            })
        };

        if let Some(annotation) = target
            && let Some(range) = annotation.range
        {
            self.annotation_list_panel
                .set_selected_annotation_id(annotation.id);
            self.document_view
                .set_cursor(range.start.line, range.start.column);
        }
    }

    fn current_annotation_index(
        &self,
        ordered: &[&crate::annotation::types::Annotation],
        cursor_pos: (usize, usize),
        forward: bool,
    ) -> Option<usize> {
        if let Some(selected_id) = self.annotation_list_panel.selected_annotation_id()
            && let Some(index) = ordered
                .iter()
                .position(|annotation| annotation.id == selected_id)
            && ordered[index]
                .range
                .map(|range| (range.start.line, range.start.column) == cursor_pos)
                .unwrap_or(false)
        {
            return Some(index);
        }

        let matching_indices: Vec<_> = ordered
            .iter()
            .enumerate()
            .filter_map(|(index, annotation)| {
                annotation.range.and_then(|range| {
                    ((range.start.line, range.start.column) == cursor_pos).then_some(index)
                })
            })
            .collect();

        if forward {
            matching_indices.last().copied()
        } else {
            matching_indices.first().copied()
        }
    }

    fn close_annotation_inspect(&mut self) {
        self.annotation_inspect_visible = false;
        self.annotation_inspect_scroll_offset = 0;
        self.keybinds.clear_pending();
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

        pub fn set_panel_available(&mut self, available: bool) -> &mut Self {
            self.state.set_annotation_panel_available(available);
            self
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

        pub fn assert_help_visible(&mut self) -> &mut Self {
            assert!(self.state.is_help_visible());
            self
        }

        pub fn assert_help_hidden(&mut self) -> &mut Self {
            assert!(!self.state.is_help_visible());
            self
        }

        pub fn assert_annotation_inspect_visible(&mut self) -> &mut Self {
            assert!(self.state.is_annotation_inspect_visible());
            self
        }

        pub fn assert_annotation_inspect_hidden(&mut self) -> &mut Self {
            assert!(!self.state.is_annotation_inspect_visible());
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
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    use super::test_harness::AppTestHarness;
    use super::{AppState, ExitResult};
    use crate::annotation::types::{Annotation, AnnotationType, TextPosition, TextRange};
    use crate::keybinds::mode::Mode;
    use crate::startup::ExportFormat;

    fn harness(content: &str) -> AppTestHarness {
        AppTestHarness::new(content)
    }

    fn create_two_deletions(harness: &mut AppTestHarness) {
        harness.keys("vldjvld").assert_annotation_count(2);
    }

    fn create_three_deletions(harness: &mut AppTestHarness) {
        harness.keys("vldjvldjvld").assert_annotation_count(3);
    }

    fn range(sl: usize, sc: usize, el: usize, ec: usize) -> TextRange {
        TextRange {
            start: TextPosition {
                line: sl,
                column: sc,
            },
            end: TextPosition {
                line: el,
                column: ec,
            },
        }
    }

    fn add_mixed_annotations(harness: &mut AppTestHarness) {
        let annotations = [
            Annotation::deletion(range(0, 1, 0, 4), "lph".into()),
            Annotation::insertion(TextPosition { line: 1, column: 2 }, "inserted".into()),
            Annotation::comment(range(1, 2, 1, 5), "ta ".into(), "note".into()),
            Annotation::deletion(range(2, 0, 2, 2), "ga".into()),
            Annotation::global_comment("overall".into()),
        ];

        for annotation in annotations {
            harness.state_mut().annotations.add(annotation);
        }
    }

    fn ordered_anchored_positions(state: &AppState) -> Vec<(usize, usize)> {
        state
            .annotations()
            .ordered()
            .into_iter()
            .filter_map(|annotation| {
                annotation
                    .range
                    .map(|range| (range.start.line, range.start.column))
            })
            .collect()
    }

    fn ordered_panel_positions(harness: &mut AppTestHarness, steps: usize) -> Vec<(usize, usize)> {
        harness.keys("<Tab>k");

        let mut positions = Vec::with_capacity(steps + 1);
        positions.push(
            harness
                .state()
                .selected_annotation_range()
                .map(|range| (range.start.line, range.start.column))
                .expect("panel should start on the first anchored annotation"),
        );

        for _ in 0..steps {
            harness.keys("j");
            let position = harness
                .state()
                .selected_annotation_range()
                .map(|range| (range.start.line, range.start.column));
            if let Some(position) = position {
                positions.push(position);
            }
        }

        positions
    }

    fn reverse_panel_positions(harness: &mut AppTestHarness, steps: usize) -> Vec<(usize, usize)> {
        harness.keys("<Tab>k");
        for _ in 0..steps {
            harness.keys("j");
        }

        let mut positions = Vec::with_capacity(steps + 1);
        positions.push(
            harness
                .state()
                .selected_annotation_range()
                .map(|range| (range.start.line, range.start.column))
                .expect("panel should land on an anchored annotation before reversing"),
        );

        for _ in 0..steps {
            harness.keys("k");
            positions.push(
                harness
                    .state()
                    .selected_annotation_range()
                    .map(|range| (range.start.line, range.start.column))
                    .expect("reverse panel step should stay on anchored annotations"),
            );
        }

        positions
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
        assert!(!state.is_annotation_inspect_visible());
        assert!(!state.is_help_visible());
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
    fn tab_keeps_panel_hidden_from_annotation_list_mode_when_terminal_is_narrow() {
        let mut harness = harness("first\nsecond");
        harness
            .set_panel_available(false)
            .keys("vld<Tab>")
            .assert_mode(Mode::Normal)
            .assert_panel_visible();

        assert!(harness.state().is_panel_hidden_due_to_width());
    }

    #[test]
    fn narrow_terminal_forces_annotation_list_mode_back_to_normal() {
        let mut harness = harness("first\nsecond");
        harness.keys("vld<Tab>").assert_mode(Mode::AnnotationList);

        harness.set_panel_available(false).assert_mode(Mode::Normal);
        assert!(harness.state().is_panel_hidden_due_to_width());
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
    fn space_opens_annotation_inspect_from_annotation_list() {
        let mut harness = harness("alpha\nbeta");

        harness
            .keys("vldjvld<Tab>k ")
            .assert_mode(Mode::AnnotationList)
            .assert_annotation_inspect_visible();
    }

    #[test]
    fn escape_dismisses_annotation_inspect_back_to_list_mode() {
        let mut harness = harness("alpha\nbeta");

        harness
            .keys("vld<Tab>k ")
            .assert_annotation_inspect_visible()
            .keys("<Esc>")
            .assert_annotation_inspect_hidden()
            .assert_mode(Mode::AnnotationList)
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
    fn next_annotation_matches_panel_order_for_mixed_annotations() {
        let mut panel_harness = harness("alpha\nbeta\ngamma");
        add_mixed_annotations(&mut panel_harness);
        let expected = ordered_panel_positions(&mut panel_harness, 4);

        let mut navigation_harness = harness("alpha\nbeta\ngamma");
        add_mixed_annotations(&mut navigation_harness);

        let anchored = ordered_anchored_positions(navigation_harness.state());
        let mut visited = Vec::with_capacity(anchored.len());
        for _ in 0..anchored.len() {
            navigation_harness.keys("]a");
            let cursor = navigation_harness.state().cursor();
            visited.push((cursor.row, cursor.col));
        }

        assert_eq!(visited, expected);
        assert_eq!(visited, anchored);
    }

    #[test]
    fn prev_annotation_matches_panel_order_for_mixed_annotations() {
        let mut panel_harness = harness("alpha\nbeta\ngamma");
        add_mixed_annotations(&mut panel_harness);
        let anchored = ordered_anchored_positions(panel_harness.state());
        let expected = reverse_panel_positions(&mut panel_harness, anchored.len() - 1);

        let mut navigation_harness = harness("alpha\nbeta\ngamma");
        add_mixed_annotations(&mut navigation_harness);
        navigation_harness.keys("G$");

        let mut visited = Vec::with_capacity(anchored.len());
        for _ in 0..anchored.len() {
            navigation_harness.keys("[a");
            let cursor = navigation_harness.state().cursor();
            visited.push((cursor.row, cursor.col));
        }

        assert_eq!(visited, expected);
        assert_eq!(visited, anchored.into_iter().rev().collect::<Vec<_>>());
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
    fn question_mark_toggles_help_on_from_normal_mode() {
        harness("hello").keys("?").assert_help_visible();
    }

    #[test]
    fn configured_help_shortcut_toggles_help_off_when_already_visible() {
        harness("hello")
            .keys("??")
            .assert_help_hidden()
            .assert_mode(Mode::Normal);
    }

    #[test]
    fn escape_dismisses_help() {
        harness("hello")
            .keys("?<Esc>")
            .assert_help_hidden()
            .assert_mode(Mode::Normal);
    }

    #[test]
    fn q_dismisses_help() {
        harness("hello")
            .keys("?q")
            .assert_help_hidden()
            .assert_mode(Mode::Normal);
    }

    #[test]
    fn other_keys_are_consumed_while_help_is_visible() {
        let mut harness = harness("hello");
        harness
            .keys("?j")
            .assert_help_visible()
            .assert_cursor(0, 0)
            .assert_mode(Mode::Normal)
            .assert_not_quit();
        assert_eq!(harness.state().help_scroll_offset(), 1);
    }

    #[test]
    fn help_dismissal_preserves_the_active_mode() {
        let mut harness = harness("hello");
        harness.state_mut().mode = Mode::Visual;
        harness.state_mut().help_visible = true;

        harness
            .keys("q")
            .assert_help_hidden()
            .assert_mode(Mode::Visual);
    }

    #[test]
    fn help_toggle_clears_pending_key_sequence() {
        let mut harness = harness("alpha\nbeta\ngamma");

        harness.keys("jg").assert_cursor(1, 0);
        assert!(harness.state().keybinds.has_pending());

        harness.keys("?").assert_help_visible();
        assert!(!harness.state().keybinds.has_pending());

        harness.keys("q").assert_help_hidden();
        assert!(!harness.state().keybinds.has_pending());

        harness.keys("g").assert_cursor(1, 0);
        assert!(harness.state().keybinds.has_pending());
    }

    #[test]
    fn opening_annotation_inspect_clears_pending_delete_sequence() {
        let mut harness = harness("alpha\nbeta");

        harness.keys("vld<Tab>kd");
        assert!(harness.state().keybinds.has_pending());

        harness
            .keys(" ")
            .assert_annotation_inspect_visible()
            .assert_mode(Mode::AnnotationList);
        assert!(!harness.state().keybinds.has_pending());

        harness
            .keys("<Esc>")
            .assert_annotation_inspect_hidden()
            .assert_mode(Mode::AnnotationList);
        assert!(!harness.state().keybinds.has_pending());
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
    fn selected_annotation_range_returns_selected_panel_annotation_range() {
        let mut harness = harness("alpha\nbeta\ngamma");
        create_two_deletions(&mut harness);

        harness.keys("<Tab>jj");

        let expected_range = harness.state().annotations().ordered()[1]
            .range
            .expect("selected panel annotation should have a range");

        let selected_range = harness
            .state()
            .selected_annotation_range()
            .expect("selected panel annotation should have a range");

        assert_eq!(selected_range, expected_range);
    }

    #[test]
    fn enter_in_annotation_list_jumps_to_selected_annotation() {
        let mut harness = harness("alpha\nbeta\ngamma");
        create_two_deletions(&mut harness);

        harness.keys("gg<Tab>jj<Enter>").assert_cursor(1, 1);
    }

    #[test]
    fn annotation_inspect_j_k_cycle_annotations_without_closing() {
        let mut harness = harness("alpha\nbeta\ngamma");
        create_two_deletions(&mut harness);

        harness.keys("<Tab>k ");
        let first = harness
            .state()
            .annotation_list_panel()
            .selected_annotation_id();

        harness.keys("j").assert_annotation_inspect_visible();
        let second = harness
            .state()
            .annotation_list_panel()
            .selected_annotation_id();

        harness.keys("k").assert_annotation_inspect_visible();
        let back_to_first = harness
            .state()
            .annotation_list_panel()
            .selected_annotation_id();

        assert_ne!(first, second);
        assert_eq!(first, back_to_first);
    }

    #[test]
    fn enter_in_annotation_inspect_jumps_to_selected_annotation() {
        let mut harness = harness("alpha\nbeta\ngamma");
        create_two_deletions(&mut harness);

        harness.keys("<Tab>kj ");
        let expected = harness
            .state()
            .selected_annotation_range()
            .expect("inspect selection should be anchored")
            .start;

        harness
            .keys("<Enter>")
            .assert_annotation_inspect_visible()
            .assert_cursor(expected.line, expected.column);
    }

    #[test]
    fn enter_in_annotation_inspect_is_noop_for_global_comments() {
        let mut harness = harness("alpha\nbeta\ngamma");
        add_mixed_annotations(&mut harness);

        harness
            .keys("<Tab>kjjjj ")
            .assert_annotation_inspect_visible();
        assert!(harness.state().selected_annotation_range().is_none());

        let cursor = harness.state().cursor();
        harness
            .keys("<Enter>")
            .assert_annotation_inspect_visible()
            .assert_cursor(cursor.row, cursor.col);
    }

    #[test]
    fn dd_is_disabled_while_annotation_inspect_is_open() {
        let mut harness = harness("alpha\nbeta");

        harness
            .keys("vld<Tab>k dd")
            .assert_annotation_count(1)
            .assert_no_confirm_dialog()
            .assert_annotation_inspect_visible();
    }

    #[test]
    fn narrow_terminal_closes_annotation_inspect_and_returns_to_normal() {
        let mut harness = harness("alpha\nbeta");

        harness
            .keys("vld<Tab>k ")
            .assert_mode(Mode::AnnotationList)
            .assert_annotation_inspect_visible();

        harness
            .set_panel_available(false)
            .assert_mode(Mode::Normal)
            .assert_annotation_inspect_hidden();
    }

    #[test]
    fn narrow_terminal_does_not_offer_annotation_inspect_entry() {
        let mut harness = harness("alpha\nbeta");

        harness
            .set_panel_available(false)
            .keys("vld<Tab> ")
            .assert_mode(Mode::Normal)
            .assert_annotation_inspect_hidden();

        assert!(harness.state().is_panel_hidden_due_to_width());
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
    fn confirm_dialog_delete_keeps_selection_on_same_list_index() {
        let mut harness = harness("alpha\nbeta\ngamma\ndelta");
        create_three_deletions(&mut harness);

        let ordered = harness.state().annotations().ordered();
        let expected_id = ordered[2].id;
        drop(ordered);

        harness
            .keys("<Tab>kjdd")
            .assert_has_confirm_dialog()
            .keys("y")
            .assert_annotation_count(2)
            .assert_no_confirm_dialog();

        assert_eq!(
            harness
                .state()
                .annotation_list_panel()
                .selected_annotation_id(),
            Some(expected_id)
        );
    }

    #[test]
    fn command_q_sets_quit_with_output() {
        let mut harness = harness("hello");
        harness.keys(":q<Enter>").assert_should_quit();

        match harness.state_mut().take_exit_result() {
            Some(ExitResult::QuitWithOutput(output)) => {
                assert_eq!(output, "No annotations.");
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
    fn command_q_uses_json_export_when_configured() {
        let mut state = AppState::new_plain_with_format(
            "demo.md".to_string(),
            "hello".to_string(),
            ExportFormat::Json,
        );

        state.handle_key(KeyEvent::new(KeyCode::Char(':'), KeyModifiers::NONE));
        state.handle_key(KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE));
        state.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

        match state.take_exit_result() {
            Some(ExitResult::QuitWithOutput(output)) => {
                assert_eq!(
                    output,
                    "{\"source\":\"demo.md\",\"total\":0,\"annotations\":[]}"
                );
            }
            _ => panic!("expected quit with output"),
        }
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

    #[test]
    fn j_k_adjust_help_scroll_offset() {
        let mut harness = harness("hello");
        harness.keys("?jjj");
        assert_eq!(harness.state().help_scroll_offset(), 3);

        harness.keys("k");
        assert_eq!(harness.state().help_scroll_offset(), 2);
    }

    #[test]
    fn help_scroll_offset_saturates_at_zero() {
        let mut harness = harness("hello");
        harness.keys("?kk");
        assert_eq!(harness.state().help_scroll_offset(), 0);
    }

    #[test]
    fn help_scroll_offset_resets_on_reopen() {
        let mut harness = harness("hello");
        harness.keys("?jjj");
        assert_eq!(harness.state().help_scroll_offset(), 3);

        harness.keys("?");
        harness.assert_help_hidden();

        harness.keys("?");
        harness.assert_help_visible();
        assert_eq!(harness.state().help_scroll_offset(), 0);
    }

    #[test]
    fn dismiss_keys_work_regardless_of_scroll_position() {
        harness("hello")
            .keys("?jjj<Esc>")
            .assert_help_hidden()
            .assert_mode(Mode::Normal);

        harness("hello")
            .keys("?jjjq")
            .assert_help_hidden()
            .assert_mode(Mode::Normal);

        harness("hello")
            .keys("?jjj?")
            .assert_help_hidden()
            .assert_mode(Mode::Normal);
    }
}
