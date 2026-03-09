use std::sync::mpsc;
use std::thread;

use crate::runtime_event_sources::AppEvent;

use super::decode::TerminalEventDecoder;
use super::decode::dispatch_app_events;

pub(super) fn start_stdin_thread(tx: mpsc::Sender<AppEvent>) {
    thread::spawn(move || {
        let mut decoder = TerminalEventDecoder::default();
        loop {
            match crossterm::event::read() {
                Ok(event) => {
                    dispatch_app_events(&tx, decoder.push(event));
                    loop {
                        match crossterm::event::poll(decoder.poll_timeout()) {
                            Ok(true) => match crossterm::event::read() {
                                Ok(event) => {
                                    dispatch_app_events(&tx, decoder.push(event));
                                }
                                Err(_) => {
                                    let _ = tx.send(AppEvent::StdinClosed);
                                    return;
                                }
                            },
                            Ok(false) => {
                                dispatch_app_events(&tx, decoder.flush_pending());
                                break;
                            }
                            Err(_) => {
                                let _ = tx.send(AppEvent::StdinClosed);
                                return;
                            }
                        }
                    }
                }
                Err(_) => {
                    let _ = tx.send(AppEvent::StdinClosed);
                    break;
                }
            }
        }
    });
}
