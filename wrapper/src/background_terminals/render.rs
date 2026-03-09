use crate::state::AppState;

pub(crate) fn server_background_terminal_count(state: &AppState) -> usize {
    state.orchestration.background_terminals.len()
}

pub(crate) fn background_terminal_count(state: &AppState) -> usize {
    server_background_terminal_count(state) + state.orchestration.background_shells.running_count()
}

pub(crate) fn render_background_terminals(state: &AppState) -> String {
    let mut processes = state
        .orchestration
        .background_terminals
        .values()
        .cloned()
        .collect::<Vec<_>>();
    processes.sort_by(|left, right| {
        left.command_display
            .cmp(&right.command_display)
            .then_with(|| left.process_id.cmp(&right.process_id))
    });
    let mut lines = Vec::new();
    if !processes.is_empty() {
        lines.push("Server-observed background terminals:".to_string());
        for (index, process) in processes.iter().enumerate() {
            lines.push(format!(
                "{:>2}. {}  [{}]",
                index + 1,
                process.command_display,
                if process.waiting {
                    "waiting"
                } else {
                    "interactive"
                }
            ));
            lines.push(format!("    process  {}", process.process_id));
            if !process.recent_inputs.is_empty() {
                lines.push(format!(
                    "    recent   {}",
                    process.recent_inputs.join(" | ")
                ));
            }
            if !process.recent_output.is_empty() {
                lines.push(format!(
                    "    output   {}",
                    process.recent_output.join(" | ")
                ));
            }
        }
    }
    if let Some(local_jobs) = state.orchestration.background_shells.render_for_ps() {
        if !lines.is_empty() {
            lines.push(String::new());
        }
        lines.extend(local_jobs);
    }
    if lines.is_empty() {
        return "No background terminals running.".to_string();
    }
    lines.push("Use :clean to stop all running background tasks.".to_string());
    lines.join("\n")
}
