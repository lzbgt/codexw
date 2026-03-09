use super::super::super::BackgroundShellIntent;
use super::super::super::BackgroundShellInteractionRecipe;
use super::super::super::BackgroundShellJobStatus;
use super::super::super::BackgroundShellManager;
use super::super::super::terminate_jobs;
use super::super::super::validate_service_capability;
use super::helpers::normalize_service_capabilities;
use super::helpers::normalize_service_label_update;
use super::helpers::parse_service_capabilities_argument;
use super::helpers::parse_service_recipe_updates;
use super::helpers::parse_service_string_update_argument;
use super::helpers::render_dependency_capability_update_summary;
use super::helpers::render_service_metadata_update_summary;

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
            Some(parse_service_string_update_argument(
                object.get("label"),
                "background_shell_update_service",
                "label",
            )?)
        } else {
            None
        };
        let protocol = if object.contains_key("protocol") {
            Some(parse_service_string_update_argument(
                object.get("protocol"),
                "background_shell_update_service",
                "protocol",
            )?)
        } else {
            None
        };
        let endpoint = if object.contains_key("endpoint") {
            Some(parse_service_string_update_argument(
                object.get("endpoint"),
                "background_shell_update_service",
                "endpoint",
            )?)
        } else {
            None
        };
        let attach_hint = if object.contains_key("attachHint") {
            Some(parse_service_string_update_argument(
                object.get("attachHint"),
                "background_shell_update_service",
                "attachHint",
            )?)
        } else {
            None
        };
        let ready_pattern = if object.contains_key("readyPattern") {
            Some(parse_service_string_update_argument(
                object.get("readyPattern"),
                "background_shell_update_service",
                "readyPattern",
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
}
