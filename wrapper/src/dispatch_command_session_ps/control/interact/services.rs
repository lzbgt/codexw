use anyhow::Result;

use crate::output::Output;
use crate::state::AppState;

use super::super::super::parse_operator_recipe_args;
use super::super::super::parse_ps_run_args;
use super::super::super::parse_ps_wait_timeout;

pub(super) fn handle_ps_service_interaction_action(
    raw_args: &str,
    args: &[&str],
    state: &mut AppState,
    output: &mut Output,
) -> Result<bool> {
    let action = args.first().copied();
    if matches!(action, Some("attach")) {
        let Some(reference) = args.get(1).copied() else {
            output.line_stderr("[session] usage: :ps attach <jobId|alias|@capability|n>")?;
            return Ok(true);
        };
        let job_id = match state
            .orchestration
            .background_shells
            .resolve_job_reference(reference)
        {
            Ok(job_id) => job_id,
            Err(err) => {
                output.line_stderr(format!("[session] {err}"))?;
                return Ok(true);
            }
        };
        let rendered = match state
            .orchestration
            .background_shells
            .attach_for_operator(&job_id)
        {
            Ok(rendered) => rendered,
            Err(err) => {
                output.line_stderr(format!("[session] {err}"))?;
                return Ok(true);
            }
        };
        output.block_stdout("Service Attachment", &rendered)?;
        return Ok(true);
    }
    if matches!(action, Some("wait")) {
        let Some(reference) = args.get(1).copied() else {
            output
                .line_stderr("[session] usage: :ps wait <jobId|alias|@capability|n> [timeoutMs]")?;
            return Ok(true);
        };
        let job_id = match state
            .orchestration
            .background_shells
            .resolve_job_reference(reference)
        {
            Ok(job_id) => job_id,
            Err(err) => {
                output.line_stderr(format!("[session] {err}"))?;
                return Ok(true);
            }
        };
        let timeout_ms = match parse_ps_wait_timeout(args.get(2).copied()) {
            Ok(timeout_ms) => timeout_ms,
            Err(err) => {
                output.line_stderr(format!("[session] {err}"))?;
                return Ok(true);
            }
        };
        let rendered = match state
            .orchestration
            .background_shells
            .wait_ready_for_operator(&job_id, timeout_ms)
        {
            Ok(rendered) => rendered,
            Err(err) => {
                output.line_stderr(format!("[session] {err}"))?;
                return Ok(true);
            }
        };
        output.block_stdout("Service Ready", &rendered)?;
        return Ok(true);
    }
    if matches!(action, Some("run" | "invoke")) {
        let Some((reference, recipe, invoke_args)) = parse_ps_run_args(raw_args) else {
            output.line_stderr(
                "[session] usage: :ps run <jobId|alias|@capability|n> <recipe> [json-args]",
            )?;
            return Ok(true);
        };
        let job_id = match state
            .orchestration
            .background_shells
            .resolve_job_reference(reference)
        {
            Ok(job_id) => job_id,
            Err(err) => {
                output.line_stderr(format!("[session] {err}"))?;
                return Ok(true);
            }
        };
        let invoke_args = match parse_operator_recipe_args(invoke_args) {
            Ok(args) => args,
            Err(err) => {
                output.line_stderr(format!("[session] {err}"))?;
                return Ok(true);
            }
        };
        let rendered = match state
            .orchestration
            .background_shells
            .invoke_recipe_for_operator_with_args(&job_id, recipe, &invoke_args)
        {
            Ok(rendered) => rendered,
            Err(err) => {
                output.line_stderr(format!("[session] {err}"))?;
                return Ok(true);
            }
        };
        output.block_stdout("Service Recipe", &rendered)?;
        return Ok(true);
    }
    Ok(false)
}
