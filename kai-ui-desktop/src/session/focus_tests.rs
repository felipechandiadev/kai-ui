//! Focus and the keyboard: Tab order, the focus ring, Return and the
//! default button, segmented controls as one stop and dialogs that keep
//! the focus. On [`Sample`] and two small apps (docs/dev/27).

use crate::components::{ButtonVariant, Dialog, button, segmented};
use crate::element::{Element, FocusOrder, Role, WidgetId, div};
use crate::geometry::Size;
use crate::layout::testing::FixedMeasure;
use crate::layout::{Interaction, Layout, LayoutInput, ScrollOffsets, StyleCache};
use crate::runtime::{App, Effects};
use crate::testing::sample_app::{Msg, Sample};
use crate::testing::{By, Key, Modifiers, Screen, TestApp, layout_of};
use crate::theme::{Density, Theme};

const SHIFT: Modifiers = Modifiers {
    shift: true,
    ..Modifiers::NONE
};

/// The widget `by` finds in `app`'s view.
fn find(app: &Sample, by: By) -> WidgetId {
    let layout = layout_of(&crate::runtime::build_view(app));
    Screen::new(&layout).get(by).id()
}

/// Every Tab stop, in order, by `test_id` or else by name, until Tab
/// wraps around.
fn tab_stops<A: App>(app: &mut TestApp<A>) -> Vec<String> {
    let mut stops: Vec<String> = Vec::new();
    for _ in 0..60 {
        app.press(Key::Tab, Modifiers::NONE);
        let focused = app.session().focused().expect("Tab focuses something");
        let screen = app.screen();
        let found = screen.by_id(focused).unwrap();
        let stop = found
            .test_id()
            .map(str::to_owned)
            .or_else(|| found.name())
            .unwrap_or_default();
        if stops.first() == Some(&stop) {
            return stops;
        }
        stops.push(stop);
    }
    panic!("Tab does not wrap around: {stops:?}");
}

#[test]
fn focus_order_follows_document_order_and_skips_disabled() {
    // The toolbar (one stop), «Incrementar» («Reiniciar» starts
    // disabled), the form with the eye after its field («Enviar» starts
    // disabled) and the palette link.
    let stops = [
        "bar.add",
        "counter.increment",
        "form.name.input",
        "form.email.input",
        "form.password.input",
        "form.password.toggle",
        "form.clear",
        "palette.open",
    ];
    assert_eq!(tab_stops(&mut TestApp::new(Sample::default())), stops);

    let mut app = TestApp::new(Sample::default());
    app.invoke(crate::testing::sample_app::Increment).unwrap();
    app.type_text(By::test_id("form.name"), "Ana");
    let enabled = tab_stops(&mut app);
    assert_eq!(enabled.len(), stops.len() + 2, "enabled buttons join");
    assert!(enabled.contains(&"counter.reset".to_owned()));
    assert!(enabled.contains(&"form.submit".to_owned()));
}

#[test]
fn return_in_name_field_activates_the_default_button() {
    use crate::ui_engine::{Intent, TextIntent, UiModel, dispatch};

    let return_key = Intent::Text(TextIntent::KeyDown {
        key: Key::Enter,
        modifiers: Modifiers::NONE,
        repeat: false,
    });
    let mut sample = Sample::default();
    let name = find(&sample, By::test_id("form.name.input"));
    let order = FocusOrder::from_tree(&crate::runtime::build_view(&sample));
    let mut model = UiModel::default();
    dispatch(&mut model, &order, Intent::SetFocus(Some(name)));
    let empty = dispatch(&mut model, &order, return_key.clone());
    assert_eq!(empty.activated, None, "«Enviar» is disabled without a name");

    let _ = sample.update(Msg::NameChanged("Ana".to_owned()));
    let send = find(&sample, By::test_id("form.submit"));
    let order = FocusOrder::from_tree(&crate::runtime::build_view(&sample));
    let result = dispatch(&mut model, &order, return_key.clone());
    assert_eq!(result.activated.map(|a| a.widget), Some(send));

    // «Correo» keeps its on_submit.
    let email = find(&sample, By::test_id("form.email.input"));
    dispatch(&mut model, &order, Intent::SetFocus(Some(email)));
    let result = dispatch(&mut model, &order, return_key);
    assert_eq!(result.activated, None);
    assert_eq!(result.text_actions.len(), 1);
}

#[test]
fn return_in_the_name_field_submits_through_the_default_button() {
    let mut app = TestApp::new(Sample::default());
    app.type_text(By::test_id("form.name"), "Ana");
    app.press(Key::Enter, Modifiers::NONE);
    assert_eq!(app.messages().last(), Some(&Msg::Submit));
    assert_eq!(app.app().submissions, ["Ana"]);
}

#[test]
fn clicked_buttons_show_no_focus_ring_but_fields_keep_their_focus_look() {
    use crate::ui_engine::{Intent, TextIntent, UiModel, dispatch};

    let tab = |shift| {
        Intent::Text(TextIntent::KeyDown {
            key: Key::Tab,
            modifiers: if shift { SHIFT } else { Modifiers::NONE },
            repeat: false,
        })
    };
    let sample = Sample::default();
    let tree = crate::runtime::build_view(&sample);
    let order = FocusOrder::from_tree(&tree);
    let theme = Theme::kai(Density::Comfortable);
    let mut styles = StyleCache::default();
    let mut compute = |model: &UiModel| {
        Layout::compute(
            &tree,
            &LayoutInput {
                viewport: Size {
                    width: 1024.0,
                    height: 768.0,
                },
                theme: &theme,
                interaction: &Interaction {
                    focused: *model.focused.get(),
                    focus_visible: *model.focus_visible.get(),
                    ..Interaction::default()
                },
                scroll: &ScrollOffsets::new(),
            },
            &mut styles,
            &mut FixedMeasure,
        )
    };
    let button = find(&sample, By::role(Role::Button).name("Incrementar"));
    let ring = |layout: &Layout| layout.node(button).unwrap().style.ring.width;
    let mut model = UiModel::default();

    dispatch(&mut model, &order, Intent::PointerDown(Some(button)));
    dispatch(&mut model, &order, Intent::PointerUp(Some(button)));
    assert_eq!(*model.focused.get(), Some(button));
    assert_eq!(ring(&compute(&model)), 0.0, "mouse click: no ring");

    dispatch(&mut model, &order, tab(false));
    dispatch(&mut model, &order, tab(true));
    assert_eq!(*model.focused.get(), Some(button));
    assert_eq!(ring(&compute(&model)), 2.0, "Tab: ring");

    // A clicked text field looks exactly as one reached with Tab.
    let name = find(&sample, By::label("Nombre"));
    let path = tree.path_to(name).unwrap();
    let field_look = |layout: &Layout| {
        path.iter()
            .map(|id| format!("{:?}", layout.node(*id).unwrap().style))
            .collect::<Vec<_>>()
    };
    let unfocused = field_look(&compute(&UiModel::default()));
    dispatch(&mut model, &order, Intent::PointerDown(Some(name)));
    assert!(!*model.focus_visible.get());
    let clicked = field_look(&compute(&model));
    dispatch(&mut model, &order, Intent::SetFocus(Some(name)));
    assert!(*model.focus_visible.get());
    assert_eq!(clicked, field_look(&compute(&model)));
    assert_ne!(clicked, unfocused, "the focused field is highlighted");
}

#[test]
fn tab_moves_through_the_form_in_order_showing_the_ring() {
    let mut app = TestApp::new(Sample::default());
    app.click(By::test_id("form.name"));
    assert!(app.is_focused(By::test_id("form.name")));
    assert!(!app.focus_visible(), "a click does not show the ring");

    app.press(Key::Tab, Modifiers::NONE);
    assert!(app.is_focused(By::test_id("form.email")));
    assert!(app.focus_visible());

    app.press(Key::Tab, SHIFT);
    assert!(app.is_focused(By::test_id("form.name")));
}

#[test]
fn the_password_eye_toggles_and_leaves_focus_in_the_field() {
    let mut app = TestApp::new(Sample::default());
    app.type_text(By::test_id("form.password"), "abc");
    assert!(!app.app().show_password);
    app.click(By::test_id("form.password.toggle"));
    assert!(app.app().show_password);
    assert!(app.is_focused(By::test_id("form.password")));
    assert_eq!(app.value(By::test_id("form.password")), "abc");
}

/// A segmented control between two buttons.
#[derive(Default)]
struct Effect {
    selected: usize,
}

impl App for Effect {
    type Msg = usize;

    fn title(&self) -> String {
        "Efecto".into()
    }

    fn update(&mut self, index: usize) -> Effects<usize> {
        self.selected = index;
        Effects::none()
    }

    fn view(&self) -> Element<usize> {
        div().children([
            button("Antes", ButtonVariant::Secondary).on_click(0),
            segmented(
                &["Ninguno", "Malla", "Brillo", "Ondas"],
                self.selected,
                |index| index,
            )
            .label("Efecto")
            .test_id("effect"),
            button("Después", ButtonVariant::Secondary).on_click(0),
        ])
    }
}

/// The effect is a segmented control: one Tab stop, arrows move between
/// effects and Space applies the focused one.
#[test]
fn segmented_controls_are_one_tab_stop() {
    let mut app = TestApp::new(Effect::default());
    let segment = |name: &str| By::role(Role::Radio).name(name);
    app.press(Key::Tab, Modifiers::NONE);
    app.press(Key::Tab, Modifiers::NONE);
    assert!(app.is_focused(segment("Ninguno")));
    app.press(Key::ArrowRight, Modifiers::NONE);
    app.press(Key::Space, Modifiers::NONE);
    assert_eq!(app.app().selected, 1);
    app.press(Key::Tab, Modifiers::NONE);
    assert!(app.is_focused(By::role(Role::Button).name("Después")));
    assert_eq!(
        tab_stops(&mut TestApp::new(Effect::default())).len(),
        3,
        "the four segments are one stop"
    );
}

#[derive(Clone, Debug, PartialEq)]
enum Ask {
    Open(&'static str),
    Close,
    Confirm,
}

/// Two buttons that open a dialog each.
#[derive(Default)]
struct Asking {
    open: Option<&'static str>,
    confirmed: Vec<&'static str>,
}

impl App for Asking {
    type Msg = Ask;

    fn title(&self) -> String {
        "Diálogos".into()
    }

    fn update(&mut self, msg: Ask) -> Effects<Ask> {
        match msg {
            Ask::Open(which) => self.open = Some(which),
            Ask::Close => self.open = None,
            Ask::Confirm => self.confirmed.extend(self.open.take()),
        }
        Effects::none()
    }

    fn view(&self) -> Element<Ask> {
        let mut layers = vec![
            button("Guardar cambios…", ButtonVariant::Outlined)
                .on_click(Ask::Open("save"))
                .test_id("save"),
            button("Eliminar borrador…", ButtonVariant::Danger)
                .on_click(Ask::Open("delete"))
                .test_id("delete"),
        ];
        if let Some(which) = self.open {
            let dialog = if which == "save" {
                Dialog::new("¿Guardar los cambios?").default_button("Guardar", Ask::Confirm)
            } else {
                Dialog::new("¿Eliminar el borrador?")
                    .destructive(true)
                    .default_button("Eliminar", Ask::Confirm)
            };
            layers.push(
                dialog
                    .message("Mensaje")
                    .cancel_button("Cancelar", Ask::Close)
                    .test_id("dialog")
                    .build(),
            );
        }
        div().class("relative w-full h-full").children(layers)
    }
}

#[test]
fn dialogs_trap_focus_and_give_it_back() {
    let mut app = TestApp::new(Asking::default());
    let dialog = || By::role(Role::Dialog);
    app.click(By::test_id("save"));
    assert_eq!(
        app.screen().get(dialog()).name().as_deref(),
        Some("¿Guardar los cambios?")
    );
    assert!(app.is_focused(By::test_id("dialog.cancel")));
    for _ in 0..3 {
        app.press(Key::Tab, Modifiers::NONE);
        let focused = app.session().focused().unwrap();
        let screen = app.screen();
        let inside = screen
            .within(screen.get(dialog()))
            .all(By::role(Role::Button));
        assert!(inside.iter().any(|button| button.id() == focused));
    }
    app.press(Key::Escape, Modifiers::NONE);
    assert!(app.screen().query(dialog()).is_none());
    assert!(app.is_focused(By::test_id("save")));
    assert!(app.app().confirmed.is_empty(), "cancel confirms nothing");

    app.click(By::test_id("delete"));
    let default = By::test_id("dialog.default");
    assert_eq!(
        app.screen().get(default.clone()).name().as_deref(),
        Some("Eliminar")
    );
    app.click(default);
    assert!(app.screen().query(dialog()).is_none());
    assert_eq!(app.app().confirmed, ["delete"]);
}
