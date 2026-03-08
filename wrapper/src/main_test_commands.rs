use crate::commands::builtin_command_names;
use crate::commands::builtin_help_lines;
use crate::commands::quote_if_needed;
use crate::commands_completion::render_slash_completion_candidates;

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
}

#[test]
fn quote_if_needed_wraps_spaces_only() {
    assert_eq!(quote_if_needed("src/main.rs"), "src/main.rs");
    assert_eq!(
        quote_if_needed("path with spaces.rs"),
        "\"path with spaces.rs\""
    );
}
