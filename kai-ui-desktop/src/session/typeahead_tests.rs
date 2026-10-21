//! Typing in an open pop-up list highlights the option it begins.

use std::time::Duration;

use super::{TYPEAHEAD_RESET, typeahead};
use crate::components::{MENU, select};
use crate::element::{Element, div};
use crate::runtime::{App, Effects};
use crate::testing::{By, TestApp};
use crate::ui_engine::{Key, Modifiers};

const FRUTAS: [&str; 6] = ["Pera", "Mango", "Manzana", "Óleo", "Melón", "Limón"];

#[derive(Default)]
struct Frutas {
    chosen: Option<usize>,
}

impl App for Frutas {
    type Msg = usize;

    fn title(&self) -> String {
        "Frutas".into()
    }

    fn update(&mut self, index: usize) -> Effects<usize> {
        self.chosen = Some(index);
        Effects::none()
    }

    fn view(&self) -> Element<usize> {
        div()
            .class("p-4")
            .child(select(&FRUTAS, self.chosen, "Elegir fruta", |index| index).test_id("fruta"))
    }
}

fn row(index: usize) -> By {
    By::test_id(format!("{MENU}.{index}"))
}

fn typed(app: &mut TestApp<Frutas>, text: &str) {
    for c in text.chars() {
        app.press(Key::Char(c), Modifiers::NONE);
    }
}

#[test]
fn letters_go_to_the_option_they_begin_and_the_same_letter_goes_on() {
    let mut app = TestApp::new(Frutas::default());
    app.click(By::test_id("fruta"));
    assert!(app.is_focused(row(0)));

    typed(&mut app, "m");
    assert!(app.is_focused(row(1)), "Mango");
    typed(&mut app, "m");
    assert!(app.is_focused(row(2)), "the same letter: Manzana");
    typed(&mut app, "m");
    assert!(app.is_focused(row(4)), "Melón");

    app.advance(TYPEAHEAD_RESET + Duration::from_millis(1));
    typed(&mut app, "manz");
    assert!(app.is_focused(row(2)), "a pause starts a new search");

    app.advance(TYPEAHEAD_RESET + Duration::from_millis(1));
    typed(&mut app, "o");
    assert!(app.is_focused(row(3)), "accents do not count: Óleo");
    typed(&mut app, "z");
    assert!(app.is_focused(row(3)), "no option begins with «oz»");

    app.press(Key::Enter, Modifiers::NONE);
    assert_eq!(app.app().chosen, Some(3));
}

#[test]
fn typeahead_wraps_and_finds_nothing_for_unknown_letters() {
    let items: Vec<String> = FRUTAS.iter().map(|item| (*item).to_owned()).collect();
    assert_eq!(typeahead(&items, 5, "p"), Some(0), "wraps to the top");
    assert_eq!(
        typeahead(&items, 1, "mang"),
        Some(1),
        "the current one fits"
    );
    assert_eq!(typeahead(&items, 0, "LIM"), Some(5));
    assert_eq!(typeahead(&items, 0, "q"), None);
    assert_eq!(typeahead(&items, 0, ""), None);
}
