use crate::commands_catalog::BuiltinCommandEntry;

pub(crate) const THREAD_COMMAND_ENTRIES: &[BuiltinCommandEntry] = &[
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
        name: "compact",
        help_syntax: "compact",
        description: "summarize conversation to prevent hitting the context limit",
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
        name: "threads",
        help_syntax: "threads [query]",
        description: "list recent threads",
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
        name: "clear-attachments",
        help_syntax: "clear-attachments",
        description: "clear queued attachments",
    },
    BuiltinCommandEntry {
        name: "interrupt",
        help_syntax: "interrupt",
        description: "interrupt the current turn or local command",
    },
];
