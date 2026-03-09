use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use crossterm::event::Event;
use crossterm::event::KeyEventKind;

use super::AppEvent;
use crate::runtime_keys::InputKey;
use crate::runtime_keys::map_key_event;

pub(crate) fn start_stdin_thread(tx: mpsc::Sender<AppEvent>) {
    thread::spawn(move || {
        let mut decoder = TerminalEventDecoder::default();
        loop {
            match crossterm::event::read() {
                Ok(event) => {
                    dispatch_app_events(&tx, decoder.push(event));
                    loop {
                        let poll_timeout = if decoder.has_pending_marker() {
                            Duration::from_millis(10)
                        } else {
                            Duration::from_millis(0)
                        };
                        match crossterm::event::poll(poll_timeout) {
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

fn dispatch_app_events(tx: &mpsc::Sender<AppEvent>, app_events: Vec<AppEvent>) {
    for app_event in app_events {
        let _ = tx.send(app_event);
    }
}

#[derive(Debug, Default)]
struct TerminalEventDecoder {
    pending_start_marker: Vec<InputKey>,
    paste: Option<BracketedPasteBuffer>,
}

#[derive(Debug, Default)]
struct BracketedPasteBuffer {
    text: String,
    pending_end_marker: Vec<InputKey>,
}

impl TerminalEventDecoder {
    fn has_pending_marker(&self) -> bool {
        !self.pending_start_marker.is_empty()
            || self
                .paste
                .as_ref()
                .is_some_and(|paste| !paste.pending_end_marker.is_empty())
    }

    fn push(&mut self, event: Event) -> Vec<AppEvent> {
        match event {
            Event::Paste(text) => {
                let mut app_events = self.flush_pending();
                self.paste = None;
                app_events.push(AppEvent::InputKey(InputKey::Paste(normalize_paste_text(
                    &text,
                ))));
                app_events
            }
            Event::Key(key_event) => {
                if !matches!(key_event.kind, KeyEventKind::Press | KeyEventKind::Repeat) {
                    return Vec::new();
                }
                let Some(key) = map_key_event(key_event) else {
                    return Vec::new();
                };
                self.push_key(key)
            }
            _ => self.flush_pending(),
        }
    }

    fn flush_pending(&mut self) -> Vec<AppEvent> {
        let mut app_events = Vec::new();

        if !self.pending_start_marker.is_empty() {
            app_events.extend(self.pending_start_marker.drain(..).map(AppEvent::InputKey));
        }

        if let Some(paste) = self.paste.as_mut()
            && !paste.pending_end_marker.is_empty()
        {
            append_keys_to_text(&mut paste.text, paste.pending_end_marker.drain(..));
        }

        app_events
    }

    fn push_key(&mut self, key: InputKey) -> Vec<AppEvent> {
        if let Some(paste) = self.paste.as_mut() {
            if let Some(app_event) = paste.push_key(key) {
                self.paste = None;
                return vec![app_event];
            }
            return Vec::new();
        }

        self.pending_start_marker.push(key);
        if matches_marker_prefix(&self.pending_start_marker, false) {
            if self.pending_start_marker.len() == marker_len() {
                self.pending_start_marker.clear();
                self.paste = Some(BracketedPasteBuffer::default());
            }
            return Vec::new();
        }

        self.pending_start_marker
            .drain(..)
            .map(AppEvent::InputKey)
            .collect()
    }
}

impl BracketedPasteBuffer {
    fn push_key(&mut self, key: InputKey) -> Option<AppEvent> {
        if self.pending_end_marker.is_empty() && key == InputKey::Esc {
            self.pending_end_marker.push(key);
            return None;
        }

        if !self.pending_end_marker.is_empty() {
            self.pending_end_marker.push(key);
            if matches_marker_prefix(&self.pending_end_marker, true) {
                if self.pending_end_marker.len() == marker_len() {
                    self.pending_end_marker.clear();
                    return Some(AppEvent::InputKey(InputKey::Paste(normalize_paste_text(
                        &self.text,
                    ))));
                }
                return None;
            }

            let drained = self.pending_end_marker.drain(..).collect::<Vec<_>>();
            append_keys_to_text(&mut self.text, drained);
            return None;
        }

        append_key_to_text(&mut self.text, key);
        None
    }
}

#[cfg(test)]
fn map_terminal_event(event: Event) -> Option<AppEvent> {
    match event {
        Event::Key(key_event) => {
            if !matches!(key_event.kind, KeyEventKind::Press | KeyEventKind::Repeat) {
                return None;
            }
            map_key_event(key_event).map(AppEvent::InputKey)
        }
        Event::Paste(text) => Some(AppEvent::InputKey(InputKey::Paste(normalize_paste_text(
            &text,
        )))),
        _ => None,
    }
}

fn normalize_paste_text(text: &str) -> String {
    text.replace("\r\n", "\n").replace('\r', "\n")
}

fn append_keys_to_text(buffer: &mut String, keys: impl IntoIterator<Item = InputKey>) {
    for key in keys {
        append_key_to_text(buffer, key);
    }
}

fn append_key_to_text(buffer: &mut String, key: InputKey) {
    match key {
        InputKey::Char(ch) => buffer.push(ch),
        InputKey::Paste(text) => buffer.push_str(&text),
        InputKey::Enter | InputKey::CtrlJ => buffer.push('\n'),
        InputKey::Tab => buffer.push('\t'),
        InputKey::Backspace => buffer.push('\u{8}'),
        InputKey::Delete => buffer.push('\u{7f}'),
        InputKey::Esc => buffer.push('\u{1b}'),
        InputKey::Left => buffer.push_str("\u{1b}[D"),
        InputKey::Right => buffer.push_str("\u{1b}[C"),
        InputKey::Up => buffer.push_str("\u{1b}[A"),
        InputKey::Down => buffer.push_str("\u{1b}[B"),
        InputKey::Home | InputKey::CtrlA => buffer.push_str("\u{1b}[H"),
        InputKey::End | InputKey::CtrlE => buffer.push_str("\u{1b}[F"),
        InputKey::CtrlC => buffer.push('\u{3}'),
        InputKey::CtrlU => buffer.push('\u{15}'),
        InputKey::CtrlW => buffer.push('\u{17}'),
    }
}

fn marker_len() -> usize {
    6
}

fn matches_marker_prefix(keys: &[InputKey], is_end_marker: bool) -> bool {
    let expected = [
        InputKey::Esc,
        InputKey::Char('['),
        InputKey::Char('2'),
        InputKey::Char('0'),
        InputKey::Char(if is_end_marker { '1' } else { '0' }),
        InputKey::Char('~'),
    ];
    keys.len() <= expected.len() && keys == &expected[..keys.len()]
}

#[cfg(test)]
mod tests {
    use crossterm::event::Event;
    use crossterm::event::KeyCode;
    use crossterm::event::KeyEvent;
    use crossterm::event::KeyModifiers;

    use super::AppEvent;
    use super::TerminalEventDecoder;
    use super::map_terminal_event;
    use crate::runtime_keys::InputKey;

    #[test]
    fn pasted_text_is_normalized_into_single_input_event() {
        let event = Event::Paste("first\r\nsecond\rthird".to_string());
        assert_eq!(
            map_terminal_event(event),
            Some(AppEvent::InputKey(InputKey::Paste(
                "first\nsecond\nthird".to_string()
            )))
        );
    }

    #[test]
    fn raw_bracketed_paste_key_burst_is_coalesced_into_single_paste_event() {
        let mut decoder = TerminalEventDecoder::default();
        let events = [
            Event::Key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE)),
            Event::Key(KeyEvent::new(KeyCode::Char('['), KeyModifiers::NONE)),
            Event::Key(KeyEvent::new(KeyCode::Char('2'), KeyModifiers::NONE)),
            Event::Key(KeyEvent::new(KeyCode::Char('0'), KeyModifiers::NONE)),
            Event::Key(KeyEvent::new(KeyCode::Char('0'), KeyModifiers::NONE)),
            Event::Key(KeyEvent::new(KeyCode::Char('~'), KeyModifiers::NONE)),
            Event::Key(KeyEvent::new(KeyCode::Char('f'), KeyModifiers::NONE)),
            Event::Key(KeyEvent::new(KeyCode::Char('i'), KeyModifiers::NONE)),
            Event::Key(KeyEvent::new(KeyCode::Char('r'), KeyModifiers::NONE)),
            Event::Key(KeyEvent::new(KeyCode::Char('s'), KeyModifiers::NONE)),
            Event::Key(KeyEvent::new(KeyCode::Char('t'), KeyModifiers::NONE)),
            Event::Key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE)),
            Event::Key(KeyEvent::new(KeyCode::Char('s'), KeyModifiers::NONE)),
            Event::Key(KeyEvent::new(KeyCode::Char('e'), KeyModifiers::NONE)),
            Event::Key(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::NONE)),
            Event::Key(KeyEvent::new(KeyCode::Char('o'), KeyModifiers::NONE)),
            Event::Key(KeyEvent::new(KeyCode::Char('n'), KeyModifiers::NONE)),
            Event::Key(KeyEvent::new(KeyCode::Char('d'), KeyModifiers::NONE)),
            Event::Key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE)),
            Event::Key(KeyEvent::new(KeyCode::Char('['), KeyModifiers::NONE)),
            Event::Key(KeyEvent::new(KeyCode::Char('2'), KeyModifiers::NONE)),
            Event::Key(KeyEvent::new(KeyCode::Char('0'), KeyModifiers::NONE)),
            Event::Key(KeyEvent::new(KeyCode::Char('1'), KeyModifiers::NONE)),
            Event::Key(KeyEvent::new(KeyCode::Char('~'), KeyModifiers::NONE)),
        ];

        let mut decoded = Vec::new();
        for event in events {
            decoded.extend(decoder.push(event));
        }
        decoded.extend(decoder.flush_pending());

        assert_eq!(
            decoded,
            vec![AppEvent::InputKey(InputKey::Paste(
                "first\nsecond".to_string()
            ))]
        );
    }

    #[test]
    fn standalone_escape_is_not_swallowed_by_paste_decoder() {
        let mut decoder = TerminalEventDecoder::default();
        let mut decoded = decoder.push(Event::Key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE)));
        decoded.extend(decoder.flush_pending());

        assert_eq!(decoded, vec![AppEvent::InputKey(InputKey::Esc)]);
    }

    #[test]
    fn split_bracketed_paste_start_marker_is_not_flushed_as_escape() {
        let mut decoder = TerminalEventDecoder::default();

        let first = decoder.push(Event::Key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE)));
        assert!(first.is_empty());
        assert!(decoder.has_pending_marker());

        let mut decoded = Vec::new();
        for event in [
            Event::Key(KeyEvent::new(KeyCode::Char('['), KeyModifiers::NONE)),
            Event::Key(KeyEvent::new(KeyCode::Char('2'), KeyModifiers::NONE)),
            Event::Key(KeyEvent::new(KeyCode::Char('0'), KeyModifiers::NONE)),
            Event::Key(KeyEvent::new(KeyCode::Char('0'), KeyModifiers::NONE)),
            Event::Key(KeyEvent::new(KeyCode::Char('~'), KeyModifiers::NONE)),
            Event::Key(KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE)),
            Event::Key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE)),
            Event::Key(KeyEvent::new(KeyCode::Char('b'), KeyModifiers::NONE)),
            Event::Key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE)),
            Event::Key(KeyEvent::new(KeyCode::Char('['), KeyModifiers::NONE)),
            Event::Key(KeyEvent::new(KeyCode::Char('2'), KeyModifiers::NONE)),
            Event::Key(KeyEvent::new(KeyCode::Char('0'), KeyModifiers::NONE)),
            Event::Key(KeyEvent::new(KeyCode::Char('1'), KeyModifiers::NONE)),
            Event::Key(KeyEvent::new(KeyCode::Char('~'), KeyModifiers::NONE)),
        ] {
            decoded.extend(decoder.push(event));
        }
        decoded.extend(decoder.flush_pending());

        assert_eq!(
            decoded,
            vec![AppEvent::InputKey(InputKey::Paste("a\nb".to_string()))]
        );
    }
}
