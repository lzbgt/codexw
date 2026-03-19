use crate::background_shells::BackgroundShellManager;
#[cfg(test)]
use crate::background_shells::BackgroundShellServiceIssueClass;
#[cfg(test)]
use crate::background_shells::validate_service_capability;

impl BackgroundShellManager {
    #[cfg(test)]
    pub(crate) fn list_services_from_tool(
        &self,
        arguments: &serde_json::Value,
    ) -> Result<String, String> {
        let object = arguments.as_object();
        let issue_filter = parse_service_issue_filter(
            object
                .and_then(|object| object.get("status"))
                .and_then(serde_json::Value::as_str),
            "background_shell_list_services",
        )?;
        let capability_filter = object
            .and_then(|object| object.get("capability"))
            .and_then(serde_json::Value::as_str);
        if let Some(capability) = capability_filter {
            validate_service_capability(capability.trim_start_matches('@'))?;
        }
        self.render_service_shells_for_ps_filtered(issue_filter, capability_filter)
            .map(|lines| lines.join("\n"))
            .ok_or_else(|| "No service shells tracked right now.".to_string())
    }
}

#[cfg(test)]
pub(crate) fn parse_service_issue_filter(
    raw: Option<&str>,
    context: &str,
) -> Result<Option<BackgroundShellServiceIssueClass>, String> {
    match raw {
        None | Some("all") => Ok(None),
        Some("ready") | Some("healthy") => Ok(Some(BackgroundShellServiceIssueClass::Ready)),
        Some("booting") => Ok(Some(BackgroundShellServiceIssueClass::Booting)),
        Some("untracked") => Ok(Some(BackgroundShellServiceIssueClass::Untracked)),
        Some("conflicts") | Some("conflict") | Some("ambiguous") => {
            Ok(Some(BackgroundShellServiceIssueClass::Conflicts))
        }
        Some(other) => Err(format!(
            "{context} `status` must be one of `all`, `ready`, `booting`, `untracked`, or `conflicts`, got `{other}`"
        )),
    }
}
