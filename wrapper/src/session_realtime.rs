use serde_json::Value;

#[path = "session_realtime_item.rs"]
mod session_realtime_item;
#[path = "session_realtime_status.rs"]
mod session_realtime_status;

use crate::state::AppState;

pub(crate) fn render_realtime_status(state: &AppState) -> String {
    session_realtime_status::render_realtime_status(state)
}

pub(crate) fn render_realtime_item(item: &Value) -> String {
    session_realtime_item::render_realtime_item(item)
}
