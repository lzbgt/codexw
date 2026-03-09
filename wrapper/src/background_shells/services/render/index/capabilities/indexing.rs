use std::collections::BTreeMap;
use std::collections::BTreeSet;

use super::refs::dependency_consumer_display;
use super::refs::provider_display;
use crate::background_shells::BackgroundShellCapabilityIssueClass;
use crate::background_shells::BackgroundShellIntent;
use crate::background_shells::BackgroundShellJobSnapshot;
use crate::background_shells::BackgroundShellManager;
use crate::background_shells::BackgroundShellServiceReadiness;
use crate::background_shells::validate_service_capability;

impl BackgroundShellManager {
    pub(crate) fn running_service_count_by_readiness(
        &self,
        readiness: BackgroundShellServiceReadiness,
    ) -> usize {
        self.snapshots()
            .into_iter()
            .filter(|job| {
                job.exit_code.is_none()
                    && job.status == "running"
                    && job.intent == BackgroundShellIntent::Service
                    && job.service_readiness == Some(readiness)
            })
            .count()
    }

    pub(crate) fn service_capability_conflicts(&self) -> Vec<(String, Vec<String>)> {
        let mut conflicts = self
            .service_capability_index()
            .into_iter()
            .filter_map(|(capability, mut jobs)| {
                if jobs.len() < 2 {
                    None
                } else {
                    jobs.sort();
                    Some((capability, jobs))
                }
            })
            .collect::<Vec<_>>();
        conflicts.sort_by(|left, right| left.0.cmp(&right.0));
        conflicts
    }

    pub(crate) fn unique_service_capability_count(&self) -> usize {
        self.service_capability_index().len()
    }

    pub(crate) fn service_capability_index(&self) -> Vec<(String, Vec<String>)> {
        let mut index = BTreeMap::<String, Vec<String>>::new();
        for snapshot in self.running_service_snapshots() {
            let job_ref = snapshot
                .alias
                .as_deref()
                .map(|alias| format!("{} ({alias})", snapshot.id))
                .unwrap_or_else(|| snapshot.id.clone());
            for capability in snapshot.service_capabilities {
                index.entry(capability).or_default().push(job_ref.clone());
            }
        }
        index.into_iter().collect()
    }

    pub(crate) fn service_capability_conflict_count(&self) -> usize {
        self.service_capability_conflicts().len()
    }

    pub(crate) fn service_capability_issue_for_ref(
        &self,
        capability_ref: &str,
    ) -> Result<BackgroundShellCapabilityIssueClass, String> {
        let capability = if let Some(raw) = capability_ref.strip_prefix('@') {
            validate_service_capability(raw)?
        } else {
            validate_service_capability(capability_ref)?
        };
        let providers = self.running_service_providers_for_capability(&capability);
        let has_consumers = self
            .capability_dependency_summaries()
            .into_iter()
            .any(|summary| summary.capability == capability);
        if providers.is_empty() && !has_consumers {
            return Err(format!("unknown service capability `@{capability}`"));
        }
        Ok(self.capability_issue_class(&capability))
    }

    pub(crate) fn service_conflicting_job_count(&self) -> usize {
        self.service_conflicting_job_ids().len()
    }

    pub(crate) fn running_service_snapshots(&self) -> Vec<BackgroundShellJobSnapshot> {
        self.snapshots()
            .into_iter()
            .filter(|job| {
                job.intent == BackgroundShellIntent::Service
                    && job.exit_code.is_none()
                    && job.status == "running"
            })
            .collect()
    }

    pub(crate) fn running_service_providers_for_capability(
        &self,
        capability: &str,
    ) -> Vec<BackgroundShellJobSnapshot> {
        self.running_service_snapshots()
            .into_iter()
            .filter(|job| {
                job.service_capabilities
                    .iter()
                    .any(|entry| entry == capability)
            })
            .collect()
    }

    pub(crate) fn render_service_capability_index_lines(&self) -> Option<Vec<String>> {
        let entries = self.capability_issue_entries();
        if entries.is_empty() {
            return None;
        }
        let mut lines = vec!["Capability index:".to_string()];
        for (capability, issue, consumers) in entries {
            let providers = self
                .running_service_providers_for_capability(&capability)
                .into_iter()
                .map(|provider| provider_display(&provider))
                .collect::<Vec<_>>();
            lines.push(format!(
                "    @{capability} -> {}{}",
                if providers.is_empty() {
                    "<missing provider>".to_string()
                } else {
                    providers.join(", ")
                },
                match issue {
                    BackgroundShellCapabilityIssueClass::Healthy => "",
                    BackgroundShellCapabilityIssueClass::Missing => " [missing]",
                    BackgroundShellCapabilityIssueClass::Booting => " [booting]",
                    BackgroundShellCapabilityIssueClass::Untracked => " [untracked]",
                    BackgroundShellCapabilityIssueClass::Ambiguous => " [conflict]",
                },
            ));
            if !consumers.is_empty() {
                lines.push(format!("      used by {}", consumers.join(", ")));
            }
        }
        Some(lines)
    }

    fn service_conflicting_job_ids(&self) -> BTreeSet<String> {
        let mut by_capability = BTreeMap::<String, Vec<String>>::new();
        for snapshot in self.running_service_snapshots() {
            for capability in &snapshot.service_capabilities {
                by_capability
                    .entry(capability.clone())
                    .or_default()
                    .push(snapshot.id.clone());
            }
        }
        by_capability
            .into_iter()
            .filter(|(_, job_ids)| job_ids.len() > 1)
            .flat_map(|(_, job_ids)| job_ids)
            .collect()
    }

    fn capability_issue_entries(
        &self,
    ) -> Vec<(String, BackgroundShellCapabilityIssueClass, Vec<String>)> {
        let capability_index = self.service_capability_index();
        let mut consumer_index = BTreeMap::<String, Vec<String>>::new();
        for dependency in self.capability_dependency_summaries() {
            let consumer = dependency_consumer_display(&dependency);
            consumer_index
                .entry(dependency.capability)
                .or_default()
                .push(format!("{consumer} [{}]", dependency.status.as_str()));
        }
        let mut capabilities = capability_index
            .iter()
            .map(|(capability, _)| capability.clone())
            .collect::<BTreeSet<_>>();
        capabilities.extend(consumer_index.keys().cloned());
        let mut entries = Vec::new();
        for capability in capabilities {
            let issue = self.capability_issue_class(&capability);
            let consumers = consumer_index.remove(&capability).unwrap_or_default();
            entries.push((capability, issue, consumers));
        }
        entries
    }

    fn capability_issue_class(&self, capability: &str) -> BackgroundShellCapabilityIssueClass {
        let providers = self.running_service_providers_for_capability(capability);
        if providers.is_empty() {
            return BackgroundShellCapabilityIssueClass::Missing;
        }
        if providers.len() > 1 {
            return BackgroundShellCapabilityIssueClass::Ambiguous;
        }
        if providers[0].service_readiness == Some(BackgroundShellServiceReadiness::Booting) {
            return BackgroundShellCapabilityIssueClass::Booting;
        }
        if providers[0].service_readiness == Some(BackgroundShellServiceReadiness::Untracked) {
            return BackgroundShellCapabilityIssueClass::Untracked;
        }
        BackgroundShellCapabilityIssueClass::Healthy
    }
}
