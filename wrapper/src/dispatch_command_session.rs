use std::process::ChildStdin;

use anyhow::Result;

use crate::Cli;
use crate::catalog_views::render_apps_list;
use crate::catalog_views::render_skills_list;
use crate::collaboration::CollaborationModeAction;
use crate::dispatch_command_utils::parse_feedback_args;
use crate::editor::LineEditor;
use crate::model_session::ModelsAction;
use crate::model_session::apply_personality_selection;
use crate::model_session::render_personality_options;
use crate::output::Output;
use crate::requests::send_clean_background_terminals;
use crate::requests::send_feedback_upload;
use crate::requests::send_load_collaboration_modes;
use crate::requests::send_load_config;
use crate::requests::send_load_experimental_features;
use crate::requests::send_load_mcp_servers;
use crate::requests::send_load_models;
use crate::requests::send_logout_account;
use crate::requests::send_thread_realtime_append_text;
use crate::requests::send_thread_realtime_start;
use crate::requests::send_thread_realtime_stop;
use crate::session_realtime::render_realtime_status;
use crate::session_snapshot::render_status_snapshot;
use crate::state::AppState;
use crate::state::summarize_text;
use crate::state::thread_id;
use crate::status_views::render_permissions_snapshot;
use crate::transcript_render::render_pending_attachments;

pub(crate) fn try_handle_session_command(
    command: &str,
    args: &[&str],
    cli: &Cli,
    resolved_cwd: &str,
    state: &mut AppState,
    _editor: &mut LineEditor,
    output: &mut Output,
    writer: &mut ChildStdin,
) -> Result<Option<bool>> {
    let result = match command {
        "apps" => {
            output.block_stdout("Apps", &render_apps_list(&state.apps))?;
            true
        }
        "skills" => {
            output.block_stdout("Skills", &render_skills_list(&state.skills))?;
            true
        }
        "models" | "model" => {
            output.line_stderr("[session] loading models")?;
            send_load_models(writer, state, ModelsAction::ShowModels)?;
            true
        }
        "mcp" => {
            output.line_stderr("[session] loading MCP server status")?;
            send_load_mcp_servers(writer, state)?;
            true
        }
        "auto" => {
            let Some(mode) = args.first() else {
                output.line_stderr("[session] usage: :auto on|off")?;
                return Ok(Some(true));
            };
            state.auto_continue = match *mode {
                "on" => true,
                "off" => false,
                _ => {
                    output.line_stderr("[session] usage: :auto on|off")?;
                    return Ok(Some(true));
                }
            };
            output.line_stderr(format!(
                "[auto] {}",
                if state.auto_continue {
                    "enabled"
                } else {
                    "disabled"
                }
            ))?;
            true
        }
        "attachments" => {
            if state.pending_local_images.is_empty() && state.pending_remote_images.is_empty() {
                output.line_stderr("[draft] no queued attachments")?;
            } else {
                let rendered = render_pending_attachments(
                    &state.pending_local_images,
                    &state.pending_remote_images,
                );
                output.block_stdout("Queued attachments", &rendered)?;
            }
            true
        }
        "approvals" | "permissions" => {
            output.block_stdout("Permissions", &render_permissions_snapshot(cli))?;
            true
        }
        "status" | "statusline" => {
            output.block_stdout("Status", &render_status_snapshot(cli, resolved_cwd, state))?;
            true
        }
        "settings" | "debug-config" => {
            output.line_stderr("[session] loading effective config")?;
            send_load_config(writer, state)?;
            true
        }
        "feedback" => {
            let owned = args.iter().map(|s| (*s).to_string()).collect::<Vec<_>>();
            let Some(parsed) = parse_feedback_args(&owned) else {
                output.line_stderr(
                    "[session] usage: :feedback <bug|bad_result|good_result|safety_check|other> [reason] [--logs|--no-logs]",
                )?;
                return Ok(Some(true));
            };
            let current_thread = state.thread_id.clone();
            output.line_stderr(format!(
                "[feedback] submitting {} feedback",
                summarize_text(&parsed.classification)
            ))?;
            send_feedback_upload(
                writer,
                state,
                parsed.classification,
                parsed.reason,
                current_thread,
                parsed.include_logs,
            )?;
            true
        }
        "logout" => {
            output.line_stderr("[session] logging out")?;
            send_logout_account(writer, state)?;
            true
        }
        "experimental" => {
            output.line_stderr("[session] loading experimental feature flags")?;
            send_load_experimental_features(writer, state)?;
            true
        }
        "personality" => {
            if state.turn_running {
                output
                    .line_stderr("[session] cannot change personality while a turn is running")?;
            } else if args.is_empty() {
                if state.models.is_empty() {
                    output.line_stderr("[session] loading models for personality options")?;
                    send_load_models(writer, state, ModelsAction::ShowPersonality)?;
                } else {
                    output.block_stdout("Personality", &render_personality_options(cli, state))?;
                }
            } else {
                let selector = args.join(" ");
                if state.models.is_empty() {
                    output.line_stderr("[session] loading models for personality selection")?;
                    send_load_models(writer, state, ModelsAction::SetPersonality(selector))?;
                } else {
                    apply_personality_selection(cli, state, &selector, output)?;
                }
            }
            true
        }
        "collab" => {
            if args.is_empty() {
                send_load_collaboration_modes(writer, state, CollaborationModeAction::ShowList)?;
            } else if state.turn_running {
                output.line_stderr(
                    "[session] cannot switch collaboration mode while a turn is running",
                )?;
            } else {
                let selector = args.join(" ");
                send_load_collaboration_modes(
                    writer,
                    state,
                    CollaborationModeAction::SetMode(selector),
                )?;
            }
            true
        }
        "plan" => {
            if state.turn_running {
                output.line_stderr(
                    "[session] cannot switch collaboration mode while a turn is running",
                )?;
            } else {
                send_load_collaboration_modes(writer, state, CollaborationModeAction::TogglePlan)?;
            }
            true
        }
        "realtime" => {
            if cli.no_experimental_api {
                output.line_stderr(
                    "[session] /realtime requires experimental API support; restart without --no-experimental-api",
                )?;
                return Ok(Some(true));
            }
            let Some(thread_id) = state.thread_id.clone() else {
                output.line_stderr("[session] start or resume a thread before using /realtime")?;
                return Ok(Some(true));
            };
            if args.is_empty() || matches!(args[0], "status" | "show") {
                output.block_stdout("Realtime", &render_realtime_status(state))?;
                return Ok(Some(true));
            }
            match args[0] {
                "start" => {
                    if state.turn_running {
                        output.line_stderr(
                            "[session] cannot start realtime while a turn is running",
                        )?;
                    } else if state.realtime_active {
                        output.line_stderr(
                            "[session] realtime is already active; use /realtime stop first",
                        )?;
                        output.block_stdout("Realtime", &render_realtime_status(state))?;
                    } else {
                        let prompt = if args.len() > 1 {
                            args[1..].join(" ")
                        } else {
                            "Text-only experimental realtime session for this thread.".to_string()
                        };
                        send_thread_realtime_start(writer, state, thread_id, prompt)?;
                    }
                }
                "send" | "append" => {
                    if !state.realtime_active {
                        output.line_stderr(
                            "[session] realtime is not active; use /realtime start first",
                        )?;
                    } else if args.len() < 2 {
                        output.line_stderr("[session] usage: /realtime send <text>")?;
                    } else {
                        send_thread_realtime_append_text(
                            writer,
                            state,
                            thread_id,
                            args[1..].join(" "),
                        )?;
                    }
                }
                "stop" => {
                    if !state.realtime_active {
                        output.line_stderr("[session] realtime is not active")?;
                    } else {
                        send_thread_realtime_stop(writer, state, thread_id)?;
                    }
                }
                other => {
                    output.line_stderr(format!("[session] unknown realtime action: {other}"))?;
                    output.block_stdout("Realtime", &render_realtime_status(state))?;
                }
            }
            true
        }
        "ps" => {
            let action = args.first().copied();
            if matches!(action, Some("clean")) {
                if cli.no_experimental_api {
                    output.line_stderr(
                        "[thread] /ps clean requires experimental API support; restart without --no-experimental-api",
                    )?;
                } else {
                    let current_thread_id = thread_id(state)?.to_string();
                    output.line_stderr("[thread] cleaning background terminals")?;
                    send_clean_background_terminals(writer, state, current_thread_id)?;
                }
            } else {
                output.line_stderr(
                    "[session] app-server does not expose background-terminal listing like the native TUI; use /ps clean to stop all background terminals for this thread",
                )?;
            }
            true
        }
        "fast"
        | "agent"
        | "multi-agents"
        | "theme"
        | "rollout"
        | "sandbox-add-read-dir"
        | "setup-default-sandbox"
        | "init" => {
            output.line_stderr(format!(
                "[session] /{command} is recognized, but this inline client does not yet implement the native Codex popup/workflow for it"
            ))?;
            true
        }
        _ => return Ok(None),
    };

    Ok(Some(result))
}
