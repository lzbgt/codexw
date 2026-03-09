use super::super::super::index::dependency_consumer_display;
use super::super::super::index::provider_display;
use crate::background_shells::BackgroundShellManager;
use crate::background_shells::BackgroundShellServiceReadiness;
use crate::background_shells::validate_service_capability;

impl BackgroundShellManager {
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
