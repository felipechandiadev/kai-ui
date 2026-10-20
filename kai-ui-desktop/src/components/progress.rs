//! Dots that jump in sequence while something is going on (`DotProgress`
//! in `kai-ui-web/src/components/DotProgress`, `wave` variant). The jump is
//! a [`Motion`] computed while painting: layout and hit-testing never move.

use crate::element::{Element, Motion, Role, div};

/// Diameter and gap of the dots: `Sm` (6 pt, gap 3) is the one inside a
/// busy button, `Lg` (16 pt, gap 8) the web default.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum DotSize {
    Sm,
    #[default]
    Md,
    Lg,
}

impl DotSize {
    /// Dot diameter and gap, in points.
    pub fn metrics(self) -> (f32, f32) {
        match self {
            Self::Sm => (6.0, 3.0),
            Self::Md => (10.0, 5.0),
            Self::Lg => (16.0, 8.0),
        }
    }
}

/// Name of a [`DotProgress`] with no label of its own.
pub const DOT_PROGRESS_LABEL: &str = "Cargando";

/// Indeterminate progress: `count` dots (3 by default) in the text color
/// (`bg-current`), each one jumping 0,16 s after the previous one. Taller
/// than the dots by the jump, so the wave is never clipped. With
/// `reduce_motion` the dots stay still.
#[derive(Clone, Debug, PartialEq)]
pub struct DotProgress {
    count: usize,
    size: DotSize,
    label: Option<String>,
    decorative: bool,
    test_id: Option<String>,
}

impl Default for DotProgress {
    fn default() -> Self {
        Self::new()
    }
}

impl DotProgress {
    pub fn new() -> Self {
        Self {
            count: 3,
            size: DotSize::default(),
            label: None,
            decorative: false,
            test_id: None,
        }
    }

    /// Number of dots (at least 1).
    pub fn count(mut self, count: usize) -> Self {
        self.count = count.max(1);
        self
    }

    pub fn size(mut self, size: DotSize) -> Self {
        self.size = size;
        self
    }

    /// Accessible name (by default «Cargando»).
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// No role nor name: the busy state belongs to the element around it
    /// (a busy button).
    pub fn decorative(mut self) -> Self {
        self.decorative = true;
        self
    }

    pub fn test_id(mut self, test_id: impl Into<String>) -> Self {
        self.test_id = Some(test_id.into());
        self
    }

    pub fn build<Msg>(self) -> Element<Msg> {
        let (dot, gap) = self.size.metrics();
        let lift = (dot * 0.7).round();
        let mut root = div()
            .class(&format!(
                "inline-flex shrink-0 items-center gap-[{gap}px] pt-[{lift}px]"
            ))
            .parts_root();
        if !self.decorative {
            root = root
                .role(Role::ProgressIndicator)
                .label(self.label.unwrap_or_else(|| DOT_PROGRESS_LABEL.to_owned()));
        }
        for index in 0..self.count {
            root = root.child(
                div()
                    .class(&format!("size-[{dot}px] shrink-0 rounded-full bg-current"))
                    .motion(Motion::wave(index))
                    .part(format!("dot.{index}")),
            );
        }
        root.test_id = self.test_id;
        root
    }
}

/// Three medium dots named «Cargando».
pub fn dot_progress<Msg>() -> Element<Msg> {
    DotProgress::new().build()
}
