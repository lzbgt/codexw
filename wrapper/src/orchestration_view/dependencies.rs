use crate::state::AppState;

use super::DependencyFilter;
use super::DependencySelection;

pub(crate) fn render_orchestration_dependencies(
    state: &AppState,
    selection: &DependencySelection,
) -> String {
    let lines = render_dependency_section(state, selection);
    if lines.is_empty() {
        return empty_dependency_filter_message(selection);
    }
    lines.join("\n")
}

pub(super) fn render_dependency_section(
    state: &AppState,
    selection: &DependencySelection,
) -> Vec<String> {
    let dependencies = crate::orchestration_registry::orchestration_dependency_edges(state)
        .into_iter()
        .filter(|edge| dependency_matches_filter(edge, selection))
        .collect::<Vec<_>>();
    if dependencies.is_empty() {
        return Vec::new();
    }
    let mut lines = vec![match selection.capability.as_deref() {
        Some(capability) => format!("Dependencies (@{capability}):"),
        None => "Dependencies:".to_string(),
    }];
    for (index, edge) in dependencies.iter().enumerate() {
        lines.push(format!(
            "{:>2}. {} -> {}  [{}{}]",
            index + 1,
            edge.from,
            edge.to,
            edge.kind,
            if edge.blocking { ", blocking" } else { "" }
        ));
    }
    lines
}

fn dependency_matches_filter(
    edge: &crate::orchestration_registry::OrchestrationDependencyEdge,
    selection: &DependencySelection,
) -> bool {
    let filter_matches = match selection.filter {
        DependencyFilter::All => true,
        DependencyFilter::Blocking => edge.blocking,
        DependencyFilter::Sidecars => !edge.blocking,
        DependencyFilter::Missing => edge.kind == "dependsOnCapability:missing",
        DependencyFilter::Booting => edge.kind == "dependsOnCapability:booting",
        DependencyFilter::Ambiguous => edge.kind == "dependsOnCapability:ambiguous",
        DependencyFilter::Satisfied => edge.kind == "dependsOnCapability:satisfied",
    };
    if !filter_matches {
        return false;
    }
    match selection.capability.as_deref() {
        Some(capability) => edge.to == format!("capability:@{capability}"),
        None => true,
    }
}

fn empty_dependency_filter_message(selection: &DependencySelection) -> String {
    let base = match selection.filter {
        DependencyFilter::All => "No dependency edges tracked right now.",
        DependencyFilter::Blocking => "No blocking dependency edges tracked right now.",
        DependencyFilter::Sidecars => "No sidecar dependency edges tracked right now.",
        DependencyFilter::Missing => "No missing capability dependencies tracked right now.",
        DependencyFilter::Booting => "No booting capability dependencies tracked right now.",
        DependencyFilter::Ambiguous => "No ambiguous capability dependencies tracked right now.",
        DependencyFilter::Satisfied => "No satisfied capability dependencies tracked right now.",
    };
    match selection.capability.as_deref() {
        Some(capability) => format!("{base} Capability selector: @{capability}."),
        None => base.to_string(),
    }
}
