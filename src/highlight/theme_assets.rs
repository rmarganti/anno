use std::fmt;
use std::io::Cursor;
use std::path::{Component, Path, PathBuf};

use serde::Serialize;
use syntect::highlighting::{Theme, ThemeSet};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuiltInThemeMode {
    Light,
    Dark,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BuiltInThemeAsset {
    pub canonical_name: &'static str,
    pub aliases: &'static [&'static str],
    pub file_name: &'static str,
    pub mode: BuiltInThemeMode,
    contents: &'static str,
}

impl BuiltInThemeAsset {
    pub fn names(self) -> impl Iterator<Item = &'static str> {
        std::iter::once(self.canonical_name).chain(self.aliases.iter().copied())
    }

    pub fn load(self) -> Result<Theme, ThemeAssetError> {
        let mut reader = Cursor::new(self.contents.as_bytes());
        ThemeSet::load_from_reader(&mut reader).map_err(ThemeAssetError::LoadBuiltIn)
    }
}

const CATPPUCCIN_LATTE: BuiltInThemeAsset = BuiltInThemeAsset {
    canonical_name: "catppuccin-latte",
    aliases: &["catppuccin latte", "latte", "catppuccin_latte"],
    file_name: "Catppuccin Latte.tmTheme",
    mode: BuiltInThemeMode::Light,
    contents: include_str!("themes/Catppuccin Latte.tmTheme"),
};

const CATPPUCCIN_MOCHA: BuiltInThemeAsset = BuiltInThemeAsset {
    canonical_name: "catppuccin-mocha",
    aliases: &["catppuccin mocha", "mocha", "catppuccin_mocha"],
    file_name: "Catppuccin Mocha.tmTheme",
    mode: BuiltInThemeMode::Dark,
    contents: include_str!("themes/Catppuccin Mocha.tmTheme"),
};

const NEVERFOREST: BuiltInThemeAsset = BuiltInThemeAsset {
    canonical_name: "neverforest",
    aliases: &[],
    file_name: "neverforest.tmTheme",
    mode: BuiltInThemeMode::Dark,
    contents: include_str!("themes/neverforest.tmTheme"),
};

const BUILT_INS: [BuiltInThemeAsset; 3] = [CATPPUCCIN_LATTE, CATPPUCCIN_MOCHA, NEVERFOREST];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ThemeAssetKind {
    BuiltIn,
    Path,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ThemeAssetSource {
    BuiltIn(&'static BuiltInThemeAsset),
    Path(PathBuf),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedThemeAsset {
    pub requested: String,
    pub source: ThemeAssetSource,
}

impl ResolvedThemeAsset {
    pub fn from_built_in_requested(asset: &'static BuiltInThemeAsset, requested: &str) -> Self {
        Self {
            requested: requested.to_owned(),
            source: ThemeAssetSource::BuiltIn(asset),
        }
    }

    pub fn load_theme(&self) -> Result<Theme, ThemeAssetError> {
        match &self.source {
            ThemeAssetSource::BuiltIn(asset) => asset.load(),
            ThemeAssetSource::Path(path) => {
                ThemeSet::get_theme(path).map_err(|source| ThemeAssetError::LoadPath {
                    path: path.clone(),
                    source,
                })
            }
        }
    }

    pub fn label(&self) -> String {
        match &self.source {
            ThemeAssetSource::BuiltIn(asset) => asset.canonical_name.to_owned(),
            ThemeAssetSource::Path(path) => path.display().to_string(),
        }
    }

    pub fn kind(&self) -> ThemeAssetKind {
        match self.source {
            ThemeAssetSource::BuiltIn(_) => ThemeAssetKind::BuiltIn,
            ThemeAssetSource::Path(_) => ThemeAssetKind::Path,
        }
    }
}

#[derive(Debug)]
pub enum ThemeAssetError {
    BuiltInNotFound {
        requested: String,
    },
    PathNotFound {
        path: PathBuf,
    },
    PathIsDirectory {
        path: PathBuf,
    },
    LoadBuiltIn(syntect::LoadingError),
    LoadPath {
        path: PathBuf,
        source: syntect::LoadingError,
    },
}

impl fmt::Display for ThemeAssetError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BuiltInNotFound { requested } => {
                write!(f, "unknown built-in theme '{requested}'")
            }
            Self::PathNotFound { path } => write!(f, "theme file not found: {}", path.display()),
            Self::PathIsDirectory { path } => {
                write!(f, "theme path points to a directory: {}", path.display())
            }
            Self::LoadBuiltIn(source) => write!(f, "failed to load built-in theme: {source}"),
            Self::LoadPath { path, source } => {
                write!(f, "failed to load theme file {}: {source}", path.display())
            }
        }
    }
}

impl std::error::Error for ThemeAssetError {}

pub fn built_in_theme_assets() -> &'static [BuiltInThemeAsset] {
    &BUILT_INS
}

pub fn find_built_in_theme(name: &str) -> Option<&'static BuiltInThemeAsset> {
    let normalized = normalize_theme_name(name);
    BUILT_INS.iter().find(|asset| {
        asset
            .names()
            .any(|candidate| normalize_theme_name(candidate) == normalized)
    })
}

pub fn default_fallback_theme_asset() -> &'static BuiltInThemeAsset {
    &NEVERFOREST
}

pub fn default_fallback_resolved_theme() -> ResolvedThemeAsset {
    let asset = default_fallback_theme_asset();
    ResolvedThemeAsset::from_built_in_requested(asset, asset.canonical_name)
}

pub fn resolve_theme_asset(requested: &str) -> Result<ResolvedThemeAsset, ThemeAssetError> {
    if looks_like_theme_path(requested) {
        let path = expand_tilde(requested);
        if !path.exists() {
            return Err(ThemeAssetError::PathNotFound { path });
        }
        if path.is_dir() {
            return Err(ThemeAssetError::PathIsDirectory { path });
        }
        return Ok(ResolvedThemeAsset {
            requested: requested.to_owned(),
            source: ThemeAssetSource::Path(path),
        });
    }

    let asset = find_built_in_theme(requested).ok_or_else(|| ThemeAssetError::BuiltInNotFound {
        requested: requested.to_owned(),
    })?;

    Ok(ResolvedThemeAsset::from_built_in_requested(
        asset, requested,
    ))
}

fn normalize_theme_name(input: &str) -> String {
    let mut normalized = String::with_capacity(input.len());
    let mut last_was_dash = false;

    for ch in input.trim().chars() {
        let mapped = match ch {
            'A'..='Z' => ch.to_ascii_lowercase(),
            'a'..='z' | '0'..='9' => ch,
            ' ' | '_' | '-' => '-',
            _ => continue,
        };

        if mapped == '-' {
            if !normalized.is_empty() && !last_was_dash {
                normalized.push('-');
            }
            last_was_dash = true;
        } else {
            normalized.push(mapped);
            last_was_dash = false;
        }
    }

    normalized.trim_matches('-').to_owned()
}

fn expand_tilde(input: &str) -> PathBuf {
    let trimmed = input.trim();

    if trimmed == "~" {
        return std::env::var_os("HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from(trimmed));
    }

    if let Some(rest) = trimmed.strip_prefix("~/") {
        if let Some(home) = std::env::var_os("HOME") {
            return PathBuf::from(home).join(rest);
        }
    }

    PathBuf::from(trimmed)
}

fn looks_like_theme_path(input: &str) -> bool {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return false;
    }

    if trimmed.starts_with('~') || trimmed.contains('/') || trimmed.contains('\\') {
        return true;
    }

    let path = Path::new(trimmed);
    if path.is_absolute() || path.extension().is_some() {
        return true;
    }

    let mut components = path.components();
    match (components.next(), components.next()) {
        (Some(Component::Normal(_)), None) => false,
        (Some(_), _) => true,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;

    #[test]
    fn built_ins_are_addressable_by_canonical_name() {
        let names: Vec<_> = built_in_theme_assets()
            .iter()
            .map(|asset| asset.canonical_name)
            .collect();
        assert_eq!(
            names,
            vec!["catppuccin-latte", "catppuccin-mocha", "neverforest"]
        );
    }

    #[test]
    fn default_fallback_theme_is_centralized() {
        let fallback = default_fallback_resolved_theme();
        assert_eq!(
            fallback.requested,
            default_fallback_theme_asset().canonical_name
        );
        assert_eq!(fallback.label(), "neverforest");
        assert_eq!(fallback.kind(), ThemeAssetKind::BuiltIn);
    }

    #[test]
    fn aliases_normalize_to_same_built_in() {
        let latte = find_built_in_theme("Catppuccin Latte").unwrap();
        assert_eq!(latte.canonical_name, "catppuccin-latte");

        let mocha = find_built_in_theme("catppuccin_mocha").unwrap();
        assert_eq!(mocha.canonical_name, "catppuccin-mocha");

        let short = find_built_in_theme("mocha").unwrap();
        assert_eq!(short.canonical_name, "catppuccin-mocha");
    }

    #[test]
    fn plain_names_do_not_fallback_to_paths() {
        let error = resolve_theme_asset("missing-theme").unwrap_err();
        assert!(matches!(error, ThemeAssetError::BuiltInNotFound { .. }));
    }

    #[test]
    fn explicit_file_names_are_treated_as_paths() {
        let error = resolve_theme_asset("missing.tmTheme").unwrap_err();
        assert!(matches!(error, ThemeAssetError::PathNotFound { .. }));
    }

    #[test]
    fn explicit_relative_paths_are_treated_as_paths() {
        let error = resolve_theme_asset("themes/missing").unwrap_err();
        assert!(matches!(error, ThemeAssetError::PathNotFound { .. }));
    }

    #[test]
    fn existing_paths_resolve_as_paths() {
        let temp_path =
            std::env::temp_dir().join(format!("anno-theme-{}.tmTheme", std::process::id()));
        fs::write(&temp_path, include_str!("themes/Catppuccin Latte.tmTheme")).unwrap();

        let resolved = resolve_theme_asset(temp_path.to_str().unwrap()).unwrap();
        assert!(matches!(resolved.source, ThemeAssetSource::Path(_)));

        fs::remove_file(temp_path).unwrap();
    }

    #[test]
    fn tilde_paths_expand_to_home_directory() {
        let home = std::env::var("HOME").unwrap();
        let temp_dir = PathBuf::from(home.clone()).join(".config/anno-test");
        fs::create_dir_all(&temp_dir).unwrap();

        let temp_path = temp_dir.join(format!("theme-{}.tmTheme", std::process::id()));
        fs::write(&temp_path, include_str!("themes/Catppuccin Latte.tmTheme")).unwrap();

        let requested = format!("~/{}", temp_path.strip_prefix(home).unwrap().display());
        let resolved = resolve_theme_asset(&requested).unwrap();
        assert!(matches!(resolved.source, ThemeAssetSource::Path(_)));

        fs::remove_file(temp_path).unwrap();
        fs::remove_dir(temp_dir).unwrap();
    }

    #[test]
    fn built_in_themes_load_successfully() {
        for asset in built_in_theme_assets() {
            let theme = asset.load().unwrap();
            assert!(
                theme.name.is_some(),
                "{} should parse",
                asset.canonical_name
            );
        }
    }

    #[test]
    fn resolved_built_in_theme_loads_successfully() {
        let resolved = resolve_theme_asset("neverforest").unwrap();
        let theme = resolved.load_theme().unwrap();
        assert!(theme.name.is_some());
    }
}
