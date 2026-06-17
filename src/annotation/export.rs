use super::store::AnnotationStore;
use super::types::{Annotation, AnnotationType};
use serde::Serialize;

/// Trait for exporting annotations to a string format.
pub trait AnnotationExporter {
    fn export(&self, store: &AnnotationStore, source_name: &str) -> String;
}

/// Exports annotations in an XML-like structured format designed for LLM agent consumption.
pub struct AgentExporter;

/// Exports annotations as structured JSON for programmatic tooling.
pub struct JsonExporter;

impl AnnotationExporter for AgentExporter {
    fn export(&self, store: &AnnotationStore, source_name: &str) -> String {
        let annotations = store
            .ordered()
            .into_iter()
            .map(ExportAnnotation::from)
            .collect::<Vec<_>>();
        let count = annotations.len();
        let mut output = String::new();

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

        for annotation in &annotations {
            output.push('\n');
            export_agent_annotation(&mut output, annotation);
        }

        output.push_str("\n</annotations>\n");
        output
    }
}

impl AnnotationExporter for JsonExporter {
    fn export(&self, store: &AnnotationStore, source_name: &str) -> String {
        let annotations = store
            .ordered()
            .into_iter()
            .map(ExportAnnotation::from)
            .collect::<Vec<_>>();

        serde_json::to_string(&JsonExport {
            source: source_name,
            total: annotations.len(),
            annotations,
        })
        .expect("json annotation export should serialize")
    }
}

#[derive(Serialize)]
struct JsonExport<'a> {
    source: &'a str,
    total: usize,
    annotations: Vec<ExportAnnotation>,
}

#[derive(Serialize)]
#[serde(tag = "type")]
enum ExportAnnotation {
    #[serde(rename = "delete")]
    Delete {
        #[serde(flatten)]
        location: ExportLocation,
        selected_text: String,
    },
    #[serde(rename = "comment")]
    Comment {
        #[serde(flatten)]
        location: ExportLocation,
        selected_text: String,
        comment: String,
    },
    #[serde(rename = "replace")]
    Replace {
        #[serde(flatten)]
        location: ExportLocation,
        selected_text: String,
        replacement: String,
    },
    #[serde(rename = "insert")]
    Insert {
        #[serde(flatten)]
        location: ExportLocation,
        text: String,
    },
    #[serde(rename = "global_comment")]
    GlobalComment { comment: String },
}

#[derive(Serialize)]
struct ExportLocation {
    #[serde(skip_serializing_if = "Option::is_none")]
    line: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    lines: Option<String>,
}

impl From<&Annotation> for ExportAnnotation {
    fn from(annotation: &Annotation) -> Self {
        let location = ExportLocation::from(annotation);

        match annotation.annotation_type {
            AnnotationType::Deletion => Self::Delete {
                location,
                selected_text: annotation.selected_text.clone(),
            },
            AnnotationType::Comment => Self::Comment {
                location,
                selected_text: annotation.selected_text.clone(),
                comment: annotation.text.clone(),
            },
            AnnotationType::Replacement => Self::Replace {
                location,
                selected_text: annotation.selected_text.clone(),
                replacement: annotation.text.clone(),
            },
            AnnotationType::Insertion => Self::Insert {
                location,
                text: annotation.text.clone(),
            },
            AnnotationType::GlobalComment => Self::GlobalComment {
                comment: annotation.text.clone(),
            },
        }
    }
}

impl From<&Annotation> for ExportLocation {
    fn from(annotation: &Annotation) -> Self {
        match annotation.range {
            Some(range) => {
                let start = range.start.line + 1;
                let end = range.end.line + 1;
                if start == end {
                    Self {
                        line: Some(start),
                        lines: None,
                    }
                } else {
                    Self {
                        line: None,
                        lines: Some(format!("{start}-{end}")),
                    }
                }
            }
            None => Self {
                line: None,
                lines: None,
            },
        }
    }
}

fn location_attr(location: &ExportLocation) -> String {
    match (location.line, location.lines.as_deref()) {
        (Some(line), None) => format!(" line=\"{line}\""),
        (None, Some(lines)) => format!(" lines=\"{lines}\""),
        _ => String::new(),
    }
}

fn export_agent_annotation(output: &mut String, annotation: &ExportAnnotation) {
    match annotation {
        ExportAnnotation::Delete {
            location,
            selected_text,
        } => {
            output.push_str(&format!(
                "<annotation type=\"delete\"{}>\n",
                location_attr(location)
            ));
            export_agent_field(output, "selected_text", selected_text);
        }
        ExportAnnotation::Comment {
            location,
            selected_text,
            comment,
        } => {
            output.push_str(&format!(
                "<annotation type=\"comment\"{}>\n",
                location_attr(location)
            ));
            export_agent_field(output, "selected_text", selected_text);
            export_agent_field(output, "comment", comment);
        }
        ExportAnnotation::Replace {
            location,
            selected_text,
            replacement,
        } => {
            output.push_str(&format!(
                "<annotation type=\"replace\"{}>\n",
                location_attr(location)
            ));
            export_agent_field(output, "selected_text", selected_text);
            export_agent_field(output, "replacement", replacement);
        }
        ExportAnnotation::Insert { location, text } => {
            output.push_str(&format!(
                "<annotation type=\"insert\"{}>\n",
                location_attr(location)
            ));
            export_agent_field(output, "text", text);
        }
        ExportAnnotation::GlobalComment { comment } => {
            output.push_str("<annotation type=\"global_comment\">\n");
            export_agent_field(output, "comment", comment);
        }
    }

    output.push_str("</annotation>\n");
}

fn export_agent_field(output: &mut String, name: &str, value: &str) {
    output.push_str(&format!("<{name}>\n"));
    output.push_str(value);
    output.push_str(&format!("\n</{name}>\n"));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::annotation::types::{Annotation, TextPosition, TextRange};

    fn exporter() -> AgentExporter {
        AgentExporter
    }

    fn json_exporter() -> JsonExporter {
        JsonExporter
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
        assert_eq!(
            exporter().export(&store, "test.md"),
            "<annotations file=\"test.md\" total=\"0\">\nThe reviewer left 0 annotations on this document.\n\n</annotations>\n"
        );
    }

    #[test]
    fn export_deletion() {
        let mut store = AnnotationStore::new();
        store.add(Annotation::deletion(range(0, 0, 0, 10), "remove me".into()));

        let result = exporter().export(&store, "test.md");
        assert!(result.contains("<annotations file=\"test.md\" total=\"1\">"));
        assert!(result.contains("1 annotation on this document."));
        assert!(result.contains("<annotation type=\"delete\" line=\"1\">"));
        assert!(result.contains("<selected_text>\nremove me\n</selected_text>"));
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
        assert!(result.contains("<annotation type=\"comment\" line=\"1\">"));
        assert!(result.contains("<selected_text>\nhello\n</selected_text>"));
        assert!(result.contains("<comment>\nneeds more detail\n</comment>"));
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
        assert!(result.contains("<annotation type=\"replace\" line=\"1\">"));
        assert!(result.contains("<selected_text>\nold text\n</selected_text>"));
        assert!(result.contains("<replacement>\nnew text\n</replacement>"));
        assert!(result.contains("</annotation>"));
    }

    #[test]
    fn export_insertion() {
        let mut store = AnnotationStore::new();
        store.add(Annotation::insertion(
            TextPosition { line: 0, column: 5 },
            "inserted content".into(),
        ));

        let result = exporter().export(&store, "test.md");
        assert!(result.contains("<annotation type=\"insert\" line=\"1\">"));
        assert!(result.contains("<text>\ninserted content\n</text>"));
    }

    #[test]
    fn export_global_comment() {
        let mut store = AnnotationStore::new();
        store.add(Annotation::global_comment("overall looks good".into()));

        let result = exporter().export(&store, "test.md");
        assert!(result.contains("<annotation type=\"global_comment\">"));
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
        assert!(result.contains("<annotation type=\"delete\" lines=\"5-7\">"));
    }

    #[test]
    fn export_single_line_range() {
        let mut store = AnnotationStore::new();
        store.add(Annotation::deletion(range(9, 0, 9, 5), "one line".into()));

        let result = exporter().export(&store, "test.md");
        assert!(result.contains("<annotation type=\"delete\" line=\"10\">"));
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

    #[test]
    fn json_export_empty_store() {
        let store = AnnotationStore::new();

        assert_eq!(
            json_exporter().export(&store, "test.md"),
            "{\"source\":\"test.md\",\"total\":0,\"annotations\":[]}"
        );
    }

    #[test]
    fn json_export_deletion() {
        let mut store = AnnotationStore::new();
        store.add(Annotation::deletion(range(2, 0, 2, 9), "remove me".into()));

        assert_eq!(
            json_exporter().export(&store, "test.md"),
            "{\"source\":\"test.md\",\"total\":1,\"annotations\":[{\"type\":\"delete\",\"line\":3,\"selected_text\":\"remove me\"}]}"
        );
    }

    #[test]
    fn json_export_comment() {
        let mut store = AnnotationStore::new();
        store.add(Annotation::comment(
            range(11, 0, 13, 5),
            "selected text".into(),
            "needs more detail".into(),
        ));

        assert_eq!(
            json_exporter().export(&store, "test.md"),
            "{\"source\":\"test.md\",\"total\":1,\"annotations\":[{\"type\":\"comment\",\"lines\":\"12-14\",\"selected_text\":\"selected text\",\"comment\":\"needs more detail\"}]}"
        );
    }

    #[test]
    fn json_export_replacement() {
        let mut store = AnnotationStore::new();
        store.add(Annotation::replacement(
            range(0, 0, 0, 4),
            "old".into(),
            "new".into(),
        ));

        assert_eq!(
            json_exporter().export(&store, "test.md"),
            "{\"source\":\"test.md\",\"total\":1,\"annotations\":[{\"type\":\"replace\",\"line\":1,\"selected_text\":\"old\",\"replacement\":\"new\"}]}"
        );
    }

    #[test]
    fn json_export_insertion() {
        let mut store = AnnotationStore::new();
        store.add(Annotation::insertion(
            TextPosition { line: 4, column: 2 },
            "inserted content".into(),
        ));

        assert_eq!(
            json_exporter().export(&store, "test.md"),
            "{\"source\":\"test.md\",\"total\":1,\"annotations\":[{\"type\":\"insert\",\"line\":5,\"text\":\"inserted content\"}]}"
        );
    }

    #[test]
    fn json_export_global_comment() {
        let mut store = AnnotationStore::new();
        store.add(Annotation::global_comment("overall looks good".into()));

        assert_eq!(
            json_exporter().export(&store, "test.md"),
            "{\"source\":\"test.md\",\"total\":1,\"annotations\":[{\"type\":\"global_comment\",\"comment\":\"overall looks good\"}]}"
        );
    }
}
