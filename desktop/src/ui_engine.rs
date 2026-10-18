//! Platform-independent intents and command dispatch for the Kai UI MVP.

use std::ops::Range;

use crate::element::{FocusKind, FocusOrder, WidgetId};
use crate::signal::Signal;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ActivationSource {
    Pointer,
    Enter,
    Space,
    Accessibility,
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
    SetFocus(Option<WidgetId>),
    /// Pointer pressed over the hit-tested widget (`None` = empty space).
    PointerDown(Option<WidgetId>),
    PointerUp(Option<WidgetId>),
    CancelPointer,
    /// Default action requested by assistive technology: focuses the widget
    /// and activates it when it is a button.
    Activate(WidgetId),
    Text(TextIntent),
}

#[derive(Clone, Debug, PartialEq)]
pub enum Command {
    SetFocus(Option<WidgetId>),
    SetPressed(Option<WidgetId>),
    Activate(WidgetId, ActivationSource),
    Text(WidgetId, TextIntent),
}

#[derive(Debug)]
pub struct UiModel {
    pub focused: Signal<Option<WidgetId>>,
    /// Button currently held down by the pointer.
    pub pressed: Signal<Option<WidgetId>>,
    pub activation_count: u32,
}

impl Default for UiModel {
    fn default() -> Self {
        Self {
            focused: Signal::new(None),
            pressed: Signal::new(None),
            activation_count: 0,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Activation {
    pub widget: WidgetId,
    pub source: ActivationSource,
    pub count: u32,
}

#[derive(Debug, Default, PartialEq)]
pub struct DispatchResult {
    pub changed: bool,
    pub activated: Option<Activation>,
    /// Text work for the focused text field.
    pub text_actions: Vec<(WidgetId, TextIntent)>,
}

/// Translate a Kai intent into commands and reduce them into the model.
/// Only widgets in `order` (focusable, enabled) can take focus or activate.
/// External work (text editing, clipboard, window/IME APIs, redraw) is returned
/// as data for the platform/application layer to perform.
pub fn dispatch(model: &mut UiModel, order: &FocusOrder, intent: Intent) -> DispatchResult {
    let commands = commands_for(model, order, intent);
    let mut result = DispatchResult::default();

    for command in commands {
        match command {
            Command::SetFocus(target) => result.changed |= model.focused.set(target),
            Command::SetPressed(target) => result.changed |= model.pressed.set(target),
            Command::Activate(widget, source) => {
                model.activation_count = model.activation_count.saturating_add(1);
                result.changed = true;
                result.activated = Some(Activation {
                    widget,
                    source,
                    count: model.activation_count,
                });
            }
            Command::Text(widget, action) => result.text_actions.push((widget, action)),
        }
    }

    result
}

/// Drop focus and pressed state that point at widgets no longer focusable
/// (removed or disabled) after the tree was rebuilt. Returns whether anything
/// changed.
pub fn sync_focus(model: &mut UiModel, order: &FocusOrder) -> bool {
    let stale = |target: Option<WidgetId>| target.is_some_and(|id| order.kind(id).is_none());
    let mut changed = false;
    if stale(*model.focused.get()) {
        changed |= model.focused.set(None);
    }
    if stale(*model.pressed.get()) {
        changed |= model.pressed.set(None);
    }
    changed
}

fn commands_for(model: &UiModel, order: &FocusOrder, intent: Intent) -> Vec<Command> {
    let focusable = |target: Option<WidgetId>| target.filter(|id| order.kind(*id).is_some());
    match intent {
        Intent::SetFocus(target) => match target {
            Some(id) if order.kind(id).is_none() => Vec::new(),
            _ => vec![Command::SetFocus(target)],
        },
        Intent::PointerDown(target) => {
            let target = focusable(target);
            let pressed = target.filter(|id| order.kind(*id) == Some(FocusKind::Button));
            vec![Command::SetFocus(target), Command::SetPressed(pressed)]
        }
        Intent::PointerUp(target) => {
            let pressed = *model.pressed.get();
            let mut commands = vec![Command::SetPressed(None)];
            if let Some(id) = pressed
                && target == Some(id)
            {
                commands.push(Command::Activate(id, ActivationSource::Pointer));
            }
            commands
        }
        Intent::CancelPointer => vec![Command::SetPressed(None)],
        Intent::Activate(id) => match order.kind(id) {
            Some(FocusKind::Button) => vec![
                Command::SetFocus(Some(id)),
                Command::Activate(id, ActivationSource::Accessibility),
            ],
            Some(FocusKind::TextField) => vec![Command::SetFocus(Some(id))],
            None => Vec::new(),
        },
        Intent::Text(TextIntent::KeyDown {
            key,
            modifiers,
            repeat,
        }) => key_commands(order, *model.focused.get(), key, modifiers, repeat),
        Intent::Text(action) => match *model.focused.get() {
            Some(id) if order.kind(id) == Some(FocusKind::TextField) => {
                vec![Command::Text(id, action)]
            }
            _ => Vec::new(),
        },
    }
}

fn key_commands(
    order: &FocusOrder,
    focused: Option<WidgetId>,
    key: Key,
    modifiers: Modifiers,
    repeat: bool,
) -> Vec<Command> {
    if key == Key::Tab && !repeat {
        return vec![Command::SetFocus(order.next(focused, modifiers.shift))];
    }

    let Some(focused) = focused else {
        return Vec::new();
    };
    match order.kind(focused) {
        Some(FocusKind::Button) => {
            if repeat {
                return Vec::new();
            }
            match key {
                Key::Enter => vec![Command::Activate(focused, ActivationSource::Enter)],
                Key::Space => vec![Command::Activate(focused, ActivationSource::Space)],
                _ => Vec::new(),
            }
        }
        Some(FocusKind::TextField) => text_key_commands(focused, key, modifiers, repeat),
        None => Vec::new(),
    }
}

fn text_key_commands(
    field: WidgetId,
    key: Key,
    modifiers: Modifiers,
    repeat: bool,
) -> Vec<Command> {
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
            return vec![Command::Text(field, action)];
        }
    }

    vec![Command::Text(
        field,
        TextIntent::KeyDown {
            key,
            modifiers,
            repeat,
        },
    )]
}

#[cfg(test)]
mod tests {
    use super::{
        Activation, ActivationSource, ImeIntent, Intent, Key, Modifiers, TextIntent, UiModel,
        dispatch, sync_focus,
    };
    use crate::element::FocusOrder;
    use crate::element::sample;

    fn order() -> FocusOrder {
        FocusOrder::from_tree(&sample::tree())
    }

    fn key(key: Key, shift: bool) -> Intent {
        Intent::Text(TextIntent::KeyDown {
            key,
            modifiers: Modifiers {
                shift,
                ..Modifiers::default()
            },
            repeat: false,
        })
    }

    fn activation(source: ActivationSource, count: u32) -> Option<Activation> {
        Some(Activation {
            widget: sample::button(),
            source,
            count,
        })
    }

    #[test]
    fn tab_and_shift_tab_follow_tree_order_and_skip_disabled() {
        let mut model = UiModel::default();
        let order = order();

        dispatch(&mut model, &order, key(Key::Tab, false));
        assert_eq!(*model.focused.get(), Some(sample::button()));
        dispatch(&mut model, &order, key(Key::Tab, false));
        assert_eq!(*model.focused.get(), Some(sample::field()));
        dispatch(&mut model, &order, key(Key::Tab, false));
        assert_eq!(*model.focused.get(), Some(sample::button()));
        dispatch(&mut model, &order, key(Key::Tab, true));
        assert_eq!(*model.focused.get(), Some(sample::field()));
    }

    #[test]
    fn enter_and_space_activate_only_the_focused_button() {
        let mut model = UiModel::default();
        let order = order();
        dispatch(&mut model, &order, Intent::SetFocus(Some(sample::button())));

        let enter = dispatch(&mut model, &order, key(Key::Enter, false));
        assert_eq!(enter.activated, activation(ActivationSource::Enter, 1));
        let space = dispatch(&mut model, &order, key(Key::Space, false));
        assert_eq!(space.activated, activation(ActivationSource::Space, 2));

        dispatch(&mut model, &order, Intent::SetFocus(Some(sample::field())));
        let no_activation = dispatch(&mut model, &order, key(Key::Enter, false));
        assert_eq!(no_activation.activated, None);
        assert_eq!(no_activation.text_actions.len(), 1);
        assert_eq!(no_activation.text_actions[0].0, sample::field());
    }

    #[test]
    fn pointer_activates_button_only_when_released_inside_it() {
        let mut model = UiModel::default();
        let order = order();
        dispatch(
            &mut model,
            &order,
            Intent::PointerDown(Some(sample::button())),
        );
        assert_eq!(*model.pressed.get(), Some(sample::button()));
        let activated = dispatch(
            &mut model,
            &order,
            Intent::PointerUp(Some(sample::button())),
        );
        assert_eq!(
            activated.activated,
            activation(ActivationSource::Pointer, 1)
        );
        assert_eq!(*model.pressed.get(), None);

        dispatch(
            &mut model,
            &order,
            Intent::PointerDown(Some(sample::button())),
        );
        let cancelled = dispatch(&mut model, &order, Intent::PointerUp(None));
        assert_eq!(cancelled.activated, None);

        dispatch(
            &mut model,
            &order,
            Intent::PointerDown(Some(sample::field())),
        );
        assert_eq!(*model.focused.get(), Some(sample::field()));
        assert_eq!(*model.pressed.get(), None);
        dispatch(&mut model, &order, Intent::PointerDown(None));
        assert_eq!(*model.focused.get(), None, "click on empty space blurs");
    }

    #[test]
    fn disabled_button_never_takes_focus_presses_or_activates() {
        let mut model = UiModel::default();
        let order = order();
        dispatch(&mut model, &order, Intent::SetFocus(Some(sample::field())));

        assert!(
            dispatch(
                &mut model,
                &order,
                Intent::SetFocus(Some(sample::disabled_button()))
            )
            .text_actions
            .is_empty()
        );
        assert_eq!(*model.focused.get(), Some(sample::field()));

        dispatch(
            &mut model,
            &order,
            Intent::PointerDown(Some(sample::disabled_button())),
        );
        assert_eq!(*model.pressed.get(), None);
        let up = dispatch(
            &mut model,
            &order,
            Intent::PointerUp(Some(sample::disabled_button())),
        );
        assert_eq!(up.activated, None);

        let a11y = dispatch(
            &mut model,
            &order,
            Intent::Activate(sample::disabled_button()),
        );
        assert_eq!(a11y.activated, None);
        assert!(!a11y.changed);
        assert_eq!(model.activation_count, 0);
        assert!(
            dispatch(&mut model, &order, Intent::Activate(sample::heading()))
                .activated
                .is_none()
        );
    }

    #[test]
    fn sync_focus_drops_focus_on_widgets_that_became_disabled() {
        let mut model = UiModel::default();
        dispatch(
            &mut model,
            &order(),
            Intent::PointerDown(Some(sample::button())),
        );
        assert!(!sync_focus(&mut model, &order()));

        let mut tree = sample::tree();
        tree.children[2].disabled = true;
        assert_eq!(tree.children[2].id, sample::button());
        assert!(sync_focus(&mut model, &FocusOrder::from_tree(&tree)));
        assert_eq!(*model.focused.get(), None);
        assert_eq!(*model.pressed.get(), None);
    }

    #[test]
    fn text_ime_and_clipboard_intents_are_gated_by_text_focus() {
        let mut model = UiModel::default();
        let order = order();
        let text = TextIntent::Ime(ImeIntent::Commit("á".to_owned()));
        assert!(
            dispatch(&mut model, &order, Intent::Text(text.clone()))
                .text_actions
                .is_empty()
        );

        dispatch(&mut model, &order, Intent::SetFocus(Some(sample::button())));
        assert!(
            dispatch(&mut model, &order, Intent::Text(TextIntent::Paste))
                .text_actions
                .is_empty()
        );

        dispatch(&mut model, &order, Intent::SetFocus(Some(sample::field())));
        let result = dispatch(&mut model, &order, Intent::Text(text.clone()));
        assert_eq!(result.text_actions, vec![(sample::field(), text)]);

        let copy = dispatch(&mut model, &order, Intent::Text(TextIntent::Copy));
        assert_eq!(copy.text_actions, vec![(sample::field(), TextIntent::Copy)]);
    }

    #[test]
    fn accessibility_activation_focuses_and_activates_buttons_only() {
        let mut model = UiModel::default();
        let order = order();
        let result = dispatch(&mut model, &order, Intent::Activate(sample::button()));
        assert_eq!(*model.focused.get(), Some(sample::button()));
        assert_eq!(
            result.activated,
            activation(ActivationSource::Accessibility, 1)
        );

        let result = dispatch(&mut model, &order, Intent::Activate(sample::field()));
        assert_eq!(*model.focused.get(), Some(sample::field()));
        assert_eq!(result.activated, None);
    }

    #[test]
    fn focus_update_reports_only_real_state_changes() {
        let mut model = UiModel::default();
        let order = order();
        assert!(dispatch(&mut model, &order, Intent::SetFocus(Some(sample::button()))).changed);
        assert!(!dispatch(&mut model, &order, Intent::SetFocus(Some(sample::button()))).changed);
    }
}
