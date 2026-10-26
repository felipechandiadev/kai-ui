//! An app of sections and pages with a root written by hand: what
//! `kai-build` will generate from `src/sections/` (docs/dev/26, R3).

use crate::commands::CommandRegistry;
use crate::components::{ButtonVariant, Sidebar, TextField, button};
use crate::element::{Element, div, text};
use crate::router::{Context, NavState, Navigable, Navigate, Page, Router, SectionSidebar};
use crate::runtime::{App, Effects};
use crate::testing::{By, Key, Modifiers, TestApp};

// --- src/state.rs ----------------------------------------------------------

/// What every page shares.
pub struct State {
    customers: Vec<String>,
}

// --- src/sections/customers/ -----------------------------------------------

mod customers {
    use super::*;
    use crate::icons::Icon;
    use crate::router::SectionMeta;

    // section.rs
    pub const SECTION: SectionMeta = SectionMeta {
        title: "Clientes",
        icon: Icon::Users,
        order: 10,
        group: Some("Datos"),
    };

    // page.rs: /customers
    pub struct List {
        pub query: String,
    }

    #[derive(Clone, Debug, PartialEq)]
    pub enum Msg {
        Query(String),
        Open(usize),
    }

    impl Page for List {
        type Msg = Msg;
        type Params = ();
        type State = State;

        fn init(_: &(), _: &State) -> (Self, Effects<Msg>) {
            (
                Self {
                    query: String::new(),
                },
                Effects::none(),
            )
        }

        fn update(&mut self, msg: Msg, cx: &mut Context<'_, State>) -> Effects<Msg> {
            match msg {
                Msg::Query(query) => self.query = query,
                Msg::Open(index) => cx.navigate(format!("/customers/{index}")),
            }
            Effects::none()
        }

        fn view(&self, state: &State) -> Element<Msg> {
            let rows = state
                .customers
                .iter()
                .enumerate()
                .filter(|(_, name)| name.to_lowercase().contains(&self.query.to_lowercase()))
                .map(|(index, name)| {
                    button(name.as_str(), ButtonVariant::Text)
                        .on_click(Msg::Open(index))
                        .test_id(format!("customers.row.{index}"))
                });
            div().children(
                std::iter::once(
                    TextField::new("Buscar", &self.query)
                        .on_input(Msg::Query)
                        .test_id("customers.find")
                        .build(),
                )
                .chain(rows),
            )
        }

        fn title(&self, _: &State) -> String {
            "Clientes".into()
        }
    }

    // [id]/page.rs: /customers/:id
    pub mod detail {
        use super::super::*;

        pub struct Detail {
            index: usize,
            draft: String,
        }

        #[derive(Clone, Debug, PartialEq)]
        pub enum Msg {
            Draft(String),
            Rename,
        }

        impl Page for Detail {
            type Msg = Msg;
            type Params = usize;
            type State = State;

            fn init(index: &usize, state: &State) -> (Self, Effects<Msg>) {
                let draft = state.customers.get(*index).cloned().unwrap_or_default();
                (
                    Self {
                        index: *index,
                        draft,
                    },
                    Effects::none(),
                )
            }

            fn update(&mut self, msg: Msg, cx: &mut Context<'_, State>) -> Effects<Msg> {
                match msg {
                    Msg::Draft(draft) => self.draft = draft,
                    Msg::Rename => {
                        if let Some(name) = cx.state().customers.get_mut(self.index) {
                            name.clone_from(&self.draft);
                        }
                        cx.back();
                    }
                }
                Effects::none()
            }

            fn view(&self, _: &State) -> Element<Msg> {
                div().children([
                    TextField::new("Nombre", &self.draft)
                        .on_input(Msg::Draft)
                        .test_id("customer.name")
                        .build(),
                    button("Guardar", ButtonVariant::Primary)
                        .on_click(Msg::Rename)
                        .test_id("customer.save"),
                ])
            }

            fn title(&self, state: &State) -> String {
                state.customers.get(self.index).cloned().unwrap_or_default()
            }
        }
    }
}

// --- src/sections/sales/ ----------------------------------------------------

mod sales {
    use super::*;
    use crate::icons::Icon;
    use crate::router::SectionMeta;

    pub const SECTION: SectionMeta = SectionMeta {
        title: "Ventas",
        icon: Icon::Wallet,
        order: 20,
        group: Some("Datos"),
    };

    /// Loads its total when it opens.
    pub struct Sales {
        pub total: Option<u32>,
    }

    #[derive(Clone, Debug, PartialEq)]
    pub enum Msg {
        Loaded(u32),
    }

    impl Page for Sales {
        type Msg = Msg;
        type Params = ();
        type State = State;

        fn init(_: &(), _: &State) -> (Self, Effects<Msg>) {
            (Self { total: None }, Effects::perform(|| 1250, Msg::Loaded))
        }

        fn update(&mut self, msg: Msg, _: &mut Context<'_, State>) -> Effects<Msg> {
            match msg {
                Msg::Loaded(total) => self.total = Some(total),
            }
            Effects::none()
        }

        fn view(&self, _: &State) -> Element<Msg> {
            let total = self
                .total
                .map_or("Cargando…".into(), |total| format!("Total {total}"));
            text(total).test_id("sales.total")
        }

        fn title(&self, _: &State) -> String {
            "Ventas".into()
        }
    }
}

// --- generated by kai-build (here by hand) -----------------------------------

#[derive(Clone, Debug, PartialEq, Eq)]
enum Route {
    Customers,
    CustomersId { id: usize },
    Sales,
}

impl crate::router::Route for Route {
    fn parse(path: &str) -> Option<Self> {
        let segments: Vec<&str> = path.trim_matches('/').split('/').collect();
        match segments.as_slice() {
            ["customers"] => Some(Self::Customers),
            ["customers", id] => id.parse().ok().map(|id| Self::CustomersId { id }),
            ["sales"] => Some(Self::Sales),
            _ => None,
        }
    }

    fn path(&self) -> String {
        match self {
            Self::Customers => "/customers".into(),
            Self::CustomersId { id } => format!("/customers/{id}"),
            Self::Sales => "/sales".into(),
        }
    }

    fn parent(&self) -> Option<Self> {
        match self {
            Self::CustomersId { .. } => Some(Self::Customers),
            _ => None,
        }
    }
}

impl Route {
    /// Index into [`SECTIONS`] of the section this route is in.
    fn section(&self) -> usize {
        match self {
            Self::Customers | Self::CustomersId { .. } => 0,
            Self::Sales => 1,
        }
    }
}

const SECTIONS: [crate::router::SectionMeta; 2] = [customers::SECTION, sales::SECTION];
/// Path of each section's own page, by index into [`SECTIONS`].
const SECTION_PATHS: [&str; 2] = ["/customers", "/sales"];

struct AppRoot {
    router: Router<Route>,
    state: State,
    sidebar: SectionSidebar,
    // Section pages live while the app runs; pages with parameters only
    // while shown.
    customers: Option<customers::List>,
    customers_id: Option<(usize, customers::detail::Detail)>,
    sales: Option<sales::Sales>,
}

#[derive(Clone, Debug, PartialEq)]
enum RootMsg {
    Navigate(Navigate),
    Sidebar(usize),
    Customers(customers::Msg),
    CustomersId(customers::detail::Msg),
    Sales(sales::Msg),
}

impl AppRoot {
    fn new(state: State) -> Self {
        Self {
            router: Router::new(Route::Customers),
            state,
            sidebar: SectionSidebar::new(&SECTIONS),
            customers: None,
            customers_id: None,
            sales: None,
        }
    }

    /// Make sure the page of the current route is alive; init it if not.
    fn open_current(&mut self) -> Effects<RootMsg> {
        match self.router.current().clone() {
            Route::Customers if self.customers.is_none() => {
                let (page, effects) = customers::List::init(&(), &self.state);
                self.customers = Some(page);
                effects.map(RootMsg::Customers)
            }
            Route::CustomersId { id }
                if self.customers_id.as_ref().map(|(at, _)| *at) != Some(id) =>
            {
                let (page, effects) = customers::detail::Detail::init(&id, &self.state);
                self.customers_id = Some((id, page));
                effects.map(RootMsg::CustomersId)
            }
            Route::Sales if self.sales.is_none() => {
                let (page, effects) = sales::Sales::init(&(), &self.state);
                self.sales = Some(page);
                effects.map(RootMsg::Sales)
            }
            _ => Effects::none(),
        }
    }

    /// Carry out navigation, drop pages with parameters that are no longer
    /// shown, and open the new one.
    fn navigate(&mut self, requests: Vec<Navigate>) -> Effects<RootMsg> {
        for request in requests {
            self.router.apply(request);
        }
        if !matches!(self.router.current(), Route::CustomersId { .. }) {
            self.customers_id = None;
        }
        self.open_current()
    }
}

impl App for AppRoot {
    type Msg = RootMsg;

    fn commands(registry: &mut CommandRegistry<Self>) {
        registry.navigation();
    }

    /// The first page opens here, so its `init` effects run.
    fn init(&mut self) -> Effects<RootMsg> {
        self.open_current()
    }

    fn title(&self) -> String {
        match self.router.current() {
            Route::Customers => self.customers.as_ref().map(|page| page.title(&self.state)),
            Route::CustomersId { .. } => self
                .customers_id
                .as_ref()
                .map(|(_, page)| page.title(&self.state)),
            Route::Sales => self.sales.as_ref().map(|page| page.title(&self.state)),
        }
        .unwrap_or_default()
    }

    fn route(&self) -> Option<String> {
        Some(crate::router::Route::path(self.router.current()))
    }

    fn update(&mut self, msg: RootMsg) -> Effects<RootMsg> {
        let mut cx = Context::new(&mut self.state);
        let effects = match msg {
            RootMsg::Navigate(request) => {
                drop(cx);
                return self.navigate(vec![request]);
            }
            RootMsg::Sidebar(row) => {
                drop(cx);
                let section = self.sidebar.section_at(row).unwrap_or(0);
                return self.navigate(vec![Navigate::To(SECTION_PATHS[section].into())]);
            }
            RootMsg::Customers(msg) => match &mut self.customers {
                Some(page) => page.update(msg, &mut cx).map(RootMsg::Customers),
                None => Effects::none(),
            },
            RootMsg::CustomersId(msg) => match &mut self.customers_id {
                Some((_, page)) => page.update(msg, &mut cx).map(RootMsg::CustomersId),
                None => Effects::none(),
            },
            RootMsg::Sales(msg) => match &mut self.sales {
                Some(page) => page.update(msg, &mut cx).map(RootMsg::Sales),
                None => Effects::none(),
            },
        };
        let requests = cx.into_requests();
        Effects::batch([effects, self.navigate(requests)])
    }

    fn view(&self) -> Element<RootMsg> {
        let page = match self.router.current() {
            Route::Customers => self
                .customers
                .as_ref()
                .map(|page| page.view(&self.state).map(RootMsg::Customers)),
            Route::CustomersId { .. } => self
                .customers_id
                .as_ref()
                .map(|(_, page)| page.view(&self.state).map(RootMsg::CustomersId)),
            Route::Sales => self
                .sales
                .as_ref()
                .map(|page| page.view(&self.state).map(RootMsg::Sales)),
        }
        .unwrap_or_else(div);
        let selected = self
            .sidebar
            .row_of(self.router.current().section())
            .unwrap_or(0);
        div().class("flex flex-row w-full h-full").children([
            Sidebar::new(&self.sidebar.groups, selected, RootMsg::Sidebar)
                .test_id("nav")
                .build()
                .class("w-56"),
            div().class("flex-1").child(page),
        ])
    }
}

impl Navigable for AppRoot {
    fn nav_state(&self) -> NavState {
        self.router.state()
    }

    fn has_route(path: &str) -> bool {
        <Route as crate::router::Route>::parse(path).is_some()
    }

    fn navigate(request: Navigate) -> RootMsg {
        RootMsg::Navigate(request)
    }
}

fn crm() -> TestApp<AppRoot> {
    TestApp::new(AppRoot::new(State {
        customers: vec!["Ana Pérez".into(), "Bruno Díaz".into(), "Carla Soto".into()],
    }))
}

#[test]
fn a_page_navigates_and_changes_the_shared_state() {
    let mut app = crm();
    assert_eq!(app.route(), Some("/customers"));

    app.click(By::test_id("customers.row.1"));
    assert_eq!(app.route(), Some("/customers/1"));
    assert_eq!(app.value(By::test_id("customer.name")), "Bruno Díaz");

    // The detail renames in the shared state and goes back on its own.
    app.type_text(By::test_id("customer.name"), " Rojas");
    app.click(By::test_id("customer.save"));
    assert_eq!(app.route(), Some("/customers"));
    assert_eq!(app.app().state.customers[1], "Bruno Díaz Rojas");
    app.screen().get(By::text("Bruno Díaz Rojas"));
    // The page with parameters is gone; the section page stays.
    assert!(app.app().customers_id.is_none());
}

#[test]
fn a_section_keeps_its_state_while_another_is_shown() {
    let mut app = crm();
    app.type_text(By::test_id("customers.find"), "car");
    assert!(app.screen().query(By::test_id("customers.row.0")).is_none());

    // The sidebar goes to Ventas, whose page loads its total on opening.
    app.click(By::test_id("nav.1"));
    assert_eq!(app.route(), Some("/sales"));
    app.run_until_idle();
    assert_eq!(
        app.screen()
            .get(By::test_id("sales.total"))
            .name()
            .as_deref(),
        Some("Total 1250")
    );

    // Back in Clientes the search is still there, typed by the person.
    app.click(By::test_id("nav.0"));
    assert_eq!(app.value(By::test_id("customers.find")), "car");
    assert!(app.screen().query(By::test_id("customers.row.0")).is_none());
    app.screen().get(By::test_id("customers.row.2"));
}

#[test]
fn the_sidebar_follows_the_route_and_the_title_the_page() {
    let mut app = crm();
    app.navigate("/customers/2");
    assert_eq!(app.app().title(), "Carla Soto");
    assert!(app.perform("kai.up"));
    assert_eq!(app.app().title(), "Clientes");
    app.navigate("/sales");
    assert_eq!(app.app().router.current().section(), 1);
    assert!(app.perform("kai.back"));
    assert_eq!(app.route(), Some("/customers"));
}

/// Agents and scripts change the view with `kai.navigate`, and the
/// sidebar follows; a path no route takes leaves the view as it was.
#[test]
fn kai_navigate_shows_a_route_and_the_sidebar_follows() {
    let mut app = crm();
    let row = |app: &TestApp<AppRoot>, index: usize| {
        app.screen()
            .get(By::test_id(format!("nav.{index}")))
            .node()
            .control_state
    };
    assert_eq!(
        app.call("kai.navigate", serde_json::json!({"path": "/sales"})),
        Ok(serde_json::json!(null))
    );
    assert_eq!(app.route(), Some("/sales"));
    assert_eq!(row(&app, 1), Some(crate::element::ControlState::On));
    assert_eq!(row(&app, 0), Some(crate::element::ControlState::Off));
    // «Ventas» loads its total in a task: the call stays in course till then.
    app.run_until_idle();

    app.call("kai.navigate", serde_json::json!({"path": "/customers/1"}))
        .unwrap();
    assert_eq!(app.app().title(), "Bruno Díaz");

    let _ = app.call("kai.navigate", serde_json::json!({"path": "/nowhere"}));
    assert_eq!(app.route(), Some("/customers/1"), "an unknown path");
    assert!(app.call("kai.navigate", serde_json::json!({})).is_err());
}

/// The arrows walk the sidebar's rows and each one shows its section.
#[test]
fn the_arrows_walk_the_sidebar_sections() {
    let mut app = crm();
    app.click(By::test_id("nav.0"));
    assert!(app.is_focused(By::test_id("nav.0")));
    app.press(Key::ArrowDown, Modifiers::NONE);
    assert_eq!(app.route(), Some("/sales"));
    assert!(app.is_focused(By::test_id("nav.1")));
    app.screen().get(By::test_id("sales.total"));
    app.press(Key::ArrowUp, Modifiers::NONE);
    assert_eq!(app.route(), Some("/customers"));
}

/// Coming back to a section with ↑ keeps the focus on the row ↑ chose,
/// not the one the route kept (↓ had already moved it to «Ventas»).
#[test]
fn the_focus_stays_on_the_row_the_arrows_chose() {
    let mut app = crm();
    app.click(By::test_id("nav.0"));
    app.press(Key::ArrowDown, Modifiers::NONE);
    app.press(Key::ArrowUp, Modifiers::NONE);
    assert_eq!(app.route(), Some("/customers"));
    assert!(app.is_focused(By::test_id("nav.0")));
}
