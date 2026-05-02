mod annotation_navigation;
mod commands;
mod core;
mod overlay_state;
mod panel_state;
mod search;

#[cfg(test)]
mod test_harness;
#[cfg(test)]
mod test_support;

use crossterm::event::KeyEvent;

#[cfg(test)]
use self::core::ANNOTATION_INSPECT_PAGE_SCROLL_LINES;
pub(super) use self::core::AppState;
use self::core::PendingAnnotation;
use crate::annotation::types::{Annotation, TextPosition};
use crate::keybinds::handler::Action;
use crate::keybinds::mode::Mode;
use crate::tui::document_view::VisualKind;
use crate::tui::input_box::InputBoxEvent;

impl AppState {
    pub fn handle_key(&mut self, key_event: KeyEvent) {
        if self.handle_overlay_key(key_event) {
            return;
        }

        let action = self.keybinds.handle(self.mode, key_event);
        self.dispatch_action(action);
    }

    fn dispatch_action(&mut self, action: Action) {
        if let Action::Repeat { action, count } = action {
            self.dispatch_repeat(*action, count);
            return;
        }

        if self.handle_annotation_list_action(&action) {
            return;
        }

        let _ = self.handle_main_action(action);
    }

    fn dispatch_repeat(&mut self, action: Action, count: usize) {
        if matches!(action, Action::EnterVisualLineMode) {
            if count == 0 {
                return;
            }

            self.dispatch_action(Action::EnterVisualLineMode);

            if count > 1 {
                self.dispatch_repeat(Action::MoveDown, count - 1);
            }

            return;
        }

        if self.is_repeatable_navigation_action(&action) {
            for _ in 0..count {
                self.dispatch_action(action.clone());
            }
        }
    }

    // Determines whether an `Action::Repeat` should be executed N times given
    // the current mode.
    //
    // Note: `ScrollOverlayUp/Down/PageUp/PageDown` are intentionally absent
    // here. Although they appear in `Action::supports_count` (so a count prefix
    // is syntactically valid), counted overlay scrolls are handled upstream in
    // `handle_counted_overlay_navigation` before `dispatch_repeat` is ever
    // reached. Adding them here would be a no-op at best and confusing at worst.
    // `SearchNext/SearchPrev` are handled here because their counted repeats are
    // issued from Normal/Visual mode before dispatching into the search actions.
    fn is_repeatable_navigation_action(&self, action: &Action) -> bool {
        match self.mode {
            Mode::Normal | Mode::Visual | Mode::VisualLine => matches!(
                action,
                Action::MoveUp
                    | Action::MoveDown
                    | Action::MoveScreenUp
                    | Action::MoveScreenDown
                    | Action::MoveLeft
                    | Action::MoveRight
                    | Action::MoveWordForward
                    | Action::MoveWordBackward
                    | Action::MoveWordEnd
                    | Action::MoveLineStart
                    | Action::MoveLineEnd
                    | Action::MoveDocumentTop
                    | Action::MoveDocumentBottom
                    | Action::HalfPageDown
                    | Action::HalfPageUp
                    | Action::FullPageDown
                    | Action::FullPageUp
                    | Action::NextAnnotation
                    | Action::PrevAnnotation
                    | Action::SearchNext
                    | Action::SearchPrev
            ),
            Mode::AnnotationList => {
                matches!(
                    action,
                    Action::MoveUp | Action::MoveDown | Action::JumpToAnnotation
                )
            }
            Mode::Insert | Mode::Command | Mode::Search => false,
        }
    }

    fn handle_annotation_list_action(&mut self, action: &Action) -> bool {
        if self.mode != Mode::AnnotationList {
            return false;
        }

        match action {
            Action::MoveDown => {
                self.annotation_list_panel
                    .move_selection_down(&self.annotations, self.annotation_list_visible_height());
                true
            }
            Action::MoveUp => {
                self.annotation_list_panel
                    .move_selection_up(&self.annotations, self.annotation_list_visible_height());
                true
            }
            Action::JumpToAnnotation => {
                if let Some(id) = self.annotation_list_panel.selected_annotation_id()
                    && let Some(annotation) = self.annotations.get(id)
                    && let Some(range) = annotation.range
                {
                    self.document_view
                        .set_cursor(range.start.line, range.start.column);
                }
                true
            }
            Action::DeleteAnnotation => {
                self.open_delete_confirmation();
                true
            }
            Action::OpenAnnotationInspect => {
                self.open_annotation_inspect();
                true
            }
            Action::ExitToNormal => {
                self.mode = Mode::Normal;
                true
            }
            _ => false,
        }
    }

    fn handle_main_action(&mut self, action: Action) -> bool {
        if self.handle_document_view_action(&action) {
            return true;
        }

        self.handle_non_document_action(action)
    }

    fn handle_document_view_action(&mut self, action: &Action) -> bool {
        match action {
            Action::EnterVisualMode => {
                self.document_view.set_visual_kind(VisualKind::Char);
                self.mode = Mode::Visual;
                true
            }
            Action::EnterVisualLineMode => {
                self.document_view.set_visual_kind(VisualKind::Line);
                self.mode = Mode::VisualLine;
                true
            }
            _ => {
                if !self.document_view.handle_action(action) {
                    return false;
                }
                true
            }
        }
    }

    fn handle_non_document_action(&mut self, action: Action) -> bool {
        match action {
            Action::EnterCommandMode
            | Action::EnterSearchMode { .. }
            | Action::EnterAnnotationListMode
            | Action::HideAnnotationList
            | Action::ToggleHelp
            | Action::ExitToNormal => self.handle_mode_transition_action(action),
            Action::NextAnnotation | Action::PrevAnnotation => {
                self.handle_annotation_navigation_action(action)
            }
            Action::CommandChar(_) | Action::CommandBackspace | Action::CommandConfirm => {
                self.handle_command_mode_action(action)
            }
            Action::SearchChar(_)
            | Action::SearchBackspace
            | Action::SearchConfirm
            | Action::SearchNext
            | Action::SearchPrev => self.handle_search_mode_action(action),
            Action::CreateDeletion | Action::CreateComment | Action::CreateReplacement => {
                self.handle_visual_annotation_action(action)
            }
            Action::CreateInsertion | Action::CreateGlobalComment => {
                self.handle_normal_annotation_action(action)
            }
            Action::InputForward(_) => self.handle_input_mode_action(action),
            Action::ForceQuit => {
                self.should_quit = true;
                true
            }
            _ => false,
        }
    }

    fn handle_mode_transition_action(&mut self, action: Action) -> bool {
        match action {
            Action::EnterCommandMode => {
                self.mode = Mode::Command;
                self.clear_command_buffer();
            }
            Action::EnterSearchMode { direction } => {
                self.enter_search_mode(direction);
            }
            Action::EnterAnnotationListMode => {
                if self.annotation_list_panel.is_visible() {
                    if self.annotation_panel_available {
                        self.mode = if self.mode == Mode::AnnotationList {
                            Mode::Normal
                        } else {
                            self.initialize_annotation_list_selection();
                            Mode::AnnotationList
                        };
                    }
                } else {
                    self.annotation_list_panel.toggle();
                    if self.annotation_panel_available {
                        self.initialize_annotation_list_selection();
                        self.mode = Mode::AnnotationList;
                    } else {
                        self.mode = Mode::Normal;
                    }
                }
            }
            Action::HideAnnotationList => {
                if self.annotation_list_panel.is_visible() {
                    self.hide_annotation_list_panel();
                }
            }
            Action::ToggleHelp => {
                self.toggle_help_overlay();
            }
            Action::ExitToNormal => {
                if self.mode == Mode::Search {
                    self.exit_search_mode();
                } else {
                    self.mode = Mode::Normal;
                    self.document_view.clear_visual();
                    self.cancel_pending_annotation();
                }
            }
            _ => return false,
        }

        true
    }

    fn handle_annotation_navigation_action(&mut self, action: Action) -> bool {
        match action {
            Action::NextAnnotation => self.jump_to_adjacent_annotation(true),
            Action::PrevAnnotation => self.jump_to_adjacent_annotation(false),
            _ => return false,
        }

        true
    }

    fn handle_command_mode_action(&mut self, action: Action) -> bool {
        match action {
            Action::CommandChar(c) => self.handle_command_char(c),
            Action::CommandBackspace => self.handle_command_backspace(),
            Action::CommandConfirm => self.handle_command_confirm(),
            _ => return false,
        }

        true
    }

    fn handle_search_mode_action(&mut self, action: Action) -> bool {
        match action {
            Action::SearchChar(c) => self.handle_search_char(c),
            Action::SearchBackspace => self.handle_search_backspace(),
            Action::SearchConfirm => self.handle_search_confirm(),
            Action::SearchNext => self.handle_search_next(),
            Action::SearchPrev => self.handle_search_prev(),
            _ => return false,
        }

        true
    }

    fn handle_visual_annotation_action(&mut self, action: Action) -> bool {
        match action {
            Action::CreateDeletion => self.create_deletion_annotation(),
            Action::CreateComment => self.start_visual_annotation_input("Comment", true),
            Action::CreateReplacement => self.start_visual_annotation_input("Replacement", false),
            _ => return false,
        }

        true
    }

    fn handle_normal_annotation_action(&mut self, action: Action) -> bool {
        match action {
            Action::CreateInsertion => self.start_insertion_annotation(),
            Action::CreateGlobalComment => self.start_global_comment_annotation(),
            _ => return false,
        }

        true
    }

    fn handle_input_mode_action(&mut self, action: Action) -> bool {
        let Action::InputForward(key_event) = action else {
            return false;
        };

        self.handle_annotation_input_key(key_event);
        true
    }

    fn cancel_pending_annotation(&mut self) {
        self.input_box = None;
        self.pending_annotation = None;
    }

    fn create_deletion_annotation(&mut self) {
        if let Some((range, text)) = self.document_view.take_visual_selection() {
            self.annotations.add(Annotation::deletion(range, text));
        }

        self.mode = Mode::Normal;
    }

    fn start_visual_annotation_input(&mut self, prompt: &str, is_comment: bool) {
        if let Some((range, selected_text)) = self.document_view.take_visual_selection() {
            let pending = if is_comment {
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
            self.input_box = Some(crate::tui::input_box::InputBox::new(prompt));
            self.mode = Mode::Insert;
        } else {
            self.mode = Mode::Normal;
        }
    }

    fn start_insertion_annotation(&mut self) {
        let cursor = self.document_view.cursor();
        let position = TextPosition {
            line: cursor.row,
            column: cursor.col,
        };

        self.pending_annotation = Some(PendingAnnotation::Insertion { position });
        self.input_box = Some(crate::tui::input_box::InputBox::new("Insertion"));
        self.mode = Mode::Insert;
    }

    fn start_global_comment_annotation(&mut self) {
        self.pending_annotation = Some(PendingAnnotation::GlobalComment);
        self.input_box = Some(crate::tui::input_box::InputBox::new("Global Comment"));
        self.mode = Mode::Insert;
    }

    fn handle_annotation_input_key(&mut self, key_event: KeyEvent) {
        if let Some(ref mut input_box) = self.input_box {
            match input_box.handle_key(key_event) {
                InputBoxEvent::Confirm => {
                    self.confirm_pending_annotation();
                    self.mode = Mode::Normal;
                }
                InputBoxEvent::Cancel => {
                    self.cancel_pending_annotation();
                    self.mode = Mode::Normal;
                }
                InputBoxEvent::Consumed => {}
            }
        }
    }

    fn confirm_pending_annotation(&mut self) {
        let text = self
            .input_box
            .as_ref()
            .map(|input_box| input_box.text())
            .unwrap_or_default();

        if let Some(pending) = self.pending_annotation.take()
            && !text.is_empty()
        {
            let annotation = match pending {
                PendingAnnotation::Comment {
                    range,
                    selected_text,
                } => Annotation::comment(range, selected_text, text),
                PendingAnnotation::Replacement {
                    range,
                    selected_text,
                } => Annotation::replacement(range, selected_text, text),
                PendingAnnotation::Insertion { position } => Annotation::insertion(position, text),
                PendingAnnotation::GlobalComment => Annotation::global_comment(text),
            };
            self.annotations.add(annotation);
        }

        self.input_box = None;
    }
}

#[cfg(test)]
mod tests;
