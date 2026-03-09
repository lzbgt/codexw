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

use crate::config_persistence::resolve_codex_home;

const CONFIG_TOML: &str = "config.toml";

pub(super) fn edit_config<F>(codex_home_override: Option<&Path>, edit: F) -> Result<()>
where
    F: FnOnce(&mut DocumentMut),
{
    let codex_home = resolve_codex_home(codex_home_override)?;
    fs::create_dir_all(&codex_home)
        .with_context(|| format!("failed to create {}", codex_home.display()))?;
    edit_config_at_home(&codex_home, edit)
}

pub(super) fn edit_config_at_home<F>(codex_home: &Path, edit: F) -> Result<()>
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

pub(super) fn config_path(codex_home: &Path) -> PathBuf {
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

pub(super) fn set_root_string(doc: &mut DocumentMut, key: &str, value_text: Option<&str>) {
    if let Some(value_text) = value_text {
        doc[key] = value(value_text);
    } else {
        doc.as_table_mut().remove(key);
    }
}

pub(super) fn ensure_tui_table(doc: &mut DocumentMut) {
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

pub(super) fn trim_empty_tui_table(doc: &mut DocumentMut) {
    trim_empty_table(doc, "tui");
}

pub(super) fn trim_empty_table(doc: &mut DocumentMut, table_key: &str) {
    let should_remove = doc
        .get(table_key)
        .and_then(Item::as_table)
        .is_some_and(|table| table.is_empty());
    if should_remove {
        doc.as_table_mut().remove(table_key);
    }
}

pub(super) fn set_nested_string(doc: &mut DocumentMut, path: &[&str], value_text: Option<&str>) {
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
