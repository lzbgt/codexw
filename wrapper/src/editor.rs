#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EditorEvent {
    Submit(String),
    CtrlC,
    Noop,
}

#[derive(Debug, Default)]
pub struct LineEditor {
    buffer: String,
    cursor_chars: usize,
    history: Vec<String>,
    history_index: Option<usize>,
    draft_before_history: String,
}

impl LineEditor {
    pub fn buffer(&self) -> &str {
        &self.buffer
    }

    pub fn cursor_chars(&self) -> usize {
        self.cursor_chars
    }

    pub fn cursor_byte_index(&self) -> usize {
        char_to_byte_index(&self.buffer, self.cursor_chars)
    }

    pub fn insert_char(&mut self, ch: char) {
        let byte_index = char_to_byte_index(&self.buffer, self.cursor_chars);
        self.buffer.insert(byte_index, ch);
        self.cursor_chars += 1;
        self.history_index = None;
    }

    pub fn insert_str(&mut self, text: &str) {
        let byte_index = char_to_byte_index(&self.buffer, self.cursor_chars);
        self.buffer.insert_str(byte_index, text);
        self.cursor_chars += text.chars().count();
        self.history_index = None;
    }

    pub fn insert_newline(&mut self) {
        self.insert_char('\n');
    }

    pub fn backspace(&mut self) {
        if self.cursor_chars == 0 {
            return;
        }
        let start = char_to_byte_index(&self.buffer, self.cursor_chars - 1);
        let end = char_to_byte_index(&self.buffer, self.cursor_chars);
        self.buffer.replace_range(start..end, "");
        self.cursor_chars -= 1;
        self.history_index = None;
    }

    pub fn delete(&mut self) {
        if self.cursor_chars >= self.char_len() {
            return;
        }
        let start = char_to_byte_index(&self.buffer, self.cursor_chars);
        let end = char_to_byte_index(&self.buffer, self.cursor_chars + 1);
        self.buffer.replace_range(start..end, "");
        self.history_index = None;
    }

    pub fn move_left(&mut self) {
        self.cursor_chars = self.cursor_chars.saturating_sub(1);
    }

    pub fn move_right(&mut self) {
        self.cursor_chars = usize::min(self.cursor_chars + 1, self.char_len());
    }

    pub fn move_home(&mut self) {
        self.cursor_chars = 0;
    }

    pub fn move_end(&mut self) {
        self.cursor_chars = self.char_len();
    }

    pub fn clear_to_start(&mut self) {
        if self.cursor_chars == 0 {
            return;
        }
        let end = char_to_byte_index(&self.buffer, self.cursor_chars);
        self.buffer.replace_range(0..end, "");
        self.cursor_chars = 0;
        self.history_index = None;
    }

    pub fn delete_prev_word(&mut self) {
        if self.cursor_chars == 0 {
            return;
        }
        let chars = self.buffer.chars().collect::<Vec<_>>();
        let mut start = self.cursor_chars;
        while start > 0 && chars[start - 1].is_whitespace() {
            start -= 1;
        }
        while start > 0 && !chars[start - 1].is_whitespace() {
            start -= 1;
        }
        let start_byte = char_to_byte_index(&self.buffer, start);
        let end_byte = char_to_byte_index(&self.buffer, self.cursor_chars);
        self.buffer.replace_range(start_byte..end_byte, "");
        self.cursor_chars = start;
        self.history_index = None;
    }

    pub fn history_prev(&mut self) {
        if self.history.is_empty() {
            return;
        }
        match self.history_index {
            None => {
                self.draft_before_history = self.buffer.clone();
                self.history_index = Some(self.history.len() - 1);
            }
            Some(0) => {}
            Some(index) => self.history_index = Some(index - 1),
        }
        if let Some(index) = self.history_index {
            self.buffer = self.history[index].clone();
            self.cursor_chars = self.char_len();
        }
    }

    pub fn history_next(&mut self) {
        let Some(index) = self.history_index else {
            return;
        };
        if index + 1 >= self.history.len() {
            self.history_index = None;
            self.buffer = self.draft_before_history.clone();
            self.cursor_chars = self.char_len();
            return;
        }
        self.history_index = Some(index + 1);
        self.buffer = self.history[index + 1].clone();
        self.cursor_chars = self.char_len();
    }

    pub fn submit(&mut self) -> EditorEvent {
        let line = self.buffer.trim_end_matches(['\n', '\r']).to_string();
        if line.trim().is_empty() {
            self.buffer.clear();
            self.cursor_chars = 0;
            self.history_index = None;
            self.draft_before_history.clear();
            return EditorEvent::Noop;
        }
        if self.history.last() != Some(&line) {
            self.history.push(line.clone());
        }
        self.buffer.clear();
        self.cursor_chars = 0;
        self.history_index = None;
        self.draft_before_history.clear();
        EditorEvent::Submit(line)
    }

    pub fn ctrl_c(&mut self) -> EditorEvent {
        if self.buffer.is_empty() {
            EditorEvent::CtrlC
        } else {
            self.buffer.clear();
            self.cursor_chars = 0;
            self.history_index = None;
            self.draft_before_history.clear();
            EditorEvent::Noop
        }
    }

    pub fn clear(&mut self) {
        self.buffer.clear();
        self.cursor_chars = 0;
        self.history_index = None;
        self.draft_before_history.clear();
    }

    pub fn replace_range(&mut self, start_byte: usize, end_byte: usize, replacement: &str) {
        self.buffer.replace_range(start_byte..end_byte, replacement);
        self.cursor_chars = self.buffer[..start_byte + replacement.len()]
            .chars()
            .count();
        self.history_index = None;
    }

    fn char_len(&self) -> usize {
        self.buffer.chars().count()
    }
}

fn char_to_byte_index(text: &str, char_index: usize) -> usize {
    if char_index == 0 {
        return 0;
    }
    text.char_indices()
        .nth(char_index)
        .map(|(idx, _)| idx)
        .unwrap_or(text.len())
}

#[cfg(test)]
mod tests {
    use super::EditorEvent;
    use super::LineEditor;

    #[test]
    fn supports_cursor_movement_and_delete() {
        let mut editor = LineEditor::default();
        for ch in "hello".chars() {
            editor.insert_char(ch);
        }
        editor.move_left();
        editor.move_left();
        editor.delete();
        assert_eq!(editor.buffer(), "helo");
        assert_eq!(editor.cursor_chars(), 3);
    }

    #[test]
    fn backspace_removes_previous_character() {
        let mut editor = LineEditor::default();
        for ch in "abc".chars() {
            editor.insert_char(ch);
        }
        editor.backspace();
        assert_eq!(editor.buffer(), "ab");
        assert_eq!(editor.cursor_chars(), 2);
    }

    #[test]
    fn backspace_removes_previous_newline_boundary() {
        let mut editor = LineEditor::default();
        editor.insert_str("ab\ncd");
        editor.move_left();
        editor.move_left();
        editor.backspace();
        assert_eq!(editor.buffer(), "abcd");
        assert_eq!(editor.cursor_chars(), 2);
    }

    #[test]
    fn delete_removes_next_newline_boundary() {
        let mut editor = LineEditor::default();
        editor.insert_str("ab\ncd");
        editor.move_home();
        editor.move_right();
        editor.move_right();
        editor.delete();
        assert_eq!(editor.buffer(), "abcd");
        assert_eq!(editor.cursor_chars(), 2);
    }

    #[test]
    fn history_navigation_restores_draft() {
        let mut editor = LineEditor::default();
        for ch in "first".chars() {
            editor.insert_char(ch);
        }
        assert_eq!(editor.submit(), EditorEvent::Submit("first".to_string()));
        for ch in "second".chars() {
            editor.insert_char(ch);
        }
        assert_eq!(editor.submit(), EditorEvent::Submit("second".to_string()));
        for ch in "dra".chars() {
            editor.insert_char(ch);
        }
        editor.history_prev();
        assert_eq!(editor.buffer(), "second");
        editor.history_prev();
        assert_eq!(editor.buffer(), "first");
        editor.history_next();
        assert_eq!(editor.buffer(), "second");
        editor.history_next();
        assert_eq!(editor.buffer(), "dra");
    }

    #[test]
    fn ctrl_u_clears_to_start_of_line() {
        let mut editor = LineEditor::default();
        for ch in "hello world".chars() {
            editor.insert_char(ch);
        }
        editor.move_left();
        editor.move_left();
        editor.clear_to_start();
        assert_eq!(editor.buffer(), "ld");
        assert_eq!(editor.cursor_chars(), 0);
    }

    #[test]
    fn ctrl_w_deletes_previous_word() {
        let mut editor = LineEditor::default();
        for ch in "hello brave world".chars() {
            editor.insert_char(ch);
        }
        editor.delete_prev_word();
        assert_eq!(editor.buffer(), "hello brave ");
        assert_eq!(editor.cursor_chars(), "hello brave ".chars().count());
    }

    #[test]
    fn insert_newline_preserves_multiline_submit() {
        let mut editor = LineEditor::default();
        editor.insert_str("first");
        editor.insert_newline();
        editor.insert_str("second");
        assert_eq!(editor.buffer(), "first\nsecond");
        assert_eq!(
            editor.submit(),
            EditorEvent::Submit("first\nsecond".to_string())
        );
    }
}
