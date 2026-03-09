use super::super::super::index::dependency_consumer_display;
use super::super::super::index::provider_display;
use crate::background_shells::BackgroundShellCapabilityIssueClass;
use crate::background_shells::BackgroundShellManager;

impl BackgroundShellManager {
    pub(crate) fn render_service_capabilities_for_ps(&self) -> Option<Vec<String>> {
        self.render_service_capabilities_for_ps_filtered(None)
    }

    pub(crate) fn render_service_capabilities_for_ps_filtered(
        &self,
        issue_filter: Option<BackgroundShellCapabilityIssueClass>,
    ) -> Option<Vec<String>> {
        let entries = self
            .service_capability_index()
            .iter()
            .map(|(capability, _)| capability.clone())
            .chain(
                self.capability_dependency_summaries()
                    .into_iter()
                    .map(|summary| summary.capability),
            )
            .collect::<std::collections::BTreeSet<_>>()
            .into_iter()
            .map(|capability| {
                let issue = self
                    .service_capability_issue_for_ref(&capability)
                    .expect("issue class");
                let consumers = self
                    .capability_dependency_summaries()
                    .into_iter()
                    .filter(|summary| summary.capability == capability)
                    .map(|summary| {
                        format!(
                            "{} [{}]",
                            dependency_consumer_display(&summary),
                            summary.status.as_str()
                        )
                    })
                    .collect::<Vec<_>>();
                (capability, issue, consumers)
            })
            .filter(|(_, issue, _)| issue_filter.is_none_or(|wanted| *issue == wanted))
            .collect::<Vec<_>>();
        if entries.is_empty() {
            return None;
        }
        let mut lines = vec!["Service capability index:".to_string()];
        for (index, (capability, issue, consumers)) in entries.iter().enumerate() {
            let providers = self
                .running_service_providers_for_capability(capability)
                .into_iter()
                .map(|provider| provider_display(&provider))
                .collect::<Vec<_>>();
            let qualifier = match issue {
                BackgroundShellCapabilityIssueClass::Healthy => "",
                BackgroundShellCapabilityIssueClass::Missing => " [missing]",
                BackgroundShellCapabilityIssueClass::Booting => " [booting]",
                BackgroundShellCapabilityIssueClass::Untracked => " [untracked]",
                BackgroundShellCapabilityIssueClass::Ambiguous => " [conflict]",
            };
            lines.push(format!(
                "{:>2}. @{} -> {}{}",
                index + 1,
                capability,
                if providers.is_empty() {
                    "<missing provider>".to_string()
                } else {
                    providers.join(", ")
                },
                qualifier
            ));
            if !consumers.is_empty() {
                lines.push(format!("    used by {}", consumers.join(", ")));
            }
        }
        lines.push(
            "Use @capability with :ps poll|send|attach|wait|run|terminate to target a reusable service."
                .to_string(),
        );
        Some(lines)
    }
}
