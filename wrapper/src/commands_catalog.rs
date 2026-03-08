#[allow(dead_code)]
#[derive(Clone, Copy)]
pub(crate) struct BuiltinCommandEntry {
    pub(crate) name: &'static str,
    pub(crate) help_syntax: &'static str,
    pub(crate) description: &'static str,
}

pub(crate) fn builtin_command_names() -> &'static [&'static str] {
    const NAMES: &[&str] = &[
        "model",
        "models",
        "fast",
        "approvals",
        "permissions",
        "setup-default-sandbox",
        "sandbox-add-read-dir",
        "experimental",
        "skills",
        "review",
        "rename",
        "new",
        "resume",
        "fork",
        "init",
        "compact",
        "plan",
        "collab",
        "agent",
        "multi-agents",
        "diff",
        "copy",
        "mention",
        "status",
        "debug-config",
        "statusline",
        "theme",
        "mcp",
        "apps",
        "logout",
        "quit",
        "exit",
        "feedback",
        "rollout",
        "ps",
        "clean",
        "clear",
        "personality",
        "realtime",
        "settings",
        "threads",
        "auto",
        "attach-image",
        "attach",
        "attach-url",
        "attachments",
        "clear-attachments",
        "interrupt",
        "help",
    ];
    NAMES
}

pub(crate) fn builtin_command_entries() -> &'static [BuiltinCommandEntry] {
    const ENTRIES: &[BuiltinCommandEntry] = &[
        BuiltinCommandEntry {
            name: "model",
            help_syntax: "model",
            description: "choose what model and reasoning effort to use",
        },
        BuiltinCommandEntry {
            name: "models",
            help_syntax: "models",
            description: "list available models",
        },
        BuiltinCommandEntry {
            name: "fast",
            help_syntax: "fast",
            description: "toggle Fast mode to enable fastest inference at 2X plan usage",
        },
        BuiltinCommandEntry {
            name: "approvals",
            help_syntax: "approvals or /permissions",
            description: "show automation and permission posture",
        },
        BuiltinCommandEntry {
            name: "permissions",
            help_syntax: "permissions or /approvals",
            description: "show automation and permission posture",
        },
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
            name: "experimental",
            help_syntax: "experimental",
            description: "list experimental feature flags from app-server",
        },
        BuiltinCommandEntry {
            name: "skills",
            help_syntax: "skills",
            description: "use skills to improve how Codex performs specific tasks",
        },
        BuiltinCommandEntry {
            name: "review",
            help_syntax: "review [instructions]",
            description: "review current changes and find issues",
        },
        BuiltinCommandEntry {
            name: "rename",
            help_syntax: "rename <name>",
            description: "rename the current thread",
        },
        BuiltinCommandEntry {
            name: "new",
            help_syntax: "new",
            description: "start a new thread",
        },
        BuiltinCommandEntry {
            name: "resume",
            help_syntax: "resume [thread-id|n]",
            description: "resume a saved thread",
        },
        BuiltinCommandEntry {
            name: "fork",
            help_syntax: "fork",
            description: "fork the current thread",
        },
        BuiltinCommandEntry {
            name: "init",
            help_syntax: "init",
            description: "create an AGENTS.md file with instructions for Codex",
        },
        BuiltinCommandEntry {
            name: "compact",
            help_syntax: "compact",
            description: "summarize conversation to prevent hitting the context limit",
        },
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
            name: "diff",
            help_syntax: "diff",
            description: "show the latest turn diff snapshot",
        },
        BuiltinCommandEntry {
            name: "copy",
            help_syntax: "copy",
            description: "copy the latest assistant reply",
        },
        BuiltinCommandEntry {
            name: "mention",
            help_syntax: "mention [query|n]",
            description: "insert or search mentionable files",
        },
        BuiltinCommandEntry {
            name: "status",
            help_syntax: "status",
            description: "show current session configuration and token usage",
        },
        BuiltinCommandEntry {
            name: "debug-config",
            help_syntax: "debug-config",
            description: "show config layers and requirement sources for debugging",
        },
        BuiltinCommandEntry {
            name: "statusline",
            help_syntax: "statusline",
            description: "show current session status",
        },
        BuiltinCommandEntry {
            name: "theme",
            help_syntax: "theme",
            description: "choose a syntax highlighting theme",
        },
        BuiltinCommandEntry {
            name: "mcp",
            help_syntax: "mcp",
            description: "list MCP servers and tools",
        },
        BuiltinCommandEntry {
            name: "apps",
            help_syntax: "apps",
            description: "list known app mentions",
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
            name: "personality",
            help_syntax: "personality [friendly|pragmatic|none|default]",
            description: "show or change the active response style",
        },
        BuiltinCommandEntry {
            name: "realtime",
            help_syntax: "realtime [start [prompt...]|send <text>|stop|status]",
            description: "experimental text realtime workflow",
        },
        BuiltinCommandEntry {
            name: "settings",
            help_syntax: "settings",
            description: "show effective backend config",
        },
        BuiltinCommandEntry {
            name: "threads",
            help_syntax: "threads [query]",
            description: "list recent threads",
        },
        BuiltinCommandEntry {
            name: "auto",
            help_syntax: "auto on|off",
            description: "toggle auto-continue",
        },
        BuiltinCommandEntry {
            name: "attach-image",
            help_syntax: "attach-image <path>",
            description: "queue a local image for next submit",
        },
        BuiltinCommandEntry {
            name: "attach",
            help_syntax: "attach <path>",
            description: "queue a local image for next submit",
        },
        BuiltinCommandEntry {
            name: "attach-url",
            help_syntax: "attach-url <url>",
            description: "queue a remote image for next submit",
        },
        BuiltinCommandEntry {
            name: "attachments",
            help_syntax: "attachments",
            description: "show queued attachments",
        },
        BuiltinCommandEntry {
            name: "clear-attachments",
            help_syntax: "clear-attachments",
            description: "clear queued attachments",
        },
        BuiltinCommandEntry {
            name: "interrupt",
            help_syntax: "interrupt",
            description: "interrupt the current turn or local command",
        },
        BuiltinCommandEntry {
            name: "help",
            help_syntax: "help",
            description: "show available commands",
        },
    ];
    ENTRIES
}
