//! The floating layer in a running session (docs/dev/32): where the focus
//! goes, what dismisses it, what the pointer reaches and where it paints.

use crate::components::{ButtonVariant, button};
use crate::element::{Element, Role, div, input, text};
use crate::geometry::Rect;
use crate::overlay::{Floating, OverlayFocus, Side};
use crate::render::DrawOp;
use crate::runtime::{App, Effects};
use crate::testing::{By, Key, Modifiers, TestApp};

#[derive(Clone, Debug, PartialEq)]
enum Msg {
    Toggle,
    Close,
    Pick(&'static str),
    Outside,
    Query(String),
}

/// A field with a list under it (focus in the anchor) or a menu button
/// with its menu (focus inside), in a clipping box, and a button outside.
struct Picker {
    focus: OverlayFocus,
    /// The anchor is a field (else a button).
    field: bool,
    open: bool,
    /// The anchor sits at the bottom of the window.
    at_bottom: bool,
    query: String,
    picked: Vec<&'static str>,
    outside: usize,
}

impl Picker {
    fn new(focus: OverlayFocus) -> Self {
        Self {
            focus,
            field: focus == OverlayFocus::Anchor,
            open: false,
            at_bottom: false,
            query: String::new(),
            picked: Vec::new(),
            outside: 0,
        }
    }

    fn panel(&self) -> Element<Msg> {
        let role = match self.focus {
            OverlayFocus::Anchor => Role::List,
            OverlayFocus::Contain => Role::Menu,
        };
        div()
            .class("w-56 p-1 flex flex-col gap-1 bg-background border")
            .role(role)
            .label("Opciones")
            .test_id("panel")
            .children([
                button("Uno", ButtonVariant::Text)
                    .on_click(Msg::Pick("uno"))
                    .test_id("one"),
                button("Dos", ButtonVariant::Text)
                    .on_click(Msg::Pick("dos"))
                    .test_id("two"),
                div().class("h-10").test_id("blank"),
            ])
            .floating(Floating::below().focus(self.focus).on_dismiss(Msg::Close))
    }
}

impl App for Picker {
    type Msg = Msg;

    fn title(&self) -> String {
        "Flotante".into()
    }

    fn update(&mut self, msg: Msg) -> Effects<Msg> {
        match msg {
            Msg::Toggle => self.open = !self.open,
            Msg::Close => self.open = false,
            Msg::Pick(which) => {
                self.picked.push(which);
                self.open = false;
            }
            Msg::Outside => self.outside += 1,
            Msg::Query(query) => {
                self.query = query;
                self.open = true;
            }
        }
        Effects::none()
    }

    fn view(&self) -> Element<Msg> {
        let mut anchor = if self.field {
            div().class("w-60").test_id("anchor").child(
                input(self.query.clone(), "Buscar")
                    .label("Buscar")
                    .on_input(Msg::Query)
                    .test_id("field"),
            )
        } else {
            div()
                .class("w-60 h-8 px-2 border")
                .role(Role::Button)
                .label("Abrir")
                .expanded(self.open)
                .on_click(Msg::Toggle)
                .test_id("anchor")
                .child(text("Abrir"))
        };
        if self.open {
            anchor = anchor.child(self.panel());
        }
        let place = if self.at_bottom {
            "absolute left-4 bottom-4"
        } else {
            "absolute left-4 top-4"
        };
        div().class("relative w-full h-full").children([
            div()
                .class(&format!("{place} h-12 overflow-hidden"))
                .test_id("clip")
                .child(anchor),
            div().class("absolute right-4 top-4").child(
                button("Fuera", ButtonVariant::Secondary)
                    .on_click(Msg::Outside)
                    .test_id("outside"),
            ),
        ])
    }
}

fn by(test_id: &str) -> By {
    By::test_id(test_id)
}

fn bounds(app: &TestApp<Picker>, test_id: &str) -> Rect {
    app.screen().get(by(test_id)).bounds()
}

fn is_open(app: &TestApp<Picker>) -> bool {
    app.screen().query(by("panel")).is_some()
}

fn open_combobox() -> TestApp<Picker> {
    let mut app = TestApp::new(Picker::new(OverlayFocus::Anchor));
    app.type_text(by("field"), "u");
    assert!(is_open(&app));
    app
}

fn open_menu() -> TestApp<Picker> {
    let mut app = TestApp::new(Picker::new(OverlayFocus::Contain));
    app.click(by("anchor"));
    assert!(is_open(&app));
    app
}

#[test]
fn hangs_below_its_anchor_outside_the_clipping_box() {
    let app = open_combobox();
    let (anchor, clip, panel) = (
        bounds(&app, "anchor"),
        bounds(&app, "clip"),
        bounds(&app, "panel"),
    );
    assert_eq!(panel.x, anchor.x);
    assert_eq!(panel.y, anchor.y + anchor.height + 4.0);
    assert!(panel.y + panel.height > clip.y + clip.height, "not clipped");
    // The parts outside the clipping box take clicks.
    let mut app = app;
    app.click(by("two"));
    assert_eq!(app.app().picked, ["dos"]);
}

#[test]
fn flips_above_at_the_bottom_of_the_window() {
    let mut picker = Picker::new(OverlayFocus::Anchor);
    picker.at_bottom = true;
    let mut app = TestApp::new(picker);
    app.type_text(by("field"), "u");
    let (anchor, panel) = (bounds(&app, "anchor"), bounds(&app, "panel"));
    assert_eq!(panel.y + panel.height, anchor.y - 4.0);
    let screen = app.screen();
    let layout_side = screen
        .get(by("panel"))
        .node()
        .floating
        .map(|placement| placement.side);
    assert_eq!(layout_side, Some(Side::Above));
}

#[test]
fn is_found_by_its_role_and_name() {
    let app = open_combobox();
    assert!(
        app.screen()
            .query(By::role(Role::List).name("Opciones"))
            .is_some()
    );
    let app = open_menu();
    assert!(
        app.screen()
            .query(By::role(Role::Menu).name("Opciones"))
            .is_some()
    );
}

#[test]
fn combobox_escape_dismisses_and_keeps_the_focus_in_the_field() {
    let mut app = open_combobox();
    assert!(app.is_focused(by("field")));
    app.press(Key::Escape, Modifiers::NONE);
    assert!(!is_open(&app));
    assert!(app.is_focused(by("field")));
    assert_eq!(app.value(by("field")), "u");
    // With nothing open, Escape is the app's again.
    app.press(Key::Escape, Modifiers::NONE);
    assert!(!is_open(&app));
}

#[test]
fn combobox_typing_and_tab_go_on_while_it_is_open() {
    let mut app = open_combobox();
    app.type_text(by("field"), "n");
    assert!(is_open(&app));
    assert_eq!(app.app().query, "un");
    // Tab goes through the list, right after the field.
    app.press(Key::Tab, Modifiers::NONE);
    assert!(app.is_focused(by("one")));
}

#[test]
fn combobox_press_outside_dismisses_and_goes_through() {
    let mut app = open_combobox();
    app.click(by("outside"));
    assert!(!is_open(&app));
    assert_eq!(app.app().outside, 1, "the press reaches the button");
}

#[test]
fn presses_inside_or_on_the_anchor_do_not_dismiss() {
    let mut app = open_combobox();
    app.click(by("blank"));
    assert!(is_open(&app));
    app.click(by("field"));
    assert!(is_open(&app));
    assert!(app.app().picked.is_empty());
}

#[test]
fn a_press_on_its_background_does_not_reach_the_anchor() {
    // A button with a hover card: the button stays in the Tab order and
    // would take a press on the card's background.
    let mut picker = Picker::new(OverlayFocus::Anchor);
    picker.field = false;
    let mut app = TestApp::new(picker);
    app.click(by("anchor"));
    assert!(is_open(&app));
    app.click(by("blank"));
    assert!(is_open(&app), "the anchor's toggle did not run");
    assert_eq!(
        app.messages()
            .iter()
            .filter(|msg| **msg == Msg::Toggle)
            .count(),
        1,
        "only the click that opened it"
    );
    app.click(by("anchor"));
    assert!(!is_open(&app), "the anchor itself still toggles");
}

#[test]
fn menu_takes_the_focus_keeps_tab_inside_and_gives_it_back_on_escape() {
    let mut app = TestApp::new(Picker::new(OverlayFocus::Contain));
    app.press(Key::Tab, Modifiers::NONE);
    assert!(app.is_focused(by("anchor")));
    app.press(Key::Space, Modifiers::NONE);
    assert!(is_open(&app));
    assert!(app.is_focused(by("one")), "first control inside");
    app.press(Key::Tab, Modifiers::NONE);
    assert!(app.is_focused(by("two")));
    app.press(Key::Tab, Modifiers::NONE);
    assert!(app.is_focused(by("one")), "Tab wraps inside");
    app.press(Key::Escape, Modifiers::NONE);
    assert!(!is_open(&app));
    assert!(app.is_focused(by("anchor")), "back where it was");
}

#[test]
fn menu_pick_closes_and_gives_the_focus_back() {
    let mut app = TestApp::new(Picker::new(OverlayFocus::Contain));
    app.press(Key::Tab, Modifiers::NONE);
    app.press(Key::Space, Modifiers::NONE);
    app.press(Key::Tab, Modifiers::NONE);
    app.press(Key::Space, Modifiers::NONE);
    assert_eq!(app.app().picked, ["dos"]);
    assert!(!is_open(&app));
    assert!(app.is_focused(by("anchor")));
}

#[test]
fn menu_press_outside_only_dismisses() {
    let mut app = TestApp::new(Picker::new(OverlayFocus::Contain));
    app.press(Key::Tab, Modifiers::NONE);
    app.press(Key::Space, Modifiers::NONE);
    app.click(by("outside"));
    assert!(!is_open(&app));
    assert_eq!(app.app().outside, 0, "the press stops at the menu");
    assert!(app.is_focused(by("anchor")));
}

#[test]
fn paints_after_what_it_hangs_over() {
    let app = open_combobox();
    let panel = bounds(&app, "panel");
    let outside = bounds(&app, "outside");
    let list = app.draw_list();
    let mut depth = 0;
    let mut found = None;
    for (index, op) in list.ops.iter().enumerate() {
        match op {
            DrawOp::PushClip(_) => depth += 1,
            DrawOp::PopClip => depth -= 1,
            DrawOp::Quad(quad) if quad.rect == panel && found.is_none() => {
                found = Some((index, depth));
            }
            _ => {}
        }
    }
    let (index, depth) = found.expect("the panel paints");
    assert_eq!(depth, 0, "outside the clipping box's clip");
    // After the rest of the content (the clipping box is painted before
    // the button on the right, the panel after both).
    let button = list
        .ops
        .iter()
        .position(|op| matches!(op, DrawOp::Quad(quad) if quad.rect == outside))
        .expect("the button paints");
    assert!(index > button);
}
