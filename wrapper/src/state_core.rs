#[path = "state_model.rs"]
mod state_model;
#[path = "state_mutations.rs"]
mod state_mutations;

use crate::rpc::RequestId;
pub(crate) use state_model::AppState;
pub(crate) use state_model::ProcessOutputBuffer;

impl AppState {
    pub(crate) fn next_request_id(&mut self) -> RequestId {
        let id = self.next_request_id;
        self.next_request_id += 1;
        RequestId::Integer(id)
    }
}
