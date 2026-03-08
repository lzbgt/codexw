use crate::commands_catalog::BuiltinCommandEntry;

pub(crate) const SESSION_COMMAND_ENTRIES: &[BuiltinCommandEntry] = &[
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
        name: "personality",
        help_syntax: "personality [friendly|pragmatic|none|default]",
        description: "show or change the active response style",
    },
    BuiltinCommandEntry {
        name: "settings",
        help_syntax: "settings",
        description: "show effective backend config",
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
