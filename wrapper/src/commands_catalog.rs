#[derive(Clone, Copy)]
pub(crate) struct BuiltinCommandEntry {
    pub(crate) name: &'static str,
    pub(crate) help_syntax: &'static str,
    pub(crate) description: &'static str,
}

use std::sync::OnceLock;

#[path = "commands_entry_runtime.rs"]
mod commands_entry_runtime;
#[path = "commands_entry_thread.rs"]
mod commands_entry_thread;

use crate::commands_entry_session_catalog::SESSION_CATALOG_COMMAND_ENTRIES;
use crate::commands_entry_session_modes::SESSION_MODE_COMMAND_ENTRIES;

pub(crate) fn builtin_command_entries() -> &'static [BuiltinCommandEntry] {
    static ENTRIES: OnceLock<Vec<BuiltinCommandEntry>> = OnceLock::new();
    ENTRIES
        .get_or_init(|| {
            let mut entries = Vec::new();
            entries.extend(SESSION_CATALOG_COMMAND_ENTRIES.iter().copied());
            entries.extend(SESSION_MODE_COMMAND_ENTRIES.iter().copied());
            entries.extend_from_slice(commands_entry_thread::THREAD_COMMAND_ENTRIES);
            entries.extend_from_slice(commands_entry_runtime::RUNTIME_COMMAND_ENTRIES);
            entries.sort_by_key(|entry| builtin_command_rank(entry.name));
            entries
        })
        .as_slice()
}

pub(crate) fn builtin_command_names() -> Vec<&'static str> {
    builtin_command_entries()
        .iter()
        .map(|entry| entry.name)
        .collect()
}

fn builtin_command_rank(name: &str) -> usize {
    match name {
        "model" => 0,
        "models" => 1,
        "fast" => 2,
        "approvals" => 3,
        "permissions" => 4,
        "setup-default-sandbox" => 5,
        "sandbox-add-read-dir" => 6,
        "experimental" => 7,
        "skills" => 8,
        "review" => 9,
        "rename" => 10,
        "new" => 11,
        "resume" => 12,
        "fork" => 13,
        "init" => 14,
        "compact" => 15,
        "plan" => 16,
        "collab" => 17,
        "agent" => 18,
        "multi-agents" => 19,
        "diff" => 20,
        "copy" => 21,
        "mention" => 22,
        "status" => 23,
        "debug-config" => 24,
        "statusline" => 25,
        "theme" => 26,
        "mcp" => 27,
        "apps" => 28,
        "logout" => 29,
        "quit" => 30,
        "exit" => 31,
        "feedback" => 32,
        "rollout" => 33,
        "ps" => 34,
        "clean" => 35,
        "clear" => 36,
        "personality" => 37,
        "realtime" => 38,
        "settings" => 39,
        "threads" => 40,
        "auto" => 41,
        "attach-image" => 42,
        "attach" => 43,
        "attach-url" => 44,
        "attachments" => 45,
        "clear-attachments" => 46,
        "interrupt" => 47,
        "help" => 48,
        _ => usize::MAX,
    }
}
