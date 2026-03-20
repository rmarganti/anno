pub mod syntect;

use ratatui::style::Style;

/// A styled span of text — the output unit of highlighting.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StyledSpan {
    pub text: String,
    pub style: Style,
}

impl StyledSpan {
    pub fn new(text: impl Into<String>, style: Style) -> Self {
        Self {
            text: text.into(),
            style,
        }
    }

    pub fn plain(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            style: Style::default(),
        }
    }
}

/// Trait abstracting syntax highlighting so the backend (syntect, tree-sitter, etc.)
/// can be swapped without touching rendering code.
pub trait Highlighter {
    /// Highlight an entire document, returning one `Vec<StyledSpan>` per line.
    fn highlight_document(&self, lines: &[String]) -> Vec<Vec<StyledSpan>>;
}
