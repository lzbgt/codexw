use std::process::ChildStdin;

use anyhow::Result;
use serde_json::Value;

#[path = "thread_realtime.rs"]
mod thread_realtime;
#[path = "thread_review.rs"]
mod thread_review;

use crate::state::AppState;

pub(crate) fn send_thread_realtime_start(
    writer: &mut ChildStdin,
    state: &mut AppState,
    thread_id: String,
    prompt: String,
) -> Result<()> {
    thread_realtime::send_thread_realtime_start(writer, state, thread_id, prompt)
}

pub(crate) fn send_thread_realtime_append_text(
    writer: &mut ChildStdin,
    state: &mut AppState,
    thread_id: String,
    text: String,
) -> Result<()> {
    thread_realtime::send_thread_realtime_append_text(writer, state, thread_id, text)
}

pub(crate) fn send_thread_realtime_stop(
    writer: &mut ChildStdin,
    state: &mut AppState,
    thread_id: String,
) -> Result<()> {
    thread_realtime::send_thread_realtime_stop(writer, state, thread_id)
}

pub(crate) fn send_start_review(
    writer: &mut ChildStdin,
    state: &mut AppState,
    thread_id: String,
    review_target: Value,
    target_description: String,
) -> Result<()> {
    thread_review::send_start_review(writer, state, thread_id, review_target, target_description)
}
