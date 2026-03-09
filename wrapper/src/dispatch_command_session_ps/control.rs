use anyhow::Result;

use crate::output::Output;
use crate::state::AppState;

use super::parse_operator_recipe_args;
use super::parse_optional_contract_field;
use super::parse_optional_contract_recipes;
use super::parse_ps_capability_list;
use super::parse_ps_contract_args;
use super::parse_ps_provide_capabilities;
use super::parse_ps_relabel_args;
use super::parse_ps_run_args;
use super::parse_ps_send_args;
use super::parse_ps_wait_timeout;

pub(super) fn handle_ps_control_action(
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
    if matches!(action, Some("provide")) {
        let Some(reference) = args.get(1).copied() else {
            output.line_stderr(
                "[session] usage: :ps provide <jobId|alias|@capability|n> <@capability...|none>",
            )?;
            return Ok(true);
        };
        let capabilities = match parse_ps_provide_capabilities(&args[2..]) {
            Ok(capabilities) => capabilities,
            Err(err) => {
                output.line_stderr(format!("[session] {err}"))?;
                return Ok(true);
            }
        };
        match state
            .orchestration
            .background_shells
            .update_service_capabilities_for_operator(reference, &capabilities)
        {
            Ok(summary) => output.line_stderr(format!("[thread] {summary}"))?,
            Err(err) => output.line_stderr(format!("[session] {err}"))?,
        }
        return Ok(true);
    }
    if matches!(action, Some("depend" | "requires")) {
        let Some(reference) = args.get(1).copied() else {
            output.line_stderr(
                "[session] usage: :ps depend <jobId|alias|@capability|n> <@capability...|none>",
            )?;
            return Ok(true);
        };
        let capabilities = match parse_ps_capability_list(
            &args[2..],
            ":ps depend <jobId|alias|@capability|n> <@capability...|none>",
        ) {
            Ok(capabilities) => capabilities,
            Err(err) => {
                output.line_stderr(format!("[session] {err}"))?;
                return Ok(true);
            }
        };
        match state
            .orchestration
            .background_shells
            .update_dependency_capabilities_for_operator(reference, &capabilities)
        {
            Ok(summary) => output.line_stderr(format!("[thread] {summary}"))?,
            Err(err) => output.line_stderr(format!("[session] {err}"))?,
        }
        return Ok(true);
    }
    if matches!(action, Some("contract")) {
        let Some((reference, contract)) = parse_ps_contract_args(raw_args) else {
            output.line_stderr(
                "[session] usage: :ps contract <jobId|alias|@capability|n> <json-object>",
            )?;
            return Ok(true);
        };
        let protocol = match parse_optional_contract_field(contract.get("protocol"), "protocol") {
            Ok(value) => value,
            Err(err) => {
                output.line_stderr(format!("[session] {err}"))?;
                return Ok(true);
            }
        };
        let endpoint = match parse_optional_contract_field(contract.get("endpoint"), "endpoint") {
            Ok(value) => value,
            Err(err) => {
                output.line_stderr(format!("[session] {err}"))?;
                return Ok(true);
            }
        };
        let attach_hint =
            match parse_optional_contract_field(contract.get("attachHint"), "attachHint") {
                Ok(value) => value,
                Err(err) => {
                    output.line_stderr(format!("[session] {err}"))?;
                    return Ok(true);
                }
            };
        let ready_pattern =
            match parse_optional_contract_field(contract.get("readyPattern"), "readyPattern") {
                Ok(value) => value,
                Err(err) => {
                    output.line_stderr(format!("[session] {err}"))?;
                    return Ok(true);
                }
            };
        let recipes = match parse_optional_contract_recipes(contract.get("recipes")) {
            Ok(value) => value,
            Err(err) => {
                output.line_stderr(format!("[session] {err}"))?;
                return Ok(true);
            }
        };
        if protocol.is_none()
            && endpoint.is_none()
            && attach_hint.is_none()
            && ready_pattern.is_none()
            && recipes.is_none()
        {
            output.line_stderr(
                "[session] :ps contract requires at least one of `protocol`, `endpoint`, `attachHint`, `readyPattern`, or `recipes`",
            )?;
            return Ok(true);
        }
        match state
            .orchestration
            .background_shells
            .update_service_contract_for_operator(
                reference,
                protocol,
                endpoint,
                attach_hint,
                ready_pattern,
                recipes,
            ) {
            Ok(summary) => output.line_stderr(format!("[thread] {summary}"))?,
            Err(err) => output.line_stderr(format!("[session] {err}"))?,
        }
        return Ok(true);
    }
    if matches!(action, Some("relabel")) {
        let Some((reference, label)) = parse_ps_relabel_args(raw_args) else {
            output.line_stderr(
                "[session] usage: :ps relabel <jobId|alias|@capability|n> <label|none>",
            )?;
            return Ok(true);
        };
        match state
            .orchestration
            .background_shells
            .update_service_label_for_operator(reference, label)
        {
            Ok(summary) => output.line_stderr(format!("[thread] {summary}"))?,
            Err(err) => output.line_stderr(format!("[session] {err}"))?,
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
