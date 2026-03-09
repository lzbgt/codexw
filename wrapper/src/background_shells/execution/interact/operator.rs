use std::collections::HashMap;

use crate::background_shells::BackgroundShellManager;
use crate::background_shells::BackgroundShellReadyWaitOutcome;

impl BackgroundShellManager {
    pub(crate) fn poll_job(
        &self,
        job_id: &str,
        after_line: u64,
        limit: usize,
    ) -> Result<String, String> {
        self.poll_from_tool(&serde_json::json!({
            "jobId": job_id,
            "afterLine": after_line,
            "limit": limit,
        }))
    }

    pub(crate) fn send_input_for_operator(
        &self,
        job_id: &str,
        text: &str,
        append_newline: bool,
    ) -> Result<usize, String> {
        self.send_input_to_job(job_id, text, append_newline)
    }

    pub(crate) fn attach_for_operator(&self, job_id: &str) -> Result<String, String> {
        self.service_attachment_summary(job_id)
    }

    pub(crate) fn wait_ready_for_operator(
        &self,
        job_id: &str,
        timeout_ms: u64,
    ) -> Result<String, String> {
        let outcome = self.wait_for_service_ready(job_id, timeout_ms)?;
        let job = self.lookup_job(job_id)?;
        let state = job.lock().expect("background shell job lock");
        let job_label = state.alias.clone().unwrap_or_else(|| state.id.clone());
        let ready_pattern = state.ready_pattern.clone().unwrap_or_default();
        let message = match outcome {
            BackgroundShellReadyWaitOutcome::AlreadyReady => {
                format!("Service background shell job {job_label} is already ready.")
            }
            BackgroundShellReadyWaitOutcome::BecameReady { waited_ms } => format!(
                "Service background shell job {job_label} became ready after {waited_ms}ms."
            ),
        };
        Ok(format!("{message}\nReady pattern: {ready_pattern}"))
    }

    #[cfg(test)]
    pub(crate) fn invoke_recipe_for_operator(
        &self,
        job_id: &str,
        recipe_name: &str,
    ) -> Result<String, String> {
        self.invoke_recipe(job_id, recipe_name, &HashMap::new(), None)
    }

    pub(crate) fn invoke_recipe_for_operator_with_args(
        &self,
        job_id: &str,
        recipe_name: &str,
        args: &HashMap<String, String>,
    ) -> Result<String, String> {
        self.invoke_recipe(job_id, recipe_name, args, None)
    }
}
