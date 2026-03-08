use std::collections::BTreeMap;
use std::collections::BTreeSet;

use super::BackgroundShellCapabilityDependencyState;
use super::BackgroundShellCapabilityDependencySummary;
use super::BackgroundShellCapabilityIssueClass;
use super::BackgroundShellIntent;
use super::BackgroundShellJobSnapshot;
use super::BackgroundShellManager;
use super::BackgroundShellServiceIssueClass;
use super::BackgroundShellServiceReadiness;
use super::terminate_jobs;
use super::validate_service_capability;

impl BackgroundShellManager {
    pub(crate) fn terminate_running_services_by_capability(
        &self,
        capability: &str,
    ) -> Result<usize, String> {
        let capability = validate_service_capability(capability.trim_start_matches('@'))?;
        let job_ids = self
            .running_service_snapshots()
            .into_iter()
            .filter(|job| {
                job.service_capabilities
                    .iter()
                    .any(|entry| entry == &capability)
            })
            .map(|job| job.id)
            .collect::<Vec<_>>();
        if job_ids.is_empty() {
            return Err(format!(
                "unknown running background shell capability `@{capability}`; use /ps capabilities to inspect reusable service roles"
            ));
        }
        Ok(terminate_jobs(self, job_ids))
    }

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

    pub(crate) fn render_for_ps(&self) -> Option<Vec<String>> {
        self.render_for_ps_filtered(None)
    }

    pub(crate) fn render_for_ps_filtered(
        &self,
        intent_filter: Option<BackgroundShellIntent>,
    ) -> Option<Vec<String>> {
        let snapshots = self.snapshots();
        let snapshots = snapshots
            .into_iter()
            .filter(|snapshot| intent_filter.is_none_or(|intent| snapshot.intent == intent))
            .collect::<Vec<_>>();
        self.render_snapshots_for_ps(
            snapshots,
            matches!(intent_filter, None | Some(BackgroundShellIntent::Service)),
            matches!(intent_filter, None | Some(BackgroundShellIntent::Service)),
        )
    }

    pub(crate) fn render_service_shells_for_ps_filtered(
        &self,
        issue_filter: Option<BackgroundShellServiceIssueClass>,
        capability_filter: Option<&str>,
    ) -> Option<Vec<String>> {
        let conflict_job_ids = self.service_conflicting_job_ids();
        let capability_filter = capability_filter
            .map(|capability| validate_service_capability(capability.trim_start_matches('@')))
            .transpose()
            .ok()?;
        let snapshots = self
            .running_service_snapshots()
            .into_iter()
            .filter(|snapshot| {
                capability_filter.as_ref().is_none_or(|capability| {
                    snapshot
                        .service_capabilities
                        .iter()
                        .any(|entry| entry == capability)
                })
            })
            .filter(|snapshot| match issue_filter {
                None => true,
                Some(BackgroundShellServiceIssueClass::Ready) => {
                    snapshot.service_readiness == Some(BackgroundShellServiceReadiness::Ready)
                }
                Some(BackgroundShellServiceIssueClass::Booting) => {
                    snapshot.service_readiness == Some(BackgroundShellServiceReadiness::Booting)
                }
                Some(BackgroundShellServiceIssueClass::Untracked) => {
                    snapshot.service_readiness == Some(BackgroundShellServiceReadiness::Untracked)
                }
                Some(BackgroundShellServiceIssueClass::Conflicts) => {
                    conflict_job_ids.contains(&snapshot.id)
                }
            })
            .collect::<Vec<_>>();
        let include_capability_index = issue_filter.is_none() && capability_filter.is_none();
        let include_conflict_summary = issue_filter.is_none() && capability_filter.is_none()
            || matches!(
                issue_filter,
                Some(BackgroundShellServiceIssueClass::Conflicts)
            );
        self.render_snapshots_for_ps(
            snapshots,
            include_capability_index,
            include_conflict_summary,
        )
    }

    fn render_snapshots_for_ps(
        &self,
        snapshots: Vec<BackgroundShellJobSnapshot>,
        include_capability_index: bool,
        include_conflict_summary: bool,
    ) -> Option<Vec<String>> {
        if snapshots.is_empty() {
            return None;
        }
        let mut lines = vec!["Local background shell jobs:".to_string()];
        for (index, snapshot) in snapshots.into_iter().enumerate() {
            lines.push(format!(
                "{:>2}. {}  [{}]",
                index + 1,
                snapshot.command,
                snapshot.status
            ));
            lines.push(format!("    job      {}", snapshot.id));
            lines.push(format!("    process  {}", snapshot.pid));
            lines.push(format!("    cwd      {}", snapshot.cwd));
            lines.push(format!("    intent   {}", snapshot.intent.as_str()));
            if let Some(label) = snapshot.label.as_deref() {
                lines.push(format!("    label    {label}"));
            }
            if let Some(alias) = snapshot.alias.as_deref() {
                lines.push(format!("    alias    {alias}"));
            }
            if !snapshot.service_capabilities.is_empty() {
                lines.push(format!(
                    "    caps     {}",
                    snapshot.service_capabilities.join(", ")
                ));
            }
            if !snapshot.dependency_capabilities.is_empty() {
                lines.push(format!(
                    "    depends  {}",
                    snapshot
                        .dependency_capabilities
                        .iter()
                        .map(|capability| format!("@{capability}"))
                        .collect::<Vec<_>>()
                        .join(", ")
                ));
            }
            if let Some(protocol) = snapshot.service_protocol.as_deref() {
                lines.push(format!("    protocol {protocol}"));
            }
            if let Some(endpoint) = snapshot.service_endpoint.as_deref() {
                lines.push(format!("    endpoint {endpoint}"));
            }
            if let Some(attach_hint) = snapshot.attach_hint.as_deref() {
                lines.push(format!("    attach   {attach_hint}"));
            }
            if !snapshot.interaction_recipes.is_empty() {
                lines.push(format!(
                    "    recipes  {}",
                    snapshot.interaction_recipes.len()
                ));
            }
            if let Some(ready_pattern) = snapshot.ready_pattern.as_deref() {
                lines.push(format!("    ready on {ready_pattern}"));
            }
            if let Some(service_readiness) = snapshot.service_readiness {
                lines.push(format!("    service  {}", service_readiness.as_str()));
            }
            lines.push(format!("    lines    {}", snapshot.total_lines));
            if let Some(source_thread_id) = snapshot.origin.source_thread_id.as_deref() {
                lines.push(format!("    origin   thread={source_thread_id}"));
            }
            if let Some(source_call_id) = snapshot.origin.source_call_id.as_deref() {
                lines.push(format!("    call     {source_call_id}"));
            }
            if !snapshot.recent_lines.is_empty() {
                lines.push(format!(
                    "    output   {}",
                    snapshot.recent_lines.join(" | ")
                ));
            }
        }
        if include_capability_index
            && let Some(capability_lines) = self.render_service_capability_index_lines()
        {
            lines.extend(capability_lines);
        }
        if include_conflict_summary {
            let conflicts = self.service_capability_conflicts();
            if !conflicts.is_empty() {
                lines.push("Capability conflicts:".to_string());
                for (capability, jobs) in conflicts {
                    lines.push(format!("    @{capability} -> {}", jobs.join(", ")));
                }
            }
        }
        Some(lines)
    }

    pub(crate) fn render_service_capabilities_for_ps(&self) -> Option<Vec<String>> {
        self.render_service_capabilities_for_ps_filtered(None)
    }

    pub(crate) fn list_services_from_tool(
        &self,
        arguments: &serde_json::Value,
    ) -> Result<String, String> {
        let object = arguments.as_object();
        let issue_filter = parse_service_issue_filter(
            object
                .and_then(|object| object.get("status"))
                .and_then(serde_json::Value::as_str),
            "background_shell_list_services",
        )?;
        let capability_filter = object
            .and_then(|object| object.get("capability"))
            .and_then(serde_json::Value::as_str);
        if let Some(capability) = capability_filter {
            validate_service_capability(capability.trim_start_matches('@'))?;
        }
        self.render_service_shells_for_ps_filtered(issue_filter, capability_filter)
            .map(|lines| lines.join("\n"))
            .ok_or_else(|| "No service shells tracked right now.".to_string())
    }

    pub(crate) fn render_service_capabilities_for_ps_filtered(
        &self,
        issue_filter: Option<BackgroundShellCapabilityIssueClass>,
    ) -> Option<Vec<String>> {
        let entries = self
            .capability_issue_entries()
            .into_iter()
            .filter(|(_, issue, _)| issue_filter.is_none_or(|wanted| *issue == wanted))
            .collect::<Vec<_>>();
        if entries.is_empty() {
            return None;
        }
        let mut lines = vec!["Service capability index:".to_string()];
        for (index, (capability, issue, consumers)) in entries.iter().enumerate() {
            let providers = self
                .running_service_providers_for_capability(capability)
                .into_iter()
                .map(|provider| provider_display(&provider))
                .collect::<Vec<_>>();
            let qualifier = match issue {
                BackgroundShellCapabilityIssueClass::Healthy => "",
                BackgroundShellCapabilityIssueClass::Missing => " [missing]",
                BackgroundShellCapabilityIssueClass::Booting => " [booting]",
                BackgroundShellCapabilityIssueClass::Ambiguous => " [conflict]",
            };
            lines.push(format!(
                "{:>2}. @{} -> {}{}",
                index + 1,
                capability,
                if providers.is_empty() {
                    "<missing provider>".to_string()
                } else {
                    providers.join(", ")
                },
                qualifier
            ));
            if !consumers.is_empty() {
                lines.push(format!("    used by {}", consumers.join(", ")));
            }
        }
        lines.push(
            "Use @capability with :ps poll|send|attach|wait|run|terminate to target a reusable service."
                .to_string(),
        );
        Some(lines)
    }

    pub(crate) fn render_single_service_capability_for_ps(
        &self,
        capability_ref: &str,
    ) -> Result<Vec<String>, String> {
        let capability = if let Some(raw) = capability_ref.strip_prefix('@') {
            validate_service_capability(raw)?
        } else {
            validate_service_capability(capability_ref)?
        };
        let providers = self.running_service_providers_for_capability(&capability);
        let consumers = self
            .capability_dependency_summaries()
            .into_iter()
            .filter(|summary| summary.capability == capability)
            .collect::<Vec<_>>();
        if providers.is_empty() && consumers.is_empty() {
            return Err(format!("unknown service capability `@{capability}`"));
        }
        let mut lines = vec![format!("Service capability: @{capability}")];
        if providers.is_empty() {
            lines.push("Providers: <missing provider>".to_string());
        } else {
            lines.push("Providers:".to_string());
            for (index, provider) in providers.iter().enumerate() {
                lines.push(format!(
                    "{:>2}. {}  [{}]",
                    index + 1,
                    provider_display(provider),
                    provider
                        .service_readiness
                        .map(BackgroundShellServiceReadiness::as_str)
                        .unwrap_or("-")
                ));
                if let Some(protocol) = provider.service_protocol.as_deref() {
                    lines.push(format!("    protocol {protocol}"));
                }
                if let Some(endpoint) = provider.service_endpoint.as_deref() {
                    lines.push(format!("    endpoint {endpoint}"));
                }
                if !provider.interaction_recipes.is_empty() {
                    lines.push(format!(
                        "    recipes  {}",
                        provider.interaction_recipes.len()
                    ));
                }
            }
            if providers.len() > 1 {
                lines.push("Conflict: ambiguous capability provider set".to_string());
            }
        }
        if consumers.is_empty() {
            lines.push("Consumers: none".to_string());
        } else {
            lines.push("Consumers:".to_string());
            for (index, consumer) in consumers.iter().enumerate() {
                let job_ref = dependency_consumer_display(consumer);
                lines.push(format!(
                    "{:>2}. {}  [{}]  blocking={}",
                    index + 1,
                    job_ref,
                    consumer.status.as_str(),
                    if consumer.blocking { "yes" } else { "no" }
                ));
            }
        }
        Ok(lines)
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

    fn running_service_providers_for_capability(
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
        BackgroundShellCapabilityIssueClass::Healthy
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
                    BackgroundShellCapabilityIssueClass::Ambiguous => " [conflict]",
                },
            ));
            if !consumers.is_empty() {
                lines.push(format!("      used by {}", consumers.join(", ")));
            }
        }
        Some(lines)
    }
}

fn dependency_consumer_display(summary: &BackgroundShellCapabilityDependencySummary) -> String {
    if let Some(alias) = summary.job_alias.as_deref() {
        format!("{} ({alias})", summary.job_id)
    } else if let Some(label) = summary.job_label.as_deref() {
        format!("{} ({label})", summary.job_id)
    } else {
        summary.job_id.clone()
    }
}

fn provider_display(snapshot: &BackgroundShellJobSnapshot) -> String {
    if let Some(alias) = snapshot.alias.as_deref() {
        format!("{} ({alias})", snapshot.id)
    } else if let Some(label) = snapshot.label.as_deref() {
        format!("{} ({label})", snapshot.id)
    } else {
        snapshot.id.clone()
    }
}

pub(super) fn parse_capability_issue_filter(
    raw: Option<&str>,
    context: &str,
) -> Result<Option<BackgroundShellCapabilityIssueClass>, String> {
    match raw {
        None | Some("all") => Ok(None),
        Some("healthy") | Some("ok") => Ok(Some(BackgroundShellCapabilityIssueClass::Healthy)),
        Some("missing") => Ok(Some(BackgroundShellCapabilityIssueClass::Missing)),
        Some("booting") => Ok(Some(BackgroundShellCapabilityIssueClass::Booting)),
        Some("ambiguous") | Some("conflicts") | Some("conflict") => {
            Ok(Some(BackgroundShellCapabilityIssueClass::Ambiguous))
        }
        Some(other) => Err(format!(
            "{context} `status` must be one of `all`, `healthy`, `missing`, `booting`, or `ambiguous`, got `{other}`"
        )),
    }
}

pub(super) fn parse_service_issue_filter(
    raw: Option<&str>,
    context: &str,
) -> Result<Option<BackgroundShellServiceIssueClass>, String> {
    match raw {
        None | Some("all") => Ok(None),
        Some("ready") | Some("healthy") => Ok(Some(BackgroundShellServiceIssueClass::Ready)),
        Some("booting") => Ok(Some(BackgroundShellServiceIssueClass::Booting)),
        Some("untracked") => Ok(Some(BackgroundShellServiceIssueClass::Untracked)),
        Some("conflicts") | Some("conflict") | Some("ambiguous") => {
            Ok(Some(BackgroundShellServiceIssueClass::Conflicts))
        }
        Some(other) => Err(format!(
            "{context} `status` must be one of `all`, `ready`, `booting`, `untracked`, or `conflicts`, got `{other}`"
        )),
    }
}
