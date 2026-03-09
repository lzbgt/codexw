use super::super::super::BackgroundShellCapabilityIssueClass;
use super::super::super::BackgroundShellIntent;
use super::super::super::BackgroundShellJobSnapshot;
use super::super::super::BackgroundShellManager;
use super::super::super::BackgroundShellServiceIssueClass;
use super::super::super::BackgroundShellServiceReadiness;
use super::super::super::validate_service_capability;
use super::index::dependency_consumer_display;
use super::index::provider_display;

impl BackgroundShellManager {
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
        let conflict_job_ids = self
            .service_capability_conflicts()
            .into_iter()
            .flat_map(|(_, jobs)| jobs)
            .filter_map(|job| {
                job.split_once(" ")
                    .map(|(id, _)| id.to_string())
                    .or(Some(job))
            })
            .collect::<std::collections::BTreeSet<_>>();
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
            .service_capability_index()
            .iter()
            .map(|(capability, _)| capability.clone())
            .chain(
                self.capability_dependency_summaries()
                    .into_iter()
                    .map(|summary| summary.capability),
            )
            .collect::<std::collections::BTreeSet<_>>()
            .into_iter()
            .map(|capability| {
                let issue = self
                    .service_capability_issue_for_ref(&capability)
                    .expect("issue class");
                let consumers = self
                    .capability_dependency_summaries()
                    .into_iter()
                    .filter(|summary| summary.capability == capability)
                    .map(|summary| {
                        format!(
                            "{} [{}]",
                            dependency_consumer_display(&summary),
                            summary.status.as_str()
                        )
                    })
                    .collect::<Vec<_>>();
                (capability, issue, consumers)
            })
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
                BackgroundShellCapabilityIssueClass::Untracked => " [untracked]",
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
