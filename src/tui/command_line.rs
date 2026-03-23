use crossterm::event::KeyEvent;

use crate::keybinds::handler::{Action, KeybindHandler};
use crate::keybinds::mode::Mode;

/// The result of a confirmed command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommandResult {
    /// `:q` — quit and emit annotations.
    Quit,
    /// `:q!` — quit silently.
    QuitForce,
    /// `:w` — write/emit annotations without quitting.
    Write,
    /// Unknown command — ignored.
    Unknown,
}

/// Events emitted by [`CommandLine`] after processing a key.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommandLineEvent {
    /// A printable character was appended to the buffer.
    Consumed,
    /// The buffer was cleared by a backspace that emptied it; caller should exit command mode.
    ExitToNormal,
    /// The user pressed Enter; the command was executed.
    Executed(CommandResult),
    /// Escape was pressed; caller should exit command mode without executing.
    Cancelled,
}

/// Manages the command-mode input buffer and command execution.
///
/// `CommandLine` owns the buffer of characters typed after `:`. The caller is
/// responsible for tracking which `Mode` is active; `CommandLine` only processes
/// key events forwarded to it while command mode is active.
pub struct CommandLine {
    /// Characters typed after the leading `:`.
    pub buffer: String,
}

impl CommandLine {
    pub fn new() -> Self {
        Self {
            buffer: String::new(),
        }
    }

    /// Clear the buffer. Call this when entering command mode.
    pub fn clear(&mut self) {
        self.buffer.clear();
    }

    /// Process a raw key event while in command mode.
    ///
    /// The handler uses a fresh `KeybindHandler` in `Command` mode to convert
    /// the raw key event to an `Action`, then delegates to `handle_action`.
    #[allow(dead_code)]
    pub fn handle_key(&mut self, key_event: KeyEvent) -> CommandLineEvent {
        let mut handler = KeybindHandler::new();
        let action = handler.handle(Mode::Command, key_event);
        self.handle_action(&action)
    }

    /// Process a pre-dispatched `Action` while in command mode.
    pub fn handle_action(&mut self, action: &Action) -> CommandLineEvent {
        match action {
            Action::CommandChar(c) => {
                self.buffer.push(*c);
                CommandLineEvent::Consumed
            }
            Action::CommandBackspace => {
                self.buffer.pop();
                if self.buffer.is_empty() {
                    CommandLineEvent::ExitToNormal
                } else {
                    CommandLineEvent::Consumed
                }
            }
            Action::CommandConfirm => {
                let result = Self::parse_command(&self.buffer);
                self.buffer.clear();
                CommandLineEvent::Executed(result)
            }
            Action::ExitToNormal => {
                self.buffer.clear();
                CommandLineEvent::Cancelled
            }
            _ => CommandLineEvent::Consumed,
        }
    }

    /// Parse the current buffer into a `CommandResult`.
    fn parse_command(buffer: &str) -> CommandResult {
        match buffer {
            "q" => CommandResult::Quit,
            "q!" => CommandResult::QuitForce,
            "w" => CommandResult::Write,
            _ => CommandResult::Unknown,
        }
    }
}

impl Default for CommandLine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyCode, KeyModifiers};

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    fn char_key(c: char) -> KeyEvent {
        key(KeyCode::Char(c))
    }

    // ── buffer management ────────────────────────────────────────

    #[test]
    fn starts_empty() {
        let cl = CommandLine::new();
        assert!(cl.buffer.is_empty());
    }

    #[test]
    fn clear_resets_buffer() {
        let mut cl = CommandLine::new();
        cl.buffer.push_str("hello");
        cl.clear();
        assert!(cl.buffer.is_empty());
    }

    #[test]
    fn typing_chars_fills_buffer() {
        let mut cl = CommandLine::new();
        assert_eq!(cl.handle_key(char_key('q')), CommandLineEvent::Consumed);
        assert_eq!(cl.handle_key(char_key('!')), CommandLineEvent::Consumed);
        assert_eq!(cl.buffer, "q!");
    }

    #[test]
    fn backspace_removes_last_char() {
        let mut cl = CommandLine::new();
        cl.buffer.push_str("qu");
        assert_eq!(
            cl.handle_key(key(KeyCode::Backspace)),
            CommandLineEvent::Consumed
        );
        assert_eq!(cl.buffer, "q");
    }

    #[test]
    fn backspace_on_last_char_emits_exit() {
        let mut cl = CommandLine::new();
        cl.buffer.push('q');
        assert_eq!(
            cl.handle_key(key(KeyCode::Backspace)),
            CommandLineEvent::ExitToNormal
        );
        assert!(cl.buffer.is_empty());
    }

    #[test]
    fn esc_emits_cancelled_and_clears() {
        let mut cl = CommandLine::new();
        cl.buffer.push_str("partial");
        assert_eq!(
            cl.handle_key(key(KeyCode::Esc)),
            CommandLineEvent::Cancelled
        );
        assert!(cl.buffer.is_empty());
    }

    // ── command parsing ──────────────────────────────────────────

    #[test]
    fn enter_on_q_emits_quit() {
        let mut cl = CommandLine::new();
        cl.buffer.push('q');
        assert_eq!(
            cl.handle_key(key(KeyCode::Enter)),
            CommandLineEvent::Executed(CommandResult::Quit)
        );
        assert!(cl.buffer.is_empty());
    }

    #[test]
    fn enter_on_q_bang_emits_quit_force() {
        let mut cl = CommandLine::new();
        cl.buffer.push_str("q!");
        assert_eq!(
            cl.handle_key(key(KeyCode::Enter)),
            CommandLineEvent::Executed(CommandResult::QuitForce)
        );
    }

    #[test]
    fn enter_on_w_emits_write() {
        let mut cl = CommandLine::new();
        cl.buffer.push('w');
        assert_eq!(
            cl.handle_key(key(KeyCode::Enter)),
            CommandLineEvent::Executed(CommandResult::Write)
        );
    }

    #[test]
    fn enter_on_unknown_emits_unknown() {
        let mut cl = CommandLine::new();
        cl.buffer.push_str("xyz");
        assert_eq!(
            cl.handle_key(key(KeyCode::Enter)),
            CommandLineEvent::Executed(CommandResult::Unknown)
        );
    }

    #[test]
    fn enter_clears_buffer() {
        let mut cl = CommandLine::new();
        cl.buffer.push('w');
        cl.handle_key(key(KeyCode::Enter));
        assert!(cl.buffer.is_empty());
    }
}
