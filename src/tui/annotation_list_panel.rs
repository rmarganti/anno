use ratatui::{
    Frame,
    layout::{Alignment, Rect},
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};
use uuid::Uuid;

use crate::annotation::store::AnnotationStore;
use crate::annotation::types::AnnotationType;
use crate::tui::theme::UiTheme;

/// Fixed width of the annotation list panel in columns.
pub const PANEL_WIDTH: u16 = 36;

/// The annotation list sidebar panel widget.
#[derive(Debug)]
pub struct AnnotationListPanel {
    visible: bool,
    selected_id: Option<Uuid>,
}

impl AnnotationListPanel {
    pub fn new() -> Self {
        Self {
            visible: false,
            selected_id: None,
        }
    }

    /// Toggle panel visibility.
    pub fn toggle(&mut self) {
        self.visible = !self.visible;
    }

    /// Whether the panel is currently visible.
    pub fn is_visible(&self) -> bool {
        self.visible
    }

    /// Return the UUID of the currently selected annotation, if any.
    pub fn selected_annotation_id(&self) -> Option<Uuid> {
        self.selected_id
    }

    /// Move the selection down by one in the ordered list.
    pub fn move_selection_down(&mut self, store: &AnnotationStore) {
        let ordered = store.ordered();
        if ordered.is_empty() {
            self.selected_id = None;
            return;
        }
        let current_idx = self.resolve_index(&ordered);
        let next_idx = (current_idx + 1).min(ordered.len() - 1);
        self.selected_id = Some(ordered[next_idx].id);
    }

    /// Move the selection up by one in the ordered list.
    pub fn move_selection_up(&mut self, store: &AnnotationStore) {
        let ordered = store.ordered();
        if ordered.is_empty() {
            self.selected_id = None;
            return;
        }
        let current_idx = self.resolve_index(&ordered);
        let next_idx = current_idx.saturating_sub(1);
        self.selected_id = Some(ordered[next_idx].id);
    }

    /// Select a specific annotation by UUID.
    #[cfg(test)]
    pub fn select_by_id(&mut self, id: Uuid) {
        self.selected_id = Some(id);
    }

    /// Render the panel into the given area.
    pub fn render(&self, frame: &mut Frame, area: Rect, store: &AnnotationStore, theme: &UiTheme) {
        let block = Block::default()
            .borders(Borders::LEFT)
            .style(theme.panel)
            .border_style(theme.panel_border);
        let inner = vertical_padding(block.inner(area), 1);
        frame.render_widget(block, area);

        let ordered = store.ordered();

        if ordered.is_empty() {
            let msg = Paragraph::new("No annotations")
                .alignment(Alignment::Center)
                .style(theme.panel);
            // Center vertically.
            let y = inner.y + inner.height / 2;
            let centered_area = Rect::new(inner.x, y, inner.width, 1);
            frame.render_widget(msg, centered_area);
            return;
        }

        let current_idx = self.resolve_index(&ordered);

        for (i, annotation) in ordered.iter().enumerate() {
            if i as u16 >= inner.height {
                break;
            }

            let is_selected = i == current_idx;
            let base_style = if is_selected {
                theme.panel_selected
            } else {
                theme.panel
            };

            let type_color = theme.annotation_type_color(&annotation.annotation_type);
            let glyph = type_glyph(&annotation.annotation_type);

            // Build the line: "██ ✕ preview text..."
            // Indicator (2 chars) + space + glyph + space + preview
            let indicator = Span::styled(
                "██",
                Style::default()
                    .fg(type_color)
                    .bg(base_style.bg.unwrap_or(theme.panel.bg.unwrap_or_default())),
            );
            let spacer = Span::styled(" ", base_style);
            let glyph_span = Span::styled(
                glyph,
                Style::default()
                    .fg(type_color)
                    .bg(base_style.bg.unwrap_or(theme.panel.bg.unwrap_or_default())),
            );
            let spacer2 = Span::styled(" ", base_style);

            // Preview text: use annotation.text if non-empty, else selected_text, else type name.
            let preview_source = if !annotation.text.is_empty() {
                &annotation.text
            } else if !annotation.selected_text.is_empty() {
                &annotation.selected_text
            } else {
                glyph
            };

            // Truncate preview to fit remaining width.
            // Used columns: 2 (indicator) + 1 (space) + glyph_width + 1 (space)
            let glyph_width = 1; // All our glyphs are single-width for layout purposes.
            let used = 2 + 1 + glyph_width + 1;
            let available = (inner.width as usize).saturating_sub(used);
            let preview: String = preview_source
                .chars()
                .filter(|c| !c.is_control())
                .take(available)
                .collect();
            // Pad to fill remaining space.
            let padded = format!("{preview:<available$}");
            let preview_span = Span::styled(padded, base_style);

            let line = Line::from(vec![indicator, spacer, glyph_span, spacer2, preview_span]);
            let line_area = Rect::new(inner.x, inner.y + i as u16, inner.width, 1);
            frame.render_widget(Paragraph::new(line), line_area);
        }
    }

    /// Resolve `selected_id` to an index in the ordered list.
    /// If the selected UUID is gone (deleted), clamp to the nearest valid index.
    /// If nothing is selected, defaults to 0.
    fn resolve_index(&self, ordered: &[&crate::annotation::types::Annotation]) -> usize {
        if ordered.is_empty() {
            return 0;
        }

        match self.selected_id {
            Some(id) => {
                // Try to find the UUID in the ordered list.
                if let Some(idx) = ordered.iter().position(|a| a.id == id) {
                    idx
                } else {
                    // UUID not found — clamp to last valid index.
                    ordered.len() - 1
                }
            }
            None => 0,
        }
    }
}

fn vertical_padding(area: Rect, padding: u16) -> Rect {
    let total_padding = padding.saturating_mul(2);
    if area.height <= total_padding {
        Rect::new(area.x, area.y, area.width, 0)
    } else {
        Rect::new(
            area.x,
            area.y.saturating_add(padding),
            area.width,
            area.height - total_padding,
        )
    }
}

impl Default for AnnotationListPanel {
    fn default() -> Self {
        Self::new()
    }
}

/// Return a Unicode glyph for each annotation type.
fn type_glyph(annotation_type: &AnnotationType) -> &'static str {
    match annotation_type {
        AnnotationType::Deletion => "✕",
        AnnotationType::Comment => "▸",
        AnnotationType::Replacement => "⇄",
        AnnotationType::Insertion => "+",
        AnnotationType::GlobalComment => "◆",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::annotation::types::{Annotation, TextPosition, TextRange};

    fn range(sl: usize, sc: usize, el: usize, ec: usize) -> TextRange {
        TextRange {
            start: TextPosition {
                line: sl,
                column: sc,
            },
            end: TextPosition {
                line: el,
                column: ec,
            },
        }
    }

    fn make_store_with_deletions(n: usize) -> (AnnotationStore, Vec<Uuid>) {
        let mut store = AnnotationStore::new();
        let mut ids = Vec::new();
        for i in 0..n {
            let ann = Annotation::deletion(range(i, 0, i, 5), format!("del{i}"));
            ids.push(ann.id);
            store.add(ann);
        }
        (store, ids)
    }

    // ───── visibility ─────

    #[test]
    fn starts_hidden() {
        let panel = AnnotationListPanel::new();
        assert!(!panel.is_visible());
    }

    #[test]
    fn toggle_visibility() {
        let mut panel = AnnotationListPanel::new();
        panel.toggle();
        assert!(panel.is_visible());
        panel.toggle();
        assert!(!panel.is_visible());
    }

    // ───── selection tracking ─────

    #[test]
    fn no_selection_initially() {
        let panel = AnnotationListPanel::new();
        assert!(panel.selected_annotation_id().is_none());
    }

    #[test]
    fn select_by_id() {
        let mut panel = AnnotationListPanel::new();
        let id = Uuid::new_v4();
        panel.select_by_id(id);
        assert_eq!(panel.selected_annotation_id(), Some(id));
    }

    #[test]
    fn move_down_from_unselected_selects_first() {
        let (store, ids) = make_store_with_deletions(3);
        let mut panel = AnnotationListPanel::new();
        panel.move_selection_down(&store);
        // With no prior selection, resolve_index returns 0, then down goes to 1.
        assert_eq!(panel.selected_annotation_id(), Some(ids[1]));
    }

    #[test]
    fn move_up_from_unselected_selects_first() {
        let (store, ids) = make_store_with_deletions(3);
        let mut panel = AnnotationListPanel::new();
        panel.move_selection_up(&store);
        // resolve_index returns 0, saturating_sub(1) = 0.
        assert_eq!(panel.selected_annotation_id(), Some(ids[0]));
    }

    #[test]
    fn move_down_clamps_at_end() {
        let (store, ids) = make_store_with_deletions(3);
        let mut panel = AnnotationListPanel::new();
        panel.select_by_id(ids[2]);
        panel.move_selection_down(&store);
        assert_eq!(panel.selected_annotation_id(), Some(ids[2]));
    }

    #[test]
    fn move_up_clamps_at_start() {
        let (store, ids) = make_store_with_deletions(3);
        let mut panel = AnnotationListPanel::new();
        panel.select_by_id(ids[0]);
        panel.move_selection_up(&store);
        assert_eq!(panel.selected_annotation_id(), Some(ids[0]));
    }

    #[test]
    fn move_down_sequential() {
        let (store, ids) = make_store_with_deletions(3);
        let mut panel = AnnotationListPanel::new();
        panel.select_by_id(ids[0]);
        panel.move_selection_down(&store);
        assert_eq!(panel.selected_annotation_id(), Some(ids[1]));
        panel.move_selection_down(&store);
        assert_eq!(panel.selected_annotation_id(), Some(ids[2]));
    }

    #[test]
    fn move_up_sequential() {
        let (store, ids) = make_store_with_deletions(3);
        let mut panel = AnnotationListPanel::new();
        panel.select_by_id(ids[2]);
        panel.move_selection_up(&store);
        assert_eq!(panel.selected_annotation_id(), Some(ids[1]));
        panel.move_selection_up(&store);
        assert_eq!(panel.selected_annotation_id(), Some(ids[0]));
    }

    // ───── UUID-based clamping on deletion ─────

    #[test]
    fn deleted_uuid_clamps_to_last() {
        let (mut store, ids) = make_store_with_deletions(3);
        let mut panel = AnnotationListPanel::new();
        // Select the last item, then delete it.
        panel.select_by_id(ids[2]);
        store.delete(ids[2]);

        // Moving should clamp to the new last element.
        panel.move_selection_down(&store);
        assert_eq!(panel.selected_annotation_id(), Some(ids[1]));
    }

    #[test]
    fn deleted_middle_uuid_clamps_to_last() {
        let (mut store, ids) = make_store_with_deletions(3);
        let mut panel = AnnotationListPanel::new();
        panel.select_by_id(ids[1]);
        store.delete(ids[1]);

        // resolve_index won't find ids[1], clamps to len-1 = 1 (which is ids[2] now).
        panel.move_selection_up(&store);
        assert_eq!(panel.selected_annotation_id(), Some(ids[0]));
    }

    #[test]
    fn all_deleted_empties_selection() {
        let (mut store, ids) = make_store_with_deletions(2);
        let mut panel = AnnotationListPanel::new();
        panel.select_by_id(ids[0]);
        store.delete(ids[0]);
        store.delete(ids[1]);

        panel.move_selection_down(&store);
        assert!(panel.selected_annotation_id().is_none());
    }

    #[test]
    fn empty_store_move_does_nothing() {
        let store = AnnotationStore::new();
        let mut panel = AnnotationListPanel::new();
        panel.move_selection_down(&store);
        assert!(panel.selected_annotation_id().is_none());
        panel.move_selection_up(&store);
        assert!(panel.selected_annotation_id().is_none());
    }

    // ───── type glyphs ─────

    #[test]
    fn all_types_have_glyphs() {
        assert_eq!(type_glyph(&AnnotationType::Deletion), "✕");
        assert_eq!(type_glyph(&AnnotationType::Comment), "▸");
        assert_eq!(type_glyph(&AnnotationType::Replacement), "⇄");
        assert_eq!(type_glyph(&AnnotationType::Insertion), "+");
        assert_eq!(type_glyph(&AnnotationType::GlobalComment), "◆");
    }
}
