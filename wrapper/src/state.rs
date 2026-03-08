use crate::rpc::RequestId;
pub(crate) use crate::state_helpers::buffer_item_delta;
pub(crate) use crate::state_helpers::buffer_process_delta;
pub(crate) use crate::state_helpers::canonicalize_or_keep;
pub(crate) use crate::state_helpers::emit_status_line;
pub(crate) use crate::state_helpers::get_string;
pub(crate) use crate::state_helpers::summarize_text;
pub(crate) use crate::state_helpers::thread_id;
pub(crate) use crate::state_model::AppState;
pub(crate) use crate::state_model::ProcessOutputBuffer;

impl AppState {
    pub(crate) fn next_request_id(&mut self) -> RequestId {
        let id = self.next_request_id;
        self.next_request_id += 1;
        RequestId::Integer(id)
    }
}
