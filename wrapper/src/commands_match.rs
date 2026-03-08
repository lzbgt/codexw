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

pub(crate) fn slash_command_at_cursor<'a>(
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

pub(crate) fn fuzzy_match_score(haystack: &str, needle: &str) -> Option<i32> {
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
