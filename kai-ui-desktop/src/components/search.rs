//! Search field (`NSSearchField`, 12 Paso 20.5).

use std::rc::Rc;

use super::themed;
use crate::element::{ControlSize, Element, Role, div, icon, input};
use crate::icons::Icon;
use crate::responder::Action;

/// Where a [`SearchField`] goes, which decides its look.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SearchVariant {
    /// A toolbar or a global search: a bordered capsule
    /// (`.roundedBezel` in AppKit).
    #[default]
    Toolbar,
    /// In a form, next to text fields: a capsule with their border
    /// (`.roundedBezel` in AppKit).
    Field,
    /// Inside a table cell or a dense list: no minimum width, a faint
    /// fill, and in Kai a border only under the pointer or with focus.
    Inline,
}

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
    variant: SearchVariant,
    size: Option<ControlSize>,
    disabled: bool,
    test_id: Option<String>,
}

/// Kai measures by control size: the box, the input's text, the
/// magnifier, the clear button and its icon.
struct KaiMetrics {
    field: &'static str,
    text: &'static str,
    icon: &'static str,
    clear: &'static str,
    clear_icon: &'static str,
}

impl KaiMetrics {
    fn of(size: ControlSize) -> Self {
        match size {
            ControlSize::Mini => Self {
                field: "h-6 px-2 gap-1",
                text: "text-xs leading-4",
                icon: "size-3",
                clear: "size-4 -mr-1",
                clear_icon: "size-3",
            },
            ControlSize::Small => Self {
                field: "h-7 px-2.5 gap-1.5",
                text: "text-xs leading-4",
                icon: "size-3.5",
                clear: "size-5 -mr-1",
                clear_icon: "size-3.5",
            },
            ControlSize::Regular => Self {
                field: "h-9 px-3 gap-2",
                text: "text-sm leading-5",
                icon: "size-4",
                clear: "size-6 -mr-1.5",
                clear_icon: "size-4",
            },
            ControlSize::Large | ControlSize::ExtraLarge => Self {
                field: "h-11 px-4 gap-2",
                text: "text-base leading-6",
                icon: "size-[18px]",
                clear: "size-7 -mr-2",
                clear_icon: "size-[18px]",
            },
        }
    }
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
        variant: SearchVariant::Toolbar,
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

    pub fn variant(mut self, variant: SearchVariant) -> Self {
        self.variant = variant;
        self
    }

    /// Its size. In AppKit it also reaches the field from an ancestor's
    /// size; in Kai only the one given here counts.
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
        let kai = KaiMetrics::of(self.size.unwrap_or_default());
        let mut field = input(self.value.clone(), self.placeholder.clone())
            .class(&themed(
                &format!(
                    "w-full self-stretch bg-transparent {} text-foreground \
                     placeholder-muted-foreground cursor-text",
                    kai.text
                ),
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
                    &format!("{} shrink-0 text-muted-foreground", kai.icon),
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
                        &format!(
                            "inline-flex {} shrink-0 items-center justify-center rounded-full \
                             text-muted-foreground cursor-pointer hover:bg-primary/10 \
                             hover:text-foreground",
                            kai.clear
                        ),
                        "inline-flex size-[16px] shrink-0 items-center justify-center \
                         rounded-full text-tertiary-label cursor-default hover:text-secondary-label",
                    ))
                    .role(Role::Button)
                    .label("Borrar búsqueda")
                    .focus_on_press(false)
                    .on_click(clear)
                    .part("clear")
                    .child(icon(Icon::CircleX).class(&themed(kai.clear_icon, "size-4"))),
            );
        }
        let cursor = if self.disabled {
            "cursor-not-allowed opacity-50"
        } else {
            "cursor-text"
        };
        let (kai_look, appkit_look) = match self.variant {
            SearchVariant::Toolbar => (
                "min-w-[160px] rounded-full border border-border bg-background \
                 hover:border-muted-foreground focus-within:border-primary",
                "min-w-[140px] h-field px-1.5 rounded-full bg-field",
            ),
            SearchVariant::Field => (
                "min-w-0 rounded-full border border-border bg-background \
                 hover:border-muted-foreground focus-within:border-primary",
                "min-w-0 h-field px-1.5 rounded-full bg-field",
            ),
            SearchVariant::Inline => (
                "min-w-0 rounded-full border border-transparent bg-muted/40 \
                 hover:border-border focus-within:border-primary focus-within:bg-background",
                "min-w-0 h-field px-1.5 rounded-full bg-field",
            ),
        };
        let mut root = div()
            .class(&themed(
                &format!(
                    "group flex items-center w-full {} {kai_look} {cursor}",
                    kai.field
                ),
                &format!(
                    "group flex items-center gap-1 w-full {appkit_look} border-[0.5px] \
                     border-separator focus-within:ring-focus window-inactive:ring-0 \
                     mix-c:border mix-c:border-tertiary-label {cursor}"
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
    use super::{SearchVariant, search_field};
    use crate::components::{ButtonVariant, button};
    use crate::element::{ControlSize, Element, Role, div};
    use crate::runtime::SimpleApp;
    use crate::testing::{By, TestApp};
    use crate::theme::{AppKitGeneration, Appearance, Theme};
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

    /// One field per variant and size; the last two in 60-point boxes.
    struct Variants;

    impl SimpleApp for Variants {
        type Msg = String;
        fn title(&self) -> String {
            "Variantes".into()
        }
        fn update(&mut self, _: String) {}
        fn view(&self) -> Element<String> {
            let field = |id: &str, variant, size: Option<ControlSize>| {
                let mut field = search_field("", |text| text).variant(variant).test_id(id);
                if let Some(size) = size {
                    field = field.control_size(size);
                }
                field.build()
            };
            let narrow = |child| div().class("flex w-[60px]").child(child);
            div()
                .class("flex flex-col items-start gap-2 p-4")
                .children([
                    field("toolbar", SearchVariant::Toolbar, None),
                    field("mini", SearchVariant::Toolbar, Some(ControlSize::Mini)),
                    field("small", SearchVariant::Toolbar, Some(ControlSize::Small)),
                    field("large", SearchVariant::Toolbar, Some(ControlSize::Large)),
                    field("field", SearchVariant::Field, Some(ControlSize::Small)),
                    narrow(field(
                        "inline",
                        SearchVariant::Inline,
                        Some(ControlSize::Small),
                    )),
                    narrow(field("toolbar.narrow", SearchVariant::Toolbar, None)),
                ])
        }
    }

    fn size(app: &TestApp<Variants>, id: &str) -> (f32, f32) {
        let bounds = app.screen().get(By::test_id(id)).bounds();
        (bounds.width, bounds.height)
    }

    #[test]
    fn kai_sizes_shrink_the_field_and_inline_fits_a_narrow_cell() {
        let app = TestApp::new(Variants);
        let height = |id| size(&app, id).1;
        assert_eq!(height("toolbar"), 36.0);
        assert_eq!(height("mini"), 24.0);
        assert_eq!(height("small"), 28.0);
        assert_eq!(height("large"), 44.0);
        assert_eq!(height("field"), 28.0);
        assert_eq!(size(&app, "inline"), (60.0, 28.0), "no minimum width");
        assert_eq!(size(&app, "toolbar.narrow").0, 160.0);
        let text = |id: &str| {
            let input = app.screen().get(By::test_id(format!("{id}.input")));
            input.node().text.font_size
        };
        assert!(text("small") < text("toolbar"), "smaller text");
        for id in ["toolbar", "field", "inline"] {
            let node = app.screen().get(By::test_id(id)).node();
            assert!(
                node.style.radius >= node.bounds.height / 2.0,
                "{id} is a capsule"
            );
        }
    }

    #[test]
    fn appkit_sizes_come_from_the_bezel_heights() {
        let theme = Theme::appkit_static(AppKitGeneration::Glass27, Appearance::Light);
        let field = theme.controls.field_height;
        let app = TestApp::builder(Variants).theme(theme).build();
        let height = |id| size(&app, id).1;
        let small = ControlSize::Small as usize;
        assert_eq!(height("toolbar"), field[ControlSize::Regular as usize]);
        assert_eq!(height("small"), field[small]);
        assert_eq!(height("field"), field[small]);
        assert_eq!(size(&app, "inline"), (60.0, field[small]));
    }
}
