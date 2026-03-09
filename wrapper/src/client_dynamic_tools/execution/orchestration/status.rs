use crate::orchestration_view::orchestration_next_action_summary_for_tool;
use crate::orchestration_view::orchestration_overview_summary;
use crate::orchestration_view::orchestration_runtime_summary;
use crate::state::AppState;

pub(crate) fn render_orchestration_status_for_tool(state: &AppState) -> String {
    let mut lines = vec![format!(
        "orchestration   {}",
        orchestration_overview_summary(state)
    )];
    if let Some(runtime) = orchestration_runtime_summary(state) {
        lines.push(format!("runtime         {runtime}"));
    }
    if let Some(next_action) = orchestration_next_action_summary_for_tool(state) {
        lines.push(format!("next action     {next_action}"));
    }
    lines.join("\n")
}
