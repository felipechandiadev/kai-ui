//! Keyboard routing as in AppKit (docs/dev/12-modelo-appkit.md, Paso 7).
//!
//! A key press is first offered to the window's key equivalents
//! ([`key_equivalent_route`], AppKit's `performKeyEquivalent:`): the default
//! button takes Return, the cancel button Escape (and Cmd+.), and any control
//! may declare a Cmd shortcut. What no equivalent takes goes to the focused
//! element; a standard [`Action`] it does not consume travels the responder
//! [`chain`]: focus, its ancestors, the window and the app. Shortcuts of the
//! app's commands ([`crate::commands`]) come after the controls' own
//! equivalents, and the session runs them by id.

use crate::element::{KeyEquivalent, WidgetId};
use crate::ui_engine::Key;

/// Standard actions, named after AppKit selectors (`NSResponder`,
/// `NSStandardKeyBindingResponding`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Action {
    /// Return in a field that does not submit (`insertNewline:`).
    InsertNewline,
    /// Escape or Cmd+. with no cancel button (`cancelOperation:`).
    CancelOperation,
    SelectAll,
    Copy,
    Cut,
    Paste,
    Undo,
    Redo,
    /// `delete:`.
    Delete,
    /// Command of the app's registry ([`crate::commands`]), by id.
    Command(&'static str),
}

/// Who may handle an action, most specific first.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Responder {
    Widget(WidgetId),
    Window,
    App,
}

/// Responder chain for the current focus: `[focus, parents…, Window, App]`.
/// `path_to_focus` goes from the root to the focused element (empty without
/// focus).
pub fn chain(path_to_focus: &[WidgetId]) -> Vec<Responder> {
    path_to_focus
        .iter()
        .rev()
        .map(|id| Responder::Widget(*id))
        .chain([Responder::Window, Responder::App])
        .collect()
}

/// Result of offering a key to the window before the focus.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KeyRoute {
    /// An enabled control declares this equivalent: activate it.
    Activate(WidgetId),
    /// Shortcut of an enabled command of the registry, by id.
    Command(&'static str),
    /// Let the focused element have it.
    PassThrough,
}

/// What the focused element does with keys, for [`key_equivalent_route`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FocusContext {
    /// The focus is a text field: plain keys are typing, not shortcuts.
    pub is_text: bool,
    /// The focus handles Return itself: a focused button (Kai web behavior)
    /// or a text field with `on_submit` (kept for compatibility).
    pub claims_return: bool,
}

/// Offers `pressed` to the key equivalents (`equivalents`: enabled controls
/// in document order), then to the shortcuts of enabled app commands
/// (`commands`), as AppKit offers it to the views before the main menu.
///
/// - Return: the default button, unless the focus claims Return.
/// - Escape and Cmd+.: the control whose equivalent is Escape.
/// - Anything else: the control with that exact equivalent, or else the
///   command with that shortcut. Neither fires without Cmd or Control
///   while typing in a text field.
///
/// With two controls on the same equivalent the first in document order
/// wins; a control also hides a command with its shortcut (debug builds
/// report both clashes).
pub fn key_equivalent_route(
    equivalents: &[(WidgetId, KeyEquivalent)],
    commands: &[(&'static str, KeyEquivalent)],
    pressed: KeyEquivalent,
    focus: FocusContext,
) -> KeyRoute {
    let wanted = if pressed == KeyEquivalent::cmd(Key::Period) {
        KeyEquivalent::ESCAPE
    } else {
        pressed
    };
    if wanted == KeyEquivalent::RETURN && focus.claims_return {
        return KeyRoute::PassThrough;
    }
    let is_named = matches!(wanted.key, Key::Enter | Key::Escape);
    if focus.is_text && !is_named && !wanted.modifiers.command && !wanted.modifiers.control {
        return KeyRoute::PassThrough;
    }
    let mut matches = equivalents
        .iter()
        .filter(|(_, equivalent)| *equivalent == wanted)
        .map(|(id, _)| *id);
    let command = commands
        .iter()
        .find(|(_, shortcut)| *shortcut == pressed)
        .map(|(id, _)| *id);
    let Some(first) = matches.next() else {
        return command.map_or(KeyRoute::PassThrough, KeyRoute::Command);
    };
    if cfg!(debug_assertions) {
        let others: Vec<WidgetId> = matches.collect();
        if !others.is_empty() {
            eprintln!(
                "Equivalente de teclado {} repetido: gana {first:?}, se ignoran {others:?}",
                wanted.label()
            );
        }
        if let Some(command) = command {
            eprintln!(
                "El equivalente {} de {first:?} tapa el atajo del comando {command}",
                wanted.label()
            );
        }
    }
    KeyRoute::Activate(first)
}

#[cfg(test)]
mod tests {
    use super::{FocusContext, KeyRoute, Responder, chain, key_equivalent_route};
    use crate::element::{KeyEquivalent, WidgetId};
    use crate::ui_engine::{Key, Modifiers};

    const SAVE: WidgetId = WidgetId(1);
    const CANCEL: WidgetId = WidgetId(2);
    const EXPORT: WidgetId = WidgetId(3);

    fn equivalents() -> Vec<(WidgetId, KeyEquivalent)> {
        vec![
            (SAVE, KeyEquivalent::RETURN),
            (CANCEL, KeyEquivalent::ESCAPE),
            (EXPORT, KeyEquivalent::cmd(Key::Char('e'))),
        ]
    }

    const FIELD: FocusContext = FocusContext {
        is_text: true,
        claims_return: false,
    };

    #[test]
    fn return_activates_default_button_from_text_field_without_submit() {
        assert_eq!(
            key_equivalent_route(&equivalents(), &[], KeyEquivalent::RETURN, FIELD),
            KeyRoute::Activate(SAVE)
        );
        assert_eq!(
            key_equivalent_route(
                &equivalents(),
                &[],
                KeyEquivalent::RETURN,
                FocusContext::default()
            ),
            KeyRoute::Activate(SAVE)
        );
    }

    #[test]
    fn text_field_with_on_submit_keeps_return() {
        let submitting = FocusContext {
            claims_return: true,
            ..FIELD
        };
        assert_eq!(
            key_equivalent_route(&equivalents(), &[], KeyEquivalent::RETURN, submitting),
            KeyRoute::PassThrough
        );
    }

    #[test]
    fn escape_activates_cancel_button() {
        for key in [KeyEquivalent::ESCAPE, KeyEquivalent::cmd(Key::Period)] {
            assert_eq!(
                key_equivalent_route(&equivalents(), &[], key, FIELD),
                KeyRoute::Activate(CANCEL)
            );
        }
        assert_eq!(
            key_equivalent_route(&[], &[], KeyEquivalent::ESCAPE, FIELD),
            KeyRoute::PassThrough
        );
    }

    #[test]
    fn disabled_default_button_is_ignored() {
        // Disabled controls are not in the list (`FocusOrder::equivalents`).
        let enabled: Vec<_> = equivalents()
            .into_iter()
            .filter(|(id, _)| *id != SAVE)
            .collect();
        assert_eq!(
            key_equivalent_route(&enabled, &[], KeyEquivalent::RETURN, FIELD),
            KeyRoute::PassThrough
        );
    }

    #[test]
    fn cmd_key_equivalent_activates_control() {
        let cmd_e = KeyEquivalent::cmd(Key::Char('e'));
        assert_eq!(
            key_equivalent_route(&equivalents(), &[], cmd_e, FIELD),
            KeyRoute::Activate(EXPORT)
        );
        let cmd_shift_e = KeyEquivalent {
            modifiers: Modifiers {
                shift: true,
                ..cmd_e.modifiers
            },
            ..cmd_e
        };
        assert_eq!(
            key_equivalent_route(&equivalents(), &[], cmd_shift_e, FIELD),
            KeyRoute::PassThrough
        );
        // A plain-letter equivalent never steals typing.
        let plain = KeyEquivalent {
            key: Key::Char('e'),
            modifiers: Modifiers::NONE,
        };
        let list = [(EXPORT, plain)];
        assert_eq!(
            key_equivalent_route(&list, &[], plain, FIELD),
            KeyRoute::PassThrough
        );
        assert_eq!(
            key_equivalent_route(&list, &[], plain, FocusContext::default()),
            KeyRoute::Activate(EXPORT)
        );
    }

    #[test]
    fn first_equivalent_in_document_order_wins() {
        let list = [
            (SAVE, KeyEquivalent::RETURN),
            (EXPORT, KeyEquivalent::RETURN),
        ];
        assert_eq!(
            key_equivalent_route(&list, &[], KeyEquivalent::RETURN, FocusContext::default()),
            KeyRoute::Activate(SAVE)
        );
    }

    #[test]
    fn command_shortcuts_come_after_controls() {
        let cmd_i = KeyEquivalent::cmd(Key::Char('i'));
        let cmd_e = KeyEquivalent::cmd(Key::Char('e'));
        let commands = [("contador.incrementar", cmd_i), ("archivo.exportar", cmd_e)];
        for focus in [FocusContext::default(), FIELD] {
            assert_eq!(
                key_equivalent_route(&equivalents(), &commands, cmd_i, focus),
                KeyRoute::Command("contador.incrementar")
            );
        }
        assert_eq!(
            key_equivalent_route(&equivalents(), &commands, cmd_e, FIELD),
            KeyRoute::Activate(EXPORT),
            "the control's equivalent wins"
        );
        let plain = KeyEquivalent {
            key: Key::Char('i'),
            modifiers: Modifiers::NONE,
        };
        let typing = [("contador.incrementar", plain)];
        assert_eq!(
            key_equivalent_route(&[], &typing, plain, FIELD),
            KeyRoute::PassThrough
        );
    }

    #[test]
    fn chain_goes_from_focus_to_app() {
        let (root, card, field) = (WidgetId(10), WidgetId(11), WidgetId(12));
        assert_eq!(
            chain(&[root, card, field]),
            vec![
                Responder::Widget(field),
                Responder::Widget(card),
                Responder::Widget(root),
                Responder::Window,
                Responder::App
            ]
        );
        assert_eq!(chain(&[]), vec![Responder::Window, Responder::App]);
    }
}
