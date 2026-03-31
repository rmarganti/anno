use crossterm::event::KeyEvent;

use super::core::{ANNOTATION_INSPECT_PAGE_SCROLL_LINES, AppState};
use crate::keybinds::handler::Action;
use crate::tui::confirm_dialog::{ConfirmDialog, ConfirmDialogEvent};

impl AppState {
    pub(super) fn handle_overlay_key(&mut self, key_event: KeyEvent) -> bool {
        if self.help_visible {
            self.handle_help_overlay_key(key_event);
            return true;
        }

        if self.handle_confirm_dialog_key(key_event) {
            return true;
        }

        if self.annotation_inspect_visible {
            self.handle_annotation_inspect_key(key_event);
            return true;
        }

        false
    }

    pub(super) fn open_delete_confirmation(&mut self) {
        self.confirm_dialog = Some(ConfirmDialog::new("Delete annotation? (y/n)"));
    }

    pub(super) fn open_annotation_inspect(&mut self) {
        if self.selected_annotation().is_none() {
            return;
        }

        self.annotation_inspect_visible = true;
        self.annotation_inspect_scroll_offset = 0;
        self.keybinds.clear_pending();
    }

    pub(super) fn close_annotation_inspect(&mut self) {
        self.annotation_inspect_visible = false;
        self.annotation_inspect_scroll_offset = 0;
        self.keybinds.clear_pending();
    }

    pub(super) fn toggle_help_overlay(&mut self) {
        self.help_visible = !self.help_visible;
        if self.help_visible {
            self.help_scroll_offset = 0;
        }
        self.keybinds.clear_pending();
    }

    fn handle_help_overlay_key(&mut self, key_event: KeyEvent) {
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
    }

    fn handle_confirm_dialog_key(&mut self, key_event: KeyEvent) -> bool {
        let Some(dialog) = self.confirm_dialog.take() else {
            return false;
        };

        match dialog.handle_key(key_event) {
            ConfirmDialogEvent::Confirm => self.confirm_selected_annotation_deletion(),
            ConfirmDialogEvent::Cancel => {}
            ConfirmDialogEvent::Consumed => {
                self.confirm_dialog = Some(dialog);
            }
        }

        true
    }

    fn confirm_selected_annotation_deletion(&mut self) {
        let Some(id) = self.annotation_list_panel.selected_annotation_id() else {
            return;
        };

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

    fn handle_annotation_inspect_key(&mut self, key_event: KeyEvent) {
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
            Action::ScrollOverlayDown => self.scroll_annotation_inspect_down(1),
            Action::ScrollOverlayUp => self.scroll_annotation_inspect_up(1),
            Action::ScrollOverlayPageDown => {
                self.scroll_annotation_inspect_down(ANNOTATION_INSPECT_PAGE_SCROLL_LINES);
            }
            Action::ScrollOverlayPageUp => {
                self.scroll_annotation_inspect_up(ANNOTATION_INSPECT_PAGE_SCROLL_LINES);
            }
            Action::JumpToAnnotation => self.jump_to_selected_annotation(),
            Action::ExitToNormal => self.close_annotation_inspect(),
            Action::ForceQuit => {
                self.should_quit = true;
            }
            _ => {}
        }
    }

    fn jump_to_selected_annotation(&mut self) {
        if let Some(annotation) = self.selected_annotation()
            && let Some(range) = annotation.range
        {
            self.document_view
                .set_cursor(range.start.line, range.start.column);
        }
    }

    fn scroll_annotation_inspect_down(&mut self, lines: u16) {
        self.annotation_inspect_scroll_offset =
            self.annotation_inspect_scroll_offset.saturating_add(lines);
    }

    fn scroll_annotation_inspect_up(&mut self, lines: u16) {
        self.annotation_inspect_scroll_offset =
            self.annotation_inspect_scroll_offset.saturating_sub(lines);
    }
}
