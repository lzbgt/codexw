use super::super::super::*;

pub(in super::super::super) fn guidance_lines(state: &AppState) -> Vec<String> {
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

    if !capability_conflicts.is_empty() {
        let conflict_count = capability_conflicts.len();
        let first = &capability_conflicts[0].0;
        return vec![
            format!(
                "{} detected across service capabilities.",
                pluralize(conflict_count, "capability conflict is", "capability conflicts are")
            ),
            format!("Resolve ambiguous reuse targets such as @{first} before relying on capability-based attachment."),
            "Use :ps capabilities to inspect the ambiguous capability map and assign more specific capabilities.".to_string(),
        ];
    }
    if ready_services > 0 {
        let provider_ref =
            unique_service_ref_by_readiness(state, BackgroundShellServiceReadiness::Ready);
        let recipe =
            unique_service_recipe_name_by_readiness(state, BackgroundShellServiceReadiness::Ready);
        return vec![
            format!(
                "{} {} ready for reuse.",
                pluralize(ready_services, "service", "services"),
                if ready_services == 1 { "is" } else { "are" }
            ),
            "Use :ps services to inspect attachment metadata and available recipes.".to_string(),
            match provider_ref.as_deref() {
                Some(job_ref) => match recipe.as_ref() {
                    Some(recipe) => format!(
                        "Use :ps attach {job_ref} or {} to reuse the service directly.",
                        operator_recipe_command(job_ref, recipe)
                    ),
                    None => format!(
                        "Use :ps attach {job_ref} to inspect endpoint and recipe details for the ready service."
                    ),
                },
                None => "Use :ps attach <jobId|alias|@capability|n> or :ps run <jobId|alias|@capability|n> <recipe> [json-args] to reuse the service directly."
                    .to_string(),
            },
        ];
    }
    if booting_services > 0 {
        let provider_ref =
            unique_service_ref_by_readiness(state, BackgroundShellServiceReadiness::Booting);
        return vec![
            format!(
                "{} still booting.",
                pluralize(booting_services, "service shell is", "service shells are")
            ),
            "Use :ps services to inspect readiness state and startup metadata.".to_string(),
            match provider_ref.as_deref() {
                Some(job_ref) => format!(
                    "Use :ps wait {job_ref} [timeoutMs] when later work depends on service readiness."
                ),
                None => "Use :ps wait <jobId|alias|@capability|n> [timeoutMs] when later work depends on service readiness."
                    .to_string(),
            },
        ];
    }
    if untracked_services > 0 {
        let provider_ref =
            unique_service_ref_by_readiness(state, BackgroundShellServiceReadiness::Untracked);
        return vec![
            format!(
                "{} missing readiness or attachment metadata.",
                pluralize(untracked_services, "service shell is", "service shells are")
            ),
            "Use :ps services untracked to inspect services that still need contract metadata."
                .to_string(),
            match provider_ref.as_deref() {
                Some(job_ref) => format!(
                    "Use :ps contract {job_ref} <json-object> or :ps relabel {job_ref} <label|none> to make the service reusable in place."
                ),
                None => "Use :ps contract <jobId|alias|@capability|n> <json-object> or :ps relabel <jobId|alias|@capability|n> <label|none> to make the service reusable in place."
                    .to_string(),
            },
        ];
    }
    if sidecar_agents + shell_sidecars > 0 {
        let sidecars = sidecar_agents + shell_sidecars;
        return vec![
            format!(
                "{} running without blocking the main agent.",
                pluralize(sidecars, "sidecar is", "sidecars are")
            ),
            "Continue independent work on the foreground thread.".to_string(),
            "Use :ps agents or :ps shells to inspect progress only when the result becomes relevant.".to_string(),
        ];
    }
    if terminals > 0 {
        return vec![
            format!(
                "{} still active.",
                pluralize(terminals, "server terminal is", "server terminals are")
            ),
            "Use :ps terminals to inspect them or :clean terminals to close them.".to_string(),
        ];
    }

    Vec::new()
}

pub(in super::super::super) fn guidance_lines_for_tool(state: &AppState) -> Vec<String> {
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

    if !capability_conflicts.is_empty() {
        let conflict_count = capability_conflicts.len();
        let first = &capability_conflicts[0].0;
        return vec![
            format!(
                "{} detected across service capabilities.",
                pluralize(
                    conflict_count,
                    "capability conflict is",
                    "capability conflicts are"
                )
            ),
            format!(
                "Use `background_shell_update_service {{\"jobId\":\"<jobId|alias|n>\",\"capabilities\":[\"@other.role\"]}}` or `background_shell_update_service {{\"jobId\":\"<jobId|alias|n>\",\"capabilities\":null}}` to resolve ambiguous reuse targets such as @{first}."
            ),
            format!(
                "Use `background_shell_inspect_capability {{\"capability\":\"@{first}\"}}` to inspect the ambiguous capability map."
            ),
        ];
    }
    if ready_services > 0 {
        let provider_ref =
            unique_service_ref_by_readiness(state, BackgroundShellServiceReadiness::Ready);
        let recipe =
            unique_service_recipe_name_by_readiness(state, BackgroundShellServiceReadiness::Ready);
        return vec![
            format!(
                "{} {} ready for reuse.",
                pluralize(ready_services, "service", "services"),
                if ready_services == 1 { "is" } else { "are" }
            ),
            "Use `background_shell_list_services {\"status\":\"ready\"}` to inspect attachment metadata and available recipes.".to_string(),
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
                None => "Use `background_shell_attach {\"jobId\":\"<jobId|alias|@capability>\"}` or `background_shell_invoke_recipe {\"jobId\":\"<jobId|alias|@capability>\",\"recipe\":\"...\"}` to reuse the ready service directly.".to_string(),
            },
        ];
    }
    if booting_services > 0 {
        let provider_ref =
            unique_service_ref_by_readiness(state, BackgroundShellServiceReadiness::Booting);
        return vec![
            format!(
                "{} still booting.",
                pluralize(booting_services, "service shell is", "service shells are")
            ),
            "Use `background_shell_list_services {\"status\":\"booting\"}` to inspect readiness state and startup metadata.".to_string(),
            match provider_ref.as_deref() {
                Some(job_ref) => format!(
                    "Use `background_shell_wait_ready {{\"jobId\":\"{job_ref}\",\"timeoutMs\":5000}}` when later work depends on service readiness."
                ),
                None => "Use `background_shell_wait_ready {\"jobId\":\"<jobId|alias|@capability>\",\"timeoutMs\":5000}` when later work depends on service readiness.".to_string(),
            },
        ];
    }
    if untracked_services > 0 {
        let provider_ref =
            unique_service_ref_by_readiness(state, BackgroundShellServiceReadiness::Untracked);
        return vec![
            format!(
                "{} missing readiness or attachment metadata.",
                pluralize(untracked_services, "service shell is", "service shells are")
            ),
            "Use `background_shell_list_services {\"status\":\"untracked\"}` to inspect services that still need contract metadata.".to_string(),
            match provider_ref.as_deref() {
                Some(job_ref) => format!(
                    "Use `background_shell_update_service {{\"jobId\":\"{job_ref}\",\"readyPattern\":\"READY\",\"protocol\":\"http\",\"endpoint\":\"http://127.0.0.1:3000\"}}` or `background_shell_update_service {{\"jobId\":\"{job_ref}\",\"label\":\"service-label\"}}` to make the service reusable in place."
                ),
                None => "Use `background_shell_update_service {\"jobId\":\"<jobId|alias|@capability>\",\"readyPattern\":\"READY\",\"protocol\":\"http\",\"endpoint\":\"http://127.0.0.1:3000\"}` or `background_shell_update_service {\"jobId\":\"<jobId|alias|@capability>\",\"label\":\"service-label\"}` to make the service reusable in place.".to_string(),
            },
        ];
    }
    if sidecar_agents + shell_sidecars > 0 {
        return vec![
            format!(
                "{} running without blocking the main agent.",
                pluralize(sidecar_agents + shell_sidecars, "sidecar is", "sidecars are")
            ),
            "Continue independent work on the foreground thread.".to_string(),
            "Use `orchestration_list_workers {\"filter\":\"agents\"}` or `orchestration_list_workers {\"filter\":\"shells\"}` to inspect progress only when the result becomes relevant.".to_string(),
        ];
    }
    if terminals > 0 {
        return vec![
            format!(
                "{} still active.",
                pluralize(terminals, "server terminal is", "server terminals are")
            ),
            "Use `orchestration_list_workers {\"filter\":\"terminals\"}` to inspect them."
                .to_string(),
        ];
    }

    Vec::new()
}
