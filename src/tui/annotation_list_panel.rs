use ratatui::{
    Frame,
    layout::{Alignment, Rect},
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};
use uuid::Uuid;

use crate::annotation::store::AnnotationStore;
use crate::annotation::types::{Annotation, AnnotationType};
use crate::tui::theme::UiTheme;

/// Fixed width of the annotation list panel in columns.
pub const PANEL_WIDTH: u16 = 36;

const EMPTY_STATE_LINES: [&str; 4] = [
    "No annotations yet",
    "",
    "Select text and press d, c, or r",
    "to create an annotation.",
];

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

    /// Select a specific annotation by UUID.
    pub fn set_selected_annotation_id(&mut self, id: Uuid) {
        self.selected_id = Some(id);
    }

    /// Move the selection down by one in the ordered list.
    pub fn move_selection_down(&mut self, store: &AnnotationStore) {
        let ordered = store.ordered();
        if ordered.is_empty() {
            self.selected_id = None;
            return;
        }
        if self.selected_id.is_none() {
            self.selected_id = Some(ordered[0].id);
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
        if self.selected_id.is_none() {
            self.selected_id = Some(ordered[0].id);
            return;
        }
        let current_idx = self.resolve_index(&ordered);
        let next_idx = current_idx.saturating_sub(1);
        self.selected_id = Some(ordered[next_idx].id);
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
            let msg = Paragraph::new(
                EMPTY_STATE_LINES
                    .iter()
                    .map(|line| Line::from(*line))
                    .collect::<Vec<_>>(),
            )
            .alignment(Alignment::Center)
            .style(theme.panel);

            let message_height = EMPTY_STATE_LINES.len() as u16;
            let centered_area = Rect::new(
                inner.x,
                inner.y + inner.height.saturating_sub(message_height) / 2,
                inner.width,
                inner.height.min(message_height),
            );
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

            // Truncate preview to fit remaining width.
            // Used columns: 2 (indicator) + 1 (space) + glyph_width + 1 (space)
            let glyph_width = 1; // All our glyphs are single-width for layout purposes.
            let used = 2 + 1 + glyph_width + 1;
            let available = (inner.width as usize).saturating_sub(used);
            let preview = format_item_preview(annotation, available);
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

fn format_item_preview(annotation: &Annotation, available: usize) -> String {
    if available == 0 {
        return String::new();
    }

    let location = line_reference(annotation);
    let preview = sanitize_preview_text(preview_source(annotation));
    let content = if preview.is_empty() {
        location
    } else {
        format!("{location} {preview}")
    };

    truncate_with_ellipsis(&content, available)
}

fn preview_source(annotation: &Annotation) -> &str {
    match annotation.annotation_type {
        AnnotationType::Deletion => &annotation.selected_text,
        AnnotationType::Comment
        | AnnotationType::Replacement
        | AnnotationType::Insertion
        | AnnotationType::GlobalComment => &annotation.text,
    }
}

fn line_reference(annotation: &Annotation) -> String {
    match annotation.range {
        Some(range) => {
            let start_line = range.start.line + 1;
            let end_line = range.end.line + 1;

            if start_line == end_line {
                format!("L{start_line}")
            } else {
                format!("L{start_line}-{end_line}")
            }
        }
        None => String::from("global"),
    }
}

fn sanitize_preview_text(text: &str) -> String {
    text.chars().filter(|c| !c.is_control()).collect()
}

fn truncate_with_ellipsis(text: &str, width: usize) -> String {
    let char_count = text.chars().count();
    if char_count <= width {
        return text.to_string();
    }

    if width == 0 {
        return String::new();
    }

    if width == 1 {
        return String::from("…");
    }

    let truncated: String = text.chars().take(width - 1).collect();
    format!("{truncated}…")
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
    use crate::tui::theme::UiTheme;
    use ratatui::{Terminal, backend::TestBackend};

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

    fn render_to_lines(width: u16, height: u16, panel: &AnnotationListPanel) -> Vec<String> {
        render_store_to_lines(width, height, panel, &AnnotationStore::new())
    }

    fn render_store_to_lines(
        width: u16,
        height: u16,
        panel: &AnnotationListPanel,
        store: &AnnotationStore,
    ) -> Vec<String> {
        let backend = TestBackend::new(width, height);
        let mut terminal = Terminal::new(backend).unwrap();
        let theme = UiTheme::default();

        terminal
            .draw(|frame| {
                panel.render(frame, Rect::new(0, 0, width, height), store, &theme);
            })
            .unwrap();

        let buffer = terminal.backend().buffer().clone();
        (0..height)
            .map(|y| {
                (0..width)
                    .map(|x| {
                        buffer
                            .cell((x, y))
                            .map(|cell| cell.symbol().chars().next().unwrap_or(' '))
                            .unwrap_or(' ')
                    })
                    .collect::<String>()
                    .trim_end()
                    .to_string()
            })
            .collect()
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
        panel.set_selected_annotation_id(id);
        assert_eq!(panel.selected_annotation_id(), Some(id));
    }

    #[test]
    fn move_down_from_unselected_selects_first() {
        let (store, ids) = make_store_with_deletions(3);
        let mut panel = AnnotationListPanel::new();
        panel.move_selection_down(&store);
        assert_eq!(panel.selected_annotation_id(), Some(ids[0]));
    }

    #[test]
    fn move_up_from_unselected_selects_first() {
        let (store, ids) = make_store_with_deletions(3);
        let mut panel = AnnotationListPanel::new();
        panel.move_selection_up(&store);
        assert_eq!(panel.selected_annotation_id(), Some(ids[0]));
    }

    #[test]
    fn move_down_clamps_at_end() {
        let (store, ids) = make_store_with_deletions(3);
        let mut panel = AnnotationListPanel::new();
        panel.set_selected_annotation_id(ids[2]);
        panel.move_selection_down(&store);
        assert_eq!(panel.selected_annotation_id(), Some(ids[2]));
    }

    #[test]
    fn move_up_clamps_at_start() {
        let (store, ids) = make_store_with_deletions(3);
        let mut panel = AnnotationListPanel::new();
        panel.set_selected_annotation_id(ids[0]);
        panel.move_selection_up(&store);
        assert_eq!(panel.selected_annotation_id(), Some(ids[0]));
    }

    #[test]
    fn move_down_sequential() {
        let (store, ids) = make_store_with_deletions(3);
        let mut panel = AnnotationListPanel::new();
        panel.set_selected_annotation_id(ids[0]);
        panel.move_selection_down(&store);
        assert_eq!(panel.selected_annotation_id(), Some(ids[1]));
        panel.move_selection_down(&store);
        assert_eq!(panel.selected_annotation_id(), Some(ids[2]));
    }

    #[test]
    fn move_up_sequential() {
        let (store, ids) = make_store_with_deletions(3);
        let mut panel = AnnotationListPanel::new();
        panel.set_selected_annotation_id(ids[2]);
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
        panel.set_selected_annotation_id(ids[2]);
        store.delete(ids[2]);

        // Moving should clamp to the new last element.
        panel.move_selection_down(&store);
        assert_eq!(panel.selected_annotation_id(), Some(ids[1]));
    }

    #[test]
    fn deleted_middle_uuid_clamps_to_last() {
        let (mut store, ids) = make_store_with_deletions(3);
        let mut panel = AnnotationListPanel::new();
        panel.set_selected_annotation_id(ids[1]);
        store.delete(ids[1]);

        // resolve_index won't find ids[1], clamps to len-1 = 1 (which is ids[2] now).
        panel.move_selection_up(&store);
        assert_eq!(panel.selected_annotation_id(), Some(ids[0]));
    }

    #[test]
    fn all_deleted_empties_selection() {
        let (mut store, ids) = make_store_with_deletions(2);
        let mut panel = AnnotationListPanel::new();
        panel.set_selected_annotation_id(ids[0]);
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

    #[test]
    fn render_empty_state_shows_informative_message() {
        let panel = AnnotationListPanel::new();
        let output = render_to_lines(36, 10, &panel).join("\n");

        assert!(
            output.contains("No annotations yet"),
            "Expected empty-state title in: {output}"
        );
        assert!(
            output.contains("Select text and press d, c, or r"),
            "Expected empty-state guidance in: {output}"
        );
        assert!(
            output.contains("to create an annotation."),
            "Expected empty-state follow-up in: {output}"
        );
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

    #[test]
    fn preview_source_matches_annotation_type() {
        let anchored_range = range(1, 0, 1, 4);

        let deletion = Annotation::deletion(anchored_range, String::from("remove me"));
        let comment = Annotation::comment(
            anchored_range,
            String::from("selected"),
            String::from("comment body"),
        );
        let replacement = Annotation::replacement(
            anchored_range,
            String::from("selected"),
            String::from("replacement text"),
        );
        let insertion = Annotation::insertion(
            TextPosition { line: 2, column: 0 },
            String::from("inserted text"),
        );
        let global = Annotation::global_comment(String::from("global body"));

        assert_eq!(preview_source(&deletion), "remove me");
        assert_eq!(preview_source(&comment), "comment body");
        assert_eq!(preview_source(&replacement), "replacement text");
        assert_eq!(preview_source(&insertion), "inserted text");
        assert_eq!(preview_source(&global), "global body");
    }

    #[test]
    fn line_reference_uses_human_readable_line_numbers() {
        let single_line = Annotation::comment(
            range(0, 1, 0, 5),
            String::from("selected"),
            String::from("note"),
        );
        let multi_line = Annotation::replacement(
            range(1, 0, 3, 2),
            String::from("selected"),
            String::from("replace"),
        );

        assert_eq!(line_reference(&single_line), "L1");
        assert_eq!(line_reference(&multi_line), "L2-4");
    }

    #[test]
    fn global_comment_uses_distinct_non_anchored_label() {
        let global = Annotation::global_comment(String::from("project-wide note"));

        assert_eq!(line_reference(&global), "global");
    }

    #[test]
    fn preview_sanitization_strips_newlines_and_control_characters() {
        let annotation = Annotation::comment(
            range(0, 0, 0, 1),
            String::from("ignored"),
            String::from("line 1\nline 2\u{0007}"),
        );

        assert_eq!(format_item_preview(&annotation, 40), "L1 line 1line 2");
    }

    #[test]
    fn truncation_adds_ellipsis_when_preview_exceeds_width() {
        let annotation = Annotation::comment(
            range(0, 0, 0, 1),
            String::from("ignored"),
            String::from("abcdefghij"),
        );

        assert_eq!(format_item_preview(&annotation, 7), "L1 abc…");
        assert_eq!(format_item_preview(&annotation, 1), "…");
    }

    #[test]
    fn rendered_items_include_location_and_preview_text() {
        let mut store = AnnotationStore::new();
        store.add(Annotation::comment(
            range(2, 0, 2, 5),
            String::from("selected"),
            String::from("comment preview"),
        ));
        store.add(Annotation::global_comment(String::from("global preview")));
        let panel = AnnotationListPanel::new();

        let output = render_store_to_lines(36, 6, &panel, &store).join("\n");

        assert!(
            output.contains("L3 comment preview"),
            "Expected anchored preview in: {output}"
        );
        assert!(
            output.contains("global global preview"),
            "Expected global preview in: {output}"
        );
    }
}
