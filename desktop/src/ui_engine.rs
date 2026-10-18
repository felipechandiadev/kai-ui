//! Platform-independent intents and command dispatch for the Kai UI MVP.

use std::ops::Range;

use crate::signal::Signal;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FocusTarget {
    Button,
    TextField,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ActivationSource {
    Pointer,
    Enter,
    Space,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Modifiers {
    pub shift: bool,
    pub command: bool,
    pub control: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Key {
    Tab,
    Enter,
    Space,
    ArrowLeft,
    ArrowRight,
    ArrowUp,
    ArrowDown,
    Home,
    End,
    Backspace,
    Delete,
    A,
    C,
    X,
    V,
    Other,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ImeIntent {
    Enabled,
    Preedit {
        text: String,
        cursor: Option<Range<usize>>,
    },
    Commit(String),
    Cancel,
    Disabled,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TextIntent {
    /// A physical/semantic key; it does not itself insert text.
    KeyDown {
        key: Key,
        modifiers: Modifiers,
        repeat: bool,
    },
    /// Text committed by the platform input system.
    InsertText(String),
    /// IME preedit/commit lifecycle, separate from committed text.
    Ime(ImeIntent),
    SelectAll,
    Copy,
    Cut,
    Paste,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Intent {
    SetFocus(Option<FocusTarget>),
    PointerDown(Option<FocusTarget>),
    PointerUp(Option<FocusTarget>),
    CancelPointer,
    Text(TextIntent),
}

#[derive(Clone, Debug, PartialEq)]
pub enum Command {
    SetFocus(Option<FocusTarget>),
    SetButtonPressed(bool),
    ActivateButton(ActivationSource),
    Text(TextIntent),
}

#[derive(Debug)]
pub struct UiModel {
    pub focused: Signal<Option<FocusTarget>>,
    pub button_pressed: Signal<bool>,
    pub activation_count: u32,
}

impl Default for UiModel {
    fn default() -> Self {
        Self {
            focused: Signal::new(None),
            button_pressed: Signal::new(false),
            activation_count: 0,
        }
    }
}

#[derive(Debug, Default, PartialEq)]
pub struct DispatchResult {
    pub changed: bool,
    pub activated: Option<(ActivationSource, u32)>,
    pub text_actions: Vec<TextIntent>,
}

/// Translate a Kai intent into commands and reduce them into the model.
/// External work (text editing, clipboard, window/IME APIs, redraw) is returned
/// as data for the platform/application layer to perform.
pub fn dispatch(model: &mut UiModel, intent: Intent) -> DispatchResult {
    let commands = commands_for(model, intent);
    let mut result = DispatchResult::default();

    for command in commands {
        match command {
            Command::SetFocus(target) => result.changed |= model.focused.set(target),
            Command::SetButtonPressed(pressed) => {
                result.changed |= model.button_pressed.set(pressed);
            }
            Command::ActivateButton(source) => {
                model.activation_count = model.activation_count.saturating_add(1);
                result.changed = true;
                result.activated = Some((source, model.activation_count));
            }
            Command::Text(action) => result.text_actions.push(action),
        }
    }

    result
}

fn commands_for(model: &UiModel, intent: Intent) -> Vec<Command> {
    match intent {
        Intent::SetFocus(target) => vec![Command::SetFocus(target)],
        Intent::PointerDown(target) => vec![
            Command::SetFocus(target),
            Command::SetButtonPressed(target == Some(FocusTarget::Button)),
        ],
        Intent::PointerUp(target) => {
            let was_pressed = *model.button_pressed.get();
            let mut commands = vec![Command::SetButtonPressed(false)];
            if was_pressed && target == Some(FocusTarget::Button) {
                commands.push(Command::ActivateButton(ActivationSource::Pointer));
            }
            commands
        }
        Intent::CancelPointer => vec![Command::SetButtonPressed(false)],
        Intent::Text(TextIntent::KeyDown {
            key,
            modifiers,
            repeat,
        }) => key_commands(*model.focused.get(), key, modifiers, repeat),
        Intent::Text(action) => {
            if *model.focused.get() == Some(FocusTarget::TextField) {
                vec![Command::Text(action)]
            } else {
                Vec::new()
            }
        }
    }
}

fn key_commands(
    focused: Option<FocusTarget>,
    key: Key,
    modifiers: Modifiers,
    repeat: bool,
) -> Vec<Command> {
    if key == Key::Tab && !repeat {
        let backwards = modifiers.shift;
        let next = match (focused, backwards) {
            (None, _) => Some(FocusTarget::Button),
            (Some(FocusTarget::Button), false) => Some(FocusTarget::TextField),
            (Some(FocusTarget::TextField), false) => Some(FocusTarget::Button),
            (Some(FocusTarget::Button), true) => Some(FocusTarget::TextField),
            (Some(FocusTarget::TextField), true) => Some(FocusTarget::Button),
        };
        return vec![Command::SetFocus(next)];
    }

    if focused == Some(FocusTarget::Button) && !repeat {
        match key {
            Key::Enter => return vec![Command::ActivateButton(ActivationSource::Enter)],
            Key::Space => return vec![Command::ActivateButton(ActivationSource::Space)],
            _ => {}
        }
    }

    if focused != Some(FocusTarget::TextField) {
        return Vec::new();
    }

    if repeat
        && !matches!(
            key,
            Key::ArrowLeft
                | Key::ArrowRight
                | Key::ArrowUp
                | Key::ArrowDown
                | Key::Backspace
                | Key::Delete
        )
    {
        return Vec::new();
    }

    if modifiers.command {
        let action = match key {
            Key::A => Some(TextIntent::SelectAll),
            Key::C => Some(TextIntent::Copy),
            Key::X => Some(TextIntent::Cut),
            Key::V => Some(TextIntent::Paste),
            _ => None,
        };
        if let Some(action) = action {
            return vec![Command::Text(action)];
        }
    }

    vec![Command::Text(TextIntent::KeyDown {
        key,
        modifiers,
        repeat,
    })]
}

#[cfg(test)]
mod tests {
    use super::{
        ActivationSource, FocusTarget, ImeIntent, Intent, Key, Modifiers, TextIntent, UiModel,
        dispatch,
    };

    #[test]
    fn tab_and_shift_tab_cycle_focus_without_platform_state() {
        let mut model = UiModel::default();
        let forward = Modifiers::default();
        let backwards = Modifiers {
            shift: true,
            ..Modifiers::default()
        };

        dispatch(
            &mut model,
            Intent::Text(TextIntent::KeyDown {
                key: Key::Tab,
                modifiers: forward,
                repeat: false,
            }),
        );
        assert_eq!(*model.focused.get(), Some(FocusTarget::Button));

        dispatch(
            &mut model,
            Intent::Text(TextIntent::KeyDown {
                key: Key::Tab,
                modifiers: forward,
                repeat: false,
            }),
        );
        assert_eq!(*model.focused.get(), Some(FocusTarget::TextField));

        dispatch(
            &mut model,
            Intent::Text(TextIntent::KeyDown {
                key: Key::Tab,
                modifiers: backwards,
                repeat: false,
            }),
        );
        assert_eq!(*model.focused.get(), Some(FocusTarget::Button));
    }

    #[test]
    fn enter_and_space_activate_only_the_focused_button() {
        let mut model = UiModel::default();
        dispatch(&mut model, Intent::SetFocus(Some(FocusTarget::Button)));

        let enter = dispatch(
            &mut model,
            Intent::Text(TextIntent::KeyDown {
                key: Key::Enter,
                modifiers: Modifiers::default(),
                repeat: false,
            }),
        );
        assert_eq!(enter.activated, Some((ActivationSource::Enter, 1)));

        let space = dispatch(
            &mut model,
            Intent::Text(TextIntent::KeyDown {
                key: Key::Space,
                modifiers: Modifiers::default(),
                repeat: false,
            }),
        );
        assert_eq!(space.activated, Some((ActivationSource::Space, 2)));

        dispatch(&mut model, Intent::SetFocus(Some(FocusTarget::TextField)));
        let no_activation = dispatch(
            &mut model,
            Intent::Text(TextIntent::KeyDown {
                key: Key::Enter,
                modifiers: Modifiers::default(),
                repeat: false,
            }),
        );
        assert_eq!(no_activation.activated, None);
        assert_eq!(no_activation.text_actions.len(), 1);
    }

    #[test]
    fn pointer_activates_button_only_when_released_inside_it() {
        let mut model = UiModel::default();
        dispatch(&mut model, Intent::PointerDown(Some(FocusTarget::Button)));
        let activated = dispatch(&mut model, Intent::PointerUp(Some(FocusTarget::Button)));
        assert_eq!(activated.activated, Some((ActivationSource::Pointer, 1)));
        assert!(!model.button_pressed.get());

        dispatch(&mut model, Intent::PointerDown(Some(FocusTarget::Button)));
        let cancelled = dispatch(&mut model, Intent::PointerUp(None));
        assert_eq!(cancelled.activated, None);
    }

    #[test]
    fn text_ime_and_clipboard_intents_are_gated_by_text_focus() {
        let mut model = UiModel::default();
        let text = TextIntent::Ime(ImeIntent::Commit("á".to_owned()));
        assert!(
            dispatch(&mut model, Intent::Text(text.clone()))
                .text_actions
                .is_empty()
        );

        dispatch(&mut model, Intent::SetFocus(Some(FocusTarget::TextField)));
        let result = dispatch(&mut model, Intent::Text(text.clone()));
        assert_eq!(result.text_actions, vec![text]);

        let copy = dispatch(&mut model, Intent::Text(TextIntent::Copy));
        assert_eq!(copy.text_actions, vec![TextIntent::Copy]);
    }

    #[test]
    fn focus_update_reports_only_real_state_changes() {
        let mut model = UiModel::default();
        assert!(dispatch(&mut model, Intent::SetFocus(Some(FocusTarget::Button))).changed);
        assert!(!dispatch(&mut model, Intent::SetFocus(Some(FocusTarget::Button))).changed);
    }
}
