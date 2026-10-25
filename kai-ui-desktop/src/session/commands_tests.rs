//! Commands from every way in: availability, shortcuts, the menu bar, the
//! palette, scripts, agents and assistive tech send the same `Msg`
//! ([`crate::commands`]). They run on [`Sample`], not the demo (docs/dev/27).

use serde_json::json;

use crate::commands::{Availability, PALETTE};
use crate::element::{KeyEquivalent, Role};
use crate::menu::{MenuEntry, menu_model};
use crate::testing::sample_app::{
    ClearForm, Increment, Msg, Reset, Sample, Submit, TogglePasswordAction,
};
use crate::testing::{By, Key, Modifiers, MsgSource, TestApp};

const CMD: Modifiers = Modifiers {
    command: true,
    ..Modifiers::NONE
};

const CMD_SHIFT: Modifiers = Modifiers {
    command: true,
    shift: true,
    ..Modifiers::NONE
};

fn row(id: &str) -> By {
    By::test_id(format!("kai.palette.{id}"))
}

fn search() -> By {
    By::test_id("kai.palette.find")
}

#[test]
fn availability_follows_update() {
    let mut app = TestApp::new(Sample::default());
    let ids: Vec<&str> = app.commands().iter().map(|c| c.id).collect();
    assert_eq!(
        ids,
        [
            "counter.increment",
            "counter.add",
            "counter.reset",
            "form.submit",
            "form.clear",
            "form.toggle_password",
            "form.last_submission",
            "task.wait",
            "kai.palette",
        ]
    );
    assert_eq!(
        app.available::<Reset>(),
        Availability::disabled("El contador ya está en cero")
    );
    app.click(By::test_id("bar.add"));
    assert!(app.available::<Reset>().is_enabled());
    assert!(!app.available::<Submit>().is_enabled());
    app.type_text(By::test_id("form.name"), "Ana");
    assert!(app.available::<Submit>().is_enabled());
}

#[test]
fn invoke_runs_enabled_actions_and_reports_disabled_ones() {
    let mut app = TestApp::new(Sample::default());
    let error = app.invoke(Reset).unwrap_err();
    assert_eq!(error.message, "El contador ya está en cero");
    assert_eq!(app.messages(), [], "a disabled action sends nothing");

    app.invoke(Increment).unwrap();
    app.invoke(Increment).unwrap();
    assert_eq!(app.app().clicks, 2);
    let screen = app.screen();
    screen
        .within(screen.get_by_test_id("counter"))
        .get(By::role(Role::Status).name("2"));
    app.invoke(Reset).unwrap();
    assert_eq!(app.messages(), [Msg::Increment, Msg::Increment, Msg::Reset]);
    assert_eq!(app.app().clicks, 0);
}

#[test]
fn menu_bar_follows_the_registry() {
    let app = TestApp::new(Sample::default());
    let model = menu_model(app.commands());
    let titles: Vec<_> = model.iter().map(|group| group.title).collect();
    assert_eq!(titles, ["Contador", "Formulario", "Ver"]);
    assert_eq!(
        model[0].items[1],
        MenuEntry {
            id: "counter.reset",
            title: "Reiniciar",
            enabled: false,
            shortcut: Some(KeyEquivalent::cmd(Key::Char('0'))),
        }
    );
    assert_eq!(
        model[1].items[1].shortcut,
        Some(KeyEquivalent::ctrl_cmd(Key::Char('l')))
    );
    let view_menu: Vec<_> = model[2].items.iter().map(|item| item.id).collect();
    assert_eq!(view_menu, [PALETTE]);
    assert!(model[2].items[0].enabled);
}

#[test]
fn command_shortcut_sends_the_same_msg_as_the_button() {
    let mut app = TestApp::new(Sample::default());
    app.press(Key::Char('i'), CMD);
    assert_eq!(app.messages(), [Msg::Increment]);
    app.click(By::role(Role::Button).name("Incrementar"));
    assert_eq!(app.messages(), [Msg::Increment, Msg::Increment]);
    assert_eq!(app.app().clicks, 2);
}

#[test]
fn disabled_command_shortcut_sends_nothing() {
    let mut app = TestApp::new(Sample::default());
    app.press(Key::Char('0'), CMD);
    assert_eq!(app.messages(), [], "nothing to reset");
    app.press(Key::Char('i'), CMD);
    app.press(Key::Char('0'), CMD);
    assert_eq!(app.messages(), [Msg::Increment, Msg::Reset]);
}

#[test]
fn command_shortcuts_do_not_steal_copy_in_a_field() {
    let mut app = TestApp::new(Sample::default());
    app.type_text(By::test_id("form.name"), "Ana");
    let typed = app.messages().len();
    app.press(Key::A, CMD);
    app.press(Key::C, CMD);
    assert_eq!(app.clipboard().text().as_deref(), Some("Ana"));
    assert_eq!(app.messages().len(), typed, "copying sends nothing");
    // Cmd shortcuts still work while typing.
    app.press(Key::Char('i'), CMD);
    assert_eq!(app.messages().last(), Some(&Msg::Increment));
}

#[test]
fn a_button_follows_its_command() {
    let mut app = TestApp::new(Sample::default());
    let reset = || By::test_id("counter.reset");
    assert!(!app.is_enabled(reset()));
    assert_eq!(
        app.screen().get(reset()).node().description.as_deref(),
        Some("El contador ya está en cero")
    );
    assert_eq!(
        app.screen().get(reset()).name().as_deref(),
        Some("Reiniciar")
    );
    app.click(reset());
    assert_eq!(app.messages(), []);
    app.click(By::test_id("bar.add"));
    assert!(app.is_enabled(reset()));
    assert_eq!(app.screen().get(reset()).node().description, None);
    app.click(reset());
    assert_eq!(app.messages().last(), Some(&Msg::Reset));
    assert_eq!(app.app().clicks, 0);
}

#[test]
fn disabled_command_stays_disabled_in_button_palette_and_menu() {
    let mut app = TestApp::new(Sample::default());
    app.press(Key::Char('p'), CMD_SHIFT);
    let reset = row("counter.reset");
    assert!(!app.is_enabled(By::test_id("counter.reset")));
    assert!(!app.is_enabled(reset.clone()));
    assert_eq!(
        app.screen().get(reset).node().description.as_deref(),
        Some("El contador ya está en cero")
    );
    let menu = menu_model(app.commands());
    let entry = menu[0].items.iter().find(|item| item.id == "counter.reset");
    assert_eq!(entry.map(|item| item.enabled), Some(false));
}

#[test]
fn command_z_undoes_typing_in_a_field_and_the_app_hears_it() {
    let mut app = TestApp::new(Sample::default());
    let name = By::test_id("form.name");
    app.type_text(name.clone(), "Ana María");
    app.press(Key::Char('z'), CMD);
    assert_eq!(app.app().name, "Ana ", "word by word");
    assert_eq!(app.value(name.clone()), "Ana ");
    app.press(Key::Char('z'), CMD);
    assert_eq!(app.app().name, "");
    app.press(Key::Char('z'), CMD_SHIFT);
    app.press(Key::Char('z'), CMD_SHIFT);
    assert_eq!(app.app().name, "Ana María");
}

/// A way to run a command: as a script, with the mouse, the keyboard,
/// the menu bar or the palette.
#[derive(Clone, Copy, Debug)]
enum Via {
    Invoke,
    /// `call(id, {})`, as an agent will.
    Json,
    Button(&'static str),
    Key(Key, Modifiers),
    Menu,
    /// Open the palette, type this and press Return.
    Palette(&'static str),
}

struct Case {
    id: &'static str,
    setup: fn(&mut TestApp<Sample>),
    invoke: fn(&mut TestApp<Sample>),
    vias: Vec<Via>,
}

/// Run `via` on a fresh app after `setup`; the model and the messages
/// sent by `via` alone.
fn run_via(case: &Case, via: Via) -> (Sample, Vec<Msg>) {
    let mut app = TestApp::new(Sample::default());
    (case.setup)(&mut app);
    let before = app.messages().len();
    match via {
        Via::Invoke => (case.invoke)(&mut app),
        Via::Json => {
            let _ = app.call(case.id, json!({}));
        }
        Via::Button(test_id) => app.click(By::test_id(test_id)),
        Via::Key(key, modifiers) => app.press(key, modifiers),
        Via::Menu => {
            let menu = menu_model(app.commands());
            let entry = menu
                .iter()
                .flat_map(|group| &group.items)
                .find(|e| e.id == case.id);
            assert!(entry.is_some(), "{} no está en el menú", case.id);
            app.perform(case.id);
        }
        Via::Palette(query) => {
            app.press(Key::Char('p'), CMD_SHIFT);
            app.type_text(search(), query);
            app.press(Key::Enter, Modifiers::NONE);
        }
    }
    (app.app().clone(), app.messages()[before..].to_vec())
}

fn type_name(app: &mut TestApp<Sample>) {
    app.type_text(By::test_id("form.name"), "Ana");
}

fn cases() -> Vec<Case> {
    vec![
        Case {
            id: "counter.increment",
            setup: |_| {},
            invoke: |app| app.invoke(Increment).unwrap(),
            vias: vec![
                Via::Invoke,
                Via::Json,
                Via::Button("bar.add"),
                Via::Button("counter.increment"),
                Via::Key(Key::Char('i'), CMD),
                Via::Menu,
                Via::Palette("Incrementar"),
            ],
        },
        Case {
            id: "counter.reset",
            setup: |app| app.invoke(Increment).unwrap(),
            invoke: |app| app.invoke(Reset).unwrap(),
            vias: vec![
                Via::Invoke,
                Via::Json,
                Via::Button("counter.reset"),
                Via::Key(Key::Char('0'), CMD),
                Via::Menu,
                Via::Palette("Reiniciar"),
            ],
        },
        Case {
            id: "form.submit",
            setup: type_name,
            invoke: |app| app.invoke(Submit).unwrap(),
            vias: vec![
                Via::Invoke,
                Via::Json,
                Via::Button("form.submit"),
                // Return in «Nombre»: «Enviar» is the default button.
                Via::Key(Key::Enter, Modifiers::NONE),
                Via::Menu,
                Via::Palette("Enviar"),
            ],
        },
        Case {
            id: "form.clear",
            setup: type_name,
            invoke: |app| app.invoke(ClearForm).unwrap(),
            vias: vec![
                Via::Invoke,
                Via::Json,
                Via::Button("form.clear"),
                Via::Key(Key::Char('l'), CMD_CTRL),
                Via::Menu,
                Via::Palette("Limpiar"),
            ],
        },
        Case {
            id: "form.toggle_password",
            setup: |_| {},
            invoke: |app| app.invoke(TogglePasswordAction).unwrap(),
            vias: vec![
                Via::Invoke,
                Via::Json,
                Via::Button("form.password.toggle"),
                Via::Menu,
                Via::Palette("contraseña"),
            ],
        },
    ]
}

const CMD_CTRL: Modifiers = Modifiers {
    command: true,
    control: true,
    ..Modifiers::NONE
};

#[test]
fn every_action_does_the_same_from_every_via() {
    for case in cases() {
        let mut setup = TestApp::new(Sample::default());
        (case.setup)(&mut setup);
        let before = setup.app().clone();
        let (expected, messages) = run_via(&case, Via::Invoke);
        assert_eq!(messages.len(), 1, "{}: un mensaje", case.id);
        assert_ne!(expected, before, "{} cambia el modelo", case.id);
        for &via in &case.vias[1..] {
            let (model, sent) = run_via(&case, via);
            assert_eq!(sent, messages, "{} vía {via:?}", case.id);
            assert_eq!(model, expected, "{} vía {via:?}", case.id);
        }
    }
}

#[test]
fn disabled_actions_do_nothing_from_any_via() {
    let disabled = [
        Case {
            id: "counter.reset",
            setup: |_| {},
            invoke: |app| {
                let error = app.invoke(Reset).unwrap_err();
                assert_eq!(error.message, "El contador ya está en cero");
            },
            vias: vec![
                Via::Invoke,
                Via::Json,
                Via::Button("counter.reset"),
                Via::Key(Key::Char('0'), CMD),
                Via::Menu,
                Via::Palette("Reiniciar"),
            ],
        },
        Case {
            id: "form.submit",
            setup: |app| app.click(By::test_id("form.name")),
            invoke: |app| {
                let error = app.invoke(Submit).unwrap_err();
                assert_eq!(error.message, "Falta el nombre");
            },
            vias: vec![
                Via::Invoke,
                Via::Json,
                Via::Button("form.submit"),
                Via::Key(Key::Enter, Modifiers::NONE),
                Via::Menu,
                Via::Palette("Enviar"),
            ],
        },
    ];
    for case in disabled {
        let mut setup = TestApp::new(Sample::default());
        (case.setup)(&mut setup);
        let before = setup.app().clone();
        for &via in &case.vias {
            let (model, sent) = run_via(&case, via);
            assert_eq!(sent, [], "{} vía {via:?}", case.id);
            assert_eq!(model, before, "{} vía {via:?}", case.id);
        }
    }
}

#[test]
fn messages_record_where_they_came_from() {
    let mut app = TestApp::new(Sample::default());
    app.click(By::role(Role::Button).name("Incrementar"));
    app.press(Key::Char('i'), CMD);
    assert!(app.perform("counter.increment"));
    app.invoke(Increment).unwrap();
    app.call("counter.add", json!({"count": 2})).unwrap();
    let button = app.screen().get(By::test_id("counter.increment")).id();
    app.accessibility_action(&accesskit::ActionRequest {
        action: accesskit::Action::Click,
        target_tree: accesskit::TreeId::ROOT,
        target_node: crate::a11y::node_id(button),
        data: None,
    });
    app.agent_call("kai-test", "counter.reset", json!({}));
    app.click(By::test_id("kai.confirm.once"));

    assert_eq!(
        app.messages(),
        [
            Msg::Increment,
            Msg::Increment,
            Msg::Increment,
            Msg::Increment,
            Msg::Add(2),
            Msg::Increment,
            Msg::Reset,
        ]
    );
    assert_eq!(
        app.message_sources(),
        [
            MsgSource::Pointer,
            MsgSource::Keyboard,
            MsgSource::Menu,
            MsgSource::Script,
            MsgSource::Script,
            MsgSource::A11y,
            MsgSource::Agent("kai-test".into()),
        ],
        "an agent's call a click allowed still comes from the agent"
    );
}

#[test]
fn actions_with_input_stay_out_of_menus_and_palette() {
    let mut app = TestApp::new(Sample::default());
    let add_amount = app.commands().iter().find(|c| c.id == "counter.add");
    assert!(add_amount.is_some_and(|c| c.needs_input));
    let menu = menu_model(app.commands());
    assert!(
        menu.iter()
            .flat_map(|g| &g.items)
            .all(|e| e.id != "counter.add")
    );
    app.press(Key::Char('p'), CMD_SHIFT);
    app.type_text(search(), "Sumar");
    assert!(app.screen().query(row("counter.add")).is_none());
    assert!(app.screen().query(By::text("Sin resultados")).is_some());
}
