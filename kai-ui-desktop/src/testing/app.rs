//! [`TestApp`]: an app driven like a user would (clicks, typing, keys)
//! through the same session the window runtime uses, without a window,
//! fonts in layout or GPU.

use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::{Arc, mpsc};
use std::time::{Duration, Instant};

use serde_json::Value;

use super::{By, Found, Screen, VIEWPORT};
use crate::commands::{Action, AuditLog, Availability, CommandError, CommandState, Query};
use crate::devtools::MsgSource;
use crate::element::WidgetId;
use crate::geometry::{Point, Size};
use crate::media::{FakeImageLoader, ImageLoader};
use crate::platform_adapter::adapt_key_input;
use crate::runtime::{App, LINE_SCROLL, PickFile, SaveFile};
use crate::session::{AgentCall, CONFIRMATION, Measure, Session};
use crate::snapshot::{self, Image, Shot};
use crate::style::Cursor;
use crate::text_field::ClipboardPort;
use crate::theme::{Density, SystemPrefs, Theme};
use crate::ui_engine::{Intent, Key, Modifiers, TextIntent};

/// Pointer moves in a [`TestApp::drag`], one every [`DRAG_STEP`].
pub const DRAG_STEPS: u32 = 8;
pub const DRAG_STEP: Duration = Duration::from_millis(16);

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
    images: Arc<dyn ImageLoader>,
}

impl<A: App> TestAppBuilder<A> {
    /// Where images come from (default: an empty [`FakeImageLoader`], which
    /// reads files and fails every URL, never touching the network).
    pub fn images(mut self, loader: impl ImageLoader + 'static) -> Self {
        self.images = Arc::new(loader);
        self
    }

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
        // Animations start at time zero: frames are the same on every run.
        session.set_clock(session.epoch());
        session.set_image_loader(self.images);
        let mut app = TestApp {
            session,
            clipboard: self.clipboard,
            confirm: Confirm::Ask,
            next_agent_call: 1,
            file_pick_answer: None,
            file_pick_request: None,
            file_save_answer: None,
            file_save_request: None,
            cursor: Cursor::Default,
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
    /// Answer for the open panel ([`TestApp::on_file_pick`]).
    file_pick_answer: Option<Option<PathBuf>>,
    file_pick_request: Option<PickFile>,
    /// Answer for the save panel ([`TestApp::on_file_save`]).
    file_save_answer: Option<Option<PathBuf>>,
    file_save_request: Option<SaveFile>,
    /// The last pointer shape the session asked the window for.
    cursor: Cursor,
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
            images: Arc::new(FakeImageLoader::new()),
        }
    }

    pub fn app(&self) -> &A {
        self.session.app()
    }

    #[cfg(test)]
    pub(crate) fn session(&self) -> &Session<A> {
        &self.session
    }

    /// Pretend the Mac changed its accessibility preferences, as the
    /// runtime reads them when the window becomes active.
    pub fn set_system_prefs(&mut self, prefs: SystemPrefs) {
        self.session.set_system_prefs(prefs);
        self.frame();
    }

    /// Let `by` pass on the test app's clock, which only moves here: the
    /// timers due in that span ([`Effects::after`]) send their messages in
    /// order, each at its own time, with a frame after each one.
    ///
    /// [`Effects::after`]: crate::runtime::Effects::after
    pub fn advance(&mut self, by: Duration) {
        let end = self.session.clock() + by;
        while let Some(due) = self.session.next_deadline().filter(|due| *due <= end) {
            self.session.set_clock(due);
            self.session.run_timers();
            self.frame();
        }
        self.session.set_clock(end);
        self.frame();
    }

    /// Wait for every [`Effects::perform`] task, the ones their messages
    /// start included, every image and every map tile loading, sending
    /// each message as its task finishes, with a frame after each batch.
    /// Panics if they take more than ten seconds.
    ///
    /// [`Effects::perform`]: crate::runtime::Effects::perform
    pub fn run_until_idle(&mut self) {
        let limit = Instant::now() + Duration::from_secs(10);
        while self.session.tasks_pending() {
            assert!(
                Instant::now() < limit,
                "las tareas de Effects::perform no terminaron en 10 s"
            );
            // Loaded map tiles show a few per frame: frames go on even when
            // nothing new landed.
            self.session.wait_for_tasks(Duration::from_millis(50));
            self.frame();
        }
    }

    /// Wait for the next tasks to finish (five seconds at most) and send
    /// their messages; the tasks those start may still run.
    #[cfg(test)]
    pub(crate) fn run_next_tasks(&mut self) {
        assert!(
            self.session.wait_for_tasks(Duration::from_secs(5)),
            "ninguna tarea terminó en 5 s"
        );
        self.frame();
    }

    /// Whether some [`Effects::perform`] task has not sent its message, or
    /// some image is still loading.
    ///
    /// [`Effects::perform`]: crate::runtime::Effects::perform
    pub fn tasks_pending(&self) -> bool {
        self.session.tasks_pending()
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
        let ran = self.session.menu_command(id);
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

    /// Whether an agent's call waits for the person in the confirmation
    /// dialog.
    pub fn confirmation_open(&self) -> bool {
        self.session.confirmation_open()
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

    /// An assistive technology's action, as AccessKit delivers it.
    #[cfg(test)]
    pub(crate) fn accessibility_action(&mut self, request: &accesskit::ActionRequest) {
        self.session.accessibility_action(request);
        self.frame();
    }

    /// Move the pointer to `point`, without pressing.
    #[cfg(test)]
    pub(crate) fn move_pointer(&mut self, point: crate::geometry::Point) {
        self.session.pointer_moved(Some(point));
        self.frame();
    }

    /// Press and release the primary button at `point`, whatever is there.
    #[cfg(test)]
    pub(crate) fn click_point(&mut self, point: crate::geometry::Point) {
        self.session.pointer_moved(Some(point));
        self.frame();
        self.session.pointer_button(true);
        self.session.pointer_button(false);
        self.frame();
    }

    /// The pointer's shape, as the window shows it (`Pointer` over links,
    /// buttons and clickable map areas).
    pub fn mouse_cursor(&self) -> Cursor {
        self.cursor
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

    /// What the current frame draws, in painter's order.
    pub fn draw_list(&self) -> crate::render::DrawList {
        self.session.paint_list()
    }

    /// Every message sent to `update`, in order.
    pub fn messages(&self) -> &[A::Msg] {
        self.session.messages()
    }

    /// Where each message of [`Self::messages`] came from.
    pub fn message_sources(&self) -> &[MsgSource] {
        self.session.message_sources()
    }

    /// Scroll the element into view, move the pointer to its center, press
    /// and release. Panics when something else is on top of that point. A
    /// disabled control takes the click and sends nothing, as on screen.
    #[track_caller]
    pub fn click(&mut self, by: By) {
        self.click_series(by, 1);
    }

    /// [`Self::click`] twice in a row: selects a word in a text input.
    #[track_caller]
    pub fn double_click(&mut self, by: By) {
        self.click_series(by, 2);
    }

    /// [`Self::click`] three times in a row: selects a line in a text input.
    #[track_caller]
    pub fn triple_click(&mut self, by: By) {
        self.click_series(by, 3);
    }

    #[track_caller]
    fn click_series(&mut self, by: By, clicks: u32) {
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
        for count in 1..=clicks {
            self.session.pointer_clicks(true, count);
            self.session.pointer_button(false);
            self.frame();
        }
    }

    /// Move the pointer over the element and turn the wheel by `delta_y`
    /// points, as the window gets it: negative moves toward the end of the
    /// content, positive toward its start.
    #[track_caller]
    pub fn scroll_wheel(&mut self, over: By, delta_y: f32) {
        self.hover(over);
        self.session.scroll_by(delta_y);
        self.frame();
    }

    /// Move the pointer over the element and scroll by `(dx, dy)` points as
    /// a trackpad does (positive moves the content right and down): it pans
    /// a map, elsewhere it scrolls across and down (with Shift held,
    /// [`Self::hold_modifiers`], a vertical `dy` scrolls across).
    #[track_caller]
    pub fn scroll_wheel_xy(&mut self, over: By, dx: f32, dy: f32) {
        self.hover(over);
        self.session.wheel(Point { x: dx, y: dy }, false);
        self.frame();
    }

    /// Move the pointer over the element and turn a mouse wheel by
    /// `notches` (positive away from the person): a map zooms in a level
    /// per notch.
    #[track_caller]
    pub fn wheel_notches(&mut self, over: By, notches: f32) {
        self.hover(over);
        self.session.wheel(
            Point {
                x: 0.0,
                y: notches * LINE_SCROLL,
            },
            true,
        );
        self.frame();
    }

    /// A whole trackpad pinch over the element's center, scaling by
    /// `factor` (2.0 zooms a map in a level).
    #[track_caller]
    pub fn pinch(&mut self, over: By, factor: f64) {
        self.hover(over);
        self.session.pinch(factor - 1.0, true);
        self.frame();
    }

    /// Press at the element's center, move the pointer by `(dx, dy)` points
    /// in `DRAG_STEPS` steps of `DRAG_STEP` and release at once: a
    /// flick, after which a map glides on ([`Self::advance`] lets it).
    #[track_caller]
    pub fn drag(&mut self, by: By, dx: f32, dy: f32) {
        self.drag_steps(by, dx, dy);
        self.session.pointer_button(false);
        self.frame();
    }

    /// [`Self::drag`], holding still for a moment before releasing: the
    /// content stops where the pointer left it.
    #[track_caller]
    pub fn drag_and_hold(&mut self, by: By, dx: f32, dy: f32) {
        self.drag_steps(by, dx, dy);
        self.advance(Duration::from_millis(150));
        self.session.pointer_button(false);
        self.frame();
    }

    /// Let go of the pointer after [`Self::drag_steps`].
    #[cfg(test)]
    pub(crate) fn release_pointer(&mut self) {
        self.session.pointer_button(false);
        self.frame();
    }

    /// Press at the element's center and move by `(dx, dy)` in
    /// [`DRAG_STEPS`] steps, still pressed.
    #[track_caller]
    pub(crate) fn drag_steps(&mut self, by: By, dx: f32, dy: f32) {
        self.hover(by);
        let start = self.session.cursor().unwrap_or_default();
        self.session.pointer_clicks(true, 1);
        self.frame();
        for step in 1..=DRAG_STEPS {
            let t = step as f32 / DRAG_STEPS as f32;
            self.advance(DRAG_STEP);
            self.session.pointer_moved(Some(Point {
                x: start.x + dx * t,
                y: start.y + dy * t,
            }));
            self.frame();
        }
    }

    /// Click at `(x, y)` points from the element's top left corner.
    #[track_caller]
    pub fn click_at(&mut self, by: By, x: f32, y: f32) {
        self.click_series_at(by, x, y, 1);
    }

    /// [`Self::click_at`] twice in a row.
    #[track_caller]
    pub fn double_click_at(&mut self, by: By, x: f32, y: f32) {
        self.click_series_at(by, x, y, 2);
    }

    #[track_caller]
    fn click_series_at(&mut self, by: By, x: f32, y: f32, clicks: u32) {
        let id = self.id_of(by);
        if self.session.reveal(id) {
            self.frame();
        }
        let bounds = self.session.layout().bounds(id).unwrap_or_default();
        self.session.pointer_moved(Some(Point {
            x: bounds.x + x,
            y: bounds.y + y,
        }));
        self.frame();
        for count in 1..=clicks {
            self.session.pointer_clicks(true, count);
            self.session.pointer_button(false);
            self.frame();
        }
    }

    /// Hold `modifiers` (⇧, ⌘…) for the pointer gestures that follow, as
    /// the window reports them; [`Modifiers::NONE`] lets go.
    pub fn hold_modifiers(&mut self, modifiers: Modifiers) {
        self.session.set_modifiers(modifiers);
    }

    /// Scroll virtual list `list` ([`Element::virtual_rows`]) until its
    /// row `index` is in view, built or not, as the scroll bar would.
    ///
    /// [`Element::virtual_rows`]: crate::element::Element::virtual_rows
    #[track_caller]
    pub fn reveal_row(&mut self, list: By, index: usize) {
        let id = self.id_of(list);
        self.session.reveal_row(id, index);
        self.frame();
    }

    /// Scroll the element into view and move the pointer to its center,
    /// without pressing.
    #[track_caller]
    pub fn hover(&mut self, by: By) {
        let id = self.id_of(by);
        if self.session.reveal(id) {
            self.frame();
        }
        let center = self.center_of(id);
        self.session.pointer_moved(Some(center));
        self.frame();
    }

    /// Secondary click (right click) at the element's center: opens the
    /// [`Element::context_menu`] of the element or of its nearest ancestor
    /// with one.
    ///
    /// [`Element::context_menu`]: crate::element::Element::context_menu
    #[track_caller]
    pub fn right_click(&mut self, by: By) {
        self.hover(by);
        self.session.secondary_click();
        self.frame();
    }

    /// The laid-out tree as text, one node per line (role, id, name,
    /// `test_id`, bounds, classes): what the inspector prints.
    pub fn dump(&self) -> String {
        crate::devtools::dump(self.session.tree(), self.session.layout())
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

    /// Whether the control is busy: a task it (or its command) started
    /// still runs, or the app marked it [`Element::busy`].
    ///
    /// [`Element::busy`]: crate::element::Element::busy
    #[track_caller]
    pub fn is_busy(&self, by: By) -> bool {
        let screen = self.screen();
        self.control_of(screen.get(by)).node().busy
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

    /// What the window does on `RedrawRequested`, minus paint; an open or
    /// save panel asked for meanwhile gets the answer of
    /// [`Self::on_file_pick`] or [`Self::on_file_save`].
    fn frame(&mut self) {
        loop {
            self.session.prepare_frame();
            let outbox = self.session.take_outbox();
            if let Some(request) = outbox.pick_file {
                self.file_pick_request = Some(request);
                self.file_save_request = None;
            }
            if let Some(request) = outbox.save_file {
                self.file_save_request = Some(request);
                self.file_pick_request = None;
            }
            if let Some(cursor) = outbox.cursor {
                self.cursor = cursor;
            }
            let answer = if self.file_save_request.is_some() {
                &self.file_save_answer
            } else {
                &self.file_pick_answer
            };
            match answer {
                Some(answer) if self.session.file_pick_pending() => {
                    self.session.finish_file_pick(answer.clone());
                }
                _ => break,
            }
        }
    }

    /// From now on, answer the save panel of [`Effects::save_file`] with
    /// `path` (`None`: the person cancels), now if one is open. Until this
    /// is called the panel stays open ([`Self::file_save_request`]).
    ///
    /// [`Effects::save_file`]: crate::runtime::Effects::save_file
    pub fn on_file_save(&mut self, path: Option<PathBuf>) {
        self.file_save_answer = Some(path);
        self.frame();
    }

    /// The save panel waiting for an answer, if any.
    pub fn file_save_request(&self) -> Option<&SaveFile> {
        self.file_save_request
            .as_ref()
            .filter(|_| self.session.file_pick_pending())
    }

    /// From now on, answer the open panel of [`Effects::pick_file`] with
    /// `path` (`None`: the person cancels), now if one is open. Until this
    /// is called the panel stays open ([`Self::file_pick_request`]).
    ///
    /// [`Effects::pick_file`]: crate::runtime::Effects::pick_file
    pub fn on_file_pick(&mut self, path: Option<PathBuf>) {
        self.file_pick_answer = Some(path);
        self.frame();
    }

    /// The open panel waiting for an answer, if any.
    pub fn file_pick_request(&self) -> Option<&PickFile> {
        self.file_pick_request
            .as_ref()
            .filter(|_| self.session.file_pick_pending())
    }

    /// Start dragging files over the window, as from Finder: drop zones
    /// take their `drag:` classes.
    pub fn hover_files(&mut self) {
        self.session.hover_files();
        self.frame();
    }

    /// The dragged files leave the window without a drop.
    pub fn cancel_file_drag(&mut self) {
        self.session.cancel_file_drag();
        self.frame();
    }

    /// Drag `paths` over the element and drop them there. Panics when no
    /// drop zone takes them.
    #[track_caller]
    pub fn drop_files(&mut self, on: By, paths: impl IntoIterator<Item = impl Into<PathBuf>>) {
        self.hover(on);
        self.session.hover_files();
        self.frame();
        let paths = paths.into_iter().map(Into::into).collect();
        assert!(
            self.session.drop_files(paths),
            "ninguna zona de soltar recibió los archivos"
        );
        self.frame();
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
    fn word_editing_and_multi_clicks_reach_the_field() {
        let clipboard = MemoryClipboard::new();
        let mut app = TestApp::builder(Form::default())
            .clipboard(clipboard.clone())
            .build();
        app.type_text(By::test_id("nombre"), "Ana Pérez");
        let option = Modifiers {
            option: true,
            ..Modifiers::NONE
        };
        app.press(Key::Backspace, option);
        assert_eq!(app.value(By::test_id("nombre")), "Ana ");
        assert_eq!(app.app().name, "Ana ");

        app.triple_click(By::test_id("nombre"));
        let (key, modifiers) = cmd(Key::C);
        app.press(key, modifiers);
        assert_eq!(clipboard.text().as_deref(), Some("Ana "));
        let (key, modifiers) = cmd(Key::Backspace);
        app.press(key, modifiers);
        assert_eq!(app.value(By::test_id("nombre")), "");
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

    mod files {
        use std::path::PathBuf;

        use crate::components::{ButtonVariant, button};
        use crate::devtools::MsgSource;
        use crate::element::{Element, Role, div, text};
        use crate::runtime::{App, Effects, IMAGE_FILE_TYPES, PickFile, SaveFile};
        use crate::testing::{By, TestApp};

        #[derive(Clone, Debug, PartialEq)]
        enum Msg {
            Choose,
            Chosen(Option<PathBuf>),
            Dropped(&'static str, Vec<PathBuf>),
        }

        #[derive(Default)]
        struct Uploader {
            zones: usize,
            disabled: bool,
        }

        impl App for Uploader {
            type Msg = Msg;
            fn title(&self) -> String {
                "Subir".into()
            }
            fn update(&mut self, msg: Msg) -> Effects<Msg> {
                match msg {
                    Msg::Choose => Effects::pick_file(PickFile::images("Elegir foto"), Msg::Chosen),
                    _ => Effects::none(),
                }
            }
            fn view(&self) -> Element<Msg> {
                let zone = |name: &'static str| {
                    div()
                        .class("w-40 h-20 border-0 drag:border-2")
                        .test_id(name)
                        .disabled(self.disabled)
                        .on_drop_files(move |paths| Msg::Dropped(name, paths))
                        .child(text(name))
                };
                div()
                    .class("flex flex-col gap-2")
                    .child(button("Elegir", ButtonVariant::Primary).on_click(Msg::Choose))
                    .children(["fotos", "logos"].into_iter().take(self.zones).map(zone))
            }
        }

        fn border(app: &TestApp<Uploader>, zone: &str) -> f32 {
            app.screen()
                .get(By::test_id(zone))
                .node()
                .style
                .border_width
                .top
        }

        #[test]
        fn the_open_panel_answers_with_the_programmed_file_or_none() {
            let mut app = TestApp::new(Uploader::default());
            app.click(By::role(Role::Button).name("Elegir"));
            assert_eq!(
                app.file_pick_request(),
                Some(&PickFile {
                    title: "Elegir foto".into(),
                    types: IMAGE_FILE_TYPES,
                })
            );
            assert_eq!(app.messages(), [Msg::Choose], "waiting for the person");

            app.on_file_pick(Some("/fotos/ana.png".into()));
            assert_eq!(app.file_pick_request(), None);
            assert_eq!(
                app.messages().last(),
                Some(&Msg::Chosen(Some("/fotos/ana.png".into())))
            );
            assert_eq!(app.message_sources().last(), Some(&MsgSource::Files));

            app.on_file_pick(None);
            app.click(By::role(Role::Button).name("Elegir"));
            assert_eq!(app.messages().last(), Some(&Msg::Chosen(None)));
        }

        #[test]
        fn the_save_panel_answers_with_the_programmed_path_or_none() {
            #[derive(Clone, Debug, PartialEq)]
            enum SaveMsg {
                Export,
                Saved(Option<PathBuf>),
            }
            struct Exporter;
            impl App for Exporter {
                type Msg = SaveMsg;
                fn title(&self) -> String {
                    "Exportar".into()
                }
                fn update(&mut self, msg: SaveMsg) -> Effects<SaveMsg> {
                    match msg {
                        SaveMsg::Export => Effects::save_file(
                            SaveFile::new("Exportar a Excel", "clientes.xlsx").types(&["xlsx"]),
                            SaveMsg::Saved,
                        ),
                        SaveMsg::Saved(_) => Effects::none(),
                    }
                }
                fn view(&self) -> Element<SaveMsg> {
                    button("Exportar", ButtonVariant::Primary).on_click(SaveMsg::Export)
                }
            }

            let mut app = TestApp::new(Exporter);
            // No answer programmed: the panel stays open (closing the real
            // window with a sheet open takes this same pending path).
            app.click(By::role(Role::Button).name("Exportar"));
            assert_eq!(
                app.file_save_request(),
                Some(&SaveFile {
                    title: "Exportar a Excel".into(),
                    name: "clientes.xlsx".into(),
                    types: &["xlsx"],
                })
            );
            assert!(app.file_pick_request().is_none());
            assert_eq!(app.messages(), [SaveMsg::Export], "waiting for the person");

            app.on_file_save(Some("/tmp/clientes.xlsx".into()));
            assert_eq!(app.file_save_request(), None);
            assert_eq!(
                app.messages().last(),
                Some(&SaveMsg::Saved(Some("/tmp/clientes.xlsx".into())))
            );
            assert_eq!(app.message_sources().last(), Some(&MsgSource::Files));

            app.on_file_save(None);
            app.click(By::role(Role::Button).name("Exportar"));
            assert_eq!(app.file_save_request(), None);
            assert_eq!(app.messages().last(), Some(&SaveMsg::Saved(None)));
        }

        #[test]
        fn dropped_files_go_to_the_only_zone_and_drag_styles_come_and_go() {
            let mut app = TestApp::new(Uploader {
                zones: 1,
                disabled: false,
            });
            assert_eq!(border(&app, "fotos"), 0.0);
            app.hover_files();
            assert_eq!(border(&app, "fotos"), 2.0, "drag: while files hover");
            app.cancel_file_drag();
            assert_eq!(border(&app, "fotos"), 0.0);

            app.drop_files(By::text("fotos"), ["/a.png", "/b.jpg"]);
            assert_eq!(
                app.messages(),
                [Msg::Dropped(
                    "fotos",
                    vec!["/a.png".into(), "/b.jpg".into()]
                )]
            );
            assert_eq!(border(&app, "fotos"), 0.0, "drag: ends with the drop");
        }

        #[test]
        fn with_several_zones_the_one_under_the_pointer_gets_the_files() {
            let mut app = TestApp::new(Uploader {
                zones: 2,
                disabled: false,
            });
            app.drop_files(By::test_id("logos"), ["/logo.webp"]);
            assert_eq!(
                app.messages(),
                [Msg::Dropped("logos", vec!["/logo.webp".into()])]
            );
        }

        #[test]
        #[should_panic(expected = "ninguna zona de soltar")]
        fn a_disabled_zone_takes_no_files() {
            let mut app = TestApp::new(Uploader {
                zones: 1,
                disabled: true,
            });
            app.hover_files();
            assert_eq!(border(&app, "fotos"), 0.0, "no drag: when disabled");
            app.drop_files(By::test_id("fotos"), ["/a.png"]);
        }
    }
}
