use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::mode::Mode;

/// Actions that can be dispatched from key events.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CharSearchDirection {
    Forward,
    Backward,
}

impl CharSearchDirection {
    pub(crate) fn reversed(self) -> Self {
        match self {
            Self::Forward => Self::Backward,
            Self::Backward => Self::Forward,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RepeatDirection {
    Same,
    Opposite,
}

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
    MoveToChar {
        target: char,
        direction: CharSearchDirection,
        until: bool,
        count: usize,
    },
    RepeatLastCharSearch {
        direction: RepeatDirection,
        count: usize,
    },

    // -- Mode transitions --
    EnterVisualMode,
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
    OpenAnnotationInspect,
    DeleteAnnotation,
    JumpToAnnotation,
    ScrollOverlayUp,
    ScrollOverlayDown,
    ScrollOverlayPageUp,
    ScrollOverlayPageDown,
    HideAnnotationList,

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

    // -- Quit --
    ForceQuit,

    // -- Counted actions --
    Repeat {
        action: Box<Action>,
        count: usize,
    },

    // -- No-op --
    None,
}

impl Action {
    fn supports_count(&self) -> bool {
        matches!(
            self,
            Action::MoveUp
                | Action::MoveDown
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
                | Action::JumpToAnnotation
                | Action::ScrollOverlayUp
                | Action::ScrollOverlayDown
                | Action::ScrollOverlayPageUp
                | Action::ScrollOverlayPageDown
        )
    }

    fn counted(self, count: Option<usize>) -> Self {
        match count {
            Some(count) if self.supports_count() => Action::Repeat {
                action: Box::new(self),
                count,
            },
            Some(_) => Action::None,
            _ => self,
        }
    }
}

/// Handles key event → action dispatch with support for multi-key sequences.
///
/// Multi-key sequences supported:
/// - Normal: `gg`, `gc`, `]a`, `[a`
/// - AnnotationList: `dd`
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PendingInput {
    FixedSequence(KeyCode),
    CharSearch {
        direction: CharSearchDirection,
        until: bool,
    },
}

pub struct KeybindHandler {
    pending: Option<PendingInput>,
    count: Option<usize>,
    count_overflowed: bool,
}

impl KeybindHandler {
    pub fn new() -> Self {
        Self {
            pending: None,
            count: None,
            count_overflowed: false,
        }
    }

    pub fn clear_pending(&mut self) {
        self.pending = None;
        self.count = None;
        self.count_overflowed = false;
    }

    /// Returns `true` if there is a pending partial key sequence.
    #[cfg_attr(not(test), allow(dead_code))]
    pub fn has_pending(&self) -> bool {
        self.pending.is_some() || self.count.is_some() || self.count_overflowed
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

    /// Translate a key event while the help overlay is visible.
    pub fn handle_help_overlay(&mut self, _mode: Mode, event: KeyEvent) -> Action {
        if matches!(event.code, KeyCode::Char('?'))
            && matches!(event.modifiers, KeyModifiers::NONE | KeyModifiers::SHIFT)
        {
            self.clear_pending();
            return Action::ToggleHelp;
        }

        match (event.code, event.modifiers) {
            (KeyCode::Esc, _) | (KeyCode::Char('q'), KeyModifiers::NONE) => {
                self.clear_pending();
                Action::ToggleHelp
            }
            _ => self.handle_counted_overlay_navigation(event, |handler, event| {
                match (event.code, event.modifiers) {
                    (KeyCode::Char('j') | KeyCode::Down, KeyModifiers::NONE) => {
                        handler.finish_action(Action::MoveDown)
                    }
                    (KeyCode::Char('k') | KeyCode::Up, KeyModifiers::NONE) => {
                        handler.finish_action(Action::MoveUp)
                    }
                    _ => Action::None,
                }
            }),
        }
    }

    /// Translate a key event while the annotation inspect overlay is visible.
    pub fn handle_annotation_inspect(&mut self, event: KeyEvent) -> Action {
        match (event.code, event.modifiers) {
            (KeyCode::Enter, _) => {
                self.clear_pending();
                Action::JumpToAnnotation
            }
            (KeyCode::Esc, _) => {
                self.clear_pending();
                Action::ExitToNormal
            }
            (KeyCode::Char('c'), KeyModifiers::CONTROL) => {
                self.clear_pending();
                Action::ForceQuit
            }
            _ => self.handle_counted_overlay_navigation(event, |handler, event| {
                match (event.code, event.modifiers) {
                    (KeyCode::Char('j'), KeyModifiers::NONE) => {
                        handler.finish_action(Action::MoveDown)
                    }
                    (KeyCode::Char('k'), KeyModifiers::NONE) => {
                        handler.finish_action(Action::MoveUp)
                    }
                    (KeyCode::Down, KeyModifiers::NONE) => {
                        handler.finish_action(Action::ScrollOverlayDown)
                    }
                    (KeyCode::Up, KeyModifiers::NONE) => {
                        handler.finish_action(Action::ScrollOverlayUp)
                    }
                    (KeyCode::PageDown, KeyModifiers::NONE)
                    | (KeyCode::Char('d'), KeyModifiers::CONTROL) => {
                        handler.finish_action(Action::ScrollOverlayPageDown)
                    }
                    (KeyCode::PageUp, KeyModifiers::NONE)
                    | (KeyCode::Char('u'), KeyModifiers::CONTROL) => {
                        handler.finish_action(Action::ScrollOverlayPageUp)
                    }
                    _ => Action::None,
                }
            }),
        }
    }

    fn handle_normal(&mut self, event: KeyEvent) -> Action {
        if matches!(event.code, KeyCode::Char('?'))
            && matches!(event.modifiers, KeyModifiers::NONE | KeyModifiers::SHIFT)
        {
            self.clear_pending();
            return Action::ToggleHelp;
        }

        // Resolve pending multi-key sequences first.
        if let Some(pending) = self.pending.take() {
            return self.resolve_normal_input(pending, event);
        }

        if self.consume_count_prefix(event) {
            return Action::None;
        }

        if self.count_overflowed {
            self.clear_pending();
            return Action::None;
        }

        if let Some(action) = self.try_handle_char_search(event) {
            return action;
        }

        match (event.code, event.modifiers) {
            // Multi-key sequence starters
            (KeyCode::Char('g'), KeyModifiers::NONE) => {
                self.pending = Some(PendingInput::FixedSequence(KeyCode::Char('g')));
                Action::None
            }
            (KeyCode::Char(']'), KeyModifiers::NONE) => {
                self.pending = Some(PendingInput::FixedSequence(KeyCode::Char(']')));
                Action::None
            }
            (KeyCode::Char('['), KeyModifiers::NONE) => {
                self.pending = Some(PendingInput::FixedSequence(KeyCode::Char('[')));
                Action::None
            }

            // Movement
            (KeyCode::Char('j') | KeyCode::Down, KeyModifiers::NONE) => {
                self.finish_action(Action::MoveDown)
            }
            (KeyCode::Char('k') | KeyCode::Up, KeyModifiers::NONE) => {
                self.finish_action(Action::MoveUp)
            }
            (KeyCode::Char('h') | KeyCode::Left, KeyModifiers::NONE) => {
                self.finish_action(Action::MoveLeft)
            }
            (KeyCode::Char('l') | KeyCode::Right, KeyModifiers::NONE) => {
                self.finish_action(Action::MoveRight)
            }
            (KeyCode::Char('w'), KeyModifiers::NONE) => self.finish_action(Action::MoveWordForward),
            (KeyCode::Char('b'), KeyModifiers::NONE) => {
                self.finish_action(Action::MoveWordBackward)
            }
            (KeyCode::Char('e'), KeyModifiers::NONE) => self.finish_action(Action::MoveWordEnd),
            (KeyCode::Char('0'), KeyModifiers::NONE) => self.finish_action(Action::MoveLineStart),
            (KeyCode::Char('$'), KeyModifiers::NONE) => self.finish_action(Action::MoveLineEnd),
            (KeyCode::Char('G'), KeyModifiers::SHIFT) => {
                self.finish_action(Action::MoveDocumentBottom)
            }
            (KeyCode::Char('d'), KeyModifiers::CONTROL) => self.finish_action(Action::HalfPageDown),
            (KeyCode::Char('u'), KeyModifiers::CONTROL) => self.finish_action(Action::HalfPageUp),
            (KeyCode::Char('f'), KeyModifiers::CONTROL) => self.finish_action(Action::FullPageDown),
            (KeyCode::Char('b'), KeyModifiers::CONTROL) => self.finish_action(Action::FullPageUp),

            // Mode transitions
            (KeyCode::Char('v'), KeyModifiers::NONE) => self.finish_action(Action::EnterVisualMode),
            (KeyCode::Char('i'), KeyModifiers::NONE) => self.finish_action(Action::CreateInsertion),
            (KeyCode::Char(':'), KeyModifiers::NONE | KeyModifiers::SHIFT) => {
                self.finish_action(Action::EnterCommandMode)
            }
            (KeyCode::Tab, KeyModifiers::NONE) => {
                self.finish_action(Action::EnterAnnotationListMode)
            }
            (KeyCode::Esc, KeyModifiers::NONE) => self.finish_action(Action::HideAnnotationList),

            // Help
            (KeyCode::Char('?'), KeyModifiers::NONE | KeyModifiers::SHIFT) => {
                self.finish_action(Action::ToggleHelp)
            }

            // Word wrap toggle
            (KeyCode::Char('W'), KeyModifiers::SHIFT) => self.finish_action(Action::ToggleWordWrap),

            // Ctrl-C — force quit
            (KeyCode::Char('c'), KeyModifiers::CONTROL) => self.finish_action(Action::ForceQuit),

            _ => {
                self.clear_pending();
                Action::None
            }
        }
    }

    fn handle_visual(&mut self, event: KeyEvent) -> Action {
        if let Some(pending) = self.pending.take() {
            return self.resolve_visual_input(pending, event);
        }

        if self.consume_count_prefix(event) {
            return Action::None;
        }

        if self.count_overflowed {
            self.clear_pending();
            return Action::None;
        }

        if let Some(action) = self.try_handle_char_search(event) {
            return action;
        }

        match (event.code, event.modifiers) {
            // Movement (extend selection)
            (KeyCode::Char('j') | KeyCode::Down, KeyModifiers::NONE) => {
                self.finish_action(Action::MoveDown)
            }
            (KeyCode::Char('k') | KeyCode::Up, KeyModifiers::NONE) => {
                self.finish_action(Action::MoveUp)
            }
            (KeyCode::Char('h') | KeyCode::Left, KeyModifiers::NONE) => {
                self.finish_action(Action::MoveLeft)
            }
            (KeyCode::Char('l') | KeyCode::Right, KeyModifiers::NONE) => {
                self.finish_action(Action::MoveRight)
            }
            (KeyCode::Char('w'), KeyModifiers::NONE) => self.finish_action(Action::MoveWordForward),
            (KeyCode::Char('b'), KeyModifiers::NONE) => {
                self.finish_action(Action::MoveWordBackward)
            }
            (KeyCode::Char('e'), KeyModifiers::NONE) => self.finish_action(Action::MoveWordEnd),
            (KeyCode::Char('0'), KeyModifiers::NONE) => self.finish_action(Action::MoveLineStart),
            (KeyCode::Char('$'), KeyModifiers::NONE) => self.finish_action(Action::MoveLineEnd),

            // Annotation creation from selection
            (KeyCode::Char('d'), KeyModifiers::NONE) => self.finish_action(Action::CreateDeletion),
            (KeyCode::Char('c'), KeyModifiers::NONE) => self.finish_action(Action::CreateComment),
            (KeyCode::Char('r'), KeyModifiers::NONE) => {
                self.finish_action(Action::CreateReplacement)
            }

            // Cancel selection
            (KeyCode::Esc, _) => self.finish_action(Action::ExitToNormal),

            // Ctrl-C — force quit
            (KeyCode::Char('c'), KeyModifiers::CONTROL) => self.finish_action(Action::ForceQuit),

            _ => {
                self.clear_pending();
                Action::None
            }
        }
    }

    fn resolve_normal_input(&mut self, pending: PendingInput, event: KeyEvent) -> Action {
        match pending {
            PendingInput::FixedSequence(first) => {
                self.resolve_fixed_normal_sequence(first, event.code)
            }
            PendingInput::CharSearch { direction, until } => {
                self.resolve_char_search(direction, until, event)
            }
        }
    }

    fn resolve_visual_input(&mut self, pending: PendingInput, event: KeyEvent) -> Action {
        match pending {
            PendingInput::CharSearch { direction, until } => {
                self.resolve_char_search(direction, until, event)
            }
            PendingInput::FixedSequence(_) => {
                self.clear_pending();
                Action::None
            }
        }
    }

    fn resolve_fixed_normal_sequence(&mut self, first: KeyCode, second: KeyCode) -> Action {
        match (first, second) {
            // gg — top of document
            (KeyCode::Char('g'), KeyCode::Char('g')) => self.finish_action(Action::MoveDocumentTop),
            // gc — global comment
            (KeyCode::Char('g'), KeyCode::Char('c')) => {
                self.finish_action(Action::CreateGlobalComment)
            }
            // ]a — next annotation
            (KeyCode::Char(']'), KeyCode::Char('a')) => self.finish_action(Action::NextAnnotation),
            // [a — previous annotation
            (KeyCode::Char('['), KeyCode::Char('a')) => self.finish_action(Action::PrevAnnotation),
            // Unrecognized sequence — discard
            _ => {
                self.clear_pending();
                Action::None
            }
        }
    }

    fn try_handle_char_search(&mut self, event: KeyEvent) -> Option<Action> {
        match (event.code, event.modifiers) {
            (KeyCode::Char('f'), KeyModifiers::NONE) => {
                self.pending = Some(PendingInput::CharSearch {
                    direction: CharSearchDirection::Forward,
                    until: false,
                });
                Some(Action::None)
            }
            (KeyCode::Char('F'), KeyModifiers::SHIFT) => {
                self.pending = Some(PendingInput::CharSearch {
                    direction: CharSearchDirection::Backward,
                    until: false,
                });
                Some(Action::None)
            }
            (KeyCode::Char('t'), KeyModifiers::NONE) => {
                self.pending = Some(PendingInput::CharSearch {
                    direction: CharSearchDirection::Forward,
                    until: true,
                });
                Some(Action::None)
            }
            (KeyCode::Char('T'), KeyModifiers::SHIFT) => {
                self.pending = Some(PendingInput::CharSearch {
                    direction: CharSearchDirection::Backward,
                    until: true,
                });
                Some(Action::None)
            }
            (KeyCode::Char(';'), KeyModifiers::NONE) => {
                Some(self.finish_char_search_repeat(RepeatDirection::Same))
            }
            (KeyCode::Char(','), KeyModifiers::NONE) => {
                Some(self.finish_char_search_repeat(RepeatDirection::Opposite))
            }
            _ => None,
        }
    }

    fn handle_insert(&mut self, event: KeyEvent) -> Action {
        if event.code == KeyCode::Char('c') && event.modifiers.contains(KeyModifiers::CONTROL) {
            Action::ForceQuit
        } else {
            Action::InputForward(event)
        }
    }

    fn handle_annotation_list(&mut self, event: KeyEvent) -> Action {
        if matches!(event.code, KeyCode::Char(' ')) && event.modifiers == KeyModifiers::NONE {
            self.clear_pending();
            return Action::OpenAnnotationInspect;
        }

        // Resolve pending multi-key sequences (dd).
        if let Some(pending) = self.pending.take() {
            return self.resolve_annotation_list_input(pending, event.code);
        }

        if self.consume_count_prefix(event) {
            return Action::None;
        }

        if self.count_overflowed {
            self.clear_pending();
            return Action::None;
        }

        match (event.code, event.modifiers) {
            (KeyCode::Char('j') | KeyCode::Down, KeyModifiers::NONE) => {
                self.finish_action(Action::MoveDown)
            }
            (KeyCode::Char('k') | KeyCode::Up, KeyModifiers::NONE) => {
                self.finish_action(Action::MoveUp)
            }
            (KeyCode::Enter, _) => self.finish_action(Action::JumpToAnnotation),
            (KeyCode::Tab, _) => self.finish_action(Action::EnterAnnotationListMode),
            (KeyCode::Esc, _) => self.finish_action(Action::HideAnnotationList),

            // Ctrl-C — force quit
            (KeyCode::Char('c'), KeyModifiers::CONTROL) => self.finish_action(Action::ForceQuit),

            // dd starter
            (KeyCode::Char('d'), KeyModifiers::NONE) => {
                self.pending = Some(PendingInput::FixedSequence(KeyCode::Char('d')));
                Action::None
            }

            _ => {
                self.clear_pending();
                Action::None
            }
        }
    }

    fn resolve_annotation_list_input(&mut self, pending: PendingInput, second: KeyCode) -> Action {
        match pending {
            PendingInput::FixedSequence(first) => match (first, second) {
                (KeyCode::Char('d'), KeyCode::Char('d')) => {
                    self.finish_action(Action::DeleteAnnotation)
                }
                _ => {
                    self.clear_pending();
                    Action::None
                }
            },
            PendingInput::CharSearch { .. } => {
                self.clear_pending();
                Action::None
            }
        }
    }

    fn consume_count_prefix(&mut self, event: KeyEvent) -> bool {
        match (event.code, event.modifiers) {
            (KeyCode::Char(c @ '1'..='9'), KeyModifiers::NONE) => {
                self.push_count_digit(c);
                true
            }
            (KeyCode::Char('0'), KeyModifiers::NONE) if self.count.is_some() => {
                self.push_count_digit('0');
                true
            }
            _ => false,
        }
    }

    fn push_count_digit(&mut self, digit: char) {
        let digit = digit
            .to_digit(10)
            .expect("count prefixes only accept ASCII digits") as usize;
        let current = self.count.unwrap_or(0);

        match current
            .checked_mul(10)
            .and_then(|count| count.checked_add(digit))
        {
            Some(count) => self.count = Some(count),
            None => {
                self.pending = None;
                self.count = None;
                self.count_overflowed = true;
            }
        }
    }

    fn finish_action(&mut self, action: Action) -> Action {
        let count = self.count.take();
        action.counted(count)
    }

    fn finish_char_search(
        &mut self,
        target: char,
        direction: CharSearchDirection,
        until: bool,
    ) -> Action {
        let count = self.count.take().unwrap_or(1);
        Action::MoveToChar {
            target,
            direction,
            until,
            count,
        }
    }

    fn finish_char_search_repeat(&mut self, direction: RepeatDirection) -> Action {
        let count = self.count.take().unwrap_or(1);
        Action::RepeatLastCharSearch { direction, count }
    }

    fn resolve_char_search(
        &mut self,
        direction: CharSearchDirection,
        until: bool,
        event: KeyEvent,
    ) -> Action {
        match event.code {
            KeyCode::Char(target) => self.finish_char_search(target, direction, until),
            _ => {
                self.clear_pending();
                Action::None
            }
        }
    }

    fn handle_counted_overlay_navigation(
        &mut self,
        event: KeyEvent,
        action_for_event: impl FnOnce(&mut Self, KeyEvent) -> Action,
    ) -> Action {
        if self.consume_count_prefix(event) {
            return Action::None;
        }

        if self.count_overflowed {
            self.clear_pending();
            return Action::None;
        }

        let action = action_for_event(self, event);
        if matches!(action, Action::None) {
            self.clear_pending();
        }
        action
    }

    fn handle_command(&mut self, event: KeyEvent) -> Action {
        if event.code == KeyCode::Char('c') && event.modifiers.contains(KeyModifiers::CONTROL) {
            return Action::ForceQuit;
        }

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

    fn repeated(action: Action, count: usize) -> Action {
        Action::Repeat {
            action: Box::new(action),
            count,
        }
    }

    fn char_search(
        target: char,
        direction: CharSearchDirection,
        until: bool,
        count: usize,
    ) -> Action {
        Action::MoveToChar {
            target,
            direction,
            until,
            count,
        }
    }

    fn repeated_char_search(direction: RepeatDirection, count: usize) -> Action {
        Action::RepeatLastCharSearch { direction, count }
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
        assert_eq!(h.handle(Mode::Normal, char_key('e')), Action::MoveWordEnd);
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

    #[test]
    fn normal_help_toggle_clears_pending_sequence() {
        let mut h = KeybindHandler::new();

        assert_eq!(h.handle(Mode::Normal, char_key('g')), Action::None);
        assert!(h.has_pending());
        assert_eq!(h.handle(Mode::Normal, char_key('?')), Action::ToggleHelp);
        assert!(!h.has_pending());
    }

    #[test]
    fn help_overlay_uses_configured_shortcut_to_toggle() {
        let mut h = KeybindHandler::new();

        assert_eq!(
            h.handle_help_overlay(Mode::Normal, char_key('?')),
            Action::ToggleHelp
        );
    }

    #[test]
    fn help_overlay_still_accepts_escape_and_q_to_close() {
        let mut h = KeybindHandler::new();

        assert_eq!(
            h.handle_help_overlay(Mode::Normal, key(KeyCode::Esc)),
            Action::ToggleHelp
        );
        assert_eq!(
            h.handle_help_overlay(Mode::Normal, char_key('q')),
            Action::ToggleHelp
        );
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
    fn normal_char_search_sequences_emit_dedicated_actions() {
        let mut h = KeybindHandler::new();

        assert_eq!(h.handle(Mode::Normal, char_key('f')), Action::None);
        assert!(h.has_pending());
        assert_eq!(
            h.handle(Mode::Normal, char_key('a')),
            char_search('a', CharSearchDirection::Forward, false, 1)
        );

        assert_eq!(h.handle(Mode::Normal, char_key('t')), Action::None);
        assert_eq!(
            h.handle(Mode::Normal, char_key(';')),
            char_search(';', CharSearchDirection::Forward, true, 1)
        );

        assert_eq!(
            h.handle(
                Mode::Normal,
                key_mod(KeyCode::Char('F'), KeyModifiers::SHIFT)
            ),
            Action::None
        );
        assert_eq!(
            h.handle(Mode::Normal, char_key('b')),
            char_search('b', CharSearchDirection::Backward, false, 1)
        );

        assert_eq!(
            h.handle(
                Mode::Normal,
                key_mod(KeyCode::Char('T'), KeyModifiers::SHIFT)
            ),
            Action::None
        );
        assert_eq!(
            h.handle(Mode::Normal, char_key('c')),
            char_search('c', CharSearchDirection::Backward, true, 1)
        );
        assert!(!h.has_pending());
    }

    #[test]
    fn normal_char_search_targets_bypass_count_parsing() {
        let mut h = KeybindHandler::new();

        assert_eq!(h.handle(Mode::Normal, char_key('f')), Action::None);
        assert_eq!(
            h.handle(Mode::Normal, char_key('0')),
            char_search('0', CharSearchDirection::Forward, false, 1)
        );

        assert_eq!(h.handle(Mode::Normal, char_key('2')), Action::None);
        assert_eq!(h.handle(Mode::Normal, char_key('t')), Action::None);
        assert_eq!(
            h.handle(Mode::Normal, char_key(';')),
            char_search(';', CharSearchDirection::Forward, true, 2)
        );
    }

    #[test]
    fn normal_char_search_repeat_keys_emit_repeat_actions() {
        let mut h = KeybindHandler::new();

        assert_eq!(
            h.handle(Mode::Normal, char_key(';')),
            repeated_char_search(RepeatDirection::Same, 1)
        );
        assert_eq!(
            h.handle(Mode::Normal, char_key(',')),
            repeated_char_search(RepeatDirection::Opposite, 1)
        );

        assert_eq!(h.handle(Mode::Normal, char_key('2')), Action::None);
        assert_eq!(
            h.handle(Mode::Normal, char_key(';')),
            repeated_char_search(RepeatDirection::Same, 2)
        );
    }

    #[test]
    fn normal_invalid_char_search_target_discards_parser_state() {
        let mut h = KeybindHandler::new();

        assert_eq!(h.handle(Mode::Normal, char_key('3')), Action::None);
        assert_eq!(h.handle(Mode::Normal, char_key('f')), Action::None);
        assert!(h.has_pending());

        assert_eq!(h.handle(Mode::Normal, key(KeyCode::Esc)), Action::None);
        assert!(!h.has_pending());
        assert_eq!(h.handle(Mode::Normal, char_key('j')), Action::MoveDown);
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

    #[test]
    fn normal_single_digit_count_wraps_motion() {
        let mut h = KeybindHandler::new();

        assert_eq!(h.handle(Mode::Normal, char_key('2')), Action::None);
        assert!(h.has_pending());
        assert_eq!(
            h.handle(Mode::Normal, char_key('j')),
            repeated(Action::MoveDown, 2)
        );
        assert!(!h.has_pending());
    }

    #[test]
    fn normal_multi_digit_count_wraps_motion() {
        let mut h = KeybindHandler::new();

        assert_eq!(h.handle(Mode::Normal, char_key('1')), Action::None);
        assert_eq!(h.handle(Mode::Normal, char_key('2')), Action::None);
        assert_eq!(
            h.handle(Mode::Normal, char_key('j')),
            repeated(Action::MoveDown, 12)
        );
    }

    #[test]
    fn normal_zero_without_count_keeps_line_start_motion() {
        let mut h = KeybindHandler::new();

        assert_eq!(h.handle(Mode::Normal, char_key('0')), Action::MoveLineStart);
    }

    #[test]
    fn normal_zero_extends_existing_count() {
        let mut h = KeybindHandler::new();

        assert_eq!(h.handle(Mode::Normal, char_key('1')), Action::None);
        assert_eq!(h.handle(Mode::Normal, char_key('0')), Action::None);
        assert_eq!(
            h.handle(Mode::Normal, char_key('j')),
            repeated(Action::MoveDown, 10)
        );
    }

    #[test]
    fn normal_counted_sequence_wraps_action() {
        let mut h = KeybindHandler::new();

        assert_eq!(h.handle(Mode::Normal, char_key('3')), Action::None);
        assert_eq!(h.handle(Mode::Normal, char_key('g')), Action::None);
        assert_eq!(
            h.handle(Mode::Normal, char_key('g')),
            repeated(Action::MoveDocumentTop, 3)
        );
    }

    #[test]
    fn normal_counted_bracket_sequence_wraps_action() {
        let mut h = KeybindHandler::new();

        assert_eq!(h.handle(Mode::Normal, char_key('2')), Action::None);
        assert_eq!(h.handle(Mode::Normal, char_key(']')), Action::None);
        assert_eq!(
            h.handle(Mode::Normal, char_key('a')),
            repeated(Action::NextAnnotation, 2)
        );
    }

    #[test]
    fn normal_counted_unsupported_action_is_rejected() {
        let mut h = KeybindHandler::new();

        assert_eq!(h.handle(Mode::Normal, char_key('2')), Action::None);
        assert_eq!(h.handle(Mode::Normal, char_key('g')), Action::None);
        assert_eq!(h.handle(Mode::Normal, char_key('c')), Action::None);
        assert!(!h.has_pending());
    }

    #[test]
    fn normal_invalid_counted_sequence_discards_parser_state() {
        let mut h = KeybindHandler::new();

        assert_eq!(h.handle(Mode::Normal, char_key('3')), Action::None);
        assert_eq!(h.handle(Mode::Normal, char_key('g')), Action::None);
        assert!(h.has_pending());

        assert_eq!(h.handle(Mode::Normal, char_key('x')), Action::None);
        assert!(!h.has_pending());
        assert_eq!(h.handle(Mode::Normal, char_key('j')), Action::MoveDown);
    }

    #[test]
    fn clear_pending_discards_partial_sequence() {
        let mut h = KeybindHandler::new();
        assert_eq!(h.handle(Mode::Normal, char_key('3')), Action::None);
        assert_eq!(h.handle(Mode::Normal, char_key('g')), Action::None);
        assert!(h.has_pending());

        h.clear_pending();

        assert!(!h.has_pending());
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
        assert_eq!(h.handle(Mode::Visual, char_key('e')), Action::MoveWordEnd);
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

    #[test]
    fn visual_counted_motion_wraps_action() {
        let mut h = KeybindHandler::new();

        assert_eq!(h.handle(Mode::Visual, char_key('3')), Action::None);
        assert_eq!(
            h.handle(Mode::Visual, char_key('j')),
            repeated(Action::MoveDown, 3)
        );
    }

    #[test]
    fn visual_char_search_sequences_emit_dedicated_actions() {
        let mut h = KeybindHandler::new();

        assert_eq!(h.handle(Mode::Visual, char_key('f')), Action::None);
        assert_eq!(
            h.handle(Mode::Visual, char_key('x')),
            char_search('x', CharSearchDirection::Forward, false, 1)
        );

        assert_eq!(h.handle(Mode::Visual, char_key('2')), Action::None);
        assert_eq!(
            h.handle(
                Mode::Visual,
                key_mod(KeyCode::Char('T'), KeyModifiers::SHIFT)
            ),
            Action::None
        );
        assert_eq!(
            h.handle(Mode::Visual, char_key('0')),
            char_search('0', CharSearchDirection::Backward, true, 2)
        );
    }

    #[test]
    fn visual_char_search_repeat_keys_emit_repeat_actions() {
        let mut h = KeybindHandler::new();

        assert_eq!(
            h.handle(Mode::Visual, char_key(';')),
            repeated_char_search(RepeatDirection::Same, 1)
        );
        assert_eq!(
            h.handle(Mode::Visual, char_key(',')),
            repeated_char_search(RepeatDirection::Opposite, 1)
        );

        assert_eq!(h.handle(Mode::Visual, char_key('3')), Action::None);
        assert_eq!(
            h.handle(Mode::Visual, char_key(',')),
            repeated_char_search(RepeatDirection::Opposite, 3)
        );
    }

    #[test]
    fn visual_counted_unsupported_action_is_rejected() {
        let mut h = KeybindHandler::new();

        assert_eq!(h.handle(Mode::Visual, char_key('2')), Action::None);
        assert_eq!(h.handle(Mode::Visual, char_key('d')), Action::None);
        assert!(!h.has_pending());
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
        assert_eq!(
            h.handle(Mode::AnnotationList, char_key(' ')),
            Action::OpenAnnotationInspect
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
    fn annotation_list_counted_dd_is_rejected() {
        let mut h = KeybindHandler::new();

        assert_eq!(h.handle(Mode::AnnotationList, char_key('4')), Action::None);
        assert_eq!(h.handle(Mode::AnnotationList, char_key('d')), Action::None);
        assert_eq!(h.handle(Mode::AnnotationList, char_key('d')), Action::None);
        assert!(!h.has_pending());
    }

    #[test]
    fn count_overflow_rejects_followup_action_and_clears_parser_state() {
        let mut h = KeybindHandler::new();

        for digit in format!("{}0", usize::MAX).chars() {
            assert_eq!(h.handle(Mode::Normal, char_key(digit)), Action::None);
        }
        assert!(h.has_pending());

        assert_eq!(h.handle(Mode::Normal, char_key('j')), Action::None);
        assert!(!h.has_pending());
        assert_eq!(h.handle(Mode::Normal, char_key('j')), Action::MoveDown);
    }

    #[test]
    fn annotation_list_space_clears_pending_sequence() {
        let mut h = KeybindHandler::new();

        assert_eq!(h.handle(Mode::AnnotationList, char_key('d')), Action::None);
        assert!(h.has_pending());

        assert_eq!(
            h.handle(Mode::AnnotationList, char_key(' ')),
            Action::OpenAnnotationInspect
        );
        assert!(!h.has_pending());
    }

    #[test]
    fn annotation_inspect_navigation_and_dismissal() {
        let mut h = KeybindHandler::new();

        assert_eq!(h.handle_annotation_inspect(char_key('j')), Action::MoveDown);
        assert_eq!(h.handle_annotation_inspect(char_key('k')), Action::MoveUp);
        assert_eq!(
            h.handle_annotation_inspect(key(KeyCode::Down)),
            Action::ScrollOverlayDown
        );
        assert_eq!(
            h.handle_annotation_inspect(key(KeyCode::Up)),
            Action::ScrollOverlayUp
        );
        assert_eq!(
            h.handle_annotation_inspect(key(KeyCode::PageDown)),
            Action::ScrollOverlayPageDown
        );
        assert_eq!(
            h.handle_annotation_inspect(key(KeyCode::PageUp)),
            Action::ScrollOverlayPageUp
        );
        assert_eq!(
            h.handle_annotation_inspect(key(KeyCode::Enter)),
            Action::JumpToAnnotation
        );
        assert_eq!(
            h.handle_annotation_inspect(key(KeyCode::Esc)),
            Action::ExitToNormal
        );
        assert_eq!(h.handle_annotation_inspect(char_key('d')), Action::None);
        assert!(!h.has_pending());
    }

    #[test]
    fn annotation_list_tab_toggles() {
        let mut h = KeybindHandler::new();
        assert_eq!(
            h.handle(Mode::AnnotationList, key(KeyCode::Tab)),
            Action::EnterAnnotationListMode
        );
    }

    #[test]
    fn annotation_list_esc_exits() {
        let mut h = KeybindHandler::new();
        assert_eq!(
            h.handle(Mode::AnnotationList, key(KeyCode::Esc)),
            Action::HideAnnotationList
        );
    }

    #[test]
    fn normal_esc_hides_annotation_list() {
        let mut h = KeybindHandler::new();
        assert_eq!(
            h.handle(Mode::Normal, key(KeyCode::Esc)),
            Action::HideAnnotationList
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

    // ── Help overlay ──────────────────────────────────────────────

    #[test]
    fn help_overlay_j_returns_move_down() {
        let mut h = KeybindHandler::new();
        assert_eq!(
            h.handle_help_overlay(Mode::Normal, char_key('j')),
            Action::MoveDown
        );
    }

    #[test]
    fn help_overlay_down_arrow_returns_move_down() {
        let mut h = KeybindHandler::new();
        assert_eq!(
            h.handle_help_overlay(Mode::Normal, key(KeyCode::Down)),
            Action::MoveDown
        );
    }

    #[test]
    fn help_overlay_counted_j_returns_repeated_move_down() {
        let mut h = KeybindHandler::new();

        assert_eq!(
            h.handle_help_overlay(Mode::Normal, char_key('3')),
            Action::None
        );
        assert_eq!(
            h.handle_help_overlay(Mode::Normal, char_key('j')),
            repeated(Action::MoveDown, 3)
        );
    }

    #[test]
    fn help_overlay_k_returns_move_up() {
        let mut h = KeybindHandler::new();
        assert_eq!(
            h.handle_help_overlay(Mode::Normal, char_key('k')),
            Action::MoveUp
        );
    }

    #[test]
    fn help_overlay_up_arrow_returns_move_up() {
        let mut h = KeybindHandler::new();
        assert_eq!(
            h.handle_help_overlay(Mode::Normal, key(KeyCode::Up)),
            Action::MoveUp
        );
    }

    #[test]
    fn help_overlay_invalid_key_clears_pending_count() {
        let mut h = KeybindHandler::new();

        assert_eq!(
            h.handle_help_overlay(Mode::Normal, char_key('2')),
            Action::None
        );
        assert!(h.has_pending());

        assert_eq!(
            h.handle_help_overlay(Mode::Normal, char_key('x')),
            Action::None
        );
        assert!(!h.has_pending());
    }

    #[test]
    fn annotation_inspect_counted_navigation_and_scroll() {
        let mut h = KeybindHandler::new();

        assert_eq!(h.handle_annotation_inspect(char_key('2')), Action::None);
        assert_eq!(
            h.handle_annotation_inspect(char_key('j')),
            repeated(Action::MoveDown, 2)
        );

        assert_eq!(h.handle_annotation_inspect(char_key('4')), Action::None);
        assert_eq!(
            h.handle_annotation_inspect(key(KeyCode::PageDown)),
            repeated(Action::ScrollOverlayPageDown, 4)
        );
    }

    #[test]
    fn annotation_inspect_dismissal_clears_pending_count() {
        let mut h = KeybindHandler::new();

        assert_eq!(h.handle_annotation_inspect(char_key('2')), Action::None);
        assert!(h.has_pending());

        assert_eq!(
            h.handle_annotation_inspect(key(KeyCode::Esc)),
            Action::ExitToNormal
        );
        assert!(!h.has_pending());
    }
}
