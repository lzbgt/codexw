use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

pub(crate) fn preview_prompt_buffer(buffer: &str) -> String {
    buffer
        .chars()
        .map(|ch| if ch == '\n' { '↩' } else { ch })
        .collect()
}

pub(crate) fn fit_prompt_buffer(
    buffer: &str,
    cursor_chars: usize,
    available_chars: usize,
) -> (String, usize) {
    let graphemes = UnicodeSegmentation::graphemes(buffer, true).collect::<Vec<_>>();
    let total_width = grapheme_slice_width(&graphemes);
    let cursor = cursor_chars.min(graphemes.len());
    if total_width <= available_chars {
        return (
            buffer.to_string(),
            grapheme_slice_width(&graphemes[..cursor]).min(available_chars),
        );
    }

    if available_chars <= 3 {
        return (".".repeat(available_chars), available_chars);
    }

    let window_width = available_chars - 3;
    let mut start = cursor;
    let mut width_before_cursor = 0;
    while start > 0 {
        let next_width = UnicodeWidthStr::width(graphemes[start - 1]);
        if width_before_cursor + next_width > window_width {
            break;
        }
        start -= 1;
        width_before_cursor += next_width;
    }

    let mut end = start;
    let mut visible_width = 0;
    while end < graphemes.len() {
        let next_width = UnicodeWidthStr::width(graphemes[end]);
        if visible_width + next_width > window_width {
            break;
        }
        visible_width += next_width;
        end += 1;
    }
    if end == start && end < graphemes.len() {
        end += 1;
    }

    let body = graphemes[start..end].concat();
    let mut visible = String::from("...");
    visible.push_str(&body);
    let cursor_in_visible = if start == 0 {
        grapheme_slice_width(&graphemes[..cursor])
    } else {
        3 + grapheme_slice_width(&graphemes[start..cursor])
    };
    (visible, cursor_in_visible.min(available_chars))
}

fn grapheme_slice_width(graphemes: &[&str]) -> usize {
    graphemes
        .iter()
        .map(|grapheme| UnicodeWidthStr::width(*grapheme))
        .sum()
}
