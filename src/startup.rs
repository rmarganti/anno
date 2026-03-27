use std::env;
use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use clap::{Parser, ValueEnum};
use serde::{Deserialize, Serialize};
use syntect::parsing::{SyntaxReference, SyntaxSet};

use crate::highlight::theme_assets::{
    ResolvedThemeAsset, ThemeAssetError, ThemeAssetSource, resolve_theme_asset,
};

#[derive(Debug, Parser)]
#[command(name = "anno", about = "Annotate markdown files in the terminal")]
pub struct Cli {
    /// Built-in theme name or path to a .tmTheme file
    #[arg(long)]
    pub theme: Option<String>,

    /// Theme mode preference used for auto theme selection
    #[arg(long = "theme-mode", value_enum)]
    pub theme_mode: Option<ThemeMode>,

    /// Syntax name or extension override for highlighting
    #[arg(long)]
    pub syntax: Option<String>,

    /// Markdown file to annotate
    pub file: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ThemeMode {
    Auto,
    Light,
    Dark,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SettingSource {
    Cli,
    Config,
    Auto,
    Fallback,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ThemeProvenanceFallback {
    AutoThemeResolutionFailed,
    DefaultThemeSelection,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedValue<T> {
    pub value: T,
    pub source: SettingSource,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedSyntax {
    pub requested: String,
    pub syntax_name: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ThemeProvenanceKind {
    BuiltIn,
    Path,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ThemeProvenance {
    pub theme_mode: ThemeMode,
    pub theme_mode_source: SettingSource,
    pub requested_theme: Option<String>,
    pub requested_theme_source: Option<SettingSource>,
    pub resolved_theme: String,
    pub resolved_theme_source: SettingSource,
    pub resolved_theme_kind: ThemeProvenanceKind,
    pub fallback: Option<ThemeProvenanceFallback>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StartupSettings {
    pub theme_mode: ResolvedValue<ThemeMode>,
    pub theme: ResolvedValue<ResolvedThemeAsset>,
    pub theme_provenance: ThemeProvenance,
    pub syntax: ResolvedValue<ResolvedSyntax>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ThemeSelection {
    resolved: ResolvedValue<ResolvedThemeAsset>,
    provenance: ThemeProvenance,
}

#[derive(Debug)]
pub enum StartupError {
    ReadSettings {
        path: PathBuf,
        source: io::Error,
    },
    ParseSettings {
        path: PathBuf,
        source: serde_json::Error,
    },
    ThemeAsset(ThemeAssetError),
    UnknownSyntax {
        requested: String,
    },
}

impl fmt::Display for StartupError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ReadSettings { path, source } => {
                write!(
                    f,
                    "failed to read settings file {}: {source}",
                    path.display()
                )
            }
            Self::ParseSettings { path, source } => {
                write!(
                    f,
                    "failed to parse settings file {}: {source}",
                    path.display()
                )
            }
            Self::ThemeAsset(source) => write!(f, "{source}"),
            Self::UnknownSyntax { requested } => write!(f, "unknown syntax '{requested}'"),
        }
    }
}

impl std::error::Error for StartupError {}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct SettingsFile {
    #[serde(default)]
    theme: Option<String>,
    #[serde(default, alias = "themeMode", alias = "theme-mode")]
    theme_mode: Option<ThemeMode>,
    #[serde(default)]
    syntax: Option<String>,
}

impl StartupSettings {
    pub fn resolve(cli: &Cli, source_name: &str) -> Result<Self, StartupError> {
        let config = load_settings_file()?;
        let syntax_set = SyntaxSet::load_defaults_newlines();

        let theme_mode = if let Some(value) = cli.theme_mode {
            ResolvedValue {
                value,
                source: SettingSource::Cli,
            }
        } else if let Some(value) = config.theme_mode {
            ResolvedValue {
                value,
                source: SettingSource::Config,
            }
        } else {
            ResolvedValue {
                value: ThemeMode::Auto,
                source: SettingSource::Auto,
            }
        };

        let theme = resolve_theme(cli.theme.as_deref(), config.theme.as_deref(), &theme_mode)?;
        let syntax = resolve_syntax(
            cli.syntax.as_deref(),
            config.syntax.as_deref(),
            source_name,
            &syntax_set,
        )?;

        Ok(Self {
            theme_mode,
            theme: theme.resolved,
            theme_provenance: theme.provenance,
            syntax,
        })
    }
}

fn load_settings_file() -> Result<SettingsFile, StartupError> {
    let Some(path) = settings_path() else {
        return Ok(SettingsFile::default());
    };

    if !path.exists() {
        return Ok(SettingsFile::default());
    }

    let contents = fs::read_to_string(&path).map_err(|source| StartupError::ReadSettings {
        path: path.clone(),
        source,
    })?;

    serde_json::from_str(&contents).map_err(|source| StartupError::ParseSettings { path, source })
}

fn settings_path() -> Option<PathBuf> {
    let home = env::var_os("HOME")?;
    Some(PathBuf::from(home).join(".config/anno/settings.json"))
}

fn resolve_theme(
    cli: Option<&str>,
    config: Option<&str>,
    theme_mode: &ResolvedValue<ThemeMode>,
) -> Result<ThemeSelection, StartupError> {
    resolve_theme_with(cli, config, theme_mode, resolve_theme_asset)
}

fn resolve_theme_with<F>(
    cli: Option<&str>,
    config: Option<&str>,
    theme_mode: &ResolvedValue<ThemeMode>,
    resolver: F,
) -> Result<ThemeSelection, StartupError>
where
    F: Fn(&str) -> Result<ResolvedThemeAsset, ThemeAssetError>,
{
    if let Some(requested) = normalize_optional(cli) {
        return resolver(requested)
            .map(|value| ThemeSelection {
                resolved: ResolvedValue {
                    value: value.clone(),
                    source: SettingSource::Cli,
                },
                provenance: build_theme_provenance(
                    theme_mode,
                    Some((requested, SettingSource::Cli)),
                    &value,
                    SettingSource::Cli,
                    None,
                ),
            })
            .map_err(StartupError::ThemeAsset);
    }

    if let Some(requested) = normalize_optional(config) {
        return resolver(requested)
            .map(|value| ThemeSelection {
                resolved: ResolvedValue {
                    value: value.clone(),
                    source: SettingSource::Config,
                },
                provenance: build_theme_provenance(
                    theme_mode,
                    Some((requested, SettingSource::Config)),
                    &value,
                    SettingSource::Config,
                    None,
                ),
            })
            .map_err(StartupError::ThemeAsset);
    }

    if let Some(requested) = auto_theme_name(theme_mode.value) {
        if let Ok(value) = resolver(requested) {
            return Ok(ThemeSelection {
                resolved: ResolvedValue {
                    value: value.clone(),
                    source: SettingSource::Auto,
                },
                provenance: build_theme_provenance(
                    theme_mode,
                    None,
                    &value,
                    SettingSource::Auto,
                    None,
                ),
            });
        }

        let fallback = resolve_default_theme(&resolver)?;
        return Ok(ThemeSelection {
            resolved: ResolvedValue {
                value: fallback.clone(),
                source: SettingSource::Fallback,
            },
            provenance: build_theme_provenance(
                theme_mode,
                None,
                &fallback,
                SettingSource::Fallback,
                Some(ThemeProvenanceFallback::AutoThemeResolutionFailed),
            ),
        });
    }

    let fallback = resolve_default_theme(&resolver)?;
    Ok(ThemeSelection {
        resolved: ResolvedValue {
            value: fallback.clone(),
            source: SettingSource::Fallback,
        },
        provenance: build_theme_provenance(
            theme_mode,
            None,
            &fallback,
            SettingSource::Fallback,
            Some(ThemeProvenanceFallback::DefaultThemeSelection),
        ),
    })
}

fn resolve_default_theme<F>(resolver: &F) -> Result<ResolvedThemeAsset, StartupError>
where
    F: Fn(&str) -> Result<ResolvedThemeAsset, ThemeAssetError>,
{
    resolver("neverforest").map_err(StartupError::ThemeAsset)
}

fn build_theme_provenance(
    theme_mode: &ResolvedValue<ThemeMode>,
    requested_theme: Option<(&str, SettingSource)>,
    resolved_theme: &ResolvedThemeAsset,
    resolved_theme_source: SettingSource,
    fallback: Option<ThemeProvenanceFallback>,
) -> ThemeProvenance {
    let (requested_theme, requested_theme_source) = requested_theme
        .map(|(requested, source)| (Some(requested.to_owned()), Some(source)))
        .unwrap_or((None, None));

    ThemeProvenance {
        theme_mode: theme_mode.value,
        theme_mode_source: theme_mode.source,
        requested_theme,
        requested_theme_source,
        resolved_theme: resolved_theme_label(resolved_theme),
        resolved_theme_source,
        resolved_theme_kind: resolved_theme_kind(resolved_theme),
        fallback,
    }
}

fn resolved_theme_label(theme: &ResolvedThemeAsset) -> String {
    match &theme.source {
        ThemeAssetSource::BuiltIn(asset) => asset.canonical_name.to_owned(),
        ThemeAssetSource::Path(path) => path.display().to_string(),
    }
}

fn resolved_theme_kind(theme: &ResolvedThemeAsset) -> ThemeProvenanceKind {
    match &theme.source {
        ThemeAssetSource::BuiltIn(_) => ThemeProvenanceKind::BuiltIn,
        ThemeAssetSource::Path(_) => ThemeProvenanceKind::Path,
    }
}

fn resolve_syntax(
    cli: Option<&str>,
    config: Option<&str>,
    source_name: &str,
    syntax_set: &SyntaxSet,
) -> Result<ResolvedValue<ResolvedSyntax>, StartupError> {
    if let Some(requested) = normalize_optional(cli) {
        return resolve_syntax_request(requested, syntax_set, SettingSource::Cli);
    }

    if let Some(requested) = normalize_optional(config) {
        return resolve_syntax_request(requested, syntax_set, SettingSource::Config);
    }

    if let Some(syntax) = detect_syntax(source_name, syntax_set) {
        return Ok(ResolvedValue {
            value: ResolvedSyntax {
                requested: source_name.to_owned(),
                syntax_name: syntax.name.clone(),
            },
            source: SettingSource::Auto,
        });
    }

    let fallback = syntax_set
        .find_syntax_by_extension("md")
        .or_else(|| syntax_set.find_syntax_by_name("Markdown"))
        .expect("markdown syntax should exist");

    Ok(ResolvedValue {
        value: ResolvedSyntax {
            requested: "markdown".to_owned(),
            syntax_name: fallback.name.clone(),
        },
        source: SettingSource::Fallback,
    })
}

fn resolve_syntax_request(
    requested: &str,
    syntax_set: &SyntaxSet,
    source: SettingSource,
) -> Result<ResolvedValue<ResolvedSyntax>, StartupError> {
    let syntax = find_syntax(requested, syntax_set).ok_or_else(|| StartupError::UnknownSyntax {
        requested: requested.to_owned(),
    })?;

    Ok(ResolvedValue {
        value: ResolvedSyntax {
            requested: requested.to_owned(),
            syntax_name: syntax.name.clone(),
        },
        source,
    })
}

fn detect_syntax<'a>(source_name: &str, syntax_set: &'a SyntaxSet) -> Option<&'a SyntaxReference> {
    let path = Path::new(source_name);

    if let Some(file_name) = path.file_name().and_then(|value| value.to_str()) {
        if let Some(syntax) = syntax_set.find_syntax_by_token(file_name) {
            return Some(syntax);
        }
    }

    path.extension()
        .and_then(|value| value.to_str())
        .and_then(|value| find_syntax(value, syntax_set))
}

fn find_syntax<'a>(requested: &str, syntax_set: &'a SyntaxSet) -> Option<&'a SyntaxReference> {
    let trimmed = requested.trim();
    let token = trimmed.trim_start_matches('.');

    syntax_set
        .find_syntax_by_token(trimmed)
        .or_else(|| syntax_set.find_syntax_by_token(token))
        .or_else(|| syntax_set.find_syntax_by_name(trimmed))
        .or_else(|| syntax_set.find_syntax_by_extension(token))
}

fn normalize_optional(value: Option<&str>) -> Option<&str> {
    value.and_then(|candidate| {
        let trimmed = candidate.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed)
        }
    })
}

fn auto_theme_name(theme_mode: ThemeMode) -> Option<&'static str> {
    match theme_mode {
        ThemeMode::Auto => None,
        ThemeMode::Light => Some("catppuccin-latte"),
        ThemeMode::Dark => Some("catppuccin-mocha"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cli_from(args: &[&str]) -> Cli {
        Cli::parse_from(args)
    }

    fn resolved_theme_mode(value: ThemeMode, source: SettingSource) -> ResolvedValue<ThemeMode> {
        ResolvedValue { value, source }
    }

    #[test]
    fn cli_theme_wins_over_config() {
        let theme = resolve_theme(
            Some("mocha"),
            Some("latte"),
            &resolved_theme_mode(ThemeMode::Light, SettingSource::Cli),
        )
        .unwrap();
        assert_eq!(theme.resolved.source, SettingSource::Cli);
        assert_eq!(theme.resolved.value.requested, "mocha");
        assert_eq!(theme.provenance.requested_theme.as_deref(), Some("mocha"));
        assert_eq!(
            theme.provenance.requested_theme_source,
            Some(SettingSource::Cli)
        );
    }

    #[test]
    fn config_theme_wins_over_auto_theme() {
        let theme = resolve_theme(
            None,
            Some("latte"),
            &resolved_theme_mode(ThemeMode::Dark, SettingSource::Config),
        )
        .unwrap();
        assert_eq!(theme.resolved.source, SettingSource::Config);
        assert_eq!(theme.resolved.value.requested, "latte");
        assert_eq!(theme.provenance.fallback, None);
    }

    #[test]
    fn dark_mode_gets_auto_dark_theme() {
        let theme = resolve_theme(
            None,
            None,
            &resolved_theme_mode(ThemeMode::Dark, SettingSource::Config),
        )
        .unwrap();
        assert_eq!(theme.resolved.source, SettingSource::Auto);
        assert_eq!(theme.resolved.value.requested, "catppuccin-mocha");
        assert_eq!(theme.provenance.resolved_theme, "catppuccin-mocha");
        assert_eq!(theme.provenance.fallback, None);
    }

    #[test]
    fn auto_mode_falls_back_to_neverforest() {
        let theme = resolve_theme(
            None,
            None,
            &resolved_theme_mode(ThemeMode::Auto, SettingSource::Auto),
        )
        .unwrap();
        assert_eq!(theme.resolved.source, SettingSource::Fallback);
        assert_eq!(theme.resolved.value.requested, "neverforest");
        assert_eq!(
            theme.provenance.fallback,
            Some(ThemeProvenanceFallback::DefaultThemeSelection)
        );
    }

    #[test]
    fn explicit_theme_errors_do_not_fallback() {
        let error = resolve_theme(
            Some("missing-theme"),
            None,
            &resolved_theme_mode(ThemeMode::Dark, SettingSource::Auto),
        )
        .unwrap_err();
        assert!(matches!(error, StartupError::ThemeAsset(_)));
    }

    #[test]
    fn auto_theme_failure_uses_neverforest_fallback() {
        let theme = resolve_theme_with(
            None,
            None,
            &resolved_theme_mode(ThemeMode::Dark, SettingSource::Auto),
            |requested| match requested {
                "catppuccin-mocha" => Err(ThemeAssetError::BuiltInNotFound {
                    requested: requested.to_owned(),
                }),
                other => resolve_theme_asset(other),
            },
        )
        .unwrap();

        assert_eq!(theme.resolved.source, SettingSource::Fallback);
        assert_eq!(theme.resolved.value.requested, "neverforest");
        assert_eq!(
            theme.provenance.fallback,
            Some(ThemeProvenanceFallback::AutoThemeResolutionFailed)
        );
    }

    #[test]
    fn theme_provenance_serializes_for_logs() {
        let theme = resolve_theme(
            None,
            Some("mocha"),
            &resolved_theme_mode(ThemeMode::Dark, SettingSource::Config),
        )
        .unwrap();
        let json = serde_json::to_string(&theme.provenance).unwrap();

        assert!(json.contains("\"resolved_theme\":\"catppuccin-mocha\""));
        assert!(json.contains("\"resolved_theme_kind\":\"built_in\""));
    }

    #[test]
    fn syntax_override_accepts_extension() {
        let syntax_set = SyntaxSet::load_defaults_newlines();
        let syntax = resolve_syntax(Some("rs"), None, "notes.md", &syntax_set).unwrap();
        assert_eq!(syntax.source, SettingSource::Cli);
        assert_eq!(syntax.value.syntax_name, "Rust");
    }

    #[test]
    fn source_name_auto_detects_syntax() {
        let syntax_set = SyntaxSet::load_defaults_newlines();
        let syntax = resolve_syntax(None, None, "src/main.rs", &syntax_set).unwrap();
        assert_eq!(syntax.source, SettingSource::Auto);
        assert_eq!(syntax.value.syntax_name, "Rust");
    }

    #[test]
    fn stdin_falls_back_to_markdown() {
        let syntax_set = SyntaxSet::load_defaults_newlines();
        let syntax = resolve_syntax(None, None, "[stdin]", &syntax_set).unwrap();
        assert_eq!(syntax.source, SettingSource::Fallback);
        assert_eq!(syntax.value.syntax_name, "Markdown");
    }

    #[test]
    fn cli_parser_accepts_new_flags() {
        let cli = cli_from(&[
            "anno",
            "--theme",
            "mocha",
            "--theme-mode",
            "dark",
            "--syntax",
            "rust",
            "demo.md",
        ]);

        assert_eq!(cli.theme.as_deref(), Some("mocha"));
        assert_eq!(cli.theme_mode, Some(ThemeMode::Dark));
        assert_eq!(cli.syntax.as_deref(), Some("rust"));
        assert_eq!(cli.file.as_deref(), Some("demo.md"));
    }
}
