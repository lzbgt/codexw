use crate::commands_metadata::builtin_command_description;

pub(crate) fn render_slash_completion_candidates(
    filter: &str,
    matches: &[&str],
    fuzzy: bool,
) -> String {
    let mut lines = Vec::new();
    if filter.is_empty() {
        lines.push("Slash commands:".to_string());
    } else {
        lines.push(format!(
            "{} matches for /{}:",
            if fuzzy { "Fuzzy" } else { "Command" },
            filter
        ));
    }
    for (idx, name) in matches.iter().take(12).enumerate() {
        lines.push(format!(
            "{:>2}. /{:<16} {}",
            idx + 1,
            name,
            builtin_command_description(name)
        ));
    }
    if matches.len() > 12 {
        lines.push(format!("...and {} more", matches.len() - 12));
    }
    lines.join("\n")
}

pub(crate) fn quote_if_needed(value: &str) -> String {
    if value.chars().any(char::is_whitespace) && !value.contains('"') {
        format!("\"{value}\"")
    } else {
        value.to_string()
    }
}
