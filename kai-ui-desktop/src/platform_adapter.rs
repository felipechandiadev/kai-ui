//! Converts winit input events into platform-independent Kai intents.

use winit::{
    event::{Ime, KeyEvent},
    keyboard::{KeyCode, ModifiersState, PhysicalKey},
};

use crate::ui_engine::{ImeIntent, Intent, Key, Modifiers, TextIntent};

pub fn adapt_keyboard_event(event: &KeyEvent, modifiers: Modifiers) -> Vec<Intent> {
    let PhysicalKey::Code(key_code) = event.physical_key else {
        return Vec::new();
    };

    adapt_key_input(
        map_key(key_code),
        modifiers,
        event.repeat,
        event.text.as_deref(),
    )
}

pub fn adapt_modifiers(modifiers: ModifiersState) -> Modifiers {
    Modifiers {
        shift: modifiers.shift_key(),
        command: modifiers.super_key(),
        control: modifiers.control_key(),
        option: modifiers.alt_key(),
    }
}

pub fn adapt_key_input(
    key: Key,
    modifiers: Modifiers,
    repeat: bool,
    committed_text: Option<&str>,
) -> Vec<Intent> {
    let mut intents = vec![Intent::Text(TextIntent::KeyDown {
        key,
        modifiers,
        repeat,
    })];

    if !modifiers.command
        && !modifiers.control
        && let Some(text) = committed_text.filter(|text| !text.is_empty())
        && should_emit_committed_text(key, text)
    {
        intents.push(Intent::Text(TextIntent::InsertText(text.to_owned())));
    }

    intents
}

pub fn adapt_ime_event(event: Ime) -> Intent {
    let ime = match event {
        Ime::Enabled => ImeIntent::Enabled,
        Ime::Preedit(text, cursor) => ImeIntent::Preedit {
            text,
            cursor: cursor.map(|(start, end)| start..end),
        },
        Ime::Commit(text) => ImeIntent::Commit(text),
        Ime::Disabled => ImeIntent::Disabled,
    };
    Intent::Text(TextIntent::Ime(ime))
}

/// winit often attaches `"\r"` to Enter; that must not become field text.
fn should_emit_committed_text(key: Key, text: &str) -> bool {
    if matches!(key, Key::Enter) {
        return false;
    }
    !text
        .chars()
        .all(|character| matches!(character, '\r' | '\n') || character.is_control())
}

fn map_key(key: KeyCode) -> Key {
    match key {
        KeyCode::Tab => Key::Tab,
        KeyCode::Enter | KeyCode::NumpadEnter => Key::Enter,
        KeyCode::Space => Key::Space,
        KeyCode::ArrowLeft => Key::ArrowLeft,
        KeyCode::ArrowRight => Key::ArrowRight,
        KeyCode::ArrowUp => Key::ArrowUp,
        KeyCode::ArrowDown => Key::ArrowDown,
        KeyCode::Home => Key::Home,
        KeyCode::End => Key::End,
        KeyCode::Backspace => Key::Backspace,
        KeyCode::Delete => Key::Delete,
        KeyCode::KeyA => Key::A,
        KeyCode::KeyC => Key::C,
        KeyCode::KeyX => Key::X,
        KeyCode::KeyV => Key::V,
        KeyCode::Escape => Key::Escape,
        KeyCode::Period => Key::Period,
        _ => char_key(key).map_or(Key::Other, Key::Char),
    }
}

/// Remaining letter and digit keys, by physical position.
fn char_key(key: KeyCode) -> Option<char> {
    const LETTERS: [(KeyCode, char); 22] = [
        (KeyCode::KeyB, 'b'),
        (KeyCode::KeyD, 'd'),
        (KeyCode::KeyE, 'e'),
        (KeyCode::KeyF, 'f'),
        (KeyCode::KeyG, 'g'),
        (KeyCode::KeyH, 'h'),
        (KeyCode::KeyI, 'i'),
        (KeyCode::KeyJ, 'j'),
        (KeyCode::KeyK, 'k'),
        (KeyCode::KeyL, 'l'),
        (KeyCode::KeyM, 'm'),
        (KeyCode::KeyN, 'n'),
        (KeyCode::KeyO, 'o'),
        (KeyCode::KeyP, 'p'),
        (KeyCode::KeyQ, 'q'),
        (KeyCode::KeyR, 'r'),
        (KeyCode::KeyS, 's'),
        (KeyCode::KeyT, 't'),
        (KeyCode::KeyU, 'u'),
        (KeyCode::KeyW, 'w'),
        (KeyCode::KeyY, 'y'),
        (KeyCode::KeyZ, 'z'),
    ];
    const DIGITS: [KeyCode; 10] = [
        KeyCode::Digit0,
        KeyCode::Digit1,
        KeyCode::Digit2,
        KeyCode::Digit3,
        KeyCode::Digit4,
        KeyCode::Digit5,
        KeyCode::Digit6,
        KeyCode::Digit7,
        KeyCode::Digit8,
        KeyCode::Digit9,
    ];
    if let Some((_, c)) = LETTERS.iter().find(|(code, _)| *code == key) {
        return Some(*c);
    }
    DIGITS
        .iter()
        .position(|code| *code == key)
        .and_then(|digit| char::from_digit(digit as u32, 10))
}

#[cfg(test)]
mod tests {
    use crate::ui_engine::{Intent, Key, Modifiers, TextIntent};

    use super::{adapt_key_input, map_key};
    use winit::keyboard::KeyCode;

    #[test]
    fn escape_period_and_other_letters_map_to_keys() {
        assert_eq!(map_key(KeyCode::Escape), Key::Escape);
        assert_eq!(map_key(KeyCode::Period), Key::Period);
        assert_eq!(map_key(KeyCode::KeyS), Key::Char('s'));
        assert_eq!(map_key(KeyCode::Digit7), Key::Char('7'));
        assert_eq!(map_key(KeyCode::KeyA), Key::A);
        assert_eq!(map_key(KeyCode::F1), Key::Other);
        // Escape carries "\u{1b}" as text in winit; it must not be inserted.
        assert_eq!(
            adapt_key_input(Key::Escape, Modifiers::default(), false, Some("\u{1b}")).len(),
            1
        );
    }

    #[test]
    fn physical_key_and_committed_text_are_separate_intents() {
        let intents = adapt_key_input(Key::A, Modifiers::default(), false, Some("á"));

        assert_eq!(
            intents,
            vec![
                Intent::Text(TextIntent::KeyDown {
                    key: Key::A,
                    modifiers: Modifiers::default(),
                    repeat: false,
                }),
                Intent::Text(TextIntent::InsertText("á".to_owned())),
            ]
        );
    }

    #[test]
    fn enter_does_not_emit_carriage_return_as_text() {
        let intents = adapt_key_input(Key::Enter, Modifiers::default(), false, Some("\r"));

        assert_eq!(intents.len(), 1);
        assert!(matches!(
            intents[0],
            Intent::Text(TextIntent::KeyDown {
                key: Key::Enter,
                ..
            })
        ));
    }

    #[test]
    fn command_shortcuts_do_not_emit_text_insertion() {
        let intents = adapt_key_input(
            Key::A,
            Modifiers {
                command: true,
                ..Modifiers::default()
            },
            false,
            Some("a"),
        );

        assert_eq!(intents.len(), 1);
        assert!(matches!(
            intents[0],
            Intent::Text(TextIntent::KeyDown { .. })
        ));
    }
}
