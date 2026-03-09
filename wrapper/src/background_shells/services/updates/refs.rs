use super::super::super::BackgroundShellCapabilityDependencySummary;
use super::super::super::BackgroundShellJobSnapshot;
use super::super::super::BackgroundShellManager;
use super::super::super::BackgroundShellServiceReadiness;
use super::super::super::validate_service_capability;

impl BackgroundShellManager {
    pub(crate) fn running_service_provider_refs_for_capability(
        &self,
        capability_ref: &str,
    ) -> Result<Vec<String>, String> {
        let capability = if let Some(raw) = capability_ref.strip_prefix('@') {
            validate_service_capability(raw)?
        } else {
            validate_service_capability(capability_ref)?
        };
        Ok(self
            .running_service_providers_for_capability(&capability)
            .into_iter()
            .map(|snapshot| mutable_job_ref(&snapshot))
            .collect())
    }

    pub(crate) fn blocking_dependency_job_refs_for_capability(
        &self,
        capability_ref: &str,
    ) -> Result<Vec<String>, String> {
        let capability = if let Some(raw) = capability_ref.strip_prefix('@') {
            validate_service_capability(raw)?
        } else {
            validate_service_capability(capability_ref)?
        };
        Ok(self
            .blocking_capability_dependency_issues()
            .into_iter()
            .filter(|summary| summary.capability == capability)
            .map(|summary| dependency_consumer_ref(&summary))
            .collect())
    }

    pub(crate) fn running_service_refs_by_readiness(
        &self,
        readiness: BackgroundShellServiceReadiness,
    ) -> Vec<String> {
        self.running_service_snapshots()
            .into_iter()
            .filter(|job| job.service_readiness == Some(readiness))
            .map(|snapshot| mutable_job_ref(&snapshot))
            .collect()
    }
}

pub(super) fn dependency_consumer_ref(
    summary: &BackgroundShellCapabilityDependencySummary,
) -> String {
    if let Some(alias) = summary.job_alias.as_deref() {
        alias.to_string()
    } else {
        summary.job_id.clone()
    }
}

pub(super) fn mutable_job_ref(snapshot: &BackgroundShellJobSnapshot) -> String {
    if let Some(alias) = snapshot.alias.as_deref() {
        alias.to_string()
    } else {
        snapshot.id.clone()
    }
}
