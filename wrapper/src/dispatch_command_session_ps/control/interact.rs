use anyhow::Result;

use crate::output::Output;
use crate::state::AppState;

use super::super::parse_operator_recipe_args;
use super::super::parse_ps_run_args;
use super::super::parse_ps_send_args;
use super::super::parse_ps_wait_timeout;

pub(super) fn handle_ps_interaction_action(
    raw_args: &str,
    args: &[&str],
    state: &mut AppState,
    output: &mut Output,
) -> Result<bool> {
    let action = args.first().copied();
    if matches!(action, Some("send" | "write" | "stdin")) {
        let Some((reference, text)) = parse_ps_send_args(raw_args) else {
            output.line_stderr("[session] usage: :ps send <jobId|alias|@capability|n> <text>")?;
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
        match state
            .orchestration
            .background_shells
            .send_input_for_operator(&job_id, text, true)
        {
            Ok(bytes_written) => output.line_stderr(format!(
                "[thread] sent {bytes_written} byte{} to background shell job {job_id}",
                if bytes_written == 1 { "" } else { "s" }
            ))?,
            Err(err) => output.line_stderr(format!("[session] {err}"))?,
        }
        return Ok(true);
    }
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
    if matches!(action, Some("poll" | "show" | "inspect")) {
        let Some(reference) = args.get(1).copied() else {
            output.line_stderr("[session] usage: :ps poll <jobId|alias|@capability|n>")?;
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
            .poll_job(&job_id, 0, 200)
        {
            Ok(rendered) => rendered,
            Err(err) => {
                output.line_stderr(format!("[session] {err}"))?;
                return Ok(true);
            }
        };
        output.block_stdout("Background Shell", &rendered)?;
        return Ok(true);
    }
    if matches!(action, Some("alias")) {
        let Some(reference) = args.get(1).copied() else {
            output.line_stderr("[session] usage: :ps alias <jobId|alias|@capability|n> <name>")?;
            return Ok(true);
        };
        let Some(alias) = args.get(2).copied() else {
            output.line_stderr("[session] usage: :ps alias <jobId|alias|@capability|n> <name>")?;
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
        match state
            .orchestration
            .background_shells
            .set_job_alias(&job_id, alias)
        {
            Ok(()) => output.line_stderr(format!(
                "[thread] background shell job {job_id} aliased as {alias}"
            ))?,
            Err(err) => output.line_stderr(format!("[session] {err}"))?,
        }
        return Ok(true);
    }
    if matches!(action, Some("unalias")) {
        let Some(reference) = args.get(1).copied() else {
            output.line_stderr("[session] usage: :ps unalias <name|jobId|alias|@capability|n>")?;
            return Ok(true);
        };
        match state
            .orchestration
            .background_shells
            .clear_job_alias(reference)
        {
            Ok(job_id) => output.line_stderr(format!(
                "[thread] removed alias {reference} from background shell job {job_id}"
            ))?,
            Err(alias_err) => {
                match state
                    .orchestration
                    .background_shells
                    .resolve_job_reference(reference)
                {
                    Ok(job_id) => {
                        if let Err(err) = state
                            .orchestration
                            .background_shells
                            .clear_job_alias_for_job(&job_id)
                        {
                            output.line_stderr(format!("[session] {err}"))?;
                        } else {
                            output.line_stderr(format!(
                                "[thread] cleared alias for background shell job {job_id}"
                            ))?;
                        }
                    }
                    Err(_) => output.line_stderr(format!("[session] {alias_err}"))?,
                }
            }
        }
        return Ok(true);
    }
    if matches!(action, Some("terminate" | "stop" | "kill")) {
        let Some(reference) = args.get(1).copied() else {
            output.line_stderr("[session] usage: :ps terminate <jobId|alias|@capability|n>")?;
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
        match state
            .orchestration
            .background_shells
            .terminate_job_for_operator(&job_id)
        {
            Ok(()) => output.line_stderr(format!(
                "[thread] terminated local background shell job {job_id}"
            ))?,
            Err(err) => output.line_stderr(format!("[session] {err}"))?,
        }
        return Ok(true);
    }
    Ok(false)
}
