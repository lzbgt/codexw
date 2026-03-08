use std::fs;
use std::io::ErrorKind;
use std::path::Path;
use std::path::PathBuf;

use anyhow::Context;
use anyhow::Result;
use toml_edit::DocumentMut;
use toml_edit::Item;
use toml_edit::Table;
use toml_edit::value;

const CONFIG_TOML: &str = "config.toml";

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
    let contents = match fs::read_to_string(&config_path) {
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

fn edit_config<F>(codex_home_override: Option<&Path>, edit: F) -> Result<()>
where
    F: FnOnce(&mut DocumentMut),
{
    let codex_home = resolve_codex_home(codex_home_override)?;
    fs::create_dir_all(&codex_home)
        .with_context(|| format!("failed to create {}", codex_home.display()))?;
    edit_config_at_home(&codex_home, edit)
}

fn edit_config_at_home<F>(codex_home: &Path, edit: F) -> Result<()>
where
    F: FnOnce(&mut DocumentMut),
{
    fs::create_dir_all(codex_home)
        .with_context(|| format!("failed to create {}", codex_home.display()))?;
    let config_path = config_path(codex_home);
    let mut doc = load_document(&config_path)?;
    edit(&mut doc);
    let mut serialized = doc.to_string();
    if !serialized.ends_with('\n') {
        serialized.push('\n');
    }
    fs::write(&config_path, serialized)
        .with_context(|| format!("failed to write {}", config_path.display()))
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

fn config_path(codex_home: &Path) -> PathBuf {
    codex_home.join(CONFIG_TOML)
}

fn load_document(config_path: &Path) -> Result<DocumentMut> {
    let contents = match fs::read_to_string(config_path) {
        Ok(contents) => contents,
        Err(err) if err.kind() == ErrorKind::NotFound => String::new(),
        Err(err) => {
            return Err(err).with_context(|| format!("failed to read {}", config_path.display()));
        }
    };
    if contents.trim().is_empty() {
        Ok(DocumentMut::new())
    } else {
        contents
            .parse::<DocumentMut>()
            .with_context(|| format!("failed to parse {}", config_path.display()))
    }
}

fn set_root_string(doc: &mut DocumentMut, key: &str, value_text: Option<&str>) {
    if let Some(value_text) = value_text {
        doc[key] = value(value_text);
    } else {
        doc.as_table_mut().remove(key);
    }
}

fn ensure_tui_table(doc: &mut DocumentMut) {
    let needs_table = !matches!(doc.get("tui"), Some(Item::Table(_)));
    if needs_table {
        doc["tui"] = Item::Table(Table::new());
    }
}

fn ensure_nested_table(doc: &mut DocumentMut, table_key: &str) {
    let needs_table = !matches!(doc.get(table_key), Some(Item::Table(_)));
    if needs_table {
        doc[table_key] = Item::Table(Table::new());
    }
}

fn trim_empty_tui_table(doc: &mut DocumentMut) {
    trim_empty_table(doc, "tui");
}

fn trim_empty_table(doc: &mut DocumentMut, table_key: &str) {
    let should_remove = doc
        .get(table_key)
        .and_then(Item::as_table)
        .is_some_and(|table| table.is_empty());
    if should_remove {
        doc.as_table_mut().remove(table_key);
    }
}

fn set_nested_string(doc: &mut DocumentMut, path: &[&str], value_text: Option<&str>) {
    if path.len() != 2 {
        return;
    }
    let table_key = path[0];
    let value_key = path[1];
    if let Some(value_text) = value_text {
        ensure_nested_table(doc, table_key);
        doc[table_key][value_key] = value(value_text);
    } else if let Some(table) = doc.get_mut(table_key).and_then(Item::as_table_mut) {
        table.remove(value_key);
    }
}

#[cfg(test)]
mod tests {
    use super::config_path;
    use super::edit_config_at_home;
    use super::load_persisted_theme_from_home;
    use super::persist_model_selection;
    use super::persist_personality_selection;
    use super::persist_service_tier_selection;
    use super::persist_theme_selection;
    use super::persist_windows_sandbox_mode;
    use toml_edit::value;

    #[test]
    fn persistence_writes_and_clears_upstream_style_keys() {
        let temp = tempfile::tempdir().expect("tempdir");
        let codex_home = temp.path().join("codex-home");

        persist_model_selection(Some(&codex_home), Some("gpt-5-codex"), Some("high"))
            .expect("persist model");
        persist_personality_selection(Some(&codex_home), Some("friendly"))
            .expect("persist personality");
        persist_service_tier_selection(Some(&codex_home), Some("fast"))
            .expect("persist service tier");
        persist_theme_selection(Some(&codex_home), "base16-ocean.dark").expect("persist theme");

        let contents = std::fs::read_to_string(config_path(&codex_home)).expect("read config");
        assert!(contents.contains("model = \"gpt-5-codex\""));
        assert!(contents.contains("model_reasoning_effort = \"high\""));
        assert!(contents.contains("personality = \"friendly\""));
        assert!(contents.contains("service_tier = \"fast\""));
        assert!(contents.contains("[tui]"));
        assert!(contents.contains("theme = \"base16-ocean.dark\""));

        persist_model_selection(Some(&codex_home), None, None).expect("clear model");
        persist_personality_selection(Some(&codex_home), None).expect("clear personality");
        persist_service_tier_selection(Some(&codex_home), None).expect("clear service tier");

        let contents = std::fs::read_to_string(config_path(&codex_home)).expect("read config");
        assert!(!contents.contains("model = "));
        assert!(!contents.contains("model_reasoning_effort = "));
        assert!(!contents.contains("personality = "));
        assert!(!contents.contains("service_tier = "));
        assert!(contents.contains("[tui]"));
        assert!(contents.contains("theme = \"base16-ocean.dark\""));
    }

    #[test]
    fn load_persisted_theme_reads_saved_tui_theme() {
        let temp = tempfile::tempdir().expect("tempdir");
        let codex_home = temp.path().join("codex-home");

        persist_theme_selection(Some(&codex_home), "solarized-dark").expect("persist theme");

        assert_eq!(
            load_persisted_theme_from_home(&codex_home).expect("load theme"),
            Some("solarized-dark".to_string())
        );
    }

    #[test]
    fn load_persisted_theme_ignores_configs_without_tui_table() {
        let temp = tempfile::tempdir().expect("tempdir");
        let codex_home = temp.path().join("codex-home");

        edit_config_at_home(&codex_home, |doc| {
            doc["model"] = value("gpt-5-codex");
        })
        .expect("write config");

        assert_eq!(
            load_persisted_theme_from_home(&codex_home).expect("load theme"),
            None
        );
    }

    #[test]
    fn persist_windows_sandbox_mode_writes_and_clears_windows_table() {
        let temp = tempfile::tempdir().expect("tempdir");
        let codex_home = temp.path().join("codex-home");

        persist_windows_sandbox_mode(Some(&codex_home), Some("elevated"))
            .expect("persist windows sandbox mode");
        let contents = std::fs::read_to_string(config_path(&codex_home)).expect("read config");
        assert!(contents.contains("[windows]"));
        assert!(contents.contains("sandbox = \"elevated\""));

        persist_windows_sandbox_mode(Some(&codex_home), None).expect("clear windows sandbox mode");
        let contents = std::fs::read_to_string(config_path(&codex_home)).expect("read config");
        assert!(!contents.contains("sandbox = "));
        assert!(!contents.contains("[windows]"));
    }
}
