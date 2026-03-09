use anyhow::Result;

use crate::output::Output;
use crate::state::AppState;

use super::super::super::parse_optional_contract_field;
use super::super::super::parse_optional_contract_recipes;
use super::super::super::parse_ps_contract_args;
use super::super::super::parse_ps_provide_capabilities;
use super::super::super::parse_ps_relabel_args;

pub(super) fn handle_ps_service_metadata_action(
    raw_args: &str,
    args: &[&str],
    state: &mut AppState,
    output: &mut Output,
) -> Result<bool> {
    let action = args.first().copied();
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
    Ok(false)
}
