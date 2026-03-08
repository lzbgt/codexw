use std::process::ChildStdin;
use std::time::Instant;

use anyhow::Result;

use crate::Cli;
use crate::input::build_turn_input;
use crate::output::Output;
use crate::prompt::build_continue_prompt;
use crate::prompt::parse_auto_mode_stop;
use crate::requests::send_load_skills;
use crate::requests::send_turn_start;
use crate::rpc::RpcNotification;
use crate::state::AppState;
use crate::state::get_string;
use crate::state::thread_id;

pub(crate) fn handle_turn_lifecycle_notification(
    notification: &RpcNotification,
    cli: &Cli,
    resolved_cwd: &str,
    state: &mut AppState,
    output: &mut Output,
    writer: &mut ChildStdin,
) -> Result<bool> {
    match notification.method.as_str() {
        "skills/changed" => {
            send_load_skills(writer, state, resolved_cwd)?;
        }
        "turn/started" => {
            state.turn_running = true;
            state.activity_started_at = Some(Instant::now());
            state.started_turn_count = state.started_turn_count.saturating_add(1);
            if let Some(turn_id) = get_string(&notification.params, &["turn", "id"]) {
                state.active_turn_id = Some(turn_id.to_string());
            }
            state.reset_turn_stream_state();
            state.last_status_line = None;
        }
        "turn/completed" => {
            output.finish_stream()?;
            let status = get_string(&notification.params, &["turn", "status"])
                .unwrap_or("unknown")
                .to_string();
            let turn_id = get_string(&notification.params, &["turn", "id"])
                .unwrap_or("?")
                .to_string();
            state.turn_running = false;
            state.active_turn_id = None;
            state.activity_started_at = None;
            state.last_status_line = None;
            if matches!(
                status.as_str(),
                "completed" | "interrupted" | "failed" | "cancelled"
            ) {
                state.completed_turn_count = state.completed_turn_count.saturating_add(1);
            }
            if status != "completed" {
                output.line_stderr(format!("[turn] completed {turn_id} status={status}"))?;
            }

            if status == "completed" {
                if let Some(message) = state.last_agent_message.clone() {
                    let stop = parse_auto_mode_stop(&message);
                    if state.auto_continue && !stop {
                        let thread_id = thread_id(state)?.to_string();
                        let continue_prompt =
                            build_continue_prompt(state.objective.as_deref(), &message);
                        let submission = build_turn_input(
                            &continue_prompt,
                            resolved_cwd,
                            &[],
                            &[],
                            &state.apps,
                            &state.plugins,
                            &state.skills,
                        );
                        output.line_stderr("[auto] continuing remaining work")?;
                        send_turn_start(
                            writer,
                            state,
                            cli,
                            resolved_cwd,
                            thread_id,
                            submission,
                            true,
                        )?;
                    } else if stop {
                        output.line_stderr("[ready] stop marker observed")?;
                    } else {
                        output.line_stderr("[ready]")?;
                    }
                } else {
                    output.line_stderr("[ready]")?;
                }
            } else {
                state.last_agent_message = None;
                output.line_stderr("[ready]")?;
            }
        }
        _ => return Ok(false),
    }
    Ok(true)
}
