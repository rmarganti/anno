/// The type of a parsed markdown block.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BlockType {
    Paragraph,
    Heading,
    Blockquote,
    ListItem,
    Code,
    HorizontalRule,
    Table,
}

/// A parsed markdown block with source-line tracking for annotation anchoring.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Block {
    /// Unique identifier for the block (e.g. "block-0", "block-1").
    pub id: String,
    /// The type of block.
    pub block_type: BlockType,
    /// The text content of the block.
    pub content: String,
    /// Indentation/heading level (heading level 1-6, list nesting depth).
    pub level: usize,
    /// The 1-based source line number where this block starts.
    pub start_line: usize,
    /// For code blocks, the optional language tag.
    pub language: Option<String>,
    /// For list items with checkbox syntax, whether the checkbox is checked.
    pub checked: Option<bool>,
}
