use crate::highlight::theme_assets::{
    ResolvedThemeAsset, ThemeAssetError, default_fallback_resolved_theme,
};

use super::{
    ResolvedValue, SettingSource, StartupError, ThemeMode, ThemeProvenance,
    ThemeProvenanceFallback, ThemeSelection, resolve_requested_string,
};

pub(super) fn resolve_theme<F>(
    cli: Option<&str>,
    config: Option<&str>,
    theme_mode: &ResolvedValue<ThemeMode>,
    resolver: F,
) -> Result<ThemeSelection, StartupError>
where
    F: Fn(&str) -> Result<ResolvedThemeAsset, ThemeAssetError>,
{
    if let Some((requested, source)) = resolve_requested_string(cli, config) {
        return resolve_requested_theme(requested, source, theme_mode, resolver);
    }

    if let Some(requested) = auto_theme_name(theme_mode.value) {
        return Ok(resolve_auto_theme(requested, theme_mode, resolver));
    }

    Ok(fallback_theme_selection(
        theme_mode,
        ThemeProvenanceFallback::DefaultThemeSelection,
    ))
}

fn resolve_requested_theme<F>(
    requested: &str,
    source: SettingSource,
    theme_mode: &ResolvedValue<ThemeMode>,
    resolver: F,
) -> Result<ThemeSelection, StartupError>
where
    F: Fn(&str) -> Result<ResolvedThemeAsset, ThemeAssetError>,
{
    resolver(requested)
        .map(|resolved| {
            theme_selection(
                theme_mode,
                Some((requested, source)),
                resolved,
                source,
                None,
            )
        })
        .map_err(StartupError::ThemeAsset)
}

fn resolve_auto_theme<F>(
    requested: &str,
    theme_mode: &ResolvedValue<ThemeMode>,
    resolver: F,
) -> ThemeSelection
where
    F: Fn(&str) -> Result<ResolvedThemeAsset, ThemeAssetError>,
{
    match resolver(requested) {
        Ok(resolved) => theme_selection(theme_mode, None, resolved, SettingSource::Auto, None),
        Err(_) => fallback_theme_selection(
            theme_mode,
            ThemeProvenanceFallback::AutoThemeResolutionFailed,
        ),
    }
}

fn fallback_theme_selection(
    theme_mode: &ResolvedValue<ThemeMode>,
    fallback: ThemeProvenanceFallback,
) -> ThemeSelection {
    theme_selection(
        theme_mode,
        None,
        default_fallback_resolved_theme(),
        SettingSource::Fallback,
        Some(fallback),
    )
}

fn theme_selection(
    theme_mode: &ResolvedValue<ThemeMode>,
    requested_theme: Option<(&str, SettingSource)>,
    resolved_theme: ResolvedThemeAsset,
    resolved_theme_source: SettingSource,
    fallback: Option<ThemeProvenanceFallback>,
) -> ThemeSelection {
    ThemeSelection {
        resolved: ResolvedValue::new(resolved_theme.clone(), resolved_theme_source),
        provenance: theme_provenance(
            theme_mode,
            requested_theme,
            &resolved_theme,
            resolved_theme_source,
            fallback,
        ),
    }
}

fn theme_provenance(
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
        resolved_theme: resolved_theme.label(),
        resolved_theme_source,
        resolved_theme_kind: resolved_theme.kind(),
        fallback,
    }
}

fn auto_theme_name(theme_mode: ThemeMode) -> Option<&'static str> {
    match theme_mode {
        ThemeMode::Auto => None,
        ThemeMode::Light => Some("catppuccin-latte"),
        ThemeMode::Dark => Some("catppuccin-mocha"),
    }
}
