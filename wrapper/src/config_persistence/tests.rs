use toml_edit::value;

use super::edit::config_path;
use super::edit::edit_config_at_home;
use super::load_persisted_theme_from_home;
use super::persist_model_selection;
use super::persist_personality_selection;
use super::persist_service_tier_selection;
use super::persist_theme_selection;
use super::persist_windows_sandbox_mode;

#[test]
fn persistence_writes_and_clears_upstream_style_keys() {
    let temp = tempfile::tempdir().expect("tempdir");
    let codex_home = temp.path().join("codex-home");

    persist_model_selection(Some(&codex_home), Some("gpt-5-codex"), Some("high"))
        .expect("persist model");
    persist_personality_selection(Some(&codex_home), Some("friendly"))
        .expect("persist personality");
    persist_service_tier_selection(Some(&codex_home), Some("fast")).expect("persist service tier");
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
