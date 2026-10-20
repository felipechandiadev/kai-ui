//! Dialogs (`NSAlert` and sheets, 12 Paso 20.6).

use super::{Button, themed};
use crate::element::{Element, Role, div, text};

/// Panel width of a [`Dialog`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum DialogWidth {
    /// 360 pt, an alert.
    Sm,
    /// 480 pt.
    #[default]
    Md,
    /// 640 pt, a form.
    Lg,
}

/// Modal dialog: a dimmed backdrop over the window and a panel with
/// [`Role::Dialog`] named after its title, holding the title, a message or
/// any content, and a right-aligned row of buttons: extra ones, then the
/// cancel button (Escape and ⌘.) and the default one (Return), red when
/// the dialog is destructive.
///
/// With `on_dismiss` or a cancel button the panel is an
/// [`Element::modal`]: the session keeps the focus inside, ignores clicks
/// outside and gives the focus back when the app stops showing it. Escape
/// sends the cancel button's message, else `on_dismiss`.
///
/// Parts: `panel`, `title`, `body`, `actions`, `cancel` and `default`.
pub struct Dialog<Msg> {
    title: String,
    message: Option<String>,
    content: Option<Element<Msg>>,
    buttons: Vec<Element<Msg>>,
    cancel: Option<(String, Msg)>,
    default: Option<(String, Msg)>,
    destructive: bool,
    width: DialogWidth,
    on_dismiss: Option<Msg>,
    test_id: Option<String>,
}

impl<Msg: Clone + 'static> Dialog<Msg> {
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            message: None,
            content: None,
            buttons: Vec::new(),
            cancel: None,
            default: None,
            destructive: false,
            width: DialogWidth::default(),
            on_dismiss: None,
            test_id: None,
        }
    }

    /// Informative text under the title.
    pub fn message(mut self, message: impl Into<String>) -> Self {
        self.message = Some(message.into());
        self
    }

    /// Free content under the title (a form, a list), after the message.
    pub fn content(mut self, content: Element<Msg>) -> Self {
        self.content = Some(content);
        self
    }

    /// An extra button, before the cancel and default ones.
    pub fn button(mut self, button: Element<Msg>) -> Self {
        self.buttons.push(button);
        self
    }

    /// Buttons that dismiss: Escape and ⌘. press it.
    pub fn cancel_button(mut self, label: impl Into<String>, msg: Msg) -> Self {
        self.cancel = Some((label.into(), msg));
        self
    }

    /// The button Return presses, with the accent color.
    pub fn default_button(mut self, label: impl Into<String>, msg: Msg) -> Self {
        self.default = Some((label.into(), msg));
        self
    }

    /// The default button destroys something: it turns red.
    pub fn destructive(mut self, destructive: bool) -> Self {
        self.destructive = destructive;
        self
    }

    pub fn width(mut self, width: DialogWidth) -> Self {
        self.width = width;
        self
    }

    /// Sent by Escape and ⌘. when there is no cancel button.
    pub fn on_dismiss(mut self, msg: Msg) -> Self {
        self.on_dismiss = Some(msg);
        self
    }

    pub fn test_id(mut self, test_id: impl Into<String>) -> Self {
        self.test_id = Some(test_id.into());
        self
    }

    pub fn build(self) -> Element<Msg> {
        let style =
            |layout: &str, kai: &str, appkit: &str| format!("{layout} {}", themed(kai, appkit));
        let width = match self.width {
            DialogWidth::Sm => "w-[360px]",
            DialogWidth::Md => "w-[480px]",
            DialogWidth::Lg => "w-[640px]",
        };
        let mut body = Vec::new();
        if let Some(message) = &self.message {
            body.push(text(message.as_str()).class(&themed(
                "text-sm leading-5 text-muted-foreground",
                "text-body text-secondary-label",
            )));
        }
        body.extend(self.content);
        let mut actions = self.buttons;
        let dismiss = self
            .cancel
            .as_ref()
            .map(|(_, msg)| msg.clone())
            .or(self.on_dismiss);
        if let Some((label, msg)) = self.cancel {
            actions.push(
                Button::new(label)
                    .cancel_button()
                    .on_click(msg)
                    .build()
                    .part("cancel"),
            );
        }
        if let Some((label, msg)) = self.default {
            actions.push(
                Button::new(label)
                    .default_button(true)
                    .destructive(self.destructive)
                    .on_click(msg)
                    .build()
                    .part("default"),
            );
        }
        let mut panel = div()
            .class(&style(
                &format!("flex flex-col {width} max-w-full border overflow-hidden"),
                "gap-4 p-6 rounded-lg border-border bg-background shadow-lg",
                "gap-3 p-5 rounded-[10px] border-separator bg-window shadow-lg",
            ))
            .role(Role::Dialog)
            .label(self.title.as_str())
            .part("panel")
            .children([
                text(self.title.as_str())
                    .class(&themed(
                        "text-lg leading-[1.4] font-semibold text-foreground",
                        "text-headline text-label",
                    ))
                    .role(Role::Heading)
                    .part("title"),
                div()
                    .class("flex flex-col gap-3")
                    .part("body")
                    .children(body),
                div()
                    .class("flex flex-wrap justify-end gap-2")
                    .part("actions")
                    .children(actions),
            ]);
        if let Some(dismiss) = dismiss {
            panel = panel.modal(dismiss);
        }
        let mut root = div()
            .class(&style(
                "absolute inset-0 z-50 flex justify-center items-start pt-24 px-4",
                "bg-foreground/30",
                "bg-black/20",
            ))
            .parts_root()
            .child(panel);
        root.test_id = self.test_id;
        root
    }
}

/// [`Dialog`] titled `title` showing `body` and `actions` (right-aligned,
/// in order). It sends no messages of its own and its panel is not
/// [`Element::modal`]: who shows it keeps the focus inside.
pub fn dialog<Msg: Clone + 'static>(
    title: &str,
    body: Element<Msg>,
    actions: impl IntoIterator<Item = Element<Msg>>,
) -> Element<Msg> {
    actions
        .into_iter()
        .fold(Dialog::new(title).content(body), Dialog::button)
        .build()
}

#[cfg(test)]
mod tests {
    use super::{Dialog, DialogWidth};
    use crate::components::{ButtonVariant, button};
    use crate::element::{Element, Role, div};
    use crate::runtime::SimpleApp;
    use crate::testing::{By, TestApp};
    use crate::ui_engine::{Key, Modifiers};

    #[derive(Default)]
    struct Files {
        open: bool,
        deleted: u32,
        dismissed: u32,
        name: String,
    }

    #[derive(Clone, Debug, PartialEq)]
    enum Msg {
        Ask,
        Delete,
        Cancel,
        Other,
        Name(String),
    }

    impl SimpleApp for Files {
        type Msg = Msg;
        fn title(&self) -> String {
            "Archivos".into()
        }
        fn update(&mut self, msg: Msg) {
            match msg {
                Msg::Ask => self.open = true,
                Msg::Delete => {
                    self.deleted += 1;
                    self.open = false;
                }
                Msg::Cancel => {
                    self.dismissed += 1;
                    self.open = false;
                }
                Msg::Other => {}
                Msg::Name(name) => self.name = name,
            }
        }
        fn view(&self) -> Element<Msg> {
            let mut root = div().class("relative flex flex-col gap-2 p-4 w-full h-full");
            root = root.children([
                button("Borrar…", ButtonVariant::Primary)
                    .on_click(Msg::Ask)
                    .test_id("borrar"),
                button("Otro", ButtonVariant::Outlined)
                    .on_click(Msg::Other)
                    .test_id("otro"),
            ]);
            if self.open {
                root = root.child(
                    Dialog::new("¿Borrar «notas.txt»?")
                        .message("No se puede deshacer.")
                        .content(
                            crate::components::TextField::new("Motivo", self.name.clone())
                                .on_input(Msg::Name)
                                .test_id("motivo")
                                .build(),
                        )
                        .cancel_button("Cancelar", Msg::Cancel)
                        .default_button("Borrar", Msg::Delete)
                        .destructive(true)
                        .width(DialogWidth::Sm)
                        .test_id("dialogo")
                        .build(),
                );
            }
            root
        }
    }

    #[test]
    fn a_dialog_traps_the_focus_and_gives_it_back() {
        let mut app = TestApp::new(Files::default());
        app.click(By::test_id("borrar"));
        assert!(app.app().open);
        app.screen()
            .get(By::role(Role::Dialog).name("¿Borrar «notas.txt»?"));
        assert!(app.is_focused(By::test_id("motivo")), "first control");

        app.press(Key::Tab, Modifiers::NONE);
        assert!(app.is_focused(By::test_id("dialogo.cancel")));
        app.press(Key::Tab, Modifiers::NONE);
        assert!(app.is_focused(By::test_id("dialogo.default")));
        app.press(Key::Tab, Modifiers::NONE);
        assert!(app.is_focused(By::test_id("motivo")), "Tab stays inside");

        let other = app.screen().get(By::test_id("otro")).bounds();
        app.click_at(crate::geometry::Point {
            x: other.x + other.width / 2.0,
            y: other.y + other.height / 2.0,
        });
        assert!(app.app().open, "clicks outside do nothing");
        assert!(app.is_focused(By::test_id("motivo")));
        assert!(!app.messages().contains(&Msg::Other));

        app.press(Key::Escape, Modifiers::NONE);
        assert_eq!(app.app().dismissed, 1);
        assert!(app.screen().query(By::role(Role::Dialog)).is_none());
        assert!(app.is_focused(By::test_id("borrar")), "focus comes back");
    }

    #[test]
    fn return_presses_the_default_button() {
        let mut app = TestApp::new(Files::default());
        app.click(By::test_id("borrar"));
        app.type_text(By::test_id("motivo"), "viejo");
        app.press(Key::Enter, Modifiers::NONE);
        assert_eq!(app.app().deleted, 1);
        assert!(!app.app().open);
        assert!(!app.session().modal_open());

        app.click(By::test_id("borrar"));
        assert!(app.session().modal_open());
        let default = app.screen().get(By::test_id("dialogo.default"));
        assert_eq!(default.role(), Role::Button);
        assert!(default.node().is_default);
    }

    #[test]
    fn without_a_cancel_button_escape_sends_on_dismiss() {
        struct Info(u32);
        impl SimpleApp for Info {
            type Msg = Msg;
            fn title(&self) -> String {
                "Info".into()
            }
            fn update(&mut self, msg: Msg) {
                if msg == Msg::Cancel {
                    self.0 += 1;
                }
            }
            fn view(&self) -> Element<Msg> {
                div().class("relative w-full h-full").child(
                    Dialog::new("Listo")
                        .message("Se guardó el archivo.")
                        .default_button("Aceptar", Msg::Other)
                        .on_dismiss(Msg::Cancel)
                        .build(),
                )
            }
        }
        let mut app = TestApp::new(Info(0));
        app.press(Key::Escape, Modifiers::NONE);
        assert_eq!(app.app().0, 1);
        let command = Modifiers {
            command: true,
            ..Modifiers::NONE
        };
        app.press(Key::Period, command);
        assert_eq!(app.app().0, 2);
    }
}
