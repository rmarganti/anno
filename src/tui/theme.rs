use std::fmt;

use ratatui::style::{Color, Modifier, Style};
use serde::Deserialize;
use syntect::highlighting::{Color as SyntectColor, Theme as SyntectTheme};

const DEFAULT_BG: ThemeColor = ThemeColor::new(24, 28, 26);
const DEFAULT_FG: ThemeColor = ThemeColor::new(230, 226, 204);
const DEFAULT_ACCENT: ThemeColor = ThemeColor::new(122, 166, 218);
const MIN_TEXT_CONTRAST: f32 = 4.5;
const MIN_UI_CONTRAST: f32 = 3.0;
const MIN_SURFACE_CONTRAST: f32 = 1.25;

/// Centralized style definitions for the application UI.
pub struct UiTheme {
    pub document: Style,
    pub cursor: Style,
    pub selection_highlight: Style,
    pub annotation_highlight: Style,
    pub status_bar: Style,
    pub status_mode: Style,
    pub input_box: Style,
    pub input_box_border: Style,
    pub input_box_title: Style,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ThemeOverrides {
    #[serde(default)]
    pub cursor: StyleOverride,
    #[serde(default)]
    pub selection: StyleOverride,
    #[serde(default)]
    pub annotation: StyleOverride,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StyleOverride {
    #[serde(default)]
    pub fg: Option<ThemeColor>,
    #[serde(default)]
    pub bg: Option<ThemeColor>,
    #[serde(default)]
    pub bold: Option<bool>,
    #[serde(default)]
    pub italic: Option<bool>,
    #[serde(default)]
    pub underlined: Option<bool>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct ThemeColor {
    r: u8,
    g: u8,
    b: u8,
}

impl ThemeColor {
    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }

    fn from_syntect(color: Option<SyntectColor>) -> Option<Self> {
        color.map(|value| Self::new(value.r, value.g, value.b))
    }

    fn to_ratatui(self) -> Color {
        Color::Rgb(self.r, self.g, self.b)
    }

    fn mix(self, other: Self, amount: f32) -> Self {
        let amount = amount.clamp(0.0, 1.0);
        let inverse = 1.0 - amount;
        Self::new(
            ((self.r as f32 * inverse) + (other.r as f32 * amount)).round() as u8,
            ((self.g as f32 * inverse) + (other.g as f32 * amount)).round() as u8,
            ((self.b as f32 * inverse) + (other.b as f32 * amount)).round() as u8,
        )
    }

    fn lighten(self, amount: f32) -> Self {
        self.mix(Self::new(255, 255, 255), amount)
    }

    fn darken(self, amount: f32) -> Self {
        self.mix(Self::new(0, 0, 0), amount)
    }

    fn luminance(self) -> f32 {
        fn channel(value: u8) -> f32 {
            let normalized = value as f32 / 255.0;
            if normalized <= 0.039_28 {
                normalized / 12.92
            } else {
                ((normalized + 0.055) / 1.055).powf(2.4)
            }
        }

        0.2126 * channel(self.r) + 0.7152 * channel(self.g) + 0.0722 * channel(self.b)
    }

    fn contrast_ratio(self, other: Self) -> f32 {
        let lighter = self.luminance().max(other.luminance());
        let darker = self.luminance().min(other.luminance());
        (lighter + 0.05) / (darker + 0.05)
    }

    fn is_dark(self) -> bool {
        self.luminance() < 0.5
    }
}

impl fmt::Debug for ThemeColor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "#{:02x}{:02x}{:02x}", self.r, self.g, self.b)
    }
}

impl<'de> Deserialize<'de> for ThemeColor {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        parse_hex_color(&value).map_err(serde::de::Error::custom)
    }
}

impl UiTheme {
    pub fn new() -> Self {
        Self::from_syntect_theme(&SyntectTheme::default(), None)
    }

    pub fn from_syntect_theme(theme: &SyntectTheme, overrides: Option<&ThemeOverrides>) -> Self {
        let settings = &theme.settings;
        let background = ThemeColor::from_syntect(settings.background).unwrap_or(DEFAULT_BG);
        let foreground = pick_readable_text(
            background,
            &[
                ThemeColor::from_syntect(settings.foreground).unwrap_or(DEFAULT_FG),
                DEFAULT_FG,
                ThemeColor::new(255, 255, 255),
                ThemeColor::new(0, 0, 0),
            ],
            MIN_TEXT_CONTRAST,
        );
        let accent = derive_accent(theme, foreground, background);

        let cursor_bg = enforce_surface_contrast(
            ThemeColor::from_syntect(settings.caret)
                .unwrap_or_else(|| accent.mix(foreground, 0.25)),
            background,
            accent,
        );
        let cursor_fg = pick_readable_text(
            cursor_bg,
            &[
                foreground,
                background,
                ThemeColor::new(255, 255, 255),
                ThemeColor::new(0, 0, 0),
            ],
            MIN_UI_CONTRAST,
        );

        let selection_bg = enforce_surface_contrast(
            ThemeColor::from_syntect(settings.selection)
                .unwrap_or_else(|| accent.mix(background, 0.68)),
            background,
            accent,
        );
        let selection_fg = pick_readable_text(
            selection_bg,
            &[
                ThemeColor::from_syntect(settings.selection_foreground).unwrap_or(foreground),
                foreground,
                background,
                ThemeColor::new(255, 255, 255),
                ThemeColor::new(0, 0, 0),
            ],
            MIN_TEXT_CONTRAST,
        );

        let annotation_fg = pick_readable_text(
            background,
            &[
                ThemeColor::from_syntect(settings.highlight).unwrap_or(accent),
                accent,
                foreground,
            ],
            MIN_UI_CONTRAST,
        );

        let status_bg = enforce_surface_contrast(accent.mix(background, 0.84), background, accent);
        let status_fg = pick_readable_text(
            status_bg,
            &[
                foreground,
                background,
                ThemeColor::new(255, 255, 255),
                ThemeColor::new(0, 0, 0),
            ],
            MIN_TEXT_CONTRAST,
        );
        let status_mode_bg =
            enforce_surface_contrast(accent.mix(background, 0.35), status_bg, accent);
        let status_mode_fg = pick_readable_text(
            status_mode_bg,
            &[
                foreground,
                background,
                ThemeColor::new(255, 255, 255),
                ThemeColor::new(0, 0, 0),
            ],
            MIN_TEXT_CONTRAST,
        );

        let input_border_fg =
            pick_readable_text(background, &[accent, foreground], MIN_UI_CONTRAST);

        let overrides = overrides.cloned().unwrap_or_default();

        Self {
            document: Style::default()
                .bg(background.to_ratatui())
                .fg(foreground.to_ratatui()),
            cursor: overrides.cursor.apply(
                Style::default()
                    .bg(cursor_bg.to_ratatui())
                    .fg(cursor_fg.to_ratatui())
                    .add_modifier(Modifier::BOLD),
            ),
            selection_highlight: overrides.selection.apply(
                Style::default()
                    .bg(selection_bg.to_ratatui())
                    .fg(selection_fg.to_ratatui()),
            ),
            annotation_highlight: overrides.annotation.apply(
                Style::default()
                    .fg(annotation_fg.to_ratatui())
                    .add_modifier(Modifier::UNDERLINED),
            ),
            status_bar: Style::default()
                .bg(status_bg.to_ratatui())
                .fg(status_fg.to_ratatui()),
            status_mode: Style::default()
                .bg(status_mode_bg.to_ratatui())
                .fg(status_mode_fg.to_ratatui())
                .add_modifier(Modifier::BOLD),
            input_box: Style::default()
                .bg(background.to_ratatui())
                .fg(foreground.to_ratatui()),
            input_box_border: Style::default().fg(input_border_fg.to_ratatui()),
            input_box_title: Style::default()
                .fg(input_border_fg.to_ratatui())
                .add_modifier(Modifier::BOLD),
        }
    }
}

impl Default for UiTheme {
    fn default() -> Self {
        Self::new()
    }
}

impl StyleOverride {
    fn apply(&self, mut style: Style) -> Style {
        if let Some(fg) = self.fg {
            style = style.fg(fg.to_ratatui());
        }
        if let Some(bg) = self.bg {
            style = style.bg(bg.to_ratatui());
        }
        style = apply_modifier(style, Modifier::BOLD, self.bold);
        style = apply_modifier(style, Modifier::ITALIC, self.italic);
        apply_modifier(style, Modifier::UNDERLINED, self.underlined)
    }
}

fn apply_modifier(style: Style, modifier: Modifier, enabled: Option<bool>) -> Style {
    match enabled {
        Some(true) => style.add_modifier(modifier),
        Some(false) => style.remove_modifier(modifier),
        None => style,
    }
}

fn derive_accent(
    theme: &SyntectTheme,
    foreground: ThemeColor,
    background: ThemeColor,
) -> ThemeColor {
    let settings = &theme.settings;
    let accent = ThemeColor::from_syntect(settings.accent)
        .or_else(|| ThemeColor::from_syntect(settings.caret))
        .or_else(|| ThemeColor::from_syntect(settings.highlight))
        .or_else(|| ThemeColor::from_syntect(settings.selection_border))
        .or_else(|| ThemeColor::from_syntect(settings.selection))
        .unwrap_or(DEFAULT_ACCENT);

    if accent.contrast_ratio(background) >= MIN_UI_CONTRAST {
        accent
    } else if background.is_dark() {
        accent.lighten(0.3).mix(foreground, 0.15)
    } else {
        accent.darken(0.3).mix(foreground, 0.15)
    }
}

fn enforce_surface_contrast(
    surface: ThemeColor,
    background: ThemeColor,
    accent: ThemeColor,
) -> ThemeColor {
    if surface.contrast_ratio(background) >= MIN_SURFACE_CONTRAST {
        surface
    } else if background.is_dark() {
        accent.lighten(0.28)
    } else {
        accent.darken(0.28)
    }
}

fn pick_readable_text(
    background: ThemeColor,
    candidates: &[ThemeColor],
    min_contrast: f32,
) -> ThemeColor {
    let mut best = candidates.first().copied().unwrap_or(DEFAULT_FG);
    let mut best_contrast = 0.0;

    for candidate in candidates
        .iter()
        .copied()
        .chain([ThemeColor::new(255, 255, 255), ThemeColor::new(0, 0, 0)])
    {
        let contrast = candidate.contrast_ratio(background);
        if contrast >= min_contrast {
            return candidate;
        }
        if contrast > best_contrast {
            best = candidate;
            best_contrast = contrast;
        }
    }

    best
}

fn parse_hex_color(input: &str) -> Result<ThemeColor, String> {
    let trimmed = input.trim();
    let hex = trimmed.strip_prefix('#').unwrap_or(trimmed);
    if hex.len() != 6 {
        return Err(format!("expected #RRGGBB color, got '{trimmed}'"));
    }

    let r = u8::from_str_radix(&hex[0..2], 16)
        .map_err(|_| format!("expected #RRGGBB color, got '{trimmed}'"))?;
    let g = u8::from_str_radix(&hex[2..4], 16)
        .map_err(|_| format!("expected #RRGGBB color, got '{trimmed}'"))?;
    let b = u8::from_str_radix(&hex[4..6], 16)
        .map_err(|_| format!("expected #RRGGBB color, got '{trimmed}'"))?;
    Ok(ThemeColor::new(r, g, b))
}

#[cfg(test)]
mod tests {
    use crate::highlight::theme_assets::built_in_theme_assets;
    use syntect::highlighting::ThemeSettings;

    use super::*;

    fn color(r: u8, g: u8, b: u8) -> SyntectColor {
        SyntectColor { r, g, b, a: 0xff }
    }

    fn rgb(color: Option<Color>, label: &str) -> ThemeColor {
        match color {
            Some(Color::Rgb(r, g, b)) => ThemeColor::new(r, g, b),
            other => panic!("expected rgb {label}, got {other:?}"),
        }
    }

    #[test]
    fn derives_cursor_selection_and_annotation_from_syntect_theme() {
        let theme = SyntectTheme {
            settings: ThemeSettings {
                foreground: Some(color(220, 220, 220)),
                background: Some(color(16, 18, 20)),
                caret: Some(color(255, 140, 64)),
                selection: Some(color(44, 92, 128)),
                accent: Some(color(96, 180, 255)),
                ..ThemeSettings::default()
            },
            ..SyntectTheme::default()
        };

        let derived = UiTheme::from_syntect_theme(&theme, None);

        assert_eq!(derived.document.fg, Some(Color::Rgb(220, 220, 220)));
        assert_eq!(derived.document.bg, Some(Color::Rgb(16, 18, 20)));
        assert_eq!(derived.cursor.bg, Some(Color::Rgb(255, 140, 64)));
        assert_eq!(
            derived.selection_highlight.bg,
            Some(Color::Rgb(44, 92, 128))
        );
        assert!(
            derived
                .annotation_highlight
                .add_modifier
                .contains(Modifier::UNDERLINED)
        );
    }

    #[test]
    fn readability_guards_replace_low_contrast_selection_foreground() {
        let theme = SyntectTheme {
            settings: ThemeSettings {
                foreground: Some(color(70, 72, 74)),
                background: Some(color(32, 34, 36)),
                selection: Some(color(42, 44, 46)),
                selection_foreground: Some(color(43, 45, 47)),
                ..ThemeSettings::default()
            },
            ..SyntectTheme::default()
        };

        let derived = UiTheme::from_syntect_theme(&theme, None);
        let selection_fg = match derived.selection_highlight.fg {
            Some(Color::Rgb(r, g, b)) => ThemeColor::new(r, g, b),
            other => panic!("expected derived rgb foreground, got {other:?}"),
        };
        let selection_bg = match derived.selection_highlight.bg {
            Some(Color::Rgb(r, g, b)) => ThemeColor::new(r, g, b),
            other => panic!("expected derived rgb background, got {other:?}"),
        };

        assert!(selection_fg.contrast_ratio(selection_bg) >= MIN_TEXT_CONTRAST);
    }

    #[test]
    fn bundled_themes_keep_ui_overlays_readable() {
        for asset in built_in_theme_assets() {
            let syntect_theme = asset.load().unwrap();
            let theme = UiTheme::from_syntect_theme(&syntect_theme, None);
            let document_bg = rgb(theme.document.bg, "document background");
            let cursor_bg = rgb(theme.cursor.bg, "cursor background");
            let cursor_fg = rgb(theme.cursor.fg, "cursor foreground");
            let selection_bg = rgb(theme.selection_highlight.bg, "selection background");
            let selection_fg = rgb(theme.selection_highlight.fg, "selection foreground");
            let annotation_fg = rgb(theme.annotation_highlight.fg, "annotation foreground");
            let status_bg = rgb(theme.status_bar.bg, "status background");
            let status_fg = rgb(theme.status_bar.fg, "status foreground");
            let status_mode_bg = rgb(theme.status_mode.bg, "status mode background");
            let status_mode_fg = rgb(theme.status_mode.fg, "status mode foreground");
            let input_border_fg = rgb(theme.input_box_border.fg, "input border foreground");

            assert!(
                cursor_bg.contrast_ratio(document_bg) >= MIN_SURFACE_CONTRAST,
                "{} cursor should stand off from the document background",
                asset.canonical_name
            );
            assert!(
                cursor_fg.contrast_ratio(cursor_bg) >= MIN_UI_CONTRAST,
                "{} cursor text should remain readable",
                asset.canonical_name
            );
            assert!(
                selection_bg.contrast_ratio(document_bg) >= MIN_SURFACE_CONTRAST,
                "{} selection should stand off from the document background",
                asset.canonical_name
            );
            assert!(
                selection_fg.contrast_ratio(selection_bg) >= MIN_TEXT_CONTRAST,
                "{} selection text should remain readable",
                asset.canonical_name
            );
            assert!(
                annotation_fg.contrast_ratio(document_bg) >= MIN_UI_CONTRAST,
                "{} annotations should remain visible",
                asset.canonical_name
            );
            assert!(
                status_fg.contrast_ratio(status_bg) >= MIN_TEXT_CONTRAST,
                "{} status bar text should remain readable",
                asset.canonical_name
            );
            assert!(
                status_mode_bg.contrast_ratio(status_bg) >= MIN_SURFACE_CONTRAST,
                "{} status mode pill should stand off from the status bar",
                asset.canonical_name
            );
            assert!(
                status_mode_fg.contrast_ratio(status_mode_bg) >= MIN_TEXT_CONTRAST,
                "{} status mode text should remain readable",
                asset.canonical_name
            );
            assert!(
                input_border_fg.contrast_ratio(document_bg) >= MIN_UI_CONTRAST,
                "{} input borders should remain visible",
                asset.canonical_name
            );
        }
    }

    #[test]
    fn overrides_apply_without_touching_syntax_scope_rules() {
        let theme = SyntectTheme {
            settings: ThemeSettings {
                foreground: Some(color(220, 220, 220)),
                background: Some(color(20, 24, 28)),
                ..ThemeSettings::default()
            },
            ..SyntectTheme::default()
        };
        let overrides = ThemeOverrides {
            cursor: StyleOverride {
                fg: Some(ThemeColor::new(1, 2, 3)),
                bg: Some(ThemeColor::new(4, 5, 6)),
                ..StyleOverride::default()
            },
            selection: StyleOverride {
                underlined: Some(true),
                ..StyleOverride::default()
            },
            annotation: StyleOverride {
                bold: Some(true),
                ..StyleOverride::default()
            },
        };

        let derived = UiTheme::from_syntect_theme(&theme, Some(&overrides));

        assert_eq!(derived.cursor.fg, Some(Color::Rgb(1, 2, 3)));
        assert_eq!(derived.cursor.bg, Some(Color::Rgb(4, 5, 6)));
        assert!(
            derived
                .selection_highlight
                .add_modifier
                .contains(Modifier::UNDERLINED)
        );
        assert!(
            derived
                .annotation_highlight
                .add_modifier
                .contains(Modifier::BOLD)
        );
    }

    #[test]
    fn theme_color_deserializes_from_hex() {
        let color: ThemeColor = serde_json::from_str("\"#7aa2f7\"").unwrap();
        assert_eq!(color, ThemeColor::new(122, 162, 247));
    }
}
