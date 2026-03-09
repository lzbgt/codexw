use super::super::super::super::BackgroundShellIntent;
use super::super::super::super::BackgroundShellManager;
use super::super::super::super::recipes::interaction_action_summary;
use super::super::super::super::recipes::render_recipe_parameters;
use super::super::super::super::service_readiness_for_state;
use super::super::process::validate_service_capability;

impl BackgroundShellManager {
    pub(crate) fn service_attachment_summary(&self, job_id: &str) -> Result<String, String> {
        let job = self.lookup_job(job_id)?;
        let state = job.lock().expect("background shell job lock");
        if state.intent != BackgroundShellIntent::Service {
            return Err(format!(
                "background shell job `{job_id}` is not a service shell"
            ));
        }
        let mut lines = vec![
            format!("Service job: {}", state.id),
            format!(
                "State: {}",
                service_readiness_for_state(&state)
                    .expect("service readiness")
                    .as_str()
            ),
            format!("Command: {}", state.command),
        ];
        if let Some(label) = state.label.as_deref() {
            lines.push(format!("Label: {label}"));
        }
        if let Some(alias) = state.alias.as_deref() {
            lines.push(format!("Alias: {alias}"));
        }
        if !state.service_capabilities.is_empty() {
            lines.push(format!(
                "Capabilities: {}",
                state.service_capabilities.join(", ")
            ));
        }
        if let Some(protocol) = state.service_protocol.as_deref() {
            lines.push(format!("Protocol: {protocol}"));
        }
        if let Some(endpoint) = state.service_endpoint.as_deref() {
            lines.push(format!("Endpoint: {endpoint}"));
        }
        if let Some(attach_hint) = state.attach_hint.as_deref() {
            lines.push(format!("Attach hint: {attach_hint}"));
        }
        if !state.interaction_recipes.is_empty() {
            lines.push("Recipes:".to_string());
            for recipe in &state.interaction_recipes {
                let mut line = format!(
                    "- {} [{}]",
                    recipe.name,
                    interaction_action_summary(&recipe.action)
                );
                if let Some(description) = recipe.description.as_deref() {
                    line.push_str(&format!(": {description}"));
                }
                lines.push(line);
                if !recipe.parameters.is_empty() {
                    lines.push(format!(
                        "  params: {}",
                        render_recipe_parameters(&recipe.parameters)
                    ));
                }
                if let Some(example) = recipe.example.as_deref() {
                    lines.push(format!("  example: {example}"));
                }
            }
        }
        if let Some(ready_pattern) = state.ready_pattern.as_deref() {
            lines.push(format!("Ready pattern: {ready_pattern}"));
        }
        if state.service_capabilities.is_empty()
            && state.service_protocol.is_none()
            && state.service_endpoint.is_none()
            && state.attach_hint.is_none()
            && state.interaction_recipes.is_empty()
        {
            lines.push(
                "No explicit service attachment metadata has been declared for this job."
                    .to_string(),
            );
        }
        Ok(lines.join("\n"))
    }

    pub(crate) fn resolve_service_capability_reference(
        &self,
        capability: &str,
    ) -> Result<String, String> {
        let capability = validate_service_capability(capability)?;
        let matches = self
            .running_service_snapshots()
            .into_iter()
            .filter(|job| {
                job.service_capabilities
                    .iter()
                    .any(|entry| entry == &capability)
            })
            .collect::<Vec<_>>();
        match matches.as_slice() {
            [] => Err(format!(
                "unknown running background shell capability `@{capability}`; use :ps capabilities to inspect reusable service roles"
            )),
            [job] => Ok(job.id.clone()),
            jobs => {
                let refs = jobs
                    .iter()
                    .map(|job| match job.alias.as_deref() {
                        Some(alias) => format!("{} ({alias})", job.id),
                        None => job.id.clone(),
                    })
                    .collect::<Vec<_>>()
                    .join(", ");
                Err(format!(
                    "background shell capability `@{capability}` is ambiguous across multiple running service jobs: {refs}; use :ps capabilities to inspect reusable service roles"
                ))
            }
        }
    }
}
