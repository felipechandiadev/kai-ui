//! [`TestApp`]: an app driven like a user would (clicks, typing, keys)
//! through the same session the window runtime uses, without a window,
//! fonts in layout or GPU.

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::mpsc;

use serde_json::Value;

use super::{By, Found, Screen, VIEWPORT};
use crate::commands::{Action, AuditLog, Availability, CommandError, CommandState, Query};
use crate::element::WidgetId;
use crate::geometry::{Point, Size};
use crate::platform_adapter::adapt_key_input;
use crate::runtime::App;
use crate::session::{AgentCall, CONFIRMATION, Measure, Session};
use crate::snapshot::{self, Image, Shot};
use crate::text_field::ClipboardPort;
use crate::theme::{Density, Theme};
use crate::ui_engine::{Intent, Key, Modifiers, TextIntent};

/// In-memory clipboard. Clones share the contents, so a test can keep one
/// and hand another to [`TestAppBuilder::clipboard`].
#[derive(Clone, Debug, Default)]
pub struct MemoryClipboard(Rc<RefCell<Option<String>>>);

impl MemoryClipboard {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn text(&self) -> Option<String> {
        self.0.borrow().clone()
    }

    pub fn set(&self, text: impl Into<String>) {
        *self.0.borrow_mut() = Some(text.into());
    }
}

impl ClipboardPort for MemoryClipboard {
    fn get_text(&mut self) -> Result<String, String> {
        self.text().ok_or_else(|| "portapapeles vacío".to_owned())
    }

    fn set_text(&mut self, text: String) -> Result<(), String> {
        self.set(text);
        Ok(())
    }
}

/// Options for a [`TestApp`]: viewport, theme and clipboard.
pub struct TestAppBuilder<A: App> {
    app: A,
    viewport: Size,
    theme: Theme,
    clipboard: MemoryClipboard,
}

impl<A: App> TestAppBuilder<A> {
    /// Window content size in logical points (default [`VIEWPORT`]).
    pub fn viewport(mut self, width: f32, height: f32) -> Self {
        self.viewport = Size { width, height };
        self
    }

    /// Theme before the app's own [`App::theme`] (default Kai).
    pub fn theme(mut self, theme: Theme) -> Self {
        self.theme = theme;
        self
    }

    pub fn clipboard(mut self, clipboard: MemoryClipboard) -> Self {
        self.clipboard = clipboard;
        self
    }

    /// Run `init` and the first frame.
    pub fn build(self) -> TestApp<A> {
        let mut session = Session::new(
            self.app,
            self.theme,
            self.viewport,
            Measure::Fixed,
            Box::new(self.clipboard.clone()),
        );
        session.record_messages();
        let mut app = TestApp {
            session,
            clipboard: self.clipboard,
            confirm: Confirm::Ask,
            next_agent_call: 1,
        };
        app.frame();
        app
    }
}

/// An app under test. Every action goes through the same event handling
/// as the window (pointer, keyboard adapter, focus engine, input editors)
/// and then prepares a frame, so queries always see the current view.
/// Layout measures text with `FixedMeasure`, like [`super::layout_of`].
///
/// ```
/// use kai::components::{ButtonVariant, button};
/// use kai::element::{Element, Role, div, text};
/// use kai::runtime::SimpleApp;
/// use kai::testing::{By, TestApp};
///
/// #[derive(Default)]
/// struct Counter(u32);
///
/// impl SimpleApp for Counter {
///     type Msg = ();
///     fn title(&self) -> String {
///         "Contador".into()
///     }
///     fn update(&mut self, (): ()) {
///         self.0 += 1;
///     }
///     fn view(&self) -> Element<()> {
///         div().children([
///             text(self.0.to_string()).role(Role::Status),
///             button("Sumar", ButtonVariant::Primary).on_click(()),
///         ])
///     }
/// }
///
/// let mut app = TestApp::new(Counter::default());
/// app.click(By::role(Role::Button).name("Sumar"));
/// assert_eq!(app.messages(), [()]);
/// app.screen().get(By::role(Role::Status).name("1"));
/// ```
pub struct TestApp<A: App> {
    session: Session<A>,
    clipboard: MemoryClipboard,
    confirm: Confirm,
    next_agent_call: u64,
}

/// What the person does when an agent's call opens the confirmation
/// dialog in a [`TestApp`]: the test app clicks that button.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Confirm {
    /// Leave the dialog open; the test answers it.
    #[default]
    Ask,
    AllowOnce,
    AllowForSession,
    Deny,
}

/// The answer to [`TestApp::agent_call`], once there is one.
#[derive(Debug)]
pub struct AgentReply {
    id: u64,
    answer: mpsc::Receiver<Result<Value, CommandError>>,
    result: RefCell<Option<Result<Value, CommandError>>>,
}

impl AgentReply {
    /// Id to cancel the call with ([`TestApp::cancel_agent_call`]).
    pub fn id(&self) -> u64 {
        self.id
    }

    /// The result, or `None` while the call waits (for confirmation or
    /// behind another one).
    pub fn result(&self) -> Option<Result<Value, CommandError>> {
        let mut result = self.result.borrow_mut();
        if result.is_none() {
            *result = self.answer.try_recv().ok();
        }
        result.clone()
    }
}

impl<A: App> TestApp<A> {
    /// [`VIEWPORT`] window, Kai theme, empty clipboard.
    pub fn new(app: A) -> Self {
        Self::builder(app).build()
    }

    pub fn builder(app: A) -> TestAppBuilder<A> {
        TestAppBuilder {
            app,
            viewport: VIEWPORT,
            theme: Theme::kai(Density::Comfortable),
            clipboard: MemoryClipboard::new(),
        }
    }

    pub fn app(&self) -> &A {
        self.session.app()
    }

    #[cfg(test)]
    pub(crate) fn session(&self) -> &Session<A> {
        &self.session
    }

    /// Run an action as a script or an agent would, without the UI: its
    /// message goes through `update` if the action is registered and
    /// available; otherwise the error says why not (the reason of a
    /// `Disabled` action, for instance).
    ///
    /// ```
    /// use kai::commands::{Action, Availability, CommandRegistry};
    /// use kai::element::{Element, div};
    /// use kai::runtime::SimpleApp;
    /// use kai::testing::TestApp;
    ///
    /// #[derive(Default)]
    /// struct Counter(u32);
    ///
    /// #[derive(Clone, Debug)]
    /// enum Msg {
    ///     Increment,
    ///     Reset,
    /// }
    ///
    /// #[derive(Clone, serde::Deserialize, schemars::JsonSchema)]
    /// struct Increment;
    ///
    /// impl Action for Increment {
    ///     type App = Counter;
    ///     const ID: &'static str = "contador.incrementar";
    ///     const TITLE: &'static str = "Incrementar";
    ///     fn into_msg(self) -> Msg {
    ///         Msg::Increment
    ///     }
    /// }
    ///
    /// #[derive(Clone, serde::Deserialize, schemars::JsonSchema)]
    /// struct Reset;
    ///
    /// impl Action for Reset {
    ///     type App = Counter;
    ///     const ID: &'static str = "contador.reiniciar";
    ///     const TITLE: &'static str = "Reiniciar";
    ///     fn available(counter: &Counter) -> Availability {
    ///         if counter.0 == 0 {
    ///             Availability::disabled("Ya está en cero")
    ///         } else {
    ///             Availability::Enabled
    ///         }
    ///     }
    ///     fn into_msg(self) -> Msg {
    ///         Msg::Reset
    ///     }
    /// }
    ///
    /// impl SimpleApp for Counter {
    ///     type Msg = Msg;
    ///     fn commands(registry: &mut CommandRegistry<Self>) {
    ///         registry.action::<Increment>();
    ///         registry.action::<Reset>();
    ///     }
    ///     fn title(&self) -> String {
    ///         "Contador".into()
    ///     }
    ///     fn update(&mut self, msg: Msg) {
    ///         match msg {
    ///             Msg::Increment => self.0 += 1,
    ///             Msg::Reset => self.0 = 0,
    ///         }
    ///     }
    ///     fn view(&self) -> Element<Msg> {
    ///         div()
    ///     }
    /// }
    ///
    /// let mut app = TestApp::new(Counter::default());
    /// let error = app.invoke(Reset).unwrap_err();
    /// assert_eq!(error.message, "Ya está en cero");
    /// app.invoke(Increment).unwrap();
    /// assert_eq!(app.available::<Reset>(), Availability::Enabled);
    /// app.invoke(Reset).unwrap();
    /// assert_eq!(app.app().0, 0);
    /// ```
    pub fn invoke<C: Action<App = A>>(&mut self, action: C) -> Result<(), CommandError> {
        self.session.send_command(C::ID, action.into_msg())?;
        self.frame();
        Ok(())
    }

    /// Run a registered query against the current model.
    pub fn query<Q: Query<App = A>>(&self, input: Q) -> Result<Q::Output, CommandError> {
        if !self.session.has_query(Q::ID) {
            return Err(CommandError::new(format!(
                "consulta no registrada: {}",
                Q::ID
            )));
        }
        Q::run(self.session.app(), input)
    }

    /// Availability of a registered action after the last `update`.
    #[track_caller]
    pub fn available<C: Action<App = A>>(&self) -> Availability {
        match self.session.command_state(C::ID) {
            Some(state) => state.availability.clone(),
            None => panic!("acción no registrada: {}", C::ID),
        }
    }

    /// Run command `id` as its menu bar item does. Returns whether
    /// something ran (a disabled action does not).
    pub fn perform(&mut self, id: &str) -> bool {
        let ran = self.session.perform_command(id);
        self.frame();
        ran
    }

    /// Every registered command with its availability after the last
    /// `update`, in registration order.
    pub fn commands(&self) -> &[CommandState] {
        self.session.commands()
    }

    /// Run command `id` on a JSON `input`, as a script does: a query
    /// returns its output, an action returns `null` (or the output of its
    /// `returns` query) once its message went through `update`. Without
    /// parameters the input is `{}`. Actions land in the [`audit_log`]
    /// as [`Origin::Script`]; nothing asks for confirmation.
    ///
    /// [`audit_log`]: Self::audit_log
    /// [`Origin::Script`]: crate::commands::Origin::Script
    pub fn call(&mut self, id: &str, input: Value) -> Result<Value, CommandError> {
        let output = self.session.call(id, input)?;
        self.frame();
        Ok(output)
    }

    /// Run command `id` as agent `client` would over MCP: only exposed
    /// commands, and a `Destructive` action opens the confirmation dialog
    /// (`kai.confirmar`). What happens then follows [`on_confirmation`];
    /// with [`Confirm::Ask`] the dialog stays open for the test to click.
    /// The reply arrives when the call ran, was rejected or cancelled.
    ///
    /// [`on_confirmation`]: Self::on_confirmation
    pub fn agent_call(&mut self, client: &str, id: &str, input: Value) -> AgentReply {
        let call_id = self.next_agent_call;
        self.next_agent_call += 1;
        let (sender, answer) = mpsc::channel();
        self.run_agent_call(AgentCall {
            id: call_id,
            client: client.to_owned(),
            command: id.to_owned(),
            input,
            reply: Box::new(move |result| {
                let _ = sender.send(result);
            }),
        });
        AgentReply {
            id: call_id,
            answer,
            result: RefCell::new(None),
        }
    }

    pub(crate) fn run_agent_call(&mut self, call: AgentCall) {
        self.session.agent_call(call);
        self.frame();
        self.answer_confirmations();
    }

    /// The agent stopped waiting for call `id` (time limit or
    /// cancellation): its dialog closes, or it leaves the queue.
    pub fn cancel_agent_call(&mut self, id: u64) {
        self.session.cancel_agent_call(id);
        self.frame();
        self.answer_confirmations();
    }

    /// What the person does when the confirmation dialog opens from now
    /// on; [`Confirm::Ask`] (the default) leaves it open.
    pub fn on_confirmation(&mut self, confirm: Confirm) {
        self.confirm = confirm;
        self.answer_confirmations();
    }

    /// An agent called `client` connected: the indicator `kai.agente`
    /// appears.
    pub fn agent_connected(&mut self, client: &str) {
        self.session.agent_connected(client);
        self.frame();
    }

    /// Actions run through JSON so far, by scripts and agents.
    pub fn audit_log(&self) -> &AuditLog {
        self.session.audit_log()
    }

    /// Click the button of the [`Confirm`] policy while a dialog is open.
    fn answer_confirmations(&mut self) {
        let part = match self.confirm {
            Confirm::Ask => return,
            Confirm::AllowOnce => "una_vez",
            Confirm::AllowForSession => "sesion",
            Confirm::Deny => "rechazar",
        };
        while self.session.confirmation_open() {
            let recorded = self.audit_log().len();
            self.click(By::test_id(format!("{CONFIRMATION}.{part}")));
            assert!(
                self.audit_log().len() > recorded,
                "el diálogo de confirmación no respondió al clic"
            );
        }
    }

    /// Press and release the primary button at `point`, whatever is there.
    #[cfg(test)]
    pub(crate) fn click_at(&mut self, point: crate::geometry::Point) {
        self.session.pointer_moved(Some(point));
        self.frame();
        self.session.pointer_button(true);
        self.session.pointer_button(false);
        self.frame();
    }

    /// Theme in use (the app's own when it asks for one).
    pub fn theme(&self) -> &Theme {
        self.session.theme()
    }

    pub fn clipboard(&self) -> &MemoryClipboard {
        &self.clipboard
    }

    /// Queries over the current layout.
    pub fn screen(&self) -> Screen<'_> {
        Screen::new(self.session.layout())
    }

    /// Every message sent to `update`, in order.
    pub fn messages(&self) -> &[A::Msg] {
        self.session.messages()
    }

    /// Scroll the element into view, move the pointer to its center, press
    /// and release. Panics when something else is on top of that point. A
    /// disabled control takes the click and sends nothing, as on screen.
    #[track_caller]
    pub fn click(&mut self, by: By) {
        let id = self.id_of(by);
        if self.session.reveal(id) {
            self.frame();
        }
        let center = self.center_of(id);
        let path = self.session.layout().hit_path(center);
        if !path.contains(&id) {
            let screen = self.screen();
            let describe = |id: Option<&WidgetId>| {
                id.and_then(|id| screen.by_id(*id))
                    .map_or("nada".to_owned(), |found| found.describe())
            };
            panic!(
                "no se puede hacer clic en {}: otro elemento tapa su centro ({:.1}, {:.1}): {}",
                describe(Some(&id)),
                center.x,
                center.y,
                describe(path.last())
            );
        }
        self.session.pointer_moved(Some(center));
        self.frame();
        self.session.pointer_button(true);
        self.session.pointer_button(false);
        self.frame();
    }

    /// Click the input, then type `text` one character at a time, each
    /// followed by a frame (the app sees one message per character).
    #[track_caller]
    pub fn type_text(&mut self, by: By, text: &str) {
        self.click(by.clone());
        let focused = self.session.focused();
        if focused.is_none_or(|id| self.session.field_value(id).is_none()) {
            panic!("{by} no recibió el foco de texto tras el clic");
        }
        for character in text.chars() {
            self.session
                .apply_intent(Intent::Text(TextIntent::InsertText(character.to_string())));
            self.frame();
        }
    }

    /// A key press as the keyboard adapter sends it: the key, plus the text
    /// it types for character keys without ⌘ or ⌃.
    pub fn press(&mut self, key: Key, modifiers: Modifiers) {
        let text = typed_text(key, modifiers);
        for intent in adapt_key_input(key, modifiers, false, text.as_deref()) {
            self.session.apply_intent(intent);
        }
        self.frame();
    }

    /// Contents of a text input (what its editor holds).
    #[track_caller]
    pub fn value(&self, by: By) -> String {
        let screen = self.screen();
        let found = self.control_of(screen.get(by));
        self.session
            .field_value(found.id())
            .unwrap_or_else(|| panic!("{} no es un campo de texto", found.describe()))
    }

    #[track_caller]
    pub fn is_focused(&self, by: By) -> bool {
        let screen = self.screen();
        self.session.focused() == Some(self.control_of(screen.get(by)).id())
    }

    /// Neither the element nor an ancestor is disabled.
    #[track_caller]
    pub fn is_enabled(&self, by: By) -> bool {
        let screen = self.screen();
        let mut node = Some(self.control_of(screen.get(by)));
        while let Some(found) = node {
            if found.node().disabled {
                return false;
            }
            node = found.parent();
        }
        true
    }

    /// Whether focus shows its ring (focus moved by keyboard).
    pub fn focus_visible(&self) -> bool {
        *self.session.ui().focus_visible.get()
    }

    /// GPU render of the current state with real text, cropped to the
    /// element (2x backing scale).
    #[track_caller]
    pub fn capture(&self, by: By) -> Result<Image, String> {
        let id = self.id_of(by);
        let interaction = self.session.interaction();
        snapshot::render_element(
            &Shot {
                tree: self.session.tree(),
                theme: self.session.theme(),
                logical: self.session.viewport(),
                scale: 2.0,
                interaction: &interaction,
                scroll: self.session.scroll(),
            },
            id,
        )
    }

    #[track_caller]
    fn id_of(&self, by: By) -> WidgetId {
        self.screen().get(by).id()
    }

    /// The input a field root stands for (the one a click focuses, or its
    /// `<id>.input` part when disabled); any other element, itself.
    fn control_of<'a>(&'a self, found: Found<'a>) -> Found<'a> {
        let screen = Screen::new(self.session.layout());
        if let Some(delegate) = found.node().focus_delegate {
            return screen.by_id(delegate).unwrap_or(found);
        }
        found
            .test_id()
            .and_then(|id| {
                screen
                    .within(found)
                    .query(By::test_id(format!("{id}.input")))
            })
            .unwrap_or(found)
    }

    fn center_of(&self, id: WidgetId) -> Point {
        let bounds = self.session.layout().bounds(id).unwrap_or_default();
        Point {
            x: bounds.x + bounds.width / 2.0,
            y: bounds.y + bounds.height / 2.0,
        }
    }

    /// What the window does on `RedrawRequested`, minus paint.
    fn frame(&mut self) {
        self.session.prepare_frame();
        let _ = self.session.take_outbox();
    }
}

fn typed_text(key: Key, modifiers: Modifiers) -> Option<String> {
    let character = match key {
        Key::Space => ' ',
        Key::A => 'a',
        Key::C => 'c',
        Key::X => 'x',
        Key::V => 'v',
        Key::Char(character) => character,
        _ => return None,
    };
    Some(if modifiers.shift {
        character.to_uppercase().collect()
    } else {
        character.to_string()
    })
}

#[cfg(test)]
mod tests {
    use super::{MemoryClipboard, TestApp};
    use crate::components::{ButtonVariant, TextField, button};
    use crate::element::sample::{self, Msg};
    use crate::element::{Element, Role, div, text};
    use crate::runtime::SimpleApp;
    use crate::testing::By;
    use crate::ui_engine::{Key, Modifiers};

    struct Sample;

    impl SimpleApp for Sample {
        type Msg = Msg;

        fn title(&self) -> String {
            "Sample".to_owned()
        }

        fn update(&mut self, _msg: Msg) {}

        fn view(&self) -> Element<Msg> {
            sample::tree()
        }
    }

    #[derive(Default)]
    struct Form {
        name: String,
        sent: Vec<String>,
    }

    #[derive(Clone, Debug, PartialEq, Eq)]
    enum FormMsg {
        Name(String),
        Send,
    }

    impl SimpleApp for Form {
        type Msg = FormMsg;

        fn title(&self) -> String {
            "Form".to_owned()
        }

        fn update(&mut self, msg: FormMsg) {
            match msg {
                FormMsg::Name(name) => self.name = name,
                FormMsg::Send => self.sent.push(std::mem::take(&mut self.name)),
            }
        }

        fn view(&self) -> Element<FormMsg> {
            div()
                .class("w-full h-full flex flex-col gap-4 p-4")
                .children([
                    TextField::new("Nombre", &self.name)
                        .test_id("nombre")
                        .on_input(FormMsg::Name)
                        .build(),
                    button("Enviar", ButtonVariant::Primary)
                        .test_id("enviar")
                        .on_click(FormMsg::Send)
                        .default_button(true)
                        .disabled(self.name.is_empty()),
                    text(format!("Enviados: {}", self.sent.len())).role(Role::Status),
                ])
        }
    }

    fn cmd(key: Key) -> (Key, Modifiers) {
        (
            key,
            Modifiers {
                command: true,
                ..Modifiers::NONE
            },
        )
    }

    #[test]
    fn clicks_reach_the_enclosing_control_and_skip_disabled_ones() {
        let mut app = TestApp::new(Sample);
        app.click(By::text("Probar interacción"));
        assert_eq!(app.messages(), [Msg::Clicked]);
        assert!(app.is_enabled(By::text("Probar interacción")));

        assert!(!app.is_enabled(By::text("No disponible")));
        app.click(By::text("No disponible"));
        assert_eq!(app.messages(), [Msg::Clicked]);
    }

    #[test]
    fn typing_sends_one_message_per_character() {
        let mut app = TestApp::new(Form::default());
        assert!(!app.is_enabled(By::test_id("enviar")));
        app.type_text(By::label("Nombre"), "Ana");
        assert!(app.is_focused(By::test_id("nombre")));
        assert_eq!(app.value(By::label("Nombre")), "Ana");
        assert_eq!(
            app.messages(),
            ["A", "An", "Ana"].map(|name| FormMsg::Name(name.to_owned()))
        );
        assert!(app.is_enabled(By::test_id("enviar")));

        app.click(By::test_id("enviar"));
        assert_eq!(app.messages().last(), Some(&FormMsg::Send));
        assert_eq!(app.app().sent, ["Ana"]);
        assert_eq!(app.value(By::test_id("nombre")), "");
        app.screen().get(By::text("Enviados: 1"));
    }

    #[test]
    fn return_in_a_field_activates_the_default_button() {
        let mut app = TestApp::new(Form::default());
        app.type_text(By::test_id("nombre"), "Ana");
        app.press(Key::Enter, Modifiers::NONE);
        assert_eq!(app.messages().last(), Some(&FormMsg::Send));
        assert_eq!(app.app().sent, ["Ana"]);
    }

    #[test]
    fn copy_and_paste_go_through_the_memory_clipboard() {
        let clipboard = MemoryClipboard::new();
        let mut app = TestApp::builder(Form::default())
            .clipboard(clipboard.clone())
            .build();
        app.type_text(By::test_id("nombre"), "Ana");
        let (key, modifiers) = cmd(Key::A);
        app.press(key, modifiers);
        let (key, modifiers) = cmd(Key::C);
        app.press(key, modifiers);
        assert_eq!(clipboard.text().as_deref(), Some("Ana"));

        app.press(Key::End, Modifiers::NONE);
        let (key, modifiers) = cmd(Key::V);
        app.press(key, modifiers);
        assert_eq!(app.value(By::test_id("nombre")), "AnaAna");
        assert_eq!(app.app().name, "AnaAna");
    }

    #[test]
    fn keys_type_their_character() {
        let mut app = TestApp::new(Form::default());
        app.click(By::test_id("nombre"));
        let shift = Modifiers {
            shift: true,
            ..Modifiers::NONE
        };
        app.press(Key::Char('o'), shift);
        app.press(Key::Char('k'), Modifiers::NONE);
        assert_eq!(app.value(By::test_id("nombre")), "Ok");
    }

    struct Covered;

    impl SimpleApp for Covered {
        type Msg = Msg;

        fn title(&self) -> String {
            "Covered".to_owned()
        }

        fn update(&mut self, _msg: Msg) {}

        fn view(&self) -> Element<Msg> {
            div().class("relative w-60 h-20").children([
                button("Debajo", ButtonVariant::Primary).on_click(Msg::Clicked),
                div().class("absolute inset-0 bg-neutral").test_id("velo"),
            ])
        }
    }

    #[test]
    #[should_panic(expected = "otro elemento tapa su centro")]
    fn clicking_a_covered_element_fails_clearly() {
        let mut app = TestApp::new(Covered);
        app.click(By::role(Role::Button).name("Debajo"));
    }

    #[test]
    #[ignore = "GPU"]
    fn capture_element_crops_to_bounds() {
        let app = TestApp::new(Form::default());
        let bounds = app.screen().get(By::test_id("enviar")).bounds();
        let image = app.capture(By::test_id("enviar")).unwrap();
        assert_eq!(image.height, (bounds.height * 2.0).ceil() as u32);
        assert!(image.width > 0 && image.width < 2048);
        assert_eq!(image.rgba.len(), (image.width * image.height * 4) as usize);
    }
}
