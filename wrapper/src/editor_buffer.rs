use unicode_segmentation::UnicodeSegmentation;

use crate::editor::LineEditor;
use crate::editor::grapheme_count;
use crate::editor::grapheme_is_whitespace;
use crate::editor::grapheme_to_byte_index;

impl LineEditor {
    pub fn insert_char(&mut self, ch: char) {
        let byte_index = grapheme_to_byte_index(&self.buffer, self.cursor_chars);
        self.buffer.insert(byte_index, ch);
        self.cursor_chars += 1;
        self.history_index = None;
    }

    pub fn insert_str(&mut self, text: &str) {
        let byte_index = grapheme_to_byte_index(&self.buffer, self.cursor_chars);
        self.buffer.insert_str(byte_index, text);
        self.cursor_chars += grapheme_count(text);
        self.history_index = None;
    }

    pub fn insert_newline(&mut self) {
        self.insert_char('\n');
    }

    pub fn backspace(&mut self) {
        if self.cursor_chars == 0 {
            return;
        }
        let start = grapheme_to_byte_index(&self.buffer, self.cursor_chars - 1);
        let end = grapheme_to_byte_index(&self.buffer, self.cursor_chars);
        self.buffer.replace_range(start..end, "");
        self.cursor_chars -= 1;
        self.history_index = None;
    }

    pub fn delete(&mut self) {
        if self.cursor_chars >= self.grapheme_len() {
            return;
        }
        let start = grapheme_to_byte_index(&self.buffer, self.cursor_chars);
        let end = grapheme_to_byte_index(&self.buffer, self.cursor_chars + 1);
        self.buffer.replace_range(start..end, "");
        self.history_index = None;
    }

    pub fn move_left(&mut self) {
        self.cursor_chars = self.cursor_chars.saturating_sub(1);
    }

    pub fn move_right(&mut self) {
        self.cursor_chars = usize::min(self.cursor_chars + 1, self.grapheme_len());
    }

    pub fn move_home(&mut self) {
        self.cursor_chars = 0;
    }

    pub fn move_end(&mut self) {
        self.cursor_chars = self.grapheme_len();
    }

    pub fn clear_to_start(&mut self) {
        if self.cursor_chars == 0 {
            return;
        }
        let end = grapheme_to_byte_index(&self.buffer, self.cursor_chars);
        self.buffer.replace_range(0..end, "");
        self.cursor_chars = 0;
        self.history_index = None;
    }

    pub fn delete_prev_word(&mut self) {
        if self.cursor_chars == 0 {
            return;
        }
        let graphemes = self.buffer.graphemes(true).collect::<Vec<_>>();
        let mut start = self.cursor_chars;
        while start > 0 && grapheme_is_whitespace(graphemes[start - 1]) {
            start -= 1;
        }
        while start > 0 && !grapheme_is_whitespace(graphemes[start - 1]) {
            start -= 1;
        }
        let start_byte = grapheme_to_byte_index(&self.buffer, start);
        let end_byte = grapheme_to_byte_index(&self.buffer, self.cursor_chars);
        self.buffer.replace_range(start_byte..end_byte, "");
        self.cursor_chars = start;
        self.history_index = None;
    }

    pub fn replace_range(&mut self, start_byte: usize, end_byte: usize, replacement: &str) {
        self.buffer.replace_range(start_byte..end_byte, replacement);
        self.cursor_chars = grapheme_count(&self.buffer[..start_byte + replacement.len()]);
        self.history_index = None;
    }
}
