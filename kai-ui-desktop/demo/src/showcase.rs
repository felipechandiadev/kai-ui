//! Component pages (/components, docs/dev/34): a registry with one
//! descriptor per component documented in `docs/componentes`. Each
//! descriptor says which props it has (and which control edits them), how
//! to draw a live instance with those props, what code writes it, which
//! variant matrix to show, and its keyboard and accessibility notes.
//! The `/components` and `/components/[id]` pages only walk the
//! registry: adding a component means adding an entry to [`COMPONENTS`].

pub mod actions;
pub mod code;
pub mod content;
pub mod data;
pub mod inputs;
pub mod navigation;
pub mod overlays;

use std::collections::HashMap;

use kai::components::{
    AlertVariant, AutocompleteMsg, AutocompleteState, Column, TableView, ViewChange,
};
use kai::datetime::Date;
use kai::element::Element;
use kai::runtime::Effects;

/// Index groups, in the order of `docs/componentes/README.md`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Group {
    Actions,
    Input,
    Content,
    Overlays,
    Navigation,
    Data,
}

impl Group {
    pub const ALL: [Group; 6] = [
        Self::Actions,
        Self::Input,
        Self::Content,
        Self::Overlays,
        Self::Navigation,
        Self::Data,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Actions => "Acciones",
            Self::Input => "Entrada de datos",
            Self::Content => "Contenido y estado",
            Self::Overlays => "Superposiciones",
            Self::Navigation => "Navegación y página",
            Self::Data => "Datos",
        }
    }
}

/// How a prop is edited in the panel, and its initial value.
#[derive(Clone, Copy, Debug)]
pub enum Control {
    /// On/off switch.
    Toggle(bool),
    /// One of several options: a segmented control for up to four short
    /// options, a select otherwise.
    Choice(&'static [&'static str], usize),
    /// Text field.
    Text(&'static str),
}

/// A component prop: `key` is its name in code (the builder method or
/// argument), `label` what the person reads in the panel.
#[derive(Clone, Copy, Debug)]
pub struct Prop {
    pub key: &'static str,
    pub label: &'static str,
    pub control: Control,
}

pub const fn toggle(key: &'static str, label: &'static str, on: bool) -> Prop {
    Prop {
        key,
        label,
        control: Control::Toggle(on),
    }
}

pub const fn choice(
    key: &'static str,
    label: &'static str,
    options: &'static [&'static str],
    selected: usize,
) -> Prop {
    Prop {
        key,
        label,
        control: Control::Choice(options, selected),
    }
}

pub const fn text(key: &'static str, label: &'static str, value: &'static str) -> Prop {
    Prop {
        key,
        label,
        control: Control::Text(value),
    }
}

/// The current value of a prop.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Value {
    Bool(bool),
    Choice(usize),
    Text(String),
}

impl Control {
    fn initial(self) -> Value {
        match self {
            Self::Toggle(on) => Value::Bool(on),
            Self::Choice(_, index) => Value::Choice(index),
            Self::Text(value) => Value::Text(value.to_owned()),
        }
    }
}

/// The props of an instance, by `key`. A missing key reads as
/// `false`, option 0 or the empty text.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Props(Vec<(&'static str, Value)>);

impl Props {
    /// The initial values of `props`.
    pub fn defaults(props: &[Prop]) -> Self {
        Self(
            props
                .iter()
                .map(|prop| (prop.key, prop.control.initial()))
                .collect(),
        )
    }

    fn get(&self, key: &str) -> Option<&Value> {
        self.0
            .iter()
            .find_map(|(k, value)| (*k == key).then_some(value))
    }

    pub fn on(&self, key: &str) -> bool {
        matches!(self.get(key), Some(Value::Bool(true)))
    }

    pub fn choice(&self, key: &str) -> usize {
        match self.get(key) {
            Some(Value::Choice(index)) => *index,
            _ => 0,
        }
    }

    pub fn text(&self, key: &str) -> &str {
        match self.get(key) {
            Some(Value::Text(text)) => text,
            _ => "",
        }
    }

    pub fn set(&mut self, key: &'static str, value: Value) {
        match self.0.iter_mut().find(|(k, _)| *k == key) {
            Some((_, slot)) => *slot = value,
            None => self.0.push((key, value)),
        }
    }

    /// These props with the changes in `set` (a matrix cell).
    pub fn with(&self, set: &[Set]) -> Self {
        let mut props = self.clone();
        for change in set {
            let (key, value) = change.value();
            props.set(key, value);
        }
        props
    }
}

/// A fixed change to one prop, for the variant matrix.
#[derive(Clone, Copy, Debug)]
pub enum Set {
    Bool(&'static str, bool),
    Choice(&'static str, usize),
    Text(&'static str, &'static str),
}

impl Set {
    fn value(self) -> (&'static str, Value) {
        match self {
            Self::Bool(key, on) => (key, Value::Bool(on)),
            Self::Choice(key, index) => (key, Value::Choice(index)),
            Self::Text(key, text) => (key, Value::Text(text.to_owned())),
        }
    }
}

/// A matrix cell: the current props with `set` on top.
#[derive(Clone, Copy, Debug)]
pub struct Variant {
    pub label: &'static str,
    pub set: &'static [Set],
}

/// A matrix row: variants, sizes or states, side by side.
#[derive(Clone, Copy, Debug)]
pub struct Axis {
    pub title: &'static str,
    pub variants: &'static [Variant],
}

/// What a preview needs besides its props.
pub struct Cx<'a> {
    /// Live page state (autocomplete list, table…).
    pub live: &'a Live,
    /// The instance's `test_id`; its parts derive from it.
    pub id: &'a str,
    /// The main instance (not a matrix cell): only it opens
    /// floating panels.
    pub main: bool,
}

impl Cx<'_> {
    /// `test_id` of a part of the instance (`<id>.<name>`).
    pub fn part(&self, name: &str) -> String {
        format!("{}.{name}", self.id)
    }
}

/// Draws an instance with these props.
pub type Preview = fn(&Props, &Cx<'_>) -> Element<PreviewMsg>;
/// The code that writes that instance.
pub type Code = fn(&Props) -> String;

/// A documented component and how its page shows it.
#[derive(Clone, Copy)]
pub struct Component {
    /// The route (`/components/<id>`) and the name of its docs page.
    pub id: &'static str,
    pub name: &'static str,
    pub group: Group,
    pub summary: &'static str,
    /// File in `docs/componentes/`.
    pub doc: &'static str,
    pub props: &'static [Prop],
    pub matrix: &'static [Axis],
    pub preview: Preview,
    pub code: Code,
    /// What the page draws over the whole window while
    /// [`Live::overlay_open`] (a modal dialog).
    pub overlay: Option<Preview>,
    pub keyboard: &'static [&'static str],
    pub accessibility: &'static [&'static str],
    /// The demo section that shows it in context: name and route.
    pub demo: Option<(&'static str, &'static str)>,
}

impl Component {
    /// Path of the docs file, from `kai-ui-desktop/`.
    pub fn doc_path(&self) -> String {
        format!("docs/componentes/{}", self.doc)
    }

    /// Has props to edit (otherwise the page is reference-only).
    pub fn is_live(&self) -> bool {
        !self.props.is_empty()
    }

    pub fn defaults(&self) -> Props {
        Props::defaults(self.props)
    }

    pub fn route(&self) -> String {
        format!("/components/{}", self.id)
    }
}

/// What a descriptor leaves as is on reference-only pages.
pub const REFERENCE: Component = Component {
    id: "",
    name: "",
    group: Group::Content,
    summary: "",
    doc: "",
    props: &[],
    matrix: &[],
    preview: |_, _| kai::components::caption("Sin vista previa"),
    code: |_| String::new(),
    overlay: None,
    keyboard: &[],
    accessibility: &[],
    demo: None,
};

/// Every component, in index order within each group.
pub static COMPONENTS: &[Component] = &[
    actions::BUTTON,
    actions::ICON_BUTTON,
    actions::TOOLBAR,
    actions::COMMAND_PALETTE,
    actions::LIST_TOOLBAR,
    inputs::TEXT_FIELD,
    inputs::TEXT_AREA,
    inputs::SEARCH_FIELD,
    inputs::AUTOCOMPLETE,
    inputs::COLOR_FIELD,
    inputs::DATE_FIELD,
    inputs::SELECT,
    inputs::SEGMENTED,
    inputs::SWITCH,
    inputs::CHECKBOX,
    inputs::MEDIA_SLOT,
    content::CARD,
    content::GROUP_BOX,
    content::STAT,
    content::TYPOGRAPHY,
    content::BADGE,
    content::ALERT,
    content::AVATAR,
    content::EMPTY_STATE,
    content::DOT_PROGRESS,
    overlays::DIALOG,
    overlays::TOAST,
    navigation::SIDEBAR,
    navigation::TABS,
    navigation::PAGE,
    navigation::PAGE_TEMPLATES,
    navigation::PANES,
    data::TABLE,
    data::VIRTUAL_LIST,
    data::CHART,
    data::MAP,
    data::DOCUMENT,
];

/// The component for the route `/components/<id>`.
pub fn find(id: &str) -> Option<&'static Component> {
    COMPONENTS.iter().find(|component| component.id == id)
}

/// What a preview asks of its page.
#[derive(Clone, Debug)]
pub enum PreviewMsg {
    /// The instance changed a prop: text typed in a field, a box
    /// checked. The panel and the code reflect it.
    Set(&'static str, Value),
    /// A stateless event (`on_click`, `on_pick`…): it is logged.
    Event(&'static str),
    Autocomplete(AutocompleteMsg<&'static str>),
    ToggleCalendar,
    Month(Date),
    Pick(Date),
    View(ViewChange),
    Row(usize),
    OpenOverlay,
    CloseOverlay(&'static str),
    /// Show a toast with this variant, title and message.
    Toast(AlertVariant, String, String),
}

/// Page state that is not a prop: what a controlled component
/// needs between messages (the autocomplete query, the calendar
/// month, the table view) and the last event.
pub struct Live {
    pub events: u32,
    pub last_event: Option<String>,
    pub autocomplete: AutocompleteState<&'static str>,
    pub calendar_open: bool,
    pub month: Date,
    pub table: TableView,
    pub row: Option<usize>,
    pub overlay_open: bool,
}

impl Default for Live {
    fn default() -> Self {
        let rows = data::communes();
        let columns: Vec<Column<data::Commune, PreviewMsg>> = data::columns();
        Self {
            events: 0,
            last_event: None,
            autocomplete: AutocompleteState::list("components.live", inputs::COMMUNES),
            calendar_open: false,
            month: inputs::today().first_of_month(),
            table: TableView::new(&rows, &columns),
            row: None,
            overlay_open: false,
        }
    }
}

impl Live {
    /// Applies `msg`: prop changes go to `props`; the page shows
    /// toasts ([`PreviewMsg::Toast`]).
    pub fn update(&mut self, msg: PreviewMsg, props: &mut Props) -> Effects<PreviewMsg> {
        match msg {
            PreviewMsg::Set(key, value) => props.set(key, value),
            PreviewMsg::Event(name) => self.event(name),
            PreviewMsg::Autocomplete(msg) => {
                let effects = self.autocomplete.update(msg);
                if let Some(picked) = self.autocomplete.selected() {
                    self.last_event = Some(format!("Elegida: {picked}"));
                }
                return effects.map(PreviewMsg::Autocomplete);
            }
            PreviewMsg::ToggleCalendar => self.calendar_open = !self.calendar_open,
            PreviewMsg::Month(month) => self.month = month,
            PreviewMsg::Pick(date) => {
                props.set("value", Value::Text(date.display()));
                self.calendar_open = false;
                self.event("on_pick");
            }
            PreviewMsg::View(change) => {
                self.table.apply(change);
                let columns: Vec<Column<data::Commune, PreviewMsg>> = data::columns();
                self.table.refresh(&data::communes(), &columns);
            }
            PreviewMsg::Row(row) => {
                self.row = Some(row);
                self.event("on_select");
            }
            PreviewMsg::OpenOverlay => self.overlay_open = true,
            PreviewMsg::CloseOverlay(name) => {
                self.overlay_open = false;
                self.event(name);
            }
            PreviewMsg::Toast(..) => self.event("toast"),
        }
        Effects::none()
    }

    fn event(&mut self, name: &str) {
        self.events += 1;
        self.last_event = Some(format!("{name} ×{}", self.events));
    }
}

/// The changed props of each component page, so they are the same when
/// coming back (`State::components`): each page is a new page every time
/// it opens.
#[derive(Default)]
pub struct Memory(HashMap<&'static str, Props>);

impl Memory {
    /// The saved props of `component`, or its initial values.
    pub fn props(&self, component: &Component) -> Props {
        self.0
            .get(component.id)
            .cloned()
            .unwrap_or_else(|| component.defaults())
    }

    pub fn remember(&mut self, component: &Component, props: &Props) {
        if *props == component.defaults() {
            self.0.remove(component.id);
        } else {
            self.0.insert(component.id, props.clone());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_are_unique_and_props_have_controls_that_fit() {
        let mut ids: Vec<_> = COMPONENTS.iter().map(|c| c.id).collect();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), COMPONENTS.len());
        for component in COMPONENTS {
            assert!(!component.name.is_empty() && !component.summary.is_empty());
            assert!(component.doc.ends_with(".md"), "{}", component.id);
            for prop in component.props {
                if let Control::Choice(options, selected) = prop.control {
                    assert!(selected < options.len(), "{}.{}", component.id, prop.key);
                }
            }
            for axis in component.matrix {
                for variant in axis.variants {
                    for set in variant.set {
                        let (key, _) = set.value();
                        assert!(
                            component.props.iter().any(|prop| prop.key == key),
                            "{}: la matriz cambia «{key}», que no es una prop",
                            component.id
                        );
                    }
                }
            }
            // Live pages write code; reference-only pages at least
            // one line of usage.
            assert!(
                !(component.code)(&component.defaults()).is_empty(),
                "{}",
                component.id
            );
        }
    }

    #[test]
    fn props_read_set_and_override() {
        let props = Props::defaults(&[
            toggle("disabled", "Deshabilitado", false),
            choice("variant", "Variante", &["A", "B"], 1),
            text("label", "Texto", "Guardar"),
        ]);
        assert!(!props.on("disabled"));
        assert_eq!(props.choice("variant"), 1);
        assert_eq!(props.text("label"), "Guardar");
        let cell = props.with(&[Set::Bool("disabled", true), Set::Text("label", "Otro")]);
        assert!(cell.on("disabled"));
        assert_eq!(cell.text("label"), "Otro");
        assert_eq!(props.text("label"), "Guardar", "the original stays");
        assert_eq!(props.text("missing"), "");
    }

    #[test]
    fn memory_forgets_props_back_at_their_defaults() {
        let button = find("button").unwrap();
        let mut memory = Memory::default();
        let mut props = memory.props(button);
        props.set("disabled", Value::Bool(true));
        memory.remember(button, &props);
        assert!(memory.props(button).on("disabled"));
        memory.remember(button, &button.defaults());
        assert_eq!(memory.0.len(), 0);
    }
}
