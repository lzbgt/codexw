use super::super::super::super::*;

pub(in super::super::super::super) fn guidance_lines_for_residuals(
    state: &AppState,
) -> Option<Vec<String>> {
    let sidecar_agents = active_sidecar_agent_task_count(state);
    let shell_sidecars = running_shell_count_by_intent(state, BackgroundShellIntent::Observation);
    if sidecar_agents + shell_sidecars > 0 {
        let sidecars = sidecar_agents + shell_sidecars;
        return Some(vec![
            format!(
                "{} running without blocking the main agent.",
                pluralize(sidecars, "sidecar is", "sidecars are")
            ),
            "Continue independent work on the foreground thread.".to_string(),
            "Use :ps agents or :ps shells to inspect progress only when the result becomes relevant.".to_string(),
        ]);
    }

    let terminals = server_background_terminal_count(state);
    if terminals > 0 {
        return Some(vec![
            format!(
                "{} still active.",
                pluralize(terminals, "server terminal is", "server terminals are")
            ),
            "Use :ps terminals to inspect them or :clean terminals to close them.".to_string(),
        ]);
    }

    None
}

pub(in super::super::super::super) fn guidance_lines_for_residuals_tool(
    state: &AppState,
) -> Option<Vec<String>> {
    let sidecar_agents = active_sidecar_agent_task_count(state);
    let shell_sidecars = running_shell_count_by_intent(state, BackgroundShellIntent::Observation);
    if sidecar_agents + shell_sidecars > 0 {
        return Some(vec![
            format!(
                "{} running without blocking the main agent.",
                pluralize(sidecar_agents + shell_sidecars, "sidecar is", "sidecars are")
            ),
            "Continue independent work on the foreground thread.".to_string(),
            "Use `orchestration_list_workers {\"filter\":\"agents\"}` or `orchestration_list_workers {\"filter\":\"shells\"}` to inspect progress only when the result becomes relevant.".to_string(),
        ]);
    }

    let terminals = server_background_terminal_count(state);
    if terminals > 0 {
        return Some(vec![
            format!(
                "{} still active.",
                pluralize(terminals, "server terminal is", "server terminals are")
            ),
            "Use `orchestration_list_workers {\"filter\":\"terminals\"}` to inspect them."
                .to_string(),
        ]);
    }

    None
}
