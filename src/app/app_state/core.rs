use crate::annotation::store::AnnotationStore;
use crate::annotation::types::{Annotation, TextRange};
use crate::app::ExitResult;
#[cfg(any(test, doctest))]
use crate::highlight::StyledSpan;
use crate::keybinds::handler::KeybindHandler;
use crate::keybinds::mode::Mode;
use crate::startup::ExportFormat;
use crate::tui::annotation_controller::AnnotationController;
use crate::tui::annotation_list_panel::AnnotationListState;
use crate::tui::command_line::CommandLine;
use crate::tui::confirm_dialog::ConfirmDialog;
use crate::tui::document_view::DocumentViewState;
use crate::tui::renderer;
use crate::tui::viewport::CursorPosition;

pub(super) const ANNOTATION_INSPECT_PAGE_SCROLL_LINES: u16 = 8;

/// Terminal-independent application state.
pub struct AppState {
    /// The source name (filename or "[stdin]").
    source_name: String,
    /// Output format used when exporting annotations on quit.
    pub(super) export_format: ExportFormat,
    /// Current input mode.
    pub(super) mode: Mode,
    /// Key event dispatcher.
    pub(super) keybinds: KeybindHandler,
    /// Annotation storage.
    pub(super) annotations: AnnotationStore,
    /// Command-line component (handles `:` command input).
    pub(super) command_line: CommandLine,
    /// Whether the app should quit.
    pub(super) should_quit: bool,
    /// The exit result to return.
    pub(super) exit_result: Option<ExitResult>,
    /// Document view state (viewport, cursor, document layout).
    pub(super) document_view: DocumentViewState,
    /// Annotation creation state machine.
    pub(super) annotation_controller: AnnotationController,
    /// Annotation list sidebar panel state.
    pub(super) annotation_list_panel: AnnotationListState,
    /// Active confirmation dialog overlay, if any.
    pub(super) confirm_dialog: Option<ConfirmDialog>,
    /// Whether the annotation inspect overlay is visible.
    pub(super) annotation_inspect_visible: bool,
    /// Scroll offset for the annotation inspect overlay content.
    pub(super) annotation_inspect_scroll_offset: u16,
    /// Whether the help overlay is visible.
    pub(super) help_visible: bool,
    /// Scroll offset for the help overlay content.
    pub(super) help_scroll_offset: u16,
    /// Whether the current terminal width can show the annotation list panel.
    pub(super) annotation_panel_available: bool,
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
        let document_view = DocumentViewState::new(doc_lines_result.plain, doc_lines_result.styled);

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
            annotation_list_panel: AnnotationListState::new(),
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

    #[cfg(test)]
    pub(crate) fn annotations_mut(&mut self) -> &mut AnnotationStore {
        &mut self.annotations
    }

    pub fn has_confirm_dialog(&self) -> bool {
        self.confirm_dialog.is_some()
    }

    pub fn is_help_visible(&self) -> bool {
        self.help_visible
    }

    #[cfg(test)]
    pub(crate) fn set_help_visible_for_test(&mut self, visible: bool) {
        self.help_visible = visible;
    }

    pub fn is_annotation_inspect_visible(&self) -> bool {
        self.annotation_inspect_visible
    }

    #[cfg_attr(not(test), allow(dead_code))]
    pub fn annotation_inspect_scroll_offset(&self) -> u16 {
        self.annotation_inspect_scroll_offset
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

    pub fn document_view(&self) -> &DocumentViewState {
        &self.document_view
    }

    pub fn document_view_mut(&mut self) -> &mut DocumentViewState {
        &mut self.document_view
    }

    pub fn annotation_controller(&self) -> &AnnotationController {
        &self.annotation_controller
    }

    #[cfg(test)]
    pub(crate) fn keybinds(&self) -> &KeybindHandler {
        &self.keybinds
    }

    pub fn confirm_dialog(&self) -> Option<&ConfirmDialog> {
        self.confirm_dialog.as_ref()
    }

    pub fn annotation_list_panel(&self) -> &AnnotationListState {
        &self.annotation_list_panel
    }

    pub fn annotation_list_panel_mut(&mut self) -> &mut AnnotationListState {
        &mut self.annotation_list_panel
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

    #[cfg(test)]
    pub(crate) fn set_mode_for_test(&mut self, mode: Mode) {
        self.mode = mode;
    }
}
