use std::process::Command;
use std::sync::Arc;
use std::sync::Mutex;

use super::super::terminate_jobs;
use super::types::BackgroundShellIntent;
use super::types::BackgroundShellJobState;
use super::types::BackgroundShellManager;

impl BackgroundShellManager {
    pub(crate) fn running_count(&self) -> usize {
        self.snapshots()
            .into_iter()
            .filter(|job| job.exit_code.is_none() && job.status == "running")
            .count()
    }

    pub(crate) fn running_count_by_intent(&self, intent: BackgroundShellIntent) -> usize {
        self.snapshots()
            .into_iter()
            .filter(|job| {
                job.exit_code.is_none() && job.status == "running" && job.intent == intent
            })
            .count()
    }

    pub(crate) fn job_count(&self) -> usize {
        self.inner
            .jobs
            .lock()
            .expect("background shell jobs lock")
            .len()
    }

    pub(crate) fn terminate_all_running(&self) -> usize {
        let job_ids = self
            .snapshots()
            .into_iter()
            .filter(|job| job.status == "running")
            .map(|job| job.id)
            .collect::<Vec<_>>();
        terminate_jobs(self, job_ids)
    }

    pub(crate) fn terminate_running_by_intent(&self, intent: BackgroundShellIntent) -> usize {
        let job_ids = self
            .snapshots()
            .into_iter()
            .filter(|job| job.status == "running" && job.intent == intent)
            .map(|job| job.id)
            .collect::<Vec<_>>();
        terminate_jobs(self, job_ids)
    }

    pub(crate) fn lookup_job(
        &self,
        job_id: &str,
    ) -> Result<Arc<Mutex<BackgroundShellJobState>>, String> {
        self.inner
            .jobs
            .lock()
            .expect("background shell jobs lock")
            .get(job_id)
            .cloned()
            .ok_or_else(|| format!("unknown background shell job `{job_id}`"))
    }
}

#[cfg(unix)]
pub(crate) fn terminate_pid(pid: u32) -> Result<(), String> {
    let status = Command::new("/bin/kill")
        .arg("-TERM")
        .arg(pid.to_string())
        .status()
        .map_err(|err| format!("failed to invoke kill for pid {pid}: {err}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("kill returned non-zero status for pid {pid}"))
    }
}

#[cfg(windows)]
pub(crate) fn terminate_pid(pid: u32) -> Result<(), String> {
    let status = Command::new("taskkill")
        .args(["/PID", &pid.to_string(), "/T", "/F"])
        .status()
        .map_err(|err| format!("failed to invoke taskkill for pid {pid}: {err}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("taskkill returned non-zero status for pid {pid}"))
    }
}
