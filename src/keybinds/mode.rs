/// The current input mode of the application.
#[cfg_attr(not(test), allow(dead_code))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Mode {
    #[default]
    Normal,
    /// Text selection mode, entered with `v`.
    Visual,
    /// Linewise selection mode, entered with `V`.
    VisualLine,
    /// Text input mode for comment/replacement/insertion text entry.
    Insert,
    /// Focused on the annotation list sidebar.
    AnnotationList,
    /// Command-line input mode, entered with `:`.
    Command,
    /// Search input mode for `/` and `?` pattern search.
    Search,
}
