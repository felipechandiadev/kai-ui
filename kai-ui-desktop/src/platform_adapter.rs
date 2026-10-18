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
        _ => Key::Other,
    }
}

#[cfg(test)]
mod tests {
    use crate::ui_engine::{Intent, Key, Modifiers, TextIntent};

    use super::adapt_key_input;

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
