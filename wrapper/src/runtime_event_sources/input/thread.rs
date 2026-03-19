use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use crate::runtime_event_sources::AppEvent;

use super::decode::TerminalEventDecoder;
use super::decode::dispatch_app_events;

const INPUT_RETRY_DELAY: Duration = Duration::from_millis(10);

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
                                Err(err) if should_retry_input_error(&err) => {
                                    thread::sleep(INPUT_RETRY_DELAY);
                                    continue;
                                }
                                Err(err) => {
                                    eprintln!("[session] terminal input reader failed: {err}");
                                    let _ = tx.send(AppEvent::StdinClosed);
                                    return;
                                }
                            },
                            Ok(false) => {
                                dispatch_app_events(&tx, decoder.flush_pending());
                                break;
                            }
                            Err(err) if should_retry_input_error(&err) => {
                                thread::sleep(INPUT_RETRY_DELAY);
                                continue;
                            }
                            Err(err) => {
                                eprintln!("[session] terminal input poll failed: {err}");
                                let _ = tx.send(AppEvent::StdinClosed);
                                return;
                            }
                        }
                    }
                }
                Err(err) if should_retry_input_error(&err) => {
                    thread::sleep(INPUT_RETRY_DELAY);
                }
                Err(err) => {
                    eprintln!("[session] terminal input read failed: {err}");
                    let _ = tx.send(AppEvent::StdinClosed);
                    break;
                }
            }
        }
    });
}

fn should_retry_input_error(err: &std::io::Error) -> bool {
    matches!(
        err.kind(),
        std::io::ErrorKind::Interrupted
            | std::io::ErrorKind::WouldBlock
            | std::io::ErrorKind::TimedOut
    ) || matches!(err.raw_os_error(), Some(4 | 11 | 35))
}

#[cfg(test)]
mod tests {
    use super::should_retry_input_error;

    #[test]
    fn retryable_input_errors_cover_transient_kinds() {
        assert!(should_retry_input_error(&std::io::Error::from(
            std::io::ErrorKind::Interrupted,
        )));
        assert!(should_retry_input_error(&std::io::Error::from(
            std::io::ErrorKind::WouldBlock,
        )));
        assert!(should_retry_input_error(&std::io::Error::from(
            std::io::ErrorKind::TimedOut,
        )));
    }

    #[test]
    fn retryable_input_errors_cover_common_terminal_errno_values() {
        assert!(should_retry_input_error(
            &std::io::Error::from_raw_os_error(4)
        ));
        assert!(should_retry_input_error(
            &std::io::Error::from_raw_os_error(11)
        ));
        assert!(should_retry_input_error(
            &std::io::Error::from_raw_os_error(35)
        ));
        assert!(!should_retry_input_error(
            &std::io::Error::from_raw_os_error(9)
        ));
    }
}
