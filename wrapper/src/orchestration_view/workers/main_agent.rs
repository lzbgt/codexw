use crate::background_shells::BackgroundShellIntent;
use crate::background_shells::BackgroundShellServiceReadiness;
use crate::orchestration_registry::active_sidecar_agent_task_count;
use crate::orchestration_registry::blocking_dependency_count;
use crate::orchestration_registry::main_agent_state_label;
use crate::orchestration_registry::running_service_count_by_readiness;
use crate::orchestration_registry::running_shell_count_by_intent;
use crate::orchestration_registry::sidecar_dependency_count;
use crate::orchestration_registry::wait_dependency_summary;
use crate::state::AppState;

use super::super::DependencyFilter;
use super::super::DependencySelection;
use super::super::WorkerFilter;
use super::super::dependencies::render_dependency_section;

pub(super) fn render_main_agent_section(state: &AppState, filter: WorkerFilter) -> Vec<String> {
    let mut lines = Vec::new();
    if !matches!(filter, WorkerFilter::Dependencies) {
        let mut main_line = format!("Main agent state: {}", main_agent_state_label(state));
        if let Some(waiting_on) = wait_dependency_summary(state) {
            main_line.push_str(&format!(" | {waiting_on}"));
        }
        main_line.push_str(&format!(
            " | sidecar agents={} | exec prereqs={} | exec sidecars={} | exec services={} (ready={} booting={} untracked={} conflicted={}) | deps blocking={} sidecar={}",
            active_sidecar_agent_task_count(state),
            running_shell_count_by_intent(state, BackgroundShellIntent::Prerequisite),
            running_shell_count_by_intent(state, BackgroundShellIntent::Observation),
            running_shell_count_by_intent(state, BackgroundShellIntent::Service),
            running_service_count_by_readiness(state, BackgroundShellServiceReadiness::Ready),
            running_service_count_by_readiness(state, BackgroundShellServiceReadiness::Booting),
            running_service_count_by_readiness(state, BackgroundShellServiceReadiness::Untracked),
            state
                .orchestration
                .background_shells
                .service_conflicting_job_count(),
            blocking_dependency_count(state),
            sidecar_dependency_count(state)
        ));
        lines.push(main_line);
    }
    let dependency_selection = if matches!(filter, WorkerFilter::Blockers) {
        DependencySelection {
            filter: DependencyFilter::Blocking,
            capability: None,
        }
    } else {
        DependencySelection {
            filter: DependencyFilter::All,
            capability: None,
        }
    };
    let dependency_lines = render_dependency_section(state, &dependency_selection);
    if !dependency_lines.is_empty() {
        if !lines.is_empty() {
            lines.push(String::new());
        }
        lines.extend(dependency_lines);
    }
    lines
}
