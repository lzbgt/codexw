use std::ffi::OsStr;
use std::path::PathBuf;
use std::process::ChildStdin;
use std::process::Command;
use std::process::Stdio;

use anyhow::Context;
use anyhow::Result;
use serde_json::json;

use crate::Cli;
use crate::commands::builtin_command_names;
use crate::commands::builtin_help_lines;
use crate::commands::longest_common_prefix;
use crate::commands::quote_if_needed;
use crate::commands::try_complete_slash_command;
use crate::editor::LineEditor;
use crate::input::build_turn_input;
use crate::output::Output;
use crate::requests::send_clean_background_terminals;
use crate::requests::send_command_exec;
use crate::requests::send_command_exec_terminate;
use crate::requests::send_feedback_upload;
use crate::requests::send_fuzzy_file_search;
use crate::requests::send_list_threads;
use crate::requests::send_load_collaboration_modes;
use crate::requests::send_load_config;
use crate::requests::send_load_experimental_features;
use crate::requests::send_load_mcp_servers;
use crate::requests::send_load_models;
use crate::requests::send_logout_account;
use crate::requests::send_start_review;
use crate::requests::send_thread_compact;
use crate::requests::send_thread_fork;
use crate::requests::send_thread_realtime_append_text;
use crate::requests::send_thread_realtime_start;
use crate::requests::send_thread_realtime_stop;
use crate::requests::send_thread_rename;
use crate::requests::send_thread_resume;
use crate::requests::send_thread_start;
use crate::requests::send_turn_interrupt;
use crate::requests::send_turn_start;
use crate::requests::send_turn_steer;
use crate::session::CollaborationModeAction;
use crate::session::ModelsAction;
use crate::session::apply_personality_selection;
use crate::session::render_personality_options;
use crate::session::render_prompt_status;
use crate::session::render_realtime_status;
use crate::session::render_status_snapshot;
use crate::state::AppState;
use crate::state::canonicalize_or_keep;
use crate::state::emit_status_line;
use crate::state::summarize_text;
use crate::state::thread_id;
use crate::views::render_apps_list;
use crate::views::render_pending_attachments;
use crate::views::render_permissions_snapshot;
use crate::views::render_skills_list;

pub(crate) fn handle_user_input(
    line: String,
    cli: &Cli,
    resolved_cwd: &str,
    state: &mut AppState,
    editor: &mut LineEditor,
    output: &mut Output,
    writer: &mut ChildStdin,
) -> Result<bool> {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return Ok(true);
    }

    if let Some(command) = trimmed.strip_prefix(':') {
        return handle_command(command, cli, resolved_cwd, state, editor, output, writer);
    }
    if let Some(command) = trimmed.strip_prefix('/') {
        if is_builtin_command(command) {
            return handle_command(command, cli, resolved_cwd, state, editor, output, writer);
        }
    }

    if let Some(command) = trimmed.strip_prefix('!') {
        if state.turn_running {
            output.line_stderr(
                "[session] wait for the active turn to finish before running a local command",
            )?;
            return Ok(true);
        }
        if state.active_exec_process_id.is_some() {
            output.line_stderr("[session] a local command is already running")?;
            return Ok(true);
        }
        let command = command.trim();
        if command.is_empty() {
            output.line_stderr("[session] usage: !<shell command>")?;
            return Ok(true);
        }
        emit_status_line(
            output,
            state,
            format!("running local command: {}", summarize_text(command)),
        )?;
        send_command_exec(writer, state, cli, resolved_cwd, command.to_string())?;
        return Ok(true);
    }

    let (local_images, remote_images) = state.take_pending_attachments();
    let submission = build_turn_input(
        trimmed,
        resolved_cwd,
        &local_images,
        &remote_images,
        &state.apps,
        &state.plugins,
        &state.skills,
    );
    if submission.items.is_empty() {
        output.line_stderr("[session] nothing to submit")?;
        return Ok(true);
    }

    let thread_id = thread_id(state)?.to_string();
    if state.turn_running {
        let turn_id = state
            .active_turn_id
            .clone()
            .context("turn is marked running but active turn id is missing")?;
        send_turn_steer(writer, state, thread_id, turn_id, submission)?;
    } else {
        send_turn_start(
            writer,
            state,
            cli,
            resolved_cwd,
            thread_id,
            submission,
            false,
        )?;
    }
    Ok(true)
}

pub(crate) fn handle_command(
    command_line: &str,
    cli: &Cli,
    resolved_cwd: &str,
    state: &mut AppState,
    editor: &mut LineEditor,
    output: &mut Output,
    writer: &mut ChildStdin,
) -> Result<bool> {
    let mut parts = command_line.split_whitespace();
    let Some(command) = parts.next() else {
        output.line_stderr("[session] empty command")?;
        return Ok(true);
    };

    match command {
        "help" | "h" => {
            for line in builtin_help_lines() {
                output.line_stderr(line)?;
            }
            output.line_stderr(
                "!<command>           run a local shell command via app-server command/exec",
            )?;
            Ok(true)
        }
        "quit" | "q" | "exit" => Ok(false),
        "new" => {
            if state.turn_running {
                output.line_stderr(
                    "[session] wait for the current turn to finish or interrupt it first",
                )?;
                return Ok(true);
            }
            output.line_stderr("[session] creating new thread")?;
            send_thread_start(writer, state, cli, resolved_cwd, None)?;
            Ok(true)
        }
        "resume" => {
            if state.turn_running {
                output.line_stderr(
                    "[session] wait for the current turn to finish or interrupt it first",
                )?;
                return Ok(true);
            }
            let maybe_arg = parts.next().map(ToOwned::to_owned);
            let Some(thread_id) = maybe_arg else {
                output.line_stderr(
                    "[session] loading recent threads; use /resume <n> or /resume <thread-id>",
                )?;
                send_list_threads(writer, state, resolved_cwd, None)?;
                return Ok(true);
            };
            let thread_id = if let Ok(index) = thread_id.parse::<usize>() {
                match state.last_listed_thread_ids.get(index.saturating_sub(1)) {
                    Some(thread_id) => thread_id.clone(),
                    None => {
                        output.line_stderr("[session] no cached thread at that index; run /threads or /resume first")?;
                        return Ok(true);
                    }
                }
            } else {
                thread_id
            };
            output.line_stderr(format!("[session] resuming thread {thread_id}"))?;
            send_thread_resume(
                writer,
                state,
                cli,
                resolved_cwd,
                thread_id.to_string(),
                None,
            )?;
            Ok(true)
        }
        "fork" => {
            if state.turn_running {
                output.line_stderr(
                    "[session] wait for the current turn to finish or interrupt it first",
                )?;
                return Ok(true);
            }
            let current_thread_id = thread_id(state)?.to_string();
            let initial_prompt = join_prompt(&parts.map(str::to_string).collect::<Vec<_>>());
            output.line_stderr(format!("[thread] forking {current_thread_id}"))?;
            send_thread_fork(
                writer,
                state,
                cli,
                resolved_cwd,
                current_thread_id,
                initial_prompt,
            )?;
            Ok(true)
        }
        "compact" => {
            if state.turn_running {
                output.line_stderr(
                    "[session] wait for the current turn to finish or interrupt it first",
                )?;
                return Ok(true);
            }
            let current_thread_id = thread_id(state)?.to_string();
            output.line_stderr("[thread] requesting compaction")?;
            send_thread_compact(writer, state, current_thread_id)?;
            Ok(true)
        }
        "review" => {
            if state.turn_running {
                output.line_stderr(
                    "[session] wait for the current turn to finish or interrupt it first",
                )?;
                return Ok(true);
            }
            let current_thread_id = thread_id(state)?.to_string();
            let args = parts.collect::<Vec<_>>().join(" ");
            let trimmed_args = args.trim();
            let (target, description) = if trimmed_args.is_empty() {
                (
                    json!({"type": "uncommittedChanges"}),
                    "current uncommitted changes".to_string(),
                )
            } else {
                (
                    json!({"type": "custom", "instructions": trimmed_args}),
                    trimmed_args.to_string(),
                )
            };
            output.line_stderr(format!(
                "[review] requesting {}",
                summarize_text(&description)
            ))?;
            send_start_review(writer, state, current_thread_id, target, description)?;
            Ok(true)
        }
        "apps" => {
            output.block_stdout("Apps", &render_apps_list(&state.apps))?;
            Ok(true)
        }
        "skills" => {
            output.block_stdout("Skills", &render_skills_list(&state.skills))?;
            Ok(true)
        }
        "models" | "model" => {
            output.line_stderr("[session] loading models")?;
            send_load_models(writer, state, ModelsAction::ShowModels)?;
            Ok(true)
        }
        "mcp" => {
            output.line_stderr("[session] loading MCP server status")?;
            send_load_mcp_servers(writer, state)?;
            Ok(true)
        }
        "clean" => {
            if cli.no_experimental_api {
                output.line_stderr(
                    "[thread] background terminal cleanup requires experimental API support; restart without --no-experimental-api",
                )?;
                return Ok(true);
            }
            let current_thread_id = thread_id(state)?.to_string();
            output.line_stderr("[thread] cleaning background terminals")?;
            send_clean_background_terminals(writer, state, current_thread_id)?;
            Ok(true)
        }
        "threads" => {
            let search_term = parts.collect::<Vec<_>>().join(" ");
            let search_term = search_term.trim();
            let search_term = if search_term.is_empty() {
                None
            } else {
                Some(search_term.to_string())
            };
            output.line_stderr("[session] loading recent threads")?;
            send_list_threads(writer, state, resolved_cwd, search_term)?;
            Ok(true)
        }
        "mention" => {
            let query = parts.collect::<Vec<_>>().join(" ");
            let query = query.trim();
            if query.is_empty() {
                editor.insert_str("@");
                return Ok(true);
            }
            if let Ok(index) = query.parse::<usize>() {
                let Some(path) = state
                    .last_file_search_paths
                    .get(index.saturating_sub(1))
                    .cloned()
                else {
                    output.line_stderr(
                        "[session] no cached file match at that index; run /mention <query> first",
                    )?;
                    return Ok(true);
                };
                let inserted = quote_if_needed(&path);
                editor.insert_str(&format!("{inserted} "));
                output.line_stderr(format!("[mention] inserted {}", summarize_text(&path)))?;
                return Ok(true);
            }
            output.line_stderr(format!("[search] files matching {}", summarize_text(query)))?;
            send_fuzzy_file_search(writer, state, resolved_cwd, query.to_string())?;
            Ok(true)
        }
        "diff" => {
            if let Some(diff) = state.last_turn_diff.as_deref() {
                output.block_stdout("Latest diff", diff)?;
            } else {
                output.line_stderr("[diff] no turn diff has been emitted yet")?;
            }
            Ok(true)
        }
        "clear" => {
            if state.turn_running {
                output.line_stderr(
                    "[session] wait for the current turn to finish or interrupt it first",
                )?;
                return Ok(true);
            }
            output.clear_screen()?;
            output.line_stderr("[thread] creating new thread after clear")?;
            send_thread_start(writer, state, cli, resolved_cwd, None)?;
            Ok(true)
        }
        "copy" => {
            if let Some(message) = state.last_agent_message.as_deref() {
                copy_to_clipboard(message, output)?;
            } else {
                output.line_stderr("[copy] no assistant reply is available yet")?;
            }
            Ok(true)
        }
        "auto" => {
            let Some(mode) = parts.next() else {
                output.line_stderr("[session] usage: :auto on|off")?;
                return Ok(true);
            };
            state.auto_continue = match mode {
                "on" => true,
                "off" => false,
                _ => {
                    output.line_stderr("[session] usage: :auto on|off")?;
                    return Ok(true);
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
            Ok(true)
        }
        "attach-image" | "attach" => {
            let Some(path) = parts.next() else {
                output.line_stderr("[session] usage: :attach-image <path>")?;
                return Ok(true);
            };
            let path = canonicalize_or_keep(path);
            state.pending_local_images.push(path.clone());
            output.line_stderr(format!("[draft] queued local image {path}"))?;
            Ok(true)
        }
        "attach-url" => {
            let Some(url) = parts.next() else {
                output.line_stderr("[session] usage: :attach-url <url>")?;
                return Ok(true);
            };
            state.pending_remote_images.push(url.to_string());
            output.line_stderr(format!("[draft] queued remote image {url}"))?;
            Ok(true)
        }
        "attachments" => {
            if state.pending_local_images.is_empty() && state.pending_remote_images.is_empty() {
                output.line_stderr("[draft] no queued attachments")?;
                return Ok(true);
            }
            let rendered = render_pending_attachments(
                &state.pending_local_images,
                &state.pending_remote_images,
            );
            output.block_stdout("Queued attachments", &rendered)?;
            Ok(true)
        }
        "clear-attachments" => {
            state.pending_local_images.clear();
            state.pending_remote_images.clear();
            output.line_stderr("[draft] cleared queued attachments")?;
            Ok(true)
        }
        "interrupt" => {
            if let Some(turn_id) = state.active_turn_id.clone() {
                output.line_stderr("[interrupt] interrupting active turn")?;
                send_turn_interrupt(writer, state, thread_id(state)?.to_string(), turn_id)?;
            } else if let Some(process_id) = state.active_exec_process_id.clone() {
                output.line_stderr("[interrupt] terminating active local command")?;
                send_command_exec_terminate(writer, state, process_id)?;
            } else {
                output.line_stderr("[interrupt] no active turn")?;
            }
            Ok(true)
        }
        "rename" => {
            let name = parts.collect::<Vec<_>>().join(" ").trim().to_string();
            if name.is_empty() {
                output.line_stderr("[session] usage: :rename <name>")?;
                return Ok(true);
            }
            let current_thread_id = thread_id(state)?.to_string();
            send_thread_rename(writer, state, current_thread_id, name)?;
            Ok(true)
        }
        "approvals" | "permissions" => {
            output.block_stdout("Permissions", &render_permissions_snapshot(cli))?;
            Ok(true)
        }
        "status" => {
            output.block_stdout("Status", &render_status_snapshot(cli, resolved_cwd, state))?;
            Ok(true)
        }
        "statusline" => {
            output.block_stdout("Status", &render_status_snapshot(cli, resolved_cwd, state))?;
            Ok(true)
        }
        "settings" => {
            output.line_stderr("[session] loading effective config")?;
            send_load_config(writer, state)?;
            Ok(true)
        }
        "feedback" => {
            let args = parts.map(str::to_string).collect::<Vec<_>>();
            let Some(parsed) = parse_feedback_args(&args) else {
                output.line_stderr(
                    "[session] usage: :feedback <bug|bad_result|good_result|safety_check|other> [reason] [--logs|--no-logs]",
                )?;
                return Ok(true);
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
            Ok(true)
        }
        "logout" => {
            output.line_stderr("[session] logging out")?;
            send_logout_account(writer, state)?;
            Ok(true)
        }
        "debug-config" => {
            output.line_stderr("[session] loading effective config")?;
            send_load_config(writer, state)?;
            Ok(true)
        }
        "experimental" => {
            output.line_stderr("[session] loading experimental feature flags")?;
            send_load_experimental_features(writer, state)?;
            Ok(true)
        }
        "personality" => {
            if state.turn_running {
                output
                    .line_stderr("[session] cannot change personality while a turn is running")?;
                return Ok(true);
            }
            let args = parts.collect::<Vec<_>>();
            if args.is_empty() {
                if state.models.is_empty() {
                    output.line_stderr("[session] loading models for personality options")?;
                    send_load_models(writer, state, ModelsAction::ShowPersonality)?;
                } else {
                    output.block_stdout("Personality", &render_personality_options(cli, state))?;
                }
                return Ok(true);
            }
            let selector = args.join(" ");
            if state.models.is_empty() {
                output.line_stderr("[session] loading models for personality selection")?;
                send_load_models(writer, state, ModelsAction::SetPersonality(selector))?;
            } else {
                apply_personality_selection(cli, state, &selector, output)?;
            }
            Ok(true)
        }
        "collab" => {
            let args = parts.collect::<Vec<_>>();
            if args.is_empty() {
                send_load_collaboration_modes(writer, state, CollaborationModeAction::ShowList)?;
                return Ok(true);
            }
            if state.turn_running {
                output.line_stderr(
                    "[session] cannot switch collaboration mode while a turn is running",
                )?;
                return Ok(true);
            }
            let selector = args.join(" ");
            send_load_collaboration_modes(
                writer,
                state,
                CollaborationModeAction::SetMode(selector),
            )?;
            Ok(true)
        }
        "plan" => {
            if state.turn_running {
                output.line_stderr(
                    "[session] cannot switch collaboration mode while a turn is running",
                )?;
                return Ok(true);
            }
            send_load_collaboration_modes(writer, state, CollaborationModeAction::TogglePlan)?;
            Ok(true)
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
            Ok(true)
        }
        "realtime" => {
            if cli.no_experimental_api {
                output.line_stderr(
                    "[session] /realtime requires experimental API support; restart without --no-experimental-api",
                )?;
                return Ok(true);
            }
            let args = parts.collect::<Vec<_>>();
            let Some(thread_id) = state.thread_id.clone() else {
                output.line_stderr("[session] start or resume a thread before using /realtime")?;
                return Ok(true);
            };
            if args.is_empty() || matches!(args[0], "status" | "show") {
                output.block_stdout("Realtime", &render_realtime_status(state))?;
                return Ok(true);
            }
            match args[0] {
                "start" => {
                    if state.turn_running {
                        output.line_stderr(
                            "[session] cannot start realtime while a turn is running",
                        )?;
                        return Ok(true);
                    }
                    if state.realtime_active {
                        output.line_stderr(
                            "[session] realtime is already active; use /realtime stop first",
                        )?;
                        output.block_stdout("Realtime", &render_realtime_status(state))?;
                        return Ok(true);
                    }
                    let prompt = if args.len() > 1 {
                        args[1..].join(" ")
                    } else {
                        "Text-only experimental realtime session for this thread.".to_string()
                    };
                    send_thread_realtime_start(writer, state, thread_id, prompt)?;
                }
                "send" | "append" => {
                    if !state.realtime_active {
                        output.line_stderr(
                            "[session] realtime is not active; use /realtime start first",
                        )?;
                        return Ok(true);
                    }
                    if args.len() < 2 {
                        output.line_stderr("[session] usage: /realtime send <text>")?;
                        return Ok(true);
                    }
                    send_thread_realtime_append_text(
                        writer,
                        state,
                        thread_id,
                        args[1..].join(" "),
                    )?;
                }
                "stop" => {
                    if !state.realtime_active {
                        output.line_stderr("[session] realtime is not active")?;
                        return Ok(true);
                    }
                    send_thread_realtime_stop(writer, state, thread_id)?;
                }
                other => {
                    output.line_stderr(format!("[session] unknown realtime action: {other}"))?;
                    output.block_stdout("Realtime", &render_realtime_status(state))?;
                }
            }
            Ok(true)
        }
        "ps" => {
            let action = parts.next();
            if matches!(action, Some("clean")) {
                if cli.no_experimental_api {
                    output.line_stderr(
                        "[thread] /ps clean requires experimental API support; restart without --no-experimental-api",
                    )?;
                    return Ok(true);
                }
                let current_thread_id = thread_id(state)?.to_string();
                output.line_stderr("[thread] cleaning background terminals")?;
                send_clean_background_terminals(writer, state, current_thread_id)?;
                return Ok(true);
            }
            output.line_stderr(
                "[session] app-server does not expose background-terminal listing like the native TUI; use /ps clean to stop all background terminals for this thread",
            )?;
            Ok(true)
        }
        _ => {
            output.line_stderr(format!("[session] unknown command: {command}"))?;
            Ok(true)
        }
    }
}

pub(crate) struct FeedbackCommand {
    pub(crate) classification: String,
    pub(crate) reason: Option<String>,
    pub(crate) include_logs: bool,
}

pub(crate) fn parse_feedback_args(args: &[String]) -> Option<FeedbackCommand> {
    if args.is_empty() {
        return None;
    }
    let mut include_logs = false;
    let mut filtered = Vec::new();
    for arg in args {
        match arg.as_str() {
            "--logs" => include_logs = true,
            "--no-logs" => include_logs = false,
            _ => filtered.push(arg.as_str()),
        }
    }
    let Some(first) = filtered.first() else {
        return None;
    };
    let classification = normalize_feedback_classification(first)?;
    let reason = join_prompt(
        &filtered[1..]
            .iter()
            .map(|part| (*part).to_string())
            .collect::<Vec<_>>(),
    );
    Some(FeedbackCommand {
        classification,
        reason,
        include_logs,
    })
}

fn normalize_feedback_classification(raw: &str) -> Option<String> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "bug" => Some("bug".to_string()),
        "bad" | "bad-result" | "bad_result" => Some("bad_result".to_string()),
        "good" | "good-result" | "good_result" => Some("good_result".to_string()),
        "safety" | "safety-check" | "safety_check" => Some("safety_check".to_string()),
        "other" => Some("other".to_string()),
        _ => None,
    }
}

pub(crate) fn update_prompt(
    output: &mut Output,
    state: &AppState,
    editor: &LineEditor,
) -> Result<()> {
    let prompt = prompt_is_visible(state).then(String::new);
    let status = prompt_is_visible(state).then(|| render_prompt_status(state));
    output.set_prompt(prompt);
    output.set_status(status);
    output
        .show_prompt(editor.buffer(), editor.cursor_chars())
        .context("show prompt")
}

pub(crate) fn prompt_is_visible(state: &AppState) -> bool {
    state.thread_id.is_some() && !state.pending_thread_switch
}

pub(crate) fn prompt_accepts_input(state: &AppState) -> bool {
    prompt_is_visible(state) && state.active_exec_process_id.is_none()
}

pub(crate) fn handle_tab_completion(
    editor: &mut LineEditor,
    state: &AppState,
    resolved_cwd: &str,
    output: &mut Output,
) -> Result<()> {
    let buffer = editor.buffer().to_string();
    let cursor_byte = editor.cursor_byte_index();

    if let Some(result) = try_complete_slash_command(editor, &buffer, cursor_byte) {
        if let Some(rendered) = result.rendered_candidates {
            output.block_stdout("Command completions", &rendered)?;
        }
        return Ok(());
    }

    if let Some(result) = try_complete_file_token(editor, &buffer, cursor_byte, resolved_cwd)? {
        if let Some(rendered) = result.rendered_candidates {
            output.block_stdout("File completions", &rendered)?;
        }
        return Ok(());
    }

    if !state.turn_running && !buffer.trim_start().starts_with('!') {
        output.line_stderr("[tab] no completion available")?;
    }
    Ok(())
}

pub(crate) struct FileCompletionResult {
    pub(crate) rendered_candidates: Option<String>,
}

pub(crate) fn try_complete_file_token(
    editor: &mut LineEditor,
    buffer: &str,
    cursor_byte: usize,
    resolved_cwd: &str,
) -> Result<Option<FileCompletionResult>> {
    let Some((start, end, token)) = current_at_token(buffer, cursor_byte) else {
        return Ok(None);
    };
    let completions = file_completions(&token, resolved_cwd)?;
    if completions.is_empty() {
        return Ok(None);
    }

    if completions.len() == 1 {
        editor.replace_range(start, end, &format!("{} ", completions[0]));
        return Ok(Some(FileCompletionResult {
            rendered_candidates: None,
        }));
    }

    let lcp = longest_common_prefix(&completions);
    let inserted_prefix = if lcp.len() > token.len() {
        &lcp
    } else {
        &token
    };
    editor.replace_range(start, end, &format!("@{inserted_prefix}"));
    let rendered_candidates = Some(
        completions
            .iter()
            .take(12)
            .enumerate()
            .map(|(idx, candidate)| format!("{:>2}. {}", idx + 1, candidate))
            .collect::<Vec<_>>()
            .join("\n"),
    );
    Ok(Some(FileCompletionResult {
        rendered_candidates,
    }))
}

fn current_at_token(buffer: &str, cursor_byte: usize) -> Option<(usize, usize, String)> {
    let safe_cursor = clamp_to_char_boundary(buffer, cursor_byte);
    let before_cursor = &buffer[..safe_cursor];
    let after_cursor = &buffer[safe_cursor..];
    let start = before_cursor
        .char_indices()
        .rfind(|(_, ch)| ch.is_whitespace())
        .map(|(idx, ch)| idx + ch.len_utf8())
        .unwrap_or(0);
    let end_rel = after_cursor
        .char_indices()
        .find(|(_, ch)| ch.is_whitespace())
        .map(|(idx, _)| idx)
        .unwrap_or(after_cursor.len());
    let end = safe_cursor + end_rel;
    let token = &buffer[start..end];
    let mention = token.strip_prefix('@')?;
    if mention.is_empty() {
        return Some((start, end, String::new()));
    }
    if mention.starts_with('@') {
        return None;
    }
    if mention
        .chars()
        .any(|ch| ch.is_whitespace() || matches!(ch, '"' | '\'' | '(' | ')' | '[' | ']'))
    {
        return None;
    }
    Some((start, end, mention.to_string()))
}

fn clamp_to_char_boundary(text: &str, cursor_byte: usize) -> usize {
    if cursor_byte >= text.len() {
        return text.len();
    }
    let mut safe = cursor_byte;
    while safe > 0 && !text.is_char_boundary(safe) {
        safe -= 1;
    }
    safe
}

fn file_completions(token: &str, resolved_cwd: &str) -> Result<Vec<String>> {
    let token = token.trim();
    let (dir_part, name_prefix) = match token.rfind(['/', '\\']) {
        Some(idx) => (&token[..=idx], &token[idx + 1..]),
        None => ("", token),
    };
    let base_dir = if dir_part.is_empty() {
        PathBuf::from(resolved_cwd)
    } else {
        PathBuf::from(resolved_cwd).join(dir_part)
    };
    if !base_dir.is_dir() {
        return Ok(Vec::new());
    }

    let mut matches = std::fs::read_dir(&base_dir)
        .with_context(|| format!("read directory {}", base_dir.display()))?
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let name = entry.file_name();
            let name = os_str_to_string(&name)?;
            if !name.starts_with(name_prefix) {
                return None;
            }
            let mut rendered = format!("{dir_part}{name}");
            if entry.path().is_dir() {
                rendered.push('/');
            }
            Some(rendered)
        })
        .collect::<Vec<_>>();
    matches.sort();
    Ok(matches)
}

fn os_str_to_string(value: &OsStr) -> Option<String> {
    value.to_str().map(ToOwned::to_owned)
}

pub(crate) fn join_prompt(parts: &[String]) -> Option<String> {
    let joined = parts.join(" ").trim().to_string();
    if joined.is_empty() {
        None
    } else {
        Some(joined)
    }
}

pub(crate) fn is_builtin_command(command_line: &str) -> bool {
    let command = command_line.split_whitespace().next().unwrap_or_default();
    matches!(command, "h" | "q") || builtin_command_names().contains(&command)
}

fn copy_to_clipboard(text: &str, output: &mut Output) -> Result<()> {
    if cfg!(target_os = "macos") {
        let Ok(mut child) = Command::new("pbcopy")
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
        else {
            output.block_stdout("Copied text", text)?;
            return Ok(());
        };
        if let Some(stdin) = child.stdin.as_mut() {
            use std::io::Write as _;
            stdin.write_all(text.as_bytes())?;
        }
        let _ = child.wait();
        output.line_stderr("[copy] copied last assistant reply to clipboard")?;
    } else {
        output.block_stdout("Copied text", text)?;
    }
    Ok(())
}
