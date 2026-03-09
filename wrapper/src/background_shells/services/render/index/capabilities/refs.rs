use crate::background_shells::BackgroundShellCapabilityDependencySummary;
use crate::background_shells::BackgroundShellCapabilityIssueClass;
use crate::background_shells::BackgroundShellJobSnapshot;

pub(crate) fn dependency_consumer_display(
    summary: &BackgroundShellCapabilityDependencySummary,
) -> String {
    if let Some(alias) = summary.job_alias.as_deref() {
        format!("{} ({alias})", summary.job_id)
    } else if let Some(label) = summary.job_label.as_deref() {
        format!("{} ({label})", summary.job_id)
    } else {
        summary.job_id.clone()
    }
}

pub(crate) fn provider_display(snapshot: &BackgroundShellJobSnapshot) -> String {
    if let Some(alias) = snapshot.alias.as_deref() {
        format!("{} ({alias})", snapshot.id)
    } else if let Some(label) = snapshot.label.as_deref() {
        format!("{} ({label})", snapshot.id)
    } else {
        snapshot.id.clone()
    }
}

pub(crate) fn parse_capability_issue_filter(
    raw: Option<&str>,
    context: &str,
) -> Result<Option<BackgroundShellCapabilityIssueClass>, String> {
    match raw {
        None | Some("all") => Ok(None),
        Some("healthy") | Some("ok") => Ok(Some(BackgroundShellCapabilityIssueClass::Healthy)),
        Some("missing") => Ok(Some(BackgroundShellCapabilityIssueClass::Missing)),
        Some("booting") => Ok(Some(BackgroundShellCapabilityIssueClass::Booting)),
        Some("untracked") => Ok(Some(BackgroundShellCapabilityIssueClass::Untracked)),
        Some("ambiguous") | Some("conflicts") | Some("conflict") => {
            Ok(Some(BackgroundShellCapabilityIssueClass::Ambiguous))
        }
        Some(other) => Err(format!(
            "{context} `status` must be one of `all`, `healthy`, `missing`, `booting`, `untracked`, or `ambiguous`, got `{other}`"
        )),
    }
}
