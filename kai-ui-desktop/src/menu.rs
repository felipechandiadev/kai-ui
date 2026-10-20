//! The menu bar, built from the command registry (13, Paso 8): one menu
//! per [`CommandState::menu`], in registration order. [`menu_model`] is the
//! platform-free part; on macOS `NativeMenu` turns it into an `NSMenu`
//! with muda.
//!
//! There is no Edit menu: AppKit's predefined Copy and Paste items would
//! take Cmd+C and Cmd+V from Kai's own text fields.

use crate::commands::{Availability, CommandKind, CommandState};
use crate::element::KeyEquivalent;

/// A menu of the menu bar.
#[derive(Clone, Debug, PartialEq)]
pub struct MenuGroup {
    pub title: &'static str,
    pub items: Vec<MenuEntry>,
}

/// An item of a [`MenuGroup`]: choosing it runs command `id`.
#[derive(Clone, Debug, PartialEq)]
pub struct MenuEntry {
    pub id: &'static str,
    pub title: &'static str,
    pub enabled: bool,
    pub shortcut: Option<KeyEquivalent>,
}

/// Group the actions that declare a menu, skipping `Hidden` ones and those
/// that need an input; menus and items keep the order in which they were
/// registered.
pub fn menu_model(states: &[CommandState]) -> Vec<MenuGroup> {
    let mut groups: Vec<MenuGroup> = Vec::new();
    for state in states {
        let Some(menu) = state.menu else { continue };
        if state.kind != CommandKind::Action
            || state.needs_input
            || state.availability == Availability::Hidden
        {
            continue;
        }
        let entry = MenuEntry {
            id: state.id,
            title: state.title,
            enabled: state.availability.is_enabled(),
            shortcut: state.shortcut,
        };
        match groups.iter_mut().find(|group| group.title == menu) {
            Some(group) => group.items.push(entry),
            None => groups.push(MenuGroup {
                title: menu,
                items: vec![entry],
            }),
        }
    }
    groups
}

/// Whether `a` and `b` differ only in which items are enabled.
fn same_structure(a: &[MenuGroup], b: &[MenuGroup]) -> bool {
    a.len() == b.len()
        && a.iter().zip(b).all(|(a, b)| {
            a.title == b.title
                && a.items.len() == b.items.len()
                && a.items
                    .iter()
                    .zip(&b.items)
                    .all(|(a, b)| a.id == b.id && a.title == b.title && a.shortcut == b.shortcut)
        })
}

#[cfg(target_os = "macos")]
pub(crate) use native::{NativeMenu, forward_events};

#[cfg(target_os = "macos")]
mod native {
    use muda::accelerator::{Accelerator, Code, Modifiers};
    use muda::{Menu, MenuEvent, MenuItem, PredefinedMenuItem, Submenu};

    use super::{MenuGroup, same_structure};
    use crate::element::KeyEquivalent;
    use crate::ui_engine::Key;

    /// The app's `NSMenu`: the application menu plus one menu per group.
    pub(crate) struct NativeMenu {
        app_name: String,
        menu: Menu,
        model: Vec<MenuGroup>,
        /// One per entry of `model`, in order.
        items: Vec<MenuItem>,
    }

    impl NativeMenu {
        /// Replace the menu bar. Must run on the main thread, after the app
        /// finished launching (winit installs its own menu then).
        pub(crate) fn install(app_name: &str, model: Vec<MenuGroup>) -> Self {
            let (menu, items) = build(app_name, &model);
            menu.init_for_nsapp();
            Self {
                app_name: app_name.to_owned(),
                menu,
                model,
                items,
            }
        }

        /// Follow the latest command states: toggle items in place, or
        /// rebuild the bar when menus or items changed.
        pub(crate) fn update(&mut self, model: Vec<MenuGroup>) {
            if same_structure(&self.model, &model) {
                let entries = model.iter().flat_map(|group| &group.items);
                for (item, entry) in self.items.iter().zip(entries) {
                    if item.is_enabled() != entry.enabled {
                        item.set_enabled(entry.enabled);
                    }
                }
                self.model = model;
            } else {
                self.menu.remove_for_nsapp();
                *self = Self::install(&self.app_name, model);
            }
        }
    }

    /// Call `handler` with the command id of every chosen item. muda keeps
    /// one handler per process.
    pub(crate) fn forward_events(handler: impl Fn(String) + Send + Sync + 'static) {
        MenuEvent::set_event_handler(Some(move |event: MenuEvent| handler(event.id.0)));
    }

    fn build(app_name: &str, model: &[MenuGroup]) -> (Menu, Vec<MenuItem>) {
        let menu = Menu::new();
        let app_menu = Submenu::new(app_name, true);
        let separator = PredefinedMenuItem::separator();
        let _ = app_menu.append_items(&[
            &PredefinedMenuItem::about(Some(&format!("Acerca de {app_name}")), None),
            &separator,
            &PredefinedMenuItem::hide(Some(&format!("Ocultar {app_name}"))),
            &PredefinedMenuItem::hide_others(Some("Ocultar otros")),
            &PredefinedMenuItem::show_all(Some("Mostrar todo")),
            &PredefinedMenuItem::separator(),
            &PredefinedMenuItem::quit(Some(&format!("Salir de {app_name}"))),
        ]);
        let _ = menu.append(&app_menu);
        let mut items = Vec::new();
        for group in model {
            let submenu = Submenu::new(group.title, true);
            for entry in &group.items {
                let item = MenuItem::with_id(
                    entry.id,
                    entry.title,
                    entry.enabled,
                    entry.shortcut.and_then(accelerator),
                );
                let _ = submenu.append(&item);
                items.push(item);
            }
            let _ = menu.append(&submenu);
        }
        (menu, items)
    }

    /// muda's form of a shortcut; `None` for keys a menu cannot show.
    pub(crate) fn accelerator(shortcut: KeyEquivalent) -> Option<Accelerator> {
        let code = match shortcut.key {
            Key::Tab => Code::Tab,
            Key::Enter => Code::Enter,
            Key::Space => Code::Space,
            Key::ArrowLeft => Code::ArrowLeft,
            Key::ArrowRight => Code::ArrowRight,
            Key::ArrowUp => Code::ArrowUp,
            Key::ArrowDown => Code::ArrowDown,
            Key::Home => Code::Home,
            Key::End => Code::End,
            Key::PageUp => Code::PageUp,
            Key::PageDown => Code::PageDown,
            Key::Backspace => Code::Backspace,
            Key::Delete => Code::Delete,
            Key::Escape => Code::Escape,
            Key::Period => Code::Period,
            Key::A => Code::KeyA,
            Key::C => Code::KeyC,
            Key::X => Code::KeyX,
            Key::V => Code::KeyV,
            Key::Char(c) => char_code(c)?,
            Key::Other => return None,
        };
        let m = shortcut.modifiers;
        let mut mods = Modifiers::empty();
        for (on, modifier) in [
            (m.command, Modifiers::SUPER),
            (m.shift, Modifiers::SHIFT),
            (m.option, Modifiers::ALT),
            (m.control, Modifiers::CONTROL),
        ] {
            if on {
                mods |= modifier;
            }
        }
        Some(Accelerator::new(Some(mods), code))
    }

    fn char_code(c: char) -> Option<Code> {
        const LETTERS: [Code; 26] = [
            Code::KeyA,
            Code::KeyB,
            Code::KeyC,
            Code::KeyD,
            Code::KeyE,
            Code::KeyF,
            Code::KeyG,
            Code::KeyH,
            Code::KeyI,
            Code::KeyJ,
            Code::KeyK,
            Code::KeyL,
            Code::KeyM,
            Code::KeyN,
            Code::KeyO,
            Code::KeyP,
            Code::KeyQ,
            Code::KeyR,
            Code::KeyS,
            Code::KeyT,
            Code::KeyU,
            Code::KeyV,
            Code::KeyW,
            Code::KeyX,
            Code::KeyY,
            Code::KeyZ,
        ];
        const DIGITS: [Code; 10] = [
            Code::Digit0,
            Code::Digit1,
            Code::Digit2,
            Code::Digit3,
            Code::Digit4,
            Code::Digit5,
            Code::Digit6,
            Code::Digit7,
            Code::Digit8,
            Code::Digit9,
        ];
        match c {
            'a'..='z' => Some(LETTERS[c as usize - 'a' as usize]),
            '0'..='9' => Some(DIGITS[c as usize - '0' as usize]),
            _ => None,
        }
    }

    #[cfg(test)]
    mod tests {
        use muda::accelerator::{Accelerator, Code, Modifiers};

        use super::accelerator;
        use crate::element::KeyEquivalent;
        use crate::ui_engine::Key;

        #[test]
        fn shortcuts_map_to_accelerators() {
            assert_eq!(
                accelerator(KeyEquivalent::cmd(Key::Char('i'))),
                Some(Accelerator::new(Some(Modifiers::SUPER), Code::KeyI))
            );
            assert_eq!(
                accelerator(KeyEquivalent::cmd(Key::Char('0'))),
                Some(Accelerator::new(Some(Modifiers::SUPER), Code::Digit0))
            );
            assert_eq!(
                accelerator(KeyEquivalent::cmd_shift(Key::Char('p'))),
                Some(Accelerator::new(
                    Some(Modifiers::SUPER | Modifiers::SHIFT),
                    Code::KeyP
                ))
            );
            assert_eq!(accelerator(KeyEquivalent::cmd(Key::Other)), None);
            assert_eq!(accelerator(KeyEquivalent::cmd(Key::Char('ñ'))), None);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{menu_model, same_structure};
    use crate::commands::{Availability, CommandKind, CommandState};

    fn state(id: &'static str, menu: Option<&'static str>) -> CommandState {
        CommandState {
            id,
            title: id,
            description: "",
            kind: CommandKind::Action,
            safety: None,
            shortcut: None,
            menu,
            needs_input: false,
            exposed: false,
            availability: Availability::Enabled,
        }
    }

    #[test]
    fn menus_group_actions_in_registration_order() {
        let mut hidden = state("a.oculta", Some("A"));
        hidden.availability = Availability::Hidden;
        let mut query = state("b.consulta", Some("B"));
        query.kind = CommandKind::Query;
        let mut with_input = state("b.con_entrada", Some("B"));
        with_input.needs_input = true;
        let model = menu_model(&[
            state("b.uno", Some("B")),
            state("a.uno", Some("A")),
            hidden,
            query,
            with_input,
            state("sin.menu", None),
            state("b.dos", Some("B")),
        ]);
        let shape: Vec<(&str, Vec<&str>)> = model
            .iter()
            .map(|group| {
                (
                    group.title,
                    group.items.iter().map(|item| item.id).collect(),
                )
            })
            .collect();
        assert_eq!(shape, [("B", vec!["b.uno", "b.dos"]), ("A", vec!["a.uno"])]);
    }

    #[test]
    fn enabling_an_item_keeps_the_structure() {
        let before = menu_model(&[state("a.uno", Some("A")), state("a.dos", Some("A"))]);
        let mut after = before.clone();
        after[0].items[1].enabled = false;
        assert!(same_structure(&before, &after));
        after[0].items.pop();
        assert!(!same_structure(&before, &after));
    }
}
