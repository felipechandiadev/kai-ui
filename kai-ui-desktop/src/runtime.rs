//! Elm-style runtime: `view` builds an element tree from the model, events
//! become messages, `update` changes the model, and the next frame renders
//! the new `view`.
//!
//! The runtime owns everything that is not app state: window, renderer,
//! text buffers, focus/hover/pressed state ([`UiModel`]), scroll offsets and
//! the caret/selection of each input. Inputs are controlled: every frame the
//! value in the tree is pushed into the editor when it differs, so an input
//! without `on_input` is effectively read-only.
//!
//! The loop waits for events (`ControlFlow::Wait`): nothing redraws on its
//! own, except an animated [`App::background`], which asks for frames at
//! [`FRAME_INTERVAL`] only while the window is visible, active and motion
//! is not reduced. Those frames reuse the last prepared draws unless
//! something else changed (`dirty`).

use std::collections::{BTreeMap, HashMap};
use std::error::Error;
use std::sync::Arc;
use std::time::Instant;

use cosmic_text::{FontSystem, Metrics};
use winit::{
    application::ApplicationHandler,
    dpi::{LogicalSize, PhysicalPosition, PhysicalSize},
    event::{ElementState, MouseButton, MouseScrollDelta, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop, EventLoopProxy},
    window::{CursorIcon, Window, WindowId},
};

use crate::a11y;
use crate::background::{BackgroundClock, BackgroundEffect, BackgroundLayer, FRAME_INTERVAL};
use crate::element::{Element, FocusKind, FocusOrder, Handlers, Kind, WidgetId};
use crate::geometry::{PhysicalPoint, Point, ScaleFactor, Size};
use crate::layout::{
    Content, Interaction, Layout, LayoutInput, ScrollOffsets, StyleCache, WindowActivity,
    text_style,
};
use crate::platform_adapter::{adapt_ime_event, adapt_keyboard_event, adapt_modifiers};
use crate::render::TextId;
use crate::render::paint::{PaintInput, paint};
use crate::render::text::TextStore;
use crate::render::wgpu_backend::WgpuRenderer;
use crate::responder::{self, Action, Responder};
use crate::style::Cursor;
use crate::text_field::{SystemClipboard, TextFieldDecorations, TextFieldState};
use crate::theme::{Appearance, Theme};
use crate::ui_engine::{Intent, Key, Modifiers, TextIntent, UiModel, dispatch, sync_focus};

/// Extra room kept visible around a focused element when scrolling to it
/// (focus rings are 2px wide with a 2px offset).
const FOCUS_REVEAL_MARGIN: f32 = 4.0;
const LINE_SCROLL: f32 = 40.0;

pub trait App: 'static {
    type Msg: Clone + 'static;

    fn title(&self) -> String;
    fn update(&mut self, msg: Self::Msg);
    fn view(&self) -> Element<Self::Msg>;

    /// Window background effect for the current model, read after every
    /// `update` (so it can change live). Default: the solid window color.
    fn background(&self) -> BackgroundEffect {
        BackgroundEffect::NONE
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

/// Build the tree for the current model with ids assigned.
pub fn build_view<A: App>(app: &A) -> Element<A::Msg> {
    let mut tree = app.view();
    tree.assign_ids();
    tree
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
    theme: Theme,
    app: A,
    title: String,
    tree: Element<A::Msg>,
    handlers: HashMap<WidgetId, Handlers<A::Msg>>,
    focus_order: FocusOrder,
    styles: StyleCache,
    text: TextStore,
    text_fields: BTreeMap<WidgetId, TextFieldState>,
    layout: Layout,
    scroll: ScrollOffsets,
    scale_factor: ScaleFactor,
    cursor: Option<Point>,
    hover_path: Vec<WidgetId>,
    modifiers: Modifiers,
    ui: UiModel,
    dragging: Option<WidgetId>,
    /// Key/main state, from `WindowEvent::Focused`.
    activity: WindowActivity,
    /// `WindowEvent::Occluded` (hidden, minimized or fully covered).
    occluded: bool,
    /// The app's background effect as of the last `view`.
    background: BackgroundEffect,
    clock: BackgroundClock,
    /// Background of the last full frame, for animation-only frames.
    background_layer: Option<BackgroundLayer>,
    /// Something besides the background animation needs a full frame.
    dirty: bool,
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
        let tree = build_view(&app);
        let mut state = Self {
            window,
            accessibility,
            renderer,
            theme,
            title: app.title(),
            app,
            focus_order: FocusOrder::from_tree(&tree),
            handlers: HashMap::new(),
            tree,
            styles: StyleCache::default(),
            text: TextStore::new(FontSystem::new()),
            text_fields: BTreeMap::new(),
            layout: Layout::default(),
            scroll: ScrollOffsets::new(),
            scale_factor,
            cursor: None,
            hover_path: Vec::new(),
            modifiers: Modifiers::default(),
            ui: UiModel::default(),
            dragging: None,
            activity: WindowActivity::default(),
            occluded: false,
            background: BackgroundEffect::NONE,
            clock: BackgroundClock::default(),
            background_layer: None,
            dirty: true,
            next_frame: None,
        };
        state.rebuild();
        state.relayout();
        let unknown = state.styles.unknown();
        if !unknown.is_empty() {
            eprintln!("Clases no reconocidas: {}", unknown.join(" "));
        }
        Ok(state)
    }

    fn viewport(&self) -> Size {
        let physical = self.window.inner_size();
        self.scale_factor
            .to_logical_size(physical.width, physical.height)
    }

    /// Run `view`, then reconcile focus and input editors with the new tree.
    fn rebuild(&mut self) {
        self.tree = build_view(&self.app);
        self.background = self.app.background();
        self.focus_order = FocusOrder::from_tree(&self.tree);
        sync_focus(&mut self.ui, &self.focus_order);
        self.handlers = self.tree.collect_handlers().into_iter().collect();

        let font_system = &mut self.text.font_system;
        let fields = &mut self.text_fields;
        let mut live = Vec::new();
        self.tree.walk(&mut |element| {
            if let Kind::Input {
                value,
                masked,
                read_only,
                ..
            } = &element.kind
            {
                let field = fields
                    .entry(element.id)
                    .or_insert_with(|| TextFieldState::new(font_system, Metrics::new(16.0, 24.0)));
                field.set_masked(font_system, *masked);
                field.set_read_only(font_system, *read_only);
                if field.preedit().is_none() {
                    field.set_text(font_system, value);
                }
                live.push(element.id);
            }
        });
        fields.retain(|id, _| live.contains(id));
        if self.dragging.is_some_and(|id| !live.contains(&id)) {
            self.dragging = None;
        }

        let title = self.app.title();
        if title != self.title {
            self.window.set_title(&title);
            self.title = title;
        }
    }

    fn relayout(&mut self) {
        let interaction = Interaction {
            hovered: self.hover_path.clone(),
            focused: *self.ui.focused.get(),
            pressed: *self.ui.pressed.get(),
            window: self.activity,
            focus_visible: *self.ui.focus_visible.get(),
        };
        self.layout = Layout::compute(
            &self.tree,
            &LayoutInput {
                viewport: self.viewport(),
                theme: &self.theme,
                interaction: &interaction,
                scroll: &self.scroll,
            },
            &mut self.styles,
            &mut self.text,
        );
        let layout = &self.layout;
        self.focus_order
            .retain_visible_equivalents(|id| layout.node(id).is_some_and(|node| !node.hidden));
        for (id, field) in &mut self.text_fields {
            if let Some(node) = self.layout.node(*id) {
                let font_system = &mut self.text.font_system;
                field.set_metrics(
                    font_system,
                    Metrics::new(node.text.font_size, node.text.line_height),
                );
                if let Some(line) = node.text_box {
                    field.set_width(font_system, line.width);
                }
            }
        }
    }

    /// Ask for a full frame (view, layout, paint).
    fn invalidate(&mut self) {
        self.dirty = true;
        self.window.request_redraw();
    }

    fn background_running(&self) -> bool {
        background_running(
            &self.background,
            self.theme.prefs.reduce_motion,
            self.occluded,
            self.activity,
        )
    }

    /// Frame for `RedrawRequested`: a full one, or only the next frame of
    /// the background animation when nothing else changed.
    fn redraw(&mut self) -> Result<(), String> {
        if !self.dirty && self.background_running() {
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

    fn send(&mut self, msg: A::Msg) {
        self.app.update(msg);
        self.invalidate();
    }

    fn handler(&self, id: WidgetId) -> Option<&Handlers<A::Msg>> {
        self.handlers.get(&id)
    }

    fn update_accessibility(&mut self) {
        let values: Vec<(WidgetId, String)> = self
            .text_fields
            .iter()
            .map(|(id, field)| (*id, field.text()))
            .collect();
        let field_values: Vec<(WidgetId, &str)> = values
            .iter()
            .map(|(id, value)| (*id, value.as_str()))
            .collect();
        let snapshot = a11y::A11ySnapshot {
            layout: &self.layout,
            scale_factor: self.scale_factor.get(),
            focused: *self.ui.focused.get(),
            window_title: &self.title,
            field_values: &field_values,
        };
        self.accessibility
            .update_if_active(|| a11y::a11y_tree(&snapshot));
    }

    fn handle_accessibility_event(&mut self, event: accesskit_winit::WindowEvent) {
        match event {
            accesskit_winit::WindowEvent::InitialTreeRequested => self.update_accessibility(),
            accesskit_winit::WindowEvent::ActionRequested(request) => {
                for intent in a11y::intents_for_action(&request, &self.focus_order) {
                    self.apply_intent(intent);
                }
            }
            accesskit_winit::WindowEvent::AccessibilityDeactivated => {}
        }
    }

    /// Recompute the hover path from the current layout; returns whether it
    /// changed.
    fn refresh_hover(&mut self) -> bool {
        let path = self
            .cursor
            .map(|point| self.layout.hit_path(point))
            .unwrap_or_default();
        if path == self.hover_path {
            return false;
        }
        self.window
            .set_cursor(match cursor_for(&self.layout, &path) {
                Cursor::Default => CursorIcon::Default,
                Cursor::Pointer => CursorIcon::Pointer,
                Cursor::Text => CursorIcon::Text,
                Cursor::NotAllowed => CursorIcon::NotAllowed,
            });
        self.hover_path = path;
        true
    }

    fn is_text_field(&self, id: Option<WidgetId>) -> bool {
        id.is_some_and(|id| self.focus_order.kind(id) == Some(FocusKind::TextField))
    }

    fn focused_text_field(&self) -> Option<WidgetId> {
        let focused = *self.ui.focused.get();
        focused.filter(|_| self.is_text_field(focused))
    }

    fn update_ime_cursor_area(&mut self) {
        let Some(id) = self.focused_text_field() else {
            return;
        };
        let (Some(line), Some(field)) = (self.layout.text_box(id), self.text_fields.get_mut(&id))
        else {
            return;
        };
        let caret = field
            .view(&mut self.text.font_system)
            .decorations
            .caret
            .map_or(Point::default(), |caret| Point {
                x: caret.x,
                y: caret.y,
            });
        let scale = self.scale_factor.get();
        self.window.set_ime_cursor_area(
            PhysicalPosition::new(
                (f64::from(line.x + caret.x) * scale).round() as i32,
                (f64::from(line.y + caret.y) * scale).round() as i32,
            ),
            PhysicalSize::new(1_u32, (f64::from(line.height) * scale).round() as u32),
        );
    }

    fn text_field_local(&self, id: WidgetId) -> Option<Point> {
        self.cursor
            .and_then(|point| self.layout.text_local_position(id, point))
    }

    fn apply_intent(&mut self, intent: Intent) {
        let previous_focus = *self.ui.focused.get();
        let previous_text_field = self.focused_text_field();
        let result = dispatch(&mut self.ui, &self.focus_order, intent);
        let current_focus = *self.ui.focused.get();
        if previous_focus != current_focus {
            if let Some(id) = current_focus {
                self.layout
                    .reveal(id, FOCUS_REVEAL_MARGIN, &mut self.scroll);
            }
            let text_field = self.focused_text_field();
            self.window.set_ime_allowed(text_field.is_some());
            if let Some(id) = text_field {
                if selects_on_focus(&self.tree, id)
                    && let Some(field) = self.text_fields.get_mut(&id)
                {
                    field.select_all(&mut self.text.font_system);
                }
                self.update_ime_cursor_area();
            }
            if let Some(previous) = previous_text_field
                && Some(previous) != text_field
            {
                self.dragging = None;
                if let Some(field) = self.text_fields.get_mut(&previous) {
                    field.set_preedit(&mut self.text.font_system, None);
                }
            }
        }
        for (id, action) in result.text_actions {
            self.apply_text_intent(id, action);
        }
        if let Some(activation) = result.activated
            && let Some(msg) = self
                .handler(activation.widget)
                .and_then(|handlers| handlers.on_click.clone())
        {
            self.send(msg);
        }
        if let Some(action) = result.action
            && let Some(msg) = action_message(&self.tree, current_focus, action)
        {
            self.send(msg);
        }
        if result.changed || current_focus != previous_focus {
            self.invalidate();
        }
    }

    fn apply_text_intent(&mut self, id: WidgetId, intent: TextIntent) {
        let submit = matches!(
            intent,
            TextIntent::KeyDown {
                key: Key::Enter,
                repeat: false,
                ..
            }
        );
        let Some(field) = self.text_fields.get_mut(&id) else {
            return;
        };
        let before = field.text();
        let effect = field.apply(intent, &mut self.text.font_system, &mut SystemClipboard);
        let after = field.text();
        if let Some(status) = effect.status {
            eprintln!("{status}");
        }
        if effect.changed {
            self.update_ime_cursor_area();
            self.invalidate();
        }
        let handlers = self.handler(id).cloned().unwrap_or_default();
        if after != before
            && let Some(on_input) = handlers.on_input
        {
            self.send(on_input(after));
        }
        if submit && let Some(msg) = handlers.on_submit {
            self.send(msg);
        }
    }

    fn scroll_wheel(&mut self, delta: MouseScrollDelta) {
        let delta_y = match delta {
            MouseScrollDelta::LineDelta(_, lines) => lines * LINE_SCROLL,
            MouseScrollDelta::PixelDelta(position) => {
                position.y as f32 / self.scale_factor.as_f32()
            }
        };
        let Some((id, scroll)) = self.layout.scroll_target(&self.hover_path, -delta_y) else {
            return;
        };
        let next = (scroll.offset - delta_y).clamp(0.0, scroll.max);
        if next != scroll.offset {
            self.scroll.insert(id, next);
            self.invalidate();
        }
    }

    fn render(&mut self) -> Result<(), String> {
        self.dirty = false;
        self.rebuild();
        self.relayout();
        if self.refresh_hover() {
            self.relayout();
        }
        for (id, scroll) in self
            .layout
            .nodes()
            .iter()
            .filter_map(|n| n.scroll.map(|s| (n.id, s)))
        {
            self.scroll.insert(id, scroll.offset);
        }
        self.scroll.retain(|id, _| {
            self.layout
                .node(*id)
                .is_some_and(|node| node.scroll.is_some())
        });

        let mut decorations: Vec<(WidgetId, TextFieldDecorations)> = Vec::new();
        for (id, field) in &mut self.text_fields {
            let Some(node) = self.layout.node(*id) else {
                continue;
            };
            let view = field.view(&mut self.text.font_system);
            self.text
                .set_buffer(TextId::value(*id), view.buffer, text_style(&node.text));
            decorations.push((*id, view.decorations));
        }
        let text_fields: Vec<(WidgetId, &TextFieldDecorations)> = decorations
            .iter()
            .map(|(id, decorations)| (*id, decorations))
            .collect();
        let list = paint(&PaintInput {
            layout: &self.layout,
            theme: &self.theme,
            focused: *self.ui.focused.get(),
            window: self.activity,
            text_fields: &text_fields,
            background: self.background,
        });
        let mut list = list;
        let running = self.background_running();
        let seconds = self.clock.tick(Instant::now(), running);
        list.background = list.background.map(|layer| layer.at(seconds));
        self.background_layer = list.background;
        let presented = self
            .renderer
            .render(&list, &mut self.text, self.scale_factor.as_f32())?;
        if !presented {
            self.invalidate();
        }
        self.update_accessibility();
        Ok(())
    }

    fn pointer_input(&mut self, pressed: bool) {
        let target = pointer_target(&self.layout, &self.hover_path);
        if pressed {
            let previous_focus = *self.ui.focused.get();
            self.apply_intent(Intent::PointerDown(target));
            let selected_on_focus = target
                .is_some_and(|id| previous_focus != Some(id) && selects_on_focus(&self.tree, id));
            self.dragging = target.filter(|_| self.is_text_field(target) && !selected_on_focus);
            if let Some(id) = self.dragging
                && let Some(local) = self.text_field_local(id)
                && let Some(field) = self.text_fields.get_mut(&id)
            {
                field.click(&mut self.text.font_system, local);
                self.update_ime_cursor_area();
            }
        } else {
            self.apply_intent(Intent::PointerUp(target));
            self.dragging = None;
        }
        self.invalidate();
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
                state.invalidate();
            }
            WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                if let Some(scale) = ScaleFactor::new(scale_factor) {
                    state.scale_factor = scale;
                    state.invalidate();
                }
            }
            WindowEvent::MouseWheel { delta, .. } => state.scroll_wheel(delta),
            WindowEvent::CursorMoved { position, .. } => {
                state.cursor = Some(state.scale_factor.to_logical_point(PhysicalPoint {
                    x: position.x,
                    y: position.y,
                }));
                let mut redraw = state.refresh_hover();
                if let Some(id) = state.dragging
                    && let Some(local) = state.text_field_local(id)
                    && let Some(field) = state.text_fields.get_mut(&id)
                {
                    field.drag(&mut state.text.font_system, local);
                    state.update_ime_cursor_area();
                    redraw = true;
                }
                if redraw {
                    state.invalidate();
                }
            }
            WindowEvent::CursorLeft { .. } => {
                state.cursor = None;
                state.refresh_hover();
                state.apply_intent(Intent::CancelPointer);
                state.dragging = None;
                state.invalidate();
            }
            WindowEvent::Focused(false) => {
                state.apply_intent(Intent::CancelPointer);
                state.dragging = None;
                for field in state.text_fields.values_mut() {
                    field.set_preedit(&mut state.text.font_system, None);
                }
                state.window.set_ime_allowed(false);
                // Variants and paint read the activity; no need to re-run
                // `view`.
                state.activity = WindowActivity::INACTIVE;
                state.invalidate();
            }
            WindowEvent::Focused(true) => {
                state
                    .window
                    .set_ime_allowed(state.focused_text_field().is_some());
                state.update_ime_cursor_area();
                state.activity = WindowActivity::ACTIVE;
                state.invalidate();
            }
            WindowEvent::ModifiersChanged(modifiers) => {
                state.modifiers = adapt_modifiers(modifiers.state());
            }
            WindowEvent::MouseInput {
                state: button_state,
                button: MouseButton::Left,
                ..
            } => state.pointer_input(button_state == ElementState::Pressed),
            WindowEvent::Ime(ime) => state.apply_intent(adapt_ime_event(ime)),
            WindowEvent::KeyboardInput { event, .. } if event.state == ElementState::Pressed => {
                for intent in adapt_keyboard_event(&event, state.modifiers) {
                    state.apply_intent(intent);
                }
            }
            WindowEvent::Occluded(occluded) => {
                state.occluded = occluded;
                if !occluded {
                    state.invalidate();
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
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{action_message, background_running, cursor_for, pointer_target, selects_on_focus};
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
                .children([
                    icon(Icon::Search).class("size-5"),
                    input("", "Buscar").class("flex-1"),
                ]),
            div()
                .class("flex h-10")
                .delegates_focus()
                .child(input("", "Inactivo").disabled(true)),
            input("abc", "Rápido").select_on_focus(true),
        ]);
        tree.assign_ids();
        let (outline, idle, quick) = (&tree.children[0], &tree.children[1], &tree.children[2]);
        let layout = compute(&tree);
        let glyph = layout.bounds(outline.children[0].id).unwrap();
        let path = layout.hit_path(Point {
            x: glyph.x + 1.0,
            y: glyph.y + 1.0,
        });
        assert_eq!(path.last(), Some(&outline.children[0].id));
        assert_eq!(pointer_target(&layout, &path), Some(outline.children[1].id));

        let bounds = layout.bounds(idle.id).unwrap();
        let path = layout.hit_path(Point {
            x: bounds.x + bounds.width - 1.0,
            y: bounds.y + 1.0,
        });
        assert_eq!(path.last(), Some(&idle.id));
        assert_eq!(pointer_target(&layout, &path), None, "no enabled input");

        assert!(selects_on_focus(&tree, quick.id));
        assert!(!selects_on_focus(&tree, outline.children[1].id));
        assert!(!selects_on_focus(&tree, outline.id));
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
