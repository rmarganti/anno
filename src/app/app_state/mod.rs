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
use crate::keybinds::handler::Action;
use crate::keybinds::mode::Mode;
use crate::tui::annotation_controller::AnnotationAction;

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
                self.command_line.clear();
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
                self.annotation_controller.cancel();
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
            Action::CommandChar(c) => self.command_line.handle_char(c),
            Action::CommandBackspace => self.command_line.handle_backspace(),
            Action::CommandConfirm => self.command_line.handle_confirm(),
            _ => return false,
        };

        self.handle_command_line_event(event);
        true
    }

    fn handle_visual_annotation_action(&mut self, action: Action) -> bool {
        let annotation_action = match action {
            Action::CreateDeletion => self
                .annotation_controller
                .create_deletion(&mut self.document_view, &mut self.annotations),
            Action::CreateComment => self
                .annotation_controller
                .start_input_for_visual_annotation("Comment", &mut self.document_view),
            Action::CreateReplacement => self
                .annotation_controller
                .start_input_for_visual_annotation("Replacement", &mut self.document_view),
            _ => return false,
        };

        self.apply_annotation_action(annotation_action);
        true
    }

    fn handle_normal_annotation_action(&mut self, action: Action) -> bool {
        let annotation_action = match action {
            Action::CreateInsertion => self
                .annotation_controller
                .start_insertion(&self.document_view),
            Action::CreateGlobalComment => self.annotation_controller.start_global_comment(),
            _ => return false,
        };

        self.apply_annotation_action(annotation_action);
        true
    }

    fn handle_input_mode_action(&mut self, action: Action) -> bool {
        let Action::InputForward(key_event) = action else {
            return false;
        };

        let annotation_action = self
            .annotation_controller
            .handle_input_key(key_event, &mut self.annotations);
        self.apply_annotation_action(annotation_action);
        true
    }

    fn apply_annotation_action(&mut self, action: AnnotationAction) {
        if let AnnotationAction::SwitchMode(mode) = action {
            self.mode = mode;
        }
    }
}

#[cfg(test)]
mod tests;
