use std::path::Path;
use std::path::PathBuf;

use super::input_types::DecodedHistoryText;
use super::input_types::LinkedMention;
use super::input_types::PluginCatalogEntry;

pub fn decode_linked_mentions(text: &str) -> DecodedHistoryText {
    let bytes = text.as_bytes();
    let mut out = String::with_capacity(text.len());
    let mut mentions = Vec::new();
    let mut index = 0usize;

    while index < bytes.len() {
        if bytes[index] == b'['
            && let Some((name, path, end_index)) = parse_linked_tool_mention(text, bytes, index)
            && !is_common_env_var(name)
            && is_tool_path(path)
        {
            out.push('$');
            out.push_str(name);
            mentions.push(LinkedMention {
                mention: name.to_string(),
                path: path.to_string(),
            });
            index = end_index;
            continue;
        }

        let Some(ch) = text[index..].chars().next() else {
            break;
        };
        out.push(ch);
        index += ch.len_utf8();
    }

    DecodedHistoryText {
        text: out,
        mentions,
    }
}

pub(crate) fn expand_inline_file_mentions(
    text: &str,
    resolved_cwd: &str,
    plugins: &[PluginCatalogEntry],
) -> String {
    let plugin_names = plugins
        .iter()
        .filter(|plugin| plugin.enabled)
        .map(|plugin| plugin.name.to_ascii_lowercase())
        .collect::<std::collections::HashSet<_>>();
    let bytes = text.as_bytes();
    let mut out = String::with_capacity(text.len());
    let mut index = 0usize;

    while index < bytes.len() {
        if bytes[index] != b'@' {
            let Some(ch) = text[index..].chars().next() else {
                break;
            };
            out.push(ch);
            index += ch.len_utf8();
            continue;
        }

        if index > 0
            && let Some(previous) = bytes.get(index - 1)
            && !previous.is_ascii_whitespace()
        {
            out.push('@');
            index += 1;
            continue;
        }

        let start = index + 1;
        let Some(first) = bytes.get(start) else {
            out.push('@');
            index += 1;
            continue;
        };
        if !is_file_token_char(*first) {
            out.push('@');
            index += 1;
            continue;
        }

        let mut end = start + 1;
        while let Some(next) = bytes.get(end)
            && is_file_token_char(*next)
        {
            end += 1;
        }

        let token = &text[start..end];
        let lowered = token.to_ascii_lowercase();
        if !token.contains('/')
            && !token.contains('\\')
            && !token.contains('.')
            && plugin_names.contains(&lowered)
        {
            out.push('@');
            out.push_str(token);
            index = end;
            continue;
        }

        if let Some(path) = resolve_file_mention_path(token, resolved_cwd) {
            out.push_str(&path);
        } else {
            out.push('@');
            out.push_str(token);
        }
        index = end;
    }

    out
}

pub(crate) fn mention_skill_path(path: &str) -> Option<String> {
    if let Some(stripped) = path.strip_prefix("skill://")
        && !stripped.is_empty()
    {
        return Some(stripped.to_string());
    }
    if path
        .rsplit(['/', '\\'])
        .next()
        .is_some_and(|name| name.eq_ignore_ascii_case("SKILL.md"))
    {
        return Some(path.to_string());
    }
    None
}

pub(crate) fn parse_linked_tool_mention<'a>(
    text: &'a str,
    text_bytes: &[u8],
    start: usize,
) -> Option<(&'a str, &'a str, usize)> {
    let sigil_index = start + 1;
    if text_bytes.get(sigil_index) != Some(&b'$') {
        return None;
    }

    let name_start = sigil_index + 1;
    let first_name_byte = text_bytes.get(name_start)?;
    if !is_mention_name_char(*first_name_byte) {
        return None;
    }

    let mut name_end = name_start + 1;
    while let Some(next_byte) = text_bytes.get(name_end)
        && is_mention_name_char(*next_byte)
    {
        name_end += 1;
    }

    if text_bytes.get(name_end) != Some(&b']') {
        return None;
    }

    let mut path_start = name_end + 1;
    while let Some(next_byte) = text_bytes.get(path_start)
        && next_byte.is_ascii_whitespace()
    {
        path_start += 1;
    }
    if text_bytes.get(path_start) != Some(&b'(') {
        return None;
    }

    let mut path_end = path_start + 1;
    while let Some(next_byte) = text_bytes.get(path_end)
        && *next_byte != b')'
    {
        path_end += 1;
    }
    if text_bytes.get(path_end) != Some(&b')') {
        return None;
    }

    let path = text[path_start + 1..path_end].trim();
    if path.is_empty() {
        return None;
    }

    let name = &text[name_start..name_end];
    Some((name, path, path_end + 1))
}

pub(crate) fn collect_prefixed_tokens(
    text: &str,
    sigil: char,
) -> std::collections::HashSet<String> {
    let bytes = text.as_bytes();
    let mut tokens = std::collections::HashSet::new();
    let mut index = 0usize;

    while index < bytes.len() {
        if bytes[index] != sigil as u8 {
            index += 1;
            continue;
        }
        if index > 0
            && let Some(previous) = bytes.get(index - 1)
            && !previous.is_ascii_whitespace()
        {
            index += 1;
            continue;
        }

        let start = index + 1;
        let Some(first) = bytes.get(start) else {
            index += 1;
            continue;
        };
        if !is_token_char(*first) {
            index += 1;
            continue;
        }
        let mut end = start + 1;
        while let Some(next) = bytes.get(end)
            && is_token_char(*next)
        {
            end += 1;
        }
        tokens.insert(text[start..end].to_ascii_lowercase());
        index = end;
    }

    tokens
}

pub(crate) fn is_common_env_var(name: &str) -> bool {
    let upper = name.to_ascii_uppercase();
    matches!(
        upper.as_str(),
        "PATH"
            | "HOME"
            | "USER"
            | "SHELL"
            | "PWD"
            | "TMPDIR"
            | "TEMP"
            | "TMP"
            | "LANG"
            | "TERM"
            | "XDG_CONFIG_HOME"
    )
}

pub(crate) fn is_mention_name_char(byte: u8) -> bool {
    matches!(byte, b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'_' | b'-')
}

pub(crate) fn is_tool_path(path: &str) -> bool {
    path.starts_with("app://")
        || path.starts_with("mcp://")
        || path.starts_with("plugin://")
        || path.starts_with("skill://")
        || path
            .rsplit(['/', '\\'])
            .next()
            .is_some_and(|name| name.eq_ignore_ascii_case("SKILL.md"))
}

fn resolve_file_mention_path(token: &str, resolved_cwd: &str) -> Option<String> {
    let token = token.trim();
    if token.is_empty() {
        return None;
    }

    let path = Path::new(token);
    let candidate = if path.is_absolute() {
        path.to_path_buf()
    } else {
        PathBuf::from(resolved_cwd).join(path)
    };
    if !candidate.exists() {
        return None;
    }

    let rendered = if path.is_absolute() {
        token.to_string()
    } else {
        token.trim_start_matches("./").to_string()
    };
    if rendered.chars().any(char::is_whitespace) && !rendered.contains('"') {
        Some(format!("\"{rendered}\""))
    } else {
        Some(rendered)
    }
}

fn is_token_char(byte: u8) -> bool {
    matches!(byte, b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'_' | b'-' | b'.' | b'/')
}

fn is_file_token_char(byte: u8) -> bool {
    matches!(
        byte,
        b'a'..=b'z'
            | b'A'..=b'Z'
            | b'0'..=b'9'
            | b'_'
            | b'-'
            | b'.'
            | b'/'
            | b'\\'
    )
}
