use crate::commands_catalog::BuiltinCommandEntry;

pub(crate) const SESSION_MODE_COMMAND_ENTRIES: &[BuiltinCommandEntry] = &[
    BuiltinCommandEntry {
        name: "plan",
        help_syntax: "plan",
        description: "toggle plan collaboration mode",
    },
    BuiltinCommandEntry {
        name: "collab",
        help_syntax: "collab [name|mode|default]",
        description: "list or change collaboration mode",
    },
    BuiltinCommandEntry {
        name: "agent",
        help_syntax: "agent",
        description: "switch the active agent thread",
    },
    BuiltinCommandEntry {
        name: "multi-agents",
        help_syntax: "multi-agents",
        description: "switch the active agent thread",
    },
    BuiltinCommandEntry {
        name: "auto",
        help_syntax: "auto on|off",
        description: "toggle auto-continue",
    },
    BuiltinCommandEntry {
        name: "attachments",
        help_syntax: "attachments",
        description: "show queued attachments",
    },
];
