use std::process::ChildStdin;

use anyhow::Result;

use crate::catalog_app_views::render_apps_list;
use crate::catalog_app_views::render_skills_list;
use crate::output::Output;
use crate::requests::send_load_experimental_features;
use crate::requests::send_load_mcp_servers;
use crate::state::AppState;

pub(crate) fn try_handle_session_catalog_list_command(
    command: &str,
    state: &mut AppState,
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
        "mcp" => {
            output.line_stderr("[session] loading MCP server status")?;
            send_load_mcp_servers(writer, state)?;
            true
        }
        "experimental" => {
            output.line_stderr("[session] loading experimental feature flags")?;
            send_load_experimental_features(writer, state)?;
            true
        }
        _ => return Ok(None),
    };

    Ok(Some(result))
}
