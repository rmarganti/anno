use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState},
    Frame,
};
use uuid::Uuid;

use crate::annotation::store::AnnotationStore;
use crate::annotation::types::AnnotationType;
use crate::keybinds::handler::Action;

/// Events emitted by [`AnnotationList`] after processing an action.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AnnotationListEvent {
    /// Jump to the annotation at the given document line.
    JumpTo { line: usize },
    /// The selected annotation was deleted.
    Delete { id: Uuid },
    /// Exit annotation list mode and return to Normal.
    Exit,
    /// Action was handled; no further processing needed.
    Consumed,
}

/// Sidebar component that lists all annotations and allows navigation.
///
/// Owns `selected_index` and `scroll_offset`. The parent is responsible
/// for rendering it in a sidebar area when `Mode::AnnotationList` is active.
pub struct AnnotationList {
    /// Index into the ordered annotation list that is currently highlighted.
    pub selected_index: usize,
    /// Number of items scrolled past at the top (for future long-list support).
    pub scroll_offset: usize,
}

impl AnnotationList {
    /// Create a new `AnnotationList` with the selection at the first item.
    pub fn new() -> Self {
        Self {
            selected_index: 0,
            scroll_offset: 0,
        }
    }

    /// Clamp `selected_index` to be within bounds of the current annotation count.
    ///
    /// Call this whenever the annotation store might have changed (e.g., after deletion).
    pub fn clamp(&mut self, annotation_count: usize) {
        if annotation_count == 0 {
            self.selected_index = 0;
        } else if self.selected_index >= annotation_count {
            self.selected_index = annotation_count - 1;
        }
    }

    /// Process an action and return the corresponding event.
    ///
    /// Requires the current annotation store to compute list bounds and resolve ids.
    pub fn handle_action(
        &mut self,
        action: &Action,
        store: &AnnotationStore,
    ) -> AnnotationListEvent {
        let ordered = store.ordered();
        let count = ordered.len();

        match action {
            Action::MoveDown => {
                if count > 0 && self.selected_index + 1 < count {
                    self.selected_index += 1;
                }
                AnnotationListEvent::Consumed
            }

            Action::MoveUp => {
                if self.selected_index > 0 {
                    self.selected_index -= 1;
                }
                AnnotationListEvent::Consumed
            }

            Action::JumpToAnnotation => {
                if let Some(annotation) = ordered.get(self.selected_index) {
                    if let Some(range) = annotation.range {
                        return AnnotationListEvent::JumpTo {
                            line: range.start.line,
                        };
                    }
                }
                // GlobalComment or empty list — nothing to jump to, just consume.
                AnnotationListEvent::Consumed
            }

            Action::DeleteAnnotation => {
                if let Some(annotation) = ordered.get(self.selected_index) {
                    let id = annotation.id;
                    // After deletion the list shrinks; clamp will be called by the parent.
                    return AnnotationListEvent::Delete { id };
                }
                AnnotationListEvent::Consumed
            }

            Action::ExitToNormal => AnnotationListEvent::Exit,

            _ => AnnotationListEvent::Consumed,
        }
    }

    /// Render the annotation list into the given sidebar area.
    pub fn render(&self, frame: &mut Frame, area: Rect, store: &AnnotationStore) {
        let ordered = store.ordered();

        let items: Vec<ListItem> = ordered
            .iter()
            .enumerate()
            .map(|(idx, annotation)| {
                let type_label = match annotation.annotation_type {
                    AnnotationType::Deletion => "DEL",
                    AnnotationType::Comment => "CMT",
                    AnnotationType::Replacement => "REP",
                    AnnotationType::Insertion => "INS",
                    AnnotationType::GlobalComment => "GCM",
                };

                let location = match annotation.range {
                    Some(r) => format!("{}:{}", r.start.line + 1, r.start.column + 1),
                    None => "global".to_string(),
                };

                // Show the annotation text; prefer the comment/replacement text,
                // fall back to selected_text, then a placeholder.
                let preview = if !annotation.text.is_empty() {
                    annotation.text.as_str()
                } else if !annotation.selected_text.is_empty() {
                    annotation.selected_text.as_str()
                } else {
                    "(empty)"
                };

                // Truncate preview so it fits reasonably in the sidebar.
                let max_preview = area.width.saturating_sub(14) as usize; // 14 for type + location
                let preview_truncated: String = preview.chars().take(max_preview).collect();
                let preview_display = if preview.chars().count() > max_preview {
                    format!("{preview_truncated}…")
                } else {
                    preview_truncated
                };

                let type_span = Span::styled(
                    format!(" {type_label} "),
                    Style::default()
                        .fg(Color::Black)
                        .bg(annotation_type_color(&annotation.annotation_type))
                        .add_modifier(Modifier::BOLD),
                );
                let loc_span = Span::styled(
                    format!(" {location} "),
                    Style::default().fg(Color::DarkGray),
                );
                let text_span = Span::raw(format!(" {preview_display}"));

                let line = Line::from(vec![type_span, loc_span, text_span]);

                let style = if idx == self.selected_index {
                    Style::default().bg(Color::DarkGray)
                } else {
                    Style::default()
                };

                ListItem::new(line).style(style)
            })
            .collect();

        let block = Block::default()
            .title(" Annotations ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Yellow));

        let list = List::new(items).block(block);

        let mut list_state = ListState::default();
        if !ordered.is_empty() {
            list_state.select(Some(self.selected_index));
        }

        frame.render_stateful_widget(list, area, &mut list_state);
    }
}

impl Default for AnnotationList {
    fn default() -> Self {
        Self::new()
    }
}

/// Return a color that identifies each annotation type visually.
fn annotation_type_color(annotation_type: &AnnotationType) -> Color {
    match annotation_type {
        AnnotationType::Deletion => Color::Red,
        AnnotationType::Comment => Color::Blue,
        AnnotationType::Replacement => Color::Magenta,
        AnnotationType::Insertion => Color::Green,
        AnnotationType::GlobalComment => Color::Cyan,
    }
}
