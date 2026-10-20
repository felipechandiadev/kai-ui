//! Development tools (13, Paso 1b), only in debug builds and always on
//! stderr (with `--mcp`, stdout carries the protocol):
//!
//! - `KAI_DEVTOOLS=msgs` prints every message `update` gets, with where it
//!   came from ([`MsgSource`]) and how long `update` took.
//! - `KAI_DEVTOOLS=frames` prints p50 and p95 of view, layout, paint and
//!   render every [`FRAME_WINDOW`] full frames.
//! - `KAI_DEVTOOLS=all` (or `msgs,frames`) turns on both.
//! - ⌘⌥I toggles the inspector ([`INSPECTOR`]): an outline over the
//!   element under the pointer with its id, role, accessible name,
//!   `test_id`, bounds and classes. Opening it prints the whole tree
//!   ([`dump`]).

use std::collections::HashMap;
use std::fmt;
use std::time::Duration;

use crate::a11y::{accessible_name, effective_role};
use crate::components::themed;
use crate::element::{Element, div, text};
use crate::geometry::{Rect, Size};
use crate::layout::Layout;

/// Test id of the inspector overlay.
pub const INSPECTOR: &str = "kai.inspector";

/// Longest class list the inspector shows.
const CLASS_LIMIT: usize = 120;

/// Full frames per [`FrameStats`] summary.
pub const FRAME_WINDOW: usize = 120;

/// Which development tools are on.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Devtools {
    pub msgs: bool,
    pub frames: bool,
}

impl Devtools {
    /// From `KAI_DEVTOOLS` in debug builds; all off in release.
    pub fn from_env() -> Self {
        if !cfg!(debug_assertions) {
            return Self::default();
        }
        std::env::var("KAI_DEVTOOLS")
            .map(|value| Self::parse(&value))
            .unwrap_or_default()
    }

    /// `msgs`, `frames`, `all` or a comma list of them; other words are
    /// ignored.
    pub fn parse(value: &str) -> Self {
        let mut devtools = Self::default();
        for word in value.split(',').map(str::trim) {
            match word {
                "msgs" => devtools.msgs = true,
                "frames" => devtools.frames = true,
                "all" => {
                    devtools.msgs = true;
                    devtools.frames = true;
                }
                _ => {}
            }
        }
        devtools
    }
}

/// Where a message sent to `update` came from.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MsgSource {
    Pointer,
    /// Keys and text input, shortcuts included.
    Keyboard,
    /// An assistive technology's action (VoiceOver, for instance).
    A11y,
    /// The menu bar.
    Menu,
    /// Code: `TestApp::invoke`, `TestApp::call`, a script.
    Script,
    /// An agent's call, with the client's name.
    Agent(String),
    /// A timer the app asked for with `Effects::after`.
    Timer,
}

impl fmt::Display for MsgSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Pointer => f.write_str("Puntero"),
            Self::Keyboard => f.write_str("Teclado"),
            Self::A11y => f.write_str("Accesibilidad"),
            Self::Menu => f.write_str("Menú"),
            Self::Script => f.write_str("Script"),
            Self::Agent(client) => write!(f, "Agente {client}"),
            Self::Timer => f.write_str("Temporizador"),
        }
    }
}

/// The log line of one message, given as its `Debug` text.
pub fn format_msg(source: &MsgSource, msg: &str, update: Duration) -> String {
    format!(
        "msg {:<13} {msg} · update {:.2} ms",
        source.to_string(),
        millis(update)
    )
}

/// How long each phase of a full frame took.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct FrameTimes {
    pub view: Duration,
    pub layout: Duration,
    pub paint: Duration,
    pub render: Duration,
}

/// The last full frames, summarized every [`FRAME_WINDOW`].
#[derive(Debug, Default)]
pub struct FrameStats {
    frames: Vec<FrameTimes>,
}

impl FrameStats {
    /// Add a frame; once the window is full, its summary (and start over).
    pub fn push(&mut self, frame: FrameTimes) -> Option<String> {
        self.frames.push(frame);
        if self.frames.len() < FRAME_WINDOW {
            return None;
        }
        let frames = std::mem::take(&mut self.frames);
        let phase = |name: &str, time: fn(&FrameTimes) -> Duration| {
            let mut times: Vec<Duration> = frames.iter().map(time).collect();
            times.sort();
            format!(
                "{name} p50 {:.2} ms p95 {:.2} ms",
                millis(percentile(&times, 50)),
                millis(percentile(&times, 95))
            )
        };
        Some(format!(
            "frames ({}): {} · {} · {} · {}",
            frames.len(),
            phase("view", |frame| frame.view),
            phase("layout", |frame| frame.layout),
            phase("paint", |frame| frame.paint),
            phase("render", |frame| frame.render),
        ))
    }
}

/// The laid-out tree, one node per line, indented by depth: role, id,
/// accessible name, `test_id`, bounds, whether it is hidden and the
/// element's classes.
pub fn dump<Msg>(tree: &Element<Msg>, layout: &Layout) -> String {
    let mut classes = HashMap::new();
    tree.walk(&mut |element| {
        classes.insert(element.id, element.class.as_str());
    });
    let nodes = layout.nodes();
    let mut depths = Vec::with_capacity(nodes.len());
    let mut out = String::new();
    for (index, node) in nodes.iter().enumerate() {
        let depth = node.parent.map_or(0, |parent| depths[parent] + 1);
        depths.push(depth);
        let class = classes.get(&node.id).copied().unwrap_or_default();
        out.push_str(&"  ".repeat(depth));
        out.push_str(&describe(layout, index, class).join(" "));
        out.push('\n');
    }
    out
}

/// What the inspector says about node `index`: role, id and name; test
/// id; bounds; classes (shortened).
pub fn describe(layout: &Layout, index: usize, class: &str) -> Vec<String> {
    let node = layout.get(index);
    let mut head = format!("{:?} #{}", effective_role(node), node.id.0);
    if let Some(name) = accessible_name(layout, index).filter(|name| !name.is_empty()) {
        head.push_str(&format!(" «{name}»"));
    }
    let mut lines = vec![head];
    if let Some(test_id) = &node.test_id {
        lines.push(format!("[{test_id}]"));
    }
    let Rect {
        x,
        y,
        width,
        height,
    } = node.bounds;
    let hidden = if node.hidden { " oculto" } else { "" };
    lines.push(format!("{x:.0},{y:.0} {width:.0}×{height:.0}{hidden}"));
    if !class.is_empty() {
        let mut shown: String = class.chars().take(CLASS_LIMIT).collect();
        if class.chars().count() > CLASS_LIMIT {
            shown.push('…');
        }
        lines.push(format!("\"{shown}\""));
    }
    lines
}

/// The inspector overlay: an outline over `target` (its visible bounds and
/// the lines of [`describe`]) and a panel with those lines, away from it.
/// Nothing in it takes the pointer.
pub fn inspector<Msg>(target: Option<(Rect, Vec<String>)>, viewport: Size) -> Element<Msg> {
    let line = |content: String| text(content).class("whitespace-nowrap");
    let (outline, lines, below) = match target {
        Some((rect, lines)) => {
            let outline = div().class(&format!(
                "absolute border-2 {} left-[{:.0}px] top-[{:.0}px] w-[{:.0}px] h-[{:.0}px]",
                themed("border-primary", "border-control-accent"),
                rect.x.max(0.0),
                rect.y.max(0.0),
                rect.width.max(0.0),
                rect.height.max(0.0),
            ));
            let below = rect.y + rect.height / 2.0 < viewport.height / 2.0;
            (Some(outline), lines, below)
        }
        None => (
            None,
            vec!["Inspector: pasa el puntero sobre un elemento (⌘⌥I cierra)".to_owned()],
            true,
        ),
    };
    let panel = div()
        .class(&format!(
            "absolute left-3 {} flex flex-col px-3 py-2 rounded-md {}",
            if below { "bottom-3" } else { "top-3" },
            themed(
                "text-xs bg-foreground text-background",
                "text-caption1 bg-control-accent text-on-accent",
            )
        ))
        .part("panel")
        .children(lines.into_iter().map(line));
    let mut root = div()
        .class("absolute inset-0 z-100 pointer-events-none")
        .parts_root()
        .test_id(INSPECTOR);
    if let Some(outline) = outline {
        root = root.child(outline.part("outline"));
    }
    root.child(panel)
}

/// Nearest-rank percentile of sorted, non-empty `times`.
fn percentile(times: &[Duration], percent: usize) -> Duration {
    times[(times.len() - 1) * percent / 100]
}

fn millis(duration: Duration) -> f64 {
    duration.as_secs_f64() * 1000.0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::element::Role;
    use crate::testing::layout_of;

    #[test]
    fn dump_lists_every_node_by_depth() {
        let mut tree: Element<()> = div().class("flex gap-2 p-4").children([
            div()
                .class("w-[100px] h-[20px]")
                .role(Role::Heading)
                .label("Título"),
            div().class("hidden").test_id("oculto"),
        ]);
        tree.assign_ids();
        let layout = layout_of(&tree);
        let [root, heading, hidden] = [&tree, &tree.children[0], &tree.children[1]].map(|e| e.id.0);
        assert_eq!(
            dump(&tree, &layout),
            format!(
                "Generic #{root} 0,0 1024×720 \"flex gap-2 p-4\"\n\
                 \x20 Heading #{heading} «Título» 16,16 100×20 \"w-[100px] h-[20px]\"\n\
                 \x20 Generic #{hidden} [oculto] 0,0 0×0 oculto \"hidden\"\n"
            )
        );
    }

    #[test]
    fn parse_takes_words_lists_and_all() {
        let both = Devtools {
            msgs: true,
            frames: true,
        };
        assert_eq!(Devtools::parse(""), Devtools::default());
        assert_eq!(
            Devtools::parse("msgs"),
            Devtools {
                msgs: true,
                frames: false
            }
        );
        assert_eq!(Devtools::parse("msgs, frames"), both);
        assert_eq!(Devtools::parse("all"), both);
        assert_eq!(
            Devtools::parse("frames,otra"),
            Devtools {
                msgs: false,
                frames: true
            }
        );
    }

    #[test]
    fn a_message_line_says_where_it_came_from() {
        let line = format_msg(
            &MsgSource::Agent("kai-test".into()),
            "Sumar(2)",
            Duration::from_micros(250),
        );
        assert_eq!(line, "msg Agente kai-test Sumar(2) · update 0.25 ms");
        let line = format_msg(&MsgSource::Pointer, "Sumar(1)", Duration::ZERO);
        assert_eq!(line, "msg Puntero       Sumar(1) · update 0.00 ms");
    }

    #[test]
    fn frame_stats_summarize_each_window() {
        let mut stats = FrameStats::default();
        for i in 1..FRAME_WINDOW {
            let ms = Duration::from_millis(i as u64);
            let frame = FrameTimes {
                view: ms,
                layout: ms * 2,
                paint: Duration::ZERO,
                render: ms,
            };
            assert_eq!(stats.push(frame), None);
        }
        let last = Duration::from_millis(FRAME_WINDOW as u64);
        let summary = stats.push(FrameTimes {
            view: last,
            layout: last * 2,
            paint: Duration::ZERO,
            render: last,
        });
        assert_eq!(
            summary.as_deref(),
            Some(
                "frames (120): view p50 60.00 ms p95 114.00 ms · \
                 layout p50 120.00 ms p95 228.00 ms · \
                 paint p50 0.00 ms p95 0.00 ms · \
                 render p50 60.00 ms p95 114.00 ms"
            )
        );
        assert_eq!(stats.push(FrameTimes::default()), None);
    }
}
