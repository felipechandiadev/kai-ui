//! The window-independent half of the runtime: the app, its tree and
//! layout, focus/hover/pressed state, scroll offsets and input editors.
//!
//! `runtime::WindowState` feeds it platform events in logical units and
//! applies its [`Outbox`] to the real window after each one; tests drive
//! the same `Session` through `kai::testing::TestApp` without a window or
//! GPU. Nothing here knows about winit.

#[cfg(test)]
mod busy_tests;
mod calls;
#[cfg(test)]
mod typeahead_tests;

use std::any::Any;
use std::collections::{BTreeMap, HashMap};
use std::ops::Range;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use crate::a11y;
use crate::background::BackgroundEffect;
use crate::commands::{
    Availability, CommandError, CommandKind, CommandRegistry, CommandState, PALETTE,
};
use crate::components::themed;
use crate::components::{MENU, MenuFrame, PaletteItem, command_palette, select_menu};
use crate::devtools::{Devtools, FrameTimes, MsgSource, describe, dump, format_msg, inspector};
use crate::effects::{self, Inbox};
use crate::element::{
    Element, FocusKind, FocusOrder, Handlers, Kind, Role, TOOLTIP_DELAY, VirtualRows, WidgetId, div,
};
use crate::fold::fold;
use crate::geo::{Camera, LatLon, MapView, TileId};
use crate::geometry::{Point, Rect, Size};
use crate::layout::testing::FixedMeasure;
use crate::layout::{
    Axis, Content, Interaction, Layout, LayoutInput, ScrollOffsets, StyleCache, TextMeasure,
    WindowActivity, text_style,
};
use crate::maps::camera::CameraState;
use crate::maps::editor::PolygonEdit;
use crate::maps::labels::Labels;
use crate::maps::tiles::{SourceKey, TileResult, TileStore};
#[cfg(feature = "document")]
use crate::document::DocumentEditor;
use crate::media::{self, ImageLoader, ImageSource, ImageStore, LoadedImage};
use crate::render::paint::{PaintInput, paint};
use crate::render::text::TextStore;
use crate::render::{DrawList, TextId};
use crate::responder::Action;
use crate::runtime::{
    App, Effect, Effects, FilePicked, HELD_TIMER_RESUME, InvocationId, PickFile, SaveFile, SubKind,
    TaskDone, action_message, build_view_with, cursor_for, pointer_target, report_once,
    selects_on_focus,
};
use crate::style::Cursor;
use crate::text_field::{ClipboardPort, TextFieldDecorations, TextFieldState, caret_blink_visible};
use crate::theme::{SystemPrefs, Theme};
use crate::ui_engine::{Intent, Key, Modifiers, TextIntent, UiModel, dispatch, sync_focus};
use crate::virtual_rows::{self, Viewport};
use accesskit::{ActionRequest, TreeUpdate};
use cosmic_text::{FontSystem, Metrics};

pub(crate) use calls::{AgentCall, CONFIRMATION};

/// Points an arrow key moves a focused map.
const MAP_KEY_STEP: f64 = 80.0;
/// A press and release on a map closer than this (points) is a click.
const MAP_CLICK_SLOP: f32 = 4.0;
/// Points an arrow key moves a focused vertex of a polygon editor (ten
/// times as far with ⇧).
const VERTEX_KEY_STEP: f32 = 1.0;

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
    /// Show the open panel; answer with [`Session::finish_file_pick`].
    pub pick_file: Option<PickFile>,
    /// Show the save panel; answer with [`Session::finish_file_pick`].
    pub save_file: Option<SaveFile>,
}

pub(crate) struct Session<A: App> {
    app: A,
    theme: Theme,
    /// The Mac's preferences, laid over every theme; `None` keeps the
    /// theme's own.
    system_prefs: Option<SystemPrefs>,
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
    /// What started the event being handled, for the tasks it performs:
    /// the clicked control or the command.
    task_origin: Option<TaskOrigin>,
    /// Cursor last asked for over the content.
    pointer_cursor: Cursor,
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
    /// The open list of a pop-up button.
    popup: Option<Popup>,
    /// Agent calls, their confirmation dialog and the audit log.
    agents: calls::Agents,
    /// The app's open [`Element::modal`] panel.
    modal: Option<AppModal>,
    /// Focus to set once the frame's tree is laid out (a modal appeared or
    /// left).
    modal_focus: Option<ModalFocus>,
    /// Pending [`Effects::after`] timers, in the order they were asked for.
    timers: Vec<Timer<A::Msg>>,
    /// Active [`App::subscriptions`], diffed after `init` and each `update`.
    subs: Vec<ActiveSub<A::Msg>>,
    /// Running [`Effects::perform`] tasks.
    tasks: Tasks<A::Msg>,
    /// Decoded images of the views, filled into the tree on rebuild.
    images: ImageStore,
    /// What turns the open panel's answer into a message, while it is open.
    file_pick: Option<FilePicked<A::Msg>>,
    /// Files are being dragged over the window.
    file_drag: bool,
    /// The time tests set; `None` follows the real clock.
    clock: Option<Instant>,
    /// When the session started: time zero of [`Motion`](crate::element::Motion).
    epoch: Instant,
    /// When the caret blink was last reset to the visible half (AppKit-style).
    caret_blink_at: Instant,
    /// Row the keys are moving to in a virtual list: the next rebuild
    /// builds it wherever it is.
    row_pin: Option<(WidgetId, usize)>,
    /// Rows built for each virtual list in the last rebuild.
    row_windows: Vec<(WidgetId, Range<usize>)>,
    /// [`Element::tooltip`] of every element of the last rebuild.
    tooltips: HashMap<WidgetId, String>,
    /// The tooltip of the element under the pointer, waiting or shown.
    tooltip: Option<Tooltip>,
    /// Element whose tooltip a press hid: it stays hidden until the
    /// pointer leaves it.
    tooltip_pressed: Option<WidgetId>,
    /// Camera of every map of the last rebuild.
    maps: HashMap<WidgetId, CameraState>,
    /// WordCraft editor of every document widget (feature `document`).
    #[cfg(feature = "document")]
    documents: HashMap<WidgetId, DocumentEditor>,
    /// The map a pointer drag is moving.
    map_drag: Option<WidgetId>,
    /// Where the pointer went down on a map: released close by, it is a
    /// click on the layer under it.
    map_press: Option<(WidgetId, Point)>,
    /// Where the pointer went down on a bar chart: released close by, it
    /// selects the category under it.
    chart_press: Option<(WidgetId, Point)>,
    /// The clickable map layer under the pointer.
    map_hover: Option<(WidgetId, String)>,
    /// The polygon editor vertex the pointer is dragging.
    vertex_drag: Option<VertexDrag>,
    /// Basemap tiles of every map, loading on the task pool.
    tiles: TileStore,
    /// Loaded basemap tiles wait to be shown, a few per frame.
    tiles_revealing: bool,
    /// Basemap labels of every map.
    labels: HashMap<WidgetId, Labels>,
    /// Some map label is fading in: frames are needed.
    labels_fading: bool,
    /// A map with a basemap, layers or a fit request changed size in the
    /// last layout: its tiles, labels or fitted view were picked for the
    /// old one.
    maps_resized: bool,
    /// Modifier keys held now (⇧ double click and ⌘ wheel on maps,
    /// Ctrl+click).
    modifiers: Modifiers,
    /// A Ctrl+click opened a context menu: its release does nothing else.
    context_press: bool,
}

/// A vertex of a map's polygon editor under the pointer's drag.
#[derive(Clone, Copy, Debug)]
struct VertexDrag {
    map: WidgetId,
    index: usize,
    /// From the vertex to where the pointer took it, so it does not jump.
    grab: Point,
    /// Some move was sent: the next one is not the first of the drag.
    moved: bool,
}

struct Tooltip {
    anchor: WidgetId,
    due: Instant,
    shown: bool,
}

/// Test id of the tooltip bubble.
pub(crate) const TOOLTIP: &str = "kai.tooltip";

/// Where the tooltip for `anchor` goes: to its right for an element at the
/// left edge of the window (a sidebar rail), else below it, or above when
/// there is no room below. Its size is estimated from the text.
fn tooltip_origin(anchor: Rect, text: &str, viewport: Size) -> Point {
    const GAP: f32 = 6.0;
    const MARGIN: f32 = 8.0;
    const HEIGHT: f32 = 24.0;
    let width = (text.chars().count() as f32 * 6.5 + 16.0).min(280.0);
    let fit_x = |x: f32| x.min(viewport.width - width - MARGIN).max(MARGIN);
    let fit_y = |y: f32| y.min(viewport.height - HEIGHT - MARGIN).max(MARGIN);
    if anchor.x + anchor.width <= 64.0 {
        return Point {
            x: anchor.x + anchor.width + GAP,
            y: fit_y(anchor.y + (anchor.height - HEIGHT) / 2.0),
        };
    }
    let below = anchor.y + anchor.height + GAP;
    let y = if below + HEIGHT + MARGIN <= viewport.height {
        below
    } else {
        anchor.y - GAP - HEIGHT
    };
    Point {
        x: fit_x(anchor.x),
        y: fit_y(y),
    }
}

/// The tooltip overlay: a bubble with `text` at `origin`, over everything
/// and taking no pointer.
fn tooltip_bubble<Msg>(text: &str, origin: Point) -> Element<Msg> {
    div()
        .class("absolute inset-0 z-60 pointer-events-none")
        .child(
            div()
                .class(&format!(
                    "absolute left-[{:.0}px] top-[{:.0}px] max-w-[280px] {}",
                    origin.x,
                    origin.y,
                    themed(
                        "px-2 py-1 rounded-md text-xs bg-foreground text-background shadow-md",
                        "px-1.5 py-0.5 rounded-[4px] text-caption1 text-label \
                         bg-control-background border border-separator shadow-md",
                    )
                ))
                .role(Role::Tooltip)
                .label(text)
                .test_id(TOOLTIP)
                .child(crate::element::text(text).class("whitespace-nowrap")),
        )
}

/// Collect the [`Element::tooltip`] of `element` and its descendants.
fn collect_tooltips<Msg>(element: &Element<Msg>, into: &mut HashMap<WidgetId, String>) {
    if let Some(tooltip) = &element.tooltip {
        into.insert(element.id, tooltip.clone());
    }
    for child in &element.children {
        collect_tooltips(child, into);
    }
}

struct Timer<Msg> {
    key: Option<String>,
    due: Instant,
    msg: Msg,
    /// The pointer or the focus is inside the element whose `test_id` is
    /// `key` ([`Effects::keyed`]).
    held: bool,
}

/// An active [`App::subscriptions`] entry, keyed for diff after each update.
struct ActiveSub<Msg> {
    key: String,
    every: Duration,
    next: Instant,
    msg: Msg,
}

/// What started an [`Effects::perform`] task: it stays busy until the
/// task, and those its message performs, deliver.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TaskOrigin {
    /// A clicked control.
    Widget(WidgetId),
    /// A command, from a shortcut, the menu bar, the palette, a button
    /// bound to it, a script or an agent.
    Command(&'static str),
}

/// Reason a busy command gives in menus, the palette and to agents.
pub const COMMAND_BUSY: &str = "En curso";

struct WaitingTask<Msg> {
    key: Option<String>,
    origin: Option<TaskOrigin>,
    done: TaskDone<Msg>,
    /// Agent / MCP call to answer when this task's message is applied.
    completes: Option<InvocationId>,
}

/// [`Effects::perform`] tasks on the pool, by id, with what turns each
/// result into a message; a replaced or cancelled task leaves this map and
/// its result is dropped when it lands.
struct Tasks<Msg> {
    inbox: Arc<Inbox>,
    next: u64,
    waiting: HashMap<u64, WaitingTask<Msg>>,
    /// Images loading on the pool for the [`ImageStore`].
    images: HashMap<u64, ImageSource>,
    /// Map tiles loading on the pool for the [`TileStore`].
    tiles: HashMap<u64, (SourceKey, TileId)>,
}

impl<Msg> Default for Tasks<Msg> {
    fn default() -> Self {
        Self {
            inbox: Arc::default(),
            next: 1,
            waiting: HashMap::new(),
            images: HashMap::new(),
            tiles: HashMap::new(),
        }
    }
}

impl<Msg> Tasks<Msg> {
    /// Drop pending tasks with `key`. Returns the agent calls they would
    /// have completed, so the session can fail them.
    fn forget(&mut self, key: &str) -> Vec<InvocationId> {
        let mut dropped = Vec::new();
        self.waiting.retain(|_, task| {
            if task.key.as_deref() == Some(key) {
                if let Some(id) = task.completes {
                    dropped.push(id);
                }
                false
            } else {
                true
            }
        });
        dropped
    }

    /// Drop pending tasks that would complete `id`.
    fn forget_completes(&mut self, id: InvocationId) {
        self.waiting
            .retain(|_, task| task.completes != Some(id));
    }

    fn completes_pending(&self, id: InvocationId) -> bool {
        self.waiting
            .values()
            .any(|task| task.completes == Some(id))
    }

    /// Origins of the tasks still running.
    fn origins(&self) -> impl Iterator<Item = TaskOrigin> + '_ {
        self.waiting.values().filter_map(|task| task.origin)
    }

    fn spawn(&mut self, task: crate::runtime::Task) -> u64 {
        let id = self.next;
        self.next += 1;
        effects::spawn(&self.inbox, id, task);
        id
    }

    fn pending(&self) -> bool {
        !self.waiting.is_empty() || !self.images.is_empty() || !self.tiles.is_empty()
    }
}

/// An [`Element::modal`] panel of the app while it is in the tree.
struct AppModal {
    panel: WidgetId,
    /// Focus to give back when it leaves.
    previous_focus: Option<WidgetId>,
}

enum ModalFocus {
    /// The first control of the modal that appeared.
    First,
    /// Back where it was before the modal.
    Restore(Option<WidgetId>),
}

/// The list of a pop-up button ([`Element::options`]) while it is open:
/// an overlay the session appends to the app's tree and drives itself, as
/// the palette. The button gets the focus back when it closes.
struct Popup {
    owner: WidgetId,
    /// Where a context menu opened; a pop-up list goes under its button.
    at: Option<Point>,
    /// Focus to give back on close: the button, or for a context menu
    /// whatever had it.
    return_focus: Option<WidgetId>,
    /// Row the keys act on, an index into `rows`.
    highlighted: usize,
    root: WidgetId,
    panel: WidgetId,
    /// Rows of the last `view`, in order.
    rows: Vec<WidgetId>,
    /// Letters typed to find an option, and when the last one came.
    typed: String,
    typed_at: Instant,
}

/// What a key does to the open pop-up list.
enum PopupKey {
    Highlight(usize),
    Choose(usize),
    /// Text typed to find an option by its first letters.
    Find(String),
    Close,
    Ignore,
}

/// Pause after which typing in an open list starts a new search.
pub(crate) const TYPEAHEAD_RESET: Duration = Duration::from_secs(1);

/// The option `typed` finds among `items` with `highlighted` the current
/// one: the next that starts with it, after the current one when `typed`
/// is one letter (or the same letter again, which goes through the
/// options with it) and from the current one otherwise, wrapping around.
fn typeahead(items: &[String], highlighted: usize, typed: &str) -> Option<usize> {
    let typed = fold(typed);
    let mut letters = typed.chars();
    let first = letters.next()?;
    let repeated = letters.all(|c| c == first);
    let (prefix, start) = if repeated {
        (first.to_string(), highlighted + 1)
    } else {
        (typed, highlighted)
    };
    (0..items.len())
        .map(|offset| (start + offset) % items.len())
        .find(|&index| fold(&items[index]).starts_with(&prefix))
}

/// Where the list of a pop-up button at `anchor` goes: below it, or above
/// when there is more room there; rows are estimated as tall as the button.
fn menu_frame(anchor: Rect, rows: usize, viewport: Size) -> MenuFrame {
    const GAP: f32 = 4.0;
    const MARGIN: f32 = 8.0;
    let height = rows as f32 * anchor.height.max(20.0) + 12.0;
    let below = viewport.height - (anchor.y + anchor.height + GAP) - MARGIN;
    let above = anchor.y - GAP - MARGIN;
    let (y, max_height) = if height <= below || below >= above {
        (anchor.y + anchor.height + GAP, below)
    } else {
        ((anchor.y - GAP - height.min(above)).max(MARGIN), above)
    };
    MenuFrame {
        x: anchor.x.max(0.0),
        y,
        min_width: anchor.width,
        max_height: max_height.max(anchor.height),
    }
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
        let effects = app.init();
        let registry = CommandRegistry::of();
        report_once("Comandos inválidos", registry.validate());
        let commands = registry.states(&app);
        let tree = build_view_with(&app, &commands);
        let mut session = Self {
            theme,
            system_prefs: None,
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
            task_origin: None,
            pointer_cursor: Cursor::Default,
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
            popup: None,
            agents: calls::Agents::default(),
            modal: None,
            modal_focus: None,
            timers: Vec::new(),
            subs: Vec::new(),
            tasks: Tasks::default(),
            images: ImageStore::default(),
            file_pick: None,
            file_drag: false,
            clock: None,
            epoch: Instant::now(),
            caret_blink_at: Instant::now(),
            row_pin: None,
            row_windows: Vec::new(),
            tooltips: HashMap::new(),
            tooltip: None,
            tooltip_pressed: None,
            maps: HashMap::new(),
            #[cfg(feature = "document")]
            documents: HashMap::new(),
            map_drag: None,
            map_press: None,
            chart_press: None,
            map_hover: None,
            vertex_drag: None,
            tiles: TileStore::default(),
            tiles_revealing: false,
            labels: HashMap::new(),
            labels_fading: false,
            maps_resized: false,
            modifiers: Modifiers::NONE,
            context_press: false,
        };
        session.apply_effects(effects);
        session.sync_subscriptions();
        session.notify_resized();
        session.rebuild();
        session.relayout();
        session
    }

    /// Tell the app the window's size ([`App::resized`]).
    fn notify_resized(&mut self) {
        if let Some(msg) = self.app.resized(self.viewport) {
            self.with_source(MsgSource::Window, |session| session.send(msg));
        }
    }

    fn now(&self) -> Instant {
        self.clock.unwrap_or_else(Instant::now)
    }

    /// Stop following the real clock: from now on time is `at` until set
    /// again (`TestApp::advance`).
    pub fn set_clock(&mut self, at: Instant) {
        self.clock = Some(at);
    }

    /// The current time, real or set.
    pub fn clock(&self) -> Instant {
        self.now()
    }

    /// Time zero of [`Motion`](crate::element::Motion): with the clock at `epoch() + t`, moving
    /// elements paint as at `t` seconds.
    pub fn epoch(&self) -> Instant {
        self.epoch
    }

    fn apply_effects(&mut self, effects: Effects<A::Msg>) {
        for effect in effects.into_vec() {
            match effect {
                Effect::After { key, delay, msg } => {
                    if let Some(key) = &key {
                        self.timers.retain(|timer| timer.key.as_ref() != Some(key));
                    }
                    let due = self.now() + delay;
                    self.timers.push(Timer {
                        key,
                        due,
                        msg,
                        held: false,
                    });
                }
                Effect::Perform {
                    key,
                    task,
                    done,
                    completes,
                } => {
                    if let Some(key) = &key {
                        for id in self.tasks.forget(key) {
                            self.fail_deferred_task(id);
                        }
                    }
                    let id = self.tasks.spawn(task);
                    let origin = self.task_origin;
                    self.tasks.waiting.insert(
                        id,
                        WaitingTask {
                            key,
                            origin,
                            done,
                            completes,
                        },
                    );
                }
                Effect::Cancel(key) => {
                    self.timers
                        .retain(|timer| timer.key.as_deref() != Some(key.as_str()));
                    for id in self.tasks.forget(&key) {
                        self.fail_deferred_task(id);
                    }
                }
                Effect::PickFile { request, done } => {
                    self.file_pick = Some(done);
                    self.outbox.save_file = None;
                    self.outbox.pick_file = Some(request);
                }
                Effect::SaveFile { request, done } => {
                    self.file_pick = Some(done);
                    self.outbox.pick_file = None;
                    self.outbox.save_file = Some(request);
                }
            }
        }
    }

    /// Wake the window with `wake` when an [`Effects::perform`] task
    /// finishes; it is called from a pool thread.
    pub fn set_task_waker(&mut self, wake: Box<dyn Fn() + Send>) {
        self.tasks.inbox.set_waker(wake);
    }

    /// Whether some [`Effects::perform`] task has not sent its message,
    /// some image or map tile is still loading, or loaded map tiles wait
    /// to be shown.
    pub fn tasks_pending(&self) -> bool {
        self.tasks.pending() || self.tiles_revealing
    }

    /// Send the message of every task that finished, in the order they
    /// finished, and keep the images that loaded (asking for a frame).
    /// Returns whether anything finished.
    pub fn run_tasks(&mut self) -> bool {
        let mut ran = false;
        for (id, output) in self.tasks.inbox.take() {
            if let Some(source) = self.tasks.images.remove(&id) {
                let result = output
                    .and_then(|output| output.downcast::<Result<LoadedImage, String>>().ok())
                    .map_or_else(|| Err("la carga falló".to_owned()), |result| *result);
                self.images.finish(&source, result);
                self.invalidate();
                ran = true;
                continue;
            }
            if let Some((source, tile)) = self.tasks.tiles.remove(&id) {
                let result = output
                    .and_then(|output| output.downcast::<TileResult>().ok())
                    .map_or_else(|| Err("la carga falló".to_owned()), |result| *result);
                self.tiles.finish(&source, tile, result);
                self.invalidate();
                ran = true;
                continue;
            }
            let Some(WaitingTask {
                origin,
                done,
                completes,
                ..
            }) = self.tasks.waiting.remove(&id)
            else {
                continue;
            };
            if let Some(output) = output {
                self.with_source(MsgSource::Task, |session| {
                    session.with_task_origin(origin, |session| session.send(done(output)));
                });
                if let Some(invocation) = completes {
                    self.finish_deferred(invocation);
                }
                ran = true;
            } else {
                if let Some(invocation) = completes {
                    self.fail_deferred_task(invocation);
                } else if origin.is_some() {
                    self.invalidate();
                }
                ran = true;
            }
        }
        ran
    }

    /// Block until a pending task finishes or `timeout` passes, then send
    /// what finished; returns whether anything did.
    pub fn wait_for_tasks(&mut self, timeout: Duration) -> bool {
        if !self.tasks.pending() {
            return false;
        }
        self.tasks.inbox.wait(timeout);
        self.run_tasks()
    }

    /// Whether the panel of [`Effects::pick_file`] or
    /// [`Effects::save_file`] waits for an answer.
    pub fn file_pick_pending(&self) -> bool {
        self.file_pick.is_some()
    }

    /// The open panel closed with `path` (`None`: cancelled): send the
    /// app's message.
    pub fn finish_file_pick(&mut self, path: Option<PathBuf>) {
        if let Some(done) = self.file_pick.take() {
            self.with_source(MsgSource::Files, |session| session.send(done(path)));
        }
    }

    /// Files entered the window (`HoveredFile`): drop zones take their
    /// `drag:` classes.
    pub fn hover_files(&mut self) {
        if !self.file_drag {
            self.file_drag = true;
            self.invalidate();
        }
    }

    /// The dragged files left the window (`HoveredFileCancelled`).
    pub fn cancel_file_drag(&mut self) {
        if self.file_drag {
            self.file_drag = false;
            self.invalidate();
        }
    }

    /// Files were dropped on the window: the drop zone gets them (see
    /// [`Session::drop_target`]). Returns whether one did.
    pub fn drop_files(&mut self, paths: Vec<PathBuf>) -> bool {
        self.cancel_file_drag();
        let handler = self
            .drop_target()
            .and_then(|id| self.handler(id))
            .and_then(|handlers| handlers.on_drop_files.clone());
        match handler {
            Some(handler) if !paths.is_empty() => {
                self.with_source(MsgSource::Files, |session| session.send(handler(paths)));
                true
            }
            _ => false,
        }
    }

    /// The drop zone that gets dropped files: the only one on screen, or
    /// the innermost under the pointer's last position. Disabled and
    /// clipped-out zones take nothing.
    pub fn drop_target(&self) -> Option<WidgetId> {
        let zones: Vec<_> = self
            .handlers
            .iter()
            .filter(|(_, handlers)| handlers.on_drop_files.is_some())
            .filter_map(|(id, _)| {
                let node = self.layout.node(*id)?;
                let visible = node.visible_bounds()?;
                (!node.hidden && !node.disabled && visible.width > 0.0 && visible.height > 0.0)
                    .then_some((*id, visible))
            })
            .collect();
        if let [(only, _)] = zones.as_slice() {
            return Some(*only);
        }
        let cursor = self.cursor?;
        self.hover_path
            .iter()
            .rev()
            .copied()
            .find(|id| zones.iter().any(|(zone, _)| zone == id))
            .or_else(|| {
                zones
                    .iter()
                    .find(|(_, visible)| visible.contains(cursor))
                    .map(|(id, _)| *id)
            })
    }

    /// Read images with `loader` from now on (tests use a
    /// [`FakeImageLoader`](crate::media::FakeImageLoader)).
    pub fn set_image_loader(&mut self, loader: Arc<dyn ImageLoader>) {
        self.images.set_loader(loader);
    }

    /// Load the images the last `view` wants, on the pool.
    fn start_image_loads(&mut self) {
        for source in self.images.take_wanted() {
            let loader = self.images.loader();
            let job = source.clone();
            let id = self.tasks.spawn(Box::new(move || {
                Box::new(media::load(loader.as_ref(), &job)) as Box<dyn Any + Send>
            }));
            self.tasks.images.insert(id, source);
        }
    }

    /// When the earliest pending timer or subscription tick is due.
    pub fn next_deadline(&self) -> Option<Instant> {
        let tooltip = self
            .tooltip
            .as_ref()
            .filter(|tooltip| !tooltip.shown)
            .map(|tooltip| tooltip.due);
        self.timers
            .iter()
            .map(|timer| timer.due)
            .chain(self.subs.iter().map(|sub| sub.next))
            .chain(tooltip)
            .chain(self.maps.values().filter_map(CameraState::deadline))
            .min()
    }

    /// Send the message of every timer that is due, earliest first, and
    /// show the waiting tooltip when its time comes. Returns whether
    /// anything ran.
    pub fn run_timers(&mut self) -> bool {
        let now = self.now();
        let mut ran = false;
        if let Some(tooltip) = &mut self.tooltip
            && !tooltip.shown
            && tooltip.due <= now
        {
            tooltip.shown = true;
            self.invalidate();
            ran = true;
        }
        self.update_held_timers();
        for timer in &mut self.timers {
            if timer.held && timer.due <= now {
                timer.due = now + HELD_TIMER_RESUME;
            }
        }
        while let Some(index) = self
            .timers
            .iter()
            .enumerate()
            .filter(|(_, timer)| timer.due <= now)
            .min_by_key(|(_, timer)| timer.due)
            .map(|(index, _)| index)
        {
            let timer = self.timers.remove(index);
            self.with_source(MsgSource::Timer, |session| session.send(timer.msg));
            ran = true;
        }
        while let Some(index) = self
            .subs
            .iter()
            .enumerate()
            .filter(|(_, sub)| sub.next <= now)
            .min_by_key(|(_, sub)| sub.next)
            .map(|(index, _)| index)
        {
            let msg = self.subs[index].msg.clone();
            let every = self.subs[index].every;
            self.subs[index].next = now + every;
            self.with_source(MsgSource::Subscription, |session| session.send(msg));
            ran = true;
        }
        ran | self.report_maps()
    }

    /// Diff [`App::subscriptions`] against the active set: start new keys,
    /// stop missing ones, keep the schedule when key and period match.
    fn sync_subscriptions(&mut self) {
        let wanted = self.app.subscriptions().into_vec();
        let now = self.now();
        let mut next = Vec::with_capacity(wanted.len());
        for sub in wanted {
            match sub.kind {
                SubKind::Every(every) => {
                    let due = self
                        .subs
                        .iter()
                        .find(|active| active.key == sub.key && active.every == every)
                        .map(|active| active.next)
                        .unwrap_or_else(|| now + every);
                    next.push(ActiveSub {
                        key: sub.key,
                        every,
                        next: due,
                        msg: sub.msg,
                    });
                }
            }
        }
        self.subs = next;
    }

    /// Whether the pointer or the focus is inside the element whose
    /// `test_id` is `key`.
    fn timer_held(&self, key: &str) -> bool {
        let Some(id) = self.tree.find_test_id(key).map(|element| element.id) else {
            return false;
        };
        self.hover_path.contains(&id)
            || self
                .ui
                .focused
                .get()
                .and_then(|focused| self.tree.path_to(focused))
                .is_some_and(|path| path.contains(&id))
    }

    /// Hold the keyed timers whose element has the pointer or the focus;
    /// one let go runs no sooner than [`HELD_TIMER_RESUME`] from now.
    fn update_held_timers(&mut self) {
        if self.timers.iter().all(|timer| timer.key.is_none()) {
            return;
        }
        let now = self.now();
        let held: Vec<bool> = self
            .timers
            .iter()
            .map(|timer| timer.key.as_deref().is_some_and(|key| self.timer_held(key)))
            .collect();
        for (timer, held) in self.timers.iter_mut().zip(held) {
            if timer.held && !held {
                timer.due = timer.due.max(now + HELD_TIMER_RESUME);
            }
            timer.held = held;
        }
    }

    pub fn app(&self) -> &A {
        &self.app
    }

    pub fn theme(&self) -> &Theme {
        &self.theme
    }

    /// Take the Mac's accessibility preferences (reduce motion, increase
    /// contrast, reduce transparency, keyboard navigation): they apply over
    /// the app's theme from now on.
    pub fn set_system_prefs(&mut self, prefs: SystemPrefs) {
        if self.system_prefs == Some(prefs) {
            return;
        }
        self.system_prefs = Some(prefs);
        self.theme = self.with_system_prefs(self.theme);
        self.rebuild();
        self.relayout();
        self.invalidate();
    }

    /// `theme` with the system's preferences. Kai keeps its focus ring on
    /// buttons with or without keyboard navigation, as on the web.
    fn with_system_prefs(&self, mut theme: Theme) -> Theme {
        if let Some(system) = self.system_prefs {
            theme.prefs = SystemPrefs {
                full_keyboard_access: if theme.is_appkit() {
                    system.full_keyboard_access
                } else {
                    theme.prefs.full_keyboard_access
                },
                ..system
            };
        }
        theme
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
        if let Some(text) = self.text_fields.get(&id).map(TextFieldState::text) {
            return Some(text);
        }
        #[cfg(feature = "document")]
        if let Some(ed) = self.documents.get(&id) {
            return Some(ed.plain_text());
        }
        None
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

    /// Run `f` with the tasks it performs started by `origin`.
    fn with_task_origin<R>(
        &mut self,
        origin: Option<TaskOrigin>,
        f: impl FnOnce(&mut Self) -> R,
    ) -> R {
        let outer = std::mem::replace(&mut self.task_origin, origin);
        let result = f(self);
        self.task_origin = outer;
        result
    }

    /// Whether a task that command `id` started is still running.
    pub fn command_busy(&self, id: &str) -> bool {
        self.tasks
            .origins()
            .any(|origin| matches!(origin, TaskOrigin::Command(command) if command == id))
    }

    /// Whether a task that control `id` started is still running, or the
    /// app marked it [`Element::busy`](crate::element::Element::busy).
    pub fn busy(&self, id: WidgetId) -> bool {
        self.tree.find(id).is_some_and(|element| element.busy)
    }

    /// The message of action `id` for `input` if it can run now: not
    /// while a task it started runs.
    pub(crate) fn command_message(
        &self,
        id: &str,
        input: serde_json::Value,
    ) -> Result<A::Msg, CommandError> {
        if self.command_busy(id) {
            self.registry.availability(&self.app, id)?;
            return Err(CommandError::new(COMMAND_BUSY));
        }
        self.registry.message_from(&self.app, id, input)
    }

    /// The `&'static` id of registered command `id`.
    fn command_origin(&self, id: &str) -> Option<TaskOrigin> {
        self.command_state(id)
            .map(|command| TaskOrigin::Command(command.id))
    }

    /// A busy command is disabled in menus and the palette ([`COMMAND_BUSY`]);
    /// the buttons bound to it turn busy.
    fn mark_busy_commands(&self, commands: &mut [CommandState]) {
        for command in commands {
            if self.command_busy(command.id) {
                command.busy = true;
                command.availability = Availability::disabled(COMMAND_BUSY);
            }
        }
    }

    /// Mark busy the controls whose tasks run, unless they opt out
    /// ([`Element::busy_feedback`](crate::element::Element::busy_feedback)).
    fn mark_busy_widgets(&mut self) {
        let widgets: Vec<WidgetId> = self
            .tasks
            .origins()
            .filter_map(|origin| match origin {
                TaskOrigin::Widget(id) => Some(id),
                TaskOrigin::Command(_) => None,
            })
            .collect();
        for id in widgets {
            if let Some(element) = self.tree.find_mut(id)
                && element.busy_feedback
            {
                element.busy = true;
            }
        }
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
        if viewport != self.viewport {
            self.viewport = viewport;
            self.notify_resized();
        }
        self.invalidate();
    }

    /// Run `view`, then reconcile focus and input editors with the new tree.
    fn rebuild(&mut self) {
        let mut commands = self.registry.states(&self.app);
        self.mark_busy_commands(&mut commands);
        if commands != self.commands {
            self.outbox.commands = Some(commands.clone());
            self.commands = commands;
        }
        let pin = self.row_pin.take().or_else(|| self.focused_row());
        self.tree = build_view_with(&self.app, &self.commands);
        self.build_rows(pin);
        self.mark_busy_widgets();
        self.images.resolve(&mut self.tree);
        self.sync_maps();
        #[cfg(feature = "document")]
        self.sync_documents();
        self.tooltips.clear();
        collect_tooltips(&self.tree, &mut self.tooltips);
        if self
            .tooltip
            .as_ref()
            .is_some_and(|tooltip| !self.tooltips.contains_key(&tooltip.anchor))
        {
            self.tooltip = None;
        }
        self.track_modal();
        self.add_agent_status();
        self.add_inspector();
        self.add_tooltip();
        self.add_popup();
        self.add_palette();
        self.add_confirmation();
        self.background = self.app.background();
        if let Some(theme) = self.app.theme().map(|theme| self.with_system_prefs(theme))
            && theme != self.theme
        {
            self.theme = theme;
            self.outbox.theme = Some(theme);
        }
        // While the palette, the confirmation dialog, a pop-up list or an
        // app modal is open only it takes focus and keys. The palette's own
        // shortcut closes it; dialogs take no shortcuts.
        let confirming = self.confirmation_open();
        let modal = self.modal.as_ref().map(|modal| modal.panel);
        let overlay = if self.palette.is_some() || confirming {
            self.tree.children.last()
        } else {
            self.popup
                .as_ref()
                .and_then(|popup| {
                    self.tree
                        .children
                        .iter()
                        .find(|child| child.id == popup.root)
                })
                .or_else(|| modal.and_then(|panel| self.tree.find(panel)))
        };
        self.focus_order = FocusOrder::from_tree(overlay.unwrap_or(&self.tree));
        self.focus_order.set_commands(
            self.commands
                .iter()
                .filter(|command| {
                    command.kind == CommandKind::Action && command.availability.is_enabled()
                })
                .filter(|command| {
                    !confirming && modal.is_none() && (overlay.is_none() || command.id == PALETTE)
                })
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
                rows,
                ..
            } = &element.kind
            {
                let field = fields
                    .entry(element.id)
                    .or_insert_with(|| TextFieldState::new(font_system, Metrics::new(16.0, 24.0)));
                field.set_multiline(font_system, rows.is_some());
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

    /// The virtual list around the focus and the index of the focused row.
    fn focused_row(&self) -> Option<(WidgetId, usize)> {
        let path = self.tree.path_to(self.focused()?)?;
        virtual_rows::row_on_path(&self.tree, &path)
    }

    fn virtual_rows(&self, list: WidgetId) -> Option<&VirtualRows<A::Msg>> {
        self.tree
            .find(list)
            .and_then(|list| list.virtual_rows.as_ref())
    }

    /// Build the rows of every virtual list for its scroll offset and last
    /// laid-out height, plus `pin` (a list and a row index).
    fn build_rows(&mut self, pin: Option<(WidgetId, usize)>) {
        let layout = &self.layout;
        let scroll = &self.scroll;
        self.row_windows = virtual_rows::materialize(&mut self.tree, &|id| Viewport {
            offset: scroll.get(&id).map_or(0.0, |offset| offset.y),
            height: layout.node(id).map(|node| node.padding_box.height),
            pinned: pin.filter(|(list, _)| *list == id).map(|(_, index)| index),
        });
        if !self.row_windows.is_empty() {
            self.tree.assign_ids();
        }
    }

    /// Whether the last layout shows rows of a virtual list that the last
    /// rebuild did not build (first layout, resize, clamped offset).
    fn rows_stale(&self) -> bool {
        self.row_windows.iter().any(|(id, built)| {
            let (Some(node), Some(rows)) = (self.layout.node(*id), self.virtual_rows(*id)) else {
                return false;
            };
            let viewport = Viewport {
                offset: node.scroll.map_or(0.0, |scroll| scroll.offset),
                height: Some(node.padding_box.height),
                pinned: None,
            };
            virtual_rows::visible_range(rows.heights(), viewport) != *built
        })
    }

    /// Scroll virtual list `list` so its row `index` is in view, from the
    /// row heights (the row needs not be built): with its extra height
    /// when that fits, else its top. Returns whether the offset changed.
    pub fn reveal_row(&mut self, list: WidgetId, index: usize) -> bool {
        let Some((top, bottom)) = self.virtual_rows(list).map(|rows| {
            let heights = rows.heights();
            let top = heights.top(index);
            (top, top + heights.height(index))
        }) else {
            return false;
        };
        let Some(node) = self.layout.node(list) else {
            return false;
        };
        let Some(scroll) = node.scroll else {
            return false;
        };
        let view = node.padding_box.height;
        let next = if top < scroll.offset {
            top
        } else if bottom > scroll.offset + view {
            (bottom - view).min(top)
        } else {
            scroll.offset
        };
        let next = next.clamp(0.0, scroll.max);
        if next == scroll.offset {
            return false;
        }
        self.scroll.insert(
            list,
            Point {
                y: next,
                ..scroll.offsets()
            },
        );
        self.invalidate();
        true
    }

    /// ↑/↓, Home/End and PageUp/PageDown on a row of a virtual list: build
    /// and reveal the row they reach and focus it (its first focusable
    /// element), activating it in a selecting group. Text fields keep
    /// their keys.
    fn row_key(&mut self, intent: &Intent) -> bool {
        let Intent::Text(TextIntent::KeyDown { key, modifiers, .. }) = intent else {
            return false;
        };
        let Some(focus) = self.focused() else {
            return false;
        };
        if *modifiers != Modifiers::NONE || self.focus_order.kind(focus) != Some(FocusKind::Button)
        {
            return false;
        }
        let Some(path) = self.tree.path_to(focus) else {
            return false;
        };
        if virtual_rows::in_own_tab_stops(&self.tree, &path) {
            return false;
        }
        let Some((list, index)) = virtual_rows::row_on_path(&self.tree, &path) else {
            return false;
        };
        let Some(rows) = self.virtual_rows(list) else {
            return false;
        };
        let heights = rows.heights();
        let view = self
            .layout
            .node(list)
            .map_or(heights.row, |node| node.padding_box.height);
        let last = rows.count.saturating_sub(1);
        let target = match key {
            Key::ArrowUp => index.saturating_sub(1),
            Key::ArrowDown => (index + 1).min(last),
            Key::Home => 0,
            Key::End => last,
            Key::PageUp => heights
                .index_at(heights.top(index) - view)
                .min(index.saturating_sub(1)),
            Key::PageDown => heights
                .index_at(heights.top(index) + view)
                .max(index + 1)
                .min(last),
            _ => return false,
        };
        if target == index {
            return true;
        }
        self.reveal_row(list, target);
        self.row_pin = Some((list, target));
        self.rebuild();
        self.relayout();
        let Some(row) = self.tree.find(list).and_then(|list| {
            list.children
                .iter()
                .find(|row| row.row_index == Some(target))
        }) else {
            return true;
        };
        let focus = self.focus_order.ids().find(|id| row.find(*id).is_some());
        let Some(focus) = focus else {
            return true;
        };
        self.apply_intent(Intent::SetFocus(Some(focus)));
        if self.focus_order.selects(focus)
            && let Some(msg) = self
                .handler(focus)
                .and_then(|handlers| handlers.on_click.clone())
        {
            self.send(msg);
        }
        self.invalidate();
        true
    }

    /// Follow the app's [`Element::modal`] panel: remember the focus when
    /// one appears and give it back when it leaves.
    fn track_modal(&mut self) {
        let panel = self.tree.last_modal().map(|panel| panel.id);
        match (self.modal.as_mut(), panel) {
            (None, Some(panel)) => {
                self.modal = Some(AppModal {
                    panel,
                    previous_focus: self.focused(),
                });
                self.modal_focus = Some(ModalFocus::First);
            }
            (Some(modal), Some(panel)) if modal.panel != panel => {
                modal.panel = panel;
                self.modal_focus = Some(ModalFocus::First);
            }
            (Some(_), None) => {
                let previous = self.modal.take().and_then(|modal| modal.previous_focus);
                self.modal_focus = Some(ModalFocus::Restore(previous));
            }
            _ => {}
        }
    }

    /// Whether the app shows an [`Element::modal`] panel.
    #[cfg(test)]
    pub fn modal_open(&self) -> bool {
        self.modal.is_some()
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
                    field.set_height(font_system, line.height);
                }
            }
        }
        for (id, camera) in &mut self.maps {
            if let Some(node) = self.layout.node(*id) {
                let size = Size {
                    width: node.padding_box.width,
                    height: node.padding_box.height,
                };
                if size != camera.size
                    && matches!(&node.content, Content::Map(spec)
                        if spec.source.is_some() || spec.fit.is_some() || !spec.layers.is_empty())
                {
                    self.maps_resized = true;
                }
                camera.size = size;
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
            file_drag: self.file_drag,
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
            let effects = self.app.update(msg);
            eprintln!("{}", format_msg(&source, &line, started.elapsed()));
            self.apply_effects(effects);
        } else {
            let effects = self.app.update(msg);
            self.apply_effects(effects);
        }
        self.sync_subscriptions();
        self.invalidate();
    }

    /// Run command `id` as a shortcut, the menu bar or the palette do: an
    /// app action sends its message when it is enabled, `kai.paleta` opens
    /// or closes the palette. Returns whether something ran: nothing does
    /// while the confirmation dialog is open.
    pub fn perform_command(&mut self, id: &str) -> bool {
        if self.confirmation_open() || self.modal.is_some() {
            return false;
        }
        self.close_popup();
        if id == PALETTE {
            if self.palette.is_some() {
                self.close_palette();
            } else {
                self.open_palette();
            }
            return true;
        }
        match self.command_message(id, serde_json::json!({})) {
            Ok(msg) => {
                let origin = self.command_origin(id);
                self.with_task_origin(origin, |session| session.send(msg));
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
        if self.command_busy(id) {
            return Err(CommandError::new(COMMAND_BUSY));
        }
        match availability {
            Availability::Enabled => {
                let origin = self.command_origin(id);
                self.with_source(MsgSource::Script, |session| {
                    session.with_task_origin(origin, |session| session.send(msg));
                });
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
        self.close_popup();
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

    /// Whether a pop-up list is open.
    #[cfg(test)]
    pub fn popup_open(&self) -> bool {
        self.popup.is_some()
    }

    /// Activating `id` opens its list: a pop-up button without `on_click`.
    fn opens_popup(&self, id: WidgetId) -> bool {
        self.handler(id)
            .is_some_and(|handlers| handlers.on_select.is_some() && handlers.on_click.is_none())
            && !self.tree.find(id).is_some_and(Element::has_context_menu)
    }

    /// Secondary button (right click) pressed at the pointer: the
    /// [`Element::context_menu`] of the innermost element under it that
    /// has one opens there.
    pub fn secondary_click(&mut self) {
        self.with_source(MsgSource::Pointer, |session| {
            session.open_context_menu_at_pointer();
        });
    }

    /// Open the context menu under the pointer; whether there was one.
    fn open_context_menu_at_pointer(&mut self) -> bool {
        self.dismiss_tooltip();
        let Some(point) = self.cursor else {
            return false;
        };
        let blocked = self.confirmation_open()
            || self.palette.is_some()
            || self
                .modal
                .as_ref()
                .is_some_and(|modal| !self.hover_path.contains(&modal.panel));
        let owner = self
            .hover_path
            .iter()
            .rev()
            .copied()
            .find(|id| self.tree.find(*id).is_some_and(Element::has_context_menu));
        let Some(owner) = owner.filter(|_| !blocked) else {
            return false;
        };
        self.close_popup();
        self.open_popup_at(owner, Some(point));
        true
    }

    fn open_popup(&mut self, owner: WidgetId) {
        self.open_popup_at(owner, None);
    }

    fn open_popup_at(&mut self, owner: WidgetId, at: Option<Point>) {
        let Some(options) = self
            .tree
            .find(owner)
            .and_then(|element| element.options.as_ref())
            .filter(|options| !options.items.is_empty())
        else {
            return;
        };
        let return_focus = if options.context {
            *self.ui.focused.get()
        } else {
            Some(owner)
        };
        self.popup = Some(Popup {
            owner,
            at,
            return_focus,
            highlighted: options.selected.unwrap_or(0),
            root: WidgetId(0),
            panel: WidgetId(0),
            rows: Vec::new(),
            typed: String::new(),
            typed_at: self.now(),
        });
        self.rebuild();
        self.relayout();
        self.focus_popup_row();
    }

    /// Close the open list, giving the focus back to its button.
    fn close_popup(&mut self) {
        let Some(popup) = self.popup.take() else {
            return;
        };
        self.rebuild();
        self.relayout();
        self.apply_intent(Intent::SetFocus(popup.return_focus));
        self.invalidate();
    }

    /// Close the list and send the button's `on_select(index)`; a list of
    /// check items stays open. Disabled items do nothing.
    fn choose_from_popup(&mut self, index: usize) {
        let Some(owner) = self.popup.as_ref().map(|popup| popup.owner) else {
            return;
        };
        let Some(options) = self
            .tree
            .find(owner)
            .and_then(|element| element.options.as_ref())
        else {
            return;
        };
        if options.is_disabled(index) {
            return;
        }
        let checks = options.checks();
        let on_select = self
            .handler(owner)
            .and_then(|handlers| handlers.on_select.clone());
        if checks {
            if let Some(popup) = &mut self.popup {
                popup.highlighted = index;
            }
        } else {
            self.close_popup();
        }
        if let Some(on_select) = on_select {
            self.send(on_select(index));
        }
        if checks {
            self.focus_popup_row();
        }
    }

    /// Focus the highlighted row, so assistive technology reads it.
    fn focus_popup_row(&mut self) {
        let row = self
            .popup
            .as_ref()
            .and_then(|popup| popup.rows.get(popup.highlighted).copied());
        if row.is_some() {
            self.apply_intent(Intent::SetFocus(row));
        }
        self.invalidate();
    }

    /// Append the open pop-up list to the tree, under its button. It closes
    /// when its button is gone, disabled or without options, and while the
    /// confirmation dialog is open.
    fn add_popup(&mut self) {
        let confirming = self.confirmation_open();
        let Some(popup) = &mut self.popup else {
            return;
        };
        let options = self
            .tree
            .find_mut(popup.owner)
            .filter(|element| !element.disabled && !confirming)
            .and_then(|element| element.options.as_mut())
            .filter(|options| !options.items.is_empty());
        let (Some(options), Some(anchor)) = (options, self.layout.node(popup.owner)) else {
            self.popup = None;
            return;
        };
        options.open = true;
        let options = options.clone();
        let count = options.items.len();
        popup.highlighted = popup.highlighted.min(count - 1);
        let anchor = popup.at.map_or(anchor.bounds, |at| Rect {
            x: at.x,
            y: at.y,
            width: 0.0,
            height: 0.0,
        });
        let frame = menu_frame(anchor, count, self.viewport);
        self.tree
            .children
            .push(select_menu(&options, popup.highlighted, frame));
        self.tree.assign_ids();
        popup.root = self
            .tree
            .children
            .last()
            .map_or(WidgetId(0), |root| root.id);
        let find = |test_id: &str| {
            self.tree
                .find_test_id(test_id)
                .map_or(WidgetId(0), |element| element.id)
        };
        popup.panel = find(MENU);
        popup.rows = (0..count)
            .map(|index| find(&format!("{MENU}.{index}")))
            .collect();
    }

    /// Keys the open pop-up list takes before anything else: arrows move
    /// the highlight, Return or Space pick it, Escape, ⌘. or Tab close the
    /// list, and typed letters highlight the option they begin
    /// ([`typeahead`]); other keys do nothing while it is open.
    fn popup_key(&mut self, intent: &Intent) -> bool {
        let Some(popup) = &self.popup else {
            return false;
        };
        let Intent::Text(text) = intent else {
            return false;
        };
        let last = popup.rows.len().saturating_sub(1);
        let action = match text {
            TextIntent::KeyDown {
                key,
                modifiers,
                repeat,
            } => match (key, *modifiers == Modifiers::NONE) {
                (Key::ArrowDown, true) => PopupKey::Highlight((popup.highlighted + 1).min(last)),
                (Key::ArrowUp, true) => PopupKey::Highlight(popup.highlighted.saturating_sub(1)),
                (Key::Home, true) => PopupKey::Highlight(0),
                (Key::End, true) => PopupKey::Highlight(last),
                (Key::Enter | Key::Space, true) if !repeat => PopupKey::Choose(popup.highlighted),
                (Key::Escape | Key::Tab, true) => PopupKey::Close,
                (Key::Period, false) if modifiers.command => PopupKey::Close,
                _ => PopupKey::Ignore,
            },
            // Space picks the option; it never searches.
            TextIntent::InsertText(text) if text.trim().is_empty() => PopupKey::Ignore,
            TextIntent::InsertText(text) => PopupKey::Find(text.clone()),
            _ => PopupKey::Ignore,
        };
        match action {
            PopupKey::Highlight(index) => {
                if let Some(popup) = &mut self.popup {
                    popup.highlighted = index;
                }
                self.focus_popup_row();
            }
            PopupKey::Find(text) => {
                let now = self.now();
                let items = self
                    .popup
                    .as_ref()
                    .and_then(|popup| self.tree.find(popup.owner))
                    .and_then(|element| element.options.as_ref())
                    .map(|options| options.items.clone())
                    .unwrap_or_default();
                let Some(popup) = &mut self.popup else {
                    return true;
                };
                if now.saturating_duration_since(popup.typed_at) > TYPEAHEAD_RESET {
                    popup.typed.clear();
                }
                popup.typed.push_str(&text);
                popup.typed_at = now;
                if let Some(index) = typeahead(&items, popup.highlighted, &popup.typed) {
                    popup.highlighted = index;
                    self.focus_popup_row();
                }
            }
            PopupKey::Choose(index) => self.choose_from_popup(index),
            PopupKey::Close => self.close_popup(),
            PopupKey::Ignore => {}
        }
        true
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

    /// Append the shown tooltip, before the palette and the dialog so those
    /// stay last.
    fn add_tooltip(&mut self) {
        let Some(tooltip) = self.tooltip.as_ref().filter(|tooltip| tooltip.shown) else {
            return;
        };
        let (Some(text), Some(node)) = (
            self.tooltips.get(&tooltip.anchor),
            self.layout.node(tooltip.anchor),
        ) else {
            return;
        };
        let anchor = node.visible_bounds().unwrap_or(node.bounds);
        let bubble = tooltip_bubble(text, tooltip_origin(anchor, text, self.viewport));
        self.tree.children.push(bubble);
        self.tree.assign_ids();
    }

    /// Follow the hover path: the innermost element under the pointer with
    /// a tooltip starts waiting for it, at once if another one was showing.
    fn update_tooltip(&mut self) {
        let anchor = self
            .hover_path
            .iter()
            .rev()
            .find(|id| self.tooltips.contains_key(id))
            .copied();
        if self.tooltip_pressed != anchor {
            self.tooltip_pressed = None;
        }
        let anchor = anchor.filter(|&id| self.tooltip_pressed != Some(id));
        if self.tooltip.as_ref().map(|tooltip| tooltip.anchor) == anchor {
            return;
        }
        let warm = self.tooltip.as_ref().is_some_and(|tooltip| tooltip.shown);
        if warm {
            self.invalidate();
        }
        let due = self.now() + TOOLTIP_DELAY;
        self.tooltip = anchor.map(|anchor| Tooltip {
            anchor,
            due,
            shown: warm,
        });
    }

    /// A press hides the tooltip until the pointer leaves its element.
    fn dismiss_tooltip(&mut self) {
        if let Some(tooltip) = self.tooltip.take() {
            self.tooltip_pressed = Some(tooltip.anchor);
            if tooltip.shown {
                self.invalidate();
            }
        }
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
            active_descendant: self.palette.as_ref().and_then(|palette| {
                let row = palette.rows.get(palette.selected)?.0;
                Some((palette.search, row))
            }),
        })
    }

    pub fn accessibility_action(&mut self, request: &ActionRequest) {
        if request.action == accesskit::Action::ShowContextMenu {
            if let Some(id) = a11y::widget_id(request.target_node)
                .filter(|id| self.tree.find(*id).is_some_and(Element::has_context_menu))
            {
                self.close_popup();
                self.open_popup(id);
            }
            return;
        }
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
        let changed = path != self.hover_path;
        if changed {
            self.hover_path = path;
            self.update_tooltip();
            self.update_held_timers();
        }
        let area = self.area_at_pointer();
        let area_changed = area != self.map_hover;
        if area_changed {
            self.map_hover = area;
            self.invalidate();
        }
        let cursor = if self.map_hover.is_some() {
            Cursor::Pointer
        } else {
            cursor_for(&self.layout, &self.hover_path)
        };
        // The control under the pointer can change its cursor without
        // moving: it turned busy, or its task delivered.
        if changed || area_changed || cursor != self.pointer_cursor {
            self.pointer_cursor = cursor;
            self.outbox.cursor = Some(cursor);
        }
        changed || area_changed
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
        #[cfg(feature = "document")]
        if let Some(ed) = self.documents.get(&id) {
            let Some(viewport) = self.layout.text_box(id) else {
                return;
            };
            let paint = ed.paint();
            let caret = paint.caret.unwrap_or(Rect {
                height: 16.0,
                ..Rect::default()
            });
            let m = crate::document::viewport_metrics(viewport, paint.page_size);
            self.outbox.ime_cursor_area = Some(Rect {
                x: m.origin_x + caret.x * m.scale,
                y: m.origin_y + caret.y * m.scale - paint.scroll_y,
                width: 0.0,
                height: (caret.height * m.scale).max(1.0),
            });
            return;
        }
        let (Some(line), Some(field)) = (self.layout.text_box(id), self.text_fields.get_mut(&id))
        else {
            return;
        };
        let caret = field
            .view(&mut self.text.font_system)
            .decorations
            .caret
            .unwrap_or(Rect {
                height: line.height,
                ..Rect::default()
            });
        self.outbox.ime_cursor_area = Some(Rect {
            x: line.x + caret.x,
            y: line.y + caret.y,
            width: 0.0,
            height: caret.height,
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
        if self.palette_key(&intent)
            || self.popup_key(&intent)
            || self.row_key(&intent)
            || self.map_key(&intent)
            || self.vertex_key(&intent)
        {
            return;
        }
        let previous_focus = *self.ui.focused.get();
        let previous_text_field = self.focused_text_field();
        let result = dispatch(&mut self.ui, &self.focus_order, intent);
        let current_focus = *self.ui.focused.get();
        if previous_focus != current_focus {
            self.update_held_timers();
            if let Some(id) = current_focus {
                self.layout
                    .reveal(id, FOCUS_REVEAL_MARGIN, &mut self.scroll);
            }
            let text_field = self.focused_text_field();
            self.outbox.ime_allowed = Some(text_field.is_some());
            if let Some(id) = text_field {
                self.reset_caret_blink();
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
            && let Some(index) = self
                .popup
                .as_ref()
                .and_then(|popup| popup.rows.iter().position(|row| *row == activation.widget))
        {
            self.choose_from_popup(index);
        } else if let Some(activation) = result.activated
            && self.opens_popup(activation.widget)
        {
            self.open_popup(activation.widget);
        } else if let Some(activation) = result.activated
            && let Some((map, control)) = self.map_control(activation.widget)
        {
            let now = self.now();
            if let Some(camera) = self.maps.get_mut(&map) {
                let center = Point {
                    x: camera.size.width / 2.0,
                    y: camera.size.height / 2.0,
                };
                camera.zoom_by(control.delta(), center, now);
            }
            self.invalidate();
        } else if let Some(activation) = result.activated
            && self.busy(activation.widget)
        {
            // A busy control takes no clicks until its task delivers.
        } else if let Some(activation) = result.activated
            && let Some(msg) = self
                .handler(activation.widget)
                .and_then(|handlers| handlers.on_click.clone())
        {
            let origin = Some(TaskOrigin::Widget(activation.widget));
            self.with_task_origin(origin, |session| session.send(msg));
        } else if let Some(activation) = result.activated
            && let Some(id) = self
                .tree
                .find(activation.widget)
                .and_then(|element| element.command)
        {
            self.perform_command(id);
        }
        if let Some(Action::Command(id)) = result.action {
            self.perform_command(id);
        } else if self.confirmation_open() && result.action == Some(Action::CancelOperation) {
            self.decide(calls::Decision::Deny);
        } else if self.palette.is_some() && result.action == Some(Action::CancelOperation) {
            self.close_palette();
        } else if matches!(result.action, Some(Action::MoveUp | Action::MoveDown))
            && let Some(id) = current_focus.filter(|id| self.opens_popup(*id))
        {
            self.open_popup(id);
        } else if let Some(action) = result.action {
            // Toolbar/ribbon often steals focus; Edit ⌘C/X/V still target the
            // text field or sole document that owns the selection (AppKit-like).
            let routed_clipboard = matches!(
                action,
                Action::Copy | Action::Cut | Action::Paste
            ) && self.clipboard_edit_target(current_focus).is_some_and(|id| {
                let intent = match action {
                    Action::Copy => TextIntent::Copy,
                    Action::Cut => TextIntent::Cut,
                    Action::Paste => TextIntent::Paste,
                    _ => unreachable!(),
                };
                self.apply_text_intent(id, intent);
                true
            });
            if !routed_clipboard
                && let Some(msg) = action_message(&self.tree, current_focus, action)
            {
                self.send(msg);
            }
        }
        if result.changed || current_focus != previous_focus {
            self.invalidate();
        }
    }

    /// Widget that should receive Edit clipboard actions for the current focus.
    fn clipboard_edit_target(&self, focused: Option<WidgetId>) -> Option<WidgetId> {
        if let Some(id) = focused {
            if self.text_fields.contains_key(&id) {
                return Some(id);
            }
            #[cfg(feature = "document")]
            if self.documents.contains_key(&id) {
                return Some(id);
            }
        }
        #[cfg(feature = "document")]
        if self.documents.len() == 1 {
            return self.documents.keys().next().copied();
        }
        None
    }

    fn apply_text_intent(&mut self, id: WidgetId, intent: TextIntent) {
        #[cfg(feature = "document")]
        if self.documents.contains_key(&id) {
            self.apply_document_intent(id, intent);
            return;
        }
        let Some(field) = self.text_fields.get_mut(&id) else {
            return;
        };
        // Return submits one line; several lines take ⌘Return.
        let submit = matches!(
            intent,
            TextIntent::KeyDown {
                key: Key::Enter,
                repeat: false,
                modifiers,
            } if modifiers.command || !field.is_multiline()
        );
        let before = field.text();
        let effect = field.apply(intent, &mut self.text.font_system, &mut *self.clipboard);
        let after = field.text();
        if let Some(status) = effect.status {
            eprintln!("{status}");
        }
        if effect.changed {
            self.reset_caret_blink();
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

    /// Give every map of the new tree its camera (new maps start at the
    /// app's view), let a changed app view move it, and show the camera's
    /// view; cameras of maps that left are dropped.
    fn sync_maps(&mut self) {
        let now = self.now();
        let maps = &mut self.maps;
        let tiles = &mut self.tiles;
        let tasks = &mut self.tasks;
        let labels = &mut self.labels;
        let mut fixed = FixedMeasure;
        let measure: &mut dyn TextMeasure = match self.measure {
            Measure::Real => &mut self.text,
            Measure::Fixed => &mut fixed,
        };
        let map_hover = &self.map_hover;
        let mut live = Vec::new();
        let mut revealing = false;
        let mut fading = false;
        self.tree.walk_mut(&mut |element| {
            if let Kind::Map(spec) = &mut element.kind {
                let camera = maps
                    .entry(element.id)
                    .or_insert_with(|| CameraState::new(spec.view));
                camera.sync_model(spec.view, spec.animate, now);
                if let Some(request) = spec.fit {
                    camera.fit(request, spec.animate, now);
                }
                spec.view = camera.view;
                spec.hovered = map_hover
                    .as_ref()
                    .filter(|(map, _)| *map == element.id)
                    .map(|(_, area)| area.clone());
                let view = Camera::new(camera.view, camera.size);
                if let Some(source) = &spec.source {
                    let (frame, loads) = tiles.frame(source, view);
                    for load in loads {
                        let id = tasks.spawn(load.run);
                        tasks.tiles.insert(id, (load.source, load.tile));
                    }
                    revealing |= frame.revealing;
                    spec.tiles = frame;
                }
                if spec.source.is_some() || !spec.layers.is_empty() {
                    let map_labels = labels.entry(element.id).or_default();
                    map_labels.update(
                        element.id,
                        view,
                        &spec.tiles,
                        &spec.layers,
                        camera.still(now),
                        now,
                        measure,
                    );
                    let (shown, fades) = map_labels.frame(element.id, now);
                    fading |= fades;
                    spec.labels = shown;
                }
                live.push(element.id);
            }
        });
        maps.retain(|id, _| live.contains(id));
        labels.retain(|id, _| live.contains(id));
        if self.map_drag.is_some_and(|id| !live.contains(&id)) {
            self.map_drag = None;
        }
        if self.map_press.is_some_and(|(id, _)| !live.contains(&id)) {
            self.map_press = None;
        }
        if self
            .vertex_drag
            .is_some_and(|drag| !live.contains(&drag.map))
        {
            self.vertex_drag = None;
        }
        if self
            .map_hover
            .as_ref()
            .is_some_and(|(id, _)| !live.contains(id))
        {
            self.map_hover = None;
        }
        self.tiles_revealing = revealing;
        self.labels_fading = fading;
        if revealing || fading {
            self.invalidate();
        }
    }

    /// Advance map animations to now, keeping frames coming while one
    /// runs, and report the gestures that ended.
    fn step_maps(&mut self) {
        let now = self.now();
        let mut animating = false;
        for camera in self.maps.values_mut() {
            camera.step(now);
            animating |= camera.animating();
        }
        if animating {
            self.invalidate();
        }
        self.report_maps();
    }

    /// Whether a map glides or zooms on its own, its labels fade in, an
    /// element in view moves ([`Motion`](crate::element::Motion)), or a text
    /// caret is blinking: the window needs frames.
    pub fn animating(&self) -> bool {
        self.labels_fading
            || self.maps.values().any(CameraState::animating)
            || self.moving()
            || self.caret_blinking()
    }

    /// Text focus with blink enabled: frames keep the insertion point toggling.
    fn caret_blinking(&self) -> bool {
        self.focused_text_field().is_some() && !self.theme.prefs.reduce_motion
    }

    /// Reset the blink phase to visible (typing, caret move, or focus).
    fn reset_caret_blink(&mut self) {
        self.caret_blink_at = self.now();
    }

    /// Whether the focused text caret should paint this frame.
    fn caret_is_visible(&self) -> bool {
        if self.focused_text_field().is_none() {
            return true;
        }
        let elapsed = self.now().saturating_duration_since(self.caret_blink_at);
        caret_blink_visible(elapsed, self.theme.prefs.reduce_motion)
    }

    /// Send `on_view_change` for every map whose gesture settled; returns
    /// whether any did.
    fn report_maps(&mut self) -> bool {
        let now = self.now();
        let mut reports: Vec<(WidgetId, MapView)> = self
            .maps
            .iter_mut()
            .filter_map(|(id, camera)| camera.take_report(now).map(|view| (*id, view)))
            .collect();
        reports.sort_by_key(|(id, _)| *id);
        let mut sent = false;
        for (id, view) in reports {
            self.invalidate();
            if let Some(handler) = self
                .handler(id)
                .and_then(|handlers| handlers.on_view_change.clone())
            {
                let source = self.source.clone().unwrap_or(MsgSource::Pointer);
                self.with_source(source, |session| session.send(handler(view)));
                sent = true;
            }
        }
        sent
    }

    /// The pointer in map `id`'s local points.
    fn map_local(&self, id: WidgetId) -> Option<Point> {
        let cursor = self.cursor?;
        let area = self.layout.node(id)?.padding_box;
        Some(Point {
            x: cursor.x - area.x,
            y: cursor.y - area.y,
        })
    }

    /// The innermost map under the pointer and the pointer in its points.
    fn map_at_pointer(&self) -> Option<(WidgetId, Point)> {
        let id = self
            .hover_path
            .iter()
            .rev()
            .copied()
            .find(|id| self.maps.contains_key(id))?;
        Some((id, self.map_local(id)?))
    }

    /// A press on a map (not on a control inside it): one starts a drag,
    /// a double click zooms in (out with ⇧) around the pointer.
    fn press_map(&mut self, target: Option<WidgetId>, clicks: u32) {
        let Some((id, local)) = self.map_at_pointer() else {
            return;
        };
        if target != Some(id) {
            return;
        }
        let now = self.now();
        let shift = self.modifiers.shift;
        let Some(camera) = self.maps.get_mut(&id) else {
            return;
        };
        if clicks >= 2 {
            camera.zoom_by(if shift { -1.0 } else { 1.0 }, local, now);
        } else {
            camera.press(local, now);
            self.map_drag = Some(id);
            self.map_press = Some((id, local));
        }
    }

    /// The clickable layer under the pointer: on a map with
    /// `on_area_click`, not over a marker or control inside it.
    fn area_at_pointer(&self) -> Option<(WidgetId, String)> {
        let (id, local) = self.map_at_pointer()?;
        if pointer_target(&self.layout, &self.hover_path) != Some(id)
            || self.handler(id)?.on_area_click.is_none()
        {
            return None;
        }
        let Content::Map(spec) = &self.layout.node(id)?.content else {
            return None;
        };
        let camera = self.maps.get(&id)?;
        let view = Camera::new(camera.view, camera.size);
        let layer = crate::maps::layers::hit(&spec.layers, &view, local)?;
        Some((id, layer.id.clone()))
    }

    /// The innermost chart under the pointer.
    fn chart_at_pointer(&self) -> Option<WidgetId> {
        self.hover_path.iter().rev().copied().find(|id| {
            matches!(
                self.layout.node(*id).map(|node| &node.content),
                Some(Content::Chart(_))
            )
        })
    }

    /// A press on a bar chart that has `on_select`.
    fn press_chart(&mut self) {
        let Some(id) = self.chart_at_pointer() else {
            return;
        };
        if self
            .handler(id)
            .is_none_or(|handlers| handlers.on_select.is_none())
        {
            return;
        }
        let Some(cursor) = self.cursor else {
            return;
        };
        self.chart_press = Some((id, cursor));
    }

    /// A release near where the pointer went down on a bar chart selects
    /// the category under it.
    fn click_chart(&mut self) {
        let Some((id, pressed)) = self.chart_press.take() else {
            return;
        };
        let Some(cursor) = self.cursor else {
            return;
        };
        if (cursor.x - pressed.x).hypot(cursor.y - pressed.y) > MAP_CLICK_SLOP {
            return;
        }
        let Some(node) = self.layout.node(id) else {
            return;
        };
        let Content::Chart(spec) = &node.content else {
            return;
        };
        let Some(index) = crate::charts::hit(node.padding_box, spec, cursor) else {
            return;
        };
        if let Some(handler) = self
            .handler(id)
            .and_then(|handlers| handlers.on_select.clone())
        {
            let source = self.source.clone().unwrap_or(MsgSource::Pointer);
            self.with_source(source, |session| session.send(handler(index)));
        }
    }

    /// A release near where the pointer went down on a map clicks the
    /// layer under it.
    fn click_map_area(&mut self) {
        let Some((id, pressed)) = self.map_press.take() else {
            return;
        };
        let Some(local) = self.map_local(id) else {
            return;
        };
        if (local.x - pressed.x).hypot(local.y - pressed.y) > MAP_CLICK_SLOP {
            return;
        }
        let Some((map, area)) = self.area_at_pointer() else {
            return;
        };
        if let Some(handler) = self
            .handler(map)
            .and_then(|handlers| handlers.on_area_click.clone())
        {
            let source = self.source.clone().unwrap_or(MsgSource::Pointer);
            self.with_source(source, |session| session.send(handler(area)));
        }
    }

    /// End a map drag: the map may glide on, else the gesture reports.
    fn release_map(&mut self) {
        self.map_press = None;
        let Some(id) = self.map_drag.take() else {
            return;
        };
        let now = self.now();
        if let Some(camera) = self.maps.get_mut(&id) {
            camera.release(now);
        }
        self.report_maps();
    }

    /// The map a zoom button belongs to, and the button.
    fn map_control(&self, widget: WidgetId) -> Option<(WidgetId, crate::maps::MapControl)> {
        let control = self.tree.find(widget)?.map_control?;
        let path = self.tree.path_to(widget)?;
        let map = path
            .iter()
            .rev()
            .copied()
            .find(|id| self.maps.contains_key(id))?;
        Some((map, control))
    }

    /// The polygon editor vertex `widget` is: its map, its index and
    /// where it stands.
    fn map_vertex(&self, widget: WidgetId) -> Option<(WidgetId, usize, LatLon)> {
        let element = self.tree.find(widget)?;
        let index = element.map_vertex?;
        let at = element.marker?.at;
        let path = self.tree.path_to(widget)?;
        let map = path
            .iter()
            .rev()
            .copied()
            .find(|id| self.maps.contains_key(id))?;
        Some((map, index, at))
    }

    /// Hand `edit` to the `on_edit` of `map`'s polygon editor.
    fn send_edit(&mut self, map: WidgetId, edit: PolygonEdit, source: MsgSource) {
        if let Some(handler) = self.handler(map).and_then(|h| h.on_edit.clone()) {
            let source = self.source.clone().unwrap_or(source);
            self.with_source(source, |session| session.send(handler(edit)));
        }
    }

    /// A press on a polygon editor vertex starts dragging it; the map
    /// stops gliding so the vertex lands where the pointer is.
    fn press_vertex(&mut self, target: Option<WidgetId>) {
        let Some((map, index, _)) = target.and_then(|id| self.map_vertex(id)) else {
            return;
        };
        let (Some(cursor), Some(node)) = (self.cursor, target.and_then(|id| self.layout.node(id)))
        else {
            return;
        };
        let handle = node.padding_box;
        if let Some(camera) = self.maps.get_mut(&map) {
            camera.halt();
        }
        self.vertex_drag = Some(VertexDrag {
            map,
            index,
            grab: Point {
                x: cursor.x - (handle.x + handle.width / 2.0),
                y: cursor.y - (handle.y + handle.height / 2.0),
            },
            moved: false,
        });
    }

    /// The pointer moved during a vertex drag: the vertex follows it.
    fn drag_vertex(&mut self) {
        let Some(drag) = self.vertex_drag else {
            return;
        };
        let (Some(local), Some(camera)) = (self.map_local(drag.map), self.maps.get(&drag.map))
        else {
            return;
        };
        let at = Camera::new(camera.view, camera.size).to_lat_lon(Point {
            x: local.x - drag.grab.x,
            y: local.y - drag.grab.y,
        });
        self.vertex_drag = Some(VertexDrag {
            moved: true,
            ..drag
        });
        self.send_edit(
            drag.map,
            PolygonEdit::Move {
                index: drag.index,
                to: at,
                first: !drag.moved,
            },
            MsgSource::Pointer,
        );
    }

    /// With a polygon editor vertex focused, the arrows move it (⇧: ten
    /// times as far), Delete removes it and Return finishes the edit.
    fn vertex_key(&mut self, intent: &Intent) -> bool {
        let Intent::Text(TextIntent::KeyDown {
            key,
            modifiers,
            repeat,
        }) = intent
        else {
            return false;
        };
        if modifiers.command || modifiers.control || modifiers.option {
            return false;
        }
        let Some((map, index, at)) = self.focused().and_then(|id| self.map_vertex(id)) else {
            return false;
        };
        let step = if modifiers.shift {
            VERTEX_KEY_STEP * 10.0
        } else {
            VERTEX_KEY_STEP
        };
        let offset = match key {
            Key::ArrowLeft => Some((-step, 0.0)),
            Key::ArrowRight => Some((step, 0.0)),
            Key::ArrowUp => Some((0.0, -step)),
            Key::ArrowDown => Some((0.0, step)),
            _ => None,
        };
        let edit = if let Some((dx, dy)) = offset {
            let Some(camera) = self.maps.get(&map) else {
                return false;
            };
            let camera = Camera::new(camera.view, camera.size);
            let local = camera.to_local(at);
            PolygonEdit::Move {
                index,
                to: camera.to_lat_lon(Point {
                    x: local.x + dx,
                    y: local.y + dy,
                }),
                first: true,
            }
        } else if *repeat {
            return false;
        } else {
            match key {
                Key::Backspace | Key::Delete if !modifiers.shift => PolygonEdit::Remove { index },
                Key::Enter => PolygonEdit::Finish,
                _ => return false,
            }
        };
        self.send_edit(map, edit, MsgSource::Keyboard);
        true
    }

    /// Arrow keys pan a focused map and `+`/`−` zoom it.
    fn map_key(&mut self, intent: &Intent) -> bool {
        let Intent::Text(TextIntent::KeyDown { key, modifiers, .. }) = intent else {
            return false;
        };
        if modifiers.command || modifiers.control || modifiers.option {
            return false;
        }
        let Some(id) = self.focused().filter(|id| self.maps.contains_key(id)) else {
            return false;
        };
        let now = self.now();
        let Some(camera) = self.maps.get_mut(&id) else {
            return false;
        };
        let center = Point {
            x: camera.size.width / 2.0,
            y: camera.size.height / 2.0,
        };
        match key {
            Key::ArrowLeft => camera.pan(MAP_KEY_STEP, 0.0, now),
            Key::ArrowRight => camera.pan(-MAP_KEY_STEP, 0.0, now),
            Key::ArrowUp => camera.pan(0.0, MAP_KEY_STEP, now),
            Key::ArrowDown => camera.pan(0.0, -MAP_KEY_STEP, now),
            Key::Char('+' | '=') => camera.zoom_by(1.0, center, now),
            Key::Char('-' | '_') => camera.zoom_by(-1.0, center, now),
            _ => return false,
        }
        self.invalidate();
        true
    }

    /// Modifier keys held now.
    pub fn set_modifiers(&mut self, modifiers: Modifiers) {
        self.modifiers = modifiers;
    }

    /// A wheel or trackpad scroll of `delta` points (positive moves the
    /// content right and down). Over a map, pixel deltas pan it and wheel
    /// notches (`lines`) or ⌘ zoom it a level per notch around the
    /// pointer; elsewhere it scrolls down ([`Self::scroll_by`]) and across
    /// ([`Self::scroll_by_x`]). With Shift, a wheel that only turns
    /// vertically scrolls across, as in AppKit.
    pub fn wheel(&mut self, delta: Point, lines: bool) {
        if let Some((id, local)) = self.map_at_pointer() {
            let now = self.now();
            let zoom = lines || self.modifiers.command;
            if let Some(camera) = self.maps.get_mut(&id) {
                if zoom {
                    if delta.y != 0.0 {
                        camera.zoom_by(f64::from(delta.y.signum()), local, now);
                    }
                } else {
                    camera.pan(f64::from(delta.x), f64::from(delta.y), now);
                }
            }
            self.invalidate();
            return;
        }
        let delta = if self.modifiers.shift && delta.x == 0.0 {
            Point { x: delta.y, y: 0.0 }
        } else {
            delta
        };
        if delta.y != 0.0 {
            self.scroll_by(delta.y);
        }
        if delta.x != 0.0 {
            self.scroll_by_x(delta.x);
        }
    }

    /// A trackpad pinch over a map: scale by `1 + delta` around the
    /// pointer; `ended` reports the gesture at once.
    pub fn pinch(&mut self, delta: f64, ended: bool) {
        let Some((id, local)) = self.map_at_pointer() else {
            return;
        };
        let now = self.now();
        if let Some(camera) = self.maps.get_mut(&id) {
            camera.pinch(1.0 + delta, local, now);
            if ended {
                camera.end_gesture();
            }
        }
        self.invalidate();
        self.with_source(MsgSource::Pointer, |session| {
            session.report_maps();
        });
    }

    /// Scroll the innermost scroll container under the pointer by `delta_y`
    /// logical points (positive moves the content down / toward the start;
    /// negative toward the end), matching [`Self::wheel`] and layout scrollers.
    pub fn scroll_by(&mut self, delta_y: f32) {
        #[cfg(feature = "document")]
        if self.scroll_document(delta_y) {
            return;
        }
        self.scroll_along(Axis::Y, delta_y);
    }

    /// Scroll the innermost horizontal scroll container under the pointer
    /// by `delta_x` logical points (positive moves the content right).
    pub fn scroll_by_x(&mut self, delta_x: f32) {
        self.scroll_along(Axis::X, delta_x);
    }

    fn scroll_along(&mut self, axis: Axis, delta: f32) {
        let Some((id, scroll)) = self.layout.scroll_target(&self.hover_path, axis, -delta) else {
            return;
        };
        let offset = scroll.offset_on(axis);
        let next = (offset - delta).clamp(0.0, scroll.max_on(axis));
        if next != offset {
            let mut offsets = scroll.offsets();
            match axis {
                Axis::X => offsets.x = next,
                Axis::Y => offsets.y = next,
            }
            self.scroll.insert(id, offsets);
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
            self.vertex_drag = None;
            self.release_map();
            self.invalidate();
            return;
        }
        let mut redraw = self.refresh_hover();
        self.drag_vertex();
        if let Some(id) = self.map_drag
            && let Some(local) = self.map_local(id)
        {
            let now = self.now();
            if let Some(camera) = self.maps.get_mut(&id) {
                redraw |= camera.drag_to(local, now);
            }
        }
        if let Some(popup) = &mut self.popup
            && let Some(index) = popup
                .rows
                .iter()
                .position(|row| self.hover_path.contains(row))
            && index != popup.highlighted
        {
            popup.highlighted = index;
            redraw = true;
        }
        if let Some(id) = self.dragging
            && let Some(local) = self.text_field_local(id)
        {
            #[cfg(feature = "document")]
            if let Some(ed) = self.documents.get_mut(&id) {
                if let Some(page) = self.layout.text_box(id) {
                    let page_local = crate::geometry::Rect {
                        x: 0.0,
                        y: 0.0,
                        width: page.width,
                        height: page.height,
                    };
                    if ed.drag_to(local, page_local) {
                        self.report_document_change(id);
                        self.reset_caret_blink();
                        self.update_ime_cursor_area();
                        redraw = true;
                    }
                }
            } else if let Some(field) = self.text_fields.get_mut(&id) {
                field.drag(&mut self.text.font_system, local);
                self.reset_caret_blink();
                self.update_ime_cursor_area();
                redraw = true;
            }
            #[cfg(not(feature = "document"))]
            if let Some(field) = self.text_fields.get_mut(&id) {
                field.drag(&mut self.text.font_system, local);
                self.reset_caret_blink();
                self.update_ime_cursor_area();
                redraw = true;
            }
        }
        if redraw {
            self.invalidate();
        }
    }

    /// Primary button pressed or released at the current pointer.
    pub fn pointer_button(&mut self, pressed: bool) {
        self.pointer_clicks(pressed, 1);
    }

    /// [`Self::pointer_button`] for the `clicks`th press of a series
    /// (2 for a double click, 3 for a triple one).
    pub fn pointer_clicks(&mut self, pressed: bool, clicks: u32) {
        self.with_source(MsgSource::Pointer, |session| {
            session.press_pointer(pressed, clicks);
        });
    }

    /// Last pointer position, in logical points.
    pub fn cursor(&self) -> Option<Point> {
        self.cursor
    }

    fn press_pointer(&mut self, pressed: bool, clicks: u32) {
        if pressed {
            self.dismiss_tooltip();
        }
        if !pressed && std::mem::take(&mut self.context_press) {
            return;
        }
        if pressed
            && self.modifiers.control
            && !self.modifiers.command
            && self.open_context_menu_at_pointer()
        {
            self.context_press = true;
            return;
        }
        let target = pointer_target(&self.layout, &self.hover_path);
        if pressed
            && let Some(popup) = &self.popup
            && !self.hover_path.contains(&popup.panel)
        {
            self.close_popup();
            return;
        }
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
        if pressed
            && self.popup.is_none()
            && self.palette.is_none()
            && let Some(modal) = &self.modal
            && !self.hover_path.contains(&modal.panel)
        {
            return;
        }
        if pressed {
            let previous_focus = *self.ui.focused.get();
            self.apply_intent(Intent::PointerDown(target));
            self.press_map(target, clicks);
            self.press_vertex(target);
            self.press_chart();
            let selected_on_focus = target
                .is_some_and(|id| previous_focus != Some(id) && selects_on_focus(&self.tree, id));
            self.dragging = target.filter(|_| self.is_text_field(target) && !selected_on_focus);
            if let Some(id) = self.dragging
                && let Some(local) = self.text_field_local(id)
            {
                #[cfg(feature = "document")]
                if let Some(ed) = self.documents.get_mut(&id) {
                    if let Some(page) = self.layout.text_box(id) {
                        let page_local = crate::geometry::Rect {
                            x: 0.0,
                            y: 0.0,
                            width: page.width,
                            height: page.height,
                        };
                        ed.click(local, page_local, clicks);
                        self.report_document_change(id);
                        self.reset_caret_blink();
                        self.update_ime_cursor_area();
                    }
                } else if let Some(field) = self.text_fields.get_mut(&id) {
                    field.click_count(&mut self.text.font_system, local, clicks);
                    self.reset_caret_blink();
                    self.update_ime_cursor_area();
                }
                #[cfg(not(feature = "document"))]
                if let Some(field) = self.text_fields.get_mut(&id) {
                    field.click_count(&mut self.text.font_system, local, clicks);
                    self.reset_caret_blink();
                    self.update_ime_cursor_area();
                }
            }
        } else {
            self.apply_intent(Intent::PointerUp(target));
            self.dragging = None;
            self.vertex_drag = None;
            self.click_map_area();
            self.click_chart();
            self.release_map();
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
            self.vertex_drag = None;
            self.release_map();
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
        self.step_maps();
        self.rebuild();
        if let Some(focus) = self.modal_focus.take() {
            self.relayout();
            let target = match focus {
                ModalFocus::First => self.focus_order.next(None, false),
                ModalFocus::Restore(previous) => previous,
            };
            self.apply_intent(Intent::SetFocus(target));
            self.rebuild();
        }
        let built = Instant::now();
        self.relayout();
        if self.rows_stale() {
            self.keep_scroll_offsets();
            self.rebuild();
            self.relayout();
        }
        if std::mem::take(&mut self.maps_resized) {
            self.rebuild();
            self.relayout();
        }
        if self.refresh_hover() {
            self.relayout();
        }
        self.frame = FrameTimes {
            view: built - started,
            layout: built.elapsed(),
            ..FrameTimes::default()
        };
        self.keep_scroll_offsets();
        self.start_image_loads();

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

    /// Remember the offsets of the last layout (clamped to its content)
    /// and forget containers that are gone.
    fn keep_scroll_offsets(&mut self) {
        for (id, scroll) in self
            .layout
            .nodes()
            .iter()
            .filter_map(|n| n.scroll.map(|s| (n.id, s)))
        {
            self.scroll.insert(id, scroll.offsets());
        }
        self.scroll.retain(|id, _| {
            self.layout
                .node(*id)
                .is_some_and(|node| node.scroll.is_some())
        });
    }

    #[cfg(feature = "document")]
    fn document_at_pointer(&self) -> Option<WidgetId> {
        self.hover_path.iter().rev().find(|id| self.documents.contains_key(id)).copied()
    }

    /// Rueda sobre el documento: scroll interno de páginas. `true` si consumió el delta.
    #[cfg(feature = "document")]
    fn scroll_document(&mut self, delta_y: f32) -> bool {
        let Some(id) = self.document_at_pointer() else {
            return false;
        };
        let Some(viewport) = self.layout.text_box(id) else {
            return false;
        };
        let Some(ed) = self.documents.get_mut(&id) else {
            return false;
        };
        if !ed.scroll_by(delta_y, viewport) {
            return false;
        }
        self.update_ime_cursor_area();
        self.invalidate();
        true
    }

    #[cfg(feature = "document")]
    fn sync_documents(&mut self) {
        let focused = *self.ui.focused.get();
        let mut live = Vec::new();
        let mut changes: Vec<(WidgetId, crate::document::DocumentChange)> = Vec::new();
        let mut specs: Vec<(
            WidgetId,
            String,
            Option<crate::document::DocumentCmdRequest>,
            Option<crate::document::DocumentLoadRequest>,
        )> = Vec::new();
        self.tree.walk(&mut |element| {
            if let Kind::Document(spec) = &element.kind {
                specs.push((
                    element.id,
                    spec.markdown.clone(),
                    spec.pending_cmd.clone(),
                    spec.pending_load.clone(),
                ));
            }
        });
        for (id, markdown, pending, pending_load) in specs {
            let ed = self
                .documents
                .entry(id)
                .or_insert_with(|| DocumentEditor::new(&markdown));
            ed.sync_markdown(&markdown);
            if let Some(viewport) = self.layout.text_box(id) {
                ed.last_viewport = viewport;
            }
            let tmp = crate::document::DocumentSpec {
                markdown: markdown.clone(),
                pending_cmd: pending,
                pending_load,
                ..Default::default()
            };
            // Pendientes primero: pueden mutar contenido y ya llaman refresh_paint.
            let mut changed = ed.apply_pending_load(&tmp);
            changed |= ed.apply_pending_cmd(&tmp);
            // Solo rebuild si cambió rev/sel/viewport size/vista — no por scroll.
            if self.layout.text_box(id).is_some() {
                ed.ensure_paint();
            }
            if changed {
                changes.push((id, ed.change()));
            }
            live.push(id);
        }
        self.documents.retain(|id, _| live.contains(id));

        let _ = focused;
        for (id, change) in changes {
            if let Some(handler) = self
                .handler(id)
                .and_then(|h| h.on_document_change.clone())
            {
                self.send(handler(change));
            }
        }
    }

    #[cfg(feature = "document")]
    fn apply_document_intent(&mut self, id: WidgetId, intent: TextIntent) {
        let change = {
            let Some(ed) = self.documents.get_mut(&id) else {
                return;
            };
            if !ed.apply_text_intent_clipboard(intent, &mut *self.clipboard) {
                return;
            }
            ed.change()
        };
        if let Some(handler) = self
            .handler(id)
            .and_then(|h| h.on_document_change.clone())
        {
            self.send(handler(change));
        }
        self.reset_caret_blink();
        self.update_ime_cursor_area();
        self.invalidate();
    }

    #[cfg(feature = "document")]
    fn report_document_change(&mut self, id: WidgetId) {
        let change = {
            let Some(ed) = self.documents.get_mut(&id) else {
                return;
            };
            ed.change()
        };
        if let Some(handler) = self
            .handler(id)
            .and_then(|h| h.on_document_change.clone())
        {
            self.send(handler(change));
        }
        self.invalidate();
    }

    /// Draw list of the last prepared frame; the background layer is not
    /// yet advanced to any point of its animation.
    pub fn paint_list(&self) -> DrawList {
        let text_fields: Vec<(WidgetId, &TextFieldDecorations)> = self
            .decorations
            .iter()
            .map(|(id, decorations)| (*id, decorations))
            .collect();
        #[cfg(feature = "document")]
        let document_paints: Vec<(WidgetId, &crate::document::DocumentPaint)> = self
            .documents
            .iter()
            .map(|(id, ed)| (*id, ed.paint()))
            .collect();
        #[cfg(not(feature = "document"))]
        let document_paints: Vec<(WidgetId, &crate::document::DocumentPaint)> = Vec::new();
        paint(&PaintInput {
            layout: &self.layout,
            theme: &self.theme,
            focused: *self.ui.focused.get(),
            window: self.activity(),
            text_fields: &text_fields,
            background: self.background,
            time: self.motion_time(),
            documents: &document_paints,
            caret_visible: self.caret_is_visible(),
        })
    }

    /// Seconds since the session started, for [`Motion`](crate::element::Motion); `None` when the
    /// user asked macOS to reduce motion, so moving elements stay put.
    fn motion_time(&self) -> Option<f32> {
        (!self.theme.prefs.reduce_motion).then(|| {
            self.now()
                .saturating_duration_since(self.epoch)
                .as_secs_f32()
        })
    }

    /// Whether an element in view moves by the clock ([`Motion`](crate::element::Motion)).
    fn moving(&self) -> bool {
        !self.theme.prefs.reduce_motion
            && self
                .layout
                .nodes()
                .iter()
                .any(|node| node.motion.is_some() && !node.hidden)
    }
}
