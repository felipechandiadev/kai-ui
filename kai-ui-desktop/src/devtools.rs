//! Development tools (13, Paso 1b), only in debug builds and always on
//! stderr (with `--mcp`, stdout carries the protocol):
//!
//! - `KAI_DEVTOOLS=msgs` prints every message `update` gets, with where it
//!   came from ([`MsgSource`]) and how long `update` took.
//! - `KAI_DEVTOOLS=frames` prints p50 and p95 of view, layout, paint and
//!   render every [`FRAME_WINDOW`] full frames.
//! - `KAI_DEVTOOLS=all` (or `msgs,frames`) turns on both.

use std::fmt;
use std::time::Duration;

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
