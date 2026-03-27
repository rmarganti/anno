use std::fs;
use std::io::{self, Read};
use std::path::Path;

use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SourceMetadata {
    pub display_name: String,
    pub syntax_hint: Option<String>,
}

pub trait InputSource {
    fn read_content(&self) -> Result<String, InputError>;
    fn name(&self) -> &str;
    fn metadata(&self) -> SourceMetadata;
}

#[derive(Debug)]
pub enum InputError {
    FileNotFound(String),
    ReadError(String),
}

impl std::fmt::Display for InputError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InputError::FileNotFound(path) => write!(f, "File not found: {path}"),
            InputError::ReadError(msg) => write!(f, "Read error: {msg}"),
        }
    }
}

impl std::error::Error for InputError {}

pub struct FileSource {
    path: String,
}

impl FileSource {
    pub fn new(path: String) -> Self {
        Self { path }
    }
}

impl InputSource for FileSource {
    fn read_content(&self) -> Result<String, InputError> {
        if !Path::new(&self.path).exists() {
            return Err(InputError::FileNotFound(self.path.clone()));
        }
        fs::read_to_string(&self.path).map_err(|e| InputError::ReadError(e.to_string()))
    }

    fn name(&self) -> &str {
        &self.path
    }

    fn metadata(&self) -> SourceMetadata {
        SourceMetadata {
            display_name: self.path.clone(),
            syntax_hint: Some(self.path.clone()),
        }
    }
}

pub struct StdinSource;

impl InputSource for StdinSource {
    fn read_content(&self) -> Result<String, InputError> {
        let mut buf = String::new();
        io::stdin()
            .read_to_string(&mut buf)
            .map_err(|e| InputError::ReadError(e.to_string()))?;
        Ok(buf)
    }

    fn name(&self) -> &str {
        "[stdin]"
    }

    fn metadata(&self) -> SourceMetadata {
        SourceMetadata {
            display_name: self.name().to_owned(),
            syntax_hint: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn file_not_found_returns_error() {
        let source = FileSource::new("/tmp/anno_nonexistent_file_test.md".to_string());
        let result = source.read_content();
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), InputError::FileNotFound(_)));
    }

    #[test]
    fn file_source_reads_content() {
        let path = "/tmp/anno_test_input.md";
        let mut f = std::fs::File::create(path).unwrap();
        f.write_all(b"# Hello\nWorld").unwrap();

        let source = FileSource::new(path.to_string());
        let content = source.read_content().unwrap();
        assert_eq!(content, "# Hello\nWorld");
        assert_eq!(source.name(), path);
        assert_eq!(source.metadata().display_name, path);
        assert_eq!(source.metadata().syntax_hint.as_deref(), Some(path));

        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn stdin_source_name() {
        let source = StdinSource;
        assert_eq!(source.name(), "[stdin]");
        assert_eq!(source.metadata().display_name, "[stdin]");
        assert_eq!(source.metadata().syntax_hint, None);
    }
}
