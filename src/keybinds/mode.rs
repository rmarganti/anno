/// The current input mode of the application.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// Default navigation mode with vim-like movement.
    Normal,
    /// Text selection mode, entered with `v`.
    Visual,
    /// Text input mode for comment/replacement/insertion text entry.
    Insert,
    /// Focused on the annotation list sidebar.
    AnnotationList,
    /// Command-line input mode, entered with `:`.
    Command,
}

impl Default for Mode {
    fn default() -> Self {
        Self::Normal
    }
}
