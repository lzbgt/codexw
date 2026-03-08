use unicode_segmentation::UnicodeSegmentation;

pub(crate) fn grapheme_to_byte_index(text: &str, grapheme_index: usize) -> usize {
    if grapheme_index == 0 {
        return 0;
    }
    UnicodeSegmentation::grapheme_indices(text, true)
        .nth(grapheme_index)
        .map(|(idx, _)| idx)
        .unwrap_or(text.len())
}

pub(crate) fn grapheme_count(text: &str) -> usize {
    UnicodeSegmentation::graphemes(text, true).count()
}

pub(crate) fn grapheme_is_whitespace(grapheme: &str) -> bool {
    grapheme.chars().all(char::is_whitespace)
}
