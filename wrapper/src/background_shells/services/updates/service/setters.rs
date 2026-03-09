use super::super::super::super::BackgroundShellIntent;
use super::super::super::super::BackgroundShellInteractionRecipe;
use super::super::super::super::BackgroundShellJobStatus;
use super::super::super::super::BackgroundShellManager;
use super::super::helpers::normalize_service_label_update;

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
        let normalized = super::super::helpers::normalize_service_capabilities(capabilities)?;
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
}
