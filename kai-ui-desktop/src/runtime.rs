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
//! something else changed (`dirty`).

use std::cell::RefCell;
use std::collections::HashSet;
use std::error::Error;
use std::fmt::{self, Debug};
use std::marker::PhantomData;
use std::sync::Arc;
use std::time::Instant;

use winit::{
    application::ApplicationHandler,
    dpi::{LogicalSize, PhysicalPosition, PhysicalSize},
    event::{ElementState, MouseButton, MouseScrollDelta, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop, EventLoopProxy},
    window::{CursorIcon, Window, WindowId},
};

use crate::background::{BackgroundClock, BackgroundEffect, BackgroundLayer, FRAME_INTERVAL};
use crate::commands::CommandRegistry;
use crate::element::{Element, Kind, WidgetId};
use crate::geometry::{PhysicalPoint, ScaleFactor};
use crate::layout::{Content, Layout, WindowActivity};
use crate::platform_adapter::{adapt_ime_event, adapt_keyboard_event, adapt_modifiers};
use crate::render::wgpu_backend::WgpuRenderer;
use crate::responder::{self, Action, Responder};
use crate::session::{Measure, Session};
use crate::style::Cursor;
use crate::text_field::SystemClipboard;
use crate::theme::{Appearance, Theme};
use crate::ui_engine::Modifiers;

const LINE_SCROLL: f32 = 40.0;

/// What `update` asks the runtime to do, as a value. Opaque and always
/// empty until async effects exist (13, Paso 15); the runtime drops it.
#[must_use]
pub struct Effects<Msg>(PhantomData<fn() -> Msg>);

impl<Msg> Effects<Msg> {
    pub fn none() -> Self {
        Self(PhantomData)
    }

    pub fn batch(effects: impl IntoIterator<Item = Self>) -> Self {
        effects.into_iter().for_each(drop);
        Self::none()
    }

    pub fn is_none(&self) -> bool {
        true
    }
}

impl<Msg> fmt::Debug for Effects<Msg> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Effects::none()")
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
    let event_loop = EventLoop::<accesskit_winit::Event>::with_user_event().build()?;
    event_loop.set_control_flow(ControlFlow::Wait);
    let proxy = event_loop.create_proxy();
    let mut runner = Runner {
        app: Some(app),
        theme,
        state: None,
        proxy,
    };
    event_loop.run_app(&mut runner)?;
    Ok(())
}

/// Build the tree for the current model with ids assigned. Debug builds
/// report duplicate keys and test ids (see [`Element::validate_ids`]), each
/// problem once per thread.
pub fn build_view<A: App>(app: &A) -> Element<A::Msg> {
    let mut tree = app.view();
    tree.assign_ids();
    if cfg!(debug_assertions) {
        report_once("Ids inválidos", tree.validate_ids());
    }
    tree
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
/// returns one. The window handles `CancelOperation` by doing nothing; the
/// app end is the hook for the command registry of 09 (today a debug log).
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

struct Runner<A: App> {
    app: Option<A>,
    theme: Theme,
    state: Option<WindowState<A>>,
    proxy: EventLoopProxy<accesskit_winit::Event>,
}

struct WindowState<A: App> {
    window: Arc<Window>,
    accessibility: accesskit_winit::Adapter,
    renderer: WgpuRenderer,
    session: Session<A>,
    scale_factor: ScaleFactor,
    modifiers: Modifiers,
    /// `WindowEvent::Occluded` (hidden, minimized or fully covered).
    occluded: bool,
    clock: BackgroundClock,
    /// Background of the last full frame, for animation-only frames.
    background_layer: Option<BackgroundLayer>,
    /// When the next animation frame is due.
    next_frame: Option<Instant>,
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
        let mut state = Self {
            window,
            accessibility,
            renderer,
            session,
            scale_factor,
            modifiers: Modifiers::default(),
            occluded: false,
            clock: BackgroundClock::default(),
            background_layer: None,
            next_frame: None,
        };
        state.flush();
        Ok(state)
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
        if !self.session.is_dirty() && self.background_running() {
            let seconds = self.clock.tick(Instant::now(), true);
            let layer = self.background_layer.map(|layer| layer.at(seconds));
            if self.renderer.redraw_background(layer.as_ref())? {
                return Ok(());
            }
        }
        self.render()
    }

    /// Schedule the next animation frame, or go back to waiting for events.
    fn control_flow(&mut self) -> ControlFlow {
        if !self.background_running() {
            self.next_frame = None;
            return ControlFlow::Wait;
        }
        let now = Instant::now();
        let due = *self.next_frame.get_or_insert(now);
        if now >= due {
            self.window.request_redraw();
            self.next_frame = Some(now + FRAME_INTERVAL);
        }
        ControlFlow::WaitUntil(self.next_frame.unwrap_or(now + FRAME_INTERVAL))
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
        let delta_y = match delta {
            MouseScrollDelta::LineDelta(_, lines) => lines * LINE_SCROLL,
            MouseScrollDelta::PixelDelta(position) => {
                position.y as f32 / self.scale_factor.as_f32()
            }
        };
        self.session.scroll_by(delta_y);
    }

    fn render(&mut self) -> Result<(), String> {
        self.session.prepare_frame();
        let mut list = self.session.paint_list();
        let running = self.background_running();
        let seconds = self.clock.tick(Instant::now(), running);
        list.background = list.background.map(|layer| layer.at(seconds));
        self.background_layer = list.background;
        let presented =
            self.renderer
                .render(&list, self.session.text_mut(), self.scale_factor.as_f32())?;
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

impl<A: App> ApplicationHandler<accesskit_winit::Event> for Runner<A> {
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
            Ok(state) => {
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
            }
            WindowEvent::MouseInput {
                state: button_state,
                button: MouseButton::Left,
                ..
            } => state
                .session
                .pointer_button(button_state == ElementState::Pressed),
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
            event_loop.set_control_flow(state.control_flow());
        }
    }

    fn user_event(&mut self, _event_loop: &ActiveEventLoop, event: accesskit_winit::Event) {
        if let Some(state) = self.state.as_mut()
            && state.window.id() == event.window_id
        {
            state.handle_accessibility_event(event.window_event);
            state.flush();
        }
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
