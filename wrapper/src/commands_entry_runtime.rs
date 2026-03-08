use crate::commands_catalog::BuiltinCommandEntry;

pub(crate) const RUNTIME_COMMAND_ENTRIES: &[BuiltinCommandEntry] = &[
    BuiltinCommandEntry {
        name: "setup-default-sandbox",
        help_syntax: "setup-default-sandbox",
        description: "set up elevated agent sandbox",
    },
    BuiltinCommandEntry {
        name: "sandbox-add-read-dir",
        help_syntax: "sandbox-add-read-dir <absolute-directory-path>",
        description: "let sandbox read a directory",
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
        description: "print the rollout file path for the current thread",
    },
    BuiltinCommandEntry {
        name: "ps",
        help_syntax: "ps [guidance|actions|blockers|dependencies [all|blocking|sidecars|missing|booting|ambiguous|satisfied] [@capability]|agents|shells|services [all|ready|booting|untracked|conflicts] [@capability]|capabilities [@capability|healthy|missing|booting|ambiguous]|terminals|attach <jobId|alias|@capability|n>|wait <jobId|alias|@capability|n> [timeoutMs]|run <jobId|alias|@capability|n> <recipe> [json-args]|poll <jobId|alias|@capability|n>|send <jobId|alias|@capability|n> <text>|terminate <jobId|alias|@capability|n>|alias <jobId|n> <name>|unalias <name>|clean [blockers|shells|services [@capability]|terminals]]",
        description: "list or stop tracked workers and background tasks",
    },
    BuiltinCommandEntry {
        name: "clean",
        help_syntax: "clean [blockers|shells|services [@capability]|terminals]",
        description: "stop background tasks for the thread",
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
