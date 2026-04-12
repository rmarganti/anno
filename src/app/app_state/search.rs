use super::core::AppState;
use crate::keybinds::handler::SearchDirection;
use crate::keybinds::mode::Mode;

impl AppState {
    fn execute_search(&mut self, pattern: &str, direction: SearchDirection) -> bool {
        self.document_view.search_text(pattern, direction)
    }

    fn repeat_search(&mut self, direction: SearchDirection) {
        let Some(pattern) = self.last_search_pattern.clone() else {
            return;
        };

        self.execute_search(&pattern, direction);
    }
}

impl AppState {
    pub(super) fn clear_search_buffer(&mut self) {
        self.search_buffer.clear();
    }

    pub(super) fn handle_search_char(&mut self, c: char) {
        self.search_buffer.push(c);
    }

    pub(super) fn handle_search_backspace(&mut self) {
        self.search_buffer.pop();
        if self.search_buffer.is_empty() {
            self.mode = Mode::Normal;
        }
    }

    pub(super) fn handle_search_confirm(&mut self) {
        if !self.search_buffer.is_empty() {
            self.last_search_pattern = Some(self.search_buffer.clone());
            let pattern = self.search_buffer.clone();
            let direction = self.last_search_direction;
            self.execute_search(&pattern, direction);
        }

        self.clear_search_buffer();
        self.mode = Mode::Normal;
    }

    pub(super) fn handle_search_next(&mut self) {
        self.repeat_search(self.last_search_direction);
    }

    pub(super) fn handle_search_prev(&mut self) {
        self.repeat_search(self.last_search_direction.reversed());
    }

    pub(super) fn enter_search_mode(&mut self, direction: SearchDirection) {
        self.mode = Mode::Search;
        self.last_search_direction = direction;
        self.clear_search_buffer();
    }
}
