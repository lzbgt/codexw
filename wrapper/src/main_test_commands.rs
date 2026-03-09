use crate::commands_catalog::builtin_command_names;
use crate::commands_catalog::builtin_help_lines;
use crate::commands_catalog::builtin_visible_command_names;
use crate::commands_completion_render::quote_if_needed;
use crate::commands_completion_render::render_slash_completion_candidates;

#[test]
fn slash_completion_rendering_includes_descriptions() {
    let rendered = render_slash_completion_candidates("re", &["resume", "review"], false);
    assert!(rendered.contains("/resume"));
    assert!(rendered.contains("resume a saved thread"));
    assert!(rendered.contains("/review"));
    assert!(rendered.contains("review current changes and find issues"));
}

#[test]
fn bare_slash_completion_uses_native_like_order() {
    let rendered = render_slash_completion_candidates("", &builtin_command_names(), false);
    let review_index = rendered.find("/review").expect("review");
    let rename_index = rendered.find("/rename").expect("rename");
    let new_index = rendered.find("/new").expect("new");
    assert!(review_index < rename_index);
    assert!(rename_index < new_index);
}

#[test]
fn help_lines_are_derived_from_command_metadata() {
    let rendered = builtin_help_lines().join("\n");
    assert!(rendered.contains(":resume [thread-id|n]"));
    assert!(rendered.contains("resume a saved thread"));
    assert!(rendered.contains(":feedback <category> [reason] [--logs|--no-logs]"));
    assert!(rendered.contains(
        ":ps [guidance [@capability]|actions [@capability]|blockers [@capability]|dependencies [all|blocking|sidecars|missing|booting|ambiguous|satisfied] [@capability]|agents|shells|services [all|ready|booting|untracked|conflicts] [@capability]|capabilities [@capability|healthy|missing|booting|untracked|ambiguous]"
    ));
}

#[test]
fn slash_catalog_matches_windows_only_command_visibility() {
    let visible = builtin_visible_command_names();
    assert!(visible.contains(&"init"));
    assert!(visible.contains(&"rollout"));
    assert!(visible.contains(&"agent"));
    assert!(visible.contains(&"multi-agents"));
    assert_eq!(
        visible.contains(&"setup-default-sandbox"),
        cfg!(target_os = "windows")
    );
    assert_eq!(
        visible.contains(&"sandbox-add-read-dir"),
        cfg!(target_os = "windows")
    );

    let rendered = render_slash_completion_candidates("", &builtin_visible_command_names(), false);
    assert_eq!(
        rendered.contains("/setup-default-sandbox"),
        cfg!(target_os = "windows")
    );
    assert_eq!(
        rendered.contains("/sandbox-add-read-dir"),
        cfg!(target_os = "windows")
    );
    assert!(builtin_command_names().contains(&"agent"));
}

#[test]
fn help_matches_windows_only_command_visibility() {
    let rendered = builtin_help_lines().join("\n");
    assert!(rendered.contains(":agent"));
    assert!(rendered.contains(":multi-agents"));
    assert!(rendered.contains(":init"));
    assert!(rendered.contains(":rollout"));
    assert_eq!(
        rendered.contains(":setup-default-sandbox"),
        cfg!(target_os = "windows")
    );
    assert_eq!(
        rendered.contains(":sandbox-add-read-dir <absolute-directory-path>"),
        cfg!(target_os = "windows")
    );
}

#[test]
fn quote_if_needed_wraps_spaces_only() {
    assert_eq!(quote_if_needed("src/main.rs"), "src/main.rs");
    assert_eq!(
        quote_if_needed("path with spaces.rs"),
        "\"path with spaces.rs\""
    );
}
