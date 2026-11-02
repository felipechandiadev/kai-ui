//! The session's half of the floating layer (docs/dev/32): which floating
//! elements are open, where the focus goes when one appears or leaves, and
//! what dismisses them (Escape, a press outside). Layout places them
//! ([`crate::layout`]); the app opens and closes them.

use super::{ModalFocus, Session};
use crate::element::{Element, Kind, Role, WidgetId};
use crate::overlay::OverlayFocus;
use crate::responder::Action;
use crate::runtime::{App, action_message};
use crate::ui_engine::{Intent, Key, Modifiers, TextIntent};

/// A floating element in the last view.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct OpenFloat {
    pub id: WidgetId,
    /// Its parent, which it hangs from.
    pub anchor: WidgetId,
    pub focus: OverlayFocus,
    /// Focus when it appeared, given back when it leaves
    /// ([`OverlayFocus::Contain`]).
    pub previous_focus: Option<WidgetId>,
}

/// Every floating element under `element`, in document order, with its
/// parent and focus rule.
fn collect<Msg>(element: &Element<Msg>, found: &mut Vec<(WidgetId, WidgetId, OverlayFocus)>) {
    for child in &element.children {
        if let Some(floating) = &child.floating {
            found.push((child.id, element.id, floating.focus));
        }
        collect(child, found);
    }
}

/// The innermost open floating element of `tree` that contains the focus
/// ([`OverlayFocus::Contain`]): only it takes Tab and keys while it is
/// open.
pub(super) fn contained<'a, Msg>(
    tree: &'a Element<Msg>,
    floats: &[OpenFloat],
) -> Option<&'a Element<Msg>> {
    floats
        .iter()
        .rev()
        .find(|float| float.focus == OverlayFocus::Contain)
        .and_then(|float| tree.find(float.id))
}

/// The first [`Role::ListBoxOption`] in the On state under `element`.
fn active_option<Msg>(element: &Element<Msg>) -> Option<WidgetId> {
    if element.role == Some(Role::ListBoxOption) && element.is_pressed() {
        return Some(element.id);
    }
    element.children.iter().find_map(active_option)
}

impl<A: App> Session<A> {
    /// Follow the floating elements of the new view: one that contains the
    /// focus takes it when it appears (its first control) and gives it
    /// back when it leaves, unless the focus already went elsewhere.
    pub(super) fn track_floats(&mut self) {
        let mut found = Vec::new();
        collect(&self.tree, &mut found);
        let focused = self.focused();
        let mut open = Vec::with_capacity(found.len());
        for (id, anchor, focus) in found {
            match self.floats.iter().find(|float| float.id == id) {
                Some(float) => open.push(OpenFloat {
                    anchor,
                    focus,
                    ..float.clone()
                }),
                None => {
                    if focus == OverlayFocus::Contain {
                        self.modal_focus = Some(ModalFocus::First);
                    }
                    open.push(OpenFloat {
                        id,
                        anchor,
                        focus,
                        previous_focus: focused,
                    });
                }
            }
        }
        let lost = focused.is_none_or(|id| self.tree.find(id).is_none());
        for gone in &self.floats {
            if gone.focus == OverlayFocus::Contain
                && lost
                && !open.iter().any(|float| float.id == gone.id)
            {
                self.modal_focus = Some(ModalFocus::Restore(gone.previous_focus));
            }
        }
        self.floats = open;
    }

    /// The dismiss message of open floating element `id`.
    fn dismiss_message(&self, id: WidgetId) -> Option<A::Msg> {
        self.tree
            .find(id)
            .and_then(|element| element.floating.as_ref())
            .and_then(|floating| floating.on_dismiss.clone())
    }

    /// Escape or ⌘. for a floating element the focus is not inside: the
    /// innermost one whose anchor has the focus (a combobox list while
    /// its field is focused), or one that contains the focus but has
    /// nothing focusable. From inside one, Escape goes up the responder
    /// chain instead ([`Element::floating`]).
    pub(super) fn float_key(&mut self, intent: &Intent) -> bool {
        let Intent::Text(TextIntent::KeyDown { key, modifiers, .. }) = intent else {
            return false;
        };
        let escape = match key {
            Key::Escape => *modifiers == Modifiers::NONE,
            Key::Period => modifiers.command && !modifiers.option && !modifiers.control,
            _ => false,
        };
        if !escape {
            return self.combobox_key(*key, *modifiers);
        }
        if self.floats.is_empty() {
            return false;
        }
        let path = self
            .focused()
            .and_then(|id| self.tree.path_to(id))
            .unwrap_or_default();
        let msg = self
            .floats
            .iter()
            .rev()
            .filter(|float| !path.contains(&float.id))
            .find(|float| float.focus == OverlayFocus::Contain || path.contains(&float.anchor))
            .and_then(|float| self.dismiss_message(float.id));
        let Some(msg) = msg else {
            return false;
        };
        self.send(msg);
        true
    }

    /// The focused input when it is a [`Role::ComboBox`].
    fn focused_combobox(&self) -> Option<WidgetId> {
        let id = self.focused()?;
        let element = self.tree.find(id)?;
        (matches!(element.kind, Kind::Input { .. }) && element.role == Some(Role::ComboBox))
            .then_some(id)
    }

    /// The open floating element hanging from an ancestor of `id`.
    fn float_of(&self, id: WidgetId) -> Option<WidgetId> {
        let path = self.tree.path_to(id)?;
        self.floats
            .iter()
            .rev()
            .find(|float| path.contains(&float.anchor) && !path.contains(&float.id))
            .map(|float| float.id)
    }

    /// Keys a focused [`Role::ComboBox`] takes before its text does (the
    /// list keeps no focus of its own, `aria-activedescendant`): ↑ and ↓
    /// travel the responder chain as `MoveUp` / `MoveDown` and Return as
    /// `InsertNewline`; a key nobody takes edits the text as usual. Tab
    /// with the list open dismisses it first, so the focus goes on to
    /// the next control and not into the list.
    fn combobox_key(&mut self, key: Key, modifiers: Modifiers) -> bool {
        let Some(field) = self.focused_combobox() else {
            return false;
        };
        let plain = modifiers == Modifiers::NONE;
        let action = match key {
            Key::ArrowDown if plain => Action::MoveDown,
            Key::ArrowUp if plain => Action::MoveUp,
            Key::Enter if plain => Action::InsertNewline,
            Key::Tab if !modifiers.command && !modifiers.control && !modifiers.option => {
                let dismiss = self
                    .float_of(field)
                    .and_then(|float| self.dismiss_message(float));
                if let Some(msg) = dismiss {
                    self.send(msg);
                    self.rebuild();
                    self.relayout();
                }
                return false;
            }
            _ => return false,
        };
        let Some(msg) = action_message(&self.tree, Some(field), action) else {
            return false;
        };
        self.send(msg);
        true
    }

    /// The active option of the focused [`Role::ComboBox`]: the
    /// [`Role::ListBoxOption`] in the On state inside its open list, for
    /// the field's `aria-activedescendant`.
    pub(super) fn combobox_active_option(&self) -> Option<(WidgetId, WidgetId)> {
        let field = self.focused_combobox()?;
        let list = self.tree.find(self.float_of(field)?)?;
        active_option(list).map(|option| (field, option))
    }

    /// A press outside the anchor of open floating elements sends their
    /// dismiss messages, innermost first. Returns whether the press stops
    /// there: it does when one of them contains the focus (a menu), as a
    /// press outside the session's pop-up lists does; otherwise it goes on
    /// to what is under the pointer.
    pub(super) fn dismiss_floats_outside(&mut self) -> bool {
        let outside: Vec<(A::Msg, bool)> = self
            .floats
            .iter()
            .rev()
            .filter(|float| !self.hover_path.contains(&float.anchor))
            .filter_map(|float| {
                let msg = self.dismiss_message(float.id)?;
                Some((msg, float.focus == OverlayFocus::Contain))
            })
            .collect();
        let mut stops = false;
        for (msg, contains_focus) in outside {
            stops |= contains_focus;
            self.send(msg);
        }
        stops
    }
}
