//! The window-independent half of the runtime: the app, its tree and
//! layout, focus/hover/pressed state, scroll offsets and input editors.
//!
//! `runtime::WindowState` feeds it platform events in logical units and
//! applies its [`Outbox`] to the real window after each one; tests drive
//! the same `Session` through `kai::testing::TestApp` without a window or
//! GPU. Nothing here knows about winit.

use std::collections::{BTreeMap, HashMap};

use accesskit::{ActionRequest, TreeUpdate};
use cosmic_text::{FontSystem, Metrics};

use crate::a11y;
use crate::background::BackgroundEffect;
use crate::element::{Element, FocusKind, FocusOrder, Handlers, Kind, WidgetId};
use crate::geometry::{Point, Rect, Size};
use crate::layout::testing::FixedMeasure;
use crate::layout::{
    Interaction, Layout, LayoutInput, ScrollOffsets, StyleCache, TextMeasure, WindowActivity,
    text_style,
};
use crate::render::paint::{PaintInput, paint};
use crate::render::text::TextStore;
use crate::render::{DrawList, TextId};
use crate::runtime::{
    App, action_message, build_view, cursor_for, pointer_target, selects_on_focus,
};
use crate::style::Cursor;
use crate::text_field::{ClipboardPort, TextFieldDecorations, TextFieldState};
use crate::theme::Theme;
use crate::ui_engine::{Intent, Key, TextIntent, UiModel, dispatch, sync_focus};

/// Extra room kept visible around a focused element when scrolling to it
/// (focus rings are 2px wide with a 2px offset).
pub(crate) const FOCUS_REVEAL_MARGIN: f32 = 4.0;

/// How layout measures text: with the fonts (`Real`, as on screen) or with
/// `FixedMeasure` (tests, no font metrics involved).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Measure {
    Real,
    Fixed,
}

/// What the window has to do after an event. Every field is the latest
/// request since the last [`Session::take_outbox`].
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct Outbox {
    pub redraw: bool,
    pub ime_allowed: Option<bool>,
    /// Caret line of the focused input, in logical points.
    pub ime_cursor_area: Option<Rect>,
    pub cursor: Option<Cursor>,
    pub title: Option<String>,
    pub theme: Option<Theme>,
}

pub(crate) struct Session<A: App> {
    app: A,
    theme: Theme,
    title: String,
    tree: Element<A::Msg>,
    handlers: HashMap<WidgetId, Handlers<A::Msg>>,
    focus_order: FocusOrder,
    styles: StyleCache,
    text: TextStore,
    text_fields: BTreeMap<WidgetId, TextFieldState>,
    /// Field decorations of the last prepared frame, for `paint_list`.
    decorations: Vec<(WidgetId, TextFieldDecorations)>,
    layout: Layout,
    scroll: ScrollOffsets,
    viewport: Size,
    cursor: Option<Point>,
    hover_path: Vec<WidgetId>,
    ui: UiModel,
    dragging: Option<WidgetId>,
    /// Key/main state of the window.
    activity: WindowActivity,
    /// The app's background effect as of the last `view`.
    background: BackgroundEffect,
    /// Something besides the background animation needs a full frame.
    dirty: bool,
    clipboard: Box<dyn ClipboardPort>,
    measure: Measure,
    /// Every message sent to `update`, when recording.
    log: Option<Vec<A::Msg>>,
    outbox: Outbox,
}

impl<A: App> Session<A> {
    /// Run `init`, the first `view` and layout.
    pub fn new(
        mut app: A,
        theme: Theme,
        viewport: Size,
        measure: Measure,
        clipboard: Box<dyn ClipboardPort>,
    ) -> Self {
        let _effects = app.init();
        let tree = build_view(&app);
        let mut session = Self {
            theme,
            title: app.title(),
            app,
            focus_order: FocusOrder::from_tree(&tree),
            handlers: HashMap::new(),
            tree,
            styles: StyleCache::default(),
            text: TextStore::new(FontSystem::new()),
            text_fields: BTreeMap::new(),
            decorations: Vec::new(),
            layout: Layout::default(),
            scroll: ScrollOffsets::new(),
            viewport,
            cursor: None,
            hover_path: Vec::new(),
            ui: UiModel::default(),
            dragging: None,
            activity: WindowActivity::default(),
            background: BackgroundEffect::NONE,
            dirty: true,
            clipboard,
            measure,
            log: None,
            outbox: Outbox::default(),
        };
        session.rebuild();
        session.relayout();
        session
    }

    pub fn app(&self) -> &A {
        &self.app
    }

    pub fn theme(&self) -> &Theme {
        &self.theme
    }

    pub fn tree(&self) -> &Element<A::Msg> {
        &self.tree
    }

    pub fn layout(&self) -> &Layout {
        &self.layout
    }

    pub fn scroll(&self) -> &ScrollOffsets {
        &self.scroll
    }

    pub fn viewport(&self) -> Size {
        self.viewport
    }

    pub fn ui(&self) -> &UiModel {
        &self.ui
    }

    pub fn focused(&self) -> Option<WidgetId> {
        *self.ui.focused.get()
    }

    pub fn background(&self) -> BackgroundEffect {
        self.background
    }

    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    pub fn text_mut(&mut self) -> &mut TextStore {
        &mut self.text
    }

    /// Classes no style rule recognized, so far.
    pub fn unknown_classes(&self) -> Vec<&str> {
        self.styles.unknown()
    }

    /// Current contents of input `id`'s editor (what the user typed, even
    /// before the app echoes it back).
    pub fn field_value(&self, id: WidgetId) -> Option<String> {
        self.text_fields.get(&id).map(TextFieldState::text)
    }

    /// Start recording every message sent to `update`.
    pub fn record_messages(&mut self) {
        self.log.get_or_insert_with(Vec::new);
    }

    pub fn messages(&self) -> &[A::Msg] {
        self.log.as_deref().unwrap_or_default()
    }

    pub fn take_outbox(&mut self) -> Outbox {
        std::mem::take(&mut self.outbox)
    }

    /// Scroll so `id` (plus the focus ring margin) is visible; returns
    /// whether some offset changed.
    pub fn reveal(&mut self, id: WidgetId) -> bool {
        let changed = self
            .layout
            .reveal(id, FOCUS_REVEAL_MARGIN, &mut self.scroll);
        if changed {
            self.invalidate();
        }
        changed
    }

    /// Window content size in logical points.
    pub fn set_viewport(&mut self, viewport: Size) {
        self.viewport = viewport;
        self.invalidate();
    }

    /// Run `view`, then reconcile focus and input editors with the new tree.
    fn rebuild(&mut self) {
        self.tree = build_view(&self.app);
        self.background = self.app.background();
        if let Some(theme) = self.app.theme()
            && theme != self.theme
        {
            self.theme = theme;
            self.outbox.theme = Some(theme);
        }
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
            self.outbox.title = Some(title.clone());
            self.title = title;
        }
    }

    /// The real key/main state unless the app forces one.
    pub fn activity(&self) -> WindowActivity {
        self.app.window_override().unwrap_or(self.activity)
    }

    fn relayout(&mut self) {
        let interaction = self.interaction();
        let mut fixed = FixedMeasure;
        let measure: &mut dyn TextMeasure = match self.measure {
            Measure::Real => &mut self.text,
            Measure::Fixed => &mut fixed,
        };
        self.layout = Layout::compute(
            &self.tree,
            &LayoutInput {
                viewport: self.viewport,
                theme: &self.theme,
                interaction: &interaction,
                scroll: &self.scroll,
            },
            &mut self.styles,
            measure,
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

    /// Hover, focus, pressed and window state as layout and paint see them.
    pub fn interaction(&self) -> Interaction {
        Interaction {
            hovered: self.hover_path.clone(),
            focused: *self.ui.focused.get(),
            pressed: *self.ui.pressed.get(),
            window: self.activity(),
            focus_visible: *self.ui.focus_visible.get(),
        }
    }

    /// Ask for a full frame (view, layout, paint).
    pub fn invalidate(&mut self) {
        self.dirty = true;
        self.outbox.redraw = true;
    }

    fn send(&mut self, msg: A::Msg) {
        if let Some(log) = &mut self.log {
            log.push(msg.clone());
        }
        let _effects = self.app.update(msg);
        self.invalidate();
    }

    fn handler(&self, id: WidgetId) -> Option<&Handlers<A::Msg>> {
        self.handlers.get(&id)
    }

    /// AccessKit tree for the current layout, scaled to physical pixels.
    pub fn a11y_tree(&self, scale_factor: f64) -> TreeUpdate {
        let values: Vec<(WidgetId, String)> = self
            .text_fields
            .iter()
            .map(|(id, field)| (*id, field.text()))
            .collect();
        let field_values: Vec<(WidgetId, &str)> = values
            .iter()
            .map(|(id, value)| (*id, value.as_str()))
            .collect();
        a11y::a11y_tree(&a11y::A11ySnapshot {
            layout: &self.layout,
            scale_factor,
            focused: *self.ui.focused.get(),
            window_title: &self.title,
            field_values: &field_values,
        })
    }

    pub fn accessibility_action(&mut self, request: &ActionRequest) {
        for intent in a11y::intents_for_action(request, &self.focus_order) {
            self.apply_intent(intent);
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
        self.outbox.cursor = Some(cursor_for(&self.layout, &path));
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
        self.outbox.ime_cursor_area = Some(Rect {
            x: line.x + caret.x,
            y: line.y + caret.y,
            width: 0.0,
            height: line.height,
        });
    }

    fn text_field_local(&self, id: WidgetId) -> Option<Point> {
        self.cursor
            .and_then(|point| self.layout.text_local_position(id, point))
    }

    pub fn apply_intent(&mut self, intent: Intent) {
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
            self.outbox.ime_allowed = Some(text_field.is_some());
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
        let effect = field.apply(intent, &mut self.text.font_system, &mut *self.clipboard);
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

    /// Scroll the innermost scroll container under the pointer by `delta_y`
    /// logical points (positive scrolls content up, like a wheel down).
    pub fn scroll_by(&mut self, delta_y: f32) {
        let Some((id, scroll)) = self.layout.scroll_target(&self.hover_path, -delta_y) else {
            return;
        };
        let next = (scroll.offset - delta_y).clamp(0.0, scroll.max);
        if next != scroll.offset {
            self.scroll.insert(id, next);
            self.invalidate();
        }
    }

    /// Pointer at `point` in logical points, or gone (`None`): updates hover
    /// and extends a selection drag in an input.
    pub fn pointer_moved(&mut self, point: Option<Point>) {
        self.cursor = point;
        if point.is_none() {
            self.refresh_hover();
            self.apply_intent(Intent::CancelPointer);
            self.dragging = None;
            self.invalidate();
            return;
        }
        let mut redraw = self.refresh_hover();
        if let Some(id) = self.dragging
            && let Some(local) = self.text_field_local(id)
            && let Some(field) = self.text_fields.get_mut(&id)
        {
            field.drag(&mut self.text.font_system, local);
            self.update_ime_cursor_area();
            redraw = true;
        }
        if redraw {
            self.invalidate();
        }
    }

    /// Primary button pressed or released at the current pointer.
    pub fn pointer_button(&mut self, pressed: bool) {
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

    /// The window became key (`true`) or lost it.
    pub fn set_window_active(&mut self, active: bool) {
        if active {
            self.outbox.ime_allowed = Some(self.focused_text_field().is_some());
            self.update_ime_cursor_area();
            self.activity = WindowActivity::ACTIVE;
        } else {
            self.apply_intent(Intent::CancelPointer);
            self.dragging = None;
            for field in self.text_fields.values_mut() {
                field.set_preedit(&mut self.text.font_system, None);
            }
            self.outbox.ime_allowed = Some(false);
            // Variants and paint read the activity; no need to re-run
            // `view`.
            self.activity = WindowActivity::INACTIVE;
        }
        self.invalidate();
    }

    /// Everything a frame needs before paint: `view`, layout, hover, scroll
    /// offsets and the text of every input.
    pub fn prepare_frame(&mut self) {
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

        self.decorations.clear();
        for (id, field) in &mut self.text_fields {
            let Some(node) = self.layout.node(*id) else {
                continue;
            };
            let view = field.view(&mut self.text.font_system);
            self.text
                .set_buffer(TextId::value(*id), view.buffer, text_style(&node.text));
            self.decorations.push((*id, view.decorations));
        }
    }

    /// Draw list of the last prepared frame; the background layer is not
    /// yet advanced to any point of its animation.
    pub fn paint_list(&self) -> DrawList {
        let text_fields: Vec<(WidgetId, &TextFieldDecorations)> = self
            .decorations
            .iter()
            .map(|(id, decorations)| (*id, decorations))
            .collect();
        paint(&PaintInput {
            layout: &self.layout,
            theme: &self.theme,
            focused: *self.ui.focused.get(),
            window: self.activity(),
            text_fields: &text_fields,
            background: self.background,
        })
    }
}
