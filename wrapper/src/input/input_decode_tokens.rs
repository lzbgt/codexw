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

fn is_token_char(byte: u8) -> bool {
    matches!(byte, b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'_' | b'-' | b'.' | b'/')
}

pub(crate) fn is_file_token_char(byte: u8) -> bool {
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
