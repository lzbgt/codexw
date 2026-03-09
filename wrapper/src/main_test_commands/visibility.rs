use crate::commands_catalog::builtin_command_names;
use crate::commands_catalog::builtin_help_lines;
use crate::commands_catalog::builtin_visible_command_names;
use crate::commands_completion_render::render_slash_completion_candidates;

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
