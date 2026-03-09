use crate::background_shells::BackgroundShellIntent;
use crate::background_shells::BackgroundShellManager;
use crate::background_shells::terminate_jobs;
use crate::background_shells::validate_service_capability;

impl BackgroundShellManager {
    pub(crate) fn terminate_running_blockers_by_capability(
        &self,
        capability: &str,
    ) -> Result<usize, String> {
        let capability = validate_service_capability(capability.trim_start_matches('@'))?;
        let job_ids = self
            .snapshots()
            .into_iter()
            .filter(|job| {
                job.status == "running"
                    && job.intent == BackgroundShellIntent::Prerequisite
                    && job
                        .dependency_capabilities
                        .iter()
                        .any(|entry| entry == &capability)
            })
            .map(|job| job.id)
            .collect::<Vec<_>>();
        if job_ids.is_empty() {
            return Err(format!(
                "unknown running blocker capability `@{capability}`; use :ps dependencies @{capability} to inspect capability-scoped blockers"
            ));
        }
        Ok(terminate_jobs(self, job_ids))
    }

    pub(crate) fn terminate_running_services_by_capability(
        &self,
        capability: &str,
    ) -> Result<usize, String> {
        let capability = validate_service_capability(capability.trim_start_matches('@'))?;
        let job_ids = self
            .running_service_snapshots()
            .into_iter()
            .filter(|job| {
                job.service_capabilities
                    .iter()
                    .any(|entry| entry == &capability)
            })
            .map(|job| job.id)
            .collect::<Vec<_>>();
        if job_ids.is_empty() {
            return Err(format!(
                "unknown running background shell capability `@{capability}`; use :ps capabilities to inspect reusable service roles"
            ));
        }
        Ok(terminate_jobs(self, job_ids))
    }
}
