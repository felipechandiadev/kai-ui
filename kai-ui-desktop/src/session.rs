//! The window-independent half of the runtime: the app, its tree and
//! layout, focus/hover/pressed state, scroll offsets and input editors.
//!
//! `runtime::WindowState` feeds it platform events in logical units and
//! applies its [`Outbox`] to the real window after each one; tests drive
//! the same `Session` through `kai::testing::TestApp` without a window or
//! GPU. Nothing here knows about winit.

mod calls;

use std::collections::{BTreeMap, HashMap};
use std::time::Instant;

use crate::a11y;
use crate::background::BackgroundEffect;
use crate::commands::{
    Availability, CommandError, CommandKind, CommandRegistry, CommandState, PALETTE,
};
use crate::components::{PaletteItem, command_palette};
use crate::devtools::{Devtools, FrameTimes, MsgSource, describe, dump, format_msg, inspector};
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
use crate::responder::Action;
use crate::runtime::{
    App, action_message, build_view_with, cursor_for, pointer_target, report_once, selects_on_focus,
};
use crate::style::Cursor;
use crate::text_field::{ClipboardPort, TextFieldDecorations, TextFieldState};
use crate::theme::Theme;
use crate::ui_engine::{Intent, Key, Modifiers, TextIntent, UiModel, dispatch, sync_focus};
use accesskit::{ActionRequest, TreeUpdate};
use cosmic_text::{FontSystem, Metrics};

pub(crate) use calls::{AgentCall, CONFIRMATION};

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
    /// Commands whose availability changed (for the menu bar).
    pub commands: Option<Vec<CommandState>>,
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
    /// Where each message of `log` came from.
    sources: Vec<MsgSource>,
    /// Origin of the event being handled; `None` is the keyboard (intents
    /// applied from outside without one).
    source: Option<MsgSource>,
    devtools: Devtools,
    /// View and layout times of the last frame.
    frame: FrameTimes,
    /// The inspector overlay is open (⌘⌥I, debug builds).
    inspector: bool,
    outbox: Outbox,
    registry: CommandRegistry<A>,
    /// Availability of every command after the last `update`.
    commands: Vec<CommandState>,
    palette: Option<Palette>,
    /// Agent calls, their confirmation dialog and the audit log.
    agents: calls::Agents,
}

/// The command palette while it is open: an overlay the session appends to
/// the app's tree and drives itself.
struct Palette {
    /// Contents of the search input.
    query: String,
    /// Highlighted row, an index into `rows`.
    selected: usize,
    /// Focus to give back on close.
    previous_focus: Option<WidgetId>,
    search: WidgetId,
    panel: WidgetId,
    /// Rows of the last `view`, in order, with their command ids.
    rows: Vec<(WidgetId, &'static str)>,
}

/// ⌘⌥I, which toggles the inspector.
fn is_inspector_key(intent: &Intent) -> bool {
    matches!(
        intent,
        Intent::Text(TextIntent::KeyDown {
            key: Key::Char('i'),
            modifiers: Modifiers {
                command: true,
                option: true,
                shift: false,
                control: false,
            },
            repeat: false,
        })
    )
}

/// Actions the palette lists for `query`: neither hidden, nor needing an
/// input, nor the framework's own, matching title, id or description,
/// ignoring case.
fn palette_items(commands: &[CommandState], query: &str) -> Vec<PaletteItem> {
    let query = query.trim().to_lowercase();
    commands
        .iter()
        .filter(|command| {
            command.kind == CommandKind::Action
                && !command.needs_input
                && command.availability != Availability::Hidden
                && !command.id.starts_with("kai.")
                && [command.title, command.id, command.description]
                    .iter()
                    .any(|text| text.to_lowercase().contains(&query))
        })
        .map(|command| PaletteItem {
            id: command.id,
            title: command.title,
            shortcut: command.shortcut,
            disabled: match &command.availability {
                Availability::Disabled { reason } => Some(reason.clone()),
                _ => None,
            },
        })
        .collect()
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
        let registry = CommandRegistry::of();
        report_once("Comandos inválidos", registry.validate());
        let commands = registry.states(&app);
        let tree = build_view_with(&app, &commands);
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
            sources: Vec::new(),
            source: None,
            devtools: Devtools::from_env(),
            frame: FrameTimes::default(),
            inspector: false,
            outbox: Outbox {
                commands: Some(commands.clone()),
                ..Outbox::default()
            },
            registry,
            commands,
            palette: None,
            agents: calls::Agents::default(),
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

    /// Where each recorded message came from, in the order of
    /// [`Self::messages`].
    pub fn message_sources(&self) -> &[MsgSource] {
        &self.sources
    }

    pub fn devtools(&self) -> Devtools {
        self.devtools
    }

    /// View and layout times of the last prepared frame.
    pub fn frame_times(&self) -> FrameTimes {
        self.frame
    }

    /// Run `f` as handling an event from `source`. Inner sources win: an
    /// agent's call that a click allowed comes from the agent.
    fn with_source<R>(&mut self, source: MsgSource, f: impl FnOnce(&mut Self) -> R) -> R {
        let outer = self.source.replace(source);
        let result = f(self);
        self.source = outer;
        result
    }

    /// Every command and its availability after the last `update`.
    pub fn commands(&self) -> &[CommandState] {
        &self.commands
    }

    pub fn command_state(&self, id: &str) -> Option<&CommandState> {
        self.commands.iter().find(|command| command.id == id)
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
        let commands = self.registry.states(&self.app);
        if commands != self.commands {
            self.outbox.commands = Some(commands.clone());
            self.commands = commands;
        }
        self.tree = build_view_with(&self.app, &self.commands);
        self.add_agent_status();
        self.add_inspector();
        self.add_palette();
        self.add_confirmation();
        self.background = self.app.background();
        if let Some(theme) = self.app.theme()
            && theme != self.theme
        {
            self.theme = theme;
            self.outbox.theme = Some(theme);
        }
        // While the palette or the confirmation dialog is open only it
        // takes focus and keys. The palette's own shortcut closes it; the
        // dialog takes no shortcuts.
        let confirming = self.confirmation_open();
        let overlay = (self.palette.is_some() || confirming)
            .then(|| self.tree.children.last())
            .flatten();
        self.focus_order = FocusOrder::from_tree(overlay.unwrap_or(&self.tree));
        self.focus_order.set_commands(
            self.commands
                .iter()
                .filter(|command| {
                    command.kind == CommandKind::Action && command.availability.is_enabled()
                })
                .filter(|command| !confirming && (overlay.is_none() || command.id == PALETTE))
                .filter_map(|command| command.shortcut.map(|shortcut| (command.id, shortcut)))
                .collect(),
        );
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
        let source = self.source.clone().unwrap_or(MsgSource::Keyboard);
        if let Some(log) = &mut self.log {
            log.push(msg.clone());
            self.sources.push(source.clone());
        }
        if self.devtools.msgs {
            let line = format!("{msg:?}");
            let started = Instant::now();
            let _effects = self.app.update(msg);
            eprintln!("{}", format_msg(&source, &line, started.elapsed()));
        } else {
            let _effects = self.app.update(msg);
        }
        self.invalidate();
    }

    /// Run command `id` as a shortcut, the menu bar or the palette do: an
    /// app action sends its message when it is enabled, `kai.paleta` opens
    /// or closes the palette. Returns whether something ran: nothing does
    /// while the confirmation dialog is open.
    pub fn perform_command(&mut self, id: &str) -> bool {
        if self.confirmation_open() {
            return false;
        }
        if id == PALETTE {
            if self.palette.is_some() {
                self.close_palette();
            } else {
                self.open_palette();
            }
            return true;
        }
        match self.registry.message(&self.app, id) {
            Ok(msg) => {
                self.send(msg);
                true
            }
            Err(_) => false,
        }
    }

    /// [`Self::perform_command`] from the menu bar.
    pub fn menu_command(&mut self, id: &str) -> bool {
        self.with_source(MsgSource::Menu, |session| session.perform_command(id))
    }

    /// Send `msg`, the message of action `id`, if `id` is a registered
    /// action that can run now; otherwise say why not.
    pub fn send_command(&mut self, id: &str, msg: A::Msg) -> Result<(), CommandError> {
        let availability = self.registry.availability(&self.app, id)?;
        match availability {
            Availability::Enabled => {
                self.with_source(MsgSource::Script, |session| session.send(msg));
                Ok(())
            }
            Availability::Disabled { reason } => Err(CommandError::new(reason)),
            Availability::Hidden => Err(CommandError::new(format!("{id} no está disponible"))),
        }
    }

    /// Whether `id` is a registered query.
    pub fn has_query(&self, id: &str) -> bool {
        self.registry.has_query(id)
    }

    /// Whether the command palette is open.
    #[cfg(test)]
    pub fn palette_open(&self) -> bool {
        self.palette.is_some()
    }

    fn open_palette(&mut self) {
        self.palette = Some(Palette {
            query: String::new(),
            selected: 0,
            previous_focus: self.focused(),
            search: WidgetId(0),
            panel: WidgetId(0),
            rows: Vec::new(),
        });
        self.rebuild();
        self.relayout();
        let search = self.palette.as_ref().map(|palette| palette.search);
        self.apply_intent(Intent::SetFocus(search));
        self.invalidate();
    }

    fn close_palette(&mut self) {
        let Some(palette) = self.palette.take() else {
            return;
        };
        self.rebuild();
        self.relayout();
        self.apply_intent(Intent::SetFocus(palette.previous_focus));
        self.invalidate();
    }

    /// Run the command of a palette row, closing the palette, unless it is
    /// disabled.
    fn run_from_palette(&mut self, id: &str) {
        if let Ok(msg) = self.registry.message(&self.app, id) {
            self.close_palette();
            self.send(msg);
        }
    }

    /// Append the open inspector to the tree, describing the innermost
    /// element under the pointer. It goes before the palette and the
    /// dialog so those stay last; `z-100` paints it over them.
    fn add_inspector(&mut self) {
        if !self.inspector {
            return;
        }
        let index_of = |id: WidgetId| self.layout.nodes().iter().position(|node| node.id == id);
        let target = self.hover_path.last().and_then(|&id| {
            let index = index_of(id)?;
            let node = self.layout.get(index);
            let class = self
                .tree
                .find(id)
                .map_or("", |element| element.class.as_str());
            let bounds = node.visible_bounds().unwrap_or(node.bounds);
            let mut lines = describe(&self.layout, index, class);
            if let Some(control) = pointer_target(&self.layout, &self.hover_path)
                .filter(|&control| control != id)
                .and_then(index_of)
            {
                let control = describe(&self.layout, control, "");
                lines.push(format!("clic → {}", control[..control.len() - 1].join(" ")));
            }
            Some((bounds, lines))
        });
        self.tree.children.push(inspector(target, self.viewport));
        self.tree.assign_ids();
    }

    /// Whether the inspector overlay is open.
    #[cfg(test)]
    pub fn inspector_open(&self) -> bool {
        self.inspector
    }

    /// Append the open palette to the tree, after the app's ids are set.
    fn add_palette(&mut self) {
        let Some(palette) = &mut self.palette else {
            return;
        };
        let items = palette_items(&self.commands, &palette.query);
        palette.selected = palette.selected.min(items.len().saturating_sub(1));
        let selected = (!items.is_empty()).then_some(palette.selected);
        self.tree
            .children
            .push(command_palette(&items, &palette.query, selected));
        self.tree.assign_ids();
        let find = |test_id: &str| {
            self.tree
                .find_test_id(test_id)
                .map_or(WidgetId(0), |element| element.id)
        };
        palette.search = find("kai.paleta.buscar");
        palette.panel = find("kai.paleta");
        palette.rows = items
            .iter()
            .map(|item| (find(&format!("kai.paleta.{}", item.id)), item.id))
            .collect();
    }

    /// Keys the open palette takes before focus sees them: arrows move the
    /// selection, Return in the search input runs it.
    fn palette_key(&mut self, intent: &Intent) -> bool {
        let Some(palette) = &mut self.palette else {
            return false;
        };
        let Intent::Text(TextIntent::KeyDown {
            key,
            modifiers,
            repeat,
        }) = intent
        else {
            return false;
        };
        if *modifiers != Modifiers::NONE {
            return false;
        }
        let last = palette.rows.len().saturating_sub(1);
        match key {
            Key::ArrowDown => palette.selected = (palette.selected + 1).min(last),
            Key::ArrowUp => palette.selected = palette.selected.saturating_sub(1),
            Key::Enter if !repeat && self.ui.focused.get() == &Some(palette.search) => {
                if let Some(&(_, id)) = palette.rows.get(palette.selected) {
                    self.run_from_palette(id);
                }
                return true;
            }
            _ => return false,
        }
        self.invalidate();
        true
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
        self.with_source(MsgSource::A11y, |session| {
            for intent in a11y::intents_for_action(request, &session.focus_order) {
                session.apply_intent(intent);
            }
        });
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
        if cfg!(debug_assertions) && is_inspector_key(&intent) {
            self.inspector = !self.inspector;
            if self.inspector {
                eprintln!("{}", dump(&self.tree, &self.layout));
            }
            self.invalidate();
            return;
        }
        if self.palette_key(&intent) {
            return;
        }
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
        let palette_row = |widget| {
            self.palette.as_ref().and_then(|palette| {
                palette
                    .rows
                    .iter()
                    .find(|(row, _)| *row == widget)
                    .map(|&(_, id)| id)
            })
        };
        if let Some(activation) = result.activated
            && let Some(decision) = self.confirmation_button(activation.widget)
        {
            self.decide(decision);
        } else if let Some(activation) = result.activated
            && let Some(id) = palette_row(activation.widget)
        {
            self.run_from_palette(id);
        } else if let Some(activation) = result.activated
            && let Some(msg) = self
                .handler(activation.widget)
                .and_then(|handlers| handlers.on_click.clone())
        {
            self.send(msg);
        }
        if let Some(Action::Command(id)) = result.action {
            self.perform_command(id);
        } else if self.confirmation_open() && result.action == Some(Action::CancelOperation) {
            self.decide(calls::Decision::Deny);
        } else if self.palette.is_some() && result.action == Some(Action::CancelOperation) {
            self.close_palette();
        } else if let Some(action) = result.action
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
        if let Some(palette) = &mut self.palette
            && palette.search == id
            && palette.query != after
        {
            palette.query = after.clone();
            palette.selected = 0;
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
        self.with_source(MsgSource::Pointer, |session| session.press_pointer(pressed));
    }

    fn press_pointer(&mut self, pressed: bool) {
        let target = pointer_target(&self.layout, &self.hover_path);
        if pressed
            && let Some(palette) = &self.palette
            && !self.hover_path.contains(&palette.panel)
        {
            self.close_palette();
            return;
        }
        if pressed
            && let Some(panel) = self.confirmation_panel()
            && !self.hover_path.contains(&panel)
        {
            return;
        }
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
        let started = Instant::now();
        self.rebuild();
        let built = Instant::now();
        self.relayout();
        if self.refresh_hover() {
            self.relayout();
        }
        self.frame = FrameTimes {
            view: built - started,
            layout: built.elapsed(),
            ..FrameTimes::default()
        };
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
