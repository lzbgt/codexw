use std::time::Instant;

use crate::rpc::RpcNotification;
use crate::state::AppState;
use crate::state::get_string;

pub(crate) fn handle_turn_started(notification: &RpcNotification, state: &mut AppState) {
    let now = Instant::now();
    state.turn_running = true;
    state.activity_started_at = Some(now);
    state.last_server_event_at = Some(now);
    state.started_turn_count = state.started_turn_count.saturating_add(1);
    if let Some(turn_id) = get_string(&notification.params, &["turn", "id"]) {
        state.active_turn_id = Some(turn_id.to_string());
    }
    state.reset_turn_stream_state();
    state.last_status_line = None;
}
