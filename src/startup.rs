use std::env;
use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use clap::{Parser, ValueEnum};
use serde::{Deserialize, Serialize};
use syntect::parsing::{SyntaxReference, SyntaxSet};

use crate::highlight::theme_assets::{
    ResolvedThemeAsset, ThemeAssetError, ThemeAssetKind as ThemeProvenanceKind, resolve_theme_asset,
};
use crate::input::SourceMetadata;
use crate::tui::theme::{DocumentBackground, ThemeOverlayOverrides};

#[path = "startup/theme_resolution.rs"]
mod theme_resolution;

#[derive(Debug, Parser)]
#[command(name = "anno", about = "Annotate markdown files in the terminal")]
pub struct Cli {
    /// Output format used when exporting annotations on :q
    #[arg(long = "export-format", value_enum, default_value_t = ExportFormat::Agent)]
    pub export_format: ExportFormat,

    /// Built-in theme name or path to a .tmTheme file
    #[arg(long)]
    pub theme: Option<String>,

    /// Theme mode preference used for auto theme selection
    #[arg(long = "theme-mode", value_enum)]
    pub theme_mode: Option<ThemeMode>,

    /// Syntax name or extension override for highlighting
    #[arg(long)]
    pub syntax: Option<String>,

    /// Display title shown in the status bar
    #[arg(long)]
    pub title: Option<String>,

    /// Write annotation output to a file instead of stdout
    #[arg(long = "output-file")]
    pub output_file: Option<String>,

    /// Text file to annotate
    pub file: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ExportFormat {
    Agent,
    Json,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ThemeMode {
    Auto,
    Light,
    Dark,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum LineNumberMode {
    Relative,
    Absolute,
}

impl Default for LineNumberMode {
    fn default() -> Self {
        Self::Relative
    }
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ResolvedValue<T> {
    pub value: T,
    pub source: SettingSource,
}

impl<T> ResolvedValue<T> {
    fn new(value: T, source: SettingSource) -> Self {
        Self { value, source }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ResolvedSyntax {
    pub requested: String,
    pub syntax_name: String,
}

impl ResolvedSyntax {
    pub fn resolve_in<'a>(&self, syntax_set: &'a SyntaxSet) -> &'a SyntaxReference {
        find_syntax(&self.requested, syntax_set)
            .or_else(|| syntax_set.find_syntax_by_name(&self.syntax_name))
            .unwrap_or_else(|| syntax_set.find_syntax_plain_text())
    }
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
    pub export_format: ExportFormat,
    pub title: Option<String>,
    pub document_background: DocumentBackground,
    pub line_number_mode: LineNumberMode,
    pub theme_mode: ResolvedValue<ThemeMode>,
    pub theme: ResolvedValue<ResolvedThemeAsset>,
    pub theme_provenance: ThemeProvenance,
    pub syntax: ResolvedValue<ResolvedSyntax>,
    pub app_theme_overlays: ThemeOverlayOverrides,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct StartupLog {
    pub source: SourceMetadata,
    pub export_format: ExportFormat,
    pub title: Option<String>,
    pub theme: ThemeProvenance,
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
    #[serde(default)]
    background: DocumentBackground,
    #[serde(default, alias = "lineNumbers", alias = "line-numbers")]
    line_numbers: LineNumberMode,
    #[serde(default, alias = "themeMode", alias = "theme-mode")]
    theme_mode: Option<ThemeMode>,
    #[serde(default)]
    syntax: Option<String>,
    #[serde(default)]
    title: Option<String>,
    #[serde(default, alias = "appTheme", alias = "app-theme")]
    app_theme: ThemeOverlayOverrides,
}

impl StartupSettings {
    pub fn resolve(
        cli: &Cli,
        source: &SourceMetadata,
        content: &str,
    ) -> Result<Self, StartupError> {
        let config = load_settings_file()?;
        let syntax_set = SyntaxSet::load_defaults_newlines();

        let theme_mode = resolve_theme_mode(cli.theme_mode, config.theme_mode);

        let theme = theme_resolution::resolve_theme(
            cli.theme.as_deref(),
            config.theme.as_deref(),
            &theme_mode,
            resolve_theme_asset,
        )?;
        let syntax = resolve_syntax(
            cli.syntax.as_deref(),
            config.syntax.as_deref(),
            source,
            content,
            &syntax_set,
        )?;

        Ok(Self {
            export_format: cli.export_format,
            title: cli.title.clone().or(config.title),
            document_background: config.background,
            line_number_mode: config.line_numbers,
            theme_mode,
            theme: theme.resolved,
            theme_provenance: theme.provenance,
            syntax,
            app_theme_overlays: config.app_theme,
        })
    }

    pub fn startup_log(&self, source: &SourceMetadata) -> StartupLog {
        StartupLog {
            source: source.clone(),
            export_format: self.export_format,
            title: self.title.clone(),
            theme: self.theme_provenance.clone(),
            syntax: self.syntax.clone(),
        }
    }

    pub fn startup_log_json(&self, source: &SourceMetadata) -> Result<String, serde_json::Error> {
        serde_json::to_string(&self.startup_log(source))
    }
}

pub fn should_log_startup() -> bool {
    env::var_os("ANNO_LOG_STARTUP").is_some_and(|value| {
        let normalized = value.to_string_lossy().trim().to_ascii_lowercase();
        !matches!(normalized.as_str(), "" | "0" | "false" | "no" | "off")
    })
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

fn resolve_syntax(
    cli: Option<&str>,
    config: Option<&str>,
    source: &SourceMetadata,
    content: &str,
    syntax_set: &SyntaxSet,
) -> Result<ResolvedValue<ResolvedSyntax>, StartupError> {
    if let Some((requested, source)) = resolve_requested_string(cli, config) {
        return resolve_syntax_request(requested, syntax_set, source);
    }

    if let Some((requested, syntax)) = detect_syntax(source, content, syntax_set) {
        return Ok(ResolvedValue::new(
            ResolvedSyntax {
                requested,
                syntax_name: syntax.name.clone(),
            },
            SettingSource::Auto,
        ));
    }

    let fallback = syntax_set.find_syntax_plain_text();

    Ok(ResolvedValue::new(
        ResolvedSyntax {
            requested: "plain text".to_owned(),
            syntax_name: fallback.name.clone(),
        },
        SettingSource::Fallback,
    ))
}

fn resolve_syntax_request(
    requested: &str,
    syntax_set: &SyntaxSet,
    source: SettingSource,
) -> Result<ResolvedValue<ResolvedSyntax>, StartupError> {
    let syntax = find_syntax(requested, syntax_set).ok_or_else(|| StartupError::UnknownSyntax {
        requested: requested.to_owned(),
    })?;

    Ok(ResolvedValue::new(
        ResolvedSyntax {
            requested: requested.to_owned(),
            syntax_name: syntax.name.clone(),
        },
        source,
    ))
}

fn detect_syntax<'a>(
    source: &SourceMetadata,
    content: &str,
    syntax_set: &'a SyntaxSet,
) -> Option<(String, &'a SyntaxReference)> {
    if let Some(source_name) = source.syntax_hint.as_deref() {
        let path = Path::new(source_name);

        if let Some(file_name) = path.file_name().and_then(|value| value.to_str())
            && let Some(syntax) = syntax_set.find_syntax_by_token(file_name)
        {
            return Some((file_name.to_owned(), syntax));
        }

        if let Some(extension) = path.extension().and_then(|value| value.to_str())
            && let Some(syntax) = find_syntax(extension, syntax_set)
        {
            return Some((extension.to_owned(), syntax));
        }
    }

    content
        .lines()
        .next()
        .and_then(|line| syntax_set.find_syntax_by_first_line(line))
        .map(|syntax| {
            (
                content.lines().next().unwrap_or_default().to_owned(),
                syntax,
            )
        })
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

fn resolve_theme_mode(
    cli: Option<ThemeMode>,
    config: Option<ThemeMode>,
) -> ResolvedValue<ThemeMode> {
    cli.map(|value| ResolvedValue::new(value, SettingSource::Cli))
        .or_else(|| config.map(|value| ResolvedValue::new(value, SettingSource::Config)))
        .unwrap_or_else(|| ResolvedValue::new(ThemeMode::Auto, SettingSource::Auto))
}

fn resolve_requested_string<'a>(
    cli: Option<&'a str>,
    config: Option<&'a str>,
) -> Option<(&'a str, SettingSource)> {
    normalize_optional(cli)
        .map(|value| (value, SettingSource::Cli))
        .or_else(|| normalize_optional(config).map(|value| (value, SettingSource::Config)))
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::panic::{self, AssertUnwindSafe};
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::*;
    use crate::highlight::theme_assets::ThemeAssetError;
    use crate::input::SourceMetadata;
    use crate::startup::theme_resolution::resolve_theme;
    use crate::test_support::env_lock;

    fn cli_from(args: &[&str]) -> Cli {
        Cli::parse_from(args)
    }

    fn with_temp_home<F>(settings: Option<&str>, test: F)
    where
        F: FnOnce(),
    {
        let _guard = env_lock();
        let original_home = env::var_os("HOME");
        let temp_home = std::env::temp_dir().join(format!(
            "anno-startup-tests-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let config_dir = temp_home.join(".config/anno");
        fs::create_dir_all(&config_dir).unwrap();

        if let Some(contents) = settings {
            fs::write(config_dir.join("settings.json"), contents).unwrap();
        }

        unsafe { env::set_var("HOME", &temp_home) };

        let result = panic::catch_unwind(AssertUnwindSafe(test));

        if let Some(home) = original_home {
            unsafe { env::set_var("HOME", home) };
        } else {
            unsafe { env::remove_var("HOME") };
        }

        fs::remove_dir_all(&temp_home).unwrap();

        if let Err(error) = result {
            panic::resume_unwind(error);
        }
    }

    fn file_source(name: &str) -> SourceMetadata {
        SourceMetadata {
            display_name: name.to_owned(),
            syntax_hint: Some(name.to_owned()),
        }
    }

    fn resolved_theme_mode(value: ThemeMode, source: SettingSource) -> ResolvedValue<ThemeMode> {
        ResolvedValue::new(value, source)
    }

    #[test]
    fn cli_theme_wins_over_config() {
        let theme = resolve_theme(
            Some("mocha"),
            Some("latte"),
            &resolved_theme_mode(ThemeMode::Light, SettingSource::Cli),
            resolve_theme_asset,
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
            resolve_theme_asset,
        )
        .unwrap();
        assert_eq!(theme.resolved.source, SettingSource::Config);
        assert_eq!(theme.resolved.value.requested, "latte");
        assert_eq!(theme.provenance.fallback, None);
    }

    #[test]
    fn startup_settings_resolve_prefers_cli_over_settings_file() {
        with_temp_home(
            Some(
                r##"{
                    "theme": "latte",
                    "line_numbers": "absolute",
                    "theme_mode": "light",
                    "syntax": "python",
                    "app_theme": {
                        "cursor": { "bg": "#112233" }
                    }
                }"##,
            ),
            || {
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

                let startup = StartupSettings::resolve(&cli, &file_source("demo.md"), "").unwrap();

                assert_eq!(startup.line_number_mode, LineNumberMode::Absolute);
                assert_eq!(startup.theme_mode.value, ThemeMode::Dark);
                assert_eq!(startup.export_format, ExportFormat::Agent);
                assert_eq!(startup.title.as_deref(), None);
                assert_eq!(startup.theme_mode.source, SettingSource::Cli);
                assert_eq!(startup.theme.source, SettingSource::Cli);
                assert_eq!(startup.document_background, DocumentBackground::Theme);
                assert_eq!(startup.theme.value.requested, "mocha");
                assert_eq!(
                    startup.theme_provenance.requested_theme.as_deref(),
                    Some("mocha")
                );
                assert_eq!(startup.syntax.source, SettingSource::Cli);
                assert_eq!(startup.syntax.value.syntax_name, "Rust");
                assert_eq!(
                    startup.app_theme_overlays.cursor.bg,
                    Some(crate::tui::theme::ThemeColor::new(17, 34, 51))
                );
            },
        );
    }

    #[test]
    fn startup_settings_resolve_uses_neverforest_when_no_theme_is_selected() {
        with_temp_home(None, || {
            let cli = cli_from(&["anno", "demo.md"]);

            let startup = StartupSettings::resolve(&cli, &file_source("demo.md"), "").unwrap();

            assert_eq!(startup.line_number_mode, LineNumberMode::Relative);
            assert_eq!(startup.document_background, DocumentBackground::Theme);
            assert_eq!(startup.theme.source, SettingSource::Fallback);
            assert_eq!(startup.theme.value.requested, "neverforest");
            assert_eq!(
                startup.theme_provenance.fallback,
                Some(ThemeProvenanceFallback::DefaultThemeSelection)
            );
        });
    }

    #[test]
    fn dark_mode_gets_auto_dark_theme() {
        let theme = resolve_theme(
            None,
            None,
            &resolved_theme_mode(ThemeMode::Dark, SettingSource::Config),
            resolve_theme_asset,
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
            resolve_theme_asset,
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
            resolve_theme_asset,
        )
        .unwrap_err();
        assert!(matches!(error, StartupError::ThemeAsset(_)));
    }

    #[test]
    fn startup_settings_surface_invalid_configured_theme_errors() {
        with_temp_home(Some(r#"{ "theme": "missing-theme" }"#), || {
            let cli = cli_from(&["anno", "demo.md"]);
            let error = StartupSettings::resolve(&cli, &file_source("demo.md"), "").unwrap_err();

            assert!(matches!(
                error,
                StartupError::ThemeAsset(ThemeAssetError::BuiltInNotFound { requested })
                if requested == "missing-theme"
            ));
        });
    }

    #[test]
    fn auto_theme_failure_uses_neverforest_fallback() {
        let theme = resolve_theme(
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
            resolve_theme_asset,
        )
        .unwrap();
        let json = serde_json::to_string(&theme.provenance).unwrap();

        assert!(json.contains("\"resolved_theme\":\"catppuccin-mocha\""));
        assert!(json.contains("\"resolved_theme_kind\":\"built_in\""));
    }

    #[test]
    fn startup_log_serializes_source_theme_and_syntax() {
        with_temp_home(Some(r#"{ "theme": "mocha", "syntax": "rust" }"#), || {
            let cli = cli_from(&[
                "anno",
                "--export-format",
                "json",
                "--title",
                "Demo",
                "demo.md",
            ]);
            let source = file_source("demo.md");
            let startup = StartupSettings::resolve(&cli, &source, "").unwrap();

            let json = startup.startup_log_json(&source).unwrap();

            assert!(json.contains("\"display_name\":\"demo.md\""));
            assert!(json.contains("\"export_format\":\"json\""));
            assert!(json.contains("\"title\":\"Demo\""));
            assert!(json.contains("\"resolved_theme\":\"catppuccin-mocha\""));
            assert!(json.contains("\"syntax_name\":\"Rust\""));
            assert!(json.contains("\"source\":\"config\""));
        });
    }

    #[test]
    fn startup_logging_env_var_accepts_common_truthy_values() {
        let _guard = env_lock();
        unsafe { env::set_var("ANNO_LOG_STARTUP", "true") };
        assert!(should_log_startup());

        unsafe { env::set_var("ANNO_LOG_STARTUP", "1") };
        assert!(should_log_startup());

        unsafe { env::remove_var("ANNO_LOG_STARTUP") };
    }

    #[test]
    fn startup_logging_env_var_rejects_common_falsey_values() {
        let _guard = env_lock();

        for value in ["", "0", "false", "no", "off"] {
            unsafe { env::set_var("ANNO_LOG_STARTUP", value) };
            assert!(
                !should_log_startup(),
                "{value} should disable startup logging"
            );
        }

        unsafe { env::remove_var("ANNO_LOG_STARTUP") };
    }

    #[test]
    fn syntax_override_accepts_extension() {
        let syntax_set = SyntaxSet::load_defaults_newlines();
        let syntax = resolve_syntax(
            Some("rs"),
            None,
            &SourceMetadata {
                display_name: "notes.md".to_owned(),
                syntax_hint: Some("notes.md".to_owned()),
            },
            "",
            &syntax_set,
        )
        .unwrap();
        assert_eq!(syntax.source, SettingSource::Cli);
        assert_eq!(syntax.value.syntax_name, "Rust");
    }

    #[test]
    fn source_name_auto_detects_syntax() {
        let syntax_set = SyntaxSet::load_defaults_newlines();
        let syntax = resolve_syntax(
            None,
            None,
            &SourceMetadata {
                display_name: "src/main.rs".to_owned(),
                syntax_hint: Some("src/main.rs".to_owned()),
            },
            "",
            &syntax_set,
        )
        .unwrap();
        assert_eq!(syntax.source, SettingSource::Auto);
        assert_eq!(syntax.value.syntax_name, "Rust");
    }

    #[test]
    fn syntax_override_accepts_dot_extension() {
        let syntax_set = SyntaxSet::load_defaults_newlines();
        let syntax =
            resolve_syntax(Some(".rs"), None, &file_source("demo.txt"), "", &syntax_set).unwrap();

        assert_eq!(syntax.source, SettingSource::Cli);
        assert_eq!(syntax.value.requested, ".rs");
        assert_eq!(syntax.value.syntax_name, "Rust");
    }

    #[test]
    fn stdin_shebang_auto_detects_syntax() {
        let syntax_set = SyntaxSet::load_defaults_newlines();
        let syntax = resolve_syntax(
            None,
            None,
            &SourceMetadata {
                display_name: "[stdin]".to_owned(),
                syntax_hint: None,
            },
            "#!/usr/bin/env python\nprint('hi')\n",
            &syntax_set,
        )
        .unwrap();
        assert_eq!(syntax.source, SettingSource::Auto);
        assert_eq!(syntax.value.syntax_name, "Python");
    }

    #[test]
    fn stdin_falls_back_to_plain_text() {
        let syntax_set = SyntaxSet::load_defaults_newlines();
        let syntax = resolve_syntax(
            None,
            None,
            &SourceMetadata {
                display_name: "[stdin]".to_owned(),
                syntax_hint: None,
            },
            "just some text",
            &syntax_set,
        )
        .unwrap();
        assert_eq!(syntax.source, SettingSource::Fallback);
        assert_eq!(syntax.value.syntax_name, "Plain Text");
    }

    #[test]
    fn config_syntax_wins_over_detected_source_hint() {
        let syntax_set = SyntaxSet::load_defaults_newlines();
        let syntax = resolve_syntax(
            None,
            Some("rust"),
            &SourceMetadata {
                display_name: "notes.txt".to_owned(),
                syntax_hint: Some("notes.txt".to_owned()),
            },
            "",
            &syntax_set,
        )
        .unwrap();
        assert_eq!(syntax.source, SettingSource::Config);
        assert_eq!(syntax.value.syntax_name, "Rust");
    }

    #[test]
    fn cli_parser_accepts_new_flags() {
        let cli = cli_from(&[
            "anno",
            "--export-format",
            "json",
            "--theme",
            "mocha",
            "--theme-mode",
            "dark",
            "--syntax",
            "rust",
            "--title",
            "My Review",
            "demo.md",
        ]);

        assert_eq!(cli.export_format, ExportFormat::Json);
        assert_eq!(cli.theme.as_deref(), Some("mocha"));
        assert_eq!(cli.theme_mode, Some(ThemeMode::Dark));
        assert_eq!(cli.syntax.as_deref(), Some("rust"));
        assert_eq!(cli.title.as_deref(), Some("My Review"));
        assert_eq!(cli.output_file, None);
        assert_eq!(cli.file.as_deref(), Some("demo.md"));
    }

    #[test]
    fn startup_settings_resolve_uses_cli_title_over_config() {
        with_temp_home(Some(r#"{ "title": "Config Title" }"#), || {
            let cli = cli_from(&["anno", "--title", "CLI Title", "demo.md"]);

            let startup = StartupSettings::resolve(&cli, &file_source("demo.md"), "").unwrap();

            assert_eq!(startup.title.as_deref(), Some("CLI Title"));
        });
    }

    #[test]
    fn startup_settings_resolve_uses_config_title_when_cli_missing() {
        with_temp_home(Some(r#"{ "title": "Config Title" }"#), || {
            let cli = cli_from(&["anno", "demo.md"]);

            let startup = StartupSettings::resolve(&cli, &file_source("demo.md"), "").unwrap();

            assert_eq!(startup.title.as_deref(), Some("Config Title"));
        });
    }

    #[test]
    fn cli_parser_accepts_output_file_flag() {
        let cli = cli_from(&["anno", "--output-file", "/tmp/out.json", "demo.md"]);

        assert_eq!(cli.output_file.as_deref(), Some("/tmp/out.json"));
        assert_eq!(cli.file.as_deref(), Some("demo.md"));
    }

    #[test]
    fn startup_settings_resolve_uses_agent_format_by_default() {
        with_temp_home(None, || {
            let cli = cli_from(&["anno", "demo.md"]);

            let startup = StartupSettings::resolve(&cli, &file_source("demo.md"), "").unwrap();

            assert_eq!(startup.export_format, ExportFormat::Agent);
        });
    }

    #[test]
    fn startup_settings_resolve_uses_cli_export_format() {
        with_temp_home(None, || {
            let cli = cli_from(&["anno", "--export-format", "json", "demo.md"]);

            let startup = StartupSettings::resolve(&cli, &file_source("demo.md"), "").unwrap();

            assert_eq!(startup.export_format, ExportFormat::Json);
        });
    }

    #[test]
    fn settings_file_accepts_app_theme_overrides() {
        let settings: SettingsFile = serde_json::from_str(
            r##"{
                "background": "default",
                "app_theme": {
                    "cursor": { "bg": "#112233" },
                    "selection": { "underlined": true },
                    "annotation": { "fg": "#abcdef" }
                }
            }"##,
        )
        .unwrap();

        assert_eq!(settings.background, DocumentBackground::Default);
        assert_eq!(
            settings.app_theme.cursor.bg,
            Some(crate::tui::theme::ThemeColor::new(17, 34, 51))
        );
        assert_eq!(settings.app_theme.selection.underlined, Some(true));
        assert_eq!(
            settings.app_theme.annotation.fg,
            Some(crate::tui::theme::ThemeColor::new(171, 205, 239))
        );
    }

    #[test]
    fn startup_settings_resolve_defaults_line_numbers_to_relative() {
        with_temp_home(None, || {
            let cli = cli_from(&["anno", "demo.md"]);

            let startup = StartupSettings::resolve(&cli, &file_source("demo.md"), "").unwrap();

            assert_eq!(startup.line_number_mode, LineNumberMode::Relative);
        });
    }

    #[test]
    fn settings_file_accepts_line_numbers_field_aliases() {
        for field_name in ["line_numbers", "lineNumbers", "line-numbers"] {
            let settings: SettingsFile =
                serde_json::from_str(&format!(r#"{{ "{field_name}": "absolute" }}"#)).unwrap();

            assert_eq!(settings.line_numbers, LineNumberMode::Absolute);
        }
    }

    #[test]
    fn settings_file_rejects_widget_style_keys_in_app_theme() {
        let error = serde_json::from_str::<SettingsFile>(
            r##"{
                "app_theme": {
                    "status_bar": { "fg": "#abcdef" }
                }
            }"##,
        )
        .unwrap_err();

        let message = error.to_string();
        assert!(message.contains("status_bar"));
        assert!(message.contains("cursor"));
        assert!(message.contains("selection"));
        assert!(message.contains("annotation"));
    }

    #[test]
    fn startup_settings_resolve_accepts_default_background() {
        with_temp_home(Some(r#"{ "background": "default" }"#), || {
            let cli = cli_from(&["anno", "demo.md"]);

            let startup = StartupSettings::resolve(&cli, &file_source("demo.md"), "").unwrap();

            assert_eq!(startup.document_background, DocumentBackground::Default);
        });
    }
}
