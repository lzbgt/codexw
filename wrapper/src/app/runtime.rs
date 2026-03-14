use std::sync::mpsc;

use anyhow::Context;
use anyhow::Result;
use serde_json::json;

use crate::Cli;
use crate::app::resume::emit_resume_exit_hint;
use crate::app_input_controls::handle_control_key;
use crate::app_input_editing::handle_editing_key;
use crate::config_persistence::load_persisted_theme;
use crate::dispatch_command_utils::join_prompt;
use crate::editor::LineEditor;
use crate::events::process_server_line;
use crate::local_api::new_command_queue;
use crate::local_api::new_event_log;
use crate::local_api::new_process_session_id;
use crate::local_api::new_shared_snapshot;
use crate::local_api::process_local_api_commands;
use crate::local_api::publish_snapshot_change_events;
use crate::local_api::start_local_api;
use crate::local_api::sync_shared_snapshot;
use crate::output::Output;
use crate::prompt_state::prompt_accepts_input;
use crate::prompt_state::update_prompt;
use crate::render_markdown_code::set_theme;
use crate::requests::send_initialize;
use crate::requests::send_json;
use crate::rpc::OutgoingResponse;
use crate::runtime_event_sources::AppEvent;
use crate::runtime_event_sources::AsyncToolResponse;
use crate::runtime_event_sources::RawModeGuard;
use crate::runtime_event_sources::start_stdin_thread;
use crate::runtime_event_sources::start_stdout_thread;
use crate::runtime_event_sources::start_tick_thread;
use crate::runtime_keys::InputKey;
use crate::runtime_process::build_start_mode;
use crate::runtime_process::effective_cwd;
use crate::runtime_process::shutdown_child;
use crate::runtime_process::spawn_server;
use crate::state::AppState;
use crate::state::AsyncToolHealthCheck;
use crate::state::SupervisionNoticeTransition;

pub(crate) fn run(cli: Cli) -> Result<()> {
    let initial_prompt = join_prompt(&cli.prompt);
    let resolved_cwd = effective_cwd(&cli)?;
    let _raw_mode = RawModeGuard::new()?;

    let mut child = spawn_server(&cli, &resolved_cwd)?;
    let stdin = child
        .stdin
        .take()
        .context("codex app-server stdin unavailable")?;
    let stdout = child
        .stdout
        .take()
        .context("codex app-server stdout unavailable")?;

    let (tx, rx) = mpsc::channel::<AppEvent>();
    start_stdout_thread(stdout, tx.clone());
    start_stdin_thread(tx.clone());
    start_tick_thread(tx.clone());

    let mut output = Output::default();
    let mut writer = stdin;
    let mut state = AppState::new(cli.auto_continue, cli.raw_json);
    let mut editor = LineEditor::default();
    let local_api_snapshot = new_shared_snapshot(new_process_session_id(), resolved_cwd.clone());
    let local_api_commands = new_command_queue();
    let local_api_events = new_event_log();
    let mut previous_local_api_snapshot = None;
    let current_local_api_snapshot = sync_shared_snapshot(&local_api_snapshot, &state);
    publish_snapshot_change_events(
        &local_api_events,
        previous_local_api_snapshot.as_ref(),
        &current_local_api_snapshot,
    );
    previous_local_api_snapshot = Some(current_local_api_snapshot);
    let local_api_handle = start_local_api(
        &cli,
        local_api_snapshot.clone(),
        local_api_commands.clone(),
        state.orchestration.background_shells.clone(),
        local_api_events.clone(),
    )?;
    if let Some(handle) = local_api_handle.as_ref() {
        output.line_stderr(format!(
            "[session] local API listening on http://{}",
            handle.bind_addr()
        ))?;
    }

    match load_persisted_theme() {
        Ok(Some(theme_name)) => set_theme(&theme_name),
        Ok(None) => {}
        Err(err) => output.line_stderr(format!("[session] failed to load saved theme: {err:#}"))?,
    }

    output.line_stderr("[session] connecting to codex app-server")?;
    send_initialize(&mut writer, &mut state, &cli, !cli.no_experimental_api)?;

    let mut start_after_initialize = Some(build_start_mode(&cli, initial_prompt));

    loop {
        update_prompt(&mut output, &state, &editor)?;
        match rx.recv() {
            Ok(AppEvent::ServerLine(line)) => {
                process_server_line(
                    line,
                    &cli,
                    &resolved_cwd,
                    &mut state,
                    &mut output,
                    &mut writer,
                    &tx,
                    &mut start_after_initialize,
                )?;
            }
            Ok(AppEvent::InputKey(key)) => {
                if !handle_input_key(
                    key,
                    &cli,
                    &resolved_cwd,
                    &mut state,
                    &mut editor,
                    &mut output,
                    &mut writer,
                )? {
                    break;
                }
            }
            Ok(AppEvent::Tick) => {
                handle_supervision_tick(&mut state, &mut output, &mut writer)?;
            }
            Ok(AppEvent::AsyncToolResponseReady(tool_response)) => {
                handle_async_tool_response(tool_response, &mut state, &mut output, &mut writer)?;
            }
            Ok(AppEvent::StdinClosed) => {
                output.line_stderr("[session] stdin closed; exiting")?;
                break;
            }
            Ok(AppEvent::ServerClosed) => {
                output.line_stderr("[session] codex app-server exited")?;
                break;
            }
            Err(_) => break,
        }
        process_local_api_commands(
            &cli,
            &resolved_cwd,
            &mut state,
            &mut output,
            &mut writer,
            &local_api_snapshot,
            &local_api_commands,
        )?;
        let current_local_api_snapshot = sync_shared_snapshot(&local_api_snapshot, &state);
        publish_snapshot_change_events(
            &local_api_events,
            previous_local_api_snapshot.as_ref(),
            &current_local_api_snapshot,
        );
        previous_local_api_snapshot = Some(current_local_api_snapshot);
    }

    emit_resume_exit_hint(&mut output, &state, &resolved_cwd)?;
    if let Some(handle) = local_api_handle {
        handle.shutdown()?;
    }
    shutdown_child(writer, child)?;
    Ok(())
}

fn handle_async_tool_response(
    tool_response: AsyncToolResponse,
    state: &mut AppState,
    output: &mut Output,
    writer: &mut std::process::ChildStdin,
) -> Result<()> {
    if state.finish_async_tool_request(&tool_response.id).is_none() {
        if let Some(abandoned) = state.finish_abandoned_async_tool_request(&tool_response.id) {
            output.line_stderr(format!(
                "[tool] abandoned async tool worker finally returned after {}s: {}",
                abandoned.timed_out_elapsed().as_secs(),
                tool_response.summary
            ))?;
            return Ok(());
        }
        output.line_stderr(format!(
            "[tool] dropped late async tool response: {}",
            tool_response.summary
        ))?;
        return Ok(());
    }
    let _ = state.refresh_async_tool_supervision_notice();
    let success = tool_response
        .result
        .get("success")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false);
    output.line_stderr(format!(
        "[tool] dynamic tool {}: {}",
        if success { "completed" } else { "failed" },
        tool_response.summary
    ))?;
    send_json(
        writer,
        &OutgoingResponse {
            id: tool_response.id,
            result: tool_response.result,
        },
    )?;
    Ok(())
}

fn handle_supervision_tick(
    state: &mut AppState,
    output: &mut Output,
    writer: &mut std::process::ChildStdin,
) -> Result<()> {
    for expired in state.expire_timed_out_async_tool_requests() {
        let backlog = state.abandoned_async_tool_request_count();
        output.line_stderr(format!(
            "[self-supervision] forcing async tool failure after {}s (limit {}s, abandoned backlog {}): {}",
            expired.elapsed.as_secs(),
            expired.hard_timeout.as_secs(),
            backlog,
            expired.summary
        ))?;
        send_json(
            writer,
            &OutgoingResponse {
                id: expired.id,
                result: json!({
                    "contentItems": [{
                        "type": "inputText",
                        "text": format!(
                        "dynamic tool `{}` exceeded its {}s runtime limit and was failed locally to avoid hanging the active turn; summary: {}",
                            expired.tool,
                            expired.hard_timeout.as_secs(),
                            expired.summary
                        )
                    }],
                    "success": false
                }),
            },
        )?;
    }
    for check in state.collect_due_async_tool_health_checks() {
        output.line_stderr(format_async_tool_health_check_line(&check))?;
    }
    match state.refresh_async_tool_supervision_notice() {
        Some(SupervisionNoticeTransition::Raised(notice)) => {
            output.line_stderr(format!(
                "[self-supervision] {} {} [{}|{}] {}",
                notice.classification.label(),
                notice.tool,
                notice.recovery_policy_kind().label(),
                notice.recommended_action(),
                notice.summary
            ))?;
        }
        Some(SupervisionNoticeTransition::Cleared) => {
            output.line_stderr("[self-supervision] async tool supervision cleared")?;
        }
        None => {}
    }
    Ok(())
}

fn format_async_tool_health_check_line(check: &AsyncToolHealthCheck) -> String {
    let inspection = match check.supervision_classification {
        Some(classification) => format!(
            "{}|{}",
            classification.label(),
            classification.recommended_action()
        ),
        None => "monitoring".to_string(),
    };
    let call = check
        .source_call_id
        .as_deref()
        .map(|value| format!(" call={value}"))
        .unwrap_or_default();
    let target = match (
        check.target_background_shell_reference.as_deref(),
        check.target_background_shell_job_id.as_deref(),
    ) {
        (Some(reference), Some(job_id)) if reference != job_id => {
            format!(
                " target={} resolved={job_id}",
                crate::state::summarize_text(reference)
            )
        }
        (Some(reference), _) => format!(" target={}", crate::state::summarize_text(reference)),
        (None, Some(job_id)) => format!(" target={job_id}"),
        (None, None) => String::new(),
    };
    let observation = match check.observed_background_shell_job.as_ref() {
        Some(job) => {
            let output_age = job
                .last_output_age
                .map(|age| format!(" output_age={}s", age.as_secs()))
                .unwrap_or_default();
            let output = job
                .latest_output_preview()
                .map(|line| format!(" output={}", crate::state::summarize_text(line)))
                .unwrap_or_default();
            format!(
                "{}|{} {} via {}{}{} job={} {} lines={} command={}{}{}",
                check.observation_state.label(),
                check.output_state.label(),
                check.owner_kind.label(),
                check.worker_thread_name,
                call,
                target,
                job.job_id,
                job.status,
                job.total_lines,
                crate::state::summarize_text(&job.command),
                output_age,
                output
            )
        }
        None => format!(
            "{}|{} {} via {}{}{}",
            check.observation_state.label(),
            check.output_state.label(),
            check.owner_kind.label(),
            check.worker_thread_name,
            call,
            target
        ),
    };
    format!(
        "[self-supervision] async worker check {}s [{}] {} next={}s for {}: {}",
        check.elapsed.as_secs(),
        inspection,
        observation,
        check.next_health_check_in.as_secs(),
        check.tool,
        check.summary
    )
}

fn handle_input_key(
    key: InputKey,
    cli: &Cli,
    resolved_cwd: &str,
    state: &mut AppState,
    editor: &mut LineEditor,
    output: &mut Output,
    writer: &mut std::process::ChildStdin,
) -> Result<bool> {
    let accepts_input = prompt_accepts_input(state);
    if let Some(continue_running) = handle_control_key(
        &key,
        cli,
        resolved_cwd,
        state,
        editor,
        output,
        writer,
        accepts_input,
    )? {
        return Ok(continue_running);
    }
    handle_editing_key(&key, resolved_cwd, state, editor, output, accepts_input)?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rpc::RequestId;
    use serde_json::json;
    use std::process::Command;
    use std::process::Stdio;

    fn spawn_sink_stdin() -> std::process::ChildStdin {
        Command::new("sh")
            .arg("-c")
            .arg("cat >/dev/null")
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("spawn sink")
            .stdin
            .take()
            .expect("stdin")
    }

    #[test]
    fn async_tool_response_clears_active_request_tracking() {
        let mut state = AppState::new(true, false);
        state.record_async_tool_request(
            RequestId::Integer(42),
            "background_shell_start".to_string(),
            "arguments= command=sleep 5 tool=background_shell_start".to_string(),
        );
        let mut output = Output::default();
        let mut writer = spawn_sink_stdin();

        handle_async_tool_response(
            AsyncToolResponse {
                id: RequestId::Integer(42),
                tool: "background_shell_start".to_string(),
                summary: "arguments= command=sleep 5 tool=background_shell_start".to_string(),
                result: json!({"success": true}),
            },
            &mut state,
            &mut output,
            &mut writer,
        )
        .expect("handle async tool response");

        assert!(state.active_async_tool_requests.is_empty());
        assert!(state.active_supervision_notice.is_none());
    }

    #[test]
    fn supervision_tick_tracks_raise_escalation_and_clear() {
        let mut state = AppState::new(true, false);
        state.record_async_tool_request(
            RequestId::Integer(7),
            "background_shell_start".to_string(),
            "arguments= command=sleep 5 tool=background_shell_start".to_string(),
        );
        if let Some(activity) = state
            .active_async_tool_requests
            .get_mut(&RequestId::Integer(7))
        {
            activity.started_at = std::time::Instant::now() - std::time::Duration::from_secs(20);
        }
        let mut output = Output::default();
        let mut writer = spawn_sink_stdin();

        handle_supervision_tick(&mut state, &mut output, &mut writer).expect("raise slow notice");
        assert_eq!(
            state
                .active_supervision_notice
                .as_ref()
                .map(|notice| notice.classification.label()),
            Some("tool_slow")
        );

        if let Some(activity) = state
            .active_async_tool_requests
            .get_mut(&RequestId::Integer(7))
        {
            activity.started_at = std::time::Instant::now() - std::time::Duration::from_secs(75);
        }
        handle_supervision_tick(&mut state, &mut output, &mut writer).expect("raise wedged notice");
        assert_eq!(
            state
                .active_supervision_notice
                .as_ref()
                .map(|notice| notice.classification.label()),
            Some("tool_wedged")
        );

        state.finish_async_tool_request(&RequestId::Integer(7));
        handle_supervision_tick(&mut state, &mut output, &mut writer).expect("clear notice");
        assert!(state.active_supervision_notice.is_none());
    }

    #[test]
    fn format_async_tool_health_check_line_reports_started_silent_job_details() {
        let line = format_async_tool_health_check_line(&AsyncToolHealthCheck {
            request_id: "9".to_string(),
            tool: "background_shell_start".to_string(),
            summary: "arguments= command=sleep 20 tool=background_shell_start".to_string(),
            owner_kind: crate::state::AsyncToolOwnerKind::WrapperBackgroundShell,
            source_call_id: Some("call-999".to_string()),
            target_background_shell_reference: Some("dev.api".to_string()),
            target_background_shell_job_id: Some("bg-9".to_string()),
            worker_thread_name: "codexw-bgtool-background_shell_start-9".to_string(),
            elapsed: std::time::Duration::from_secs(18),
            next_health_check_in: std::time::Duration::from_secs(5),
            supervision_classification: Some(crate::state::AsyncToolSupervisionClass::ToolSlow),
            observation_state:
                crate::state::AsyncToolObservationState::WrapperBackgroundShellStartedNoOutputYet,
            output_state: crate::state::AsyncToolOutputState::NoOutputObservedYet,
            observed_background_shell_job: Some(
                crate::state::AsyncToolObservedBackgroundShellJob {
                    job_id: "bg-9".to_string(),
                    status: "running".to_string(),
                    command: "sleep 20".to_string(),
                    total_lines: 0,
                    last_output_age: None,
                    recent_lines: Vec::new(),
                },
            ),
        });

        assert!(line.contains("async worker check 18s"));
        assert!(line.contains("[tool_slow|observe_or_interrupt]"));
        assert!(
            line.contains("wrapper_background_shell_started_no_output_yet|no_output_observed_yet")
        );
        assert!(line.contains("call=call-999"));
        assert!(line.contains("target=dev.api"));
        assert!(line.contains("resolved=bg-9"));
        assert!(line.contains("job=bg-9 running"));
        assert!(line.contains("lines=0"));
        assert!(line.contains("command=sleep 20"));
        assert!(line.contains("next=5s"));
    }

    #[test]
    fn format_async_tool_health_check_line_reports_streaming_output_details() {
        let line = format_async_tool_health_check_line(&AsyncToolHealthCheck {
            request_id: "10".to_string(),
            tool: "background_shell_start".to_string(),
            summary: "arguments= command=python stage2.py --quick tool=background_shell_start"
                .to_string(),
            owner_kind: crate::state::AsyncToolOwnerKind::WrapperBackgroundShell,
            source_call_id: Some("call-1000".to_string()),
            target_background_shell_reference: Some("dev.api".to_string()),
            target_background_shell_job_id: Some("bg-10".to_string()),
            worker_thread_name: "codexw-bgtool-background_shell_start-10".to_string(),
            elapsed: std::time::Duration::from_secs(24),
            next_health_check_in: std::time::Duration::from_secs(9),
            supervision_classification: Some(crate::state::AsyncToolSupervisionClass::ToolSlow),
            observation_state:
                crate::state::AsyncToolObservationState::WrapperBackgroundShellStreamingOutput,
            output_state: crate::state::AsyncToolOutputState::RecentOutputObserved,
            observed_background_shell_job: Some(
                crate::state::AsyncToolObservedBackgroundShellJob {
                    job_id: "bg-10".to_string(),
                    status: "running".to_string(),
                    command: "python stage2.py --quick".to_string(),
                    total_lines: 3,
                    last_output_age: Some(std::time::Duration::from_secs(2)),
                    recent_lines: vec!["stage1 ok".to_string(), "READY".to_string()],
                },
            ),
        });

        assert!(line.contains("wrapper_background_shell_streaming_output|recent_output_observed"));
        assert!(line.contains("output_age=2s"));
        assert!(line.contains("output=READY"));
        assert!(line.contains("next=9s"));
        assert!(line.contains("target=dev.api"));
        assert!(line.contains("resolved=bg-10"));
        assert!(line.contains("job=bg-10 running"));
    }

    #[test]
    fn supervision_tick_force_fails_timed_out_async_tool_requests() {
        let mut state = AppState::new(true, false);
        state.record_async_tool_request_with_timeout(
            RequestId::Integer(9),
            "background_shell_start".to_string(),
            "arguments= command=sleep 5 tool=background_shell_start".to_string(),
            std::time::Duration::from_secs(1),
        );
        if let Some(activity) = state
            .active_async_tool_requests
            .get_mut(&RequestId::Integer(9))
        {
            activity.started_at = std::time::Instant::now() - std::time::Duration::from_secs(75);
        }
        let mut output = Output::default();
        let mut writer = spawn_sink_stdin();

        handle_supervision_tick(&mut state, &mut output, &mut writer)
            .expect("expire timed out async tool");

        assert!(state.active_async_tool_requests.is_empty());
        assert_eq!(state.abandoned_async_tool_request_count(), 1);
        assert!(state.active_supervision_notice.is_none());
    }

    #[test]
    fn late_async_tool_response_clears_abandoned_request_after_timeout_cleanup() {
        let mut state = AppState::new(true, false);
        state.record_async_tool_request_with_timeout(
            RequestId::Integer(404),
            "background_shell_start".to_string(),
            "arguments= command=sleep 5 tool=background_shell_start".to_string(),
            std::time::Duration::from_secs(1),
        );
        if let Some(activity) = state
            .active_async_tool_requests
            .get_mut(&RequestId::Integer(404))
        {
            activity.started_at = std::time::Instant::now() - std::time::Duration::from_secs(75);
        }
        let _expired = state.expire_timed_out_async_tool_requests();
        assert_eq!(state.abandoned_async_tool_request_count(), 1);
        let mut output = Output::default();
        let mut writer = spawn_sink_stdin();

        handle_async_tool_response(
            AsyncToolResponse {
                id: RequestId::Integer(404),
                tool: "background_shell_start".to_string(),
                summary: "arguments= command=sleep 5 tool=background_shell_start".to_string(),
                result: json!({"success": true}),
            },
            &mut state,
            &mut output,
            &mut writer,
        )
        .expect("drop late async tool response");

        assert!(state.active_async_tool_requests.is_empty());
        assert_eq!(state.abandoned_async_tool_request_count(), 0);
        assert!(state.active_supervision_notice.is_none());
    }
}
