use crate::background_shells::BackgroundShellManager;
use crate::background_shells::render_service_metadata_update_summary;

impl BackgroundShellManager {
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
}
