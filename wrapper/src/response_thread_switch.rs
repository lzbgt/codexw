use std::process::ChildStdin;

use anyhow::Result;
use serde_json::Value;

use crate::Cli;
use crate::output::Output;
use crate::response_thread_loaded::handle_loaded_thread;
use crate::state::AppState;

pub(crate) fn handle_started_thread(
    result: &Value,
    cli: &Cli,
    resolved_cwd: &str,
    state: &mut AppState,
    output: &mut Output,
    writer: &mut ChildStdin,
    initial_prompt: Option<&str>,
) -> Result<()> {
    handle_loaded_thread(
        result,
        cli,
        resolved_cwd,
        state,
        output,
        writer,
        initial_prompt,
        "started",
        "thread/start missing thread.id",
        false,
    )
}

pub(crate) fn handle_resumed_thread(
    result: &Value,
    cli: &Cli,
    resolved_cwd: &str,
    state: &mut AppState,
    output: &mut Output,
    writer: &mut ChildStdin,
    initial_prompt: Option<&str>,
) -> Result<()> {
    handle_loaded_thread(
        result,
        cli,
        resolved_cwd,
        state,
        output,
        writer,
        initial_prompt,
        "resumed",
        "thread/resume missing thread.id",
        true,
    )
}

pub(crate) fn handle_forked_thread(
    result: &Value,
    cli: &Cli,
    resolved_cwd: &str,
    state: &mut AppState,
    output: &mut Output,
    writer: &mut ChildStdin,
    initial_prompt: Option<&str>,
) -> Result<()> {
    handle_loaded_thread(
        result,
        cli,
        resolved_cwd,
        state,
        output,
        writer,
        initial_prompt,
        "forked to",
        "thread/fork missing thread.id",
        true,
    )
}
