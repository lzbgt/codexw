use crate::background_shells::BackgroundShellManager;

impl BackgroundShellManager {
    #[cfg(test)]
    pub(crate) fn list_from_tool(&self) -> String {
        let snapshots = self.snapshots();
        if snapshots.is_empty() {
            return "No background shell jobs.".to_string();
        }
        let mut lines = vec!["Background shell jobs:".to_string()];
        for snapshot in snapshots {
            let mut line = format!(
                "{}  {}  intent={}  pid={}  {}",
                snapshot.id,
                snapshot.status,
                snapshot.intent.as_str(),
                snapshot.pid,
                snapshot.command
            );
            if let Some(exit_code) = snapshot.exit_code {
                line.push_str(&format!("  exit={exit_code}"));
            }
            if let Some(label) = snapshot.label.as_deref() {
                line.push_str(&format!("  label={label}"));
            }
            if let Some(alias) = snapshot.alias.as_deref() {
                line.push_str(&format!("  alias={alias}"));
            }
            if !snapshot.service_capabilities.is_empty() {
                line.push_str(&format!(
                    "  caps={}",
                    snapshot.service_capabilities.join(",")
                ));
            }
            if !snapshot.dependency_capabilities.is_empty() {
                line.push_str(&format!(
                    "  depends={}",
                    snapshot
                        .dependency_capabilities
                        .iter()
                        .map(|capability| format!("@{capability}"))
                        .collect::<Vec<_>>()
                        .join(",")
                ));
            }
            if let Some(protocol) = snapshot.service_protocol.as_deref() {
                line.push_str(&format!("  protocol={protocol}"));
            }
            if let Some(endpoint) = snapshot.service_endpoint.as_deref() {
                line.push_str(&format!("  endpoint={endpoint}"));
            }
            if !snapshot.interaction_recipes.is_empty() {
                line.push_str(&format!("  recipes={}", snapshot.interaction_recipes.len()));
            }
            if let Some(service_readiness) = snapshot.service_readiness {
                line.push_str(&format!("  service={}", service_readiness.as_str()));
            }
            if let Some(source_thread_id) = snapshot.origin.source_thread_id.as_deref() {
                line.push_str(&format!("  source={source_thread_id}"));
            }
            if snapshot.status == "failed" && !snapshot.recent_lines.is_empty() {
                line.push_str(&format!("  {}", snapshot.recent_lines.join(" | ")));
            }
            lines.push(line);
        }
        lines.push(
            "Use background_shell_poll with a jobId to inspect logs while continuing other work."
                .to_string(),
        );
        if let Some(capability_lines) = self.render_service_capability_index_lines() {
            lines.extend(capability_lines);
        }
        let conflicts = self.service_capability_conflicts();
        if !conflicts.is_empty() {
            lines.push("Capability conflicts:".to_string());
            for (capability, jobs) in conflicts {
                lines.push(format!("@{capability} -> {}", jobs.join(", ")));
            }
        }
        lines.join("\n")
    }
}
