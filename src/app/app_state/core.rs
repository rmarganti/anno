use crate::annotation::store::AnnotationStore;
use crate::annotation::types::{Annotation, TextPosition, TextRange};
use crate::app::ExitResult;
#[cfg(any(test, doctest))]
use crate::highlight::StyledSpan;
use crate::keybinds::handler::KeybindHandler;
use crate::keybinds::mode::Mode;
use crate::startup::ExportFormat;
use crate::tui::annotation_list_panel::{AnnotationListState, PANEL_WIDTH};
use crate::tui::confirm_dialog::ConfirmDialog;
use crate::tui::document_view::DocumentViewState;
use crate::tui::input_box::InputBox;
use crate::tui::renderer;
use crate::tui::viewport::CursorPosition;

pub(super) const ANNOTATION_INSPECT_PAGE_SCROLL_LINES: u16 = 8;

/// Tracks the kind of annotation being created via the input box.
#[derive(Debug, Clone)]
pub(super) enum PendingAnnotation {
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
    /// Command-mode input buffer.
    pub(super) command_buffer: String,
    /// Whether the app should quit.
    pub(super) should_quit: bool,
    /// The exit result to return.
    pub(super) exit_result: Option<ExitResult>,
    /// Document view state (viewport, cursor, document layout).
    pub(super) document_view: DocumentViewState,
    /// Active input box (shown in Insert mode for annotation text entry).
    pub(super) input_box: Option<InputBox<'static>>,
    /// The pending annotation being created (set when entering Insert mode).
    pub(super) pending_annotation: Option<PendingAnnotation>,
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
    /// Dimensions available to centered overlays in the main content area.
    pub(super) overlay_area: (u16, u16),
    /// Whether the current terminal width can show the annotation list panel.
    pub(super) annotation_panel_available: bool,
    /// Number of annotation rows visible inside the panel's current layout.
    pub(super) annotation_list_visible_height: u16,
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
            command_buffer: String::new(),
            should_quit: false,
            exit_result: None,
            document_view,
            input_box: None,
            pending_annotation: None,
            annotation_list_panel: AnnotationListState::new(),
            confirm_dialog: None,
            annotation_inspect_visible: false,
            annotation_inspect_scroll_offset: 0,
            help_visible: false,
            help_scroll_offset: 0,
            overlay_area: (80, 23),
            annotation_panel_available: true,
            annotation_list_visible_height:
                crate::tui::annotation_list_panel::visible_content_height(
                    ratatui::layout::Rect::new(0, 0, PANEL_WIDTH, 23),
                ),
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

    #[cfg_attr(not(test), allow(dead_code))]
    pub fn help_scroll_offset(&self) -> u16 {
        self.help_scroll_offset
    }

    pub fn is_panel_visible(&self) -> bool {
        self.annotation_list_panel.is_visible()
    }

    pub fn is_panel_hidden_due_to_width(&self) -> bool {
        self.annotation_list_panel.is_visible() && !self.annotation_panel_available
    }

    pub fn command_buffer(&self) -> &str {
        &self.command_buffer
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

    pub fn annotation_list_visible_height(&self) -> u16 {
        self.annotation_list_visible_height
    }

    pub(crate) fn set_annotation_list_visible_height(&mut self, visible_height: u16) {
        self.annotation_list_visible_height = visible_height;
    }

    pub(crate) fn set_overlay_area(&mut self, width: u16, height: u16) {
        self.overlay_area = (width, height);
        self.clamp_help_scroll_offset();
        self.clamp_annotation_inspect_scroll_offset();
    }

    pub fn document_view_mut(&mut self) -> &mut DocumentViewState {
        &mut self.document_view
    }

    pub fn input_box(&self) -> Option<&InputBox<'static>> {
        self.input_box.as_ref()
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
