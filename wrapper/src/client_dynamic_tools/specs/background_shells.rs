use serde_json::Value;

#[path = "background_shells/jobs.rs"]
mod jobs;
#[path = "background_shells/services.rs"]
mod services;

pub(crate) fn background_shell_tool_specs() -> Vec<Value> {
    let mut specs = jobs::interactive_job_tool_specs();
    specs.extend(services::service_tool_specs());
    specs.extend(jobs::cleanup_job_tool_specs());
    specs
}
