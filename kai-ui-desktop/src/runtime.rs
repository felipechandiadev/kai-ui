//! Elm-style runtime: `view` builds an element tree from the model, events
//! become messages, `update` changes the model, and the next frame renders
//! the new `view`.
//!
//! The runtime owns everything that is not app state. A window-independent
//! session keeps text buffers, focus/hover/pressed state, scroll offsets
//! and the caret/selection of each input; the window state around it owns
//! the window, renderer and accessibility adapter, feeds it events and
//! applies what it asks of the window (cursor, IME, title, redraw). Inputs
//! are controlled: every frame the value in the tree is pushed into the
//! editor when it differs, so an input without `on_input` is effectively
//! read-only.
//!
//! The loop waits for events (`ControlFlow::Wait`): nothing redraws on its
//! own, except an animated [`App::background`], which asks for frames at
//! [`FRAME_INTERVAL`] only while the window is visible, active and motion
//! is not reduced. Those frames reuse the last prepared draws unless
//! something else changed (`dirty`). A map that glides or zooms on its own
//! also asks for frames until it settles. Pending timers ([`Effects::after`])
//! wake the loop at the nearest one (`ControlFlow::WaitUntil`).

use std::any::Any;
use std::cell::RefCell;
use std::collections::HashSet;
use std::error::Error;
use std::fmt::{self, Debug};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use winit::{
    application::ApplicationHandler,
    dpi::{LogicalSize, PhysicalPosition, PhysicalSize},
    event::{ElementState, MouseButton, MouseScrollDelta, TouchPhase, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop, EventLoopProxy},
    window::{CursorIcon, Window, WindowId},
};

use crate::background::{BackgroundClock, BackgroundEffect, BackgroundLayer, FRAME_INTERVAL};
use crate::commands::{Availability, CommandRegistry, CommandState};
use crate::devtools::{FrameStats, FrameTimes};
use crate::element::{Element, Kind, WidgetId};
use crate::geometry::{PhysicalPoint, Point, ScaleFactor, Size};
use crate::layout::{Content, Layout, WindowActivity};
use crate::platform_adapter::{adapt_ime_event, adapt_keyboard_event, adapt_modifiers};
use crate::render::wgpu_backend::WgpuRenderer;
use crate::responder::{self, Action, Responder};
use crate::session::{Measure, Session};
use crate::style::Cursor;
use crate::text_field::SystemClipboard;
use crate::theme::{Appearance, Theme};
use crate::ui_engine::{ClickCounter, Modifiers};

/// Points per wheel notch.
pub(crate) const LINE_SCROLL: f32 = 40.0;

/// What `update` asks the runtime to do, as a value (a subset of 13,
/// Paso 15): [`Effects::after`] sends a message later,
/// [`Effects::perform`] runs work off the UI thread and
/// [`Effects::pick_file`] asks for a file.
#[must_use]
pub struct Effects<Msg>(Vec<Effect<Msg>>);

/// A task of [`Effects::perform`], with its result type erased.
pub(crate) type Task = Box<dyn FnOnce() -> Box<dyn Any + Send> + Send>;
/// Turns a task's erased result back into the app's message.
pub(crate) type TaskDone<Msg> = Box<dyn FnOnce(Box<dyn Any + Send>) -> Msg>;

pub(crate) enum Effect<Msg> {
    /// Send `msg` after `delay`. A keyed timer replaces the pending one
    /// with the same key.
    After {
        key: Option<String>,
        delay: Duration,
        msg: Msg,
    },
    /// Run `task` on the task pool and send `done(result)`. A keyed task
    /// replaces the pending one with the same key (its result is dropped).
    Perform {
        key: Option<String>,
        task: Task,
        done: TaskDone<Msg>,
    },
    /// Drop the pending timer or task with this key.
    Cancel(String),
    /// Ask the person for a file with the system's open panel.
    PickFile {
        request: PickFile,
        done: FilePicked<Msg>,
    },
}

/// Turns the file chosen in the open panel (`None`: cancelled) into the
/// app's message.
pub(crate) type FilePicked<Msg> = Box<dyn FnOnce(Option<PathBuf>) -> Msg>;

/// Extensions of the images `kai::media` decodes.
pub const IMAGE_FILE_TYPES: &[&str] = &["png", "jpg", "jpeg", "webp"];

/// What the open panel of [`Effects::pick_file`] shows.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PickFile {
    /// Title and message of the panel ("Elegir foto").
    pub title: String,
    /// Extensions that can be chosen, without the dot; empty: any file.
    pub types: &'static [&'static str],
}

impl PickFile {
    /// Any file.
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            types: &[],
        }
    }

    /// The images `kai::media` decodes ([`IMAGE_FILE_TYPES`]).
    pub fn images(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            types: IMAGE_FILE_TYPES,
        }
    }
}

impl<Msg> Effects<Msg> {
    pub fn none() -> Self {
        Self(Vec::new())
    }

    pub fn batch(effects: impl IntoIterator<Item = Self>) -> Self {
        Self(effects.into_iter().flat_map(|effects| effects.0).collect())
    }

    pub fn is_none(&self) -> bool {
        self.0.is_empty()
    }

    /// Send `msg` to `update` once `delay` has passed. The window does not
    /// wake up before that; tests move time with `TestApp::advance`.
    pub fn after(delay: Duration, msg: Msg) -> Self {
        Self(vec![Effect::After {
            key: None,
            delay,
            msg,
        }])
    }

    /// Run `task` off the UI thread, on a small shared pool, and send
    /// `to_msg(result)` to `update` when it finishes. Reading files,
    /// decoding or network calls go here so the frame never waits for
    /// them. A task that panics sends nothing. Tests wait for tasks with
    /// `TestApp::run_until_idle`.
    pub fn perform<T: Send + 'static>(
        task: impl FnOnce() -> T + Send + 'static,
        to_msg: impl FnOnce(T) -> Msg + 'static,
    ) -> Self {
        Self(vec![Effect::Perform {
            key: None,
            task: Box::new(move || Box::new(task()) as Box<dyn Any + Send>),
            done: Box::new(move |output| {
                let output = output
                    .downcast::<T>()
                    .expect("el resultado de la tarea tiene el tipo de su to_msg");
                to_msg(*output)
            }),
        }])
    }

    /// Show the system's open panel (`NSOpenPanel`) and send
    /// `to_msg(Some(path))` with the chosen file, or `to_msg(None)` when
    /// the person cancels. One panel at a time: a second request while one
    /// is open replaces it. Tests answer it with `TestApp::on_file_pick`.
    pub fn pick_file(
        request: PickFile,
        to_msg: impl FnOnce(Option<PathBuf>) -> Msg + 'static,
    ) -> Self {
        Self(vec![Effect::PickFile {
            request,
            done: Box::new(to_msg),
        }])
    }

    /// Give these timers and tasks `key`: a new one with the same key
    /// replaces the pending one, and [`Effects::cancel`] drops it.
    pub fn keyed(mut self, key: impl Into<String>) -> Self {
        let key = key.into();
        for effect in &mut self.0 {
            match effect {
                Effect::After { key: slot, .. } | Effect::Perform { key: slot, .. } => {
                    *slot = Some(key.clone());
                }
                Effect::Cancel(_) | Effect::PickFile { .. } => {}
            }
        }
        self
    }

    /// Drop the pending timer or task with `key`, if any (a running task
    /// finishes, but its message is not sent).
    pub fn cancel(key: impl Into<String>) -> Self {
        Self(vec![Effect::Cancel(key.into())])
    }

    pub(crate) fn into_vec(self) -> Vec<Effect<Msg>> {
        self.0
    }
}

impl<Msg: Debug> fmt::Debug for Effects<Msg> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.0.is_empty() {
            return f.write_str("Effects::none()");
        }
        let mut list = f.debug_list();
        for effect in &self.0 {
            match effect {
                Effect::After { key, delay, msg } => {
                    list.entry(&format_args!("after({delay:?}, {msg:?}, key: {key:?})"))
                }
                Effect::Perform { key, .. } => list.entry(&format_args!("perform(key: {key:?})")),
                Effect::Cancel(key) => list.entry(&format_args!("cancel({key:?})")),
                Effect::PickFile { request, .. } => {
                    list.entry(&format_args!("pick_file({:?})", request.title))
                }
            };
        }
        list.finish()
    }
}

/// An Elm app: a model that turns messages into a new model (and effects)
/// and is drawn by `view`. Apps without effects implement [`SimpleApp`].
pub trait App: 'static {
    type Msg: Clone + Debug + 'static;

    /// Called once, before the first `view`.
    fn init(&mut self) -> Effects<Self::Msg> {
        Effects::none()
    }

    fn title(&self) -> String;
    fn update(&mut self, msg: Self::Msg) -> Effects<Self::Msg>;
    fn view(&self) -> Element<Self::Msg>;

    /// Window background effect for the current model, read after every
    /// `update` (so it can change live). Default: the solid window color.
    fn background(&self) -> BackgroundEffect {
        BackgroundEffect::NONE
    }

    /// Theme the app asks for, read after every `update`; `None` keeps the
    /// one given to [`run`].
    fn theme(&self) -> Option<Theme> {
        None
    }

    /// Forced key/main state (gallery and captures); `None` follows the
    /// real window.
    fn window_override(&self) -> Option<WindowActivity> {
        None
    }

    /// Message for a new size of the window's content, in logical points:
    /// once after `init` and again on every resize. Default: none. Apps
    /// use it for what classes like `sm:` cannot do, such as starting
    /// with the sidebar collapsed in a narrow window.
    fn resized(&self, _size: Size) -> Option<Self::Msg> {
        None
    }

    /// Declare the app's commands, once at startup (see [`crate::commands`]).
    fn commands(_registry: &mut CommandRegistry<Self>)
    where
        Self: Sized,
    {
    }
}

/// An [`App`] whose `update` has no effects; every `SimpleApp` is an `App`.
pub trait SimpleApp: 'static {
    type Msg: Clone + Debug + 'static;

    fn title(&self) -> String;
    fn update(&mut self, msg: Self::Msg);
    fn view(&self) -> Element<Self::Msg>;

    /// See [`App::background`].
    fn background(&self) -> BackgroundEffect {
        BackgroundEffect::NONE
    }

    /// See [`App::theme`].
    fn theme(&self) -> Option<Theme> {
        None
    }

    /// See [`App::window_override`].
    fn window_override(&self) -> Option<WindowActivity> {
        None
    }

    /// See [`App::resized`].
    fn resized(&self, _size: Size) -> Option<Self::Msg> {
        None
    }

    /// See [`App::commands`].
    fn commands(_registry: &mut CommandRegistry<Self>)
    where
        Self: Sized,
    {
    }
}

impl<T: SimpleApp> App for T {
    type Msg = T::Msg;

    fn title(&self) -> String {
        SimpleApp::title(self)
    }

    fn update(&mut self, msg: Self::Msg) -> Effects<Self::Msg> {
        SimpleApp::update(self, msg);
        Effects::none()
    }

    fn view(&self) -> Element<Self::Msg> {
        SimpleApp::view(self)
    }

    fn background(&self) -> BackgroundEffect {
        SimpleApp::background(self)
    }

    fn theme(&self) -> Option<Theme> {
        SimpleApp::theme(self)
    }

    fn window_override(&self) -> Option<WindowActivity> {
        SimpleApp::window_override(self)
    }

    fn resized(&self, size: Size) -> Option<Self::Msg> {
        SimpleApp::resized(self, size)
    }

    fn commands(registry: &mut CommandRegistry<Self>) {
        <T as SimpleApp>::commands(registry);
    }
}

/// Whether an animated background should get frames now: the effect moves,
/// motion is not reduced, and the window is visible and active.
pub fn background_running(
    background: &BackgroundEffect,
    reduce_motion: bool,
    occluded: bool,
    activity: WindowActivity,
) -> bool {
    background.is_animated() && !reduce_motion && !occluded && activity == WindowActivity::ACTIVE
}

/// Open a window and run `app` until it is closed.
pub fn run<A: App>(app: A, theme: Theme) -> Result<(), Box<dyn Error>> {
    run_with(app, theme, |_| {})
}

/// Like [`run`], and also serve the exposed commands over MCP on stdin and
/// stdout (see [`crate::mcp`]). The app quits when the client disconnects.
#[cfg(feature = "mcp")]
pub fn run_with_mcp<A: App>(app: A, theme: Theme) -> Result<(), Box<dyn Error>> {
    let tools = crate::mcp::Tools::of::<A>(app.title());
    run_with(app, theme, move |proxy| {
        let proxy = proxy.clone();
        crate::mcp::serve_stdio(tools, move |event| {
            let _ = proxy.send_event(UserEvent::Mcp(event));
        });
    })
}

fn run_with<A: App>(
    app: A,
    theme: Theme,
    start: impl FnOnce(&EventLoopProxy<UserEvent>),
) -> Result<(), Box<dyn Error>> {
    let event_loop = EventLoop::<UserEvent>::with_user_event().build()?;
    event_loop.set_control_flow(ControlFlow::Wait);
    let proxy = event_loop.create_proxy();
    #[cfg(target_os = "macos")]
    {
        let proxy = proxy.clone();
        crate::menu::forward_events(move |id| {
            let _ = proxy.send_event(UserEvent::Menu(id));
        });
    }
    start(&proxy);
    let mut runner = Runner {
        app: Some(app),
        theme,
        state: None,
        proxy,
        #[cfg(feature = "mcp")]
        pending: Vec::new(),
    };
    event_loop.run_app(&mut runner)?;
    Ok(())
}

/// Build the tree for the current model with ids assigned, its controls
/// bound to commands ([`Element::on_command`]) following their current
/// availability. Debug builds report duplicate keys and test ids (see
/// [`Element::validate_ids`]), each problem once per thread. Virtual lists
/// get their first rows, as before their first layout.
pub fn build_view<A: App>(app: &A) -> Element<A::Msg> {
    let mut tree = build_view_with(app, &CommandRegistry::<A>::of().states(app));
    if !crate::virtual_rows::materialize(&mut tree, &|_| Default::default()).is_empty() {
        tree.assign_ids();
    }
    tree
}

/// [`build_view`] with the command states the session already computed,
/// virtual lists still without rows.
pub fn build_view_with<A: App>(app: &A, commands: &[CommandState]) -> Element<A::Msg> {
    let mut tree = app.view();
    bind_commands(&mut tree, commands);
    tree.assign_ids();
    if cfg!(debug_assertions) {
        report_once("Ids inválidos", tree.validate_ids());
    }
    tree
}

/// Disable controls whose command is not available, and give them the
/// command's id as default `test_id` (parts keep theirs, `<owner>.<part>`)
/// and its title as default name.
fn bind_commands<Msg>(element: &mut Element<Msg>, commands: &[CommandState]) {
    if let Some(id) = element.command {
        match commands.iter().find(|command| command.id == id) {
            Some(command) => {
                match &command.availability {
                    Availability::Enabled => {}
                    Availability::Disabled { reason } => {
                        element.disabled = true;
                        element.description.get_or_insert_with(|| reason.clone());
                    }
                    Availability::Hidden => element.disabled = true,
                }
                if element.part.is_none() {
                    element.test_id.get_or_insert_with(|| id.to_owned());
                }
                if element.label.is_none() && element.text_content().is_empty() {
                    element.label = Some(command.title.to_owned());
                }
            }
            None => report_once(
                "Comandos inválidos",
                vec![format!("on_command con \"{id}\", que no está registrado")],
            ),
        }
    }
    for child in &mut element.children {
        bind_commands(child, commands);
    }
}

/// Debug builds: print each problem once per thread, after `context`.
pub(crate) fn report_once(context: &str, problems: Vec<String>) {
    if !cfg!(debug_assertions) {
        return;
    }
    thread_local! {
        static REPORTED: RefCell<HashSet<String>> = RefCell::new(HashSet::new());
    }
    REPORTED.with_borrow_mut(|reported| {
        for problem in problems {
            if !reported.contains(&problem) {
                eprintln!("{context}: {problem}");
                reported.insert(problem);
            }
        }
    });
}

/// Nearest control (clickable or editable element, enabled or not) on a
/// hit-test path, or the input an outline delegates focus to. Disabled
/// controls still capture the pointer.
pub fn pointer_target(layout: &Layout, path: &[WidgetId]) -> Option<WidgetId> {
    path.iter().rev().find_map(|id| {
        let node = layout.node(*id)?;
        if node.control {
            Some(node.id)
        } else {
            node.focus_delegate
        }
    })
}

/// Message for a standard action that the focused element did not consume:
/// the first `on_action` along the responder chain (focus, ancestors) that
/// returns one. The window handles `CancelOperation` by doing nothing.
/// `Action::Command` never gets here: the session runs it from the
/// registry.
pub fn action_message<Msg>(
    tree: &Element<Msg>,
    focused: Option<WidgetId>,
    action: Action,
) -> Option<Msg> {
    let path = focused.and_then(|id| tree.path_to(id)).unwrap_or_default();
    for responder in responder::chain(&path) {
        match responder {
            Responder::Widget(id) => {
                let handler = tree
                    .find(id)
                    .and_then(|element| element.handlers.on_action.as_ref());
                if let Some(msg) = handler.and_then(|handler| handler(action)) {
                    return Some(msg);
                }
            }
            Responder::Window if action == Action::CancelOperation => return None,
            Responder::Window => {}
            Responder::App => {
                if cfg!(debug_assertions) {
                    eprintln!("Acción sin responder: {action:?}");
                }
            }
        }
    }
    None
}

/// Whether gaining focus should select the whole value of input `id`.
pub fn selects_on_focus<Msg>(tree: &Element<Msg>, id: WidgetId) -> bool {
    tree.find(id).is_some_and(|element| {
        matches!(
            element.kind,
            Kind::Input {
                select_on_focus: true,
                ..
            }
        )
    })
}

/// Cursor icon for a hit-test path: the innermost explicit `cursor-*`, or
/// a text cursor over inputs.
pub fn cursor_for(layout: &Layout, path: &[WidgetId]) -> Cursor {
    for id in path.iter().rev() {
        let Some(node) = layout.node(*id) else {
            continue;
        };
        if let Some(cursor) = node.style.cursor {
            return cursor;
        }
        if matches!(node.content, Content::Input { .. }) {
            return Cursor::Text;
        }
    }
    Cursor::Default
}

/// What reaches the event loop from outside winit's own events.
#[derive(Debug)]
enum UserEvent {
    A11y(accesskit_winit::Event),
    /// The menu bar item of this command id was chosen.
    #[cfg_attr(not(target_os = "macos"), allow(dead_code))]
    Menu(String),
    /// An [`Effects::perform`] task finished.
    Task,
    /// News from the MCP server: a client connected, called or cancelled a
    /// tool, or went away.
    #[cfg(feature = "mcp")]
    Mcp(crate::mcp::Event),
}

impl From<accesskit_winit::Event> for UserEvent {
    fn from(event: accesskit_winit::Event) -> Self {
        Self::A11y(event)
    }
}

struct Runner<A: App> {
    app: Option<A>,
    theme: Theme,
    state: Option<WindowState<A>>,
    proxy: EventLoopProxy<UserEvent>,
    /// MCP events that arrived before the window existed.
    #[cfg(feature = "mcp")]
    pending: Vec<crate::mcp::Event>,
}

struct WindowState<A: App> {
    window: Arc<Window>,
    accessibility: accesskit_winit::Adapter,
    renderer: WgpuRenderer,
    session: Session<A>,
    scale_factor: ScaleFactor,
    modifiers: Modifiers,
    clicks: ClickCounter,
    /// `WindowEvent::Occluded` (hidden, minimized or fully covered).
    occluded: bool,
    clock: BackgroundClock,
    /// Background of the last full frame, for animation-only frames.
    background_layer: Option<BackgroundLayer>,
    /// When the next animation frame is due.
    next_frame: Option<Instant>,
    /// Times of full frames, with `KAI_DEVTOOLS=frames`.
    frames: Option<FrameStats>,
    /// Files of a drop: winit sends one `DroppedFile` per file, and they go
    /// to the drop zone together before the loop waits again.
    dropped: Vec<PathBuf>,
    #[cfg(target_os = "macos")]
    menu: Option<crate::menu::NativeMenu>,
}

impl<A: App> WindowState<A> {
    fn new(
        window: Arc<Window>,
        accessibility: accesskit_winit::Adapter,
        renderer: WgpuRenderer,
        theme: Theme,
        app: A,
    ) -> Result<Self, String> {
        let scale_factor = ScaleFactor::new(window.scale_factor())
            .ok_or_else(|| "La ventana informó un factor de escala inválido".to_owned())?;
        let size = window.inner_size();
        let viewport = scale_factor.to_logical_size(size.width, size.height);
        let session = Session::new(
            app,
            theme,
            viewport,
            Measure::Real,
            Box::new(SystemClipboard),
        );
        let unknown = session.unknown_classes();
        if !unknown.is_empty() {
            eprintln!("Clases no reconocidas: {}", unknown.join(" "));
        }
        let frames = session.devtools().frames.then(FrameStats::default);
        let mut state = Self {
            frames,
            window,
            accessibility,
            renderer,
            session,
            scale_factor,
            modifiers: Modifiers::default(),
            clicks: ClickCounter::default(),
            occluded: false,
            clock: BackgroundClock::default(),
            background_layer: None,
            next_frame: None,
            dropped: Vec::new(),
            #[cfg(target_os = "macos")]
            menu: None,
        };
        state.flush();
        Ok(state)
    }

    /// Hand an MCP event to the session: tool calls run as agent calls,
    /// which may wait for confirmation.
    #[cfg(feature = "mcp")]
    fn mcp(&mut self, event: crate::mcp::Event) {
        use crate::mcp::Event;
        match event {
            Event::Connected(client) => self.session.agent_connected(&client),
            Event::Call(invocation) => self.session.agent_call(invocation.into_agent_call()),
            Event::Cancel(id) => self.session.cancel_agent_call(id.0),
            Event::Closed => {}
        }
    }

    /// Apply what the session asked of the window since the last flush.
    fn flush(&mut self) {
        let outbox = self.session.take_outbox();
        if let Some(allowed) = outbox.ime_allowed {
            self.window.set_ime_allowed(allowed);
        }
        if let Some(area) = outbox.ime_cursor_area {
            let scale = self.scale_factor.get();
            self.window.set_ime_cursor_area(
                PhysicalPosition::new(
                    (f64::from(area.x) * scale).round() as i32,
                    (f64::from(area.y) * scale).round() as i32,
                ),
                PhysicalSize::new(
                    ((f64::from(area.width) * scale).round() as u32).max(1),
                    (f64::from(area.height) * scale).round() as u32,
                ),
            );
        }
        if let Some(cursor) = outbox.cursor {
            self.window.set_cursor(match cursor {
                Cursor::Default => CursorIcon::Default,
                Cursor::Pointer => CursorIcon::Pointer,
                Cursor::Text => CursorIcon::Text,
                Cursor::NotAllowed => CursorIcon::NotAllowed,
            });
        }
        if let Some(title) = outbox.title {
            self.window.set_title(&title);
        }
        if let Some(theme) = outbox.theme {
            self.window.set_theme(window_theme(&theme));
        }
        if outbox.redraw {
            self.window.request_redraw();
        }
        #[cfg(target_os = "macos")]
        if let Some(commands) = outbox.commands {
            let model = crate::menu::menu_model(&commands);
            match &mut self.menu {
                Some(menu) => menu.update(model),
                None => {
                    let name = self.window.title();
                    self.menu = Some(crate::menu::NativeMenu::install(&name, model));
                }
            }
        }
        if let Some(request) = outbox.pick_file {
            let path = crate::platform::file_panel::pick(&request);
            self.session.finish_file_pick(path);
            self.flush();
        }
    }

    fn resize(&mut self) {
        let size = self.window.inner_size();
        self.session
            .set_viewport(self.scale_factor.to_logical_size(size.width, size.height));
    }

    fn background_running(&self) -> bool {
        background_running(
            &self.session.background(),
            self.session.theme().prefs.reduce_motion,
            self.occluded,
            self.session.activity(),
        )
    }

    /// Frame for `RedrawRequested`: a full one, or only the next frame of
    /// the background animation when nothing else changed.
    fn redraw(&mut self) -> Result<(), String> {
        if !self.session.is_dirty() && !self.session.animating() && self.background_running() {
            let seconds = self.clock.tick(Instant::now(), true);
            let layer = self.background_layer.map(|layer| layer.at(seconds));
            if self.renderer.redraw_background(layer.as_ref())? {
                return Ok(());
            }
        }
        self.render()
    }

    /// Schedule the next animation frame or timer, or go back to waiting
    /// for events.
    fn control_flow(&mut self) -> ControlFlow {
        let timer = self.session.next_deadline();
        if !self.background_running() && !self.session.animating() {
            self.next_frame = None;
            return timer.map_or(ControlFlow::Wait, ControlFlow::WaitUntil);
        }
        let now = Instant::now();
        let due = *self.next_frame.get_or_insert(now);
        if now >= due {
            self.window.request_redraw();
            self.next_frame = Some(now + FRAME_INTERVAL);
        }
        let frame = self.next_frame.unwrap_or(now + FRAME_INTERVAL);
        ControlFlow::WaitUntil(timer.map_or(frame, |timer| timer.min(frame)))
    }

    fn update_accessibility(&mut self) {
        let session = &self.session;
        let scale = self.scale_factor.get();
        self.accessibility
            .update_if_active(|| session.a11y_tree(scale));
    }

    fn handle_accessibility_event(&mut self, event: accesskit_winit::WindowEvent) {
        match event {
            accesskit_winit::WindowEvent::InitialTreeRequested => self.update_accessibility(),
            accesskit_winit::WindowEvent::ActionRequested(request) => {
                self.session.accessibility_action(&request);
            }
            accesskit_winit::WindowEvent::AccessibilityDeactivated => {}
        }
    }

    fn scroll_wheel(&mut self, delta: MouseScrollDelta) {
        let (delta, lines) = match delta {
            MouseScrollDelta::LineDelta(x, y) => (
                Point {
                    x: x * LINE_SCROLL,
                    y: y * LINE_SCROLL,
                },
                true,
            ),
            MouseScrollDelta::PixelDelta(position) => {
                let scale = self.scale_factor.as_f32();
                (
                    Point {
                        x: position.x as f32 / scale,
                        y: position.y as f32 / scale,
                    },
                    false,
                )
            }
        };
        self.session.wheel(delta, lines);
    }

    fn render(&mut self) -> Result<(), String> {
        self.session.prepare_frame();
        let painting = Instant::now();
        let mut list = self.session.paint_list();
        let paint = painting.elapsed();
        let running = self.background_running();
        let seconds = self.clock.tick(Instant::now(), running);
        list.background = list.background.map(|layer| layer.at(seconds));
        self.background_layer = list.background;
        let rendering = Instant::now();
        let presented =
            self.renderer
                .render(&list, self.session.text_mut(), self.scale_factor.as_f32())?;
        if let Some(frames) = &mut self.frames {
            let times = FrameTimes {
                paint,
                render: rendering.elapsed(),
                ..self.session.frame_times()
            };
            if let Some(summary) = frames.push(times) {
                eprintln!("{summary}");
            }
        }
        if !presented {
            self.session.invalidate();
        }
        self.update_accessibility();
        Ok(())
    }
}

/// Title bar appearance: an AppKit theme forces its own; Kai follows the
/// system, as before.
fn window_theme(theme: &Theme) -> Option<winit::window::Theme> {
    theme.is_appkit().then_some(match theme.appearance {
        Appearance::Light => winit::window::Theme::Light,
        Appearance::Dark => winit::window::Theme::Dark,
    })
}

impl<A: App> ApplicationHandler<UserEvent> for Runner<A> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let Some(app) = self.app.take() else {
            return;
        };
        let minimum = self.theme.minimum_viewport();
        // AccessKit requires the adapter to exist before the window is shown.
        let attrs = Window::default_attributes()
            .with_title(app.title())
            .with_visible(false)
            .with_inner_size(LogicalSize::new(1024.0, 720.0))
            .with_min_inner_size(LogicalSize::new(
                f64::from(minimum.width),
                f64::from(minimum.height),
            ))
            .with_theme(window_theme(&self.theme));
        let window = match event_loop.create_window(attrs) {
            Ok(window) => Arc::new(window),
            Err(error) => {
                eprintln!("No se pudo crear la ventana: {error}");
                event_loop.exit();
                return;
            }
        };
        let accessibility = accesskit_winit::Adapter::with_event_loop_proxy(
            event_loop,
            &window,
            self.proxy.clone(),
        );
        window.set_visible(true);
        let theme = self.theme;
        let state = pollster::block_on(WgpuRenderer::new(Arc::clone(&window)))
            .and_then(|renderer| WindowState::new(window, accessibility, renderer, theme, app));
        match state {
            #[cfg_attr(not(feature = "mcp"), allow(unused_mut))]
            Ok(mut state) => {
                let proxy = Mutex::new(self.proxy.clone());
                state.session.set_task_waker(Box::new(move || {
                    let proxy = proxy.lock().unwrap_or_else(|poison| poison.into_inner());
                    let _ = proxy.send_event(UserEvent::Task);
                }));
                if state.session.run_tasks() {
                    state.flush();
                }
                #[cfg(feature = "mcp")]
                {
                    for event in self.pending.drain(..) {
                        state.mcp(event);
                    }
                    state.flush();
                }
                state.window.request_redraw();
                self.state = Some(state);
            }
            Err(error) => {
                eprintln!("No se pudo iniciar el renderer de Kai: {error}");
                event_loop.exit();
            }
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        let Some(state) = self.state.as_mut() else {
            return;
        };
        state.accessibility.process_event(&state.window, &event);
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => {
                state.renderer.resize(size.width, size.height);
                state.resize();
            }
            WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                if let Some(scale) = ScaleFactor::new(scale_factor) {
                    state.scale_factor = scale;
                    state.resize();
                }
            }
            WindowEvent::MouseWheel { delta, .. } => state.scroll_wheel(delta),
            WindowEvent::CursorMoved { position, .. } => {
                let point = state.scale_factor.to_logical_point(PhysicalPoint {
                    x: position.x,
                    y: position.y,
                });
                state.session.pointer_moved(Some(point));
            }
            WindowEvent::CursorLeft { .. } => state.session.pointer_moved(None),
            WindowEvent::Focused(active) => state.session.set_window_active(active),
            WindowEvent::ModifiersChanged(modifiers) => {
                state.modifiers = adapt_modifiers(modifiers.state());
                state.session.set_modifiers(state.modifiers);
            }
            WindowEvent::PinchGesture { delta, phase, .. } => {
                let ended = matches!(phase, TouchPhase::Ended | TouchPhase::Cancelled);
                state.session.pinch(delta, ended);
            }
            WindowEvent::MouseInput {
                state: button_state,
                button: MouseButton::Left,
                ..
            } => {
                let pressed = button_state == ElementState::Pressed;
                let clicks = if pressed {
                    let point = state.session.cursor().unwrap_or_default();
                    state.clicks.press(Instant::now(), point)
                } else {
                    1
                };
                state.session.pointer_clicks(pressed, clicks);
            }
            WindowEvent::HoveredFile(_) => state.session.hover_files(),
            WindowEvent::HoveredFileCancelled => state.session.cancel_file_drag(),
            WindowEvent::DroppedFile(path) => state.dropped.push(path),
            WindowEvent::Ime(ime) => state.session.apply_intent(adapt_ime_event(ime)),
            WindowEvent::KeyboardInput { event, .. } if event.state == ElementState::Pressed => {
                for intent in adapt_keyboard_event(&event, state.modifiers) {
                    state.session.apply_intent(intent);
                }
            }
            WindowEvent::Occluded(occluded) => {
                state.occluded = occluded;
                if !occluded {
                    state.session.invalidate();
                }
            }
            WindowEvent::RedrawRequested => {
                if let Err(error) = state.redraw() {
                    eprintln!("Error al presentar el frame GPU: {error}");
                    event_loop.exit();
                }
            }
            _ => {}
        }
        state.flush();
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        if let Some(state) = self.state.as_mut() {
            if !state.dropped.is_empty() {
                let paths = std::mem::take(&mut state.dropped);
                state.session.drop_files(paths);
                state.flush();
            }
            if state.session.run_timers() {
                state.flush();
            }
            event_loop.set_control_flow(state.control_flow());
        }
    }

    fn user_event(&mut self, event_loop: &ActiveEventLoop, event: UserEvent) {
        #[cfg(feature = "mcp")]
        let event = match event {
            UserEvent::Mcp(crate::mcp::Event::Closed) => {
                event_loop.exit();
                return;
            }
            UserEvent::Mcp(event) if self.state.is_none() => {
                self.pending.push(event);
                return;
            }
            event => event,
        };
        #[cfg(not(feature = "mcp"))]
        let _ = event_loop;
        let Some(state) = self.state.as_mut() else {
            return;
        };
        match event {
            UserEvent::A11y(event) if state.window.id() == event.window_id => {
                state.handle_accessibility_event(event.window_event);
            }
            UserEvent::A11y(_) => return,
            UserEvent::Menu(id) => {
                state.session.menu_command(&id);
            }
            UserEvent::Task => {
                state.session.run_tasks();
            }
            #[cfg(feature = "mcp")]
            UserEvent::Mcp(event) => state.mcp(event),
        }
        state.flush();
    }
}

#[cfg(test)]
mod tests {
    use super::{
        App, Effects, SimpleApp, action_message, background_running, cursor_for, pointer_target,
        selects_on_focus,
    };
    use crate::background::{BackgroundEffect, BackgroundKind, BackgroundMotion};
    use crate::element::sample::{self, Msg};
    use crate::element::{Element, div, icon, input};
    use crate::geometry::{Point, Size};
    use crate::icons::Icon;
    use crate::layout::WindowActivity;
    use crate::layout::testing::FixedMeasure;
    use crate::layout::{Interaction, Layout, LayoutInput, ScrollOffsets, StyleCache};
    use crate::responder::Action;
    use crate::style::Cursor;
    use crate::theme::{Density, Theme};

    fn layout() -> Layout {
        compute(&sample::tree())
    }

    fn compute(tree: &Element<Msg>) -> Layout {
        Layout::compute(
            tree,
            &LayoutInput {
                viewport: Size {
                    width: 800.0,
                    height: 600.0,
                },
                theme: &Theme::kai(Density::Comfortable),
                interaction: &Interaction::default(),
                scroll: &ScrollOffsets::new(),
            },
            &mut StyleCache::default(),
            &mut FixedMeasure,
        )
    }

    #[derive(Default)]
    struct Counter {
        count: u32,
    }

    impl SimpleApp for Counter {
        type Msg = Msg;

        fn title(&self) -> String {
            format!("{}", self.count)
        }

        fn update(&mut self, msg: Msg) {
            if msg == Msg::Clicked {
                self.count += 1;
            }
        }

        fn view(&self) -> Element<Msg> {
            div().on_click(Msg::Clicked)
        }
    }

    #[test]
    fn simple_app_updates_like_before_with_no_effects() {
        let mut counter = Counter::default();
        assert!(App::init(&mut counter).is_none());
        let effects = App::update(&mut counter, Msg::Clicked);
        assert!(effects.is_none());
        let _ = App::update(&mut counter, Msg::Submitted);
        assert_eq!(counter.count, 1);
        assert_eq!(App::title(&counter), "1");
        assert_eq!(App::theme(&counter), None);
        assert_eq!(App::window_override(&counter), None);
    }

    #[test]
    fn effects_batch_of_none_is_none() {
        let batch = Effects::<Msg>::batch([Effects::none(), Effects::none()]);
        assert!(batch.is_none());
        assert!(Effects::<Msg>::batch([]).is_none());
        assert_eq!(format!("{batch:?}"), "Effects::none()");

        let timers = Effects::batch([
            Effects::after(std::time::Duration::from_secs(1), Msg::Clicked).keyed("uno"),
            Effects::cancel("dos"),
        ]);
        assert!(!timers.is_none());
        assert_eq!(
            format!("{timers:?}"),
            r#"[after(1s, Clicked, key: Some("uno")), cancel("dos")]"#
        );
    }

    #[test]
    fn pointer_targets_bubble_from_text_to_the_enclosing_control() {
        let layout = layout();
        let label = layout.bounds(sample::button_label()).unwrap();
        let path = layout.hit_path(Point {
            x: label.x + 2.0,
            y: label.y + 2.0,
        });
        assert_eq!(path.last(), Some(&sample::button_label()));
        assert_eq!(pointer_target(&layout, &path), Some(sample::button()));

        let disabled = layout.bounds(sample::disabled_button()).unwrap();
        let path = layout.hit_path(Point {
            x: disabled.x + 1.0,
            y: disabled.y + 1.0,
        });
        assert_eq!(
            pointer_target(&layout, &path),
            Some(sample::disabled_button())
        );
        let heading = layout.bounds(sample::heading()).unwrap();
        let path = layout.hit_path(Point {
            x: heading.x + 1.0,
            y: heading.y + 1.0,
        });
        assert_eq!(pointer_target(&layout, &path), None);
    }

    #[test]
    fn outlines_delegate_pointer_focus_to_their_first_enabled_input() {
        let mut tree: Element<Msg> = div().class("flex flex-col").children([
            div()
                .class("flex h-10 px-3 border")
                .delegates_focus()
                .test_id("contorno")
                .children([
                    icon(Icon::Search).class("size-5").test_id("lupa"),
                    input("", "Buscar").class("flex-1").test_id("buscar"),
                ]),
            div()
                .class("flex h-10")
                .delegates_focus()
                .test_id("inactivo")
                .child(input("", "Inactivo").disabled(true)),
            input("abc", "Rápido")
                .select_on_focus(true)
                .test_id("rapido"),
        ]);
        tree.assign_ids();
        let id = |test_id| tree.find_test_id(test_id).unwrap().id;
        let (outline, glyph, search) = (id("contorno"), id("lupa"), id("buscar"));
        let layout = compute(&tree);
        let bounds = layout.bounds(glyph).unwrap();
        let path = layout.hit_path(Point {
            x: bounds.x + 1.0,
            y: bounds.y + 1.0,
        });
        assert_eq!(path.last(), Some(&glyph));
        assert_eq!(pointer_target(&layout, &path), Some(search));

        let idle = id("inactivo");
        let bounds = layout.bounds(idle).unwrap();
        let path = layout.hit_path(Point {
            x: bounds.x + bounds.width - 1.0,
            y: bounds.y + 1.0,
        });
        assert_eq!(path.last(), Some(&idle));
        assert_eq!(pointer_target(&layout, &path), None, "no enabled input");

        assert!(selects_on_focus(&tree, id("rapido")));
        assert!(!selects_on_focus(&tree, search));
        assert!(!selects_on_focus(&tree, outline));
    }

    #[test]
    fn action_bubbles_to_first_handler() {
        let mut tree: Element<Msg> = div()
            .on_action(|action| (action == Action::CancelOperation).then_some(Msg::Clicked))
            .child(
                div()
                    .on_action(|action| (action == Action::Copy).then_some(Msg::Submitted))
                    .child(input("", "Campo")),
            );
        tree.assign_ids();
        let field = tree.children[0].children[0].id;
        assert_eq!(
            action_message(&tree, Some(field), Action::Copy),
            Some(Msg::Submitted)
        );
        assert_eq!(
            action_message(&tree, Some(field), Action::CancelOperation),
            Some(Msg::Clicked),
            "skips the inner handler that returns None"
        );
        assert_eq!(action_message(&tree, Some(field), Action::Undo), None);
        assert_eq!(
            action_message(&tree, None, Action::CancelOperation),
            None,
            "without focus only the window and the app respond"
        );
    }

    /// Animation frames only for a moving effect in a visible, active
    /// window without reduced motion; static effects never ask for frames.
    #[test]
    fn background_frames_run_only_when_animated_visible_and_active() {
        let active = WindowActivity::ACTIVE;
        let moving =
            BackgroundEffect::new(BackgroundKind::Aurora).with_motion(BackgroundMotion::Animated);
        assert!(background_running(&moving, false, false, active));
        assert!(
            !background_running(&moving, true, false, active),
            "reduced motion"
        );
        assert!(
            !background_running(&moving, false, true, active),
            "occluded"
        );
        assert!(!background_running(
            &moving,
            false,
            false,
            WindowActivity::INACTIVE
        ));
        let still = BackgroundEffect::new(BackgroundKind::Aurora);
        assert!(!background_running(&still, false, false, active));
        assert!(!background_running(
            &BackgroundEffect::NONE,
            false,
            false,
            active
        ));
    }

    #[test]
    fn cursor_comes_from_classes_or_inputs() {
        let layout = layout();
        let field = layout.bounds(sample::field()).unwrap();
        let path = layout.hit_path(Point {
            x: field.x + 5.0,
            y: field.y + 5.0,
        });
        assert_eq!(cursor_for(&layout, &path), Cursor::Text);
        assert_eq!(cursor_for(&layout, &[sample::root()]), Cursor::Default);
    }
}
