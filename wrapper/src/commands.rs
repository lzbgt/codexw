use crate::editor::LineEditor;

#[allow(dead_code)]
#[derive(Clone, Copy)]
struct BuiltinCommandEntry {
    name: &'static str,
    help_syntax: &'static str,
    description: &'static str,
}

pub(crate) struct SlashCompletionResult {
    pub(crate) rendered_candidates: Option<String>,
}

fn builtin_command_entry(command: &str) -> Option<&'static BuiltinCommandEntry> {
    builtin_command_entries()
        .iter()
        .find(|entry| entry.name == command)
}

fn builtin_command_names() -> &'static [&'static str] {
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

fn builtin_command_description(command: &str) -> &'static str {
    builtin_command_entry(command)
        .map(|entry| entry.description)
        .unwrap_or("command")
}

pub(crate) fn try_complete_slash_command(
    editor: &mut LineEditor,
    buffer: &str,
    cursor_byte: usize,
) -> Option<SlashCompletionResult> {
    let Some((command_start, command_end, prefix)) = slash_command_at_cursor(buffer, cursor_byte)
    else {
        return None;
    };

    let mut prefix_matches = builtin_command_names()
        .iter()
        .copied()
        .filter(|name| name.starts_with(prefix))
        .collect::<Vec<_>>();

    if prefix_matches.is_empty() && prefix.is_empty() {
        prefix_matches = builtin_command_names().to_vec();
    }

    if prefix_matches.len() == 1 {
        editor.replace_range(
            command_start,
            command_end,
            &format!("/{} ", prefix_matches[0]),
        );
        return Some(SlashCompletionResult {
            rendered_candidates: None,
        });
    }

    if !prefix_matches.is_empty() {
        let lcp = longest_common_prefix(&prefix_matches);
        if lcp.len() > prefix.len() {
            editor.replace_range(command_start, command_end, &format!("/{lcp}"));
            return Some(SlashCompletionResult {
                rendered_candidates: None,
            });
        }

        return Some(SlashCompletionResult {
            rendered_candidates: Some(render_slash_completion_candidates(
                prefix,
                &prefix_matches,
                false,
            )),
        });
    }

    let mut fuzzy_matches = builtin_command_names()
        .iter()
        .filter_map(|name| fuzzy_match_score(name, prefix).map(|score| (*name, score)))
        .collect::<Vec<_>>();
    if fuzzy_matches.is_empty() {
        return None;
    }
    fuzzy_matches.sort_by(|(name_a, score_a), (name_b, score_b)| {
        score_a.cmp(score_b).then_with(|| name_a.cmp(name_b))
    });
    let fuzzy_names = fuzzy_matches
        .into_iter()
        .map(|(name, _)| name)
        .collect::<Vec<_>>();
    Some(SlashCompletionResult {
        rendered_candidates: Some(render_slash_completion_candidates(
            prefix,
            &fuzzy_names,
            true,
        )),
    })
}

pub(crate) fn render_slash_completion_candidates(
    filter: &str,
    matches: &[&str],
    fuzzy: bool,
) -> String {
    let mut lines = Vec::new();
    if filter.is_empty() {
        lines.push("Slash commands:".to_string());
    } else {
        lines.push(format!(
            "{} matches for /{}:",
            if fuzzy { "Fuzzy" } else { "Command" },
            filter
        ));
    }
    for (idx, name) in matches.iter().take(12).enumerate() {
        lines.push(format!(
            "{:>2}. /{:<16} {}",
            idx + 1,
            name,
            builtin_command_description(name)
        ));
    }
    if matches.len() > 12 {
        lines.push(format!("…and {} more", matches.len() - 12));
    }
    lines.join("\n")
}

pub(crate) fn quote_if_needed(value: &str) -> String {
    if value.chars().any(char::is_whitespace) && !value.contains('"') {
        format!("\"{value}\"")
    } else {
        value.to_string()
    }
}

pub(crate) fn longest_common_prefix<S: AsRef<str>>(values: &[S]) -> String {
    if values.is_empty() {
        return String::new();
    }
    let mut prefix = values[0].as_ref().to_string();
    for value in &values[1..] {
        let mut next = String::new();
        for (a, b) in prefix.chars().zip(value.as_ref().chars()) {
            if a != b {
                break;
            }
            next.push(a);
        }
        prefix = next;
        if prefix.is_empty() {
            break;
        }
    }
    prefix
}

fn slash_command_at_cursor<'a>(
    buffer: &'a str,
    cursor_byte: usize,
) -> Option<(usize, usize, &'a str)> {
    let first_line_end = buffer.find('\n').unwrap_or(buffer.len());
    if cursor_byte > first_line_end {
        return None;
    }
    let first_line = &buffer[..first_line_end];
    let Some(stripped) = first_line.strip_prefix('/') else {
        return None;
    };
    let name_end = stripped
        .char_indices()
        .find(|(_, ch)| ch.is_whitespace())
        .map(|(idx, _)| idx)
        .unwrap_or(stripped.len());
    let command_end = 1 + name_end;
    if cursor_byte > command_end {
        return None;
    }
    Some((0, command_end, &stripped[..name_end]))
}

fn fuzzy_match_score(haystack: &str, needle: &str) -> Option<i32> {
    if needle.is_empty() {
        return Some(i32::MAX);
    }

    let mut lowered_chars = Vec::new();
    let mut lowered_to_orig_char_idx = Vec::new();
    for (orig_idx, ch) in haystack.chars().enumerate() {
        for lc in ch.to_lowercase() {
            lowered_chars.push(lc);
            lowered_to_orig_char_idx.push(orig_idx);
        }
    }

    let lowered_needle = needle.to_lowercase().chars().collect::<Vec<_>>();
    let mut result_orig_indices = Vec::with_capacity(lowered_needle.len());
    let mut last_lower_pos = None;
    let mut cur = 0usize;
    for &nc in &lowered_needle {
        let mut found_at = None;
        while cur < lowered_chars.len() {
            if lowered_chars[cur] == nc {
                found_at = Some(cur);
                cur += 1;
                break;
            }
            cur += 1;
        }
        let pos = found_at?;
        result_orig_indices.push(lowered_to_orig_char_idx[pos]);
        last_lower_pos = Some(pos);
    }

    let first_lower_pos = if result_orig_indices.is_empty() {
        0usize
    } else {
        let target_orig = result_orig_indices[0];
        lowered_to_orig_char_idx
            .iter()
            .position(|&oi| oi == target_orig)
            .unwrap_or(0)
    };
    let last_lower_pos = last_lower_pos.unwrap_or(first_lower_pos);
    let window =
        (last_lower_pos as i32 - first_lower_pos as i32 + 1) - (lowered_needle.len() as i32);
    let mut score = window.max(0);
    if first_lower_pos == 0 {
        score -= 100;
    }
    Some(score)
}

fn builtin_command_entries() -> &'static [BuiltinCommandEntry] {
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
            help_syntax: "realtime",
            description: "experimental realtime workflow",
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
