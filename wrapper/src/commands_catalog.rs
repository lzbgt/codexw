#[derive(Clone, Copy)]
pub(crate) struct BuiltinCommandEntry {
    pub(crate) name: &'static str,
    pub(crate) help_syntax: &'static str,
    pub(crate) description: &'static str,
}

pub(crate) use crate::commands_entries::builtin_command_entries;

pub(crate) fn builtin_command_names() -> Vec<&'static str> {
    builtin_command_entries()
        .iter()
        .map(|entry| entry.name)
        .collect()
}
