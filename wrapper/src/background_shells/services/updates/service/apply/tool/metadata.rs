use crate::background_shells::BackgroundShellManager;
use crate::background_shells::parse_service_capabilities_argument;
use crate::background_shells::parse_service_string_update_argument;
use crate::background_shells::render_service_metadata_update_summary;

impl BackgroundShellManager {
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

        let interaction_recipes = super::contract::parse_service_recipes_for_tool(object)?;
        super::contract::require_service_update_fields(
            capabilities.is_some(),
            label.is_some(),
            protocol.is_some(),
            endpoint.is_some(),
            attach_hint.is_some(),
            ready_pattern.is_some(),
            interaction_recipes.is_some(),
        )?;

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
        let (normalized_protocol, normalized_endpoint, normalized_attach_hint, normalized_ready_pattern, recipe_count) =
            super::contract::apply_service_contract_updates(
                self,
                &resolved_job_id,
                protocol,
                endpoint,
                attach_hint,
                ready_pattern,
                interaction_recipes,
            )?;

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
}
