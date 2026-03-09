use super::super::index::dependency_consumer_display;
use super::super::index::provider_display;
use crate::background_shells::BackgroundShellCapabilityIssueClass;
use crate::background_shells::BackgroundShellManager;
use crate::background_shells::BackgroundShellServiceReadiness;
use crate::background_shells::validate_service_capability;

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

    pub(crate) fn render_single_service_capability_for_ps(
        &self,
        capability_ref: &str,
    ) -> Result<Vec<String>, String> {
        let capability = if let Some(raw) = capability_ref.strip_prefix('@') {
            validate_service_capability(raw)?
        } else {
            validate_service_capability(capability_ref)?
        };
        let providers = self.running_service_providers_for_capability(&capability);
        let consumers = self
            .capability_dependency_summaries()
            .into_iter()
            .filter(|summary| summary.capability == capability)
            .collect::<Vec<_>>();
        if providers.is_empty() && consumers.is_empty() {
            return Err(format!("unknown service capability `@{capability}`"));
        }
        let mut lines = vec![format!("Service capability: @{capability}")];
        if providers.is_empty() {
            lines.push("Providers: <missing provider>".to_string());
        } else {
            lines.push("Providers:".to_string());
            for (index, provider) in providers.iter().enumerate() {
                lines.push(format!(
                    "{:>2}. {}  [{}]",
                    index + 1,
                    provider_display(provider),
                    provider
                        .service_readiness
                        .map(BackgroundShellServiceReadiness::as_str)
                        .unwrap_or("-")
                ));
                if let Some(protocol) = provider.service_protocol.as_deref() {
                    lines.push(format!("    protocol {protocol}"));
                }
                if let Some(endpoint) = provider.service_endpoint.as_deref() {
                    lines.push(format!("    endpoint {endpoint}"));
                }
                if !provider.interaction_recipes.is_empty() {
                    lines.push(format!(
                        "    recipes  {}",
                        provider.interaction_recipes.len()
                    ));
                }
            }
            if providers.len() > 1 {
                lines.push("Conflict: ambiguous capability provider set".to_string());
            }
        }
        if consumers.is_empty() {
            lines.push("Consumers: none".to_string());
        } else {
            lines.push("Consumers:".to_string());
            for (index, consumer) in consumers.iter().enumerate() {
                let job_ref = dependency_consumer_display(consumer);
                lines.push(format!(
                    "{:>2}. {}  [{}]  blocking={}",
                    index + 1,
                    job_ref,
                    consumer.status.as_str(),
                    if consumer.blocking { "yes" } else { "no" }
                ));
            }
        }
        Ok(lines)
    }
}
