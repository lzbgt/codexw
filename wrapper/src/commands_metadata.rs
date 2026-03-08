use crate::commands_catalog::BuiltinCommandEntry;
use crate::commands_catalog::builtin_command_entries;
pub(crate) use crate::commands_catalog::builtin_command_names;

pub(crate) fn builtin_help_lines() -> Vec<String> {
    builtin_command_entries()
        .iter()
        .map(|entry| format!(":{:<26} {}", entry.help_syntax, entry.description))
        .collect()
}

pub(crate) fn builtin_command_description(command: &str) -> &'static str {
    builtin_command_entry(command)
        .map(|entry| entry.description)
        .unwrap_or("command")
}

fn builtin_command_entry(command: &str) -> Option<&'static BuiltinCommandEntry> {
    builtin_command_entries()
        .iter()
        .find(|entry| entry.name == command)
}
