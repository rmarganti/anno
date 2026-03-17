use uuid::Uuid;

/// The kind of annotation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AnnotationType {
    /// Mark selected text for removal.
    Deletion,
    /// Add a comment on selected text.
    Comment,
    /// Suggest replacement text for a selection.
    Replacement,
    /// Insert new text at a cursor position.
    Insertion,
    /// A general comment not tied to specific text.
    GlobalComment,
}

/// A single annotation attached to the document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Annotation {
    /// Unique identifier.
    pub id: Uuid,
    /// The block this annotation is anchored to (`None` for `GlobalComment`).
    pub block_id: Option<String>,
    /// Character offset of the selection start within the block.
    pub start_offset: usize,
    /// Character offset of the selection end within the block.
    pub end_offset: usize,
    /// The original selected text (empty for `Insertion` and `GlobalComment`).
    pub selected_text: String,
    /// The annotation type.
    pub annotation_type: AnnotationType,
    /// Associated text: comment body, replacement text, or insertion text.
    /// Empty for `Deletion`.
    pub text: String,
    /// Creation timestamp (milliseconds since UNIX epoch).
    pub timestamp: u128,
}

impl Annotation {
    /// Create a new annotation with a generated UUID and current timestamp.
    pub fn new(
        block_id: Option<String>,
        start_offset: usize,
        end_offset: usize,
        selected_text: String,
        annotation_type: AnnotationType,
        text: String,
    ) -> Self {
        use std::time::{SystemTime, UNIX_EPOCH};

        Self {
            id: Uuid::new_v4(),
            block_id,
            start_offset,
            end_offset,
            selected_text,
            annotation_type,
            text,
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis(),
        }
    }

    /// Create a `Deletion` annotation for selected text within a block.
    pub fn deletion(block_id: String, start: usize, end: usize, selected_text: String) -> Self {
        Self::new(
            Some(block_id),
            start,
            end,
            selected_text,
            AnnotationType::Deletion,
            String::new(),
        )
    }

    /// Create a `Comment` annotation on selected text within a block.
    pub fn comment(
        block_id: String,
        start: usize,
        end: usize,
        selected_text: String,
        comment: String,
    ) -> Self {
        Self::new(
            Some(block_id),
            start,
            end,
            selected_text,
            AnnotationType::Comment,
            comment,
        )
    }

    /// Create a `Replacement` annotation for selected text within a block.
    pub fn replacement(
        block_id: String,
        start: usize,
        end: usize,
        selected_text: String,
        replacement: String,
    ) -> Self {
        Self::new(
            Some(block_id),
            start,
            end,
            selected_text,
            AnnotationType::Replacement,
            replacement,
        )
    }

    /// Create an `Insertion` annotation at a cursor position within a block.
    pub fn insertion(block_id: String, offset: usize, text: String) -> Self {
        Self::new(
            Some(block_id),
            offset,
            offset,
            String::new(),
            AnnotationType::Insertion,
            text,
        )
    }

    /// Create a `GlobalComment` annotation (not anchored to any block).
    pub fn global_comment(comment: String) -> Self {
        Self::new(
            None,
            0,
            0,
            String::new(),
            AnnotationType::GlobalComment,
            comment,
        )
    }
}
