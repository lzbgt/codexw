mod edit;
#[cfg(test)]
mod tests;

use std::io::ErrorKind;
use std::path::Path;
use std::path::PathBuf;

use anyhow::Context;
use anyhow::Result;
use toml_edit::DocumentMut;
use toml_edit::Item;
use toml_edit::value;

use crate::config_persistence::edit::config_path;
use crate::config_persistence::edit::edit_config;
#[cfg(test)]
use crate::config_persistence::edit::edit_config_at_home;
use crate::config_persistence::edit::ensure_tui_table;
use crate::config_persistence::edit::set_nested_string;
use crate::config_persistence::edit::set_root_string;
use crate::config_persistence::edit::trim_empty_table;
use crate::config_persistence::edit::trim_empty_tui_table;

pub(crate) fn persist_model_selection(
    codex_home_override: Option<&Path>,
    model: Option<&str>,
    effort: Option<&str>,
) -> Result<()> {
    edit_config(codex_home_override, |doc| {
        set_root_string(doc, "model", model);
        set_root_string(doc, "model_reasoning_effort", effort);
        trim_empty_tui_table(doc);
    })
}

pub(crate) fn persist_personality_selection(
    codex_home_override: Option<&Path>,
    personality: Option<&str>,
) -> Result<()> {
    edit_config(codex_home_override, |doc| {
        set_root_string(doc, "personality", personality);
        trim_empty_tui_table(doc);
    })
}

pub(crate) fn persist_service_tier_selection(
    codex_home_override: Option<&Path>,
    service_tier: Option<&str>,
) -> Result<()> {
    edit_config(codex_home_override, |doc| {
        set_root_string(doc, "service_tier", service_tier);
        trim_empty_tui_table(doc);
    })
}

pub(crate) fn persist_theme_selection(
    codex_home_override: Option<&Path>,
    theme: &str,
) -> Result<()> {
    edit_config(codex_home_override, |doc| {
        ensure_tui_table(doc);
        doc["tui"]["theme"] = value(theme);
    })
}

pub(crate) fn persist_windows_sandbox_mode(
    codex_home_override: Option<&Path>,
    mode: Option<&str>,
) -> Result<()> {
    edit_config(codex_home_override, |doc| {
        set_nested_string(doc, &["windows", "sandbox"], mode);
        trim_empty_table(doc, "windows");
        trim_empty_tui_table(doc);
    })
}

pub(crate) fn load_persisted_theme() -> Result<Option<String>> {
    load_persisted_theme_from_home(resolve_codex_home(None)?.as_path())
}

fn load_persisted_theme_from_home(codex_home: &Path) -> Result<Option<String>> {
    let config_path = config_path(codex_home);
    let contents = match std::fs::read_to_string(&config_path) {
        Ok(contents) => contents,
        Err(err) if err.kind() == ErrorKind::NotFound => return Ok(None),
        Err(err) => {
            return Err(err).with_context(|| format!("failed to read {}", config_path.display()));
        }
    };
    if contents.trim().is_empty() {
        return Ok(None);
    }
    let doc = contents
        .parse::<DocumentMut>()
        .with_context(|| format!("failed to parse {}", config_path.display()))?;
    Ok(doc
        .get("tui")
        .and_then(Item::as_table)
        .and_then(|table| table.get("theme"))
        .and_then(Item::as_str)
        .map(ToString::to_string))
}

pub(crate) fn resolve_codex_home(codex_home_override: Option<&Path>) -> Result<PathBuf> {
    if let Some(codex_home) = codex_home_override {
        return Ok(codex_home.to_path_buf());
    }
    if let Some(codex_home) = std::env::var_os("CODEX_HOME") {
        return Ok(PathBuf::from(codex_home));
    }
    let home = std::env::var_os("HOME").context("HOME is not set and CODEX_HOME is not set")?;
    Ok(PathBuf::from(home).join(".codex"))
}
