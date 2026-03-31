/// The current input mode of the application.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Mode {
    #[default]
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
