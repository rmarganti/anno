/// Commands that components can emit to request application-level actions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppCommand {
    /// Quit the application.
    Quit(QuitKind),
}

/// How the application should quit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QuitKind {
    /// Quit and print annotations to stdout.
    WithOutput,
    /// Quit without printing.
    Silent,
}
