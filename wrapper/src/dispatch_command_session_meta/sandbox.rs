use anyhow::Result;
use std::path::Path;
use std::process::ChildStdin;

use crate::Cli;
use crate::config_persistence::resolve_codex_home;
use crate::output::Output;
use crate::policy::thread_sandbox_mode;
use crate::requests::send_windows_sandbox_setup_start;
use crate::state::AppState;
use crate::state::summarize_text;
use crate::windows_sandbox_read_grants::WINDOWS_SANDBOX_READ_ROOT_USAGE;
use crate::windows_sandbox_read_grants::grant_read_root_non_elevated;
use crate::windows_sandbox_read_grants::parse_windows_sandbox_read_root_arg;

pub(crate) fn handle_setup_default_sandbox_command(
    args: &[&str],
    resolved_cwd: &str,
    state: &mut AppState,
    output: &mut Output,
    writer: &mut ChildStdin,
) -> Result<bool> {
    if !args.is_empty() {
        output.line_stderr("[session] usage: :setup-default-sandbox")?;
        return Ok(true);
    }
    if !cfg!(target_os = "windows") {
        output.line_stderr("[session] /setup-default-sandbox is only available on Windows")?;
        return Ok(true);
    }
    output.line_stderr("[session] starting Windows sandbox setup (elevated)")?;
    send_windows_sandbox_setup_start(writer, state, resolved_cwd, "elevated")?;
    Ok(true)
}

pub(crate) fn handle_sandbox_add_read_dir_command(
    raw_args: &str,
    cli: &Cli,
    resolved_cwd: &str,
    state: &mut AppState,
    output: &mut Output,
) -> Result<bool> {
    let Some(read_root) = parse_windows_sandbox_read_root_arg(raw_args) else {
        output.line_stderr(WINDOWS_SANDBOX_READ_ROOT_USAGE)?;
        return Ok(true);
    };
    if !cfg!(target_os = "windows") {
        output.line_stderr("[session] /sandbox-add-read-dir is only available on Windows")?;
        return Ok(true);
    }
    let codex_home = resolve_codex_home(state.codex_home_override.as_deref())?;
    output.line_stderr(format!(
        "[session] granting sandbox read access to {} ...",
        summarize_text(&read_root)
    ))?;
    match grant_read_root_non_elevated(
        &thread_sandbox_mode(cli, state),
        Path::new(resolved_cwd),
        codex_home.as_path(),
        Path::new(&read_root),
    ) {
        Ok(path) => output.line_stderr(format!(
            "[session] sandbox read access granted for {}",
            path.display()
        ))?,
        Err(err) => output.line_stderr(format!(
            "[session] failed to grant sandbox read access: {err}"
        ))?,
    }
    Ok(true)
}
