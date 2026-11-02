//! `Effects::copy_text`: a «Copiar» button puts its text on the clipboard
//! through the session's `ClipboardPort` (`MemoryClipboard` in tests),
//! also when the effect comes from a page through `Effects::map`.

use kai::components::{ButtonVariant, button};
use kai::element::{Element, Role, div};
use kai::runtime::{App, Effects};
use kai::testing::{By, MemoryClipboard, TestApp};

#[derive(Clone, Debug, PartialEq)]
enum Msg {
    Copy,
    Inner(InnerMsg),
}

#[derive(Clone, Debug, PartialEq)]
enum InnerMsg {
    Copy,
}

struct Snippet {
    code: String,
}

impl App for Snippet {
    type Msg = Msg;

    fn title(&self) -> String {
        "Copiar".into()
    }

    fn update(&mut self, msg: Msg) -> Effects<Msg> {
        match msg {
            Msg::Copy => Effects::copy_text(self.code.clone()),
            // As a page's effects reach the root.
            Msg::Inner(InnerMsg::Copy) => {
                Effects::<InnerMsg>::copy_text(format!("// mapeado\n{}", self.code)).map(Msg::Inner)
            }
        }
    }

    fn view(&self) -> Element<Msg> {
        div().class("flex flex-col gap-2").children([
            button("Copiar", ButtonVariant::Outlined)
                .on_click(Msg::Copy)
                .test_id("copy"),
            button("Copiar mapeado", ButtonVariant::Outlined)
                .on_click(Msg::Inner(InnerMsg::Copy))
                .test_id("copy.mapped"),
        ])
    }
}

#[test]
fn copy_text_puts_the_text_on_the_clipboard() {
    let clipboard = MemoryClipboard::new();
    let mut app = TestApp::builder(Snippet {
        code: "button(\"Guardar\", ButtonVariant::Primary)".into(),
    })
    .clipboard(clipboard.clone())
    .build();
    assert_eq!(clipboard.text(), None);

    app.click(By::test_id("copy"));
    assert_eq!(
        clipboard.text().as_deref(),
        Some("button(\"Guardar\", ButtonVariant::Primary)")
    );
    assert_eq!(app.clipboard().text(), clipboard.text());

    app.click(By::role(Role::Button).name("Copiar mapeado"));
    assert_eq!(
        clipboard.text().as_deref(),
        Some("// mapeado\nbutton(\"Guardar\", ButtonVariant::Primary)")
    );
}

#[test]
fn copy_text_is_named_in_the_effects_debug() {
    let effects = Effects::<()>::copy_text("hola").keyed("ignored");
    assert_eq!(format!("{effects:?}"), "[copy_text(\"hola\")]");
    assert!(!effects.is_none());
}
