use crate::commands_catalog::BuiltinCommandEntry;

pub(crate) const RUNTIME_COMMAND_ENTRIES: &[BuiltinCommandEntry] = &[
    BuiltinCommandEntry {
        name: "setup-default-sandbox",
        help_syntax: "setup-default-sandbox",
        description: "native sandbox setup workflow not yet ported",
    },
    BuiltinCommandEntry {
        name: "sandbox-add-read-dir",
        help_syntax: "sandbox-add-read-dir",
        description: "native sandbox read-dir workflow not yet ported",
    },
    BuiltinCommandEntry {
        name: "init",
        help_syntax: "init",
        description: "create an AGENTS.md file with instructions for Codex",
    },
    BuiltinCommandEntry {
        name: "theme",
        help_syntax: "theme",
        description: "choose a syntax highlighting theme",
    },
    BuiltinCommandEntry {
        name: "logout",
        help_syntax: "logout",
        description: "log out of Codex",
    },
    BuiltinCommandEntry {
        name: "quit",
        help_syntax: "quit",
        description: "exit CodexW",
    },
    BuiltinCommandEntry {
        name: "exit",
        help_syntax: "exit",
        description: "exit CodexW",
    },
    BuiltinCommandEntry {
        name: "feedback",
        help_syntax: "feedback <category> [reason] [--logs|--no-logs]",
        description: "submit feedback through app-server",
    },
    BuiltinCommandEntry {
        name: "rollout",
        help_syntax: "rollout",
        description: "native rollout-path display not yet ported",
    },
    BuiltinCommandEntry {
        name: "ps",
        help_syntax: "ps [clean]",
        description: "explain background-terminal limits or stop all background terminals",
    },
    BuiltinCommandEntry {
        name: "clean",
        help_syntax: "clean",
        description: "stop background terminals for the thread",
    },
    BuiltinCommandEntry {
        name: "clear",
        help_syntax: "clear",
        description: "clear terminal and start a new thread",
    },
    BuiltinCommandEntry {
        name: "realtime",
        help_syntax: "realtime [start [prompt...]|send <text>|stop|status]",
        description: "experimental text realtime workflow",
    },
    BuiltinCommandEntry {
        name: "help",
        help_syntax: "help",
        description: "show available commands",
    },
];
