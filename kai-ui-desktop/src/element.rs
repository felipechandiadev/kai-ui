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

use std::borrow::Cow;
use std::collections::HashMap;
use std::fmt;
use std::rc::Rc;

use crate::icons::Icon;
use crate::responder::Action;
use crate::ui_engine::{Key, Modifiers};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct WidgetId(pub u64);

/// `NSControl.ControlSize`. Inherited down the tree like SwiftUI's
/// `.controlSize()`: an element without its own size takes its nearest
/// ancestor's, and the root takes `Theme::control_size` (`Regular`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ControlSize {
    Mini,
    Small,
    #[default]
    Regular,
    Large,
    ExtraLarge,
}

impl ControlSize {
    pub const ALL: [ControlSize; 5] = [
        Self::Mini,
        Self::Small,
        Self::Regular,
        Self::Large,
        Self::ExtraLarge,
    ];

    /// Position in [`ControlSize::ALL`], for per-size tables (`[f32; 5]`).
    pub const fn index(self) -> usize {
        self as usize
    }
}

/// `NSControl.StateValue`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ControlState {
    #[default]
    Off,
    On,
    Mixed,
}

/// Shortcut that activates a control even when it has no focus
/// (`keyEquivalent` + `keyEquivalentModifierMask`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KeyEquivalent {
    pub key: Key,
    pub modifiers: Modifiers,
}

impl KeyEquivalent {
    /// Return: the window's default button.
    pub const RETURN: Self = Self {
        key: Key::Enter,
        modifiers: Modifiers::NONE,
    };
    /// Escape: the cancel button.
    pub const ESCAPE: Self = Self {
        key: Key::Escape,
        modifiers: Modifiers::NONE,
    };

    /// Command + `key`.
    pub const fn cmd(key: Key) -> Self {
        Self {
            key,
            modifiers: Modifiers {
                command: true,
                ..Modifiers::NONE
            },
        }
    }

    /// Command + Shift + `key`.
    pub const fn cmd_shift(key: Key) -> Self {
        Self {
            key,
            modifiers: Modifiers {
                command: true,
                shift: true,
                ..Modifiers::NONE
            },
        }
    }

    /// Menu-style label (`⌘S`, `↩`, `⎋`), exposed to assistive technology.
    pub fn label(&self) -> String {
        let m = self.modifiers;
        let mut out = String::new();
        for (on, symbol) in [
            (m.control, '⌃'),
            (m.option, '⌥'),
            (m.shift, '⇧'),
            (m.command, '⌘'),
        ] {
            if on {
                out.push(symbol);
            }
        }
        match self.key {
            Key::Enter => out.push('↩'),
            Key::Escape => out.push('⎋'),
            Key::Tab => out.push('⇥'),
            Key::Space => out.push_str("Space"),
            Key::Backspace => out.push('⌫'),
            Key::Delete => out.push('⌦'),
            Key::ArrowLeft => out.push('←'),
            Key::ArrowRight => out.push('→'),
            Key::ArrowUp => out.push('↑'),
            Key::ArrowDown => out.push('↓'),
            Key::Home => out.push('↖'),
            Key::End => out.push('↘'),
            Key::Period => out.push('.'),
            Key::A => out.push('A'),
            Key::C => out.push('C'),
            Key::X => out.push('X'),
            Key::V => out.push('V'),
            Key::Char(c) => out.extend(c.to_uppercase()),
            Key::Other => {}
        }
        out
    }
}

/// What every `NSControl` has in common, without class inheritance.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ControlProps {
    /// `None` = inherit from the nearest ancestor that sets it (root:
    /// `Regular`).
    pub size: Option<ControlSize>,
    /// `None` = not a two- or three-state control.
    pub state: Option<ControlState>,
    /// The window's default button: Return activates it.
    pub is_default: bool,
    pub key_equivalent: Option<KeyEquivalent>,
}

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
    /// On/off toggle (`NSSwitch`); its state is the control state.
    Switch,
    /// Checkbox (`NSButton` `.checkbox`): On, Off or Mixed, from the control
    /// state; named after its title.
    Checkbox,
    /// One choice among several (`NSSegmentedControl` select-one); its
    /// [`Role::Radio`] items carry the state.
    RadioGroup,
    /// Item of a [`Role::RadioGroup`]: On when selected.
    Radio,
    /// Button that opens a list to pick from (`NSPopUpButton`); named
    /// after the current choice.
    PopUpButton,
    /// The open list of a [`Role::PopUpButton`].
    Menu,
    /// Option of a [`Role::Menu`]: On when it is the current choice.
    MenuItem,
    TextInput,
    Image,
    /// Modal panel over the window; its name is its title.
    Dialog,
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
    /// Standard actions this element handles when they travel the
    /// responder chain (`None` = handles none).
    pub on_action: Option<Rc<dyn Fn(Action) -> Option<Msg>>>,
    /// Pop-up button: the option picked from its list, by index.
    pub on_select: Option<Rc<dyn Fn(usize) -> Msg>>,
}

impl<Msg> Default for Handlers<Msg> {
    fn default() -> Self {
        Self {
            on_click: None,
            on_input: None,
            on_submit: None,
            on_action: None,
            on_select: None,
        }
    }
}

impl<Msg: Clone> Clone for Handlers<Msg> {
    fn clone(&self) -> Self {
        Self {
            on_click: self.on_click.clone(),
            on_input: self.on_input.clone(),
            on_submit: self.on_submit.clone(),
            on_action: self.on_action.clone(),
            on_select: self.on_select.clone(),
        }
    }
}

impl<Msg: fmt::Debug> fmt::Debug for Handlers<Msg> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Handlers")
            .field("on_click", &self.on_click)
            .field("on_input", &self.on_input.is_some())
            .field("on_submit", &self.on_submit)
            .field("on_action", &self.on_action.is_some())
            .field("on_select", &self.on_select.is_some())
            .finish()
    }
}

/// The list of a pop-up button ([`Element::options`]).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Options {
    pub items: Vec<String>,
    /// The current choice, checked in the list.
    pub selected: Option<usize>,
    /// The list is open; the session sets it while it shows the list.
    pub open: bool,
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
    /// Stable id for tests and UI automation (AccessKit `author_id`, macOS
    /// `accessibilityIdentifier`). Unique per window; not part of the
    /// [`WidgetId`].
    pub test_id: Option<String>,
    /// Name of this element inside its component (`"input"`, `"header"`…).
    /// Without its own `test_id`, it gets `<owner>.<part>` (see
    /// [`Element::assign_ids`]).
    pub part: Option<Cow<'static, str>>,
    /// Component root: parts below it derive from its `test_id`, or get
    /// none when it has no id.
    pub parts_root: bool,
    /// `test_id` was derived from `part`, not set by the app.
    derived_test_id: bool,
    pub disabled: bool,
    /// Size, on/off/mixed state, default button and key equivalent.
    pub control: ControlProps,
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
    /// Its controls are one Tab stop; see [`Element::focus_group`].
    pub focus_group: bool,
    /// Pop-up button list; see [`Element::options`].
    pub options: Option<Options>,
    pub handlers: Handlers<Msg>,
    /// Command this control runs ([`Element::on_command`]); a control even
    /// without a message, as the palette's rows, which the session runs.
    pub command: Option<&'static str>,
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
            test_id: None,
            part: None,
            parts_root: false,
            derived_test_id: false,
            disabled: false,
            control: ControlProps::default(),
            required: false,
            description: None,
            delegates_focus: false,
            focus_on_press: true,
            focus_group: false,
            options: None,
            handlers: Handlers::default(),
            command: None,
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

    /// Stable id for tests and automation, e.g. `"form.enviar"` or
    /// `"cliente.42.borrar"`: lowercase segments separated by `.`, no
    /// personal data (assistive tools can read it), `kai.*` reserved.
    pub fn test_id(mut self, test_id: impl Into<String>) -> Self {
        self.test_id = Some(test_id.into());
        self.derived_test_id = false;
        self
    }

    /// Name this element inside its component; see [`Element::part`].
    pub fn part(mut self, part: impl Into<Cow<'static, str>>) -> Self {
        self.part = Some(part.into());
        self
    }

    /// Mark a component root; see [`Element::parts_root`].
    pub fn parts_root(mut self) -> Self {
        self.parts_root = true;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// The opposite of [`Element::disabled`] (`NSControl.isEnabled`).
    pub fn enabled(self, enabled: bool) -> Self {
        self.disabled(!enabled)
    }

    /// Toggle buttons: sugar for `state(On | Off)`.
    pub fn pressed(self, pressed: bool) -> Self {
        self.state(if pressed {
            ControlState::On
        } else {
            ControlState::Off
        })
    }

    /// Whether the control is in the `On` state (`aria-pressed="true"`).
    pub fn is_pressed(&self) -> bool {
        self.control.state == Some(ControlState::On)
    }

    /// Two- or three-state control (`NSControl.state`).
    pub fn state(mut self, state: ControlState) -> Self {
        self.control.state = Some(state);
        self
    }

    /// Control size for this element and the descendants that do not set
    /// their own.
    pub fn control_size(mut self, size: ControlSize) -> Self {
        self.control.size = Some(size);
        self
    }

    /// Mark as the window's default button: Return activates it (it keeps
    /// any key equivalent already set).
    pub fn default_button(mut self, on: bool) -> Self {
        self.control.is_default = on;
        if on {
            self.control
                .key_equivalent
                .get_or_insert(KeyEquivalent::RETURN);
        } else if self.control.key_equivalent == Some(KeyEquivalent::RETURN) {
            self.control.key_equivalent = None;
        }
        self
    }

    /// Escape activates this control.
    pub fn cancel_button(self) -> Self {
        self.key_equivalent(KeyEquivalent::ESCAPE)
    }

    /// Shortcut that activates this control without focus.
    pub fn key_equivalent(mut self, key: KeyEquivalent) -> Self {
        self.control.key_equivalent = Some(key);
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

    /// Make the controls inside one Tab stop, as `NSSegmentedControl`: Tab
    /// enters at the selected one (`On`, else the first) and leaves the
    /// group; the arrow keys move the focus between them, and Space or
    /// Return activates the focused one.
    pub fn focus_group(mut self) -> Self {
        self.focus_group = true;
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

    /// Run an app command on click. While the command is not available the
    /// control is disabled, with the reason as its accessible description;
    /// its `test_id` defaults to the command id and, without text, its
    /// accessible name to the command title.
    pub fn on_command<C>(mut self, command: C) -> Self
    where
        C: crate::commands::Action,
        C::App: crate::runtime::App<Msg = Msg>,
    {
        self.command = Some(C::ID);
        self.handlers.on_click = Some(command.into_msg());
        self
    }

    /// Message sent when Enter is pressed in an input.
    pub fn on_submit(mut self, msg: Msg) -> Self {
        self.handlers.on_submit = Some(msg);
        self
    }

    /// Make this a pop-up button (`NSPopUpButton`) over `items`, with
    /// `selected` checked: a click, Space, Return or an arrow key opens the
    /// list, which the session shows and drives (arrows, Return, Escape,
    /// click outside); picking sends [`Element::on_select`].
    pub fn options(mut self, items: Vec<String>, selected: Option<usize>) -> Self {
        self.options = Some(Options {
            items,
            selected,
            open: false,
        });
        self
    }

    /// Message for the option picked from the list of
    /// [`Element::options`], by index.
    pub fn on_select(mut self, handler: impl Fn(usize) -> Msg + 'static) -> Self {
        self.handlers.on_select = Some(Rc::new(handler));
        self
    }

    /// Handle standard actions (Escape → `CancelOperation`, Cmd+Z → `Undo`…)
    /// that the focused element or a descendant did not consume. Return
    /// `None` to let the action continue up the responder chain.
    pub fn on_action(mut self, handler: impl Fn(Action) -> Option<Msg> + 'static) -> Self {
        self.handlers.on_action = Some(Rc::new(handler));
        self
    }

    /// Give every element an id derived from its parent's id and its key or
    /// index among siblings, and a `test_id` to every [`Element::part`]
    /// without one: `<owner>.<part>`, where the owner is the nearest
    /// ancestor with its own `test_id` or that is a
    /// [`Element::parts_root`] (a root without id gives its parts none).
    /// Call on the root after `view`; calling it again gives the same ids.
    pub fn assign_ids(&mut self) {
        self.assign(hash(0x6b61_6920_7569, b"root"));
        self.derive_test_ids(None);
    }

    fn derive_test_ids(&mut self, owner: Option<&str>) {
        if self.derived_test_id {
            self.test_id = None;
            self.derived_test_id = false;
        }
        let own = self.test_id.clone();
        if own.is_none()
            && let (Some(part), Some(owner)) = (&self.part, owner)
        {
            self.test_id = Some(format!("{owner}.{part}"));
            self.derived_test_id = true;
        }
        let next = match &own {
            Some(own) => Some(own.as_str()),
            None if self.parts_root => None,
            None => owner,
        };
        for child in &mut self.children {
            child.derive_test_ids(next);
        }
    }

    /// The element with this `test_id` (own or derived), in document order.
    pub fn find_test_id(&self, test_id: &str) -> Option<&Element<Msg>> {
        if self.test_id.as_deref() == Some(test_id) {
            return Some(self);
        }
        self.children
            .iter()
            .find_map(|child| child.find_test_id(test_id))
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

    /// Keys repeated among siblings (they collide into one [`WidgetId`]) and
    /// test ids repeated in the tree, each reported with the paths of both
    /// elements, e.g. `root > 2 > 0[key="ana"]`. Empty when ids are sound.
    pub fn validate_ids(&self) -> Vec<String> {
        let mut problems = Vec::new();
        self.validate("root", &mut HashMap::new(), &mut problems);
        problems
    }

    fn validate<'a>(
        &'a self,
        path: &str,
        test_ids: &mut HashMap<&'a str, String>,
        problems: &mut Vec<String>,
    ) {
        if let Some(test_id) = &self.test_id {
            match test_ids.get(test_id.as_str()) {
                Some(first) => {
                    problems.push(format!("test_id \"{test_id}\" repetido: {first} y {path}"));
                }
                None => {
                    test_ids.insert(test_id, path.to_owned());
                }
            }
        }
        let mut keys: HashMap<&str, String> = HashMap::new();
        for (index, child) in self.children.iter().enumerate() {
            let child_path = match &child.key {
                Some(key) => format!("{path} > {index}[key=\"{key}\"]"),
                None => format!("{path} > {index}"),
            };
            if let Some(key) = &child.key {
                match keys.get(key.as_str()) {
                    Some(first) => problems.push(format!(
                        "key \"{key}\" repetida entre hermanos: {first} y {child_path}"
                    )),
                    None => {
                        keys.insert(key, child_path.clone());
                    }
                }
            }
            child.validate(&child_path, test_ids, problems);
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

    pub fn find_mut(&mut self, id: WidgetId) -> Option<&mut Element<Msg>> {
        if self.id == id {
            return Some(self);
        }
        self.children
            .iter_mut()
            .find_map(|child| child.find_mut(id))
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
            _ if self.handlers.on_click.is_some()
                || self.handlers.on_select.is_some()
                || self.command.is_some() =>
            {
                Some(FocusKind::Button)
            }
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
        matches!(self.kind, Kind::Input { .. })
            || self.handlers.on_click.is_some()
            || self.handlers.on_select.is_some()
            || self.command.is_some()
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
            if h.on_click.is_some()
                || h.on_input.is_some()
                || h.on_submit.is_some()
                || h.on_action.is_some()
                || h.on_select.is_some()
            {
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
    /// Text field with `on_submit`: it keeps Return for itself.
    submits: bool,
    /// Nearest [`Element::focus_group`] ancestor.
    group: Option<WidgetId>,
    /// In the `On` state: Tab enters its group here.
    selected: bool,
}

/// Focusable elements in document order (disabled controls are skipped),
/// and the key equivalents of enabled controls.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FocusOrder {
    entries: Vec<FocusEntry>,
    equivalents: Vec<(WidgetId, KeyEquivalent)>,
    /// Shortcuts of enabled app commands, after the controls' equivalents.
    commands: Vec<(&'static str, KeyEquivalent)>,
}

impl FocusOrder {
    pub fn from_tree<Msg>(root: &Element<Msg>) -> Self {
        let mut order = Self::default();
        order.collect(root, None);
        order
    }

    fn collect<Msg>(&mut self, element: &Element<Msg>, group: Option<WidgetId>) {
        if let Some(kind) = element.focus_kind() {
            self.entries.push(FocusEntry {
                id: element.id,
                kind,
                focus_on_press: element.focus_on_press,
                submits: kind == FocusKind::TextField && element.handlers.on_submit.is_some(),
                group,
                selected: element.is_pressed(),
            });
        }
        // A button that does not take focus on press can still have an
        // equivalent.
        if let Some(equivalent) = element.control.key_equivalent
            && !element.disabled
            && element.handlers.on_click.is_some()
        {
            self.equivalents.push((element.id, equivalent));
        }
        let group = if element.focus_group {
            Some(element.id)
        } else {
            group
        };
        for child in &element.children {
            self.collect(child, group);
        }
    }

    fn members(&self, group: WidgetId) -> impl Iterator<Item = &FocusEntry> + Clone {
        self.entries
            .iter()
            .filter(move |entry| entry.group == Some(group))
    }

    /// The member Tab stops at in `group`: the selected one, else the first.
    fn group_stop(&self, group: WidgetId) -> Option<WidgetId> {
        let mut members = self.members(group);
        members
            .clone()
            .find(|entry| entry.selected)
            .or_else(|| members.next())
            .map(|entry| entry.id)
    }

    /// Where Tab counts `id` from: itself, or its group's stop.
    fn stop_of(&self, id: WidgetId) -> Option<WidgetId> {
        match self.entry(id)?.group {
            Some(group) => self.group_stop(group),
            None => Some(id),
        }
    }

    /// The neighbor of `id` inside its focus group (arrow keys), staying
    /// at the ends; `None` when `id` is in no group.
    pub fn group_step(&self, id: WidgetId, backwards: bool) -> Option<WidgetId> {
        let group = self.entry(id)?.group?;
        let members: Vec<WidgetId> = self.members(group).map(|entry| entry.id).collect();
        let index = members.iter().position(|member| *member == id)?;
        let next = if backwards {
            index.saturating_sub(1)
        } else {
            (index + 1).min(members.len() - 1)
        };
        Some(members[next])
    }

    fn entry(&self, id: WidgetId) -> Option<&FocusEntry> {
        self.entries.iter().find(|entry| entry.id == id)
    }

    pub fn kind(&self, id: WidgetId) -> Option<FocusKind> {
        self.entry(id).map(|entry| entry.kind)
    }

    /// Whether a pointer press on `id` moves focus to it.
    pub fn focus_on_press(&self, id: WidgetId) -> bool {
        self.entry(id).is_some_and(|entry| entry.focus_on_press)
    }

    /// Whether `id` is a text field with `on_submit`.
    pub fn submits(&self, id: WidgetId) -> bool {
        self.entry(id).is_some_and(|entry| entry.submits)
    }

    pub fn ids(&self) -> impl Iterator<Item = WidgetId> + '_ {
        self.entries.iter().map(|entry| entry.id)
    }

    /// Key equivalents of enabled controls, in document order.
    pub fn equivalents(&self) -> &[(WidgetId, KeyEquivalent)] {
        &self.equivalents
    }

    /// Shortcuts of the enabled commands (by id), offered to keys after
    /// every control's equivalent.
    pub fn commands(&self) -> &[(&'static str, KeyEquivalent)] {
        &self.commands
    }

    pub fn set_commands(&mut self, commands: Vec<(&'static str, KeyEquivalent)>) {
        self.commands = commands;
    }

    /// Drop the equivalents of controls `visible` rejects (hidden by
    /// `display: none`, known only after layout).
    pub fn retain_visible_equivalents(&mut self, visible: impl Fn(WidgetId) -> bool) {
        self.equivalents.retain(|(id, _)| visible(*id));
    }

    /// Tab / Shift+Tab successor, wrapping around. Without a current focus,
    /// Tab starts at the first element and Shift+Tab at the last one. A
    /// focus group is a single stop (see [`Element::focus_group`]).
    pub fn next(&self, current: Option<WidgetId>, backwards: bool) -> Option<WidgetId> {
        let stops: Vec<WidgetId> = self
            .entries
            .iter()
            .filter(|entry| {
                entry
                    .group
                    .is_none_or(|group| self.group_stop(group) == Some(entry.id))
            })
            .map(|entry| entry.id)
            .collect();
        let len = stops.len();
        if len == 0 {
            return None;
        }
        let position = current
            .and_then(|id| self.stop_of(id))
            .and_then(|id| stops.iter().position(|stop| *stop == id));
        let index = match (position, backwards) {
            (None, false) => 0,
            (None, true) => len - 1,
            (Some(index), false) => (index + 1) % len,
            (Some(index), true) => (index + len - 1) % len,
        };
        Some(stops[index])
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
    use super::{
        ControlSize, ControlState, Element, FocusKind, FocusOrder, KeyEquivalent, WidgetId, div,
        text,
    };
    use crate::geometry::Size;
    use crate::layout::testing::FixedMeasure;
    use crate::layout::{Interaction, Layout, LayoutInput, ScrollOffsets, StyleCache};
    use crate::theme::{Density, Theme};
    use crate::ui_engine::Key;

    #[test]
    fn control_size_is_inherited() {
        let mut tree: Element<Msg> = div().children([
            div()
                .control_size(ControlSize::Small)
                .child(div().child(div().on_click(Msg::Clicked)))
                .child(div().control_size(ControlSize::Large)),
            div().on_click(Msg::Clicked),
        ]);
        tree.assign_ids();
        let layout = Layout::compute(
            &tree,
            &LayoutInput {
                viewport: Size {
                    width: 200.0,
                    height: 200.0,
                },
                theme: &Theme::kai(Density::Comfortable),
                interaction: &Interaction::default(),
                scroll: &ScrollOffsets::new(),
            },
            &mut StyleCache::default(),
            &mut FixedMeasure,
        );
        let size = |id: WidgetId| layout.node(id).unwrap().control_size;
        let small = &tree.children[0];
        assert_eq!(size(tree.id), ControlSize::Regular);
        assert_eq!(size(small.children[0].children[0].id), ControlSize::Small);
        assert_eq!(size(small.children[1].id), ControlSize::Large);
        assert_eq!(size(tree.children[1].id), ControlSize::Regular);
        assert_eq!(
            ControlSize::ALL[ControlSize::Large.index()],
            ControlSize::Large
        );
    }

    #[test]
    fn default_button_sets_return_equivalent() {
        let button: Element<Msg> = div().on_click(Msg::Clicked).default_button(true);
        assert!(button.control.is_default);
        assert_eq!(button.control.key_equivalent, Some(KeyEquivalent::RETURN));
        let off = button.default_button(false);
        assert!(!off.control.is_default);
        assert_eq!(off.control.key_equivalent, None);

        let save = KeyEquivalent::cmd(Key::Char('s'));
        let custom: Element<Msg> = div().key_equivalent(save).default_button(true);
        assert_eq!(custom.control.key_equivalent, Some(save));
        assert_eq!(save.label(), "⌘S");
        let cancel: Element<Msg> = div().cancel_button();
        assert_eq!(cancel.control.key_equivalent, Some(KeyEquivalent::ESCAPE));
        assert_eq!(KeyEquivalent::RETURN.label(), "↩");
    }

    #[test]
    fn focus_order_collects_enabled_equivalents_and_submitting_fields() {
        use super::input;

        let mut tree: Element<Msg> = div().children([
            div()
                .on_click(Msg::Clicked)
                .focus_on_press(false)
                .default_button(true),
            div().on_click(Msg::Clicked).cancel_button().disabled(true),
            div().cancel_button(),
            input("", "a").on_submit(Msg::Submitted),
            input("", "b"),
        ]);
        tree.assign_ids();
        let ids: Vec<WidgetId> = tree.children.iter().map(|child| child.id).collect();
        let mut order = FocusOrder::from_tree(&tree);
        assert_eq!(order.equivalents(), &[(ids[0], KeyEquivalent::RETURN)]);
        assert!(order.submits(ids[3]));
        assert!(!order.submits(ids[4]));
        order.retain_visible_equivalents(|id| id != ids[0]);
        assert!(order.equivalents().is_empty());
    }

    #[test]
    fn pressed_maps_to_state_on() {
        let on: Element<Msg> = div().pressed(true);
        assert_eq!(on.control.state, Some(ControlState::On));
        assert!(on.is_pressed());
        let off: Element<Msg> = div().pressed(false);
        assert_eq!(off.control.state, Some(ControlState::Off));
        assert!(!off.is_pressed());
        let mixed: Element<Msg> = div().state(ControlState::Mixed);
        assert!(!mixed.is_pressed());
        assert!(div::<Msg>().enabled(false).disabled);
        assert_eq!(div::<Msg>().control.state, None);
    }

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
    fn a_focus_group_is_one_tab_stop_walked_by_arrows() {
        let segment = |on: bool| div().on_click(Msg::Clicked).pressed(on);
        let mut tree: Element<Msg> = div().children([
            div().on_click(Msg::Clicked),
            div()
                .focus_group()
                .children([segment(false), segment(true), segment(false)]),
            div().on_click(Msg::Clicked),
        ]);
        tree.assign_ids();
        let before = tree.children[0].id;
        let after = tree.children[2].id;
        let members: Vec<WidgetId> = tree.children[1].children.iter().map(|s| s.id).collect();
        let order = FocusOrder::from_tree(&tree);
        assert_eq!(order.ids().count(), 5, "every member can take focus");
        assert_eq!(order.next(Some(before), false), Some(members[1]));
        assert_eq!(order.next(Some(members[2]), false), Some(after));
        assert_eq!(order.next(Some(after), true), Some(members[1]));
        assert_eq!(order.next(Some(members[0]), true), Some(before));
        assert_eq!(order.group_step(members[1], true), Some(members[0]));
        assert_eq!(order.group_step(members[0], true), Some(members[0]));
        assert_eq!(order.group_step(members[2], false), Some(members[2]));
        assert_eq!(order.group_step(before, false), None);

        // Without a selected member Tab enters at the first one.
        let mut plain: Element<Msg> = div().children([div()
            .focus_group()
            .children([segment(false), segment(false)])]);
        plain.assign_ids();
        let first = plain.children[0].children[0].id;
        assert_eq!(FocusOrder::from_tree(&plain).next(None, false), Some(first));
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

    fn ids(tree: &Element<Msg>) -> Vec<WidgetId> {
        let mut ids = Vec::new();
        tree.walk(&mut |element| ids.push(element.id));
        ids
    }

    #[test]
    fn test_id_does_not_change_widget_ids() {
        let plain = sample::tree();
        let mut tagged = sample::tree();
        tagged.test_id = Some("app".to_owned());
        for (index, child) in tagged.children.iter_mut().enumerate() {
            child.test_id = Some(format!("app.{index}"));
        }
        tagged.assign_ids();
        assert_eq!(ids(&plain), ids(&tagged));
        assert!(tagged.validate_ids().is_empty());
    }

    #[test]
    fn duplicate_keys_are_reported_with_both_paths() {
        let tree: Element<Msg> = div().child(div().children([
            text("Ana").key("ana"),
            text("Luis").key("luis"),
            text("Ana bis").key("ana"),
        ]));
        assert_eq!(
            tree.validate_ids(),
            vec![
                "key \"ana\" repetida entre hermanos: root > 0 > 0[key=\"ana\"] y \
                 root > 0 > 2[key=\"ana\"]"
                    .to_owned()
            ]
        );
        let cousins: Element<Msg> = div().children([
            div().child(text("a").key("x")),
            div().child(text("b").key("x")),
        ]);
        assert!(cousins.validate_ids().is_empty(), "keys are per parent");
    }

    #[test]
    fn duplicate_test_ids_are_reported() {
        let tree: Element<Msg> = div().children([
            div().test_id("form.enviar"),
            div().child(div().test_id("form.enviar")),
            div().test_id("form.limpiar"),
        ]);
        assert_eq!(
            tree.validate_ids(),
            vec!["test_id \"form.enviar\" repetido: root > 0 y root > 1 > 0".to_owned()]
        );
    }

    /// A card-like component: root, header with a title, content.
    fn widget(test_id: Option<&str>) -> Element<Msg> {
        let mut root = div().parts_root().children([
            div().part("header").child(text("Título").part("title")),
            div().part("content").child(text("Cuerpo")),
        ]);
        root.test_id = test_id.map(str::to_owned);
        root
    }

    fn test_ids(tree: &Element<Msg>) -> Vec<String> {
        let mut out = Vec::new();
        tree.walk(&mut |element| out.extend(element.test_id.clone()));
        out
    }

    #[test]
    fn parts_derive_from_the_nearest_explicit_test_id() {
        let mut tree: Element<Msg> = div()
            .test_id("pantalla")
            .children([widget(Some("x")), div().child(text("y").part("nota"))]);
        tree.assign_ids();
        assert_eq!(
            test_ids(&tree),
            [
                "pantalla",
                "x",
                "x.header",
                "x.title",
                "x.content",
                "pantalla.nota"
            ]
        );
        assert_eq!(
            tree.find_test_id("x.title").map(|e| e.text_content()),
            Some("Título".to_owned())
        );
        assert!(tree.validate_ids().is_empty());
    }

    #[test]
    fn explicit_test_id_wins_over_part() {
        let mut card = widget(Some("x"));
        card.children[1] = div().part("content").test_id("x.cuerpo");
        let mut tree: Element<Msg> = div().child(card);
        tree.assign_ids();
        assert_eq!(test_ids(&tree), ["x", "x.header", "x.title", "x.cuerpo"]);
    }

    #[test]
    fn assign_ids_twice_keeps_derived_ids() {
        let mut tree: Element<Msg> = div().children([widget(Some("a")), widget(Some("b"))]);
        tree.assign_ids();
        let first = test_ids(&tree);
        tree.assign_ids();
        assert_eq!(test_ids(&tree), first);

        tree.children[1].test_id = Some("a".to_owned());
        tree.assign_ids();
        assert_eq!(
            tree.validate_ids(),
            [
                "test_id \"a\" repetido: root > 0 y root > 1",
                "test_id \"a.header\" repetido: root > 0 > 0 y root > 1 > 0",
                "test_id \"a.title\" repetido: root > 0 > 0 > 0 y root > 1 > 0 > 0",
                "test_id \"a.content\" repetido: root > 0 > 1 y root > 1 > 1",
            ]
        );
    }

    #[test]
    fn parts_without_owner_get_no_test_id() {
        let mut tree: Element<Msg> = div().child(widget(None));
        tree.assign_ids();
        assert!(test_ids(&tree).is_empty());

        let mut inside: Element<Msg> = div().test_id("form").child(widget(None));
        inside.assign_ids();
        assert_eq!(
            test_ids(&inside),
            ["form"],
            "a component root without id does not borrow its ancestor's"
        );
    }
}
