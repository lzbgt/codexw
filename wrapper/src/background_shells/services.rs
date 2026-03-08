use std::collections::BTreeMap;
use std::collections::BTreeSet;

use super::BackgroundShellCapabilityDependencyState;
use super::BackgroundShellCapabilityDependencySummary;
use super::BackgroundShellCapabilityIssueClass;
use super::BackgroundShellIntent;
use super::BackgroundShellInteractionRecipe;
use super::BackgroundShellJobSnapshot;
use super::BackgroundShellJobStatus;
use super::BackgroundShellManager;
use super::BackgroundShellServiceIssueClass;
use super::BackgroundShellServiceReadiness;
use super::parse_background_shell_interaction_recipes;
use super::terminate_jobs;
use super::validate_service_capability;

impl BackgroundShellManager {
    pub(crate) fn set_running_service_contract(
        &self,
        job_id: &str,
        protocol: Option<Option<String>>,
        endpoint: Option<Option<String>>,
        attach_hint: Option<Option<String>>,
        ready_pattern: Option<Option<String>>,
        interaction_recipes: Option<Vec<BackgroundShellInteractionRecipe>>,
    ) -> Result<(), String> {
        let normalized_protocol = match protocol {
            Some(protocol) => Some(normalize_service_label_update(protocol)?),
            None => None,
        };
        let normalized_endpoint = match endpoint {
            Some(endpoint) => Some(normalize_service_label_update(endpoint)?),
            None => None,
        };
        let normalized_attach_hint = match attach_hint {
            Some(attach_hint) => Some(normalize_service_label_update(attach_hint)?),
            None => None,
        };
        let normalized_ready_pattern = match ready_pattern {
            Some(ready_pattern) => Some(normalize_service_label_update(ready_pattern)?),
            None => None,
        };
        let job = self.lookup_job(job_id)?;
        let mut state = job.lock().expect("background shell job lock");
        if !matches!(state.status, BackgroundShellJobStatus::Running) {
            return Err(format!(
                "background shell job `{job_id}` is not running; only running service jobs can change service metadata"
            ));
        }
        if state.intent != BackgroundShellIntent::Service {
            return Err(format!(
                "background shell job `{job_id}` is not a service job; only running service jobs can change service metadata"
            ));
        }
        if let Some(protocol) = normalized_protocol.clone() {
            state.service_protocol = protocol;
        }
        if let Some(endpoint) = normalized_endpoint.clone() {
            state.service_endpoint = endpoint;
        }
        if let Some(attach_hint) = normalized_attach_hint.clone() {
            state.attach_hint = attach_hint;
        }
        if let Some(recipes) = interaction_recipes {
            state.interaction_recipes = recipes;
        }
        if let Some(ready_pattern) = normalized_ready_pattern {
            state.ready_pattern = ready_pattern.clone();
            state.service_ready = ready_pattern.as_ref().is_some_and(|pattern| {
                state.lines.iter().any(|entry| entry.text.contains(pattern))
            });
        }
        Ok(())
    }

    pub(crate) fn set_running_dependency_capabilities(
        &self,
        job_id: &str,
        capabilities: &[String],
    ) -> Result<Vec<String>, String> {
        let normalized = normalize_service_capabilities(capabilities)?;
        let job = self.lookup_job(job_id)?;
        let mut state = job.lock().expect("background shell job lock");
        if !matches!(state.status, BackgroundShellJobStatus::Running) {
            return Err(format!(
                "background shell job `{job_id}` is not running; only running jobs can change dependency capabilities"
            ));
        }
        state.dependency_capabilities = normalized.clone();
        Ok(normalized)
    }

    pub(crate) fn set_running_service_label(
        &self,
        job_id: &str,
        label: Option<String>,
    ) -> Result<Option<String>, String> {
        let normalized = normalize_service_label_update(label)?;
        let job = self.lookup_job(job_id)?;
        let mut state = job.lock().expect("background shell job lock");
        if !matches!(state.status, BackgroundShellJobStatus::Running) {
            return Err(format!(
                "background shell job `{job_id}` is not running; only running service jobs can change service metadata"
            ));
        }
        if state.intent != BackgroundShellIntent::Service {
            return Err(format!(
                "background shell job `{job_id}` is not a service job; only running service jobs can change service metadata"
            ));
        }
        state.label = normalized.clone();
        Ok(normalized)
    }

    pub(crate) fn set_running_service_capabilities(
        &self,
        job_id: &str,
        capabilities: &[String],
    ) -> Result<Vec<String>, String> {
        let normalized = normalize_service_capabilities(capabilities)?;
        let job = self.lookup_job(job_id)?;
        let mut state = job.lock().expect("background shell job lock");
        if !matches!(state.status, BackgroundShellJobStatus::Running) {
            return Err(format!(
                "background shell job `{job_id}` is not running; only running service jobs can change reusable capabilities"
            ));
        }
        if state.intent != BackgroundShellIntent::Service {
            return Err(format!(
                "background shell job `{job_id}` is not a service job; only running service jobs can change reusable capabilities"
            ));
        }
        state.service_capabilities = normalized.clone();
        Ok(normalized)
    }

    pub(crate) fn update_service_label_for_operator(
        &self,
        reference: &str,
        label: Option<String>,
    ) -> Result<String, String> {
        let resolved_job_id = self.resolve_job_reference(reference)?;
        let normalized = self.set_running_service_label(&resolved_job_id, label)?;
        Ok(render_service_metadata_update_summary(
            &resolved_job_id,
            None,
            Some(normalized),
            None,
            None,
            None,
            None,
            None,
        ))
    }

    pub(crate) fn update_service_capabilities_for_operator(
        &self,
        reference: &str,
        capabilities: &[String],
    ) -> Result<String, String> {
        let resolved_job_id = self.resolve_job_reference(reference)?;
        let normalized = self.set_running_service_capabilities(&resolved_job_id, capabilities)?;
        Ok(render_service_metadata_update_summary(
            &resolved_job_id,
            Some(&normalized),
            None,
            None,
            None,
            None,
            None,
            None,
        ))
    }

    pub(crate) fn update_service_contract_for_operator(
        &self,
        reference: &str,
        protocol: Option<Option<String>>,
        endpoint: Option<Option<String>>,
        attach_hint: Option<Option<String>>,
        ready_pattern: Option<Option<String>>,
        interaction_recipes: Option<Vec<BackgroundShellInteractionRecipe>>,
    ) -> Result<String, String> {
        let resolved_job_id = self.resolve_job_reference(reference)?;
        let normalized_protocol = protocol
            .clone()
            .map(normalize_service_label_update)
            .transpose()?;
        let normalized_endpoint = endpoint
            .clone()
            .map(normalize_service_label_update)
            .transpose()?;
        let normalized_attach_hint = attach_hint
            .clone()
            .map(normalize_service_label_update)
            .transpose()?;
        let normalized_ready_pattern = ready_pattern
            .clone()
            .map(normalize_service_label_update)
            .transpose()?;
        let recipe_count = interaction_recipes.as_ref().map(Vec::len);
        self.set_running_service_contract(
            &resolved_job_id,
            protocol,
            endpoint,
            attach_hint,
            ready_pattern,
            interaction_recipes,
        )?;
        Ok(render_service_metadata_update_summary(
            &resolved_job_id,
            None,
            None,
            normalized_protocol,
            normalized_endpoint,
            normalized_attach_hint,
            normalized_ready_pattern,
            recipe_count,
        ))
    }

    pub(crate) fn update_dependency_capabilities_for_operator(
        &self,
        reference: &str,
        capabilities: &[String],
    ) -> Result<String, String> {
        let resolved_job_id = self.resolve_job_reference(reference)?;
        let normalized =
            self.set_running_dependency_capabilities(&resolved_job_id, capabilities)?;
        Ok(render_dependency_capability_update_summary(
            &resolved_job_id,
            &normalized,
        ))
    }

    pub(crate) fn update_service_from_tool(
        &self,
        arguments: &serde_json::Value,
    ) -> Result<String, String> {
        let object = arguments.as_object().ok_or_else(|| {
            "background_shell_update_service expects an object argument".to_string()
        })?;
        let job_id = object
            .get("jobId")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| "background_shell_update_service requires `jobId`".to_string())?;
        let capabilities = if object.contains_key("capabilities") {
            Some(parse_service_capabilities_argument(
                object.get("capabilities"),
                "background_shell_update_service",
                "capabilities",
            )?)
        } else {
            None
        };
        let label = if object.contains_key("label") {
            Some(parse_service_label_argument(
                object.get("label"),
                "background_shell_update_service",
            )?)
        } else {
            None
        };
        let protocol = if object.contains_key("protocol") {
            Some(parse_service_label_argument(
                object.get("protocol"),
                "background_shell_update_service",
            )?)
        } else {
            None
        };
        let endpoint = if object.contains_key("endpoint") {
            Some(parse_service_label_argument(
                object.get("endpoint"),
                "background_shell_update_service",
            )?)
        } else {
            None
        };
        let attach_hint = if object.contains_key("attachHint") {
            Some(parse_service_label_argument(
                object.get("attachHint"),
                "background_shell_update_service",
            )?)
        } else {
            None
        };
        let ready_pattern = if object.contains_key("readyPattern") {
            Some(parse_service_label_argument(
                object.get("readyPattern"),
                "background_shell_update_service",
            )?)
        } else {
            None
        };
        let interaction_recipes = if object.contains_key("recipes") {
            Some(parse_service_recipe_updates(
                object.get("recipes"),
                "background_shell_update_service",
            )?)
        } else {
            None
        };
        if capabilities.is_none()
            && label.is_none()
            && protocol.is_none()
            && endpoint.is_none()
            && attach_hint.is_none()
            && ready_pattern.is_none()
            && interaction_recipes.is_none()
        {
            return Err(
                "background_shell_update_service requires at least one mutable field such as `capabilities`, `label`, `protocol`, `endpoint`, `attachHint`, `readyPattern`, or `recipes`"
                    .to_string(),
            );
        }
        let resolved_job_id = self.resolve_job_reference(job_id)?;
        let normalized_capabilities = match capabilities {
            Some(capabilities) => {
                Some(self.set_running_service_capabilities(&resolved_job_id, &capabilities)?)
            }
            None => None,
        };
        let normalized_label = match label {
            Some(label) => Some(self.set_running_service_label(&resolved_job_id, label)?),
            None => None,
        };
        let normalized_protocol = protocol
            .clone()
            .map(normalize_service_label_update)
            .transpose()?;
        let normalized_endpoint = endpoint
            .clone()
            .map(normalize_service_label_update)
            .transpose()?;
        let normalized_attach_hint = attach_hint
            .clone()
            .map(normalize_service_label_update)
            .transpose()?;
        let normalized_ready_pattern = ready_pattern
            .clone()
            .map(normalize_service_label_update)
            .transpose()?;
        let recipe_count = interaction_recipes.as_ref().map(Vec::len);
        if protocol.is_some()
            || endpoint.is_some()
            || attach_hint.is_some()
            || ready_pattern.is_some()
            || interaction_recipes.is_some()
        {
            self.set_running_service_contract(
                &resolved_job_id,
                protocol,
                endpoint,
                attach_hint,
                ready_pattern,
                interaction_recipes,
            )?;
        }
        Ok(render_service_metadata_update_summary(
            &resolved_job_id,
            normalized_capabilities.as_deref(),
            normalized_label,
            normalized_protocol,
            normalized_endpoint,
            normalized_attach_hint,
            normalized_ready_pattern,
            recipe_count,
        ))
    }

    pub(crate) fn update_dependencies_from_tool(
        &self,
        arguments: &serde_json::Value,
    ) -> Result<String, String> {
        let object = arguments.as_object().ok_or_else(|| {
            "background_shell_update_dependencies expects an object argument".to_string()
        })?;
        let job_id = object
            .get("jobId")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| "background_shell_update_dependencies requires `jobId`".to_string())?;
        let capabilities = parse_service_capabilities_argument(
            object.get("dependsOnCapabilities"),
            "background_shell_update_dependencies",
            "dependsOnCapabilities",
        )?;
        let resolved_job_id = self.resolve_job_reference(job_id)?;
        let normalized =
            self.set_running_dependency_capabilities(&resolved_job_id, &capabilities)?;
        Ok(render_dependency_capability_update_summary(
            &resolved_job_id,
            &normalized,
        ))
    }

    pub(crate) fn terminate_running_blockers_by_capability(
        &self,
        capability: &str,
    ) -> Result<usize, String> {
        let capability = validate_service_capability(capability.trim_start_matches('@'))?;
        let job_ids = self
            .snapshots()
            .into_iter()
            .filter(|job| {
                job.status == "running"
                    && job.intent == BackgroundShellIntent::Prerequisite
                    && job
                        .dependency_capabilities
                        .iter()
                        .any(|entry| entry == &capability)
            })
            .map(|job| job.id)
            .collect::<Vec<_>>();
        if job_ids.is_empty() {
            return Err(format!(
                "unknown running blocker capability `@{capability}`; use /ps dependencies @{capability} to inspect capability-scoped blockers"
            ));
        }
        Ok(terminate_jobs(self, job_ids))
    }

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

fn normalize_service_capabilities(capabilities: &[String]) -> Result<Vec<String>, String> {
    let mut normalized = capabilities
        .iter()
        .map(|capability| validate_service_capability(capability.trim_start_matches('@')))
        .collect::<Result<Vec<_>, _>>()?;
    normalized.sort();
    normalized.dedup();
    Ok(normalized)
}

fn normalize_service_label_update(label: Option<String>) -> Result<Option<String>, String> {
    match label {
        Some(label) => {
            let trimmed = label.trim();
            if trimmed.is_empty() {
                Err("service label cannot be empty".to_string())
            } else {
                Ok(Some(trimmed.to_string()))
            }
        }
        None => Ok(None),
    }
}

fn parse_service_capabilities_argument(
    value: Option<&serde_json::Value>,
    context: &str,
    field_name: &str,
) -> Result<Vec<String>, String> {
    let value = value.ok_or_else(|| format!("{context} requires `{field_name}`"))?;
    let array = value
        .as_array()
        .ok_or_else(|| format!("{context} `{field_name}` must be an array"))?;
    let raw = array
        .iter()
        .enumerate()
        .map(|(index, value)| {
            value
                .as_str()
                .map(str::to_string)
                .ok_or_else(|| format!("{context} `{field_name}[{index}]` must be a string"))
        })
        .collect::<Result<Vec<_>, _>>()?;
    normalize_service_capabilities(&raw)
}

fn parse_service_label_argument(
    value: Option<&serde_json::Value>,
    context: &str,
) -> Result<Option<String>, String> {
    match value {
        Some(serde_json::Value::Null) => Ok(None),
        Some(serde_json::Value::String(label)) => {
            normalize_service_label_update(Some(label.to_string()))
        }
        Some(_) => Err(format!("{context} `label` must be a string or null")),
        None => Err(format!("{context} requires `label`")),
    }
}

fn parse_service_recipe_updates(
    value: Option<&serde_json::Value>,
    context: &str,
) -> Result<Vec<BackgroundShellInteractionRecipe>, String> {
    match value {
        Some(serde_json::Value::Null) => Ok(Vec::new()),
        Some(value) => parse_background_shell_interaction_recipes(Some(value))
            .map_err(|err| format!("{context}: {err}")),
        None => Err(format!("{context} requires `recipes`")),
    }
}

fn render_service_metadata_update_summary(
    job_id: &str,
    capabilities: Option<&[String]>,
    label: Option<Option<String>>,
    protocol: Option<Option<String>>,
    endpoint: Option<Option<String>>,
    attach_hint: Option<Option<String>>,
    ready_pattern: Option<Option<String>>,
    recipe_count: Option<usize>,
) -> String {
    let mut parts = Vec::new();
    if let Some(capabilities) = capabilities {
        if capabilities.is_empty() {
            parts.push("cleared reusable capabilities".to_string());
        } else {
            parts.push(format!(
                "reusable capabilities={}",
                capabilities
                    .iter()
                    .map(|capability| format!("@{capability}"))
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
    }
    if let Some(label) = label {
        match label {
            Some(label) => parts.push(format!("label={label}")),
            None => parts.push("cleared label".to_string()),
        }
    }
    if let Some(protocol) = protocol {
        match protocol {
            Some(protocol) => parts.push(format!("protocol={protocol}")),
            None => parts.push("cleared protocol".to_string()),
        }
    }
    if let Some(endpoint) = endpoint {
        match endpoint {
            Some(endpoint) => parts.push(format!("endpoint={endpoint}")),
            None => parts.push("cleared endpoint".to_string()),
        }
    }
    if let Some(attach_hint) = attach_hint {
        match attach_hint {
            Some(attach_hint) => parts.push(format!("attachHint={attach_hint}")),
            None => parts.push("cleared attachHint".to_string()),
        }
    }
    if let Some(ready_pattern) = ready_pattern {
        match ready_pattern {
            Some(ready_pattern) => parts.push(format!("readyPattern={ready_pattern}")),
            None => parts.push("cleared readyPattern".to_string()),
        }
    }
    if let Some(recipe_count) = recipe_count {
        if recipe_count == 0 {
            parts.push("cleared recipes".to_string());
        } else {
            parts.push(format!("recipes={recipe_count}"));
        }
    }

    if parts.is_empty() {
        format!("No service metadata changed for background shell job {job_id}.")
    } else {
        format!(
            "Updated service metadata for background shell job {job_id}: {}.",
            parts.join("; ")
        )
    }
}

fn render_dependency_capability_update_summary(job_id: &str, capabilities: &[String]) -> String {
    if capabilities.is_empty() {
        format!("Cleared dependency capabilities for background shell job {job_id}.")
    } else {
        format!(
            "Updated dependency capabilities for background shell job {job_id}: {}",
            capabilities
                .iter()
                .map(|capability| format!("@{capability}"))
                .collect::<Vec<_>>()
                .join(", ")
        )
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
