use super::super::super::super::*;

pub(in super::super::super::super) fn action_lines_for_residuals(
    state: &AppState,
    audience: ActionAudience,
) -> Option<Vec<String>> {
    let sidecar_agents = active_sidecar_agent_task_count(state);
    let shell_sidecars = running_shell_count_by_intent(state, BackgroundShellIntent::Observation);
    if sidecar_agents + shell_sidecars > 0 {
        return Some(match audience {
            ActionAudience::Operator => vec![
                "Run `:ps agents` to inspect sidecar agent progress.".to_string(),
                "Run `:ps shells` to inspect non-blocking shell jobs.".to_string(),
                "Continue foreground work until one of those results becomes relevant."
                    .to_string(),
            ],
            ActionAudience::Tool => vec![
                "Use `orchestration_list_workers {\"filter\":\"agents\"}` to inspect sidecar agent progress.".to_string(),
                "Use `orchestration_list_workers {\"filter\":\"shells\"}` to inspect non-blocking shell jobs.".to_string(),
                "Continue foreground work until one of those results becomes relevant."
                    .to_string(),
            ],
        });
    }

    let terminals = server_background_terminal_count(state);
    if terminals > 0 {
        return Some(match audience {
            ActionAudience::Operator => vec![
                "Run `:ps terminals` to inspect server-observed background terminals."
                    .to_string(),
                "Run `:clean terminals` to close them if they are no longer needed."
                    .to_string(),
            ],
            ActionAudience::Tool => vec![
                "Use `orchestration_list_workers {\"filter\":\"terminals\"}` to inspect server-observed background terminals.".to_string(),
                "Terminal cleanup is operator-only; use `:clean terminals` from the wrapper when they are no longer needed.".to_string(),
            ],
        });
    }

    None
}
