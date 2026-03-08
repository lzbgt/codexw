use unicode_segmentation::UnicodeSegmentation;

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
        grapheme_to_byte_index(&self.buffer, self.cursor_chars)
    }

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
            self.cursor_chars = self.grapheme_len();
        }
    }

    pub fn history_next(&mut self) {
        let Some(index) = self.history_index else {
            return;
        };
        if index + 1 >= self.history.len() {
            self.history_index = None;
            self.buffer = self.draft_before_history.clone();
            self.cursor_chars = self.grapheme_len();
            return;
        }
        self.history_index = Some(index + 1);
        self.buffer = self.history[index + 1].clone();
        self.cursor_chars = self.grapheme_len();
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
        self.cursor_chars = grapheme_count(&self.buffer[..start_byte + replacement.len()]);
        self.history_index = None;
    }

    fn grapheme_len(&self) -> usize {
        grapheme_count(&self.buffer)
    }
}

fn grapheme_to_byte_index(text: &str, grapheme_index: usize) -> usize {
    if grapheme_index == 0 {
        return 0;
    }
    UnicodeSegmentation::grapheme_indices(text, true)
        .nth(grapheme_index)
        .map(|(idx, _)| idx)
        .unwrap_or(text.len())
}

fn grapheme_count(text: &str) -> usize {
    UnicodeSegmentation::graphemes(text, true).count()
}

fn grapheme_is_whitespace(grapheme: &str) -> bool {
    grapheme.chars().all(char::is_whitespace)
}
