//! Search field (`NSSearchField`, 12 Paso 20.5).

use std::rc::Rc;

use super::themed;
use crate::element::{ControlSize, Element, Role, div, icon, input};
use crate::icons::Icon;
use crate::responder::Action;

/// Text input with a magnifier and, while it holds text, a button that
/// clears it. Escape clears it too; on an empty field Escape goes on up
/// the responder chain (a dialog's cancel, for instance). Every edit,
/// clearing included, goes through `on_input`, so the app keeps the value.
///
/// Parts: `<id>.icon`, `<id>.input` and `<id>.clear`.
pub struct SearchField<Msg> {
    value: String,
    placeholder: String,
    label: String,
    on_input: Rc<dyn Fn(String) -> Msg>,
    on_submit: Option<Msg>,
    size: Option<ControlSize>,
    disabled: bool,
    test_id: Option<String>,
}

/// A [`SearchField`] showing `value`, sending each edit to `on_input`.
pub fn search_field<Msg>(
    value: impl Into<String>,
    on_input: impl Fn(String) -> Msg + 'static,
) -> SearchField<Msg> {
    SearchField {
        value: value.into(),
        placeholder: "Buscar".to_owned(),
        label: "Buscar".to_owned(),
        on_input: Rc::new(on_input),
        on_submit: None,
        size: None,
        disabled: false,
        test_id: None,
    }
}

impl<Msg: Clone + 'static> SearchField<Msg> {
    pub fn placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.placeholder = placeholder.into();
        self
    }

    /// Accessible name (default «Buscar»).
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = label.into();
        self
    }

    /// Sent on Return; without it, searching follows the typing.
    pub fn on_submit(mut self, msg: Msg) -> Self {
        self.on_submit = Some(msg);
        self
    }

    pub fn control_size(mut self, size: ControlSize) -> Self {
        self.size = Some(size);
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn test_id(mut self, test_id: impl Into<String>) -> Self {
        self.test_id = Some(test_id.into());
        self
    }

    pub fn build(self) -> Element<Msg> {
        let empty = self.value.is_empty();
        let clear = (self.on_input)(String::new());
        let on_action = {
            let clear = clear.clone();
            move |action| (action == Action::CancelOperation && !empty).then(|| clear.clone())
        };
        let mut field = input(self.value.clone(), self.placeholder.clone())
            .class(&themed(
                "w-full self-stretch bg-transparent text-sm leading-5 text-foreground \
                 placeholder-muted-foreground cursor-text",
                "w-full self-stretch bg-transparent text-control text-label \
                 placeholder-placeholder cursor-text",
            ))
            .role(Role::SearchInput)
            .label(self.label.clone())
            .disabled(self.disabled)
            .on_action(on_action)
            .claims_escape(!empty)
            .part("input");
        field.handlers.on_input = Some(self.on_input.clone());
        field.handlers.on_submit = self.on_submit.clone();

        let mut children = vec![
            icon(Icon::Search)
                .class(&themed(
                    "size-4 shrink-0 text-muted-foreground",
                    "size-[13px] shrink-0 text-secondary-label",
                ))
                .part("icon"),
            div()
                .class("flex flex-1 min-w-0 self-stretch items-center")
                .child(field),
        ];
        if !empty && !self.disabled {
            children.push(
                div()
                    .class(&themed(
                        "inline-flex size-6 shrink-0 -mr-1.5 items-center justify-center \
                         rounded-full text-muted-foreground cursor-pointer hover:bg-primary/10 \
                         hover:text-foreground",
                        "inline-flex size-[16px] shrink-0 items-center justify-center \
                         rounded-full text-tertiary-label cursor-default hover:text-secondary-label",
                    ))
                    .role(Role::Button)
                    .label("Borrar búsqueda")
                    .focus_on_press(false)
                    .on_click(clear)
                    .part("clear")
                    .child(icon(Icon::CircleX).class("size-4")),
            );
        }
        let cursor = if self.disabled {
            "cursor-not-allowed opacity-50"
        } else {
            "cursor-text"
        };
        let mut root = div()
            .class(&themed(
                &format!(
                    "group flex items-center gap-2 w-full min-w-[160px] h-9 px-3 rounded-full \
                     border border-border bg-background hover:border-muted-foreground \
                     focus-within:border-primary {cursor}"
                ),
                &format!(
                    "group flex items-center gap-1 w-full min-w-[140px] h-field px-1.5 \
                     rounded-full border-[0.5px] border-separator bg-field \
                     focus-within:ring-focus window-inactive:ring-0 mix-c:border \
                     mix-c:border-tertiary-label {cursor}"
                ),
            ))
            .delegates_focus()
            .parts_root()
            .children(children);
        root.test_id = self.test_id;
        if let Some(size) = self.size {
            root = root.control_size(size);
        }
        root
    }
}

#[cfg(test)]
mod tests {
    use super::search_field;
    use crate::components::{ButtonVariant, button};
    use crate::element::{Element, Role, div};
    use crate::runtime::SimpleApp;
    use crate::testing::{By, TestApp};
    use crate::ui_engine::{Key, Modifiers};

    #[derive(Default)]
    struct Search {
        query: String,
        submitted: u32,
        cancelled: u32,
    }

    #[derive(Clone, Debug, PartialEq)]
    enum Msg {
        Query(String),
        Submit,
        Cancel,
    }

    impl SimpleApp for Search {
        type Msg = Msg;
        fn title(&self) -> String {
            "Buscar".into()
        }
        fn update(&mut self, msg: Msg) {
            match msg {
                Msg::Query(query) => self.query = query,
                Msg::Submit => self.submitted += 1,
                Msg::Cancel => self.cancelled += 1,
            }
        }
        fn view(&self) -> Element<Msg> {
            div().class("flex flex-col gap-2 p-4").children([
                search_field(&self.query, Msg::Query)
                    .on_submit(Msg::Submit)
                    .test_id("buscar")
                    .build(),
                button("Cancelar", ButtonVariant::Text)
                    .on_click(Msg::Cancel)
                    .cancel_button(),
            ])
        }
    }

    #[test]
    fn typing_searches_and_the_clear_button_or_escape_empty_it() {
        let mut app = TestApp::new(Search::default());
        app.screen().get(By::role(Role::SearchInput).name("Buscar"));
        assert!(app.screen().query(By::test_id("buscar.clear")).is_none());

        app.type_text(By::test_id("buscar"), "áreas");
        assert_eq!(app.app().query, "áreas");
        app.press(Key::Enter, Modifiers::NONE);
        assert_eq!(app.app().submitted, 1);

        app.click(By::test_id("buscar.clear"));
        assert_eq!(app.app().query, "");
        assert!(app.is_focused(By::test_id("buscar")), "focus stays");
        assert!(app.screen().query(By::test_id("buscar.clear")).is_none());

        app.type_text(By::test_id("buscar"), "x");
        app.press(Key::Escape, Modifiers::NONE);
        assert_eq!(app.app().query, "");
        assert_eq!(app.app().cancelled, 0, "Escape cleared the field");
        app.press(Key::Escape, Modifiers::NONE);
        assert_eq!(app.app().cancelled, 1, "empty: the cancel button");
    }
}
