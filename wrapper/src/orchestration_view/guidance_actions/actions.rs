use super::*;

pub(super) fn action_lines(state: &AppState, audience: ActionAudience) -> Vec<String> {
    let waits = active_wait_task_count(state);
    let prereqs = running_shell_count_by_intent(state, BackgroundShellIntent::Prerequisite);
    let sidecar_agents = active_sidecar_agent_task_count(state);
    let shell_sidecars = running_shell_count_by_intent(state, BackgroundShellIntent::Observation);
    let blocking_capability_issues = state
        .orchestration
        .background_shells
        .blocking_capability_dependency_issues();
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

    if let Some(issue) = blocking_capability_issues
        .iter()
        .find(|issue| issue.status == BackgroundShellCapabilityDependencyState::Missing)
    {
        let blocker_ref = first_blocking_ref_for_capability(state, &issue.capability)
            .unwrap_or_else(|| "<jobId|alias|n>".to_string());
        let service_ref =
            unique_running_service_ref(state).unwrap_or_else(|| "<jobId|alias|n>".to_string());
        return match audience {
            ActionAudience::Operator => vec![
                format!(
                    "Run `:ps capabilities @{}` to inspect the missing provider map.",
                    issue.capability
                ),
                format!(
                    "Run `:ps provide {service_ref} @{}` to retarget an existing running service, or start a new provider for that role.",
                    issue.capability
                ),
                format!(
                    "Run `:ps depend {blocker_ref} <@capability...|none>` to retarget the blocked shell if it should wait on a different reusable role."
                ),
                format!(
                    "Run `:ps dependencies missing @{}` to inspect the blocked dependency edges.",
                    issue.capability
                ),
                format!(
                    "If the blocked shell is no longer needed, run `:clean blockers @{}`.",
                    issue.capability
                ),
            ],
            ActionAudience::Tool => vec![
                format!(
                    "Use `background_shell_inspect_capability {{\"capability\":\"@{}\"}}` to inspect the missing provider map.",
                    issue.capability
                ),
                format!(
                    "Use `background_shell_update_service {{\"jobId\":\"{service_ref}\",\"capabilities\":[\"@{}\"]}}` to retarget an existing running service, or start a new provider for that capability.",
                    issue.capability
                ),
                format!(
                    "Use `background_shell_update_dependencies {{\"jobId\":\"{blocker_ref}\",\"dependsOnCapabilities\":[\"@other.role\"]}}` to retarget the blocked shell if it should depend on a different reusable role."
                ),
                format!(
                    "Use `orchestration_list_dependencies {{\"filter\":\"missing\",\"capability\":\"@{}\"}}` to inspect the blocked dependency edges.",
                    issue.capability
                ),
                format!(
                    "Use `background_shell_clean {{\"scope\":\"blockers\",\"capability\":\"@{}\"}}` to abandon the blocking prerequisite shells if they are no longer needed.",
                    issue.capability
                ),
            ],
        };
    }
    if let Some(issue) = blocking_capability_issues
        .iter()
        .find(|issue| issue.status == BackgroundShellCapabilityDependencyState::Ambiguous)
    {
        let blocker_ref = first_blocking_ref_for_capability(state, &issue.capability)
            .unwrap_or_else(|| "<jobId|alias|n>".to_string());
        let provider_ref = first_provider_ref_for_capability(state, &issue.capability)
            .unwrap_or_else(|| "<jobId|alias|n>".to_string());
        return match audience {
            ActionAudience::Operator => vec![
                format!(
                    "Run `:ps capabilities @{}` to inspect the ambiguous provider set.",
                    issue.capability
                ),
                format!(
                    "Run `:ps provide {provider_ref} <@other.role|none>` to remove or replace @{} on one running provider before falling back to cleanup.",
                    issue.capability
                ),
                format!(
                    "Run `:ps depend {blocker_ref} <@capability...|none>` if the blocked shell should be retargeted to a different dependency role."
                ),
                format!(
                    "Run `:clean services @{}` to clear the conflicting reusable role in one step.",
                    issue.capability
                ),
                format!(
                    "Run `:ps services @{}` to inspect the remaining providers.",
                    issue.capability
                ),
            ],
            ActionAudience::Tool => vec![
                format!(
                    "Use `background_shell_inspect_capability {{\"capability\":\"@{}\"}}` to inspect the ambiguous provider set.",
                    issue.capability
                ),
                format!(
                    "Use `background_shell_update_service {{\"jobId\":\"{provider_ref}\",\"capabilities\":[\"@other.role\"]}}` or `background_shell_update_service {{\"jobId\":\"{provider_ref}\",\"capabilities\":null}}` to remove or replace the conflicting reusable role before falling back to cleanup."
                ),
                format!(
                    "Use `background_shell_update_dependencies {{\"jobId\":\"{blocker_ref}\",\"dependsOnCapabilities\":[\"@other.role\"]}}` if the blocked shell should be retargeted to a different dependency role."
                ),
                format!(
                    "Use `background_shell_clean {{\"scope\":\"services\",\"capability\":\"@{}\"}}` to clear the conflicting reusable role in one step.",
                    issue.capability
                ),
                format!(
                    "Use `background_shell_list_services {{\"capability\":\"@{}\"}}` to inspect the remaining providers.",
                    issue.capability
                ),
            ],
        };
    }
    if let Some(issue) = blocking_capability_issues
        .iter()
        .find(|issue| issue.status == BackgroundShellCapabilityDependencyState::Booting)
    {
        let provider_ref = first_provider_ref_for_capability(state, &issue.capability)
            .unwrap_or_else(|| format!("@{}", issue.capability));
        return match audience {
            ActionAudience::Operator => vec![
                format!(
                    "Run `:ps services booting @{}` to inspect the booting provider state.",
                    issue.capability
                ),
                format!("Run `:ps wait {provider_ref} 5000` to wait on the capability provider."),
                format!(
                    "Run `:ps dependencies booting @{}` to keep the dependency view focused.",
                    issue.capability
                ),
            ],
            ActionAudience::Tool => vec![
                format!(
                    "Use `background_shell_list_services {{\"status\":\"booting\",\"capability\":\"@{}\"}}` to inspect the booting provider state.",
                    issue.capability
                ),
                format!(
                    "Use `background_shell_wait_ready {{\"jobId\":\"{provider_ref}\",\"timeoutMs\":5000}}` to wait on the capability provider."
                ),
                format!(
                    "Use `orchestration_list_dependencies {{\"filter\":\"booting\",\"capability\":\"@{}\"}}` to keep the dependency view focused.",
                    issue.capability
                ),
            ],
        };
    }
    if prereqs > 0 {
        let blocker_ref = unique_shell_ref_by_intent(state, BackgroundShellIntent::Prerequisite);
        return match audience {
            ActionAudience::Operator => {
                let mut lines = vec![
                    "Run `:ps blockers` to inspect the gating shell or wait dependency."
                        .to_string(),
                ];
                if let Some(job_ref) = blocker_ref.as_deref() {
                    lines.push(format!(
                        "Run `:ps poll {job_ref}` to inspect the blocking shell output directly."
                    ));
                } else {
                    lines.push(
                        "Run `:ps poll <jobId|alias|@capability|n>` on the blocker you care about."
                            .to_string(),
                    );
                }
                lines.push(
                    "Run `:clean blockers` to abandon the current blocking prerequisite work."
                        .to_string(),
                );
                lines
            }
            ActionAudience::Tool => {
                let mut lines = vec![
                    "Use `orchestration_list_workers {\"filter\":\"blockers\"}` to inspect the gating shell or wait dependency.".to_string(),
                ];
                if let Some(job_ref) = blocker_ref.as_deref() {
                    lines.push(format!(
                        "Use `background_shell_poll {{\"jobId\":\"{job_ref}\"}}` to inspect the blocking shell output directly."
                    ));
                } else {
                    lines.push(
                        "Use `background_shell_poll {\"jobId\":\"<jobId|alias|@capability>\"}` on the blocker you care about."
                            .to_string(),
                    );
                }
                lines.push(
                    "Use `background_shell_clean {\"scope\":\"blockers\"}` to abandon the current blocking prerequisite work.".to_string(),
                );
                lines
            }
        };
    }
    if waits > 0 {
        return match audience {
            ActionAudience::Operator => vec![
                "Run `:ps blockers` to inspect the active wait dependencies.".to_string(),
                "Run `:multi-agents` to refresh spawned agent threads.".to_string(),
                "Run `:resume <n>` to switch into the agent thread that matters.".to_string(),
            ],
            ActionAudience::Tool => vec![
                "Use `orchestration_list_workers {\"filter\":\"blockers\"}` to inspect the active wait dependencies.".to_string(),
                "Use `orchestration_list_workers {\"filter\":\"agents\"}` to inspect cached and live agent workers.".to_string(),
                "Continue foreground work until one of the waiting agent results becomes critical.".to_string(),
            ],
        };
    }
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

pub(super) fn action_lines_for_capability(
    state: &AppState,
    audience: ActionAudience,
    capability: &str,
) -> Result<Vec<String>, String> {
    if let Some(issue) = state
        .orchestration
        .background_shells
        .blocking_capability_dependency_issues()
        .into_iter()
        .find(|issue| issue.capability == capability)
    {
        return Ok(match (issue.status, audience) {
            (BackgroundShellCapabilityDependencyState::Missing, ActionAudience::Operator) => {
                let blocker_ref = first_blocking_ref_for_capability(state, capability)
                    .unwrap_or_else(|| "<jobId|alias|n>".to_string());
                let service_ref = unique_running_service_ref(state)
                    .unwrap_or_else(|| "<jobId|alias|n>".to_string());
                vec![
                    format!(
                        "Run `:ps capabilities @{capability}` to inspect the missing provider map."
                    ),
                    format!(
                        "Run `:ps provide {service_ref} @{capability}` to retarget an existing running service, or start a new provider for that role."
                    ),
                    format!(
                        "Run `:ps depend {blocker_ref} <@capability...|none>` to retarget the blocked shell if it should wait on a different reusable role."
                    ),
                    format!(
                        "Run `:ps dependencies missing @{capability}` to inspect the blocked dependency edges."
                    ),
                    format!(
                        "If the blocked shell is no longer needed, run `:clean blockers @{capability}`."
                    ),
                ]
            }
            (BackgroundShellCapabilityDependencyState::Missing, ActionAudience::Tool) => {
                let blocker_ref = first_blocking_ref_for_capability(state, capability)
                    .unwrap_or_else(|| "<jobId|alias|n>".to_string());
                let service_ref = unique_running_service_ref(state)
                    .unwrap_or_else(|| "<jobId|alias|n>".to_string());
                vec![
                    format!(
                        "Use `background_shell_inspect_capability {{\"capability\":\"@{capability}\"}}` to inspect the missing provider map."
                    ),
                    format!(
                        "Use `background_shell_update_service {{\"jobId\":\"{service_ref}\",\"capabilities\":[\"@{capability}\"]}}` to retarget an existing running service, or start a new provider for that capability."
                    ),
                    format!(
                        "Use `background_shell_update_dependencies {{\"jobId\":\"{blocker_ref}\",\"dependsOnCapabilities\":[\"@other.role\"]}}` to retarget the blocked shell if it should depend on a different reusable role."
                    ),
                    format!(
                        "Use `orchestration_list_dependencies {{\"filter\":\"missing\",\"capability\":\"@{capability}\"}}` to inspect the blocked dependency edges."
                    ),
                    format!(
                        "Use `background_shell_clean {{\"scope\":\"blockers\",\"capability\":\"@{capability}\"}}` to abandon the blocking prerequisite shells if they are no longer needed."
                    ),
                ]
            }
            (BackgroundShellCapabilityDependencyState::Ambiguous, ActionAudience::Operator) => {
                let blocker_ref = first_blocking_ref_for_capability(state, capability)
                    .unwrap_or_else(|| "<jobId|alias|n>".to_string());
                let provider_ref = first_provider_ref_for_capability(state, capability)
                    .unwrap_or_else(|| "<jobId|alias|n>".to_string());
                vec![
                    format!(
                        "Run `:ps capabilities @{capability}` to inspect the ambiguous provider set."
                    ),
                    format!(
                        "Run `:ps provide {provider_ref} <@other.role|none>` to remove or replace @{capability} on one running provider before falling back to cleanup."
                    ),
                    format!(
                        "Run `:ps depend {blocker_ref} <@capability...|none>` if the blocked shell should be retargeted to a different dependency role."
                    ),
                    format!(
                        "Run `:clean services @{capability}` to clear the conflicting reusable role in one step."
                    ),
                    format!("Run `:ps services @{capability}` to inspect the remaining providers."),
                ]
            }
            (BackgroundShellCapabilityDependencyState::Ambiguous, ActionAudience::Tool) => {
                let blocker_ref = first_blocking_ref_for_capability(state, capability)
                    .unwrap_or_else(|| "<jobId|alias|n>".to_string());
                let provider_ref = first_provider_ref_for_capability(state, capability)
                    .unwrap_or_else(|| "<jobId|alias|n>".to_string());
                vec![
                    format!(
                        "Use `background_shell_inspect_capability {{\"capability\":\"@{capability}\"}}` to inspect the ambiguous provider set."
                    ),
                    format!(
                        "Use `background_shell_update_service {{\"jobId\":\"{provider_ref}\",\"capabilities\":[\"@other.role\"]}}` or `background_shell_update_service {{\"jobId\":\"{provider_ref}\",\"capabilities\":null}}` to remove or replace the conflicting reusable role before falling back to cleanup."
                    ),
                    format!(
                        "Use `background_shell_update_dependencies {{\"jobId\":\"{blocker_ref}\",\"dependsOnCapabilities\":[\"@other.role\"]}}` if the blocked shell should be retargeted to a different dependency role."
                    ),
                    format!(
                        "Use `background_shell_clean {{\"scope\":\"services\",\"capability\":\"@{capability}\"}}` to clear the conflicting reusable role in one step."
                    ),
                    format!(
                        "Use `background_shell_list_services {{\"capability\":\"@{capability}\"}}` to inspect the remaining providers."
                    ),
                ]
            }
            (BackgroundShellCapabilityDependencyState::Booting, ActionAudience::Operator) => {
                let provider_ref = first_provider_ref_for_capability(state, capability)
                    .unwrap_or_else(|| format!("@{capability}"));
                vec![
                    format!(
                        "Run `:ps services booting @{capability}` to inspect the booting provider state."
                    ),
                    format!(
                        "Run `:ps wait {provider_ref} 5000` to wait on the capability provider."
                    ),
                    format!(
                        "Run `:ps dependencies booting @{capability}` to keep the dependency view focused."
                    ),
                ]
            }
            (BackgroundShellCapabilityDependencyState::Booting, ActionAudience::Tool) => {
                let provider_ref = first_provider_ref_for_capability(state, capability)
                    .unwrap_or_else(|| format!("@{capability}"));
                vec![
                    format!(
                        "Use `background_shell_list_services {{\"status\":\"booting\",\"capability\":\"@{capability}\"}}` to inspect the booting provider state."
                    ),
                    format!(
                        "Use `background_shell_wait_ready {{\"jobId\":\"{provider_ref}\",\"timeoutMs\":5000}}` to wait on the capability provider."
                    ),
                    format!(
                        "Use `orchestration_list_dependencies {{\"filter\":\"booting\",\"capability\":\"@{capability}\"}}` to keep the dependency view focused."
                    ),
                ]
            }
            (BackgroundShellCapabilityDependencyState::Satisfied, _) => vec![],
        });
    }

    Ok(
        match (
            state
                .orchestration
                .background_shells
                .service_capability_issue_for_ref(capability)?,
            audience,
        ) {
            (BackgroundShellCapabilityIssueClass::Missing, ActionAudience::Operator) => {
                let service_ref = unique_running_service_ref(state)
                    .unwrap_or_else(|| "<jobId|alias|n>".to_string());
                vec![
                    format!(
                        "Run `:ps capabilities @{capability}` to confirm there is no running provider."
                    ),
                    format!(
                        "Run `:ps provide {service_ref} @{capability}` to retarget an existing running service, or start a new provider for that role."
                    ),
                ]
            }
            (BackgroundShellCapabilityIssueClass::Missing, ActionAudience::Tool) => {
                let service_ref = unique_running_service_ref(state)
                    .unwrap_or_else(|| "<jobId|alias|n>".to_string());
                vec![
                    format!(
                        "Use `background_shell_inspect_capability {{\"capability\":\"@{capability}\"}}` to confirm there is no running provider."
                    ),
                    format!(
                        "Use `background_shell_update_service {{\"jobId\":\"{service_ref}\",\"capabilities\":[\"@{capability}\"]}}` to retarget an existing running service, or start a new provider for that capability."
                    ),
                ]
            }
            (BackgroundShellCapabilityIssueClass::Ambiguous, ActionAudience::Operator) => vec![
                format!("Run `:ps capabilities @{capability}` to inspect providers and consumers."),
                {
                    let provider_ref = first_provider_ref_for_capability(state, capability)
                        .unwrap_or_else(|| "<jobId|alias|n>".to_string());
                    format!(
                        "Run `:ps provide {provider_ref} <@other.role|none>` to remove or replace @{capability} on one running provider before falling back to cleanup."
                    )
                },
                format!(
                    "Run `:clean services @{capability}` to clear the ambiguous reusable role."
                ),
                format!("Run `:ps services @{capability}` to verify the surviving providers."),
            ],
            (BackgroundShellCapabilityIssueClass::Ambiguous, ActionAudience::Tool) => {
                let provider_ref = first_provider_ref_for_capability(state, capability)
                    .unwrap_or_else(|| "<jobId|alias|n>".to_string());
                vec![
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
                ]
            }
            (BackgroundShellCapabilityIssueClass::Booting, ActionAudience::Operator) => {
                let provider_ref = first_provider_ref_for_capability(state, capability)
                    .unwrap_or_else(|| format!("@{capability}"));
                vec![
                    format!(
                        "Run `:ps services booting @{capability}` to inspect provider readiness."
                    ),
                    format!("Run `:ps wait {provider_ref} 5000` for the booting service you need."),
                    "Run `:ps capabilities booting` to keep the capability view focused."
                        .to_string(),
                ]
            }
            (BackgroundShellCapabilityIssueClass::Booting, ActionAudience::Tool) => {
                let provider_ref = first_provider_ref_for_capability(state, capability)
                    .unwrap_or_else(|| format!("@{capability}"));
                vec![
                format!(
                    "Use `background_shell_list_services {{\"status\":\"booting\",\"capability\":\"@{capability}\"}}` to inspect provider readiness."
                ),
                format!(
                    "Use `background_shell_wait_ready {{\"jobId\":\"{provider_ref}\",\"timeoutMs\":5000}}` for the booting service you need."
                ),
                "Use `background_shell_list_capabilities {\"status\":\"booting\"}` to keep the capability view focused.".to_string(),
            ]
            }
            (BackgroundShellCapabilityIssueClass::Untracked, ActionAudience::Operator) => {
                let provider_ref = first_provider_ref_for_capability(state, capability)
                    .unwrap_or_else(|| format!("@{capability}"));
                vec![
                    format!(
                        "Run `:ps services untracked @{capability}` to inspect the provider missing readiness or attachment metadata."
                    ),
                    format!(
                        "Run `:ps contract {provider_ref} <json-object>` to add `readyPattern`, `protocol`, `endpoint`, or recipes in place for @{capability}."
                    ),
                    format!(
                        "Run `:ps relabel {provider_ref} <label|none>` if the reusable service needs a clearer operator label."
                    ),
                ]
            }
            (BackgroundShellCapabilityIssueClass::Untracked, ActionAudience::Tool) => {
                let provider_ref = first_provider_ref_for_capability(state, capability)
                    .unwrap_or_else(|| format!("@{capability}"));
                vec![
                    format!(
                        "Use `background_shell_list_services {{\"status\":\"untracked\",\"capability\":\"@{capability}\"}}` to inspect the provider missing readiness or attachment metadata."
                    ),
                    format!(
                        "Use `background_shell_update_service {{\"jobId\":\"{provider_ref}\",\"readyPattern\":\"READY\",\"protocol\":\"http\",\"endpoint\":\"http://127.0.0.1:3000\"}}` to add a live readiness or attachment contract for @{capability}."
                    ),
                    format!(
                        "Use `background_shell_update_service {{\"jobId\":\"{provider_ref}\",\"label\":\"service-label\"}}` if the reusable service needs a clearer operator label."
                    ),
                ]
            }
            (BackgroundShellCapabilityIssueClass::Healthy, ActionAudience::Operator) => {
                let provider_ref = first_provider_ref_for_capability(state, capability)
                    .unwrap_or_else(|| format!("@{capability}"));
                let recipe = first_recipe_name_for_capability(state, capability);
                vec![
                    format!(
                        "Run `:ps attach {provider_ref}` to inspect endpoint and recipe details."
                    ),
                    match recipe.as_ref() {
                        Some(recipe) => format!(
                            "Run `:ps attach {provider_ref}` or `{}` to reuse the ready service directly.",
                            operator_recipe_command(&provider_ref, recipe)
                        ),
                        None => format!(
                            "Run `:ps attach {provider_ref}` to inspect endpoint and recipe details."
                        ),
                    },
                ]
            }
            (BackgroundShellCapabilityIssueClass::Healthy, ActionAudience::Tool) => {
                let provider_ref = first_provider_ref_for_capability(state, capability)
                    .unwrap_or_else(|| format!("@{capability}"));
                let recipe = first_recipe_name_for_capability(state, capability);
                vec![
                    format!(
                        "Use `background_shell_attach {{\"jobId\":\"{provider_ref}\"}}` to inspect endpoint and recipe details."
                    ),
                    match recipe.as_ref() {
                        Some(recipe) => format!(
                            "Use `background_shell_attach {{\"jobId\":\"{provider_ref}\"}}` or `{}` to reuse the ready service directly.",
                            tool_recipe_call(&provider_ref, recipe)
                        ),
                        None => format!(
                            "Use `background_shell_attach {{\"jobId\":\"{provider_ref}\"}}` to inspect endpoint and recipe details."
                        ),
                    },
                ]
            }
        },
    )
}
