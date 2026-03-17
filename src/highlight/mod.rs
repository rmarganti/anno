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
    /// Highlight a single line of markdown prose, returning styled spans for
    /// inline formatting (bold, italic, inline code, links).
    fn highlight_line(&self, line: &str) -> Vec<StyledSpan>;

    /// Highlight a code block's content with language-specific grammars.
    /// `language` is the optional fence language tag (e.g. "rust", "python").
    fn highlight_code_block(&self, code: &str, language: Option<&str>) -> Vec<Vec<StyledSpan>>;
}
