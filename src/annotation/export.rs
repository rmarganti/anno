use super::store::AnnotationStore;
use super::types::{Annotation, AnnotationType};

/// Trait for exporting annotations to a string format.
pub trait AnnotationExporter {
    fn export(&self, store: &AnnotationStore) -> String;
}

/// Exports annotations in Plannotator-compatible markdown format.
pub struct PlannotatorExporter;

impl AnnotationExporter for PlannotatorExporter {
    fn export(&self, store: &AnnotationStore) -> String {
        let ordered = store.ordered();

        if ordered.is_empty() {
            return "No changes detected.".to_string();
        }

        let mut output = String::from("# Plan Feedback\n\n");

        let count = ordered.len();
        let plural = if count > 1 { "s" } else { "" };
        output.push_str(&format!(
            "I've reviewed this plan and have {count} piece{plural} of feedback:\n\n"
        ));

        for (index, ann) in ordered.iter().enumerate() {
            export_annotation(&mut output, ann, index + 1);
            output.push('\n');
        }

        output.push_str("---\n");
        output
    }
}

fn export_annotation(output: &mut String, ann: &Annotation, number: usize) {
    output.push_str(&format!("## {number}. "));

    match ann.annotation_type {
        AnnotationType::Deletion => {
            output.push_str("Remove this\n");
            output.push_str(&format!("```\n{}\n```\n", ann.selected_text));
            output.push_str("> I don't want this in the plan.\n");
        }
        AnnotationType::Insertion => {
            output.push_str("Add this\n");
            output.push_str(&format!("```\n{}\n```\n", ann.text));
        }
        AnnotationType::Replacement => {
            output.push_str("Change this\n");
            output.push_str(&format!("**From:**\n```\n{}\n```\n", ann.selected_text));
            output.push_str(&format!("**To:**\n```\n{}\n```\n", ann.text));
        }
        AnnotationType::Comment => {
            output.push_str(&format!("Feedback on: \"{}\"\n", ann.selected_text));
            output.push_str(&format!("> {}\n", ann.text));
        }
        AnnotationType::GlobalComment => {
            output.push_str("General feedback about the plan\n");
            output.push_str(&format!("> {}\n", ann.text));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::annotation::types::{Annotation, TextPosition, TextRange};

    fn exporter() -> PlannotatorExporter {
        PlannotatorExporter
    }

    fn range(sl: usize, sc: usize, el: usize, ec: usize) -> TextRange {
        TextRange {
            start: TextPosition { line: sl, column: sc },
            end: TextPosition { line: el, column: ec },
        }
    }

    // ───── Individual annotation types ─────

    #[test]
    fn export_empty_store() {
        let store = AnnotationStore::new();
        assert_eq!(exporter().export(&store), "No changes detected.");
    }

    #[test]
    fn export_deletion() {
        let mut store = AnnotationStore::new();
        store.add(Annotation::deletion(
            range(0, 0, 0, 10),
            "remove me".into(),
        ));

        let result = exporter().export(&store);
        assert!(result.contains("# Plan Feedback"));
        assert!(result.contains("1 piece of feedback"));
        assert!(result.contains("## 1. Remove this"));
        assert!(result.contains("```\nremove me\n```"));
        assert!(result.contains("> I don't want this in the plan."));
        assert!(result.ends_with("---\n"));
    }

    #[test]
    fn export_comment() {
        let mut store = AnnotationStore::new();
        store.add(Annotation::comment(
            range(0, 0, 0, 5),
            "hello".into(),
            "needs more detail".into(),
        ));

        let result = exporter().export(&store);
        assert!(result.contains("## 1. Feedback on: \"hello\""));
        assert!(result.contains("> needs more detail"));
    }

    #[test]
    fn export_replacement() {
        let mut store = AnnotationStore::new();
        store.add(Annotation::replacement(
            range(0, 0, 0, 5),
            "old text".into(),
            "new text".into(),
        ));

        let result = exporter().export(&store);
        assert!(result.contains("## 1. Change this"));
        assert!(result.contains("**From:**\n```\nold text\n```"));
        assert!(result.contains("**To:**\n```\nnew text\n```"));
    }

    #[test]
    fn export_insertion() {
        let mut store = AnnotationStore::new();
        store.add(Annotation::insertion(
            TextPosition { line: 0, column: 5 },
            "inserted content".into(),
        ));

        let result = exporter().export(&store);
        assert!(result.contains("## 1. Add this"));
        assert!(result.contains("```\ninserted content\n```"));
    }

    #[test]
    fn export_global_comment() {
        let mut store = AnnotationStore::new();
        store.add(Annotation::global_comment("overall looks good".into()));

        let result = exporter().export(&store);
        assert!(result.contains("## 1. General feedback about the plan"));
        assert!(result.contains("> overall looks good"));
    }

    // ───── Combined output ─────

    #[test]
    fn export_multiple_annotations_numbered_correctly() {
        let mut store = AnnotationStore::new();
        store.add(Annotation::deletion(
            range(0, 0, 0, 5),
            "first".into(),
        ));
        store.add(Annotation::comment(
            range(0, 10, 0, 15),
            "second".into(),
            "a comment".into(),
        ));
        store.add(Annotation::global_comment("general note".into()));

        let result = exporter().export(&store);
        assert!(result.contains("3 pieces of feedback"));
        assert!(result.contains("## 1."));
        assert!(result.contains("## 2."));
        assert!(result.contains("## 3."));
    }

    // ───── Ordering ─────

    #[test]
    fn export_ordering_matches_line_then_column() {
        let mut store = AnnotationStore::new();
        // Add in reverse order
        store.add(Annotation::global_comment("global".into()));
        store.add(Annotation::deletion(
            range(5, 0, 5, 5),
            "later line".into(),
        ));
        store.add(Annotation::comment(
            range(1, 10, 1, 15),
            "second in line".into(),
            "note".into(),
        ));
        store.add(Annotation::deletion(
            range(1, 0, 1, 5),
            "first in line".into(),
        ));

        let result = exporter().export(&store);

        // Verify ordering: line 1 col 0, line 1 col 10, line 5, then global
        let pos_first = result.find("first in line").unwrap();
        let pos_second = result.find("second in line").unwrap();
        let pos_later = result.find("later line").unwrap();
        let pos_global = result.find("General feedback").unwrap();

        assert!(pos_first < pos_second);
        assert!(pos_second < pos_later);
        assert!(pos_later < pos_global);
    }

    #[test]
    fn export_plural_grammar() {
        let mut store = AnnotationStore::new();
        store.add(Annotation::global_comment("only one".into()));
        let result = exporter().export(&store);
        assert!(result.contains("1 piece of feedback"));

        store.add(Annotation::global_comment("two now".into()));
        let result = exporter().export(&store);
        assert!(result.contains("2 pieces of feedback"));
    }
}
