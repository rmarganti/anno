use uuid::Uuid;

/// The kind of annotation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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

impl AnnotationType {
    /// Return the gutter rendering priority for this annotation type.
    ///
    /// Lower numbers indicate higher priority when multiple annotations share
    /// the same range.
    #[allow(dead_code)]
    pub fn priority(&self) -> u8 {
        match self {
            Self::Deletion => 0,
            Self::Replacement => 1,
            Self::Insertion => 2,
            Self::Comment => 3,
            Self::GlobalComment => 4,
        }
    }
}

/// A line/column position within the document.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct TextPosition {
    pub line: usize,
    pub column: usize,
}

/// A range of text within the document, defined by start and end positions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TextRange {
    pub start: TextPosition,
    pub end: TextPosition,
}

/// A typed annotation anchor used for document gutter rendering.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AnnotationIndicator {
    pub range: TextRange,
    pub annotation_type: AnnotationType,
}

/// A single annotation attached to the document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Annotation {
    /// Unique identifier.
    pub id: Uuid,
    /// The text range this annotation is anchored to (`None` for `GlobalComment`).
    pub range: Option<TextRange>,
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
        range: Option<TextRange>,
        selected_text: String,
        annotation_type: AnnotationType,
        text: String,
    ) -> Self {
        use std::time::{SystemTime, UNIX_EPOCH};

        Self {
            id: Uuid::new_v4(),
            range,
            selected_text,
            annotation_type,
            text,
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis(),
        }
    }

    /// Create a `Deletion` annotation for selected text.
    pub fn deletion(range: TextRange, selected_text: String) -> Self {
        Self::new(
            Some(range),
            selected_text,
            AnnotationType::Deletion,
            String::new(),
        )
    }

    /// Create a `Comment` annotation on selected text.
    pub fn comment(range: TextRange, selected_text: String, comment: String) -> Self {
        Self::new(Some(range), selected_text, AnnotationType::Comment, comment)
    }

    /// Create a `Replacement` annotation for selected text.
    pub fn replacement(range: TextRange, selected_text: String, replacement: String) -> Self {
        Self::new(
            Some(range),
            selected_text,
            AnnotationType::Replacement,
            replacement,
        )
    }

    /// Create an `Insertion` annotation at a cursor position.
    pub fn insertion(position: TextPosition, text: String) -> Self {
        Self::new(
            Some(TextRange {
                start: position,
                end: position,
            }),
            String::new(),
            AnnotationType::Insertion,
            text,
        )
    }

    /// Create a `GlobalComment` annotation (not anchored to any position).
    pub fn global_comment(comment: String) -> Self {
        Self::new(None, String::new(), AnnotationType::GlobalComment, comment)
    }
}

#[cfg(test)]
mod tests {
    use super::{AnnotationIndicator, AnnotationType, TextPosition, TextRange};

    #[test]
    fn annotation_type_priority_orders_gutter_precedence() {
        assert_eq!(AnnotationType::Deletion.priority(), 0);
        assert_eq!(AnnotationType::Replacement.priority(), 1);
        assert_eq!(AnnotationType::Insertion.priority(), 2);
        assert_eq!(AnnotationType::Comment.priority(), 3);
        assert_eq!(AnnotationType::GlobalComment.priority(), 4);
    }

    #[test]
    fn annotation_indicator_pairs_range_with_type() {
        let range = TextRange {
            start: TextPosition { line: 3, column: 4 },
            end: TextPosition { line: 3, column: 9 },
        };
        let indicator = AnnotationIndicator {
            range,
            annotation_type: AnnotationType::Replacement,
        };

        assert_eq!(indicator.range, range);
        assert_eq!(indicator.annotation_type, AnnotationType::Replacement);
    }
}
