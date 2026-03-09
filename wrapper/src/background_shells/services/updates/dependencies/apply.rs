use crate::background_shells::BackgroundShellJobStatus;
use crate::background_shells::BackgroundShellManager;

use super::super::helpers::normalize_service_capabilities;
use super::super::helpers::parse_service_capabilities_argument;
use super::super::helpers::render_dependency_capability_update_summary;

impl BackgroundShellManager {
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
}
