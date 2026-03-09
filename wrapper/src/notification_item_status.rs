mod item;

use anyhow::Result;
use serde_json::Value;

use crate::Cli;
use crate::notification_item_status::item::render_item_started;
use crate::output::Output;
use crate::state::AppState;
use crate::transcript_approval_summary::summarize_server_request_resolved;
use crate::transcript_status_summary::summarize_model_reroute;

pub(crate) fn handle_status_update(
    method: &str,
    params: &Value,
    cli: &Cli,
    state: &mut AppState,
    output: &mut Output,
) -> Result<bool> {
    match method {
        "model/rerouted" => {
            output.line_stderr(format!("[model] {}", summarize_model_reroute(params)))?;
        }
        "item/started" => render_item_started(params, cli, state)?,
        "item/agentMessage/delta"
        | "item/reasoning/summaryTextDelta"
        | "item/reasoning/textDelta"
        | "item/reasoning/summaryPartAdded" => {}
        "serverRequest/resolved" => {
            if state.last_status_line.as_deref() == Some("waiting on approval") {
                state.last_status_line = None;
            }
            if cli.verbose_events {
                output.line_stderr(format!(
                    "[approval] resolved {}",
                    summarize_server_request_resolved(params)
                ))?;
            }
        }
        _ => return Ok(false),
    }
    Ok(true)
}
