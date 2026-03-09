use std::process::ChildStdin;

use anyhow::Result;

#[path = "dispatch_command_session_ps/clean.rs"]
mod clean;
#[path = "dispatch_command_session_ps/control.rs"]
mod control;
#[path = "dispatch_command_session_ps/parse.rs"]
mod parse;
#[path = "dispatch_command_session_ps/views.rs"]
mod views;

use crate::Cli;
use crate::output::Output;
use crate::state::AppState;

pub(crate) use self::clean::execute_clean_target;
pub(crate) use self::parse::parse_clean_selection;
#[cfg(test)]
pub(crate) use self::parse::parse_clean_target;
pub(crate) use self::parse::parse_operator_recipe_args;
pub(crate) use self::parse::parse_optional_contract_field;
pub(crate) use self::parse::parse_optional_contract_recipes;
pub(crate) use self::parse::parse_ps_capability_issue_filter;
pub(crate) use self::parse::parse_ps_capability_list;
pub(crate) use self::parse::parse_ps_contract_args;
#[cfg(test)]
pub(crate) use self::parse::parse_ps_dependency_filter;
pub(crate) use self::parse::parse_ps_dependency_selector;
pub(crate) use self::parse::parse_ps_filter;
pub(crate) use self::parse::parse_ps_focus_capability;
pub(crate) use self::parse::parse_ps_provide_capabilities;
pub(crate) use self::parse::parse_ps_relabel_args;
pub(crate) use self::parse::parse_ps_run_args;
pub(crate) use self::parse::parse_ps_send_args;
#[cfg(test)]
pub(crate) use self::parse::parse_ps_service_issue_filter;
pub(crate) use self::parse::parse_ps_service_selector;
pub(crate) use self::parse::parse_ps_wait_timeout;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CleanTarget {
    All,
    Blockers,
    Shells,
    Services,
    Terminals,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CleanSelection {
    pub(crate) target: CleanTarget,
    pub(crate) capability: Option<String>,
}

pub(crate) fn handle_ps_command(
    raw_args: &str,
    args: &[&str],
    cli: &Cli,
    state: &mut AppState,
    output: &mut Output,
    writer: &mut ChildStdin,
) -> Result<bool> {
    let action = args.first().copied();
    if matches!(action, Some("clean")) {
        match parse_clean_selection(&args[1..], ":ps clean") {
            Ok(selection) => {
                execute_clean_target(selection, cli, state, output, writer)?;
            }
            Err(err) => {
                output.line_stderr(format!("[session] {err}"))?;
            }
        }
    } else if control::handle_ps_control_action(raw_args, args, state, output)? {
    } else if views::handle_ps_view_action(args, state, output)? {
    } else {
        output.line_stderr(
            "[session] usage: :ps [guidance [@capability]|actions [@capability]|blockers [@capability]|dependencies [all|blocking|sidecars|missing|booting|ambiguous|satisfied] [@capability]|agents|shells|services [all|ready|booting|untracked|conflicts] [@capability]|capabilities [@capability|healthy|missing|booting|untracked|ambiguous]|terminals|attach|wait|run|poll|send|terminate|alias|unalias|provide <jobId|alias|@capability|n> <@capability...|none>|depend <jobId|alias|@capability|n> <@capability...|none>|contract <jobId|alias|@capability|n> <json-object>|relabel <jobId|alias|@capability|n> <label|none>|clean [blockers [@capability]|shells|services [@capability]|terminals]]",
        )?;
    }
    Ok(true)
}
