use std::io::BufRead;
use std::io::BufReader;
use std::process::ChildStdout;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

#[path = "runtime_event_sources/input.rs"]
mod input;
#[path = "runtime_event_sources/terminal.rs"]
mod terminal;

pub(crate) use terminal::RawModeGuard;

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum AppEvent {
    ServerLine(String),
    InputKey(crate::runtime_keys::InputKey),
    Tick,
    StdinClosed,
    ServerClosed,
}

pub(crate) fn start_stdout_thread(stdout: ChildStdout, tx: mpsc::Sender<AppEvent>) {
    thread::spawn(move || {
        let mut reader = BufReader::new(stdout);
        let mut line = String::new();
        loop {
            line.clear();
            match reader.read_line(&mut line) {
                Ok(0) => {
                    let _ = tx.send(AppEvent::ServerClosed);
                    break;
                }
                Ok(_) => {
                    let trimmed = line.trim_end_matches(['\n', '\r']).to_string();
                    let _ = tx.send(AppEvent::ServerLine(trimmed));
                }
                Err(_) => {
                    let _ = tx.send(AppEvent::ServerClosed);
                    break;
                }
            }
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
