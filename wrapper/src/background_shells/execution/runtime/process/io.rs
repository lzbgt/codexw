use std::io::BufRead;
use std::io::BufReader;
use std::sync::Arc;
use std::sync::Mutex;
use std::thread;

use super::super::super::super::BackgroundShellJobState;
use super::super::super::super::BackgroundShellJobStatus;
use super::super::super::super::BackgroundShellManager;
use super::super::super::super::BackgroundShellOutputLine;
use super::super::super::super::MAX_STORED_LINES;
use super::super::super::super::terminate_pid;

impl BackgroundShellManager {
    pub(crate) fn terminate_job(&self, job_id: &str) -> Result<(), String> {
        let job = self.lookup_job(job_id)?;
        let pid = {
            let state = job.lock().expect("background shell job lock");
            if !matches!(state.status, BackgroundShellJobStatus::Running) {
                return Ok(());
            }
            state.pid
        };
        terminate_pid(pid)?;
        let mut state = job.lock().expect("background shell job lock");
        state.status = BackgroundShellJobStatus::Terminated(None);
        state.stdin = None;
        Ok(())
    }

    pub(crate) fn send_input_to_job(
        &self,
        job_id: &str,
        text: &str,
        append_newline: bool,
    ) -> Result<usize, String> {
        let job = self.lookup_job(job_id)?;
        let mut state = job.lock().expect("background shell job lock");
        if !matches!(state.status, BackgroundShellJobStatus::Running) {
            return Err(format!("background shell job `{job_id}` is not running"));
        }
        let stdin = state
            .stdin
            .as_mut()
            .ok_or_else(|| format!("background shell job `{job_id}` is not accepting stdin"))?;
        let mut payload = text.as_bytes().to_vec();
        if append_newline {
            payload.push(b'\n');
        }
        use std::io::Write;
        stdin
            .write_all(&payload)
            .map_err(|err| format!("failed to write to background shell job `{job_id}`: {err}"))?;
        stdin.flush().map_err(|err| {
            format!("failed to flush background shell job `{job_id}` stdin: {err}")
        })?;
        Ok(payload.len())
    }
}

pub(crate) fn spawn_output_reader<R>(
    reader: R,
    job: Arc<Mutex<BackgroundShellJobState>>,
    stream_name: Option<&'static str>,
) where
    R: std::io::Read + Send + 'static,
{
    thread::spawn(move || {
        let reader = BufReader::new(reader);
        for line in reader.lines() {
            match line {
                Ok(line) => append_output_line(&job, stream_name, &line),
                Err(err) => {
                    append_output_line(
                        &job,
                        Some("stderr"),
                        &format!("background shell reader error: {err}"),
                    );
                    break;
                }
            }
        }
    });
}

fn append_output_line(
    job: &Arc<Mutex<BackgroundShellJobState>>,
    stream_name: Option<&'static str>,
    line: &str,
) {
    let text = if let Some(stream_name) = stream_name {
        format!("[{stream_name}] {line}")
    } else {
        line.to_string()
    };
    let mut state = job.lock().expect("background shell job lock");
    state.total_lines += 1;
    state.last_output_at = Some(std::time::Instant::now());
    let cursor = state.total_lines;
    if !state.service_ready
        && let Some(pattern) = state.ready_pattern.as_deref()
        && (line.contains(pattern) || text.contains(pattern))
    {
        state.service_ready = true;
    }
    state
        .lines
        .push_back(BackgroundShellOutputLine { cursor, text });
    if state.lines.len() > MAX_STORED_LINES {
        state.lines.pop_front();
    }
}

pub(crate) fn terminate_jobs(manager: &BackgroundShellManager, job_ids: Vec<String>) -> usize {
    let mut terminated = 0;
    for job_id in job_ids {
        if manager.terminate_job(&job_id).is_ok() {
            terminated += 1;
        }
    }
    terminated
}
