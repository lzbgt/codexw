use serde_json::Value;

use crate::background_shells::BackgroundShellManager;
use crate::local_api::LocalApiSnapshot;
use crate::local_api::snapshot::local_api_shell_job;

pub(super) fn resolve_shell_snapshot(
    snapshot: &LocalApiSnapshot,
    reference: &str,
) -> std::result::Result<
    crate::local_api::snapshot::LocalApiBackgroundShellJob,
    (&'static str, &'static str),
> {
    if let Some(shell) = snapshot
        .workers
        .background_shells
        .iter()
        .find(|shell| shell.id == reference)
    {
        return Ok(shell.clone());
    }
    if let Some(shell) = snapshot
        .workers
        .background_shells
        .iter()
        .find(|shell| shell.alias.as_deref() == Some(reference))
    {
        return Ok(shell.clone());
    }
    if let Some(capability) = reference.strip_prefix('@') {
        let matches: Vec<_> = snapshot
            .capabilities
            .iter()
            .filter(|entry| entry.capability.trim_start_matches('@') == capability)
            .flat_map(|entry| entry.providers.iter())
            .filter_map(|provider| {
                snapshot
                    .workers
                    .background_shells
                    .iter()
                    .find(|shell| shell.id == provider.job_id)
                    .cloned()
            })
            .collect();
        return match matches.as_slice() {
            [shell] => Ok(shell.clone()),
            [] => Err(("shell_not_found", "unknown shell reference")),
            _ => Err(("shell_reference_ambiguous", "shell reference is ambiguous")),
        };
    }
    if let Ok(index) = reference.parse::<usize>() {
        if index == 0 {
            return Err(("validation_error", "shell index must be 1-based"));
        }
        if let Some(shell) = snapshot.workers.background_shells.get(index - 1) {
            return Ok(shell.clone());
        }
    }
    Err(("shell_not_found", "unknown shell reference"))
}

pub(super) fn current_shell_value(
    background_shells: &BackgroundShellManager,
    shell_id: &str,
) -> Option<Value> {
    background_shells
        .snapshots()
        .into_iter()
        .find(|snapshot| snapshot.id == shell_id)
        .map(local_api_shell_job)
        .and_then(|shell| serde_json::to_value(shell).ok())
}
