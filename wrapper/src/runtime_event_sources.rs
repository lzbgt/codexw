use std::io::BufRead;
use std::io::BufReader;
use std::process::ChildStderr;
use std::process::ChildStdout;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use crate::rpc::RequestId;

#[path = "runtime_event_sources/input.rs"]
mod input;
#[path = "runtime_event_sources/terminal.rs"]
mod terminal;

pub(crate) use terminal::RawModeGuard;

#[allow(dead_code)]
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct AsyncToolResponse {
    pub(crate) id: RequestId,
    pub(crate) tool: String,
    pub(crate) summary: String,
    pub(crate) result: serde_json::Value,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum AppEvent {
    InputKey(crate::runtime_keys::InputKey),
    Tick,
    #[allow(dead_code)]
    AsyncToolResponseReady(AsyncToolResponse),
    StdinClosed,
    ServerClosed,
}

pub(crate) fn start_stdout_thread(
    stdout: ChildStdout,
    server_tx: mpsc::Sender<String>,
    control_tx: mpsc::Sender<AppEvent>,
) {
    thread::spawn(move || {
        let mut reader = BufReader::new(stdout);
        let mut line = String::new();
        loop {
            line.clear();
            match reader.read_line(&mut line) {
                Ok(0) => {
                    let _ = control_tx.send(AppEvent::ServerClosed);
                    break;
                }
                Ok(_) => {
                    let trimmed = line.trim_end_matches(['\n', '\r']).to_string();
                    if server_tx.send(trimmed).is_err() {
                        break;
                    }
                }
                Err(_) => {
                    let _ = control_tx.send(AppEvent::ServerClosed);
                    break;
                }
            }
        }
    });
}

const ROLLOUT_QUEUE_CLOSED_FRAGMENT: &str =
    "failed to record rollout items: failed to queue rollout items: channel closed";

#[derive(Default)]
struct AppServerStderrFilter {
    suppressed_rollout_queue_closed_count: usize,
}

impl AppServerStderrFilter {
    fn observe_line(&mut self, line: &str) -> Vec<String> {
        let mut visible = Vec::new();
        if line.contains(ROLLOUT_QUEUE_CLOSED_FRAGMENT) {
            self.suppressed_rollout_queue_closed_count += 1;
            if self.suppressed_rollout_queue_closed_count == 1 {
                visible.push(
                    "[session] app-server rollout recorder queue is already closed; suppressing repeated rollout write errors"
                        .to_string(),
                );
            }
            return visible;
        }

        self.flush(&mut visible);
        visible.push(line.to_string());
        visible
    }

    fn finish(mut self) -> Vec<String> {
        let mut visible = Vec::new();
        self.flush(&mut visible);
        visible
    }

    fn flush(&mut self, visible: &mut Vec<String>) {
        if self.suppressed_rollout_queue_closed_count > 1 {
            visible.push(format!(
                "[session] suppressed {} repeated app-server rollout queue closed error(s)",
                self.suppressed_rollout_queue_closed_count - 1
            ));
        }
        self.suppressed_rollout_queue_closed_count = 0;
    }
}

pub(crate) fn start_stderr_thread(stderr: ChildStderr) {
    thread::spawn(move || {
        let reader = BufReader::new(stderr);
        let mut filter = AppServerStderrFilter::default();
        for line in reader.lines() {
            match line {
                Ok(line) => {
                    for visible in filter.observe_line(&line) {
                        eprintln!("{visible}");
                    }
                }
                Err(err) => {
                    for visible in filter.finish() {
                        eprintln!("{visible}");
                    }
                    eprintln!("[session] failed to read codex app-server stderr: {err}");
                    return;
                }
            }
        }
        for visible in filter.finish() {
            eprintln!("{visible}");
        }
    });
}

pub(crate) fn start_stdin_thread(tx: mpsc::Sender<AppEvent>) {
    input::start_stdin_thread(tx);
}

pub(crate) fn start_tick_thread(tx: mpsc::Sender<AppEvent>) {
    thread::spawn(move || {
        loop {
            thread::sleep(Duration::from_millis(200));
            if tx.send(AppEvent::Tick).is_err() {
                break;
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::AppServerStderrFilter;

    #[test]
    fn stderr_filter_suppresses_repeated_rollout_queue_closed_errors() {
        let mut filter = AppServerStderrFilter::default();

        let first = filter.observe_line("2026-03-15 ERROR failed to record rollout items: failed to queue rollout items: channel closed");
        let second = filter.observe_line("2026-03-15 ERROR failed to record rollout items: failed to queue rollout items: channel closed");
        let third = filter.observe_line("2026-03-15 ERROR failed to record rollout items: failed to queue rollout items: channel closed");
        let next = filter.observe_line("2026-03-15 INFO recovered");

        assert_eq!(
            first,
            vec![
                "[session] app-server rollout recorder queue is already closed; suppressing repeated rollout write errors"
                    .to_string()
            ]
        );
        assert!(second.is_empty());
        assert!(third.is_empty());
        assert_eq!(
            next,
            vec![
                "[session] suppressed 2 repeated app-server rollout queue closed error(s)"
                    .to_string(),
                "2026-03-15 INFO recovered".to_string()
            ]
        );
    }
}
