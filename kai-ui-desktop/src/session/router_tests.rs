//! Routes in a running app: the navigation commands, and scroll and focus
//! kept per route ([`crate::router`]).

use std::rc::Rc;

use crate::commands::{Availability, CommandRegistry};
use crate::components::{ButtonVariant, TextField, button, virtual_list};
use crate::element::{Element, div, text};
use crate::router::{NavState, Navigable, Navigate, Route, Router};
use crate::runtime::{App, Effects};
use crate::testing::{By, TestApp};
use crate::ui_engine::{Key, Modifiers};

#[derive(Clone, Debug, PartialEq, Eq)]
enum Page {
    Customers,
    Customer { id: u32 },
    Sales,
}

impl Route for Page {
    fn parse(path: &str) -> Option<Self> {
        let segments: Vec<&str> = path.trim_matches('/').split('/').collect();
        match segments.as_slice() {
            ["customers"] => Some(Self::Customers),
            ["customers", id] => id.parse().ok().map(|id| Self::Customer { id }),
            ["sales"] => Some(Self::Sales),
            _ => None,
        }
    }

    fn path(&self) -> String {
        match self {
            Self::Customers => "/customers".into(),
            Self::Customer { id } => format!("/customers/{id}"),
            Self::Sales => "/sales".into(),
        }
    }

    fn parent(&self) -> Option<Self> {
        match self {
            Self::Customer { .. } => Some(Self::Customers),
            _ => None,
        }
    }
}

struct Crm {
    router: Router<Page>,
    query: String,
}

#[derive(Clone, Debug, PartialEq)]
enum Msg {
    Navigate(Navigate),
    Query(String),
}

impl App for Crm {
    type Msg = Msg;

    fn commands(registry: &mut CommandRegistry<Self>) {
        registry.navigation();
    }

    fn title(&self) -> String {
        "CRM".into()
    }

    fn route(&self) -> Option<String> {
        Some(self.router.current().path())
    }

    fn update(&mut self, msg: Msg) -> Effects<Msg> {
        match msg {
            Msg::Navigate(to) => {
                self.router.apply(to);
            }
            Msg::Query(query) => self.query = query,
        }
        Effects::none()
    }

    fn view(&self) -> Element<Msg> {
        let page = match self.router.current() {
            Page::Customers => {
                let rows = Rc::new(|index: usize| {
                    button(format!("Cliente {index}"), ButtonVariant::Text)
                        .on_click(Msg::Navigate(Navigate::To(format!("/customers/{index}"))))
                });
                div().class("flex flex-col flex-1 min-h-0").children([
                    TextField::new("Buscar", &self.query)
                        .on_input(Msg::Query)
                        .test_id("customers.find")
                        .build(),
                    virtual_list(200, 32.0, rows)
                        .class("flex-1")
                        .test_id("customers.list"),
                ])
            }
            Page::Customer { id } => text(format!("Ficha del cliente {id}")).test_id("customer"),
            Page::Sales => text("Ventas").test_id("sales"),
        };
        div().class("flex flex-col w-full h-full").child(page)
    }
}

impl Navigable for Crm {
    fn nav_state(&self) -> NavState {
        self.router.state()
    }

    fn navigate(request: Navigate) -> Msg {
        Msg::Navigate(request)
    }
}

const CMD: Modifiers = Modifiers {
    command: true,
    ..Modifiers::NONE
};

fn crm() -> TestApp<Crm> {
    TestApp::new(Crm {
        router: Router::new(Page::Customers),
        query: String::new(),
    })
}

fn availability(app: &TestApp<Crm>, id: &str) -> bool {
    app.session()
        .commands()
        .iter()
        .find(|command| command.id == id)
        .is_some_and(|command| command.availability == Availability::Enabled)
}

#[test]
fn the_commands_walk_the_history_and_say_when_they_cannot() {
    let mut app = crm();
    assert_eq!(app.route(), Some("/customers"));
    assert!(!availability(&app, "kai.back"));
    assert!(!availability(&app, "kai.up"));

    app.click(By::text("Cliente 3"));
    assert_eq!(app.route(), Some("/customers/3"));
    assert!(availability(&app, "kai.back"));
    assert!(availability(&app, "kai.up"));

    assert!(app.perform("kai.back"));
    assert_eq!(app.route(), Some("/customers"));
    assert!(availability(&app, "kai.forward"));
    assert!(app.perform("kai.forward"));
    assert_eq!(app.route(), Some("/customers/3"));
    assert!(app.perform("kai.up"));
    assert_eq!(app.route(), Some("/customers"));

    // Agents and tests go by path; an unknown one changes nothing.
    app.navigate("/sales");
    assert_eq!(app.route(), Some("/sales"));
    app.screen().get(By::test_id("sales"));
    app.navigate("/nowhere");
    assert_eq!(app.route(), Some("/sales"));
}

#[test]
fn the_shortcuts_are_command_brackets_and_command_up() {
    let mut app = crm();
    app.navigate("/customers/5");
    app.press(Key::Char('['), CMD);
    assert_eq!(app.route(), Some("/customers"));
    app.press(Key::Char(']'), CMD);
    assert_eq!(app.route(), Some("/customers/5"));
    app.press(Key::ArrowUp, CMD);
    assert_eq!(app.route(), Some("/customers"));
}

#[test]
fn coming_back_restores_the_scroll_and_the_focus() {
    let mut app = crm();
    app.type_text(By::test_id("customers.find"), "ana");
    app.scroll_wheel(By::test_id("customers.list"), -900.0);
    let scrolled = app.session().scroll().clone();
    assert!(
        scrolled.values().any(|offset| offset.y > 0.0),
        "the list scrolled: {scrolled:?}"
    );
    assert!(app.is_focused(By::test_id("customers.find")));

    app.navigate("/customers/40");
    app.screen().get(By::test_id("customer"));
    assert!(
        app.session()
            .scroll()
            .values()
            .all(|offset| offset.y == 0.0),
        "a new route starts at the top"
    );

    assert!(app.perform("kai.back"));
    assert_eq!(app.session().scroll(), &scrolled);
    assert!(app.is_focused(By::test_id("customers.find")));
}
