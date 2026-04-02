mod annotation_navigation;
mod commands;
mod core;
mod overlay_state;
mod panel_state;

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
        if self.handle_annotation_list_action(&action) {
            return;
        }

        let _ = self.handle_main_action(action);
    }

    fn handle_annotation_list_action(&mut self, action: &Action) -> bool {
        if self.mode != Mode::AnnotationList {
            return false;
        }

        match action {
            Action::MoveDown => {
                self.annotation_list_panel
                    .move_selection_down(&self.annotations);
                true
            }
            Action::MoveUp => {
                self.annotation_list_panel
                    .move_selection_up(&self.annotations);
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
        if !self.document_view.handle_action(action) {
            return false;
        }

        if matches!(action, Action::EnterVisualMode) {
            self.mode = Mode::Visual;
        }

        true
    }

    fn handle_non_document_action(&mut self, action: Action) -> bool {
        match action {
            Action::EnterCommandMode
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
                self.mode = Mode::Normal;
                self.document_view.clear_visual();
                self.cancel_pending_annotation();
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
        let event = match action {
            Action::CommandChar(c) => self.handle_command_char(c),
            Action::CommandBackspace => self.handle_command_backspace(),
            Action::CommandConfirm => self.handle_command_confirm(),
            _ => return false,
        };

        self.handle_command_line_event(event);
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
