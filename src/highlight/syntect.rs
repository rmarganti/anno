use ratatui::style::{Color, Modifier, Style};
use syntect::easy::HighlightLines;
use syntect::highlighting::{FontStyle, ThemeSet};
use syntect::parsing::SyntaxSet;

use super::{Highlighter, StyledSpan};

/// Syntect-based highlighter with support for markdown inline formatting
/// and language-specific code block highlighting.
pub struct SyntectHighlighter {
    syntax_set: SyntaxSet,
    theme_set: ThemeSet,
    no_color: bool,
}

impl SyntectHighlighter {
    /// Create a new highlighter. Respects the `NO_COLOR` env var — when set,
    /// all output is unstyled.
    pub fn new() -> Self {
        let no_color = std::env::var("NO_COLOR").is_ok();
        Self {
            syntax_set: SyntaxSet::load_defaults_newlines(),
            theme_set: ThemeSet::load_defaults(),
            no_color,
        }
    }

    /// Find the best syntect syntax for the given language tag.
    fn find_syntax(&self, language: &str) -> Option<&syntect::parsing::SyntaxReference> {
        self.syntax_set
            .find_syntax_by_token(language)
            .or_else(|| self.syntax_set.find_syntax_by_extension(language))
    }

    /// Convert a syntect RGBA color to a ratatui Color.
    fn to_ratatui_color(c: syntect::highlighting::Color) -> Color {
        Color::Rgb(c.r, c.g, c.b)
    }

    /// Convert a syntect FontStyle to ratatui Modifier.
    fn to_ratatui_modifier(font_style: FontStyle) -> Modifier {
        let mut modifier = Modifier::empty();
        if font_style.contains(FontStyle::BOLD) {
            modifier |= Modifier::BOLD;
        }
        if font_style.contains(FontStyle::ITALIC) {
            modifier |= Modifier::ITALIC;
        }
        if font_style.contains(FontStyle::UNDERLINE) {
            modifier |= Modifier::UNDERLINED;
        }
        modifier
    }
}

impl Default for SyntectHighlighter {
    fn default() -> Self {
        Self::new()
    }
}

impl Highlighter for SyntectHighlighter {
    fn highlight_line(&self, line: &str) -> Vec<StyledSpan> {
        if self.no_color || line.is_empty() {
            return vec![StyledSpan::plain(line)];
        }
        parse_inline_markdown(line)
    }

    fn highlight_code_block(&self, code: &str, language: Option<&str>) -> Vec<Vec<StyledSpan>> {
        if self.no_color {
            return code.lines().map(|l| vec![StyledSpan::plain(l)]).collect();
        }

        let syntax = language
            .and_then(|lang| self.find_syntax(lang))
            .unwrap_or_else(|| self.syntax_set.find_syntax_plain_text());

        let theme = &self.theme_set.themes["base16-ocean.dark"];
        let mut h = HighlightLines::new(syntax, theme);

        code.lines()
            .map(|line| {
                let regions = h
                    .highlight_line(line, &self.syntax_set)
                    .unwrap_or_default();

                regions
                    .into_iter()
                    .map(|(style, text)| {
                        let ratatui_style = Style::default()
                            .fg(Self::to_ratatui_color(style.foreground))
                            .add_modifier(Self::to_ratatui_modifier(style.font_style));
                        StyledSpan::new(text, ratatui_style)
                    })
                    .collect()
            })
            .collect()
    }
}

/// Parse a line of markdown prose and return styled spans for inline formatting:
/// bold, italic, bold-italic, inline code, and links.
fn parse_inline_markdown(line: &str) -> Vec<StyledSpan> {
    let mut spans: Vec<StyledSpan> = Vec::new();
    let chars: Vec<char> = line.chars().collect();
    let len = chars.len();
    let mut i = 0;
    let mut plain_buf = String::new();

    let flush_plain = |buf: &mut String, spans: &mut Vec<StyledSpan>| {
        if !buf.is_empty() {
            spans.push(StyledSpan::plain(buf.clone()));
            buf.clear();
        }
    };

    while i < len {
        // --- Inline code: `...` ---
        if chars[i] == '`' {
            if let Some(end) = find_closing(&chars, i + 1, '`') {
                flush_plain(&mut plain_buf, &mut spans);
                let text: String = chars[i + 1..end].iter().collect();
                spans.push(StyledSpan::new(
                    text,
                    Style::default().fg(Color::Yellow),
                ));
                i = end + 1;
                continue;
            }
        }

        // --- Bold-italic: ***...*** ---
        if i + 2 < len && chars[i] == '*' && chars[i + 1] == '*' && chars[i + 2] == '*' {
            if let Some(end) = find_closing_seq(&chars, i + 3, &['*', '*', '*']) {
                flush_plain(&mut plain_buf, &mut spans);
                let text: String = chars[i + 3..end].iter().collect();
                spans.push(StyledSpan::new(
                    text,
                    Style::default().add_modifier(Modifier::BOLD | Modifier::ITALIC),
                ));
                i = end + 3;
                continue;
            }
        }

        // --- Bold: **...** ---
        if i + 1 < len && chars[i] == '*' && chars[i + 1] == '*' {
            if let Some(end) = find_closing_seq(&chars, i + 2, &['*', '*']) {
                flush_plain(&mut plain_buf, &mut spans);
                let text: String = chars[i + 2..end].iter().collect();
                spans.push(StyledSpan::new(
                    text,
                    Style::default().add_modifier(Modifier::BOLD),
                ));
                i = end + 2;
                continue;
            }
        }

        // --- Italic: *...* (single, not preceded by another *) ---
        if chars[i] == '*' && !(i + 1 < len && chars[i + 1] == '*') {
            if let Some(end) = find_closing(&chars, i + 1, '*') {
                flush_plain(&mut plain_buf, &mut spans);
                let text: String = chars[i + 1..end].iter().collect();
                spans.push(StyledSpan::new(
                    text,
                    Style::default().add_modifier(Modifier::ITALIC),
                ));
                i = end + 1;
                continue;
            }
        }

        // --- Links: [text](url) ---
        if chars[i] == '[' {
            if let Some(close_bracket) = find_closing(&chars, i + 1, ']') {
                if close_bracket + 1 < len && chars[close_bracket + 1] == '(' {
                    if let Some(close_paren) = find_closing(&chars, close_bracket + 2, ')') {
                        flush_plain(&mut plain_buf, &mut spans);
                        let text: String = chars[i + 1..close_bracket].iter().collect();
                        spans.push(StyledSpan::new(
                            text,
                            Style::default()
                                .fg(Color::Blue)
                                .add_modifier(Modifier::UNDERLINED),
                        ));
                        i = close_paren + 1;
                        continue;
                    }
                }
            }
        }

        plain_buf.push(chars[i]);
        i += 1;
    }

    flush_plain(&mut plain_buf, &mut spans);

    if spans.is_empty() {
        vec![StyledSpan::plain(line)]
    } else {
        spans
    }
}

/// Find the index of the closing `delim` character starting from `start`.
fn find_closing(chars: &[char], start: usize, delim: char) -> Option<usize> {
    for j in start..chars.len() {
        if chars[j] == delim {
            return Some(j);
        }
    }
    None
}

/// Find the starting index of a closing multi-char sequence (e.g. `**`, `***`).
fn find_closing_seq(chars: &[char], start: usize, seq: &[char]) -> Option<usize> {
    let seq_len = seq.len();
    if chars.len() < seq_len {
        return None;
    }
    for j in start..=chars.len() - seq_len {
        if chars[j..j + seq_len] == *seq {
            return Some(j);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- Highlighter trait: highlight_line ---

    #[test]
    fn plain_text_returns_single_span() {
        let h = SyntectHighlighter::new();
        let spans = h.highlight_line("Hello world");
        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0].text, "Hello world");
    }

    #[test]
    fn bold_text_is_styled() {
        let h = SyntectHighlighter::new();
        let spans = h.highlight_line("before **bold** after");
        assert!(spans.len() >= 3);
        let bold_span = spans.iter().find(|s| s.text == "bold").unwrap();
        assert!(bold_span.style.add_modifier.contains(Modifier::BOLD));
    }

    #[test]
    fn italic_text_is_styled() {
        let h = SyntectHighlighter::new();
        let spans = h.highlight_line("some *italic* text");
        let italic_span = spans.iter().find(|s| s.text == "italic").unwrap();
        assert!(italic_span.style.add_modifier.contains(Modifier::ITALIC));
    }

    #[test]
    fn bold_italic_text_is_styled() {
        let h = SyntectHighlighter::new();
        let spans = h.highlight_line("***both***");
        let span = spans.iter().find(|s| s.text == "both").unwrap();
        assert!(span.style.add_modifier.contains(Modifier::BOLD));
        assert!(span.style.add_modifier.contains(Modifier::ITALIC));
    }

    #[test]
    fn inline_code_is_styled() {
        let h = SyntectHighlighter::new();
        let spans = h.highlight_line("use `foo()` here");
        let code_span = spans.iter().find(|s| s.text == "foo()").unwrap();
        assert_eq!(code_span.style.fg, Some(Color::Yellow));
    }

    #[test]
    fn link_text_is_styled() {
        let h = SyntectHighlighter::new();
        let spans = h.highlight_line("click [here](https://example.com) now");
        let link_span = spans.iter().find(|s| s.text == "here").unwrap();
        assert_eq!(link_span.style.fg, Some(Color::Blue));
        assert!(link_span.style.add_modifier.contains(Modifier::UNDERLINED));
    }

    #[test]
    fn empty_line_returns_single_span() {
        let h = SyntectHighlighter::new();
        let spans = h.highlight_line("");
        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0].text, "");
    }

    // --- Highlighter trait: highlight_code_block ---

    #[test]
    fn code_block_with_known_language_returns_styled_spans() {
        let h = SyntectHighlighter::new();
        let lines = h.highlight_code_block("fn main() {}", Some("rust"));
        assert_eq!(lines.len(), 1);
        // Should produce multiple styled spans for rust syntax
        assert!(!lines[0].is_empty());
        // The concatenated text should equal the original
        let text: String = lines[0].iter().map(|s| s.text.as_str()).collect();
        assert_eq!(text, "fn main() {}");
    }

    #[test]
    fn code_block_with_unknown_language_still_returns_text() {
        let h = SyntectHighlighter::new();
        let lines = h.highlight_code_block("some code", Some("nonexistent_lang_xyz"));
        assert_eq!(lines.len(), 1);
        let text: String = lines[0].iter().map(|s| s.text.as_str()).collect();
        assert_eq!(text, "some code");
    }

    #[test]
    fn code_block_with_no_language_returns_plain() {
        let h = SyntectHighlighter::new();
        let lines = h.highlight_code_block("just text", None);
        assert_eq!(lines.len(), 1);
        let text: String = lines[0].iter().map(|s| s.text.as_str()).collect();
        assert_eq!(text, "just text");
    }

    #[test]
    fn code_block_multiline_preserves_lines() {
        let h = SyntectHighlighter::new();
        let code = "let x = 1;\nlet y = 2;\nlet z = x + y;";
        let lines = h.highlight_code_block(code, Some("rust"));
        assert_eq!(lines.len(), 3);
    }

    // --- NO_COLOR ---

    #[test]
    fn no_color_highlight_line_returns_unstyled() {
        let h = SyntectHighlighter {
            syntax_set: SyntaxSet::load_defaults_newlines(),
            theme_set: ThemeSet::load_defaults(),
            no_color: true,
        };
        let spans = h.highlight_line("**bold** *italic*");
        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0].text, "**bold** *italic*");
        assert_eq!(spans[0].style, Style::default());
    }

    #[test]
    fn no_color_code_block_returns_unstyled() {
        let h = SyntectHighlighter {
            syntax_set: SyntaxSet::load_defaults_newlines(),
            theme_set: ThemeSet::load_defaults(),
            no_color: true,
        };
        let lines = h.highlight_code_block("fn main() {}", Some("rust"));
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].len(), 1);
        assert_eq!(lines[0][0].text, "fn main() {}");
        assert_eq!(lines[0][0].style, Style::default());
    }

    // --- Inline parsing edge cases ---

    #[test]
    fn unclosed_bold_treated_as_plain() {
        let h = SyntectHighlighter::new();
        let spans = h.highlight_line("**unclosed");
        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0].text, "**unclosed");
    }

    #[test]
    fn mixed_inline_formatting() {
        let h = SyntectHighlighter::new();
        let spans = h.highlight_line("**bold** and *italic* and `code`");
        let texts: Vec<&str> = spans.iter().map(|s| s.text.as_str()).collect();
        assert!(texts.contains(&"bold"));
        assert!(texts.contains(&"italic"));
        assert!(texts.contains(&"code"));
    }

    #[test]
    fn link_without_url_treated_as_plain() {
        let h = SyntectHighlighter::new();
        let spans = h.highlight_line("[text] not a link");
        // Should not crash, treated as plain text
        let full: String = spans.iter().map(|s| s.text.as_str()).collect();
        assert_eq!(full, "[text] not a link");
    }
}
