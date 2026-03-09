use std::collections::BTreeMap;
use std::collections::BTreeSet;

use super::super::super::BackgroundShellCapabilityDependencyState;
use super::super::super::BackgroundShellCapabilityDependencySummary;
use super::super::super::BackgroundShellCapabilityIssueClass;
use super::super::super::BackgroundShellIntent;
use super::super::super::BackgroundShellJobSnapshot;
use super::super::super::BackgroundShellManager;
use super::super::super::BackgroundShellServiceReadiness;
use super::super::super::validate_service_capability;

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

pub(super) fn dependency_consumer_display(
    summary: &BackgroundShellCapabilityDependencySummary,
) -> String {
    if let Some(alias) = summary.job_alias.as_deref() {
        format!("{} ({alias})", summary.job_id)
    } else if let Some(label) = summary.job_label.as_deref() {
        format!("{} ({label})", summary.job_id)
    } else {
        summary.job_id.clone()
    }
}

pub(super) fn provider_display(snapshot: &BackgroundShellJobSnapshot) -> String {
    if let Some(alias) = snapshot.alias.as_deref() {
        format!("{} ({alias})", snapshot.id)
    } else if let Some(label) = snapshot.label.as_deref() {
        format!("{} ({label})", snapshot.id)
    } else {
        snapshot.id.clone()
    }
}

pub(crate) fn parse_capability_issue_filter(
    raw: Option<&str>,
    context: &str,
) -> Result<Option<BackgroundShellCapabilityIssueClass>, String> {
    match raw {
        None | Some("all") => Ok(None),
        Some("healthy") | Some("ok") => Ok(Some(BackgroundShellCapabilityIssueClass::Healthy)),
        Some("missing") => Ok(Some(BackgroundShellCapabilityIssueClass::Missing)),
        Some("booting") => Ok(Some(BackgroundShellCapabilityIssueClass::Booting)),
        Some("untracked") => Ok(Some(BackgroundShellCapabilityIssueClass::Untracked)),
        Some("ambiguous") | Some("conflicts") | Some("conflict") => {
            Ok(Some(BackgroundShellCapabilityIssueClass::Ambiguous))
        }
        Some(other) => Err(format!(
            "{context} `status` must be one of `all`, `healthy`, `missing`, `booting`, `untracked`, or `ambiguous`, got `{other}`"
        )),
    }
}
