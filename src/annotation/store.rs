use uuid::Uuid;

use super::types::Annotation;

/// Collection of annotations with CRUD operations and position-based ordering.
#[derive(Debug, Default)]
pub struct AnnotationStore {
    annotations: Vec<Annotation>,
}

impl AnnotationStore {
    /// Create an empty store.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add an annotation and return its id.
    pub fn add(&mut self, annotation: Annotation) -> Uuid {
        let id = annotation.id;
        self.annotations.push(annotation);
        id
    }

    /// Delete an annotation by its id. Returns `true` if it was found and removed.
    pub fn delete(&mut self, id: Uuid) -> bool {
        let len_before = self.annotations.len();
        self.annotations.retain(|a| a.id != id);
        self.annotations.len() < len_before
    }

    /// Return the number of annotations.
    pub fn len(&self) -> usize {
        self.annotations.len()
    }

    /// Return whether the store is empty.
    pub fn is_empty(&self) -> bool {
        self.annotations.is_empty()
    }

    /// Return annotations ordered by position: block id (lexicographic),
    /// then start offset, then end offset, then creation timestamp.
    /// `GlobalComment` annotations (no block id) are sorted to the end.
    pub fn ordered(&self) -> Vec<&Annotation> {
        let mut refs: Vec<&Annotation> = self.annotations.iter().collect();
        refs.sort_by(|a, b| {
            let a_key = (
                a.block_id.as_deref(),
                a.start_offset,
                a.end_offset,
                a.timestamp,
            );
            let b_key = (
                b.block_id.as_deref(),
                b.start_offset,
                b.end_offset,
                b.timestamp,
            );
            // `None` (GlobalComment) sorts after `Some(_)` because `None < Some(_)` in Rust,
            // so we reverse that by mapping: Some(x) → (0, Some(x)), None → (1, None).
            let a_sort = (if a_key.0.is_some() { 0 } else { 1 }, a_key);
            let b_sort = (if b_key.0.is_some() { 0 } else { 1 }, b_key);
            a_sort.cmp(&b_sort)
        });
        refs
    }

    /// Get an annotation by id.
    pub fn get(&self, id: Uuid) -> Option<&Annotation> {
        self.annotations.iter().find(|a| a.id == id)
    }

    /// Return all annotations as an unordered slice.
    pub fn all(&self) -> &[Annotation] {
        &self.annotations
    }

    /// Return all annotations that overlap the given block and character range.
    pub fn overlapping(&self, block_id: &str, start: usize, end: usize) -> Vec<&Annotation> {
        self.annotations
            .iter()
            .filter(|a| {
                a.block_id.as_deref() == Some(block_id)
                    && a.start_offset < end
                    && a.end_offset > start
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::annotation::types::{Annotation, AnnotationType};

    // ───── helpers ─────

    fn deletion(block_id: &str, start: usize, end: usize) -> Annotation {
        Annotation::deletion(block_id.to_string(), start, end, "x".repeat(end - start))
    }

    fn comment(block_id: &str, start: usize, end: usize) -> Annotation {
        Annotation::comment(
            block_id.to_string(),
            start,
            end,
            "sel".into(),
            "note".into(),
        )
    }

    fn global() -> Annotation {
        Annotation::global_comment("global note".into())
    }

    // ───── CRUD ─────

    #[test]
    fn add_and_retrieve() {
        let mut store = AnnotationStore::new();
        let ann = deletion("block-0", 0, 5);
        let id = ann.id;
        store.add(ann);

        assert_eq!(store.len(), 1);
        assert!(!store.is_empty());
        assert!(store.get(id).is_some());
        assert_eq!(store.get(id).unwrap().annotation_type, AnnotationType::Deletion);
    }

    #[test]
    fn delete_existing() {
        let mut store = AnnotationStore::new();
        let ann = deletion("block-0", 0, 5);
        let id = ann.id;
        store.add(ann);

        assert!(store.delete(id));
        assert!(store.is_empty());
        assert!(store.get(id).is_none());
    }

    #[test]
    fn delete_nonexistent_returns_false() {
        let mut store = AnnotationStore::new();
        assert!(!store.delete(Uuid::new_v4()));
    }

    #[test]
    fn len_and_is_empty() {
        let mut store = AnnotationStore::new();
        assert!(store.is_empty());
        assert_eq!(store.len(), 0);

        store.add(deletion("block-0", 0, 1));
        store.add(deletion("block-0", 1, 2));
        assert_eq!(store.len(), 2);
        assert!(!store.is_empty());
    }

    // ───── Ordering ─────

    #[test]
    fn ordered_by_block_then_offset() {
        let mut store = AnnotationStore::new();
        store.add(deletion("block-1", 10, 20));
        store.add(deletion("block-0", 5, 10));
        store.add(deletion("block-0", 0, 5));

        let ordered = store.ordered();
        assert_eq!(ordered[0].block_id.as_deref(), Some("block-0"));
        assert_eq!(ordered[0].start_offset, 0);
        assert_eq!(ordered[1].block_id.as_deref(), Some("block-0"));
        assert_eq!(ordered[1].start_offset, 5);
        assert_eq!(ordered[2].block_id.as_deref(), Some("block-1"));
    }

    #[test]
    fn global_comments_sort_last() {
        let mut store = AnnotationStore::new();
        store.add(global());
        store.add(deletion("block-0", 0, 5));
        store.add(global());

        let ordered = store.ordered();
        // block-anchored annotation first, then global comments
        assert!(ordered[0].block_id.is_some());
        assert!(ordered[1].block_id.is_none());
        assert!(ordered[2].block_id.is_none());
    }

    // ───── Overlapping ─────

    #[test]
    fn overlapping_finds_intersecting_annotations() {
        let mut store = AnnotationStore::new();
        store.add(deletion("block-0", 0, 10));
        store.add(deletion("block-0", 5, 15));
        store.add(deletion("block-0", 20, 30));
        store.add(deletion("block-1", 0, 10));

        // Query range [3, 12) in block-0 should match the first two.
        let hits = store.overlapping("block-0", 3, 12);
        assert_eq!(hits.len(), 2);
    }

    #[test]
    fn overlapping_no_match_when_adjacent() {
        let mut store = AnnotationStore::new();
        store.add(deletion("block-0", 0, 5));

        // Range [5, 10) starts exactly where the annotation ends — no overlap.
        let hits = store.overlapping("block-0", 5, 10);
        assert!(hits.is_empty());
    }

    // ───── Edge cases ─────

    #[test]
    fn empty_selection_annotation() {
        let mut store = AnnotationStore::new();
        let ann = Annotation::insertion("block-0".into(), 5, "inserted".into());
        let id = ann.id;
        store.add(ann);

        let a = store.get(id).unwrap();
        assert_eq!(a.start_offset, 5);
        assert_eq!(a.end_offset, 5);
        assert_eq!(a.annotation_type, AnnotationType::Insertion);
    }

    #[test]
    fn annotations_at_document_boundaries() {
        let mut store = AnnotationStore::new();
        // Annotation at the very start of the first block
        store.add(deletion("block-0", 0, 1));
        // Annotation at an arbitrary large offset (end of document)
        store.add(comment("block-99", 1000, 1050));

        assert_eq!(store.len(), 2);
        let ordered = store.ordered();
        assert_eq!(ordered[0].block_id.as_deref(), Some("block-0"));
        assert_eq!(ordered[1].block_id.as_deref(), Some("block-99"));
    }

    #[test]
    fn multiple_overlapping_annotations_on_same_range() {
        let mut store = AnnotationStore::new();
        store.add(deletion("block-0", 0, 10));
        store.add(comment("block-0", 0, 10));

        // Both should be returned for the same range.
        let hits = store.overlapping("block-0", 0, 10);
        assert_eq!(hits.len(), 2);
    }
}
