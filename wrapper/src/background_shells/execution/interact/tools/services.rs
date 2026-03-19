use crate::background_shells::BackgroundShellManager;
use crate::background_shells::parse_background_shell_timeout_ms;
#[cfg(test)]
use crate::background_shells::parse_capability_issue_filter;
use crate::background_shells::recipes::parse_recipe_arguments_map;

impl BackgroundShellManager {
    pub(crate) fn attach_from_tool(&self, arguments: &serde_json::Value) -> Result<String, String> {
        let object = arguments
            .as_object()
            .ok_or_else(|| "background_shell_attach expects an object argument".to_string())?;
        let job_id = object
            .get("jobId")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| "background_shell_attach requires `jobId`".to_string())?;
        let resolved_job_id = self.resolve_job_reference(job_id)?;
        self.service_attachment_summary(&resolved_job_id)
    }

    #[cfg(test)]
    pub(crate) fn inspect_capability_from_tool(
        &self,
        arguments: &serde_json::Value,
    ) -> Result<String, String> {
        let object = arguments.as_object().ok_or_else(|| {
            "background_shell_inspect_capability expects an object argument".to_string()
        })?;
        let capability = object
            .get("capability")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| {
                "background_shell_inspect_capability requires `capability`".to_string()
            })?;
        Ok(self
            .render_single_service_capability_for_ps(capability)?
            .join("\n"))
    }

    #[cfg(test)]
    pub(crate) fn list_capabilities_from_tool(
        &self,
        arguments: &serde_json::Value,
    ) -> Result<String, String> {
        let object = arguments.as_object();
        let issue_filter = parse_capability_issue_filter(
            object
                .and_then(|object| object.get("status"))
                .and_then(serde_json::Value::as_str),
            "background_shell_list_capabilities",
        )?;
        let rendered = self
            .render_service_capabilities_for_ps_filtered(issue_filter)
            .ok_or_else(|| "No reusable service capabilities tracked right now.".to_string())?;
        Ok(rendered.join("\n"))
    }

    pub(crate) fn wait_ready_from_tool(
        &self,
        arguments: &serde_json::Value,
    ) -> Result<String, String> {
        let object = arguments
            .as_object()
            .ok_or_else(|| "background_shell_wait_ready expects an object argument".to_string())?;
        let job_id = object
            .get("jobId")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| "background_shell_wait_ready requires `jobId`".to_string())?;
        let timeout_ms = parse_background_shell_timeout_ms(
            object.get("timeoutMs"),
            "background_shell_wait_ready",
        )?
        .unwrap_or(crate::background_shells::DEFAULT_READY_WAIT_TIMEOUT_MS);
        let resolved_job_id = self.resolve_job_reference(job_id)?;
        self.wait_ready_for_operator(&resolved_job_id, timeout_ms)
    }

    pub(crate) fn invoke_recipe_from_tool(
        &self,
        arguments: &serde_json::Value,
    ) -> Result<String, String> {
        let object = arguments.as_object().ok_or_else(|| {
            "background_shell_invoke_recipe expects an object argument".to_string()
        })?;
        let job_id = object
            .get("jobId")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| "background_shell_invoke_recipe requires `jobId`".to_string())?;
        let recipe_name = object
            .get("recipe")
            .and_then(serde_json::Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| "background_shell_invoke_recipe requires `recipe`".to_string())?;
        let wait_for_ready_ms = parse_background_shell_timeout_ms(
            object.get("waitForReadyMs"),
            "background_shell_invoke_recipe",
        )?;
        let args =
            parse_recipe_arguments_map(object.get("args"), "background_shell_invoke_recipe")?;
        let resolved_job_id = self.resolve_job_reference(job_id)?;
        self.invoke_recipe(&resolved_job_id, recipe_name, &args, wait_for_ready_ms)
    }
}
