use std::collections::BTreeSet;

use crate::background_shells::BackgroundShellCapabilityIssueClass;
use crate::background_shells::BackgroundShellJobSnapshot;
use crate::orchestration_registry::main_agent_state_label;
use crate::orchestration_registry::wait_dependency_summary;
use crate::state::AppState;

use super::LocalApiBackgroundShellJob;
use super::LocalApiBackgroundShellOrigin;
use super::LocalApiBackgroundTerminal;
use super::LocalApiCachedAgentThread;
use super::LocalApiCapabilityConsumer;
use super::LocalApiCapabilityEntry;
use super::LocalApiCapabilityProvider;
use super::LocalApiLiveAgentTask;
use super::LocalApiTranscriptEntry;
use super::LocalApiWorkersSnapshot;

pub(super) fn workers_snapshot(state: &AppState) -> LocalApiWorkersSnapshot {
    let mut live_agent_tasks = state
        .orchestration
        .live_agent_tasks
        .values()
        .cloned()
        .collect::<Vec<_>>();
    live_agent_tasks.sort_by(|left, right| left.id.cmp(&right.id));

    let mut background_terminals = state
        .orchestration
        .background_terminals
        .values()
        .cloned()
        .collect::<Vec<_>>();
    background_terminals.sort_by(|left, right| left.process_id.cmp(&right.process_id));

    LocalApiWorkersSnapshot {
        main_agent_state: main_agent_state_label(state).to_string(),
        wait_summary: wait_dependency_summary(state),
        cached_agent_threads: state
            .orchestration
            .cached_agent_threads
            .iter()
            .cloned()
            .map(|thread| LocalApiCachedAgentThread {
                id: thread.id,
                status: thread.status,
                preview: thread.preview,
                updated_at: thread.updated_at,
            })
            .collect(),
        live_agent_tasks: live_agent_tasks
            .into_iter()
            .map(|task| LocalApiLiveAgentTask {
                id: task.id,
                tool: task.tool,
                status: task.status,
                sender_thread_id: task.sender_thread_id,
                receiver_thread_ids: task.receiver_thread_ids,
                prompt: task.prompt,
                agent_statuses: task.agent_statuses,
            })
            .collect(),
        background_shells: state
            .orchestration
            .background_shells
            .snapshots()
            .into_iter()
            .map(local_api_shell_job)
            .collect(),
        background_terminals: background_terminals
            .into_iter()
            .map(|terminal| LocalApiBackgroundTerminal {
                item_id: terminal.item_id,
                process_id: terminal.process_id,
                command_display: terminal.command_display,
                waiting: terminal.waiting,
                recent_inputs: terminal.recent_inputs,
                recent_output: terminal.recent_output,
            })
            .collect(),
    }
}

pub(crate) fn local_api_shell_job(
    snapshot: BackgroundShellJobSnapshot,
) -> LocalApiBackgroundShellJob {
    LocalApiBackgroundShellJob {
        id: snapshot.id,
        pid: snapshot.pid,
        command: snapshot.command,
        cwd: snapshot.cwd,
        intent: snapshot.intent.as_str().to_string(),
        label: snapshot.label,
        alias: snapshot.alias,
        service_capabilities: snapshot.service_capabilities,
        dependency_capabilities: snapshot.dependency_capabilities,
        service_protocol: snapshot.service_protocol,
        service_endpoint: snapshot.service_endpoint,
        attach_hint: snapshot.attach_hint,
        interaction_recipe_names: snapshot
            .interaction_recipes
            .into_iter()
            .map(|recipe| recipe.name)
            .collect(),
        ready_pattern: snapshot.ready_pattern,
        service_readiness: snapshot
            .service_readiness
            .map(|value| value.as_str().to_string()),
        origin: LocalApiBackgroundShellOrigin {
            source_thread_id: snapshot.origin.source_thread_id,
            source_call_id: snapshot.origin.source_call_id,
            source_tool: snapshot.origin.source_tool,
        },
        status: snapshot.status,
        exit_code: snapshot.exit_code,
        total_lines: snapshot.total_lines,
        last_output_age_seconds: snapshot.last_output_age.map(|value| value.as_secs()),
        recent_lines: snapshot.recent_lines,
    }
}

pub(super) fn capabilities_snapshot(state: &AppState) -> Vec<LocalApiCapabilityEntry> {
    let manager = &state.orchestration.background_shells;
    let dependency_summaries = manager.capability_dependency_summaries();
    let mut capabilities = manager
        .service_capability_index()
        .into_iter()
        .map(|(capability, _)| capability)
        .collect::<BTreeSet<_>>();
    capabilities.extend(
        dependency_summaries
            .iter()
            .map(|summary| summary.capability.clone()),
    );

    let mut entries = Vec::new();
    for capability in capabilities {
        let issue = manager
            .service_capability_issue_for_ref(&capability)
            .unwrap_or(BackgroundShellCapabilityIssueClass::Missing);
        let providers = manager
            .running_service_providers_for_capability(&capability)
            .into_iter()
            .map(|job| LocalApiCapabilityProvider {
                job_id: job.id,
                alias: job.alias,
                label: job.label,
                readiness: job
                    .service_readiness
                    .map(|value| value.as_str().to_string()),
                protocol: job.service_protocol,
                endpoint: job.service_endpoint,
            })
            .collect::<Vec<_>>();
        let consumers = dependency_summaries
            .iter()
            .filter(|summary| summary.capability == capability)
            .map(|summary| LocalApiCapabilityConsumer {
                job_id: summary.job_id.clone(),
                alias: summary.job_alias.clone(),
                label: summary.job_label.clone(),
                blocking: summary.blocking,
                status: summary.status.as_str().to_string(),
            })
            .collect::<Vec<_>>();
        entries.push(LocalApiCapabilityEntry {
            capability,
            issue: capability_issue_label(issue).to_string(),
            providers,
            consumers,
        });
    }
    entries.sort_by(|left, right| left.capability.cmp(&right.capability));
    entries
}

pub(super) fn transcript_snapshot(state: &AppState) -> Vec<LocalApiTranscriptEntry> {
    state
        .conversation_history
        .iter()
        .cloned()
        .map(|message| LocalApiTranscriptEntry {
            role: message.role,
            text: message.text,
        })
        .collect()
}

fn capability_issue_label(issue: BackgroundShellCapabilityIssueClass) -> &'static str {
    match issue {
        BackgroundShellCapabilityIssueClass::Healthy => "healthy",
        BackgroundShellCapabilityIssueClass::Missing => "missing",
        BackgroundShellCapabilityIssueClass::Booting => "booting",
        BackgroundShellCapabilityIssueClass::Untracked => "untracked",
        BackgroundShellCapabilityIssueClass::Ambiguous => "ambiguous",
    }
}
