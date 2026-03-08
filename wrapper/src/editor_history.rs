use crate::editor::EditorEvent;
use crate::editor::LineEditor;

impl LineEditor {
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
}
