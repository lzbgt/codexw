pub(crate) fn current_at_token(buffer: &str, cursor_byte: usize) -> Option<(usize, usize, String)> {
    let safe_cursor = clamp_to_char_boundary(buffer, cursor_byte);
    let before_cursor = &buffer[..safe_cursor];
    let after_cursor = &buffer[safe_cursor..];
    let start = before_cursor
        .char_indices()
        .rfind(|(_, ch)| ch.is_whitespace())
        .map(|(idx, ch)| idx + ch.len_utf8())
        .unwrap_or(0);
    let end_rel = after_cursor
        .char_indices()
        .find(|(_, ch)| ch.is_whitespace())
        .map(|(idx, _)| idx)
        .unwrap_or(after_cursor.len());
    let end = safe_cursor + end_rel;
    let token = &buffer[start..end];
    let mention = token.strip_prefix('@')?;
    if mention.is_empty() {
        return Some((start, end, String::new()));
    }
    if mention.starts_with('@') {
        return None;
    }
    if mention
        .chars()
        .any(|ch| ch.is_whitespace() || matches!(ch, '"' | '\'' | '(' | ')' | '[' | ']'))
    {
        return None;
    }
    Some((start, end, mention.to_string()))
}

fn clamp_to_char_boundary(text: &str, cursor_byte: usize) -> usize {
    if cursor_byte >= text.len() {
        return text.len();
    }
    let mut safe = cursor_byte;
    while safe > 0 && !text.is_char_boundary(safe) {
        safe -= 1;
    }
    safe
}
