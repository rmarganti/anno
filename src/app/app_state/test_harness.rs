#![allow(dead_code)]

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseEvent, MouseEventKind};
use ratatui::layout::Rect;

use super::AppState;
use crate::keybinds::mode::Mode;
use crate::tui::annotation_list_panel::{PANEL_WIDTH, visible_content_height};

pub(crate) struct AppTestHarness {
    state: AppState,
}

impl AppTestHarness {
    pub(crate) fn new(content: &str) -> Self {
        let mut state = AppState::new_plain("[test]".to_string(), content.to_string());
        state.set_overlay_area(80, 23);
        state.set_annotation_list_visible_height(visible_content_height(Rect::new(
            0,
            0,
            PANEL_WIDTH,
            23,
        )));
        state.document_view_mut().update_dimensions(80, 24);

        Self { state }
    }

    pub(crate) fn key(&mut self, code: KeyCode) -> &mut Self {
        self.key_mod(code, KeyModifiers::NONE)
    }

    pub(crate) fn key_mod(&mut self, code: KeyCode, modifiers: KeyModifiers) -> &mut Self {
        self.state.handle_key(KeyEvent::new(code, modifiers));
        self
    }

    pub(crate) fn keys(&mut self, sequence: &str) -> &mut Self {
        for key_event in parse_key_sequence(sequence) {
            self.state.handle_key(key_event);
        }
        self
    }

    pub(crate) fn mouse(&mut self, kind: MouseEventKind) -> bool {
        self.state.handle_mouse(MouseEvent {
            kind,
            column: 0,
            row: 0,
            modifiers: KeyModifiers::NONE,
        })
    }

    pub(crate) fn mouse_scroll_up(&mut self) -> bool {
        self.mouse(MouseEventKind::ScrollUp)
    }

    pub(crate) fn mouse_scroll_down(&mut self) -> bool {
        self.mouse(MouseEventKind::ScrollDown)
    }

    pub(crate) fn state(&self) -> &AppState {
        &self.state
    }

    pub(crate) fn state_mut(&mut self) -> &mut AppState {
        &mut self.state
    }

    pub(crate) fn set_panel_available(&mut self, available: bool) -> &mut Self {
        self.state.set_annotation_panel_available(available);
        self
    }

    pub(crate) fn assert_mode(&mut self, expected: Mode) -> &mut Self {
        assert_eq!(self.state.mode(), expected);
        self
    }

    pub(crate) fn assert_annotation_count(&mut self, expected: usize) -> &mut Self {
        assert_eq!(self.state.annotation_count(), expected);
        self
    }

    pub(crate) fn assert_cursor(&mut self, row: usize, col: usize) -> &mut Self {
        let cursor = self.state.cursor();
        assert_eq!(cursor.row, row);
        assert_eq!(cursor.col, col);
        self
    }

    pub(crate) fn assert_should_quit(&mut self) -> &mut Self {
        assert!(self.state.should_quit());
        self
    }

    pub(crate) fn assert_not_quit(&mut self) -> &mut Self {
        assert!(!self.state.should_quit());
        self
    }

    pub(crate) fn assert_has_confirm_dialog(&mut self) -> &mut Self {
        assert!(self.state.has_confirm_dialog());
        self
    }

    pub(crate) fn assert_no_confirm_dialog(&mut self) -> &mut Self {
        assert!(!self.state.has_confirm_dialog());
        self
    }

    pub(crate) fn assert_panel_visible(&mut self) -> &mut Self {
        assert!(self.state.is_panel_visible());
        self
    }

    pub(crate) fn assert_panel_hidden(&mut self) -> &mut Self {
        assert!(!self.state.is_panel_visible());
        self
    }

    pub(crate) fn assert_help_visible(&mut self) -> &mut Self {
        assert!(self.state.is_help_visible());
        self
    }

    pub(crate) fn assert_help_hidden(&mut self) -> &mut Self {
        assert!(!self.state.is_help_visible());
        self
    }

    pub(crate) fn assert_annotation_inspect_visible(&mut self) -> &mut Self {
        assert!(self.state.is_annotation_inspect_visible());
        self
    }

    pub(crate) fn assert_annotation_inspect_hidden(&mut self) -> &mut Self {
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
