use anyhow::Result;

use crate::output::Output;
use crate::state::AppState;

use super::super::super::parse_ps_capability_list;

pub(super) fn handle_ps_service_dependency_action(
    args: &[&str],
    state: &mut AppState,
    output: &mut Output,
) -> Result<bool> {
    let action = args.first().copied();
    if !matches!(action, Some("depend" | "requires")) {
        return Ok(false);
    }
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
    Ok(true)
}
