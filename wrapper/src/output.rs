#[path = "output_prompt.rs"]
mod output_prompt;
#[path = "output_stream.rs"]
mod output_stream;

pub(crate) const CLEAR_LINE: &str = "\r\x1b[2K";
pub(crate) use output_stream::write_crlf;

#[derive(Default)]
pub struct Output {
    pub(crate) prompt: Option<String>,
    pub(crate) status: Option<String>,
    pub(crate) prompt_visible: bool,
    pub(crate) status_visible: bool,
}

impl Output {
    pub fn set_prompt(&mut self, prompt: Option<String>) {
        self.prompt = prompt;
    }

    pub fn set_status(&mut self, status: Option<String>) {
        self.status = status;
    }
}
