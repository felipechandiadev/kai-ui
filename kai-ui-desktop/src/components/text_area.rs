//! Text area (`NSTextView` in a bordered scroll view): a [`TextField`] of
//! several lines.

use std::rc::Rc;

use super::TextField;
use crate::element::Element;

/// Multiline field with the look of [`TextField`]: Return breaks the line
/// and ⌘Return sends `on_submit`. It shows a fixed number of rows
/// ([`TextArea::rows`], 3 by default) or grows with its text
/// ([`TextArea::auto_grow`]) and scrolls beyond. `max_length` cuts what the
/// user types and shows a counter under the field.
///
/// Parts: those of [`TextField`] plus `<id>.counter`.
pub struct TextArea<Msg> {
    field: TextField<Msg>,
    rows: (u16, u16),
    max_length: Option<usize>,
    counter: Option<bool>,
    on_input: Option<Rc<dyn Fn(String) -> Msg>>,
}

/// A [`TextArea`] labeled `label` showing `value`.
pub fn text_area<Msg>(label: impl Into<String>, value: impl Into<String>) -> TextArea<Msg> {
    TextArea {
        field: TextField::new(label, value),
        rows: (3, 3),
        max_length: None,
        counter: None,
        on_input: None,
    }
}

impl<Msg: 'static> TextArea<Msg> {
    /// Always `rows` lines tall.
    pub fn rows(mut self, rows: u16) -> Self {
        self.rows = (rows, rows);
        self
    }

    /// From `min` lines, growing with the text up to `max`.
    pub fn auto_grow(mut self, min: u16, max: u16) -> Self {
        self.rows = (min, max);
        self
    }

    /// At most `max` characters: longer edits are cut, and the counter
    /// shows `n/max` unless [`TextArea::counter`] hides it.
    pub fn max_length(mut self, max: usize) -> Self {
        self.max_length = Some(max);
        self
    }

    /// Show the character count (on by default with `max_length`).
    pub fn counter(mut self, show: bool) -> Self {
        self.counter = Some(show);
        self
    }

    pub fn placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.field = self.field.placeholder(placeholder);
        self
    }

    pub fn helper_text(mut self, text: impl Into<String>) -> Self {
        self.field = self.field.helper_text(text);
        self
    }

    pub fn required(mut self, required: bool) -> Self {
        self.field = self.field.required(required);
        self
    }

    pub fn read_only(mut self, read_only: bool) -> Self {
        self.field = self.field.read_only(read_only);
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.field = self.field.disabled(disabled);
        self
    }

    pub fn on_input(mut self, handler: impl Fn(String) -> Msg + 'static) -> Self {
        self.on_input = Some(Rc::new(handler));
        self
    }

    /// Sent on ⌘Return.
    pub fn on_submit(mut self, msg: Msg) -> Self {
        self.field = self.field.on_submit(msg);
        self
    }

    pub fn test_id(mut self, test_id: impl Into<String>) -> Self {
        self.field = self.field.test_id(test_id);
        self
    }

    pub fn build(self) -> Element<Msg> {
        let mut field = self.field;
        let count = field.value.chars().count();
        field.rows = Some(self.rows);
        field.counter =
            self.counter
                .unwrap_or(self.max_length.is_some())
                .then(|| match self.max_length {
                    Some(max) => format!("{count}/{max}"),
                    None => count.to_string(),
                });
        field.on_input = match (self.on_input, self.max_length) {
            (Some(handler), Some(max)) => Some(Rc::new(move |value: String| {
                handler(value.chars().take(max).collect())
            })),
            (handler, None) => handler,
            (None, Some(_)) => None,
        };
        field.build()
    }
}

#[cfg(test)]
mod tests {
    use super::text_area;
    use crate::element::{Element, Role, div};
    use crate::runtime::SimpleApp;
    use crate::testing::{By, TestApp};
    use crate::ui_engine::{Key, Modifiers};

    #[derive(Default)]
    struct Notes {
        text: String,
        sent: u32,
    }

    #[derive(Clone, Debug, PartialEq)]
    enum Msg {
        Edit(String),
        Send,
    }

    impl SimpleApp for Notes {
        type Msg = Msg;
        fn title(&self) -> String {
            "Notas".into()
        }
        fn update(&mut self, msg: Msg) {
            match msg {
                Msg::Edit(text) => self.text = text,
                Msg::Send => self.sent += 1,
            }
        }
        fn view(&self) -> Element<Msg> {
            div()
                .class("flex flex-col gap-2 p-4 w-[320px]")
                .children([text_area("Notas", &self.text)
                    .auto_grow(2, 4)
                    .max_length(40)
                    .on_input(Msg::Edit)
                    .on_submit(Msg::Send)
                    .test_id("notas")
                    .build()])
        }
    }

    const COMMAND: Modifiers = Modifiers {
        command: true,
        ..Modifiers::NONE
    };

    #[test]
    fn return_breaks_lines_command_return_submits_and_the_area_grows() {
        let mut app = TestApp::new(Notes::default());
        app.screen().get(By::role(Role::TextArea).name("Notas"));
        let height =
            |app: &TestApp<Notes>| app.screen().get(By::test_id("notas.input")).bounds().height;
        let two_rows = height(&app);

        app.type_text(By::test_id("notas"), "uno");
        app.press(Key::Enter, Modifiers::NONE);
        app.type_text(By::test_id("notas"), "dos");
        assert_eq!(app.app().text, "uno\ndos");
        assert_eq!(app.app().sent, 0, "Return is a line break");
        assert_eq!(height(&app), two_rows);

        app.press(Key::Enter, Modifiers::NONE);
        app.type_text(By::test_id("notas"), "tres");
        assert!(height(&app) > two_rows, "grows with its lines");
        app.press(Key::Enter, Modifiers::NONE);
        app.press(Key::Enter, Modifiers::NONE);
        app.press(Key::Enter, Modifiers::NONE);
        let tallest = height(&app);
        assert!((tallest - two_rows * 2.0).abs() < 0.5, "stops at 4 rows");

        app.press(Key::Enter, COMMAND);
        assert_eq!(app.app().sent, 1);
        assert_eq!(
            app.screen()
                .get(By::test_id("notas.counter"))
                .name()
                .as_deref(),
            Some("15/40")
        );
    }

    #[test]
    fn max_length_cuts_what_is_typed() {
        let mut app = TestApp::new(Notes::default());
        app.type_text(By::test_id("notas"), &"x".repeat(50));
        assert_eq!(app.app().text.chars().count(), 40);
        assert_eq!(app.value(By::test_id("notas")), "x".repeat(40));
        assert_eq!(
            app.screen()
                .get(By::test_id("notas.counter"))
                .name()
                .as_deref(),
            Some("40/40")
        );
    }
}
