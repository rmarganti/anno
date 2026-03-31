use super::store::AnnotationStore;
use super::types::{Annotation, AnnotationType};

/// Trait for exporting annotations to a string format.
pub trait AnnotationExporter {
    fn export(&self, store: &AnnotationStore, source_name: &str) -> String;
}

/// Exports annotations in an XML-like structured format designed for LLM agent consumption.
pub struct AgentExporter;

impl AnnotationExporter for AgentExporter {
    fn export(&self, store: &AnnotationStore, source_name: &str) -> String {
        let ordered = store.ordered();

        if ordered.is_empty() {
            return "No annotations.".to_string();
        }

        let count = ordered.len();
        let mut output = String::new();

        // Open <annotations> tag
        if source_name == "[stdin]" {
            output.push_str(&format!(
                "<annotations source=\"stdin\" total=\"{count}\">\n"
            ));
        } else {
            output.push_str(&format!(
                "<annotations file=\"{source_name}\" total=\"{count}\">\n"
            ));
        }

        let plural = if count == 1 {
            "annotation"
        } else {
            "annotations"
        };
        output.push_str(&format!(
            "The reviewer left {count} {plural} on this document.\n"
        ));

        for ann in &ordered {
            output.push('\n');
            export_annotation(&mut output, ann);
        }

        output.push_str("\n</annotations>\n");
        output
    }
}

fn line_attr(ann: &Annotation) -> String {
    match ann.range {
        Some(range) => {
            let start = range.start.line + 1;
            let end = range.end.line + 1;
            if start == end {
                format!(" line=\"{start}\"")
            } else {
                format!(" lines=\"{start}-{end}\"")
            }
        }
        None => String::new(),
    }
}

fn export_annotation(output: &mut String, ann: &Annotation) {
    let lines = line_attr(ann);

    match ann.annotation_type {
        AnnotationType::Deletion => {
            output.push_str(&format!("<delete{lines}>\n"));
            output.push_str(&ann.selected_text);
            output.push_str("\n</delete>\n");
        }
        AnnotationType::Comment => {
            output.push_str(&format!("<comment{lines}>\n"));
            output.push_str(&ann.text);
            output.push_str("\n</comment>\n");
        }
        AnnotationType::Replacement => {
            output.push_str(&format!("<replace{lines}>\n"));
            output.push_str("<original>\n");
            output.push_str(&ann.selected_text);
            output.push_str("\n</original>\n");
            output.push_str("<replacement>\n");
            output.push_str(&ann.text);
            output.push_str("\n</replacement>\n");
            output.push_str("</replace>\n");
        }
        AnnotationType::Insertion => {
            output.push_str(&format!("<insert{lines}>\n"));
            output.push_str(&ann.text);
            output.push_str("\n</insert>\n");
        }
        AnnotationType::GlobalComment => {
            output.push_str("<comment>\n");
            output.push_str(&ann.text);
            output.push_str("\n</comment>\n");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::annotation::types::{Annotation, TextPosition, TextRange};

    fn exporter() -> AgentExporter {
        AgentExporter
    }

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

    // ───── Individual annotation types ─────

    #[test]
    fn export_empty_store() {
        let store = AnnotationStore::new();
        assert_eq!(exporter().export(&store, "test.md"), "No annotations.");
    }

    #[test]
    fn export_deletion() {
        let mut store = AnnotationStore::new();
        store.add(Annotation::deletion(range(0, 0, 0, 10), "remove me".into()));

        let result = exporter().export(&store, "test.md");
        assert!(result.contains("<annotations file=\"test.md\" total=\"1\">"));
        assert!(result.contains("1 annotation on this document."));
        assert!(result.contains("<delete line=\"1\">\nremove me\n</delete>"));
        assert!(result.contains("</annotations>"));
    }

    #[test]
    fn export_comment() {
        let mut store = AnnotationStore::new();
        store.add(Annotation::comment(
            range(0, 0, 0, 5),
            "hello".into(),
            "needs more detail".into(),
        ));

        let result = exporter().export(&store, "test.md");
        assert!(result.contains("<comment line=\"1\">\nneeds more detail\n</comment>"));
    }

    #[test]
    fn export_replacement() {
        let mut store = AnnotationStore::new();
        store.add(Annotation::replacement(
            range(0, 0, 0, 5),
            "old text".into(),
            "new text".into(),
        ));

        let result = exporter().export(&store, "test.md");
        assert!(result.contains("<replace line=\"1\">"));
        assert!(result.contains("<original>\nold text\n</original>"));
        assert!(result.contains("<replacement>\nnew text\n</replacement>"));
        assert!(result.contains("</replace>"));
    }

    #[test]
    fn export_insertion() {
        let mut store = AnnotationStore::new();
        store.add(Annotation::insertion(
            TextPosition { line: 0, column: 5 },
            "inserted content".into(),
        ));

        let result = exporter().export(&store, "test.md");
        assert!(result.contains("<insert line=\"1\">\ninserted content\n</insert>"));
    }

    #[test]
    fn export_global_comment() {
        let mut store = AnnotationStore::new();
        store.add(Annotation::global_comment("overall looks good".into()));

        let result = exporter().export(&store, "test.md");
        assert!(result.contains("<comment>\noverall looks good\n</comment>"));
    }

    // ───── Line number formatting ─────

    #[test]
    fn export_multiline_range() {
        let mut store = AnnotationStore::new();
        store.add(Annotation::deletion(
            range(4, 0, 6, 10),
            "three lines".into(),
        ));

        let result = exporter().export(&store, "test.md");
        assert!(result.contains("<delete lines=\"5-7\">"));
    }

    #[test]
    fn export_single_line_range() {
        let mut store = AnnotationStore::new();
        store.add(Annotation::deletion(range(9, 0, 9, 5), "one line".into()));

        let result = exporter().export(&store, "test.md");
        assert!(result.contains("<delete line=\"10\">"));
    }

    // ───── Source name handling ─────

    #[test]
    fn export_stdin_source() {
        let mut store = AnnotationStore::new();
        store.add(Annotation::global_comment("a note".into()));

        let result = exporter().export(&store, "[stdin]");
        assert!(result.contains("<annotations source=\"stdin\" total=\"1\">"));
    }

    #[test]
    fn export_file_source() {
        let mut store = AnnotationStore::new();
        store.add(Annotation::global_comment("a note".into()));

        let result = exporter().export(&store, "src/main.rs");
        assert!(result.contains("<annotations file=\"src/main.rs\" total=\"1\">"));
    }

    // ───── Combined output ─────

    #[test]
    fn export_multiple_annotations() {
        let mut store = AnnotationStore::new();
        store.add(Annotation::deletion(range(0, 0, 0, 5), "first".into()));
        store.add(Annotation::comment(
            range(0, 10, 0, 15),
            "second".into(),
            "a comment".into(),
        ));
        store.add(Annotation::global_comment("general note".into()));

        let result = exporter().export(&store, "test.md");
        assert!(result.contains("total=\"3\""));
        assert!(result.contains("3 annotations on this document."));
    }

    // ───── Ordering ─────

    #[test]
    fn export_ordering_matches_line_then_column() {
        let mut store = AnnotationStore::new();
        // Add in reverse order
        store.add(Annotation::global_comment("global".into()));
        store.add(Annotation::deletion(range(5, 0, 5, 5), "later line".into()));
        store.add(Annotation::comment(
            range(1, 10, 1, 15),
            "second in line".into(),
            "note".into(),
        ));
        store.add(Annotation::deletion(
            range(1, 0, 1, 5),
            "first in line".into(),
        ));

        let result = exporter().export(&store, "test.md");

        let pos_first = result.find("first in line").unwrap();
        let pos_second = result.find("note").unwrap();
        let pos_later = result.find("later line").unwrap();
        let pos_global = result.find("global").unwrap();

        assert!(pos_first < pos_second);
        assert!(pos_second < pos_later);
        assert!(pos_later < pos_global);
    }
}
