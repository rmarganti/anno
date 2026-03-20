use ratatui::style::{Color, Modifier, Style};
use std::str::FromStr;
use syntect::easy::HighlightLines;
use syntect::highlighting::{
    Color as SyntectColor, FontStyle, ScopeSelectors, StyleModifier, Theme, ThemeItem,
    ThemeSettings,
};
use syntect::parsing::SyntaxSet;

use super::{Highlighter, StyledSpan};

/// Magic value stored in the alpha byte of a `syntect::highlighting::Color` to signal
/// that `r` should be interpreted as an ANSI 256-color palette index rather than an
/// RGB red component. Alpha = 0 is syntect's own "no color / inherit" sentinel.
const ANSI_SENTINEL: u8 = 1;

/// Construct a `syntect::highlighting::Color` that encodes an ANSI palette index.
/// In `to_ratatui_color` this is decoded back to `Color::Indexed(idx)`.
const fn ansi(idx: u8) -> SyntectColor {
    SyntectColor {
        r: idx,
        g: 0,
        b: 0,
        a: ANSI_SENTINEL,
    }
}

/// The syntect transparent/inherit sentinel: a = 0.
/// `to_ratatui_color` maps this to `Color::Reset` (terminal default foreground).
const INHERIT: SyntectColor = SyntectColor {
    r: 0,
    g: 0,
    b: 0,
    a: 0,
};

/// Build a programmatic syntect `Theme` that maps Markdown scope roles to ANSI
/// terminal palette colors. Using palette indices (rather than RGB) means the
/// user's terminal color scheme controls the appearance of highlighted text.
///
/// Color mapping (base16 role → ANSI index):
///
/// | Role              | ANSI | Used for                              |
/// |-------------------|------|---------------------------------------|
/// | base0B (green)    |  2   | Headings, bold markers                |
/// | base0A (yellow)   |  3   | Keywords, important punctuation       |
/// | base0D (blue)     |  4   | Strings, lists                        |
/// | base0C (cyan)     |  6   | Inline code, support                  |
/// | base08 (red)      |  1   | Functions, links                      |
/// | base0E (magenta)  |  5   | Tags, emphasis markers                |
/// | base0F (dark gray)|  8   | Deprecated / embedded content         |
/// | (default)         |  —   | Plain text → Color::Reset (terminal)  |
fn build_ansi_theme() -> Theme {
    fn item(scope_str: &str, fg: SyntectColor, font_style: Option<FontStyle>) -> ThemeItem {
        ThemeItem {
            scope: ScopeSelectors::from_str(scope_str).expect("valid scope selector"),
            style: StyleModifier {
                foreground: Some(fg),
                background: None,
                font_style,
            },
        }
    }

    Theme {
        name: Some("anno-ansi".to_owned()),
        author: None,
        settings: ThemeSettings {
            foreground: Some(INHERIT),
            background: None,
            ..ThemeSettings::default()
        },
        scopes: vec![
            // Headings — green (base0B)
            item("markup.heading", ansi(2), Some(FontStyle::BOLD)),
            // Bold text — green (base0B), bold modifier
            item("markup.bold", ansi(2), Some(FontStyle::BOLD)),
            // Italic text — magenta (base0E), italic modifier
            item("markup.italic", ansi(5), Some(FontStyle::ITALIC)),
            // Inline code — cyan (base0C)
            item("markup.raw.inline", ansi(6), None),
            // Fenced code blocks — green (base0B)
            item("markup.raw.block", ansi(2), None),
            // Blockquotes — dark gray (base0F)
            item("markup.quote", ansi(8), None),
            // List markers — blue (base0D)
            item("markup.list", ansi(4), None),
            // Punctuation (*, #, `, etc.) — yellow (base0A)
            item("punctuation.definition", ansi(3), None),
            // Link text [...] — red (base08)
            item("entity.name.tag", ansi(1), None),
            // Link URL (...) — red (base08), underlined
            item("markup.underline.link", ansi(1), Some(FontStyle::UNDERLINE)),
            // HTML comments — dark gray (base0F)
            item("comment", ansi(8), None),
        ],
    }
}

/// Syntect-based highlighter using the built-in Markdown grammar to
/// statefully highlight an entire document (headings, code fences,
/// inline formatting, etc.) in a single pass.
pub struct SyntectHighlighter {
    syntax_set: SyntaxSet,
    theme: Theme,
    no_color: bool,
}

impl SyntectHighlighter {
    /// Create a new highlighter. Respects the `NO_COLOR` env var — when set,
    /// all output is unstyled.
    pub fn new() -> Self {
        let no_color = std::env::var("NO_COLOR").is_ok();
        Self {
            syntax_set: SyntaxSet::load_defaults_newlines(),
            theme: build_ansi_theme(),
            no_color,
        }
    }

    /// Convert a syntect RGBA color to a ratatui Color.
    ///
    /// - `a == ANSI_SENTINEL`: treat `r` as a raw ANSI 256-color palette index →
    ///   `Color::Indexed(r)`.
    /// - `a == 0` (syntect "no color / inherit"): → `Color::Reset` (terminal default
    ///   foreground).
    /// - Anything else: fall back to `Color::Rgb` (unused by our theme but kept for
    ///   forward-compatibility).
    fn to_ratatui_color(c: SyntectColor) -> Color {
        match c.a {
            ANSI_SENTINEL => Color::Indexed(c.r),
            0 => Color::Reset,
            _ => Color::Rgb(c.r, c.g, c.b),
        }
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
    fn highlight_document(&self, lines: &[String]) -> Vec<Vec<StyledSpan>> {
        if self.no_color {
            return lines
                .iter()
                .map(|l| vec![StyledSpan::plain(l.as_str())])
                .collect();
        }

        let syntax = self
            .syntax_set
            .find_syntax_by_extension("md")
            .unwrap_or_else(|| self.syntax_set.find_syntax_plain_text());

        let mut h = HighlightLines::new(syntax, &self.theme);

        lines
            .iter()
            .map(|line| {
                let regions = h.highlight_line(line, &self.syntax_set).unwrap_or_default();

                if regions.is_empty() {
                    vec![StyledSpan::plain("")]
                } else {
                    regions
                        .into_iter()
                        .map(|(style, text)| {
                            let ratatui_style = Style::default()
                                .fg(Self::to_ratatui_color(style.foreground))
                                .add_modifier(Self::to_ratatui_modifier(style.font_style));
                            StyledSpan::new(text, ratatui_style)
                        })
                        .collect()
                }
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use ratatui::style::Color;

    use super::*;

    fn make_highlighter(no_color: bool) -> SyntectHighlighter {
        SyntectHighlighter {
            syntax_set: SyntaxSet::load_defaults_newlines(),
            theme: build_ansi_theme(),
            no_color,
        }
    }

    fn highlight_one(h: &SyntectHighlighter, line: &str) -> Vec<StyledSpan> {
        let lines = vec![line.to_owned()];
        h.highlight_document(&lines).into_iter().next().unwrap()
    }

    // --- Basic document highlighting ---

    #[test]
    fn plain_text_preserves_content() {
        let h = SyntectHighlighter::new();
        let spans = highlight_one(&h, "Hello world");
        let text: String = spans.iter().map(|s| s.text.as_str()).collect();
        assert_eq!(text, "Hello world");
    }

    #[test]
    fn empty_line_returns_single_span() {
        let h = SyntectHighlighter::new();
        let spans = highlight_one(&h, "");
        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0].text, "");
    }

    #[test]
    fn multiline_preserves_line_count() {
        let h = SyntectHighlighter::new();
        let lines: Vec<String> = vec!["# Heading".into(), "".into(), "Some text".into()];
        let result = h.highlight_document(&lines);
        assert_eq!(result.len(), 3);
    }

    #[test]
    fn roundtrip_preserves_text() {
        let h = SyntectHighlighter::new();
        let inputs = vec![
            "plain text".to_owned(),
            "**bold** text".to_owned(),
            "*italic* text".to_owned(),
            "`code` text".to_owned(),
            "# heading".to_owned(),
            "> blockquote".to_owned(),
            "- list item".to_owned(),
            "[link](url) text".to_owned(),
            "".to_owned(),
        ];
        let result = h.highlight_document(&inputs);
        for (i, spans) in result.iter().enumerate() {
            let roundtrip: String = spans.iter().map(|s| s.text.as_str()).collect();
            assert_eq!(
                roundtrip, inputs[i],
                "roundtrip failed for line {i}: {:?}",
                inputs[i]
            );
        }
    }

    #[test]
    fn heading_gets_styled() {
        let h = SyntectHighlighter::new();
        let spans = highlight_one(&h, "# Heading");
        // Syntect should produce styled (non-default) spans for a heading.
        let has_styling = spans.iter().any(|s| s.style != Style::default());
        assert!(has_styling, "heading should have non-default styling");
    }

    // --- ANSI color tests ---

    #[test]
    fn heading_uses_ansi_color_not_rgb() {
        let h = SyntectHighlighter::new();
        let spans = highlight_one(&h, "# Heading");
        // Every colored span must use an Indexed or Reset color, never Rgb.
        for span in &spans {
            if let Some(fg) = span.style.fg {
                assert!(
                    matches!(fg, Color::Indexed(_) | Color::Reset),
                    "heading span has RGB color {:?} — expected ANSI palette color",
                    fg
                );
            }
        }
    }

    #[test]
    fn heading_foreground_is_green_ansi2() {
        let h = SyntectHighlighter::new();
        let spans = highlight_one(&h, "# Heading");
        // The heading text (and its `#` marker) should be ANSI 2 (green / base0B).
        let has_green = spans.iter().any(|s| s.style.fg == Some(Color::Indexed(2)));
        assert!(
            has_green,
            "heading should contain a span with ANSI color 2 (green)"
        );
    }

    #[test]
    fn inline_code_uses_ansi_cyan() {
        let h = SyntectHighlighter::new();
        let spans = highlight_one(&h, "`code`");
        // Inline code should be ANSI 6 (cyan / base0C).
        let has_cyan = spans.iter().any(|s| s.style.fg == Some(Color::Indexed(6)));
        assert!(
            has_cyan,
            "inline code should contain a span with ANSI color 6 (cyan)"
        );
    }

    #[test]
    fn to_ratatui_color_ansi_sentinel() {
        let c = SyntectColor {
            r: 3,
            g: 0,
            b: 0,
            a: ANSI_SENTINEL,
        };
        assert_eq!(SyntectHighlighter::to_ratatui_color(c), Color::Indexed(3));
    }

    #[test]
    fn to_ratatui_color_inherit() {
        let c = SyntectColor {
            r: 0,
            g: 0,
            b: 0,
            a: 0,
        };
        assert_eq!(SyntectHighlighter::to_ratatui_color(c), Color::Reset);
    }

    #[test]
    fn to_ratatui_color_rgb_fallback() {
        let c = SyntectColor {
            r: 100,
            g: 150,
            b: 200,
            a: 255,
        };
        assert_eq!(
            SyntectHighlighter::to_ratatui_color(c),
            Color::Rgb(100, 150, 200)
        );
    }

    // --- NO_COLOR ---

    #[test]
    fn no_color_returns_unstyled() {
        let h = make_highlighter(true);
        let lines = vec!["**bold** *italic*".to_owned()];
        let result = h.highlight_document(&lines);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].len(), 1);
        assert_eq!(result[0][0].text, "**bold** *italic*");
        assert_eq!(result[0][0].style, Style::default());
    }

    #[test]
    fn no_color_multiline() {
        let h = make_highlighter(true);
        let lines = vec![
            "# Heading".to_owned(),
            "```rust".to_owned(),
            "fn main() {}".to_owned(),
            "```".to_owned(),
        ];
        let result = h.highlight_document(&lines);
        assert_eq!(result.len(), 4);
        for (i, spans) in result.iter().enumerate() {
            assert_eq!(spans.len(), 1);
            assert_eq!(spans[0].text, lines[i]);
            assert_eq!(spans[0].style, Style::default());
        }
    }
}
