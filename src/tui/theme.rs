use ratatui::style::{Color, Modifier, Style};

use crate::annotation::types::AnnotationType;

/// Centralized style definitions for the document renderer.
pub struct Theme {
    pub cursor: Style,
    pub selection_highlight: Style,
    pub annotation_highlight: Style,

    // Per-annotation-type colors.
    pub deletion_color: Color,
    pub comment_color: Color,
    pub replacement_color: Color,
    pub insertion_color: Color,
    pub global_comment_color: Color,

    // Panel styles.
    pub panel_bg: Color,
    pub panel_selected: Style,
    pub panel_border: Style,

    // Selected-annotation document highlight (subtle bg overlay).
    pub selected_annotation_highlight: Style,
}

impl Theme {
    pub fn new() -> Self {
        Self {
            cursor: Style::default().bg(Color::White).fg(Color::Black),
            selection_highlight: Style::default().bg(Color::Black).fg(Color::White),
            annotation_highlight: Style::default().add_modifier(Modifier::UNDERLINED),

            deletion_color: Color::Red,
            comment_color: Color::Yellow,
            replacement_color: Color::Blue,
            insertion_color: Color::Green,
            global_comment_color: Color::Magenta,

            panel_bg: Color::DarkGray,
            panel_selected: Style::default().bg(Color::DarkGray).fg(Color::White),
            panel_border: Style::default().fg(Color::Gray),

            selected_annotation_highlight: Style::default().bg(Color::DarkGray),
        }
    }

    /// Return the color associated with a given annotation type.
    pub fn annotation_type_color(&self, annotation_type: &AnnotationType) -> Color {
        match annotation_type {
            AnnotationType::Deletion => self.deletion_color,
            AnnotationType::Comment => self.comment_color,
            AnnotationType::Replacement => self.replacement_color,
            AnnotationType::Insertion => self.insertion_color,
            AnnotationType::GlobalComment => self.global_comment_color,
        }
    }
}

impl Default for Theme {
    fn default() -> Self {
        Self::new()
    }
}
