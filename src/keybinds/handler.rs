use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::mode::Mode;

/// Actions that can be dispatched from key events.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    // -- Movement --
    MoveUp,
    MoveDown,
    MoveLeft,
    MoveRight,
    MoveWordForward,
    MoveWordBackward,
    MoveWordEnd,
    MoveLineStart,
    MoveLineEnd,
    MoveDocumentTop,
    MoveDocumentBottom,
    HalfPageDown,
    HalfPageUp,
    FullPageDown,
    FullPageUp,

    // -- Mode transitions --
    EnterVisualMode,
    EnterInsertMode,
    EnterCommandMode,
    EnterAnnotationListMode,
    ExitToNormal,

    // -- Annotation creation (Visual mode) --
    CreateDeletion,
    CreateComment,
    CreateReplacement,

    // -- Annotation creation (Normal mode) --
    CreateInsertion,
    CreateGlobalComment,

    // -- Annotation navigation --
    NextAnnotation,
    PrevAnnotation,

    // -- Annotation list actions --
    DeleteAnnotation,
    JumpToAnnotation,

    // -- Command mode --
    CommandChar(char),
    CommandBackspace,
    CommandConfirm,

    // -- Input mode --
    /// Forward the raw key event to the input box for handling.
    InputForward(KeyEvent),

    // -- Help --
    ToggleHelp,

    // -- Word wrap --
    ToggleWordWrap,

    // -- No-op --
    None,
}

/// Handles key event → action dispatch with support for multi-key sequences.
///
/// Multi-key sequences supported:
/// - Normal: `gg`, `gc`, `]a`, `[a`
/// - AnnotationList: `dd`
pub struct KeybindHandler {
    pending: Option<KeyCode>,
}

impl KeybindHandler {
    pub fn new() -> Self {
        Self { pending: None }
    }

    /// Returns `true` if there is a pending partial key sequence.
    #[cfg_attr(not(test), allow(dead_code))]
    pub fn has_pending(&self) -> bool {
        self.pending.is_some()
    }

    /// Translate a key event into an action given the current mode.
    pub fn handle(&mut self, mode: Mode, event: KeyEvent) -> Action {
        match mode {
            Mode::Normal => self.handle_normal(event),
            Mode::Visual => self.handle_visual(event),
            Mode::Insert => self.handle_insert(event),
            Mode::AnnotationList => self.handle_annotation_list(event),
            Mode::Command => self.handle_command(event),
        }
    }

    fn handle_normal(&mut self, event: KeyEvent) -> Action {
        // Resolve pending multi-key sequences first.
        if let Some(first) = self.pending.take() {
            return self.resolve_normal_sequence(first, event.code);
        }

        match (event.code, event.modifiers) {
            // Multi-key sequence starters
            (KeyCode::Char('g'), KeyModifiers::NONE) => {
                self.pending = Some(KeyCode::Char('g'));
                Action::None
            }
            (KeyCode::Char(']'), KeyModifiers::NONE) => {
                self.pending = Some(KeyCode::Char(']'));
                Action::None
            }
            (KeyCode::Char('['), KeyModifiers::NONE) => {
                self.pending = Some(KeyCode::Char('['));
                Action::None
            }

            // Movement
            (KeyCode::Char('j') | KeyCode::Down, KeyModifiers::NONE) => Action::MoveDown,
            (KeyCode::Char('k') | KeyCode::Up, KeyModifiers::NONE) => Action::MoveUp,
            (KeyCode::Char('h') | KeyCode::Left, KeyModifiers::NONE) => Action::MoveLeft,
            (KeyCode::Char('l') | KeyCode::Right, KeyModifiers::NONE) => Action::MoveRight,
            (KeyCode::Char('w'), KeyModifiers::NONE) => Action::MoveWordForward,
            (KeyCode::Char('b'), KeyModifiers::NONE) => Action::MoveWordBackward,
            (KeyCode::Char('e'), KeyModifiers::NONE) => Action::MoveWordEnd,
            (KeyCode::Char('0'), KeyModifiers::NONE) => Action::MoveLineStart,
            (KeyCode::Char('$'), KeyModifiers::NONE) => Action::MoveLineEnd,
            (KeyCode::Char('G'), KeyModifiers::SHIFT) => Action::MoveDocumentBottom,
            (KeyCode::Char('d'), KeyModifiers::CONTROL) => Action::HalfPageDown,
            (KeyCode::Char('u'), KeyModifiers::CONTROL) => Action::HalfPageUp,
            (KeyCode::Char('f'), KeyModifiers::CONTROL) => Action::FullPageDown,
            (KeyCode::Char('b'), KeyModifiers::CONTROL) => Action::FullPageUp,

            // Mode transitions
            (KeyCode::Char('v'), KeyModifiers::NONE) => Action::EnterVisualMode,
            (KeyCode::Char('i'), KeyModifiers::NONE) => Action::CreateInsertion,
            (KeyCode::Char(':'), KeyModifiers::NONE | KeyModifiers::SHIFT) => {
                Action::EnterCommandMode
            }
            (KeyCode::Tab, KeyModifiers::NONE) => Action::EnterAnnotationListMode,

            // Help
            (KeyCode::Char('?'), KeyModifiers::NONE | KeyModifiers::SHIFT) => Action::ToggleHelp,

            // Word wrap toggle
            (KeyCode::Char('W'), KeyModifiers::SHIFT) => Action::ToggleWordWrap,

            _ => Action::None,
        }
    }

    fn resolve_normal_sequence(&mut self, first: KeyCode, second: KeyCode) -> Action {
        match (first, second) {
            // gg — top of document
            (KeyCode::Char('g'), KeyCode::Char('g')) => Action::MoveDocumentTop,
            // gc — global comment
            (KeyCode::Char('g'), KeyCode::Char('c')) => Action::CreateGlobalComment,
            // ]a — next annotation
            (KeyCode::Char(']'), KeyCode::Char('a')) => Action::NextAnnotation,
            // [a — previous annotation
            (KeyCode::Char('['), KeyCode::Char('a')) => Action::PrevAnnotation,
            // Unrecognized sequence — discard
            _ => Action::None,
        }
    }

    fn handle_visual(&mut self, event: KeyEvent) -> Action {
        match (event.code, event.modifiers) {
            // Movement (extend selection)
            (KeyCode::Char('j') | KeyCode::Down, KeyModifiers::NONE) => Action::MoveDown,
            (KeyCode::Char('k') | KeyCode::Up, KeyModifiers::NONE) => Action::MoveUp,
            (KeyCode::Char('h') | KeyCode::Left, KeyModifiers::NONE) => Action::MoveLeft,
            (KeyCode::Char('l') | KeyCode::Right, KeyModifiers::NONE) => Action::MoveRight,
            (KeyCode::Char('w'), KeyModifiers::NONE) => Action::MoveWordForward,
            (KeyCode::Char('b'), KeyModifiers::NONE) => Action::MoveWordBackward,
            (KeyCode::Char('e'), KeyModifiers::NONE) => Action::MoveWordEnd,
            (KeyCode::Char('0'), KeyModifiers::NONE) => Action::MoveLineStart,
            (KeyCode::Char('$'), KeyModifiers::NONE) => Action::MoveLineEnd,

            // Annotation creation from selection
            (KeyCode::Char('d'), KeyModifiers::NONE) => Action::CreateDeletion,
            (KeyCode::Char('c'), KeyModifiers::NONE) => Action::CreateComment,
            (KeyCode::Char('r'), KeyModifiers::NONE) => Action::CreateReplacement,

            // Cancel selection
            (KeyCode::Esc, _) => Action::ExitToNormal,

            _ => Action::None,
        }
    }

    fn handle_insert(&mut self, event: KeyEvent) -> Action {
        Action::InputForward(event)
    }

    fn handle_annotation_list(&mut self, event: KeyEvent) -> Action {
        // Resolve pending multi-key sequences (dd).
        if let Some(first) = self.pending.take() {
            return self.resolve_annotation_list_sequence(first, event.code);
        }

        match (event.code, event.modifiers) {
            (KeyCode::Char('j') | KeyCode::Down, KeyModifiers::NONE) => Action::MoveDown,
            (KeyCode::Char('k') | KeyCode::Up, KeyModifiers::NONE) => Action::MoveUp,
            (KeyCode::Enter, _) => Action::JumpToAnnotation,
            (KeyCode::Tab | KeyCode::Esc, _) => Action::ExitToNormal,

            // dd starter
            (KeyCode::Char('d'), KeyModifiers::NONE) => {
                self.pending = Some(KeyCode::Char('d'));
                Action::None
            }

            _ => Action::None,
        }
    }

    fn resolve_annotation_list_sequence(&mut self, first: KeyCode, second: KeyCode) -> Action {
        match (first, second) {
            (KeyCode::Char('d'), KeyCode::Char('d')) => Action::DeleteAnnotation,
            _ => Action::None,
        }
    }

    fn handle_command(&mut self, event: KeyEvent) -> Action {
        match event.code {
            KeyCode::Esc => Action::ExitToNormal,
            KeyCode::Enter => Action::CommandConfirm,
            KeyCode::Backspace => Action::CommandBackspace,
            KeyCode::Char(c) => Action::CommandChar(c),
            _ => Action::None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    fn key_mod(code: KeyCode, modifiers: KeyModifiers) -> KeyEvent {
        KeyEvent::new(code, modifiers)
    }

    fn char_key(c: char) -> KeyEvent {
        key(KeyCode::Char(c))
    }

    // ── Normal mode: single keys ──────────────────────────────────

    #[test]
    fn normal_movement_keys() {
        let mut h = KeybindHandler::new();
        assert_eq!(h.handle(Mode::Normal, char_key('j')), Action::MoveDown);
        assert_eq!(h.handle(Mode::Normal, char_key('k')), Action::MoveUp);
        assert_eq!(h.handle(Mode::Normal, char_key('h')), Action::MoveLeft);
        assert_eq!(h.handle(Mode::Normal, char_key('l')), Action::MoveRight);
        assert_eq!(
            h.handle(Mode::Normal, char_key('w')),
            Action::MoveWordForward
        );
        assert_eq!(
            h.handle(Mode::Normal, char_key('b')),
            Action::MoveWordBackward
        );
        assert_eq!(
            h.handle(Mode::Normal, char_key('e')),
            Action::MoveWordEnd
        );
        assert_eq!(h.handle(Mode::Normal, char_key('0')), Action::MoveLineStart);
        assert_eq!(h.handle(Mode::Normal, char_key('$')), Action::MoveLineEnd);
    }

    #[test]
    fn normal_shift_g_moves_to_bottom() {
        let mut h = KeybindHandler::new();
        assert_eq!(
            h.handle(
                Mode::Normal,
                key_mod(KeyCode::Char('G'), KeyModifiers::SHIFT)
            ),
            Action::MoveDocumentBottom
        );
    }

    #[test]
    fn normal_ctrl_movement() {
        let mut h = KeybindHandler::new();
        assert_eq!(
            h.handle(
                Mode::Normal,
                key_mod(KeyCode::Char('d'), KeyModifiers::CONTROL)
            ),
            Action::HalfPageDown
        );
        assert_eq!(
            h.handle(
                Mode::Normal,
                key_mod(KeyCode::Char('u'), KeyModifiers::CONTROL)
            ),
            Action::HalfPageUp
        );
        assert_eq!(
            h.handle(
                Mode::Normal,
                key_mod(KeyCode::Char('f'), KeyModifiers::CONTROL)
            ),
            Action::FullPageDown
        );
        assert_eq!(
            h.handle(
                Mode::Normal,
                key_mod(KeyCode::Char('b'), KeyModifiers::CONTROL)
            ),
            Action::FullPageUp
        );
    }

    #[test]
    fn normal_mode_transitions() {
        let mut h = KeybindHandler::new();
        assert_eq!(
            h.handle(Mode::Normal, char_key('v')),
            Action::EnterVisualMode
        );
        assert_eq!(
            h.handle(Mode::Normal, char_key('i')),
            Action::CreateInsertion
        );
        assert_eq!(
            h.handle(Mode::Normal, char_key(':')),
            Action::EnterCommandMode
        );
        assert_eq!(
            h.handle(Mode::Normal, key(KeyCode::Tab)),
            Action::EnterAnnotationListMode
        );
    }

    #[test]
    fn normal_help_toggle() {
        let mut h = KeybindHandler::new();
        assert_eq!(h.handle(Mode::Normal, char_key('?')), Action::ToggleHelp);
    }

    // ── Normal mode: multi-key sequences ──────────────────────────

    #[test]
    fn normal_gg_sequence() {
        let mut h = KeybindHandler::new();
        assert_eq!(h.handle(Mode::Normal, char_key('g')), Action::None);
        assert!(h.has_pending());
        assert_eq!(
            h.handle(Mode::Normal, char_key('g')),
            Action::MoveDocumentTop
        );
        assert!(!h.has_pending());
    }

    #[test]
    fn normal_gc_sequence() {
        let mut h = KeybindHandler::new();
        assert_eq!(h.handle(Mode::Normal, char_key('g')), Action::None);
        assert_eq!(
            h.handle(Mode::Normal, char_key('c')),
            Action::CreateGlobalComment
        );
    }

    #[test]
    fn normal_bracket_a_sequences() {
        let mut h = KeybindHandler::new();

        // ]a
        assert_eq!(h.handle(Mode::Normal, char_key(']')), Action::None);
        assert_eq!(
            h.handle(Mode::Normal, char_key('a')),
            Action::NextAnnotation
        );

        // [a
        assert_eq!(h.handle(Mode::Normal, char_key('[')), Action::None);
        assert_eq!(
            h.handle(Mode::Normal, char_key('a')),
            Action::PrevAnnotation
        );
    }

    #[test]
    fn normal_invalid_sequence_discards() {
        let mut h = KeybindHandler::new();
        assert_eq!(h.handle(Mode::Normal, char_key('g')), Action::None);
        // 'x' is not a valid continuation of 'g'
        assert_eq!(h.handle(Mode::Normal, char_key('x')), Action::None);
        assert!(!h.has_pending());
        // Next key should work normally
        assert_eq!(h.handle(Mode::Normal, char_key('j')), Action::MoveDown);
    }

    // ── Visual mode ───────────────────────────────────────────────

    #[test]
    fn visual_movement_extends_selection() {
        let mut h = KeybindHandler::new();
        assert_eq!(h.handle(Mode::Visual, char_key('j')), Action::MoveDown);
        assert_eq!(h.handle(Mode::Visual, char_key('k')), Action::MoveUp);
        assert_eq!(h.handle(Mode::Visual, char_key('h')), Action::MoveLeft);
        assert_eq!(h.handle(Mode::Visual, char_key('l')), Action::MoveRight);
        assert_eq!(
            h.handle(Mode::Visual, char_key('w')),
            Action::MoveWordForward
        );
        assert_eq!(
            h.handle(Mode::Visual, char_key('b')),
            Action::MoveWordBackward
        );
        assert_eq!(
            h.handle(Mode::Visual, char_key('e')),
            Action::MoveWordEnd
        );
        assert_eq!(h.handle(Mode::Visual, char_key('0')), Action::MoveLineStart);
        assert_eq!(h.handle(Mode::Visual, char_key('$')), Action::MoveLineEnd);
    }

    #[test]
    fn visual_annotation_actions() {
        let mut h = KeybindHandler::new();
        assert_eq!(
            h.handle(Mode::Visual, char_key('d')),
            Action::CreateDeletion
        );
        assert_eq!(h.handle(Mode::Visual, char_key('c')), Action::CreateComment);
        assert_eq!(
            h.handle(Mode::Visual, char_key('r')),
            Action::CreateReplacement
        );
    }

    #[test]
    fn visual_esc_exits() {
        let mut h = KeybindHandler::new();
        assert_eq!(
            h.handle(Mode::Visual, key(KeyCode::Esc)),
            Action::ExitToNormal
        );
    }

    // ── Insert mode ───────────────────────────────────────────────

    #[test]
    fn insert_mode_forwards_all_keys() {
        let mut h = KeybindHandler::new();
        let event = char_key('a');
        let action = h.handle(Mode::Insert, event);
        assert!(matches!(action, Action::InputForward(_)));
    }

    #[test]
    fn insert_mode_forwards_esc() {
        let mut h = KeybindHandler::new();
        let event = key(KeyCode::Esc);
        let action = h.handle(Mode::Insert, event);
        assert!(matches!(action, Action::InputForward(_)));
    }

    #[test]
    fn insert_mode_forwards_enter() {
        let mut h = KeybindHandler::new();
        let event = key(KeyCode::Enter);
        let action = h.handle(Mode::Insert, event);
        assert!(matches!(action, Action::InputForward(_)));
    }

    // ── Annotation list mode ──────────────────────────────────────

    #[test]
    fn annotation_list_navigation() {
        let mut h = KeybindHandler::new();
        assert_eq!(
            h.handle(Mode::AnnotationList, char_key('j')),
            Action::MoveDown
        );
        assert_eq!(
            h.handle(Mode::AnnotationList, char_key('k')),
            Action::MoveUp
        );
        assert_eq!(
            h.handle(Mode::AnnotationList, key(KeyCode::Enter)),
            Action::JumpToAnnotation
        );
    }

    #[test]
    fn annotation_list_dd_deletes() {
        let mut h = KeybindHandler::new();
        assert_eq!(h.handle(Mode::AnnotationList, char_key('d')), Action::None);
        assert!(h.has_pending());
        assert_eq!(
            h.handle(Mode::AnnotationList, char_key('d')),
            Action::DeleteAnnotation
        );
    }

    #[test]
    fn annotation_list_exit() {
        let mut h = KeybindHandler::new();
        assert_eq!(
            h.handle(Mode::AnnotationList, key(KeyCode::Tab)),
            Action::ExitToNormal
        );
        assert_eq!(
            h.handle(Mode::AnnotationList, key(KeyCode::Esc)),
            Action::ExitToNormal
        );
    }

    // ── Command mode ──────────────────────────────────────────────

    #[test]
    fn command_mode_input() {
        let mut h = KeybindHandler::new();
        assert_eq!(
            h.handle(Mode::Command, char_key('q')),
            Action::CommandChar('q')
        );
        assert_eq!(
            h.handle(Mode::Command, key(KeyCode::Backspace)),
            Action::CommandBackspace
        );
        assert_eq!(
            h.handle(Mode::Command, key(KeyCode::Enter)),
            Action::CommandConfirm
        );
        assert_eq!(
            h.handle(Mode::Command, key(KeyCode::Esc)),
            Action::ExitToNormal
        );
    }

    // ── Mode transition flows ─────────────────────────────────────

    #[test]
    fn normal_to_visual_and_back() {
        let mut h = KeybindHandler::new();
        assert_eq!(
            h.handle(Mode::Normal, char_key('v')),
            Action::EnterVisualMode
        );
        // Now in Visual mode, Esc returns to Normal
        assert_eq!(
            h.handle(Mode::Visual, key(KeyCode::Esc)),
            Action::ExitToNormal
        );
    }

    #[test]
    fn normal_to_command_and_back() {
        let mut h = KeybindHandler::new();
        assert_eq!(
            h.handle(Mode::Normal, char_key(':')),
            Action::EnterCommandMode
        );
        assert_eq!(
            h.handle(Mode::Command, key(KeyCode::Esc)),
            Action::ExitToNormal
        );
    }

    #[test]
    fn arrow_keys_work_in_normal() {
        let mut h = KeybindHandler::new();
        assert_eq!(h.handle(Mode::Normal, key(KeyCode::Down)), Action::MoveDown);
        assert_eq!(h.handle(Mode::Normal, key(KeyCode::Up)), Action::MoveUp);
        assert_eq!(h.handle(Mode::Normal, key(KeyCode::Left)), Action::MoveLeft);
        assert_eq!(
            h.handle(Mode::Normal, key(KeyCode::Right)),
            Action::MoveRight
        );
    }

    #[test]
    fn normal_shift_w_toggles_word_wrap() {
        let mut h = KeybindHandler::new();
        assert_eq!(
            h.handle(
                Mode::Normal,
                key_mod(KeyCode::Char('W'), KeyModifiers::SHIFT)
            ),
            Action::ToggleWordWrap
        );
    }

    #[test]
    fn unrecognized_key_returns_none() {
        let mut h = KeybindHandler::new();
        assert_eq!(h.handle(Mode::Normal, char_key('z')), Action::None);
        assert_eq!(h.handle(Mode::Visual, char_key('z')), Action::None);
        assert_eq!(h.handle(Mode::AnnotationList, char_key('z')), Action::None);
    }
}
