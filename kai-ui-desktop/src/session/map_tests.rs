//! A part with its own message type inside a parent app: [`Element::map`],
//! [`Effects::map`] and [`Subscriptions::map`].

use std::path::PathBuf;
use std::rc::Rc;
use std::time::Duration;

use crate::components::{ButtonVariant, TextField, button};
use crate::document::DocumentChange;
use crate::element::{Element, Handlers, div, text};
use crate::geo::{LatLon, MapView};
use crate::maps::editor::PolygonEdit;
use crate::responder::Action;
use crate::runtime::{App, Effects, Subscriptions};
use crate::testing::{By, TestApp};

/// The part: a counter with a name field and a slow save.
#[derive(Default)]
struct Counter {
    count: u32,
    name: String,
    saved: bool,
}

#[derive(Clone, Debug, PartialEq)]
enum CounterMsg {
    Add,
    Name(String),
    Save,
    Saved,
    Tick,
}

impl Counter {
    fn update(&mut self, msg: CounterMsg) -> Effects<CounterMsg> {
        match msg {
            CounterMsg::Add => self.count += 1,
            CounterMsg::Name(name) => self.name = name,
            CounterMsg::Save => return Effects::perform(|| 7, |_| CounterMsg::Saved),
            CounterMsg::Saved => self.saved = true,
            CounterMsg::Tick => {}
        }
        Effects::none()
    }

    fn view(&self) -> Element<CounterMsg> {
        div().children([
            button("Sumar", ButtonVariant::Primary)
                .on_click(CounterMsg::Add)
                .test_id("counter.add"),
            TextField::new("Nombre", &self.name)
                .on_input(CounterMsg::Name)
                .test_id("counter.name")
                .build(),
            button("Guardar", ButtonVariant::Text)
                .on_click(CounterMsg::Save)
                .test_id("counter.save"),
            text(self.count.to_string()),
        ])
    }
}

/// The parent: wraps the part's messages and keeps one of its own.
#[derive(Default)]
struct Shell {
    counter: Counter,
    other: u32,
}

#[derive(Clone, Debug, PartialEq)]
enum ShellMsg {
    Counter(CounterMsg),
    Other,
}

impl App for Shell {
    type Msg = ShellMsg;

    fn title(&self) -> String {
        "Contenedor".into()
    }

    fn update(&mut self, msg: ShellMsg) -> Effects<ShellMsg> {
        match msg {
            ShellMsg::Counter(msg) => self.counter.update(msg).map(ShellMsg::Counter),
            ShellMsg::Other => {
                self.other += 1;
                Effects::none()
            }
        }
    }

    fn subscriptions(&self) -> Subscriptions<ShellMsg> {
        Subscriptions::every("tick", Duration::from_secs(1), CounterMsg::Tick)
            .map(ShellMsg::Counter)
    }

    fn view(&self) -> Element<ShellMsg> {
        div().children([
            self.counter.view().map(ShellMsg::Counter),
            button("Otro", ButtonVariant::Text)
                .on_click(ShellMsg::Other)
                .test_id("other"),
        ])
    }
}

#[test]
fn a_mapped_part_receives_its_clicks_typing_and_task_results() {
    let mut app = TestApp::new(Shell::default());

    app.click(By::test_id("counter.add"));
    app.click(By::test_id("counter.add"));
    app.type_text(By::test_id("counter.name"), "Ana");
    app.click(By::test_id("other"));
    assert_eq!(app.app().counter.count, 2);
    assert_eq!(app.app().counter.name, "Ana");
    assert_eq!(app.app().other, 1);
    assert_eq!(app.messages()[0], ShellMsg::Counter(CounterMsg::Add));

    // The task's result reaches the part through the parent.
    app.click(By::test_id("counter.save"));
    app.run_until_idle();
    assert!(app.app().counter.saved);
    assert!(
        app.messages()
            .contains(&ShellMsg::Counter(CounterMsg::Saved))
    );

    // So does a subscription.
    app.advance(Duration::from_secs(2));
    assert!(
        app.messages()
            .contains(&ShellMsg::Counter(CounterMsg::Tick))
    );
}

#[derive(Clone, Debug, PartialEq)]
enum Inner {
    Click,
    Text(String),
    Index(usize),
    Files(usize),
    View(f64),
    Area(String),
    Action,
    Row(usize),
    Edit(usize),
    Document(String),
}

#[derive(Clone, Debug, PartialEq)]
struct Outer(Inner);

/// Every handler of the inner element, fired by hand.
fn fire(handlers: &Handlers<Outer>) -> Vec<Outer> {
    let mut out = Vec::new();
    out.extend(handlers.on_click.clone());
    out.extend(handlers.on_input.as_ref().map(|h| h("hola".into())));
    out.extend(handlers.on_submit.clone());
    out.extend(handlers.on_select.as_ref().map(|h| h(2)));
    out.extend(
        handlers
            .on_drop_files
            .as_ref()
            .map(|h| h(vec![PathBuf::from("a"), PathBuf::from("b")])),
    );
    let view = MapView {
        center: LatLon { lat: 1.0, lon: 2.0 },
        zoom: 3.0,
    };
    out.extend(handlers.on_view_change.as_ref().map(|h| h(view)));
    out.extend(handlers.on_area_click.as_ref().map(|h| h("A1".into())));
    let edit = PolygonEdit::Move {
        index: 5,
        to: LatLon { lat: 0.0, lon: 0.0 },
        first: true,
    };
    out.extend(handlers.on_edit.as_ref().map(|h| h(edit)));
    let change = DocumentChange {
        plain_text: "doc".into(),
        ..DocumentChange::default()
    };
    out.extend(handlers.on_document_change.as_ref().map(|h| h(change)));
    out.extend(
        handlers
            .on_action
            .as_ref()
            .and_then(|h| h(Action::SelectAll)),
    );
    out
}

#[test]
fn every_handler_and_lazy_row_is_converted() {
    let row: Rc<dyn Fn(usize) -> Element<Inner>> =
        Rc::new(|index| div().on_click(Inner::Row(index)));
    let mut inner: Element<Inner> = div()
        .on_click(Inner::Click)
        .on_submit(Inner::Click)
        .virtual_rows(10, 20.0, None, row)
        .child(text("hijo").on_click(Inner::Index(9)));
    inner.handlers.on_input = Some(Rc::new(Inner::Text));
    inner.handlers.on_select = Some(Rc::new(Inner::Index));
    inner.handlers.on_drop_files = Some(Rc::new(|files: Vec<PathBuf>| Inner::Files(files.len())));
    inner.handlers.on_view_change = Some(Rc::new(|view: MapView| Inner::View(view.zoom)));
    inner.handlers.on_area_click = Some(Rc::new(Inner::Area));
    inner.handlers.on_edit = Some(Rc::new(|edit| match edit {
        PolygonEdit::Move { index, .. } => Inner::Edit(index),
        _ => Inner::Edit(0),
    }));
    inner.handlers.on_document_change = Some(Rc::new(|change: DocumentChange| {
        Inner::Document(change.plain_text)
    }));
    inner.handlers.on_action = Some(Rc::new(|action| {
        (action == Action::SelectAll).then_some(Inner::Action)
    }));

    let outer = inner.map(Outer);
    let wrap = |inner| Outer(inner);
    assert_eq!(
        fire(&outer.handlers),
        [
            wrap(Inner::Click),
            wrap(Inner::Text("hola".into())),
            wrap(Inner::Click),
            wrap(Inner::Index(2)),
            wrap(Inner::Files(2)),
            wrap(Inner::View(3.0)),
            wrap(Inner::Area("A1".into())),
            wrap(Inner::Edit(5)),
            wrap(Inner::Document("doc".into())),
            wrap(Inner::Action),
        ]
    );
    // An action the inner element does not handle stays unhandled.
    let unhandled = outer.handlers.on_action.as_ref().unwrap()(Action::Copy);
    assert_eq!(unhandled, None);
    // Children and rows built later convert too.
    assert_eq!(
        outer.children[0].handlers.on_click,
        Some(wrap(Inner::Index(9)))
    );
    let rows = outer.virtual_rows.as_ref().unwrap();
    assert_eq!((rows.count, rows.row_height), (10, 20.0));
    assert_eq!(
        (rows.render)(4).handlers.on_click,
        Some(wrap(Inner::Row(4)))
    );
    assert_eq!(outer.row_count, Some(10));
}
