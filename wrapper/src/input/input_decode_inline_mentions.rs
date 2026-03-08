use super::input_decode_tokens::is_file_token_char;
use crate::input::input_types::PluginCatalogEntry;

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

fn resolve_file_mention_path(token: &str, resolved_cwd: &str) -> Option<String> {
    use std::path::Path;
    use std::path::PathBuf;

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
