use super::capabilities::provider_display;
use crate::background_shells::BackgroundShellCapabilityDependencyState;
use crate::background_shells::BackgroundShellCapabilityDependencySummary;
use crate::background_shells::BackgroundShellManager;
use crate::background_shells::BackgroundShellServiceReadiness;

impl BackgroundShellManager {
    pub(crate) fn capability_dependency_summaries(
        &self,
    ) -> Vec<BackgroundShellCapabilityDependencySummary> {
        let services = self.running_service_snapshots();
        let mut summaries = Vec::new();
        for snapshot in self
            .snapshots()
            .into_iter()
            .filter(|job| job.exit_code.is_none() && job.status == "running")
        {
            for capability in &snapshot.dependency_capabilities {
                let providers = services
                    .iter()
                    .filter(|service| {
                        service
                            .service_capabilities
                            .iter()
                            .any(|entry| entry == capability)
                    })
                    .map(provider_display)
                    .collect::<Vec<_>>();
                let status = if providers.is_empty() {
                    BackgroundShellCapabilityDependencyState::Missing
                } else if providers.len() > 1 {
                    BackgroundShellCapabilityDependencyState::Ambiguous
                } else if services.iter().any(|service| {
                    service
                        .service_capabilities
                        .iter()
                        .any(|entry| entry == capability)
                        && service.service_readiness
                            == Some(BackgroundShellServiceReadiness::Booting)
                }) {
                    BackgroundShellCapabilityDependencyState::Booting
                } else {
                    BackgroundShellCapabilityDependencyState::Satisfied
                };
                summaries.push(BackgroundShellCapabilityDependencySummary {
                    job_id: snapshot.id.clone(),
                    job_alias: snapshot.alias.clone(),
                    job_label: snapshot.label.clone(),
                    capability: capability.clone(),
                    blocking: snapshot.intent.is_blocking(),
                    status,
                    providers,
                });
            }
        }
        summaries.sort_by(|left, right| {
            left.blocking
                .cmp(&right.blocking)
                .reverse()
                .then_with(|| left.job_id.cmp(&right.job_id))
                .then_with(|| left.capability.cmp(&right.capability))
        });
        summaries
    }

    pub(crate) fn blocking_capability_dependency_issues(
        &self,
    ) -> Vec<BackgroundShellCapabilityDependencySummary> {
        self.capability_dependency_summaries()
            .into_iter()
            .filter(|summary| {
                summary.blocking
                    && !matches!(
                        summary.status,
                        BackgroundShellCapabilityDependencyState::Satisfied
                    )
            })
            .collect()
    }

    pub(crate) fn capability_dependency_count_by_state(
        &self,
        status: BackgroundShellCapabilityDependencyState,
    ) -> usize {
        self.capability_dependency_summaries()
            .into_iter()
            .filter(|summary| summary.status == status)
            .count()
    }
}
