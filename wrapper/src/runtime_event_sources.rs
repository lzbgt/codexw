use std::io::BufRead;
use std::io::BufReader;
use std::process::ChildStdout;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use anyhow::Context;
use anyhow::Result;
use crossterm::event::Event;
use crossterm::event::KeyEventKind;
use crossterm::terminal;

use crate::runtime_input::InputKey;
use crate::runtime_input::map_key_event;

pub(crate) enum AppEvent {
    ServerLine(String),
    InputKey(InputKey),
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
    thread::spawn(move || {
        loop {
            match crossterm::event::read() {
                Ok(Event::Key(key_event)) => {
                    if !matches!(key_event.kind, KeyEventKind::Press | KeyEventKind::Repeat) {
                        continue;
                    }
                    if let Some(key) = map_key_event(key_event) {
                        let _ = tx.send(AppEvent::InputKey(key));
                    }
                }
                Ok(_) => {}
                Err(_) => {
                    let _ = tx.send(AppEvent::StdinClosed);
                    break;
                }
            }
        }
    });
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

pub(crate) struct RawModeGuard;

impl RawModeGuard {
    pub(crate) fn new() -> Result<Self> {
        terminal::enable_raw_mode().context("enable raw terminal mode")?;
        Ok(Self)
    }
}

impl Drop for RawModeGuard {
    fn drop(&mut self) {
        let _ = terminal::disable_raw_mode();
    }
}
