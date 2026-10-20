//! Controls and commands stay busy while the tasks they started run.

use std::sync::{Arc, Condvar, Mutex};
use std::time::Duration;

use serde_json::json;

use super::COMMAND_BUSY;
use crate::commands::{Action, Availability, CommandRegistry};
use crate::components::{Button, ButtonVariant, DotProgress, button};
use crate::element::{Element, Role, div, text};
use crate::layout::Content;
use crate::render::DrawOp;
use crate::runtime::{App, Effects};
use crate::style::Cursor;
use crate::testing::{By, TestApp};
use crate::theme::{Density, Theme};

/// Closed until [`Gate::open`]: tasks that wait on it stay running.
#[derive(Clone, Default)]
struct Gate(Arc<(Mutex<bool>, Condvar)>);

impl Gate {
    fn open(&self) {
        *self.0.0.lock().unwrap() = true;
        self.0.1.notify_all();
    }

    fn pass(&self) {
        let open = self.0.0.lock().unwrap();
        drop(self.0.1.wait_while(open, |open| !*open).unwrap());
    }
}

#[derive(Clone, Debug, PartialEq)]
enum Msg {
    Save,
    Saved,
    SaveAndReload,
    Reload,
    Reloaded,
    Sync,
    Synced,
    Later,
    Done,
    Export,
    Exported,
    Click,
}

#[derive(Default)]
struct Busy {
    gate: Gate,
    reload_gate: Gate,
    saved: u32,
    reloaded: u32,
    clicks: u32,
}

#[derive(Clone, serde::Deserialize, schemars::JsonSchema)]
struct Export;

impl Action for Export {
    type App = Busy;
    const ID: &'static str = "prueba.exportar";
    const TITLE: &'static str = "Exportar";
    fn into_msg(self) -> Msg {
        Msg::Export
    }
}

impl Busy {
    fn wait(&self, done: Msg) -> Effects<Msg> {
        let gate = self.gate.clone();
        Effects::perform(move || gate.pass(), move |()| done)
    }
}

impl App for Busy {
    type Msg = Msg;

    fn commands(registry: &mut CommandRegistry<Self>) {
        registry.action::<Export>().expose();
    }

    fn title(&self) -> String {
        "Ocupado".into()
    }

    fn update(&mut self, msg: Msg) -> Effects<Msg> {
        match msg {
            Msg::Save | Msg::Sync => {
                let done = if msg == Msg::Save {
                    Msg::Saved
                } else {
                    Msg::Synced
                };
                return self.wait(done);
            }
            Msg::SaveAndReload => return Effects::perform(|| (), |()| Msg::Reload),
            Msg::Reload => {
                let gate = self.reload_gate.clone();
                return Effects::perform(move || gate.pass(), |()| Msg::Reloaded);
            }
            Msg::Export => return self.wait(Msg::Exported),
            Msg::Later => return Effects::after(Duration::from_secs(1), Msg::Done),
            Msg::Saved => self.saved += 1,
            Msg::Reloaded => self.reloaded += 1,
            Msg::Click => self.clicks += 1,
            Msg::Synced | Msg::Done | Msg::Exported => {}
        }
        Effects::none()
    }

    fn view(&self) -> Element<Msg> {
        div().children([
            button("Guardar", ButtonVariant::Primary)
                .on_click(Msg::Save)
                .test_id("guardar"),
            button("Guardar y recargar", ButtonVariant::Secondary)
                .on_click(Msg::SaveAndReload)
                .test_id("recargar"),
            Button::new("Sincronizar")
                .busy_feedback(false)
                .on_click(Msg::Sync)
                .build()
                .test_id("sincronizar"),
            button("Recordar", ButtonVariant::Secondary)
                .on_click(Msg::Later)
                .test_id("recordar"),
            button("Exportar", ButtonVariant::Secondary)
                .on_command(Export)
                .test_id("exportar"),
            button("Clic", ButtonVariant::Secondary).on_click(Msg::Click),
            text(format!("{} guardados", self.saved)).role(Role::Status),
        ])
    }
}

fn by(test_id: &str) -> By {
    By::test_id(test_id)
}

#[test]
fn a_clicked_button_is_busy_until_its_task_delivers() {
    let mut app = TestApp::new(Busy::default());
    let before = app.screen().get(by("guardar")).node().bounds;
    app.hover(by("guardar"));
    app.click(by("guardar"));
    assert!(app.is_busy(by("guardar")));
    assert!(
        !app.is_busy(By::role(Role::Button).name("Clic")),
        "only the button that started it"
    );
    assert_eq!(app.mouse_cursor(), Cursor::Wait);
    assert_eq!(
        app.screen().get(by("guardar")).node().bounds,
        before,
        "same width"
    );

    app.click(by("guardar"));
    let saves = app.messages().iter().filter(|msg| **msg == Msg::Save);
    assert_eq!(saves.count(), 1, "a busy button takes no clicks");
    app.click(By::role(Role::Button).name("Clic"));
    assert_eq!(app.app().clicks, 1, "the rest of the window goes on");

    app.app().gate.open();
    app.run_until_idle();
    assert!(!app.is_busy(by("guardar")));
    assert_eq!(app.app().saved, 1);
    assert_eq!(app.mouse_cursor(), Cursor::Pointer);
    app.click(by("guardar"));
    app.run_until_idle();
    assert_eq!(app.app().saved, 2);
}

#[test]
fn chained_tasks_keep_the_button_busy_until_the_last_one() {
    let mut app = TestApp::new(Busy::default());
    app.click(by("recargar"));
    app.run_next_tasks();
    assert!(app.messages().contains(&Msg::Reload));
    assert!(
        app.is_busy(by("recargar")),
        "the reload the save started is still the button's"
    );

    app.app().reload_gate.open();
    app.run_until_idle();
    assert_eq!(app.app().reloaded, 1);
    assert!(!app.is_busy(by("recargar")));
}

#[test]
fn opting_out_keeps_the_button_usable_and_timers_never_make_it_busy() {
    let mut app = TestApp::new(Busy::default());
    app.click(by("sincronizar"));
    assert!(app.tasks_pending());
    assert!(!app.is_busy(by("sincronizar")));
    app.click(by("sincronizar"));
    let syncs = app.messages().iter().filter(|msg| **msg == Msg::Sync);
    assert_eq!(syncs.count(), 2);

    app.click(by("recordar"));
    assert!(!app.is_busy(by("recordar")), "Effects::after is no task");
    app.app().gate.open();
    app.run_until_idle();
}

#[test]
fn a_busy_command_blocks_its_buttons_the_menu_the_palette_and_agents() {
    let mut app = TestApp::new(Busy::default());
    assert!(app.perform(Export::ID));
    assert!(app.is_busy(by("exportar")), "bound buttons show it");
    assert_eq!(
        app.available::<Export>(),
        Availability::disabled(COMMAND_BUSY)
    );
    assert!(!app.perform(Export::ID), "shortcut, menu and palette");
    assert_eq!(app.invoke(Export).unwrap_err().message, COMMAND_BUSY);
    assert_eq!(
        app.call(Export::ID, json!({})).unwrap_err().message,
        COMMAND_BUSY
    );
    app.click(by("exportar"));
    let exports = app.messages().iter().filter(|msg| **msg == Msg::Export);
    assert_eq!(exports.count(), 1);

    app.app().gate.open();
    app.run_until_idle();
    assert!(!app.is_busy(by("exportar")));
    assert_eq!(app.available::<Export>(), Availability::Enabled);
    app.click(by("exportar"));
    assert!(
        app.is_busy(by("exportar")),
        "a click goes through the command"
    );
    app.run_until_idle();
}

#[test]
fn the_label_stays_for_assistive_tech_under_the_dots() {
    let mut app = TestApp::new(Busy::default());
    app.click(by("guardar"));
    assert!(app.is_busy(By::role(Role::Button).name("Guardar")));
    app.app().gate.open();
    app.run_until_idle();
}

/// Tops of the 10 pt dots of a [`DotProgress`] in the paint list.
fn dot_tops(app: &TestApp<Dots>) -> Vec<f32> {
    app.draw_list()
        .ops
        .iter()
        .filter_map(|op| match op {
            DrawOp::Quad(quad) if quad.rect.width == 10.0 && quad.rect.height == 10.0 => {
                Some(quad.rect.y)
            }
            _ => None,
        })
        .collect()
}

struct Dots;

impl App for Dots {
    type Msg = Msg;

    fn title(&self) -> String {
        "Puntos".into()
    }

    fn update(&mut self, _: Msg) -> Effects<Msg> {
        Effects::none()
    }

    fn view(&self) -> Element<Msg> {
        div()
            .class("p-4")
            .child(DotProgress::new().test_id("carga").build())
    }
}

#[test]
fn dots_jump_in_turn_while_painting_and_layout_stays() {
    let mut app = TestApp::new(Dots);
    let still = dot_tops(&app);
    assert_eq!(still.len(), 3);
    assert!(app.session().animating(), "dots in view need frames");
    let dot = app.screen().get(by("carga.dot.0")).node().bounds;
    assert!(matches!(
        app.screen().get(by("carga.dot.0")).node().content,
        Content::Box
    ));

    // The first dot peaks at a quarter of the cycle; the second waits.
    app.advance(Duration::from_millis(262));
    let moved = dot_tops(&app);
    assert!(
        (moved[0] - (still[0] - 7.0)).abs() < 0.1,
        "{moved:?} {still:?}"
    );
    assert!(moved[1] < still[1] && moved[1] > moved[0], "{moved:?}");
    assert_eq!(moved[2], still[2], "the third has not started");
    assert_eq!(
        app.screen().get(by("carga.dot.0")).node().bounds,
        dot,
        "only the paint moves"
    );
}

#[test]
fn reduced_motion_keeps_the_dots_still_and_asks_for_no_frames() {
    let mut theme = Theme::kai(Density::Comfortable);
    theme.prefs.reduce_motion = true;
    let mut app = TestApp::builder(Dots).theme(theme).build();
    let still = dot_tops(&app);
    assert!(!app.session().animating());
    app.advance(Duration::from_millis(262));
    assert_eq!(dot_tops(&app), still);
}
