pub(crate) use crate::editor_graphemes::grapheme_count;
pub(crate) use crate::editor_graphemes::grapheme_is_whitespace;
pub(crate) use crate::editor_graphemes::grapheme_to_byte_index;

#[path = "editor_buffer.rs"]
mod editor_buffer;
#[path = "editor_history.rs"]
mod editor_history;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EditorEvent {
    Submit(String),
    CtrlC,
    Noop,
}

#[derive(Debug, Default)]
pub struct LineEditor {
    pub(crate) buffer: String,
    pub(crate) cursor_chars: usize,
    pub(crate) history: Vec<String>,
    pub(crate) history_index: Option<usize>,
    pub(crate) draft_before_history: String,
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
    pub(crate) fn grapheme_len(&self) -> usize {
        grapheme_count(&self.buffer)
    }
}
