use std::time::Duration;
use std::time::Instant;

use crate::background_shells::BackgroundShellIntent;
use crate::background_shells::BackgroundShellJobStatus;
use crate::background_shells::BackgroundShellManager;
use crate::background_shells::BackgroundShellReadyWaitOutcome;
use crate::background_shells::BackgroundShellServiceReadiness;
use crate::background_shells::READY_WAIT_POLL_INTERVAL_MS;
use crate::background_shells::service_readiness_for_state;

impl BackgroundShellManager {
    pub(crate) fn wait_for_service_ready(
        &self,
        job_id: &str,
        timeout_ms: u64,
    ) -> Result<BackgroundShellReadyWaitOutcome, String> {
        let start = Instant::now();
        loop {
            let job = self.lookup_job(job_id)?;
            let state = job.lock().expect("background shell job lock");
            if state.intent != BackgroundShellIntent::Service {
                return Err(format!(
                    "background shell job `{job_id}` is not a service shell"
                ));
            }
            let readiness = service_readiness_for_state(&state).expect("service readiness");
            match readiness {
                BackgroundShellServiceReadiness::Ready => {
                    let waited_ms = start.elapsed().as_millis() as u64;
                    return Ok(if waited_ms == 0 {
                        BackgroundShellReadyWaitOutcome::AlreadyReady
                    } else {
                        BackgroundShellReadyWaitOutcome::BecameReady { waited_ms }
                    });
                }
                BackgroundShellServiceReadiness::Untracked => {
                    return Err(format!(
                        "background shell job `{job_id}` does not declare a `readyPattern`; readiness is untracked"
                    ));
                }
                BackgroundShellServiceReadiness::Booting => {
                    if !matches!(state.status, BackgroundShellJobStatus::Running) {
                        return Err(format!(
                            "background shell job `{job_id}` stopped before reaching its ready pattern"
                        ));
                    }
                }
            }
            drop(state);
            let waited_ms = start.elapsed().as_millis() as u64;
            if waited_ms >= timeout_ms {
                return Err(format!(
                    "background shell job `{job_id}` did not become ready within {timeout_ms}ms"
                ));
            }
            let remaining_ms = timeout_ms.saturating_sub(waited_ms);
            std::thread::sleep(Duration::from_millis(
                READY_WAIT_POLL_INTERVAL_MS.min(remaining_ms.max(1)),
            ));
        }
    }
}
