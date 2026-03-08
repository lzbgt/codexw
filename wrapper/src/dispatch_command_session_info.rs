use std::process::ChildStdin;

use anyhow::Result;

use crate::Cli;
use crate::catalog_views::render_apps_list;
use crate::catalog_views::render_skills_list;
use crate::editor::LineEditor;
use crate::model_session::ModelsAction;
use crate::model_session::apply_personality_selection;
use crate::model_session::render_personality_options;
use crate::output::Output;
use crate::requests::send_load_config;
use crate::requests::send_load_experimental_features;
use crate::requests::send_load_mcp_servers;
use crate::requests::send_load_models;
use crate::session_realtime::render_realtime_status;
use crate::session_snapshot::render_status_snapshot;
use crate::state::AppState;
use crate::status_views::render_permissions_snapshot;
use crate::transcript_render::render_pending_attachments;

pub(crate) fn try_handle_session_info_command(
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
        "realtime" if args.is_empty() || matches!(args[0], "status" | "show") => {
            output.block_stdout("Realtime", &render_realtime_status(state))?;
            true
        }
        _ => return Ok(None),
    };

    Ok(Some(result))
}
