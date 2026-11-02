//! Autocomplete (combobox): a text field that offers options as one types,
//! from a list in memory or from a search off the UI thread, in a list
//! that floats under the field (docs/dev/32).
//!
//! The app keeps an [`AutocompleteState`] per field, passes its
//! [`AutocompleteMsg`]s back to [`AutocompleteState::update`] and draws it
//! with [`autocomplete`]. The focus never leaves the field: ↑ and ↓ move
//! the active option (`aria-activedescendant`), Return or a click picks
//! it, Escape closes the list and a second Escape clears the text. Tab
//! closes the list without picking and goes on to the next control.

use std::fmt;
use std::rc::Rc;
use std::sync::Arc;
use std::time::Duration;

use super::{TextField, themed};
use crate::element::{Element, Kind, Role, div, text};
use crate::fold::fold;
use crate::forms::{FormMsg, FormState};
use crate::overlay::Floating;
use crate::responder::Action;
use crate::runtime::Effects;

/// What an [`AutocompleteState`] offers: anything with a label.
pub trait AutocompleteItem: Clone + Send + 'static {
    /// Text of its option, and of the field once it is picked.
    fn label(&self) -> String;

    /// Second, quieter line of its option (a RUT, a commune…).
    fn detail(&self) -> Option<String> {
        None
    }
}

impl AutocompleteItem for String {
    fn label(&self) -> String {
        self.clone()
    }
}

impl AutocompleteItem for &'static str {
    fn label(&self) -> String {
        (*self).to_owned()
    }
}

/// A search run off the UI thread: the options for a query, or the
/// message to show when it fails.
type Search<T> = Arc<dyn Fn(&str) -> Result<Vec<T>, String> + Send + Sync>;

#[derive(Clone)]
enum Source<T> {
    /// Filtered in place, as one types.
    List(Vec<T>),
    /// Asked with `Effects::perform` once typing pauses.
    Search(Search<T>),
}

/// Where the options of an [`AutocompleteState`] are.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AutocompleteStatus {
    /// Nothing asked yet, or fewer characters than
    /// [`AutocompleteState::min_chars`].
    Idle,
    /// Waiting for the pause in typing or for the search.
    Loading,
    /// The options of the last query (perhaps none).
    Ready,
    /// The search failed with this message.
    Failed(String),
}

/// Messages of an [`AutocompleteState`]. The app wraps them in its own
/// `Msg` and hands them back to [`AutocompleteState::update`] as they are.
#[derive(Clone, Debug, PartialEq)]
pub enum AutocompleteMsg<T> {
    /// The text of the field changed.
    Input(String),
    /// Typing paused: search the query of this generation.
    Search(u64),
    /// The search of this generation answered.
    Loaded(u64, Result<Vec<T>, String>),
    /// ↓: open the list, or activate the next option.
    Next,
    /// ↑: activate the previous option (from the first, back to the text).
    Previous,
    /// Return: pick the active option.
    Accept,
    /// A click on this option.
    Pick(usize),
    /// Escape, a click outside or Tab: close the list.
    Dismiss,
    /// Escape with the list closed: empty the field.
    Clear,
}

/// Model of an autocomplete field: the typed text, the options, the active
/// one and the picked one. With [`AutocompleteState::search`] it waits for
/// a pause in typing ([`AutocompleteState::debounce`]) and runs the search
/// on the task pool; an answer to an older query is dropped (each query
/// has a generation, and a new one cancels the pending timer and task).
///
/// ```
/// use kai::components::{AutocompleteMsg, AutocompleteState};
///
/// let mut city = AutocompleteState::list("ciudad", ["Arica", "Valparaíso", "Valdivia"]);
/// let _ = city.update(AutocompleteMsg::Input("val".into()));
/// assert_eq!(city.options(), ["Valparaíso", "Valdivia"]);
/// let _ = city.update(AutocompleteMsg::Next);
/// let _ = city.update(AutocompleteMsg::Accept);
/// assert_eq!(city.selected(), Some(&"Valparaíso"));
/// assert_eq!(city.query(), "Valparaíso");
/// ```
#[derive(Clone)]
pub struct AutocompleteState<T> {
    key: String,
    source: Source<T>,
    min_chars: usize,
    debounce: Duration,
    limit: usize,
    query: String,
    options: Vec<T>,
    status: AutocompleteStatus,
    open: bool,
    active: Option<usize>,
    selected: Option<T>,
    generation: u64,
}

impl<T: fmt::Debug> fmt::Debug for AutocompleteState<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AutocompleteState")
            .field("key", &self.key)
            .field("query", &self.query)
            .field("options", &self.options)
            .field("status", &self.status)
            .field("open", &self.open)
            .field("active", &self.active)
            .field("selected", &self.selected)
            .field("generation", &self.generation)
            .finish_non_exhaustive()
    }
}

/// Pause in typing before a search ([`AutocompleteState::debounce`]).
pub const AUTOCOMPLETE_DEBOUNCE: Duration = Duration::from_millis(250);

/// Options shown at most ([`AutocompleteState::limit`]).
pub const AUTOCOMPLETE_LIMIT: usize = 8;

impl<T: AutocompleteItem> AutocompleteState<T> {
    fn new(key: impl Into<String>, source: Source<T>, min_chars: usize) -> Self {
        Self {
            key: key.into(),
            source,
            min_chars,
            debounce: AUTOCOMPLETE_DEBOUNCE,
            limit: AUTOCOMPLETE_LIMIT,
            query: String::new(),
            options: Vec::new(),
            status: AutocompleteStatus::Idle,
            open: false,
            active: None,
            selected: None,
            generation: 0,
        }
    }

    /// Options from `items`, those whose label contains the typed text
    /// (without case or accents). With an empty field ↓ shows the first
    /// ones. `key` names its timers and tasks: unique per window.
    pub fn list(key: impl Into<String>, items: impl IntoIterator<Item = T>) -> Self {
        Self::new(key, Source::List(items.into_iter().collect()), 0)
    }

    /// Options from `search`, run with `Effects::perform` once typing
    /// pauses and at least two characters are typed. It runs off the UI
    /// thread, so it may block (a database, an HTTP call); `Err` is the
    /// message the list shows. `key` names its timers and tasks: unique
    /// per window.
    pub fn search(
        key: impl Into<String>,
        search: impl Fn(&str) -> Result<Vec<T>, String> + Send + Sync + 'static,
    ) -> Self {
        Self::new(key, Source::Search(Arc::new(search)), 2)
    }

    /// Characters to type before it looks for options (0: ↓ on an empty
    /// field shows them).
    pub fn min_chars(mut self, min_chars: usize) -> Self {
        self.min_chars = min_chars;
        self
    }

    /// Pause in typing before a search ([`AUTOCOMPLETE_DEBOUNCE`]).
    pub fn debounce(mut self, debounce: Duration) -> Self {
        self.debounce = debounce;
        self
    }

    /// Options shown at most ([`AUTOCOMPLETE_LIMIT`]).
    pub fn limit(mut self, limit: usize) -> Self {
        self.limit = limit.max(1);
        self
    }

    /// Start with `item` picked, its label in the field.
    pub fn with_selected(mut self, item: T) -> Self {
        self.query = item.label();
        self.selected = Some(item);
        self
    }

    /// The text in the field.
    pub fn query(&self) -> &str {
        &self.query
    }

    /// The options of the last query.
    pub fn options(&self) -> &[T] {
        &self.options
    }

    pub fn status(&self) -> &AutocompleteStatus {
        &self.status
    }

    /// Whether the list is open.
    pub fn is_open(&self) -> bool {
        self.open
    }

    /// The option ↑ and ↓ point at, by index into [`Self::options`].
    pub fn active(&self) -> Option<usize> {
        self.active
    }

    /// The option picked last; typing other text forgets it.
    pub fn selected(&self) -> Option<&T> {
        self.selected.as_ref()
    }

    /// Generation of the current query: an answer from another one is
    /// stale.
    pub fn generation(&self) -> u64 {
        self.generation
    }

    fn debounce_key(&self) -> String {
        format!("kai.autocomplete.{}.debounce", self.key)
    }

    fn task_key(&self) -> String {
        format!("kai.autocomplete.{}.search", self.key)
    }

    /// Forget the pending timer and search: their answers would be stale.
    fn cancel(&mut self) -> Effects<AutocompleteMsg<T>> {
        self.generation += 1;
        Effects::batch([
            Effects::cancel(self.debounce_key()),
            Effects::cancel(self.task_key()),
        ])
    }

    fn long_enough(&self) -> bool {
        self.query.chars().count() >= self.min_chars
    }

    /// Filter the list in place, or start a search right away.
    fn look_up(&mut self) -> Effects<AutocompleteMsg<T>> {
        match &self.source {
            Source::List(items) => {
                let query = fold(self.query.trim());
                self.options = items
                    .iter()
                    .filter(|item| fold(&item.label()).contains(&query))
                    .take(self.limit)
                    .cloned()
                    .collect();
                self.status = AutocompleteStatus::Ready;
                Effects::none()
            }
            Source::Search(_) => {
                let cancel = self.cancel();
                self.options.clear();
                self.status = AutocompleteStatus::Loading;
                Effects::batch([cancel, self.run_search(self.generation)])
            }
        }
    }

    fn run_search(&self, generation: u64) -> Effects<AutocompleteMsg<T>> {
        let Source::Search(search) = &self.source else {
            return Effects::none();
        };
        let search = Arc::clone(search);
        let query = self.query.trim().to_owned();
        let limit = self.limit;
        Effects::perform(
            move || {
                search(&query).map(|mut options| {
                    options.truncate(limit);
                    options
                })
            },
            move |result| AutocompleteMsg::Loaded(generation, result),
        )
        .keyed(self.task_key())
    }

    /// Apply `msg`. The effects (the pause before a search, the search)
    /// go back to the app's `update`, mapped to its `Msg`.
    pub fn update(&mut self, msg: AutocompleteMsg<T>) -> Effects<AutocompleteMsg<T>> {
        match msg {
            AutocompleteMsg::Input(query) => {
                if self
                    .selected
                    .as_ref()
                    .is_some_and(|item| item.label() != query)
                {
                    self.selected = None;
                }
                self.query = query;
                self.active = None;
                let cancel = self.cancel();
                if !self.long_enough() {
                    self.open = false;
                    self.options.clear();
                    self.status = AutocompleteStatus::Idle;
                    return cancel;
                }
                self.open = true;
                match self.source {
                    Source::List(_) => Effects::batch([cancel, self.look_up()]),
                    Source::Search(_) => {
                        self.options.clear();
                        self.status = AutocompleteStatus::Loading;
                        Effects::batch([
                            cancel,
                            Effects::after(self.debounce, AutocompleteMsg::Search(self.generation))
                                .keyed(self.debounce_key()),
                        ])
                    }
                }
            }
            AutocompleteMsg::Search(generation) => {
                if generation != self.generation {
                    return Effects::none();
                }
                self.run_search(generation)
            }
            AutocompleteMsg::Loaded(generation, result) => {
                if generation != self.generation {
                    // Stale: the text changed since this search started.
                    return Effects::none();
                }
                match result {
                    Ok(mut options) => {
                        options.truncate(self.limit);
                        self.options = options;
                        self.status = AutocompleteStatus::Ready;
                    }
                    Err(message) => {
                        self.options.clear();
                        self.status = AutocompleteStatus::Failed(message);
                    }
                }
                self.active = None;
                Effects::none()
            }
            AutocompleteMsg::Next => {
                if !self.open {
                    if !self.long_enough() {
                        return Effects::none();
                    }
                    self.open = true;
                    self.active = None;
                    if self.status == AutocompleteStatus::Ready
                        && matches!(self.source, Source::Search(_))
                    {
                        return Effects::none();
                    }
                    return self.look_up();
                }
                let count = self.options.len();
                if count > 0 {
                    self.active = Some(self.active.map_or(0, |index| (index + 1).min(count - 1)));
                }
                Effects::none()
            }
            AutocompleteMsg::Previous => {
                let count = self.options.len();
                if self.open && count > 0 {
                    self.active = match self.active {
                        None => Some(count - 1),
                        Some(0) => None,
                        Some(index) => Some(index - 1),
                    };
                }
                Effects::none()
            }
            AutocompleteMsg::Accept => match self.active {
                Some(index) if self.open => self.pick(index),
                _ => Effects::none(),
            },
            AutocompleteMsg::Pick(index) => self.pick(index),
            AutocompleteMsg::Dismiss => {
                self.open = false;
                self.active = None;
                if self.status == AutocompleteStatus::Loading {
                    self.status = AutocompleteStatus::Idle;
                    return self.cancel();
                }
                Effects::none()
            }
            AutocompleteMsg::Clear => {
                self.query.clear();
                self.selected = None;
                self.options.clear();
                self.status = AutocompleteStatus::Idle;
                self.open = false;
                self.active = None;
                self.cancel()
            }
        }
    }

    fn pick(&mut self, index: usize) -> Effects<AutocompleteMsg<T>> {
        let Some(item) = self.options.get(index).cloned() else {
            return Effects::none();
        };
        self.query = item.label();
        self.selected = Some(item);
        self.open = false;
        self.active = None;
        self.cancel()
    }

    /// Put the label of the picked option in field `name` of `form`, or
    /// empty it while nothing is picked: its validators see the choice,
    /// not the typed text. Call it after [`Self::update`].
    pub fn write_to(&self, form: &mut FormState, name: &str) {
        let value = self.selected.as_ref().map(T::label).unwrap_or_default();
        if form.value(name) != Some(value.as_str()) {
            form.set(name, &value);
        }
    }
}

/// Builder of an autocomplete field ([`autocomplete`]).
pub struct Autocomplete<Msg> {
    field: TextField<Msg>,
    label: String,
    open: bool,
    has_text: bool,
    status: AutocompleteStatus,
    /// Label and detail of each option.
    options: Vec<(String, Option<String>)>,
    active: Option<usize>,
    empty_text: String,
    loading_text: String,
    disabled: bool,
    send: Rc<dyn Fn(Intent) -> Msg>,
}

/// The messages the view sends, which carry no options.
#[derive(Clone, Copy)]
enum Intent {
    Next,
    Previous,
    Accept,
    Pick(usize),
    Dismiss,
    Clear,
}

/// Autocomplete field `label` showing `state`: a [`TextField`] with the
/// [`Role::ComboBox`] role and, while open, its list in the floating layer
/// under it, as wide as the field ([`Floating::match_width`]). `wrap`
/// turns its messages into the app's (`Msg::City`).
///
/// Parts: `<id>.input`, `<id>.list`, `<id>.option.<index>` and
/// `<id>.status` (searching, no results or the error).
pub fn autocomplete<Msg: Clone + 'static, T: AutocompleteItem>(
    label: impl Into<String>,
    state: &AutocompleteState<T>,
    wrap: impl Fn(AutocompleteMsg<T>) -> Msg + 'static,
) -> Autocomplete<Msg> {
    let label = label.into();
    let wrap = Rc::new(wrap);
    let field = {
        let wrap = Rc::clone(&wrap);
        TextField::new(label.clone(), &state.query)
            .on_input(move |value| wrap(AutocompleteMsg::Input(value)))
    };
    let send: Rc<dyn Fn(Intent) -> Msg> = Rc::new(move |intent| {
        wrap(match intent {
            Intent::Next => AutocompleteMsg::Next,
            Intent::Previous => AutocompleteMsg::Previous,
            Intent::Accept => AutocompleteMsg::Accept,
            Intent::Pick(index) => AutocompleteMsg::Pick(index),
            Intent::Dismiss => AutocompleteMsg::Dismiss,
            Intent::Clear => AutocompleteMsg::Clear,
        })
    });
    Autocomplete {
        field,
        label,
        open: state.open,
        has_text: !state.query.is_empty(),
        status: state.status.clone(),
        options: state
            .options
            .iter()
            .map(|item| (item.label(), item.detail()))
            .collect(),
        active: state.active,
        empty_text: "Sin resultados".into(),
        loading_text: "Buscando…".into(),
        disabled: false,
        send,
    }
}

/// The first element under `element` (itself included) that `matches`.
fn find_mut<'a, Msg>(
    element: &'a mut Element<Msg>,
    matches: &dyn Fn(&Element<Msg>) -> bool,
) -> Option<&'a mut Element<Msg>> {
    if matches(element) {
        return Some(element);
    }
    element
        .children
        .iter_mut()
        .find_map(|child| find_mut(child, matches))
}

impl<Msg: Clone + 'static> Autocomplete<Msg> {
    /// `test_id` of the field; its parts derive from it.
    pub fn test_id(mut self, test_id: impl Into<String>) -> Self {
        self.field = self.field.test_id(test_id);
        self
    }

    pub fn placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.field = self.field.placeholder(placeholder);
        self
    }

    pub fn helper_text(mut self, text: impl Into<String>) -> Self {
        self.field = self.field.helper_text(text);
        self
    }

    pub fn required(mut self, required: bool) -> Self {
        self.field = self.field.required(required);
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self.field = self.field.disabled(disabled);
        self
    }

    /// Any other setting of its [`TextField`] (density, icons, `on_blur`…).
    /// Leave `on_input` alone: it feeds the autocomplete.
    pub fn field(mut self, f: impl FnOnce(TextField<Msg>) -> TextField<Msg>) -> Self {
        self.field = f(self.field);
        self
    }

    /// What the list says when the query has no options ("Sin
    /// resultados").
    pub fn empty_text(mut self, text: impl Into<String>) -> Self {
        self.empty_text = text.into();
        self
    }

    /// What the list says while it searches ("Buscando…").
    pub fn loading_text(mut self, text: impl Into<String>) -> Self {
        self.loading_text = text.into();
        self
    }

    fn list(&self) -> Element<Msg> {
        let send = &self.send;
        let status_row = |message: &str, role: Role| {
            div()
                .class(&themed(
                    "px-2 py-1.5 text-sm leading-5 text-muted-foreground",
                    "px-2 py-0.5 text-body text-secondary-label",
                ))
                .role(role)
                .label(message)
                .part("status")
                .child(text(message))
        };
        let rows: Vec<Element<Msg>> = match &self.status {
            AutocompleteStatus::Loading => vec![status_row(&self.loading_text, Role::Status)],
            AutocompleteStatus::Failed(message) => vec![status_row(message, Role::Alert)],
            AutocompleteStatus::Idle | AutocompleteStatus::Ready if self.options.is_empty() => {
                vec![status_row(&self.empty_text, Role::Status)]
            }
            AutocompleteStatus::Idle | AutocompleteStatus::Ready => self
                .options
                .iter()
                .enumerate()
                .map(|(index, (label, detail))| {
                    let active = self.active == Some(index);
                    let (kai_mark, appkit_mark) = if active {
                        ("bg-neutral", "bg-fill-2")
                    } else {
                        ("hover:bg-accent", "hover:bg-fill-2")
                    };
                    div()
                        .class(&themed(
                            &format!(
                                "flex flex-col px-2 py-1.5 rounded text-sm leading-5 \
                                 text-foreground cursor-pointer {kai_mark}"
                            ),
                            &format!(
                                "flex flex-col px-2 py-0.5 rounded-[4px] text-body text-label \
                                 cursor-default {appkit_mark}"
                            ),
                        ))
                        .role(Role::ListBoxOption)
                        .label(label.clone())
                        .pressed(active)
                        .key(index.to_string())
                        .part(format!("option.{index}"))
                        .focus_on_press(false)
                        .on_click(send(Intent::Pick(index)))
                        .child(text(label.clone()))
                        .children(detail.as_ref().map(|detail| {
                            text(detail.clone()).class(&themed(
                                "text-xs leading-4 text-muted-foreground",
                                "text-[11px] text-secondary-label",
                            ))
                        }))
                })
                .collect(),
        };
        div()
            .class(&themed(
                "flex flex-col p-1 rounded-md border border-border bg-background shadow-lg",
                "flex flex-col p-[5px] rounded-[6px] border-[0.5px] border-separator bg-window \
                 shadow-lg",
            ))
            .role(Role::ListBox)
            .label(self.label.clone())
            .part("list")
            .children(rows)
            .floating(
                Floating::below()
                    .match_width()
                    .on_dismiss(send(Intent::Dismiss)),
            )
    }

    pub fn build(self) -> Element<Msg> {
        let open = self.open && !self.disabled;
        let list = open.then(|| self.list());
        let send = Rc::clone(&self.send);
        let has_text = self.has_text;
        let disabled = self.disabled;
        let active = self.active;
        let on_action = move |action| {
            if disabled {
                return None;
            }
            let intent = match action {
                Action::MoveDown => Intent::Next,
                Action::MoveUp if open => Intent::Previous,
                // Return without an active option is the field's (submit).
                Action::InsertNewline if open && active.is_some() => Intent::Accept,
                Action::CancelOperation if open => Intent::Dismiss,
                Action::CancelOperation if has_text => Intent::Clear,
                _ => return None,
            };
            Some(send(intent))
        };
        let mut root = self.field.build();
        if let Some(input) = find_mut(&mut root, &|element| {
            matches!(element.kind, Kind::Input { .. })
        }) {
            input.role = Some(Role::ComboBox);
            input.expanded = Some(open);
            input.claims_escape = open || has_text;
            let inner = input.handlers.on_action.take();
            input.handlers.on_action = Some(Rc::new(move |action| {
                inner
                    .as_ref()
                    .and_then(|handler| handler(action))
                    .or_else(|| on_action(action))
            }));
        }
        if let Some(list) = list
            && let Some(shell) = find_mut(&mut root, &|element| {
                element.part.as_deref() == Some("shell")
            })
        {
            shell.children.push(list);
        }
        root
    }
}

/// Autocomplete fields bound to a [`FormState`].
impl FormState {
    /// [`autocomplete`] for field `name`: its error and touched state come
    /// from the form, its value is the picked option
    /// ([`AutocompleteState::write_to`]), and leaving it marks it touched.
    pub fn autocomplete<Msg: Clone + 'static, T: AutocompleteItem>(
        &self,
        name: &str,
        label: impl Into<String>,
        state: &AutocompleteState<T>,
        wrap_form: fn(FormMsg) -> Msg,
        wrap: impl Fn(AutocompleteMsg<T>) -> Msg + 'static,
    ) -> Autocomplete<Msg> {
        let error = self.error(name);
        let blur = wrap_form(FormMsg::Blur(name.to_owned()));
        autocomplete(label, state, wrap)
            .test_id(name)
            .field(move |field| {
                let field = field.on_blur(blur);
                match error {
                    Some(message) => field.error(true).helper_text(message),
                    None => field,
                }
            })
    }
}

#[cfg(test)]
#[path = "autocomplete_tests.rs"]
mod tests;
