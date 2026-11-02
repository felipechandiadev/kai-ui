//! The autocomplete: its model alone, then in a running window (typing,
//! the pause before a search, stale answers, keys, pointer and forms).

use std::sync::{Arc, Mutex};
use std::time::Duration;

use super::{
    AUTOCOMPLETE_DEBOUNCE, AutocompleteItem, AutocompleteMsg, AutocompleteState,
    AutocompleteStatus, autocomplete,
};
use crate::components::{ButtonVariant, button};
use crate::element::{Element, Role, div};
use crate::forms::{Field, FormMsg, FormState, Validators};
use crate::runtime::{App, Effects};
use crate::testing::{By, Key, Modifiers, TestApp};

const CITIES: [&str; 5] = ["Arica", "Valparaíso", "Valdivia", "Valle Hermoso", "Temuco"];

fn cities() -> AutocompleteState<&'static str> {
    AutocompleteState::list("ciudad", CITIES)
}

fn input(state: &mut AutocompleteState<&'static str>, text: &str) {
    let _ = state.update(AutocompleteMsg::Input(text.into()));
}

#[test]
fn a_list_filters_without_case_or_accents_and_up_to_its_limit() {
    let mut city = cities();
    input(&mut city, "VALPARAISO");
    assert_eq!(city.options(), ["Valparaíso"]);
    assert!(city.is_open());
    input(&mut city, "val");
    assert_eq!(city.options(), ["Valparaíso", "Valdivia", "Valle Hermoso"]);

    let mut short = cities().limit(2);
    input(&mut short, "a");
    assert_eq!(short.options().len(), 2);

    input(&mut city, "zzz");
    assert!(city.options().is_empty());
    assert_eq!(city.status(), &AutocompleteStatus::Ready);
    assert!(city.is_open(), "the list says there are no results");
}

#[test]
fn fewer_characters_than_min_chars_close_the_list() {
    let mut city = cities().min_chars(2);
    input(&mut city, "v");
    assert!(!city.is_open());
    assert_eq!(city.status(), &AutocompleteStatus::Idle);
    let _ = city.update(AutocompleteMsg::Next);
    assert!(!city.is_open(), "↓ does not open it either");
    input(&mut city, "va");
    assert!(city.is_open());

    // With 0, ↓ on an empty field shows the first options.
    let mut any = cities();
    let _ = any.update(AutocompleteMsg::Next);
    assert!(any.is_open());
    assert_eq!(any.options().len(), CITIES.len());
}

#[test]
fn arrows_move_the_active_option_and_return_picks_it() {
    let mut city = cities();
    input(&mut city, "val");
    assert_eq!(city.active(), None);
    for (msg, active) in [
        (AutocompleteMsg::Next, Some(0)),
        (AutocompleteMsg::Next, Some(1)),
        (AutocompleteMsg::Next, Some(2)),
        (AutocompleteMsg::Next, Some(2)),
        (AutocompleteMsg::Previous, Some(1)),
        (AutocompleteMsg::Previous, Some(0)),
        (AutocompleteMsg::Previous, None),
        (AutocompleteMsg::Previous, Some(2)),
    ] {
        let _ = city.update(msg.clone());
        assert_eq!(city.active(), active, "after {msg:?}");
    }
    let _ = city.update(AutocompleteMsg::Accept);
    assert_eq!(city.selected(), Some(&"Valle Hermoso"));
    assert_eq!(city.query(), "Valle Hermoso");
    assert!(!city.is_open());

    // Typing other text forgets the choice; the same text keeps it.
    input(&mut city, "Valle Hermoso");
    assert_eq!(city.selected(), Some(&"Valle Hermoso"));
    input(&mut city, "Valle");
    assert_eq!(city.selected(), None);
}

#[test]
fn dismiss_keeps_the_text_and_clear_empties_it() {
    let mut city = cities();
    input(&mut city, "val");
    let _ = city.update(AutocompleteMsg::Next);
    let _ = city.update(AutocompleteMsg::Dismiss);
    assert!(!city.is_open());
    assert_eq!(city.active(), None);
    assert_eq!(city.query(), "val");
    let _ = city.update(AutocompleteMsg::Accept);
    assert_eq!(
        city.selected(),
        None,
        "Return on a closed list picks nothing"
    );
    let _ = city.update(AutocompleteMsg::Next);
    assert!(city.is_open(), "↓ opens it again");
    let _ = city.update(AutocompleteMsg::Clear);
    assert_eq!(city.query(), "");
    assert!(!city.is_open());
    assert!(city.options().is_empty());
}

#[test]
fn an_answer_to_an_older_query_is_dropped() {
    let mut client: AutocompleteState<String> =
        AutocompleteState::search("cliente", |_| Ok(Vec::new()));
    let _ = client.update(AutocompleteMsg::Input("an".into()));
    let first = client.generation();
    assert_eq!(client.status(), &AutocompleteStatus::Loading);
    let _ = client.update(AutocompleteMsg::Input("ana".into()));
    let second = client.generation();
    assert_ne!(first, second);

    // The pause and the answer of "an" arrive late: both are ignored.
    assert!(client.update(AutocompleteMsg::Search(first)).is_none());
    let _ = client.update(AutocompleteMsg::Loaded(first, Ok(vec!["Andrés".into()])));
    assert!(client.options().is_empty());
    assert_eq!(client.status(), &AutocompleteStatus::Loading);

    assert!(!client.update(AutocompleteMsg::Search(second)).is_none());
    let _ = client.update(AutocompleteMsg::Loaded(second, Ok(vec!["Ana".into()])));
    assert_eq!(client.options(), ["Ana"]);
    assert_eq!(client.status(), &AutocompleteStatus::Ready);

    // After a pick or a clear, an answer still on its way is stale too.
    let pending = client.generation();
    let _ = client.update(AutocompleteMsg::Pick(0));
    let _ = client.update(AutocompleteMsg::Loaded(pending, Ok(vec!["Otro".into()])));
    assert_eq!(client.options(), ["Ana"]);
    assert!(!client.is_open());
}

#[test]
fn a_failed_search_shows_its_message_and_dismiss_stops_a_pending_one() {
    let mut client: AutocompleteState<String> =
        AutocompleteState::search("cliente", |_| Ok(Vec::new()));
    let _ = client.update(AutocompleteMsg::Input("an".into()));
    let generation = client.generation();
    let _ = client.update(AutocompleteMsg::Loaded(
        generation,
        Err("Sin conexión".into()),
    ));
    assert_eq!(
        client.status(),
        &AutocompleteStatus::Failed("Sin conexión".into())
    );

    let _ = client.update(AutocompleteMsg::Input("ana".into()));
    let _ = client.update(AutocompleteMsg::Dismiss);
    assert_eq!(client.status(), &AutocompleteStatus::Idle);
    let late = client.generation() - 1;
    let _ = client.update(AutocompleteMsg::Loaded(late, Ok(vec!["Ana".into()])));
    assert!(client.options().is_empty());
}

/// A client: its name, and its RUT as the detail.
#[derive(Clone, Debug, PartialEq)]
struct Client(&'static str, &'static str);

impl AutocompleteItem for Client {
    fn label(&self) -> String {
        self.0.to_owned()
    }

    fn detail(&self) -> Option<String> {
        Some(self.1.to_owned())
    }
}

const CLIENTS: [Client; 4] = [
    Client("Ana Pérez", "11.111.111-1"),
    Client("Andrés Soto", "12.345.678-5"),
    Client("Anita Rojas", "9.876.543-3"),
    Client("Bruno Díaz", "7.654.321-K"),
];

#[derive(Clone, Debug, PartialEq)]
enum Msg {
    Client(AutocompleteMsg<Client>),
    City(AutocompleteMsg<&'static str>),
    Submit,
    Next,
    Form(FormMsg),
}

/// A form with a searched client, a city from a list and a button.
struct Lookup {
    client: AutocompleteState<Client>,
    city: AutocompleteState<&'static str>,
    /// Queries the search ran, in order.
    searched: Arc<Mutex<Vec<String>>>,
    submitted: usize,
    form: FormState,
}

impl Lookup {
    fn new() -> Self {
        let searched = Arc::new(Mutex::new(Vec::new()));
        let log = Arc::clone(&searched);
        let client = AutocompleteState::search("cliente", move |query| {
            log.lock().unwrap().push(query.to_owned());
            if query == "error" {
                return Err("No se pudo buscar".into());
            }
            let query = crate::fold::fold(query);
            Ok(CLIENTS
                .iter()
                .filter(|client| crate::fold::fold(client.0).contains(&query))
                .cloned()
                .collect())
        });
        Self {
            client,
            city: cities(),
            searched,
            submitted: 0,
            form: FormState::new().field("ciudad", Field::new().validator(Validators::required())),
        }
    }

    fn searched(&self) -> Vec<String> {
        self.searched.lock().unwrap().clone()
    }
}

impl App for Lookup {
    type Msg = Msg;

    fn title(&self) -> String {
        "Autocompletar".into()
    }

    fn update(&mut self, msg: Msg) -> Effects<Msg> {
        match msg {
            Msg::Client(msg) => self.client.update(msg).map(Msg::Client),
            Msg::City(msg) => {
                let effects = self.city.update(msg).map(Msg::City);
                self.city.write_to(&mut self.form, "ciudad");
                effects
            }
            Msg::Submit => {
                self.submitted += 1;
                Effects::none()
            }
            Msg::Next => Effects::none(),
            Msg::Form(msg) => {
                self.form.update(msg);
                Effects::none()
            }
        }
    }

    fn view(&self) -> Element<Msg> {
        div().class("flex flex-col gap-4 p-4 w-96").children([
            autocomplete("Cliente", &self.client, Msg::Client)
                .test_id("cliente")
                .field(|field| field.on_submit(Msg::Submit))
                .build(),
            self.form
                .autocomplete("ciudad", "Ciudad", &self.city, Msg::Form, Msg::City)
                .build(),
            button("Siguiente", ButtonVariant::Secondary)
                .on_click(Msg::Next)
                .test_id("next"),
        ])
    }
}

fn app() -> TestApp<Lookup> {
    TestApp::builder(Lookup::new())
        .viewport(600.0, 500.0)
        .build()
}

const CLIENT: &str = "cliente.input";
const CITY: &str = "ciudad.input";

fn plain(key: Key) -> Modifiers {
    let _ = key;
    Modifiers::NONE
}

fn press(app: &mut TestApp<Lookup>, key: Key) {
    app.press(key, plain(key));
}

fn option_names(app: &TestApp<Lookup>) -> Vec<String> {
    app.screen()
        .all(By::role(Role::ListBoxOption))
        .iter()
        .filter_map(|option| option.name())
        .collect()
}

fn list_open(app: &TestApp<Lookup>) -> bool {
    app.screen().query(By::role(Role::ListBox)).is_some()
}

#[test]
fn typing_waits_for_a_pause_and_searches_once() {
    let mut app = app();
    app.type_text(By::test_id(CLIENT), "a");
    app.advance(Duration::from_millis(100));
    app.type_text(By::test_id(CLIENT), "n");
    app.advance(Duration::from_millis(100));
    assert!(app.app().searched().is_empty(), "no search while typing");
    assert_eq!(
        app.screen()
            .get(By::test_id("cliente.status"))
            .name()
            .as_deref(),
        Some("Buscando…")
    );
    assert_eq!(
        app.screen().get(By::test_id("cliente.status")).role(),
        Role::Status
    );

    app.advance(AUTOCOMPLETE_DEBOUNCE);
    app.run_until_idle();
    assert_eq!(
        app.app().searched(),
        ["an"],
        "one search, for the last text"
    );
    assert_eq!(
        option_names(&app),
        ["Ana Pérez", "Andrés Soto", "Anita Rojas"]
    );
    assert!(
        app.is_focused(By::test_id(CLIENT)),
        "the focus stays in the field"
    );
    let field = app.screen().get(By::test_id(CLIENT));
    assert_eq!(field.role(), Role::ComboBox);
    let list = app.screen().get(By::test_id("cliente.list"));
    assert_eq!(list.name().as_deref(), Some("Cliente"));
    let shell = app.screen().get(By::test_id("cliente.shell")).bounds();
    assert!(
        (list.bounds().width - shell.width).abs() < 0.5,
        "as wide as the field"
    );
    assert!(list.bounds().y >= shell.y + shell.height, "under the field");
}

#[test]
fn a_slow_answer_to_older_text_never_shows() {
    let mut app = app();
    app.type_text(By::test_id(CLIENT), "an");
    app.advance(AUTOCOMPLETE_DEBOUNCE);
    // The search of "an" has started; the person keeps typing before it
    // answers, and the answer comes in only now.
    app.type_text(By::test_id(CLIENT), "d");
    app.run_until_idle();
    assert!(option_names(&app).is_empty(), "still waiting for the pause");
    assert_eq!(
        app.screen()
            .get(By::test_id("cliente.status"))
            .name()
            .as_deref(),
        Some("Buscando…")
    );
    app.advance(AUTOCOMPLETE_DEBOUNCE);
    app.run_until_idle();
    assert_eq!(option_names(&app), ["Andrés Soto"]);
    let loaded = app
        .messages()
        .iter()
        .filter(|msg| matches!(msg, Msg::Client(AutocompleteMsg::Loaded(..))))
        .count();
    assert_eq!(loaded, 1, "the answer to \"an\" never reaches update");
    assert_eq!(app.app().searched().last().map(String::as_str), Some("and"));
}

#[test]
fn no_results_and_errors_show_in_the_list() {
    let mut app = app();
    app.type_text(By::test_id(CLIENT), "zz");
    app.advance(AUTOCOMPLETE_DEBOUNCE);
    app.run_until_idle();
    assert_eq!(
        app.screen()
            .get(By::test_id("cliente.status"))
            .name()
            .as_deref(),
        Some("Sin resultados")
    );

    let mut app = self::app();
    app.type_text(By::test_id(CLIENT), "error");
    app.advance(AUTOCOMPLETE_DEBOUNCE);
    app.run_until_idle();
    let status = app.screen().get(By::test_id("cliente.status"));
    assert_eq!(status.role(), Role::Alert);
    assert_eq!(status.name().as_deref(), Some("No se pudo buscar"));
}

#[test]
fn arrows_and_return_pick_with_the_focus_in_the_field() {
    let mut app = app();
    app.type_text(By::test_id(CITY), "val");
    assert_eq!(
        option_names(&app),
        ["Valparaíso", "Valdivia", "Valle Hermoso"]
    );
    press(&mut app, Key::ArrowDown);
    press(&mut app, Key::ArrowDown);
    assert_eq!(app.app().city.active(), Some(1));
    let active = |app: &TestApp<Lookup>| {
        app.screen()
            .all(By::role(Role::ListBoxOption))
            .iter()
            .filter(|option| {
                app.session()
                    .tree()
                    .find(option.id())
                    .is_some_and(Element::is_pressed)
            })
            .filter_map(|option| option.name())
            .collect::<Vec<_>>()
    };
    assert_eq!(active(&app), ["Valdivia"]);
    press(&mut app, Key::ArrowUp);
    assert_eq!(active(&app), ["Valparaíso"]);
    assert!(app.is_focused(By::test_id(CITY)));
    assert_eq!(
        app.value(By::test_id(CITY)),
        "val",
        "↑ did not move the caret's text"
    );

    press(&mut app, Key::Enter);
    assert_eq!(app.value(By::test_id(CITY)), "Valparaíso");
    assert!(!list_open(&app));
    assert_eq!(app.app().form.value("ciudad"), Some("Valparaíso"));
    assert!(app.is_focused(By::test_id(CITY)));
    assert_eq!(app.app().submitted, 0);
}

#[test]
fn assistive_tech_reads_the_active_option_and_the_expanded_state() {
    let mut app = app();
    app.type_text(By::test_id(CITY), "val");
    press(&mut app, Key::ArrowDown);
    let field = app.screen().get(By::test_id(CITY)).id();
    let option = app.screen().get(By::test_id("ciudad.option.0")).id();
    let update = app.session().a11y_tree(1.0);
    let node = |id| {
        update
            .nodes
            .iter()
            .find(|(node, _)| *node == crate::a11y::node_id(id))
            .map(|(_, node)| node.clone())
            .unwrap()
    };
    assert_eq!(node(field).role(), accesskit::Role::EditableComboBox);
    assert_eq!(node(field).is_expanded(), Some(true));
    assert_eq!(
        node(field).active_descendant(),
        Some(crate::a11y::node_id(option))
    );
    assert_eq!(node(option).role(), accesskit::Role::ListBoxOption);
    assert_eq!(node(option).is_selected(), Some(true));
    assert_eq!(node(option).label(), Some("Valparaíso"));
    let list = app.screen().get(By::test_id("ciudad.list")).id();
    assert_eq!(node(list).role(), accesskit::Role::ListBox);

    press(&mut app, Key::Escape);
    let update = app.session().a11y_tree(1.0);
    let field = update
        .nodes
        .iter()
        .find(|(node, _)| *node == crate::a11y::node_id(field))
        .map(|(_, node)| node.clone())
        .unwrap();
    assert_eq!(field.is_expanded(), Some(false));
    assert_eq!(field.active_descendant(), None);
}

#[test]
fn return_without_an_active_option_submits_the_field() {
    let mut app = app();
    app.type_text(By::test_id(CLIENT), "an");
    app.advance(AUTOCOMPLETE_DEBOUNCE);
    app.run_until_idle();
    press(&mut app, Key::Enter);
    assert_eq!(app.app().submitted, 1);
    assert_eq!(app.app().client.selected(), None);
    press(&mut app, Key::ArrowDown);
    press(&mut app, Key::Enter);
    assert_eq!(app.app().submitted, 1);
    assert_eq!(
        app.app().client.selected(),
        Some(&Client("Ana Pérez", "11.111.111-1"))
    );
}

#[test]
fn escape_closes_then_clears() {
    let mut app = app();
    app.type_text(By::test_id(CITY), "val");
    press(&mut app, Key::ArrowDown);
    press(&mut app, Key::Escape);
    assert!(!list_open(&app));
    assert_eq!(app.value(By::test_id(CITY)), "val");
    press(&mut app, Key::ArrowDown);
    assert!(list_open(&app), "↓ opens it again");
    press(&mut app, Key::Escape);
    press(&mut app, Key::Escape);
    assert_eq!(app.value(By::test_id(CITY)), "");
    assert!(app.is_focused(By::test_id(CITY)));
}

#[test]
fn tab_closes_the_list_without_picking_and_moves_on() {
    let mut app = app();
    app.type_text(By::test_id(CITY), "val");
    press(&mut app, Key::ArrowDown);
    press(&mut app, Key::Tab);
    assert!(!list_open(&app));
    assert!(
        app.is_focused(By::test_id("next")),
        "the next control, not an option"
    );
    assert_eq!(app.app().city.selected(), None);
    assert_eq!(app.value(By::test_id(CITY)), "val");
    assert_eq!(
        app.app().form.error("ciudad").as_deref(),
        Some("Este campo es obligatorio"),
        "leaving the field touched it; nothing was picked"
    );

    // Shift+Tab goes back to the client field the same way.
    app.type_text(By::test_id(CITY), "a");
    assert!(list_open(&app));
    app.press(
        Key::Tab,
        Modifiers {
            shift: true,
            ..Modifiers::NONE
        },
    );
    assert!(!list_open(&app));
    assert!(app.is_focused(By::test_id(CLIENT)));
}

#[test]
fn a_click_picks_and_a_click_outside_dismisses() {
    let mut app = app();
    app.type_text(By::test_id(CITY), "val");
    app.hover(By::test_id("ciudad.option.2"));
    app.click(By::test_id("ciudad.option.2"));
    assert_eq!(app.value(By::test_id(CITY)), "Valle Hermoso");
    assert!(!list_open(&app));
    assert!(
        app.is_focused(By::test_id(CITY)),
        "the click kept the focus"
    );
    assert_eq!(app.app().form.error("ciudad"), None);

    app.type_text(By::test_id(CITY), "x");
    assert!(list_open(&app));
    assert_eq!(app.app().city.selected(), None);
    assert_eq!(
        app.app().form.value("ciudad"),
        Some(""),
        "typing forgot the pick"
    );
    // The list covers the button under the field; the client field is
    // outside it.
    app.click(By::test_id(CLIENT));
    assert!(!list_open(&app));
    assert!(app.is_focused(By::test_id(CLIENT)));
    assert_eq!(app.value(By::test_id(CITY)), "Valle Hermosox");
}
