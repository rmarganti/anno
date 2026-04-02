use super::core::AppState;
use crate::annotation::types::Annotation;

impl AppState {
    pub(super) fn jump_to_adjacent_annotation(&mut self, forward: bool) {
        let cursor = self.document_view.cursor();
        let cursor_pos = (cursor.row, cursor.col);
        let ordered = self.annotations.ordered();

        if ordered.is_empty() {
            return;
        }

        let current_idx = self.current_annotation_index(&ordered, cursor_pos, forward);

        let target = if forward {
            if let Some(current_idx) = current_idx {
                ordered
                    .iter()
                    .skip(current_idx + 1)
                    .find(|annotation| annotation.range.is_some())
            } else {
                ordered.iter().find(|annotation| {
                    annotation
                        .range
                        .map(|range| (range.start.line, range.start.column) > cursor_pos)
                        .unwrap_or(false)
                })
            }
        } else if let Some(current_idx) = current_idx {
            ordered[..current_idx]
                .iter()
                .rev()
                .find(|annotation| annotation.range.is_some())
        } else {
            ordered.iter().rev().find(|annotation| {
                annotation
                    .range
                    .map(|range| (range.start.line, range.start.column) < cursor_pos)
                    .unwrap_or(false)
            })
        };

        if let Some(annotation) = target
            && let Some(range) = annotation.range
        {
            self.annotation_list_panel
                .set_selected_annotation_id(annotation.id);
            self.document_view
                .set_cursor(range.start.line, range.start.column);
        }
    }

    fn current_annotation_index(
        &self,
        ordered: &[&Annotation],
        cursor_pos: (usize, usize),
        forward: bool,
    ) -> Option<usize> {
        if let Some(selected_id) = self.annotation_list_panel.selected_annotation_id()
            && let Some(index) = ordered
                .iter()
                .position(|annotation| annotation.id == selected_id)
            && ordered[index]
                .range
                .map(|range| (range.start.line, range.start.column) == cursor_pos)
                .unwrap_or(false)
        {
            return Some(index);
        }

        let matching_indices: Vec<_> = ordered
            .iter()
            .enumerate()
            .filter_map(|(index, annotation)| {
                annotation.range.and_then(|range| {
                    ((range.start.line, range.start.column) == cursor_pos).then_some(index)
                })
            })
            .collect();

        if forward {
            matching_indices.last().copied()
        } else {
            matching_indices.first().copied()
        }
    }
}
