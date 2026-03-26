use super::app_command::{AppCommand, QuitKind};

/// A component that manages the command-mode input buffer (`:` commands).
///
/// Handles character input, backspace, and confirmation, returning an
/// `Option<AppCommand>` when a command is confirmed.
pub struct CommandLine {
    buffer: String,
}

/// The result of handling a command-mode action.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommandLineEvent {
    /// The command was confirmed and produced an `AppCommand`.
    Command(AppCommand),
    /// Backspace emptied the buffer — the caller should exit command mode.
    ExitToNormal,
    /// The input was consumed (character added or backspace with remaining text).
    Consumed,
}

impl CommandLine {
    pub fn new() -> Self {
        Self {
            buffer: String::new(),
        }
    }

    /// Returns the current buffer contents (for display in the status bar).
    pub fn buffer(&self) -> &str {
        &self.buffer
    }

    /// Handle a character typed in command mode.
    pub fn handle_char(&mut self, c: char) -> CommandLineEvent {
        self.buffer.push(c);
        CommandLineEvent::Consumed
    }

    /// Handle backspace in command mode.
    pub fn handle_backspace(&mut self) -> CommandLineEvent {
        self.buffer.pop();
        if self.buffer.is_empty() {
            CommandLineEvent::ExitToNormal
        } else {
            CommandLineEvent::Consumed
        }
    }

    /// Handle Enter (confirm) in command mode. Parses the buffer into an
    /// `AppCommand` and resets the buffer.
    pub fn handle_confirm(&mut self) -> CommandLineEvent {
        let cmd = match self.buffer.as_str() {
            "q" => AppCommand::Quit(QuitKind::WithOutput),
            "q!" => AppCommand::Quit(QuitKind::Silent),
            _ => {
                self.buffer.clear();
                return CommandLineEvent::ExitToNormal;
            }
        };
        self.buffer.clear();
        CommandLineEvent::Command(cmd)
    }

    /// Clear the buffer (e.g. when exiting command mode via Esc).
    pub fn clear(&mut self) {
        self.buffer.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn char_appends_to_buffer() {
        let mut cl = CommandLine::new();
        assert_eq!(cl.handle_char('q'), CommandLineEvent::Consumed);
        assert_eq!(cl.buffer(), "q");
        cl.handle_char('!');
        assert_eq!(cl.buffer(), "q!");
    }

    #[test]
    fn backspace_removes_last_char() {
        let mut cl = CommandLine::new();
        cl.handle_char('q');
        cl.handle_char('!');
        assert_eq!(cl.handle_backspace(), CommandLineEvent::Consumed);
        assert_eq!(cl.buffer(), "q");
    }

    #[test]
    fn backspace_on_empty_exits_to_normal() {
        let mut cl = CommandLine::new();
        cl.handle_char('a');
        assert_eq!(cl.handle_backspace(), CommandLineEvent::ExitToNormal);
        assert_eq!(cl.buffer(), "");
    }

    #[test]
    fn confirm_q_returns_quit_with_output() {
        let mut cl = CommandLine::new();
        cl.handle_char('q');
        assert_eq!(
            cl.handle_confirm(),
            CommandLineEvent::Command(AppCommand::Quit(QuitKind::WithOutput))
        );
        assert_eq!(cl.buffer(), "");
    }

    #[test]
    fn confirm_q_bang_returns_quit_silent() {
        let mut cl = CommandLine::new();
        cl.handle_char('q');
        cl.handle_char('!');
        assert_eq!(
            cl.handle_confirm(),
            CommandLineEvent::Command(AppCommand::Quit(QuitKind::Silent))
        );
    }

    #[test]
    fn confirm_unknown_command_exits_to_normal() {
        let mut cl = CommandLine::new();
        cl.handle_char('x');
        assert_eq!(cl.handle_confirm(), CommandLineEvent::ExitToNormal);
        assert_eq!(cl.buffer(), "");
    }

    #[test]
    fn clear_empties_buffer() {
        let mut cl = CommandLine::new();
        cl.handle_char('q');
        cl.clear();
        assert_eq!(cl.buffer(), "");
    }
}
