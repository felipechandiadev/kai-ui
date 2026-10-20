//! Toasts: short notices stacked over the bottom right corner of the
//! window that close on their own (12 Paso 20.7), except errors. The app
//! keeps the list in its model and closes each one with [`toast_timer`],
//! which waits while the pointer or the focus is on the toast.

use std::time::Duration;

use super::{AlertVariant, BezelStyle, Button, alert_look, themed};
use crate::element::{ControlSize, Element, Role, div, icon, text};
use crate::icons::Icon;
use crate::runtime::Effects;

/// How long a toast stays before [`toast_timer`] closes it.
pub const TOAST_DURATION: Duration = Duration::from_secs(4);

/// Test id of [`toast_stack`]; each toast is `kai.aviso.<id>`, with parts
/// `title`, `message`, `action` and `close`.
pub const TOAST_STACK: &str = "kai.avisos";

/// One notice of a [`toast_stack`]. `id` tells it apart from the others
/// (keys, test ids, timers).
#[derive(Clone, Debug, PartialEq)]
pub struct Toast<Msg> {
    pub id: u64,
    pub variant: AlertVariant,
    pub title: String,
    pub message: Option<String>,
    /// Button label and its message («Deshacer», for instance).
    pub action: Option<(String, Msg)>,
}

impl<Msg> Toast<Msg> {
    pub fn new(id: u64, variant: AlertVariant, title: impl Into<String>) -> Self {
        Self {
            id,
            variant,
            title: title.into(),
            message: None,
            action: None,
        }
    }

    pub fn message(mut self, message: impl Into<String>) -> Self {
        self.message = Some(message.into());
        self
    }

    pub fn action(mut self, label: impl Into<String>, msg: Msg) -> Self {
        self.action = Some((label.into(), msg));
        self
    }
}

/// Timer key of toast `id`, to cancel its timer with [`Effects::cancel`].
pub fn toast_key(id: u64) -> String {
    format!("kai.aviso.{id}")
}

/// Send `close` after [`TOAST_DURATION`]: what `update` returns when it
/// adds `toast`. The timer waits while the pointer or the focus is on the
/// toast ([`Effects::keyed`]). An error has no timer: it stays until
/// closed, so nobody misses it (WCAG 2.2.1).
pub fn toast_timer<Msg>(toast: &Toast<Msg>, close: Msg) -> Effects<Msg> {
    if toast.variant == AlertVariant::Error {
        return Effects::none();
    }
    Effects::after(TOAST_DURATION, close).keyed(toast_key(toast.id))
}

/// The toasts, newest at the bottom, in a layer over the bottom right
/// corner of the nearest `relative` ancestor. Each one is tinted like the
/// [`alert`](super::alert) of its variant over an opaque panel; errors are
/// a [`Role::Alert`] (read at once), the rest a [`Role::Status`]. The close
/// button sends `on_close(id)`.
pub fn toast_stack<Msg: Clone + 'static>(
    toasts: &[Toast<Msg>],
    on_close: impl Fn(u64) -> Msg,
) -> Element<Msg> {
    div()
        .class("absolute right-4 bottom-4 z-40 flex flex-col gap-2 w-80 max-w-full")
        .test_id(TOAST_STACK)
        .children(
            toasts
                .iter()
                .map(|toast| toast_view(toast, on_close(toast.id))),
        )
}

fn toast_view<Msg: Clone + 'static>(toast: &Toast<Msg>, close: Msg) -> Element<Msg> {
    let (icon_kind, tint, icon_color) = alert_look(toast.variant);
    let role = if toast.variant == AlertVariant::Error {
        Role::Alert
    } else {
        Role::Status
    };
    let mut lines = vec![
        text(toast.title.as_str())
            .class(&themed(
                "text-sm leading-5 font-medium text-foreground",
                "text-body font-semibold text-label",
            ))
            .part("title"),
    ];
    if let Some(message) = &toast.message {
        lines.push(
            text(message.as_str())
                .class(&themed(
                    "text-sm leading-5 text-muted-foreground",
                    "text-callout text-secondary-label",
                ))
                .part("message"),
        );
    }
    if let Some((label, msg)) = &toast.action {
        lines.push(
            div().class(&themed("flex -ml-3", "flex pt-1 -ml-1")).child(
                Button::new(label.as_str())
                    .bezel(BezelStyle::Borderless)
                    .control_size(ControlSize::Small)
                    .on_click(msg.clone())
                    .build()
                    .part("action"),
            ),
        );
    }
    let close = div()
        .class(&themed(
            "inline-flex size-6 shrink-0 -mr-1 -mt-0.5 items-center justify-center rounded-md \
             text-muted-foreground cursor-pointer hover:bg-primary/10 hover:text-foreground",
            "inline-flex size-[18px] shrink-0 items-center justify-center rounded-full \
             text-secondary-label cursor-default hover:bg-fill-2",
        ))
        .role(Role::Button)
        .label("Cerrar aviso")
        .focus_on_press(false)
        .on_click(close)
        .part("close")
        .child(icon(Icon::X).class("size-4"));
    let body = div()
        .class(&format!(
            "flex items-start gap-3 w-full p-4 {tint} {}",
            themed("rounded-lg", "rounded-[10px]")
        ))
        .children([
            icon(icon_kind).class(&format!("size-5 shrink-0 {icon_color}")),
            div()
                .class("flex flex-col flex-1 min-w-0 gap-0.5")
                .children(lines),
            close,
        ]);
    div()
        .class(&themed(
            "w-full rounded-lg bg-background shadow-lg",
            "w-full rounded-[10px] bg-window shadow-lg",
        ))
        .role(role)
        .label(toast.title.as_str())
        .key(toast.id.to_string())
        .test_id(toast_key(toast.id))
        .parts_root()
        .child(body)
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::{TOAST_DURATION, Toast, toast_key, toast_stack, toast_timer};
    use crate::components::{AlertVariant, ButtonVariant, button};
    use crate::element::{Element, Role, div};
    use crate::runtime::{App, Effects, HELD_TIMER_RESUME};
    use crate::testing::{By, TestApp};
    use crate::ui_engine::{Key, Modifiers};

    #[derive(Default)]
    struct Notices {
        toasts: Vec<Toast<Msg>>,
        next: u64,
        undone: u32,
    }

    #[derive(Clone, Debug, PartialEq)]
    enum Msg {
        Show(AlertVariant),
        Close(u64),
        Snooze(u64),
        Undo,
    }

    impl App for Notices {
        type Msg = Msg;
        fn title(&self) -> String {
            "Avisos".into()
        }
        fn update(&mut self, msg: Msg) -> Effects<Msg> {
            match msg {
                Msg::Show(variant) => {
                    self.next += 1;
                    let id = self.next;
                    let title = if variant == AlertVariant::Error {
                        "No se pudo guardar"
                    } else {
                        "Guardado"
                    };
                    let toast = Toast::new(id, variant, title)
                        .message("notas.txt")
                        .action("Deshacer", Msg::Undo);
                    let timer = toast_timer(&toast, Msg::Close(id));
                    self.toasts.push(toast);
                    timer
                }
                Msg::Close(id) => {
                    self.toasts.retain(|toast| toast.id != id);
                    Effects::cancel(toast_key(id))
                }
                Msg::Snooze(id) => self
                    .toasts
                    .iter()
                    .find(|toast| toast.id == id)
                    .map_or_else(Effects::none, |toast| toast_timer(toast, Msg::Close(id))),
                Msg::Undo => {
                    self.undone += 1;
                    Effects::none()
                }
            }
        }
        fn view(&self) -> Element<Msg> {
            div().class("relative w-full h-full p-4").children([
                button("Guardar", ButtonVariant::Primary)
                    .on_click(Msg::Show(AlertVariant::Success))
                    .test_id("guardar"),
                button("Fallar", ButtonVariant::Outlined)
                    .on_click(Msg::Show(AlertVariant::Error))
                    .test_id("fallar"),
                button("Posponer", ButtonVariant::Outlined)
                    .on_click(Msg::Snooze(self.next))
                    .test_id("posponer"),
                toast_stack(&self.toasts, Msg::Close),
            ])
        }
    }

    fn closes(app: &TestApp<Notices>) -> usize {
        app.messages()
            .iter()
            .filter(|msg| matches!(msg, Msg::Close(_)))
            .count()
    }

    #[test]
    fn toasts_close_on_their_own_after_four_seconds_but_errors_stay() {
        let mut app = TestApp::new(Notices::default());
        app.click(By::test_id("guardar"));
        let saved = app.screen().get(By::role(Role::Status).name("Guardado"));
        assert_eq!(saved.test_id(), Some("kai.aviso.1"));

        app.advance(Duration::from_secs(3));
        app.click(By::test_id("fallar"));
        app.screen()
            .get(By::role(Role::Alert).name("No se pudo guardar"));

        app.advance(Duration::from_secs(1));
        assert!(app.screen().query(By::test_id("kai.aviso.1")).is_none());
        assert!(app.screen().query(By::test_id("kai.aviso.2")).is_some());

        app.advance(TOAST_DURATION * 3);
        assert!(
            app.screen().query(By::test_id("kai.aviso.2")).is_some(),
            "an error waits to be closed"
        );
        assert_eq!(closes(&app), 1);
        app.click(By::test_id("kai.aviso.2.close"));
        assert!(app.app().toasts.is_empty());
    }

    #[test]
    fn the_pointer_on_a_toast_holds_it_open() {
        let mut app = TestApp::new(Notices::default());
        app.click(By::test_id("guardar"));
        app.hover(By::test_id("kai.aviso.1.message"));
        app.advance(TOAST_DURATION * 3);
        assert_eq!(app.app().toasts.len(), 1, "held while the pointer is on it");

        app.hover(By::test_id("guardar"));
        app.advance(HELD_TIMER_RESUME - Duration::from_millis(1));
        assert_eq!(app.app().toasts.len(), 1, "time to move back to it");
        app.advance(Duration::from_millis(1));
        assert!(app.app().toasts.is_empty());
        assert_eq!(closes(&app), 1);
    }

    #[test]
    fn the_focus_on_a_toast_holds_it_open() {
        let mut app = TestApp::new(Notices::default());
        app.click(By::test_id("guardar"));
        let action = By::test_id("kai.aviso.1.action");
        for _ in 0..6 {
            if app.is_focused(action.clone()) {
                break;
            }
            app.press(Key::Tab, Modifiers::NONE);
        }
        assert!(app.is_focused(action), "Tab reaches the toast's button");
        app.advance(TOAST_DURATION * 2);
        assert_eq!(app.app().toasts.len(), 1);

        let back = Modifiers {
            shift: true,
            ..Modifiers::NONE
        };
        app.press(Key::Tab, back);
        assert!(app.is_focused(By::test_id("posponer")));
        app.advance(HELD_TIMER_RESUME);
        assert!(app.app().toasts.is_empty(), "closes once the focus leaves");
    }

    #[test]
    fn the_close_button_cancels_the_timer_and_the_action_runs() {
        let mut app = TestApp::new(Notices::default());
        app.click(By::test_id("guardar"));
        app.click(By::test_id("kai.aviso.1.action"));
        assert_eq!(app.app().undone, 1);
        app.click(By::test_id("kai.aviso.1.close"));
        assert!(app.app().toasts.is_empty());
        app.advance(TOAST_DURATION * 2);
        assert_eq!(closes(&app), 1, "the timer was cancelled");
    }

    /// `cargo test toast_capture -- --ignored` writes
    /// `target/snapshots/toasts-<theme>.png` (needs a GPU).
    #[test]
    #[ignore]
    fn toast_capture() {
        use crate::theme::{AppKitGeneration, Appearance, Density, Theme};
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("target/snapshots");
        std::fs::create_dir_all(&dir).unwrap();
        let themes = [
            ("kai", Theme::kai(Density::Comfortable)),
            (
                "appkit",
                Theme::appkit_static(AppKitGeneration::Glass27, Appearance::Light),
            ),
        ];
        for (name, theme) in themes {
            let mut app = TestApp::builder(Notices::default()).theme(theme).build();
            app.click(By::test_id("guardar"));
            app.click(By::test_id("fallar"));
            let image = app.capture(By::test_id(super::TOAST_STACK)).unwrap();
            crate::snapshot::write_png(&dir.join(format!("toasts-{name}.png")), &image).unwrap();
        }
    }

    #[test]
    fn a_keyed_timer_replaces_the_pending_one() {
        let mut app = TestApp::new(Notices::default());
        app.click(By::test_id("guardar"));
        app.advance(Duration::from_secs(3));
        app.click(By::test_id("posponer"));
        app.advance(Duration::from_secs(3));
        assert_eq!(app.app().toasts.len(), 1, "restarted at 3 s");
        app.advance(Duration::from_secs(1));
        assert!(app.app().toasts.is_empty());
        assert_eq!(closes(&app), 1);
    }
}
