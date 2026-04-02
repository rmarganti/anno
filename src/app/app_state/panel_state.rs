use super::core::AppState;
use crate::keybinds::mode::Mode;

impl AppState {
    pub(super) fn initialize_annotation_list_selection(&mut self) {
        self.annotation_list_panel
            .ensure_selection_initialized(&self.annotations, self.annotation_list_visible_height());
    }

    pub(super) fn hide_annotation_list_panel(&mut self) {
        self.annotation_list_panel.toggle();
        self.close_annotation_inspect();
        self.mode = Mode::Normal;
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
}
