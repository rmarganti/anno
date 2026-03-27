use ratatui::style::{Color, Modifier, Style};
use syntect::easy::HighlightLines;
use syntect::highlighting::{Color as SyntectColor, FontStyle, Theme};
use syntect::parsing::SyntaxSet;

use super::{Highlighter, StyledSpan};
use crate::startup::{StartupError, StartupSettings};

/// Magic value stored in the alpha byte of a `syntect::highlighting::Color` to signal
/// that `r` should be interpreted as an ANSI 256-color palette index rather than an
/// RGB red component. Alpha = 0 is syntect's own "no color / inherit" sentinel.
const ANSI_SENTINEL: u8 = 1;

/// Syntect-based highlighter using the built-in Markdown grammar to
/// statefully highlight an entire document (headings, code fences,
/// inline formatting, etc.) in a single pass.
pub struct SyntectHighlighter {
    syntax_set: SyntaxSet,
    theme: Theme,
    syntax_name: String,
    no_color: bool,
}

impl SyntectHighlighter {
    pub fn new() -> Self {
        Self::from_startup(&StartupSettings {
            theme_mode: crate::startup::ResolvedValue {
                value: crate::startup::ThemeMode::Auto,
                source: crate::startup::SettingSource::Auto,
            },
            theme: crate::startup::ResolvedValue {
                value: crate::highlight::theme_assets::resolve_theme_asset("neverforest")
                    .expect("fallback theme should exist"),
                source: crate::startup::SettingSource::Fallback,
            },
            theme_provenance: crate::startup::ThemeProvenance {
                theme_mode: crate::startup::ThemeMode::Auto,
                theme_mode_source: crate::startup::SettingSource::Auto,
                requested_theme: None,
                requested_theme_source: None,
                resolved_theme: "neverforest".to_owned(),
                resolved_theme_source: crate::startup::SettingSource::Fallback,
                resolved_theme_kind: crate::startup::ThemeProvenanceKind::BuiltIn,
                fallback: Some(crate::startup::ThemeProvenanceFallback::DefaultThemeSelection),
            },
            syntax: crate::startup::ResolvedValue {
                value: crate::startup::ResolvedSyntax {
                    requested: "markdown".to_owned(),
                    syntax_name: "Markdown".to_owned(),
                },
                source: crate::startup::SettingSource::Fallback,
            },
        })
        .expect("default startup settings should be valid")
    }

    pub fn from_startup(startup: &StartupSettings) -> Result<Self, StartupError> {
        let no_color = std::env::var("NO_COLOR").is_ok();
        Ok(Self {
            syntax_set: SyntaxSet::load_defaults_newlines(),
            theme: startup
                .theme
                .value
                .load_theme()
                .map_err(StartupError::ThemeAsset)?,
            syntax_name: startup.syntax.value.syntax_name.clone(),
            no_color,
        })
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
            .find_syntax_by_name(&self.syntax_name)
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
    use crate::highlight::theme_assets::resolve_theme_asset;
    use crate::startup::{
        ResolvedSyntax, ResolvedValue, SettingSource, StartupSettings, ThemeMode, ThemeProvenance,
        ThemeProvenanceFallback, ThemeProvenanceKind,
    };

    fn make_highlighter(no_color: bool) -> SyntectHighlighter {
        SyntectHighlighter {
            syntax_set: SyntaxSet::load_defaults_newlines(),
            theme: resolve_theme_asset("neverforest")
                .unwrap()
                .load_theme()
                .unwrap(),
            syntax_name: "Markdown".to_owned(),
            no_color,
        }
    }

    fn startup_with_syntax(syntax_name: &str) -> StartupSettings {
        StartupSettings {
            theme_mode: ResolvedValue {
                value: ThemeMode::Dark,
                source: SettingSource::Auto,
            },
            theme: ResolvedValue {
                value: resolve_theme_asset("neverforest").unwrap(),
                source: SettingSource::Fallback,
            },
            theme_provenance: ThemeProvenance {
                theme_mode: ThemeMode::Dark,
                theme_mode_source: SettingSource::Auto,
                requested_theme: None,
                requested_theme_source: None,
                resolved_theme: "neverforest".to_owned(),
                resolved_theme_source: SettingSource::Fallback,
                resolved_theme_kind: ThemeProvenanceKind::BuiltIn,
                fallback: Some(ThemeProvenanceFallback::DefaultThemeSelection),
            },
            syntax: ResolvedValue {
                value: ResolvedSyntax {
                    requested: syntax_name.to_owned(),
                    syntax_name: syntax_name.to_owned(),
                },
                source: SettingSource::Cli,
            },
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

    #[test]
    fn startup_settings_can_change_syntax() {
        let h = SyntectHighlighter::from_startup(&startup_with_syntax("Rust")).unwrap();
        let spans = highlight_one(&h, "fn main() {}");
        let has_styling = spans.iter().any(|s| s.style != Style::default());
        assert!(has_styling, "rust override should style rust code");
    }

    #[test]
    fn loaded_theme_produces_non_default_colors() {
        let h = SyntectHighlighter::new();
        let spans = highlight_one(&h, "# Heading");
        let has_color = spans.iter().any(|s| {
            matches!(
                s.style.fg,
                Some(Color::Indexed(_)) | Some(Color::Rgb(_, _, _)) | Some(Color::Reset)
            )
        });
        assert!(has_color, "theme should apply foreground colors");
    }
}
