use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use crossterm::event::KeyModifiers;

#[derive(Debug, Clone, Copy)]
pub(crate) enum InputKey {
    Char(char),
    Esc,
    Backspace,
    Delete,
    Left,
    Right,
    Home,
    End,
    Up,
    Down,
    Tab,
    Enter,
    CtrlJ,
    CtrlC,
    CtrlA,
    CtrlE,
    CtrlU,
    CtrlW,
}

pub(crate) fn map_key_event(key_event: KeyEvent) -> Option<InputKey> {
    match (key_event.code, key_event.modifiers) {
        (KeyCode::Esc, _) => Some(InputKey::Esc),
        (KeyCode::Char('c'), modifiers) if modifiers.contains(KeyModifiers::CONTROL) => {
            Some(InputKey::CtrlC)
        }
        (KeyCode::Char('j'), modifiers) if modifiers.contains(KeyModifiers::CONTROL) => {
            Some(InputKey::CtrlJ)
        }
        (KeyCode::Char('a'), modifiers) if modifiers.contains(KeyModifiers::CONTROL) => {
            Some(InputKey::CtrlA)
        }
        (KeyCode::Char('e'), modifiers) if modifiers.contains(KeyModifiers::CONTROL) => {
            Some(InputKey::CtrlE)
        }
        (KeyCode::Char('u'), modifiers) if modifiers.contains(KeyModifiers::CONTROL) => {
            Some(InputKey::CtrlU)
        }
        (KeyCode::Char('w'), modifiers) if modifiers.contains(KeyModifiers::CONTROL) => {
            Some(InputKey::CtrlW)
        }
        (KeyCode::Enter, _) => Some(InputKey::Enter),
        (KeyCode::Backspace, _) => Some(InputKey::Backspace),
        (KeyCode::Delete, _) => Some(InputKey::Delete),
        (KeyCode::Left, _) => Some(InputKey::Left),
        (KeyCode::Right, _) => Some(InputKey::Right),
        (KeyCode::Home, _) => Some(InputKey::Home),
        (KeyCode::End, _) => Some(InputKey::End),
        (KeyCode::Up, _) => Some(InputKey::Up),
        (KeyCode::Down, _) => Some(InputKey::Down),
        (KeyCode::Tab, _) => Some(InputKey::Tab),
        (KeyCode::Char(ch), modifiers)
            if modifiers.is_empty() || modifiers == KeyModifiers::SHIFT =>
        {
            Some(InputKey::Char(ch))
        }
        _ => None,
    }
}
