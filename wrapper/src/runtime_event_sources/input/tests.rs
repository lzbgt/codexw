use crossterm::event::Event;
use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use crossterm::event::KeyModifiers;

use super::decode::TerminalEventDecoder;
use super::decode::map_terminal_event;
use crate::runtime_event_sources::AppEvent;
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
