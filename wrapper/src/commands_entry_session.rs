use crate::commands_catalog::BuiltinCommandEntry;

pub(crate) use crate::commands_entry_session_catalog::SESSION_CATALOG_COMMAND_ENTRIES;
pub(crate) use crate::commands_entry_session_modes::SESSION_MODE_COMMAND_ENTRIES;

pub(crate) fn session_command_entries() -> impl Iterator<Item = &'static BuiltinCommandEntry> {
    SESSION_CATALOG_COMMAND_ENTRIES
        .iter()
        .chain(SESSION_MODE_COMMAND_ENTRIES.iter())
}
