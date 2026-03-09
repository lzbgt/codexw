use crate::background_shells::BackgroundShellInteractionRecipe;
use crate::background_shells::BackgroundShellManager;

use super::super::super::helpers::normalize_service_label_update;
use super::super::super::helpers::render_service_metadata_update_summary;

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
}
