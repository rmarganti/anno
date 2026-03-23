use crate::highlight::{Highlighter, StyledSpan};
use crate::tui::renderer;

/// Pure document data: source name, plain-text lines, and highlighted lines.
pub struct Document {
    pub source_name: String,
    pub lines: Vec<String>,
    pub styled_lines: Vec<Vec<StyledSpan>>,
}

impl Document {
    pub fn new(source_name: String, content: &str, highlighter: &dyn Highlighter) -> Self {
        let result = renderer::text_to_lines(content, highlighter);
        Self {
            source_name,
            lines: result.plain,
            styled_lines: result.styled,
        }
    }
}
