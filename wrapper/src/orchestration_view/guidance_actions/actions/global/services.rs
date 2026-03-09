use super::super::super::*;

pub(in super::super::super) fn action_lines(
    state: &AppState,
    audience: ActionAudience,
) -> Vec<String> {
    let sidecar_agents = active_sidecar_agent_task_count(state);
    let shell_sidecars = running_shell_count_by_intent(state, BackgroundShellIntent::Observation);
    let capability_conflicts = state
        .orchestration
        .background_shells
        .service_capability_conflicts();
    let ready_services =
        running_service_count_by_readiness(state, BackgroundShellServiceReadiness::Ready);
    let booting_services =
        running_service_count_by_readiness(state, BackgroundShellServiceReadiness::Booting);
    let untracked_services =
        running_service_count_by_readiness(state, BackgroundShellServiceReadiness::Untracked);
    let terminals = server_background_terminal_count(state);

    if let Some((capability, _)) = capability_conflicts.first() {
        let provider_ref = first_provider_ref_for_capability(state, capability)
            .unwrap_or_else(|| "<jobId|alias|n>".to_string());
        return match audience {
            ActionAudience::Operator => vec![
                format!("Run `:ps capabilities @{capability}` to inspect providers and consumers."),
                format!(
                    "Run `:ps provide {provider_ref} <@other.role|none>` to remove or replace @{capability} on one running provider before falling back to cleanup."
                ),
                format!(
                    "Run `:clean services @{capability}` to clear the ambiguous reusable role."
                ),
                format!("Run `:ps services @{capability}` to verify the surviving providers."),
            ],
            ActionAudience::Tool => vec![
                format!(
                    "Use `background_shell_inspect_capability {{\"capability\":\"@{capability}\"}}` to inspect providers and consumers."
                ),
                format!(
                    "Use `background_shell_update_service {{\"jobId\":\"{provider_ref}\",\"capabilities\":[\"@other.role\"]}}` or `background_shell_update_service {{\"jobId\":\"{provider_ref}\",\"capabilities\":null}}` to remove or replace the conflicting reusable role before falling back to cleanup."
                ),
                format!(
                    "Use `background_shell_clean {{\"scope\":\"services\",\"capability\":\"@{capability}\"}}` to clear the ambiguous reusable role."
                ),
                format!(
                    "Use `background_shell_list_services {{\"capability\":\"@{capability}\"}}` to verify the surviving providers."
                ),
            ],
        };
    }
    if ready_services > 0 {
        let provider_ref =
            unique_service_ref_by_readiness(state, BackgroundShellServiceReadiness::Ready);
        let recipe =
            unique_service_recipe_name_by_readiness(state, BackgroundShellServiceReadiness::Ready);
        return match audience {
            ActionAudience::Operator => vec![
                "Run `:ps services ready` to inspect reusable service metadata.".to_string(),
                match provider_ref.as_deref() {
                    Some(job_ref) => format!(
                        "Run `:ps attach {job_ref}` to inspect endpoint and recipe details."
                    ),
                    None => "Run `:ps attach <jobId|alias|@capability|n>` to inspect endpoint and recipe details."
                        .to_string(),
                },
                match provider_ref.as_deref() {
                    Some(job_ref) => match recipe.as_ref() {
                        Some(recipe) => format!(
                            "Run `:ps attach {job_ref}` or `{}` to reuse the ready service directly.",
                            operator_recipe_command(job_ref, recipe)
                        ),
                        None => format!(
                            "Run `:ps attach {job_ref}` to inspect endpoint and recipe details for the ready service."
                        ),
                    },
                    None => "Run `:ps run <jobId|alias|@capability|n> <recipe> [json-args]` to reuse the ready service directly."
                        .to_string(),
                },
            ],
            ActionAudience::Tool => vec![
                "Use `background_shell_list_services {\"status\":\"ready\"}` to inspect reusable service metadata.".to_string(),
                match provider_ref.as_deref() {
                    Some(job_ref) => format!(
                        "Use `background_shell_attach {{\"jobId\":\"{job_ref}\"}}` to inspect endpoint and recipe details for the ready service."
                    ),
                    None => "Use `background_shell_attach {\"jobId\":\"<jobId|alias|@capability>\"}` to inspect endpoint and recipe details for the service you choose.".to_string(),
                },
                match provider_ref.as_deref() {
                    Some(job_ref) => match recipe.as_ref() {
                        Some(recipe) => format!(
                            "Use `background_shell_attach {{\"jobId\":\"{job_ref}\"}}` or `{}` to reuse the ready service directly.",
                            tool_recipe_call(job_ref, recipe)
                        ),
                        None => format!(
                            "Use `background_shell_attach {{\"jobId\":\"{job_ref}\"}}` to inspect endpoint and recipe details for the ready service."
                        ),
                    },
                    None => "Use `background_shell_invoke_recipe {\"jobId\":\"<jobId|alias|@capability>\",\"recipe\":\"...\"}` to reuse the ready service directly.".to_string(),
                },
            ],
        };
    }
    if booting_services > 0 {
        let provider_ref =
            unique_service_ref_by_readiness(state, BackgroundShellServiceReadiness::Booting);
        return match audience {
            ActionAudience::Operator => vec![
                "Run `:ps services booting` to inspect readiness state and startup metadata."
                    .to_string(),
                match provider_ref.as_deref() {
                    Some(job_ref) => format!(
                        "Run `:ps wait {job_ref} [timeoutMs]` for the booting service you need."
                    ),
                    None => "Run `:ps wait <jobId|alias|@capability|n> [timeoutMs]` for the booting service you need."
                        .to_string(),
                },
                "Run `:ps capabilities booting` to keep the capability view focused.".to_string(),
            ],
            ActionAudience::Tool => vec![
                "Use `background_shell_list_services {\"status\":\"booting\"}` to inspect readiness state and startup metadata.".to_string(),
                match provider_ref.as_deref() {
                    Some(job_ref) => format!(
                        "Use `background_shell_wait_ready {{\"jobId\":\"{job_ref}\",\"timeoutMs\":5000}}` for the booting service you need."
                    ),
                    None => "Use `background_shell_wait_ready {\"jobId\":\"<jobId|alias|@capability>\",\"timeoutMs\":5000}` for the booting service you need.".to_string(),
                },
                "Use `background_shell_list_capabilities {\"status\":\"booting\"}` to keep the capability view focused.".to_string(),
            ],
        };
    }
    if untracked_services > 0 {
        let provider_ref =
            unique_service_ref_by_readiness(state, BackgroundShellServiceReadiness::Untracked);
        return match audience {
            ActionAudience::Operator => vec![
                "Run `:ps services untracked` to inspect reusable services that still lack readiness or attachment contract metadata."
                    .to_string(),
                match provider_ref.as_deref() {
                    Some(job_ref) => format!(
                        "Run `:ps contract {job_ref} <json-object>` to add fields such as `readyPattern`, `protocol`, `endpoint`, `attachHint`, or `recipes`."
                    ),
                    None => "Run `:ps contract <jobId|alias|@capability|n> <json-object>` to add fields such as `readyPattern`, `protocol`, `endpoint`, `attachHint`, or `recipes`."
                        .to_string(),
                },
                match provider_ref.as_deref() {
                    Some(job_ref) => format!(
                        "Run `:ps relabel {job_ref} <label|none>` if the service also needs a clearer operator-facing identity."
                    ),
                    None => "Run `:ps relabel <jobId|alias|@capability|n> <label|none>` if the service also needs a clearer operator-facing identity."
                        .to_string(),
                },
            ],
            ActionAudience::Tool => vec![
                "Use `background_shell_list_services {\"status\":\"untracked\"}` to inspect reusable services that still lack readiness or attachment contract metadata."
                    .to_string(),
                match provider_ref.as_deref() {
                    Some(job_ref) => format!(
                        "Use `background_shell_update_service {{\"jobId\":\"{job_ref}\",\"readyPattern\":\"READY\",\"protocol\":\"http\",\"endpoint\":\"http://127.0.0.1:3000\"}}` to add reusable contract metadata in place."
                    ),
                    None => "Use `background_shell_update_service {\"jobId\":\"<jobId|alias|@capability>\",\"readyPattern\":\"READY\",\"protocol\":\"http\",\"endpoint\":\"http://127.0.0.1:3000\"}` to add reusable contract metadata in place."
                        .to_string(),
                },
                match provider_ref.as_deref() {
                    Some(job_ref) => format!(
                        "Use `background_shell_update_service {{\"jobId\":\"{job_ref}\",\"label\":\"service-label\"}}` if the service also needs a clearer operator-facing identity."
                    ),
                    None => "Use `background_shell_update_service {\"jobId\":\"<jobId|alias|@capability>\",\"label\":\"service-label\"}` if the service also needs a clearer operator-facing identity."
                        .to_string(),
                },
            ],
        };
    }
    if sidecar_agents + shell_sidecars > 0 {
        return match audience {
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
        };
    }
    if terminals > 0 {
        return match audience {
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
        };
    }

    Vec::new()
}
