//! Platform-independent intents and command dispatch for the Kai UI MVP.

use std::ops::Range;
use std::time::{Duration, Instant};

use crate::element::{FocusKind, FocusOrder, KeyEquivalent, WidgetId};
use crate::geometry::Point;
use crate::responder::{Action, FocusContext, KeyRoute, key_equivalent_route};
use crate::signal::Signal;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ActivationSource {
    Pointer,
    Enter,
    Space,
    /// ↑ or ↓ inside a selecting focus group (a sidebar).
    Arrow,
    Accessibility,
    /// Return for the default button, Escape for the cancel button, or a
    /// control's own shortcut.
    KeyEquivalent,
}

/// Counts the presses of a double or triple click: a press continues the
/// series when it comes within [`Self::INTERVAL`] of the previous one and
/// no more than [`Self::SLOP`] points away from it.
#[derive(Clone, Copy, Debug, Default)]
pub struct ClickCounter {
    last: Option<(Instant, Point)>,
    count: u32,
}

impl ClickCounter {
    /// macOS default (`NSEvent.doubleClickInterval`).
    pub const INTERVAL: Duration = Duration::from_millis(500);
    pub const SLOP: f32 = 4.0;

    /// Record a press at `point` and return its place in the series (1, 2,
    /// 3…).
    pub fn press(&mut self, at: Instant, point: Point) -> u32 {
        let continues = self.last.is_some_and(|(time, previous)| {
            at.saturating_duration_since(time) <= Self::INTERVAL
                && (point.x - previous.x).abs() <= Self::SLOP
                && (point.y - previous.y).abs() <= Self::SLOP
        });
        self.count = if continues { self.count + 1 } else { 1 };
        self.last = Some((at, point));
        self.count
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Modifiers {
    pub shift: bool,
    pub command: bool,
    pub control: bool,
    /// ⌥ Option (Alt).
    pub option: bool,
}

impl Modifiers {
    pub const NONE: Self = Self {
        shift: false,
        command: false,
        control: false,
        option: false,
    };
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
    PageUp,
    PageDown,
    Backspace,
    Delete,
    Escape,
    /// `.` (Cmd+. cancels, like Escape).
    Period,
    A,
    C,
    X,
    V,
    /// Any other letter or digit key, lowercase, by its US-layout position
    /// (for `KeyEquivalent::cmd(Key::Char('s'))`).
    Char(char),
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

/// Where a focus change came from: keyboard focus is shown with a ring
/// (`keyboard-focus:`), pointer focus on buttons is not.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FocusOrigin {
    Keyboard,
    Pointer,
    /// Assistive technology or the app.
    Programmatic,
}

#[derive(Clone, Debug, PartialEq)]
pub enum UiCommand {
    SetFocus(Option<WidgetId>, FocusOrigin),
    SetPressed(Option<WidgetId>),
    Activate(WidgetId, ActivationSource),
    Text(WidgetId, TextIntent),
    /// Standard action for the responder chain.
    Perform(Action),
}

#[derive(Debug)]
pub struct UiModel {
    pub focused: Signal<Option<WidgetId>>,
    /// Button currently held down by the pointer.
    pub pressed: Signal<Option<WidgetId>>,
    /// The focus came from the keyboard or assistive technology, not a
    /// pointer press (`:focus-visible`).
    pub focus_visible: Signal<bool>,
    pub activation_count: u32,
}

impl Default for UiModel {
    fn default() -> Self {
        Self {
            focused: Signal::new(None),
            pressed: Signal::new(None),
            focus_visible: Signal::new(false),
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
    /// Standard action nobody consumed yet; the runtime sends it up the
    /// responder chain (`responder::chain`).
    pub action: Option<Action>,
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
            UiCommand::SetFocus(target, origin) => {
                result.changed |= model.focused.set(target);
                result.changed |= model.focus_visible.set(origin != FocusOrigin::Pointer);
            }
            UiCommand::SetPressed(target) => result.changed |= model.pressed.set(target),
            UiCommand::Activate(widget, source) => {
                model.activation_count = model.activation_count.saturating_add(1);
                result.changed = true;
                result.activated = Some(Activation {
                    widget,
                    source,
                    count: model.activation_count,
                });
            }
            UiCommand::Text(widget, action) => result.text_actions.push((widget, action)),
            UiCommand::Perform(action) => result.action = Some(action),
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

fn commands_for(model: &UiModel, order: &FocusOrder, intent: Intent) -> Vec<UiCommand> {
    let focusable = |target: Option<WidgetId>| target.filter(|id| order.kind(*id).is_some());
    match intent {
        Intent::SetFocus(target) => match target {
            Some(id) if order.kind(id).is_none() => Vec::new(),
            _ => vec![UiCommand::SetFocus(target, FocusOrigin::Programmatic)],
        },
        Intent::PointerDown(target) => {
            let target = focusable(target);
            let pressed = target.filter(|id| order.kind(*id) == Some(FocusKind::Button));
            if target.is_some_and(|id| !order.focus_on_press(id)) {
                return vec![UiCommand::SetPressed(pressed)];
            }
            vec![
                UiCommand::SetFocus(target, FocusOrigin::Pointer),
                UiCommand::SetPressed(pressed),
            ]
        }
        Intent::PointerUp(target) => {
            let pressed = *model.pressed.get();
            let mut commands = vec![UiCommand::SetPressed(None)];
            if let Some(id) = pressed
                && target == Some(id)
            {
                commands.push(UiCommand::Activate(id, ActivationSource::Pointer));
            }
            commands
        }
        Intent::CancelPointer => vec![UiCommand::SetPressed(None)],
        Intent::Activate(id) => match order.kind(id) {
            Some(FocusKind::Button) => vec![
                UiCommand::SetFocus(Some(id), FocusOrigin::Programmatic),
                UiCommand::Activate(id, ActivationSource::Accessibility),
            ],
            Some(FocusKind::TextField) => {
                vec![UiCommand::SetFocus(Some(id), FocusOrigin::Programmatic)]
            }
            None => Vec::new(),
        },
        Intent::Text(TextIntent::KeyDown {
            key,
            modifiers,
            repeat,
        }) => key_commands(order, *model.focused.get(), key, modifiers, repeat),
        Intent::Text(action) => match *model.focused.get() {
            Some(id) if order.kind(id) == Some(FocusKind::TextField) => {
                vec![UiCommand::Text(id, action)]
            }
            _ => Vec::new(),
        },
    }
}

/// Keys go first to the key equivalents (default and cancel buttons,
/// shortcuts), then to the focused element; standard actions nobody takes
/// are returned for the responder chain.
fn key_commands(
    order: &FocusOrder,
    focused: Option<WidgetId>,
    key: Key,
    modifiers: Modifiers,
    repeat: bool,
) -> Vec<UiCommand> {
    if key == Key::Tab && !repeat {
        return vec![UiCommand::SetFocus(
            order.next(focused, modifiers.shift),
            FocusOrigin::Keyboard,
        )];
    }

    let kind = focused.and_then(|id| order.kind(id));
    let pressed = KeyEquivalent { key, modifiers };
    if !repeat {
        let focus = FocusContext {
            is_text: kind == Some(FocusKind::TextField),
            claims_return: kind == Some(FocusKind::Button)
                || focused.is_some_and(|id| order.submits(id) || order.multiline(id)),
            claims_escape: focused.is_some_and(|id| order.cancels(id)),
        };
        match key_equivalent_route(order.equivalents(), order.commands(), pressed, focus) {
            KeyRoute::Activate(id) => {
                return vec![UiCommand::Activate(id, ActivationSource::KeyEquivalent)];
            }
            KeyRoute::Command(name) => return vec![UiCommand::Perform(Action::Command(name))],
            KeyRoute::PassThrough => {}
        }
        if pressed == KeyEquivalent::ESCAPE || pressed == KeyEquivalent::cmd(Key::Period) {
            return vec![UiCommand::Perform(Action::CancelOperation)];
        }
    }

    match (focused, kind) {
        (Some(focused), Some(FocusKind::Button)) => {
            let selects = order.selects(focused);
            if modifiers == Modifiers::NONE
                && let Some(backwards) = match key {
                    Key::ArrowUp => Some(true),
                    Key::ArrowDown => Some(false),
                    Key::ArrowLeft if !selects => Some(true),
                    Key::ArrowRight if !selects => Some(false),
                    _ => None,
                }
                && let Some(next) = order.group_step(focused, backwards)
            {
                if !selects {
                    return vec![UiCommand::SetFocus(Some(next), FocusOrigin::Keyboard)];
                }
                if next == focused {
                    return Vec::new();
                }
                return vec![
                    UiCommand::SetFocus(Some(next), FocusOrigin::Keyboard),
                    UiCommand::Activate(next, ActivationSource::Arrow),
                ];
            }
            if repeat {
                return Vec::new();
            }
            match key {
                Key::Enter => vec![UiCommand::Activate(focused, ActivationSource::Enter)],
                Key::Space => vec![UiCommand::Activate(focused, ActivationSource::Space)],
                _ => standard_action(key, modifiers)
                    .map(UiCommand::Perform)
                    .into_iter()
                    .collect(),
            }
        }
        (Some(field), Some(FocusKind::TextField)) => {
            let mut commands = text_key_commands(field, key, modifiers, repeat);
            if !repeat {
                let unconsumed = match standard_action(key, modifiers) {
                    // The field has no undo stack yet.
                    Some(action @ (Action::Undo | Action::Redo)) => Some(action),
                    _ if pressed == KeyEquivalent::RETURN
                        && !order.submits(field)
                        && !order.multiline(field) =>
                    {
                        Some(Action::InsertNewline)
                    }
                    _ => None,
                };
                commands.extend(unconsumed.map(UiCommand::Perform));
            }
            commands
        }
        _ if repeat => Vec::new(),
        _ => standard_action(key, modifiers)
            .map(UiCommand::Perform)
            .into_iter()
            .collect(),
    }
}

/// Standard key bindings of macOS for keys the focus does not handle.
fn standard_action(key: Key, modifiers: Modifiers) -> Option<Action> {
    let Modifiers {
        shift,
        command,
        control,
        option,
    } = modifiers;
    if command && !control && !option {
        return match (key, shift) {
            (Key::A, false) => Some(Action::SelectAll),
            (Key::C, false) => Some(Action::Copy),
            (Key::X, false) => Some(Action::Cut),
            (Key::V, false) => Some(Action::Paste),
            (Key::Char('z'), false) => Some(Action::Undo),
            (Key::Char('z'), true) => Some(Action::Redo),
            _ => None,
        };
    }
    if modifiers != Modifiers::NONE {
        return None;
    }
    match key {
        Key::Backspace | Key::Delete => Some(Action::Delete),
        Key::ArrowLeft => Some(Action::MoveLeft),
        Key::ArrowRight => Some(Action::MoveRight),
        Key::ArrowUp => Some(Action::MoveUp),
        Key::ArrowDown => Some(Action::MoveDown),
        _ => None,
    }
}

fn text_key_commands(
    field: WidgetId,
    key: Key,
    modifiers: Modifiers,
    repeat: bool,
) -> Vec<UiCommand> {
    if repeat
        && !matches!(
            key,
            Key::ArrowLeft
                | Key::ArrowRight
                | Key::ArrowUp
                | Key::ArrowDown
                | Key::Backspace
                | Key::Delete
                | Key::Enter
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
            return vec![UiCommand::Text(field, action)];
        }
    }

    vec![UiCommand::Text(
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
    use crate::element::sample;
    use crate::element::{Element, FocusOrder, WidgetId, div, input};
    use crate::responder::Action;

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
    fn buttons_that_do_not_focus_on_press_keep_focus_where_it_was() {
        let mut tree: Element<sample::Msg> = div().children([
            input("", "Contraseña"),
            div().on_click(sample::Msg::Clicked).focus_on_press(false),
        ]);
        tree.assign_ids();
        let (field, toggle) = (tree.children[0].id, tree.children[1].id);
        let order = FocusOrder::from_tree(&tree);
        let mut model = UiModel::default();
        dispatch(&mut model, &order, Intent::PointerDown(Some(field)));

        dispatch(&mut model, &order, Intent::PointerDown(Some(toggle)));
        assert_eq!(*model.focused.get(), Some(field));
        assert_eq!(*model.pressed.get(), Some(toggle));
        let up = dispatch(&mut model, &order, Intent::PointerUp(Some(toggle)));
        assert_eq!(up.activated.map(|a| a.widget), Some(toggle));
        assert_eq!(*model.focused.get(), Some(field));

        dispatch(&mut model, &order, key(Key::Tab, false));
        assert_eq!(
            *model.focused.get(),
            Some(toggle),
            "still reachable by keyboard"
        );
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

    fn tree_with_default_and_cancel() -> (Element<sample::Msg>, [WidgetId; 4]) {
        let mut tree: Element<sample::Msg> = div().children([
            input("", "Nombre").on_input(sample::Msg::Edited),
            input("", "Correo").on_submit(sample::Msg::Submitted),
            div().on_click(sample::Msg::Clicked).cancel_button(),
            div().on_click(sample::Msg::Clicked).default_button(true),
        ]);
        tree.assign_ids();
        let ids = [0, 1, 2, 3].map(|index| tree.children[index].id);
        (tree, ids)
    }

    fn plain(key: Key) -> Intent {
        Intent::Text(TextIntent::KeyDown {
            key,
            modifiers: Modifiers::NONE,
            repeat: false,
        })
    }

    fn cmd(key: Key, shift: bool) -> Intent {
        Intent::Text(TextIntent::KeyDown {
            key,
            modifiers: Modifiers {
                command: true,
                shift,
                ..Modifiers::NONE
            },
            repeat: false,
        })
    }

    #[test]
    fn return_and_escape_go_to_key_equivalents_first() {
        let (tree, [name, email, cancel, save]) = tree_with_default_and_cancel();
        let order = FocusOrder::from_tree(&tree);
        let mut model = UiModel::default();
        dispatch(&mut model, &order, Intent::SetFocus(Some(name)));

        let enter = dispatch(&mut model, &order, plain(Key::Enter));
        let activated = enter.activated.unwrap();
        assert_eq!(
            (activated.widget, activated.source),
            (save, ActivationSource::KeyEquivalent)
        );
        assert!(enter.text_actions.is_empty());
        assert_eq!(*model.focused.get(), Some(name), "focus stays in the field");

        dispatch(&mut model, &order, Intent::SetFocus(Some(email)));
        let submit = dispatch(&mut model, &order, plain(Key::Enter));
        assert_eq!(submit.activated, None, "on_submit keeps Return");
        assert_eq!(submit.text_actions.len(), 1);

        let escape = dispatch(&mut model, &order, plain(Key::Escape));
        assert_eq!(escape.activated.map(|a| a.widget), Some(cancel));
        let period = dispatch(&mut model, &order, cmd(Key::Period, false));
        assert_eq!(period.activated.map(|a| a.widget), Some(cancel));

        // A focused button keeps Return; Escape still cancels.
        dispatch(&mut model, &order, Intent::SetFocus(Some(cancel)));
        let enter = dispatch(&mut model, &order, plain(Key::Enter));
        assert_eq!(
            enter.activated.map(|a| (a.widget, a.source)),
            Some((cancel, ActivationSource::Enter))
        );
    }

    #[test]
    fn up_and_down_in_a_selecting_group_move_and_activate() {
        let row = |on: bool| div().on_click(()).pressed(on);
        let mut tree: Element<()> = div().children([div().selecting_focus_group().children([
            row(true),
            row(false),
            row(false),
        ])]);
        tree.assign_ids();
        let rows: Vec<WidgetId> = tree.children[0].children.iter().map(|r| r.id).collect();
        let order = FocusOrder::from_tree(&tree);
        let mut model = UiModel::default();
        dispatch(&mut model, &order, Intent::SetFocus(Some(rows[0])));

        let down = dispatch(&mut model, &order, plain(Key::ArrowDown));
        assert_eq!(*model.focused.get(), Some(rows[1]));
        assert_eq!(
            down.activated.map(|a| (a.widget, a.source)),
            Some((rows[1], ActivationSource::Arrow))
        );
        assert!(*model.focus_visible.get());

        let right = dispatch(&mut model, &order, plain(Key::ArrowRight));
        assert_eq!(right.activated, None);
        assert_eq!(right.action, Some(Action::MoveRight), "← and → pass by");
        assert_eq!(*model.focused.get(), Some(rows[1]));

        dispatch(&mut model, &order, plain(Key::ArrowDown));
        let end = dispatch(&mut model, &order, plain(Key::ArrowDown));
        assert_eq!(end.activated, None, "the last row is not chosen again");
        assert_eq!(*model.focused.get(), Some(rows[2]));
        let up = dispatch(&mut model, &order, plain(Key::ArrowUp));
        assert_eq!(up.activated.map(|a| a.widget), Some(rows[1]));
    }

    #[test]
    fn unconsumed_keys_become_standard_actions() {
        let mut model = UiModel::default();
        let order = order();
        let escape = dispatch(&mut model, &order, plain(Key::Escape));
        assert_eq!(escape.action, Some(Action::CancelOperation));
        assert_eq!(escape.activated, None);

        dispatch(&mut model, &order, Intent::SetFocus(Some(sample::button())));
        let copy = dispatch(&mut model, &order, cmd(Key::C, false));
        assert_eq!(copy.action, Some(Action::Copy));
        let down = dispatch(&mut model, &order, plain(Key::ArrowDown));
        assert_eq!(down.action, Some(Action::MoveDown), "not in a focus group");
        assert_eq!(*model.focused.get(), Some(sample::button()));

        dispatch(&mut model, &order, Intent::SetFocus(Some(sample::field())));
        let copy = dispatch(&mut model, &order, cmd(Key::C, false));
        assert_eq!(copy.action, None, "the field copies");
        assert_eq!(copy.text_actions, vec![(sample::field(), TextIntent::Copy)]);
        let redo = dispatch(&mut model, &order, cmd(Key::Char('z'), true));
        assert_eq!(redo.action, Some(Action::Redo));
        let escape = dispatch(&mut model, &order, plain(Key::Escape));
        assert_eq!(escape.action, Some(Action::CancelOperation));
        assert!(escape.text_actions.is_empty());

        let (mut tree, [name, ..]) = tree_with_default_and_cancel();
        tree.children.truncate(1);
        let order = FocusOrder::from_tree(&tree);
        dispatch(&mut model, &order, Intent::SetFocus(Some(name)));
        let enter = dispatch(&mut model, &order, plain(Key::Enter));
        assert_eq!(enter.action, Some(Action::InsertNewline));
    }

    #[test]
    fn a_multiline_field_keeps_return_from_the_default_button() {
        let (mut tree, _) = tree_with_default_and_cancel();
        tree.children[0] = input("", "Notas")
            .on_input(sample::Msg::Edited)
            .multiline(3, 3);
        tree.assign_ids();
        let notes = tree.children[0].id;
        let order = FocusOrder::from_tree(&tree);
        let mut model = UiModel::default();
        dispatch(&mut model, &order, Intent::SetFocus(Some(notes)));

        let enter = dispatch(&mut model, &order, plain(Key::Enter));
        assert_eq!(enter.activated, None);
        assert_eq!(enter.action, None, "the field consumes the line break");
        assert_eq!(enter.text_actions.len(), 1);
        let held = dispatch(
            &mut model,
            &order,
            Intent::Text(TextIntent::KeyDown {
                key: Key::Enter,
                modifiers: Modifiers::NONE,
                repeat: true,
            }),
        );
        assert_eq!(held.text_actions.len(), 1, "a held Return repeats");
    }

    #[test]
    fn pointer_focus_is_not_visible() {
        let mut model = UiModel::default();
        let order = order();
        dispatch(
            &mut model,
            &order,
            Intent::PointerDown(Some(sample::button())),
        );
        assert_eq!(*model.focused.get(), Some(sample::button()));
        assert!(!*model.focus_visible.get());
        let tab = dispatch(&mut model, &order, key(Key::Tab, false));
        assert!(tab.changed);
        assert!(*model.focus_visible.get());
        dispatch(
            &mut model,
            &order,
            Intent::PointerDown(Some(sample::field())),
        );
        assert!(!*model.focus_visible.get());
        dispatch(&mut model, &order, Intent::Activate(sample::button()));
        assert!(*model.focus_visible.get(), "assistive technology");
    }

    #[test]
    fn click_series_continue_only_close_in_time_and_place() {
        use super::ClickCounter;
        use crate::geometry::Point;
        use std::time::{Duration, Instant};
        let start = Instant::now();
        let at = |ms: u64| start + Duration::from_millis(ms);
        let here = Point { x: 10.0, y: 10.0 };
        let mut clicks = ClickCounter::default();
        assert_eq!(clicks.press(at(0), here), 1);
        assert_eq!(clicks.press(at(200), Point { x: 12.0, y: 9.0 }), 2);
        assert_eq!(clicks.press(at(400), here), 3);
        assert_eq!(clicks.press(at(1000), here), 1, "too late");
        assert_eq!(clicks.press(at(1100), Point { x: 30.0, y: 10.0 }), 1);
    }

    #[test]
    fn focus_update_reports_only_real_state_changes() {
        let mut model = UiModel::default();
        let order = order();
        assert!(dispatch(&mut model, &order, Intent::SetFocus(Some(sample::button()))).changed);
        assert!(!dispatch(&mut model, &order, Intent::SetFocus(Some(sample::button()))).changed);
    }
}
