//! DOM-like element tree returned by an app's `view`.
//!
//! Every frame the app rebuilds the whole tree from its model, as in Elm. An
//! element is a box (`div`), a text run (`text`), an icon (`icon`) or a
//! single-line input (`input`); its look is a Tailwind-like class string (`tw`), and its
//! behavior is a set of messages ([`Handlers`]) the runtime sends to the
//! app's `update`.
//!
//! [`WidgetId`]s are derived from the position in the tree (or from
//! [`Element::key`]) by [`Element::assign_ids`], so focus, hover, scroll and
//! text-editing state survive rebuilds as long as the structure does. Use
//! keys for lists whose items can move.

use std::fmt;
use std::rc::Rc;

use crate::icons::Icon;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct WidgetId(pub u64);

/// Semantic role exposed to assistive technology.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Role {
    Generic,
    Main,
    Header,
    Group,
    Heading,
    Paragraph,
    Label,
    /// Live region announcing state changes.
    Status,
    Button,
    TextInput,
    Image,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Kind {
    Div,
    Text(String),
    /// Sized by `size-*` (1em by default) and colored by the text color.
    Icon(Icon),
    Input {
        value: String,
        placeholder: String,
        /// Password entry: shown as bullets, copy and cut disabled.
        masked: bool,
        /// Selectable and copyable but not editable.
        read_only: bool,
        /// Select the whole value when the input gains focus.
        select_on_focus: bool,
    },
}

pub struct Handlers<Msg> {
    pub on_click: Option<Msg>,
    pub on_input: Option<Rc<dyn Fn(String) -> Msg>>,
    pub on_submit: Option<Msg>,
}

impl<Msg> Default for Handlers<Msg> {
    fn default() -> Self {
        Self {
            on_click: None,
            on_input: None,
            on_submit: None,
        }
    }
}

impl<Msg: Clone> Clone for Handlers<Msg> {
    fn clone(&self) -> Self {
        Self {
            on_click: self.on_click.clone(),
            on_input: self.on_input.clone(),
            on_submit: self.on_submit.clone(),
        }
    }
}

impl<Msg: fmt::Debug> fmt::Debug for Handlers<Msg> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Handlers")
            .field("on_click", &self.on_click)
            .field("on_input", &self.on_input.is_some())
            .field("on_submit", &self.on_submit)
            .finish()
    }
}

#[derive(Debug)]
pub struct Element<Msg> {
    pub id: WidgetId,
    pub key: Option<String>,
    pub kind: Kind,
    pub class: String,
    pub children: Vec<Element<Msg>>,
    pub role: Option<Role>,
    /// Accessible name; buttons default to their text content.
    pub label: Option<String>,
    pub disabled: bool,
    pub is_pressed: bool,
    /// Required form field, exposed to assistive technology.
    pub required: bool,
    /// Accessible description (e.g. helper text).
    pub description: Option<String>,
    /// A click on this element (outside any control) focuses its first
    /// enabled input, like `<label for>` or `delegatesFocus`.
    pub delegates_focus: bool,
    /// Whether a pointer press moves focus to this control (`false` =
    /// `preventDefault` on `mousedown`: it activates but keeps the focus
    /// where it was).
    pub focus_on_press: bool,
    pub handlers: Handlers<Msg>,
}

pub fn div<Msg>() -> Element<Msg> {
    Element::new(Kind::Div)
}

pub fn text<Msg>(content: impl Into<String>) -> Element<Msg> {
    Element::new(Kind::Text(content.into()))
}

pub fn icon<Msg>(icon: Icon) -> Element<Msg> {
    Element::new(Kind::Icon(icon))
}

/// Controlled single-line input: it shows `value`, and edits are reported
/// through [`Element::on_input`]; the app must feed the new value back.
pub fn input<Msg>(value: impl Into<String>, placeholder: impl Into<String>) -> Element<Msg> {
    Element::new(Kind::Input {
        value: value.into(),
        placeholder: placeholder.into(),
        masked: false,
        read_only: false,
        select_on_focus: false,
    })
}

impl<Msg> Element<Msg> {
    fn new(kind: Kind) -> Self {
        Self {
            id: WidgetId(0),
            key: None,
            kind,
            class: String::new(),
            children: Vec::new(),
            role: None,
            label: None,
            disabled: false,
            is_pressed: false,
            required: false,
            description: None,
            delegates_focus: false,
            focus_on_press: true,
            handlers: Handlers::default(),
        }
    }

    /// Append Tailwind-like classes.
    pub fn class(mut self, classes: &str) -> Self {
        if !self.class.is_empty() && !classes.is_empty() {
            self.class.push(' ');
        }
        self.class.push_str(classes);
        self
    }

    pub fn child(mut self, child: Element<Msg>) -> Self {
        self.children.push(child);
        self
    }

    pub fn children(mut self, children: impl IntoIterator<Item = Element<Msg>>) -> Self {
        self.children.extend(children);
        self
    }

    pub fn key(mut self, key: impl Into<String>) -> Self {
        self.key = Some(key.into());
        self
    }

    pub fn role(mut self, role: Role) -> Self {
        self.role = Some(role);
        self
    }

    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn pressed(mut self, pressed: bool) -> Self {
        self.is_pressed = pressed;
        self
    }

    pub fn required(mut self, required: bool) -> Self {
        self.required = required;
        self
    }

    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    pub fn delegates_focus(mut self) -> Self {
        self.delegates_focus = true;
        self
    }

    pub fn focus_on_press(mut self, focus_on_press: bool) -> Self {
        self.focus_on_press = focus_on_press;
        self
    }

    /// Inputs only: show bullets instead of the value.
    pub fn masked(mut self, on: bool) -> Self {
        if let Kind::Input { masked, .. } = &mut self.kind {
            *masked = on;
        }
        self
    }

    /// Inputs only: allow selecting and copying but not editing.
    pub fn read_only(mut self, on: bool) -> Self {
        if let Kind::Input { read_only, .. } = &mut self.kind {
            *read_only = on;
        }
        self
    }

    /// Inputs only: select the whole value on focus.
    pub fn select_on_focus(mut self, on: bool) -> Self {
        if let Kind::Input {
            select_on_focus, ..
        } = &mut self.kind
        {
            *select_on_focus = on;
        }
        self
    }

    pub fn on_click(mut self, msg: Msg) -> Self {
        self.handlers.on_click = Some(msg);
        self
    }

    pub fn on_input(mut self, handler: impl Fn(String) -> Msg + 'static) -> Self {
        self.handlers.on_input = Some(Rc::new(handler));
        self
    }

    /// Message sent when Enter is pressed in an input.
    pub fn on_submit(mut self, msg: Msg) -> Self {
        self.handlers.on_submit = Some(msg);
        self
    }

    /// Give every element an id derived from its parent's id and its key or
    /// index among siblings. Call once on the root after `view`.
    pub fn assign_ids(&mut self) {
        self.assign(hash(0x6b61_6920_7569, b"root"));
    }

    fn assign(&mut self, id: u64) {
        self.id = WidgetId(id);
        for (index, child) in self.children.iter_mut().enumerate() {
            let child_id = match &child.key {
                Some(key) => hash(id ^ 0x9e37_79b9_7f4a_7c15, key.as_bytes()),
                None => hash(id, &(index as u64).to_le_bytes()),
            };
            child.assign(child_id);
        }
    }

    /// Depth-first, pre-order traversal (document order).
    pub fn walk<'a>(&'a self, visit: &mut impl FnMut(&'a Element<Msg>)) {
        visit(self);
        for child in &self.children {
            child.walk(visit);
        }
    }

    pub fn find(&self, id: WidgetId) -> Option<&Element<Msg>> {
        if self.id == id {
            return Some(self);
        }
        self.children.iter().find_map(|child| child.find(id))
    }

    /// Ids from the root to `id`, inclusive.
    pub fn path_to(&self, id: WidgetId) -> Option<Vec<WidgetId>> {
        if self.id == id {
            return Some(vec![id]);
        }
        self.children.iter().find_map(|child| {
            child.path_to(id).map(|mut path| {
                path.insert(0, self.id);
                path
            })
        })
    }

    pub fn focus_kind(&self) -> Option<FocusKind> {
        if self.disabled {
            return None;
        }
        match self.kind {
            Kind::Input { .. } => Some(FocusKind::TextField),
            _ if self.handlers.on_click.is_some() => Some(FocusKind::Button),
            _ => None,
        }
    }

    /// First enabled input in this subtree, in document order.
    pub fn first_input(&self) -> Option<WidgetId> {
        if matches!(self.kind, Kind::Input { .. }) && !self.disabled {
            return Some(self.id);
        }
        self.children.iter().find_map(Element::first_input)
    }

    /// Clickable or editable, enabled or not: such elements are hit-test
    /// targets and expose semantic roles.
    pub fn is_control(&self) -> bool {
        matches!(self.kind, Kind::Input { .. }) || self.handlers.on_click.is_some()
    }

    /// Concatenated text of the subtree, used as default accessible name.
    pub fn text_content(&self) -> String {
        let mut parts = Vec::new();
        self.walk(&mut |element| {
            if let Kind::Text(text) = &element.kind {
                parts.push(text.as_str());
            }
        });
        parts.join(" ")
    }
}

impl<Msg: Clone> Element<Msg> {
    /// Handlers of every element that has any, by id.
    pub fn collect_handlers(&self) -> Vec<(WidgetId, Handlers<Msg>)> {
        let mut handlers = Vec::new();
        self.walk(&mut |element| {
            let h = &element.handlers;
            if h.on_click.is_some() || h.on_input.is_some() || h.on_submit.is_some() {
                handlers.push((element.id, h.clone()));
            }
        });
        handlers
    }
}

/// FNV-1a over `seed` and `bytes`, kept below 2^62 so accessibility node ids
/// (`id + 1`) never overflow.
fn hash(seed: u64, bytes: &[u8]) -> u64 {
    let mut value = 0xcbf2_9ce4_8422_2325_u64 ^ seed;
    for byte in seed.to_le_bytes().iter().chain(bytes) {
        value ^= u64::from(*byte);
        value = value.wrapping_mul(0x0100_0000_01b3);
    }
    value >> 2
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FocusKind {
    Button,
    TextField,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct FocusEntry {
    id: WidgetId,
    kind: FocusKind,
    focus_on_press: bool,
}

/// Focusable elements in document order; disabled controls are skipped.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FocusOrder(Vec<FocusEntry>);

impl FocusOrder {
    pub fn from_tree<Msg>(root: &Element<Msg>) -> Self {
        let mut order = Vec::new();
        root.walk(&mut |element| {
            if let Some(kind) = element.focus_kind() {
                order.push(FocusEntry {
                    id: element.id,
                    kind,
                    focus_on_press: element.focus_on_press,
                });
            }
        });
        Self(order)
    }

    fn entry(&self, id: WidgetId) -> Option<&FocusEntry> {
        self.0.iter().find(|entry| entry.id == id)
    }

    pub fn kind(&self, id: WidgetId) -> Option<FocusKind> {
        self.entry(id).map(|entry| entry.kind)
    }

    /// Whether a pointer press on `id` moves focus to it.
    pub fn focus_on_press(&self, id: WidgetId) -> bool {
        self.entry(id).is_some_and(|entry| entry.focus_on_press)
    }

    pub fn ids(&self) -> impl Iterator<Item = WidgetId> + '_ {
        self.0.iter().map(|entry| entry.id)
    }

    /// Tab / Shift+Tab successor, wrapping around. Without a current focus,
    /// Tab starts at the first element and Shift+Tab at the last one.
    pub fn next(&self, current: Option<WidgetId>, backwards: bool) -> Option<WidgetId> {
        let len = self.0.len();
        if len == 0 {
            return None;
        }
        let position = current.and_then(|id| self.0.iter().position(|entry| entry.id == id));
        let index = match (position, backwards) {
            (None, false) => 0,
            (None, true) => len - 1,
            (Some(index), false) => (index + 1) % len,
            (Some(index), true) => (index + len - 1) % len,
        };
        Some(self.0[index].id)
    }
}

#[cfg(test)]
pub mod sample {
    //! Tree shared by layout, paint, engine and accessibility tests.

    use super::{Element, Role, WidgetId, div, input, text};

    #[derive(Clone, Debug, PartialEq, Eq)]
    pub enum Msg {
        Clicked,
        Edited(String),
        Submitted,
    }

    pub fn tree() -> Element<Msg> {
        let mut root = div()
            .class("w-full h-full flex flex-col gap-4 p-4 bg-neutral overflow-y-auto")
            .role(Role::Main)
            .children([
                text("Kai UI Desktop")
                    .class("text-xl font-semibold")
                    .role(Role::Heading),
                text("Geometría lógica y superficie GPU en píxeles físicos"),
                div()
                    .class("inline-flex items-center h-control px-4 bg-primary text-white")
                    .on_click(Msg::Clicked)
                    .child(text("Probar interacción")),
                div()
                    .class("inline-flex items-center h-control px-4 disabled:opacity-50")
                    .on_click(Msg::Clicked)
                    .disabled(true)
                    .child(text("No disponible")),
                text("Listo").class("text-sm").role(Role::Status),
                input("hola", "Escribe aquí")
                    .class("w-full h-control px-3 border border-border focus:border-primary")
                    .on_input(Msg::Edited)
                    .on_submit(Msg::Submitted),
            ]);
        root.assign_ids();
        root
    }

    fn child(index: usize) -> WidgetId {
        tree().children[index].id
    }

    pub fn root() -> WidgetId {
        tree().id
    }
    pub fn heading() -> WidgetId {
        child(0)
    }
    pub fn description() -> WidgetId {
        child(1)
    }
    pub fn button() -> WidgetId {
        child(2)
    }
    pub fn disabled_button() -> WidgetId {
        child(3)
    }
    pub fn status() -> WidgetId {
        child(4)
    }
    pub fn field() -> WidgetId {
        child(5)
    }
    pub fn button_label() -> WidgetId {
        tree().children[2].children[0].id
    }
}

#[cfg(test)]
mod tests {
    use super::sample::{self, Msg};
    use super::{Element, FocusKind, FocusOrder, WidgetId, div, text};

    #[test]
    fn ids_are_stable_unique_and_keyed_children_keep_theirs_when_moved() {
        let first = sample::tree();
        let second = sample::tree();
        let mut ids = Vec::new();
        first.walk(&mut |element| ids.push(element.id));
        let mut again = Vec::new();
        second.walk(&mut |element| again.push(element.id));
        assert_eq!(ids, again);
        let mut unique = ids.clone();
        unique.sort();
        unique.dedup();
        assert_eq!(unique.len(), ids.len());
        assert!(ids.iter().all(|id| id.0 < 1 << 62));

        let keyed = |order: [&str; 2]| {
            let mut root: Element<Msg> = div().children(order.map(|key| text(key).key(key)));
            root.assign_ids();
            root
        };
        let ab = keyed(["a", "b"]);
        let ba = keyed(["b", "a"]);
        assert_eq!(ab.children[0].id, ba.children[1].id);
    }

    #[test]
    fn focus_order_skips_disabled_and_non_interactive_elements() {
        let tree = sample::tree();
        let order = FocusOrder::from_tree(&tree);
        assert_eq!(
            order.ids().collect::<Vec<_>>(),
            vec![sample::button(), sample::field()]
        );
        assert_eq!(order.kind(sample::button()), Some(FocusKind::Button));
        assert_eq!(order.kind(sample::field()), Some(FocusKind::TextField));
        assert_eq!(order.kind(sample::disabled_button()), None);
        assert!(tree.find(sample::disabled_button()).unwrap().is_control());
        assert!(!tree.find(sample::heading()).unwrap().is_control());
    }

    #[test]
    fn focus_order_wraps_in_both_directions() {
        let order = FocusOrder::from_tree(&sample::tree());
        let (button, field) = (sample::button(), sample::field());
        assert_eq!(order.next(None, false), Some(button));
        assert_eq!(order.next(None, true), Some(field));
        assert_eq!(order.next(Some(button), false), Some(field));
        assert_eq!(order.next(Some(field), false), Some(button));
        assert_eq!(
            order.next(Some(sample::disabled_button()), false),
            Some(button)
        );
        let empty: Element<Msg> = div();
        assert_eq!(FocusOrder::from_tree(&empty).next(None, false), None);
    }

    #[test]
    fn paths_text_content_and_handlers_follow_the_tree() {
        let tree = sample::tree();
        assert_eq!(
            tree.path_to(sample::button_label()),
            Some(vec![
                sample::root(),
                sample::button(),
                sample::button_label()
            ])
        );
        assert_eq!(tree.path_to(WidgetId(7)), None);
        assert_eq!(
            tree.find(sample::button()).unwrap().text_content(),
            "Probar interacción"
        );
        let handlers = tree.collect_handlers();
        assert_eq!(handlers.len(), 3);
        let field = &handlers
            .iter()
            .find(|(id, _)| *id == sample::field())
            .unwrap()
            .1;
        assert_eq!(
            field.on_input.as_ref().map(|f| f("x".to_owned())),
            Some(Msg::Edited("x".to_owned()))
        );
        assert_eq!(field.on_submit, Some(Msg::Submitted));
        let _ = div::<Msg>().class("").class("p-4").class("m-2");
        assert_eq!(div::<Msg>().class("p-4").class("m-2").class, "p-4 m-2");
    }
}
