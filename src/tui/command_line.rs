use super::app_command::AppCommand;

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
