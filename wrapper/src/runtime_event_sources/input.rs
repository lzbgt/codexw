#[path = "input/decode.rs"]
mod decode;
#[path = "input/thread.rs"]
mod thread;

#[cfg(test)]
#[path = "input/tests.rs"]
mod tests;

use std::sync::mpsc;

use crate::runtime_event_sources::AppEvent;

pub(super) fn start_stdin_thread(tx: mpsc::Sender<AppEvent>) {
    self::thread::start_stdin_thread(tx);
}
