use uuid::Uuid;

use super::types::{Annotation, TextRange};

/// Collection of annotations with CRUD operations and position-based ordering.
#[derive(Debug, Default)]
pub struct AnnotationStore {
    annotations: Vec<Annotation>,
}

#[allow(dead_code)] // TODO: methods used when annotation CRUD is wired up
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

    /// Return annotations ordered by position: start line, start column,
    /// end line, end column, then creation timestamp.
    /// `GlobalComment` annotations (`None` range) are sorted to the end.
    pub fn ordered(&self) -> Vec<&Annotation> {
        let mut refs: Vec<&Annotation> = self.annotations.iter().collect();
        refs.sort_by(|a, b| {
            let a_key = a.range.map(|r| (0, r.start.line, r.start.column, r.end.line, r.end.column));
            let b_key = b.range.map(|r| (0, r.start.line, r.start.column, r.end.line, r.end.column));
            // None (GlobalComment) sorts last: map to (1, ...) vs (0, ...) for Some.
            let a_sort = a_key.unwrap_or((1, 0, 0, 0, 0));
            let b_sort = b_key.unwrap_or((1, 0, 0, 0, 0));
            a_sort.cmp(&b_sort).then(a.timestamp.cmp(&b.timestamp))
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

    /// Return all annotations that overlap the given text range.
    /// Two ranges overlap if one starts before the other ends and vice versa,
    /// using `(line, column)` tuple ordering.
    pub fn overlapping(&self, range: &TextRange) -> Vec<&Annotation> {
        self.annotations
            .iter()
            .filter(|a| {
                if let Some(r) = a.range {
                    r.start < range.end && range.start < r.end
                } else {
                    false
                }
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::annotation::types::{Annotation, AnnotationType, TextPosition, TextRange};

    // ───── helpers ─────

    fn range(sl: usize, sc: usize, el: usize, ec: usize) -> TextRange {
        TextRange {
            start: TextPosition { line: sl, column: sc },
            end: TextPosition { line: el, column: ec },
        }
    }

    fn pos(line: usize, column: usize) -> TextPosition {
        TextPosition { line, column }
    }

    fn deletion(sl: usize, sc: usize, el: usize, ec: usize) -> Annotation {
        Annotation::deletion(range(sl, sc, el, ec), "x".into())
    }

    fn comment(sl: usize, sc: usize, el: usize, ec: usize) -> Annotation {
        Annotation::comment(range(sl, sc, el, ec), "sel".into(), "note".into())
    }

    fn global() -> Annotation {
        Annotation::global_comment("global note".into())
    }

    // ───── CRUD ─────

    #[test]
    fn add_and_retrieve() {
        let mut store = AnnotationStore::new();
        let ann = deletion(0, 0, 0, 5);
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
        let ann = deletion(0, 0, 0, 5);
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

        store.add(deletion(0, 0, 0, 1));
        store.add(deletion(0, 1, 0, 2));
        assert_eq!(store.len(), 2);
        assert!(!store.is_empty());
    }

    // ───── Ordering ─────

    #[test]
    fn ordered_by_line_then_column() {
        let mut store = AnnotationStore::new();
        store.add(deletion(5, 0, 5, 10));   // line 5
        store.add(deletion(1, 5, 1, 10));   // line 1, col 5
        store.add(deletion(1, 0, 1, 5));    // line 1, col 0

        let ordered = store.ordered();
        let r0 = ordered[0].range.unwrap();
        let r1 = ordered[1].range.unwrap();
        let r2 = ordered[2].range.unwrap();
        assert_eq!((r0.start.line, r0.start.column), (1, 0));
        assert_eq!((r1.start.line, r1.start.column), (1, 5));
        assert_eq!((r2.start.line, r2.start.column), (5, 0));
    }

    #[test]
    fn global_comments_sort_last() {
        let mut store = AnnotationStore::new();
        store.add(global());
        store.add(deletion(0, 0, 0, 5));
        store.add(global());

        let ordered = store.ordered();
        // Range-anchored annotation first, then global comments
        assert!(ordered[0].range.is_some());
        assert!(ordered[1].range.is_none());
        assert!(ordered[2].range.is_none());
    }

    // ───── Overlapping ─────

    #[test]
    fn overlapping_finds_intersecting_annotations() {
        let mut store = AnnotationStore::new();
        store.add(deletion(1, 0, 1, 10));
        store.add(deletion(1, 5, 1, 15));
        store.add(deletion(3, 0, 3, 10));
        store.add(deletion(5, 0, 5, 10));  // different line entirely

        // Query range [1:3, 1:12) should match the first two.
        let query = range(1, 3, 1, 12);
        let hits = store.overlapping(&query);
        assert_eq!(hits.len(), 2);
    }

    #[test]
    fn overlapping_no_match_when_adjacent() {
        let mut store = AnnotationStore::new();
        store.add(deletion(1, 0, 1, 5));

        // Range [1:5, 1:10) starts exactly where the annotation ends — no overlap.
        let query = range(1, 5, 1, 10);
        let hits = store.overlapping(&query);
        assert!(hits.is_empty());
    }

    // ───── Edge cases ─────

    #[test]
    fn empty_selection_annotation() {
        let mut store = AnnotationStore::new();
        let ann = Annotation::insertion(pos(1, 5), "inserted".into());
        let id = ann.id;
        store.add(ann);

        let a = store.get(id).unwrap();
        let r = a.range.unwrap();
        assert_eq!(r.start, r.end);
        assert_eq!((r.start.line, r.start.column), (1, 5));
        assert_eq!(a.annotation_type, AnnotationType::Insertion);
    }

    #[test]
    fn annotations_at_document_boundaries() {
        let mut store = AnnotationStore::new();
        // Annotation at the very start
        store.add(deletion(0, 0, 0, 1));
        // Annotation far into the document
        store.add(comment(99, 0, 99, 50));

        assert_eq!(store.len(), 2);
        let ordered = store.ordered();
        assert_eq!(ordered[0].range.unwrap().start.line, 0);
        assert_eq!(ordered[1].range.unwrap().start.line, 99);
    }

    #[test]
    fn multiple_overlapping_annotations_on_same_range() {
        let mut store = AnnotationStore::new();
        store.add(deletion(1, 0, 1, 10));
        store.add(comment(1, 0, 1, 10));

        // Both should be returned for the same range.
        let query = range(1, 0, 1, 10);
        let hits = store.overlapping(&query);
        assert_eq!(hits.len(), 2);
    }
}
