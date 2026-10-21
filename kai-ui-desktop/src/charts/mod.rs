//! Native charts: the [`chart`] element, painted with quads and meshes
//! like a map. This cut draws bars and sparklines; axes, line/area and
//! donut charts reuse [`ChartSpec`] later.
//!
//! ```ignore
//! chart(ChartKind::Bar)
//!     .categories(["Ene", "Feb", "Mar"])
//!     .series("Clics", &[4.0, 12.0, 7.0])
//!     .selected(self.mes)
//!     .on_select(Msg::Mes)
//!     .class("h-48 w-full")
//!     .label("Clics por mes");
//! ```

use std::sync::Arc;

use crate::element::{Element, Kind, Role, div, text};
use crate::geometry::{Point, Rect};
use crate::render::geometry::{MeshBuilder, StrokeStyle};
use crate::render::{Affine2, Color, DrawList, GeometryDraw, Mesh, Quad};
use crate::theme::Theme;

/// How a [`chart`] plots its series.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChartKind {
    /// Vertical bars, one slot per category; several series sit side by
    /// side in the slot. Category labels sit under the plot.
    Bar,
    /// A compact polyline of the first series, with a faint fill under it.
    /// No axes or labels.
    Sparkline,
}

/// One named list of Y values. Missing entries in a shorter series are 0.
#[derive(Clone, Debug, PartialEq)]
pub struct ChartSeries {
    pub name: String,
    pub values: Vec<f32>,
}

/// Values compare by bits; the builder only stores finite numbers.
impl Eq for ChartSeries {}

/// What a [`chart`] element shows.
#[derive(Clone, Debug, PartialEq)]
pub struct ChartSpec {
    pub kind: ChartKind,
    pub categories: Vec<String>,
    pub series: Vec<ChartSeries>,
    pub selected: Option<usize>,
}

/// Values compare by bits; the builder only stores finite numbers.
impl Eq for ChartSpec {}

const PLOT_PAD: f32 = 8.0;
const SPARK_PAD: f32 = 2.0;
const LABEL_H: f32 = 20.0;
const BAR_GAP: f32 = 0.25;
const BAR_RADIUS: f32 = 3.0;
const SPARK_WIDTH: f32 = 1.5;
const SERIES_TOKENS: [&str; 4] = ["primary", "success", "warning", "error"];

impl ChartSpec {
    pub fn new(kind: ChartKind) -> Self {
        Self {
            kind,
            categories: Vec::new(),
            series: Vec::new(),
            selected: None,
        }
    }

    /// Categories, or the longest series when the chart has no names.
    pub fn category_count(&self) -> usize {
        self.categories.len().max(
            self.series
                .iter()
                .map(|series| series.values.len())
                .max()
                .unwrap_or(0),
        )
    }

    /// Plot box inside `area` (the padding box): room for labels under bars.
    pub fn plot_rect(&self, area: Rect) -> Rect {
        let pad = match self.kind {
            ChartKind::Bar => PLOT_PAD,
            ChartKind::Sparkline => SPARK_PAD,
        };
        let labels = match self.kind {
            ChartKind::Bar if !self.categories.is_empty() => LABEL_H,
            _ => 0.0,
        };
        Rect {
            x: area.x + pad,
            y: area.y + pad,
            width: (area.width - pad * 2.0).max(0.0),
            height: (area.height - pad - labels).max(0.0),
        }
    }

    /// Bar rectangles in window points, skipping empty or non-finite values.
    pub(crate) fn bars(&self, area: Rect) -> Vec<Bar> {
        if self.kind != ChartKind::Bar || self.series.is_empty() {
            return Vec::new();
        }
        let plot = self.plot_rect(area);
        let n = self.category_count();
        if n == 0 || plot.width <= 0.0 || plot.height <= 0.0 {
            return Vec::new();
        }
        let series_n = self.series.len() as f32;
        let slot = plot.width / n as f32;
        let gap = slot * BAR_GAP;
        let inner = (slot - gap).max(0.0);
        let bar_w = inner / series_n;
        let max = self
            .series
            .iter()
            .flat_map(|series| series.values.iter().copied())
            .filter(|value| value.is_finite())
            .fold(0.0_f32, f32::max);
        let max = if max > 0.0 { max } else { 1.0 };
        let mut bars = Vec::new();
        for (series_index, series) in self.series.iter().enumerate() {
            for category in 0..n {
                let value = series
                    .values
                    .get(category)
                    .copied()
                    .filter(|value| value.is_finite())
                    .unwrap_or(0.0)
                    .max(0.0);
                let height = (value / max) * plot.height;
                if height < 0.5 {
                    continue;
                }
                let x = plot.x + category as f32 * slot + gap / 2.0 + series_index as f32 * bar_w;
                bars.push(Bar {
                    series: series_index,
                    category,
                    rect: Rect {
                        x,
                        y: plot.y + plot.height - height,
                        width: bar_w.max(0.0),
                        height,
                    },
                });
            }
        }
        bars
    }

    /// Vertices of the first series across `area`, left to right.
    pub fn sparkline_points(&self, area: Rect) -> Vec<Point> {
        let Some(series) = self.series.first() else {
            return Vec::new();
        };
        let plot = self.plot_rect(area);
        let n = series.values.len();
        if n == 0 || plot.width <= 0.0 || plot.height <= 0.0 {
            return Vec::new();
        }
        let (min, max) = value_extent(&series.values);
        let span = (max - min).max(1e-6);
        series
            .values
            .iter()
            .enumerate()
            .map(|(index, value)| {
                let value = if value.is_finite() { *value } else { min };
                let t = if n == 1 {
                    0.5
                } else {
                    index as f32 / (n - 1) as f32
                };
                Point {
                    x: plot.x + t * plot.width,
                    y: plot.y + plot.height - ((value - min) / span) * plot.height,
                }
            })
            .collect()
    }

    /// Accessible description of the first series: `"Ene 4, Feb 12"`.
    pub fn accessible_description(&self) -> String {
        let Some(series) = self.series.first() else {
            return String::new();
        };
        if self.categories.is_empty() {
            return series
                .values
                .iter()
                .map(|value| format_value(*value))
                .collect::<Vec<_>>()
                .join(", ");
        }
        self.categories
            .iter()
            .enumerate()
            .map(|(index, name)| {
                let value = series.values.get(index).copied().unwrap_or(0.0);
                format!("{name} {}", format_value(value))
            })
            .collect::<Vec<_>>()
            .join(", ")
    }
}

/// A painted bar: which series and category it belongs to.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Bar {
    pub series: usize,
    pub category: usize,
    pub rect: Rect,
}

/// Category under `at` (window points) of a bar chart; `None` on a
/// sparkline, outside the plot, or when there are no categories.
pub(crate) fn hit(area: Rect, spec: &ChartSpec, at: Point) -> Option<usize> {
    if spec.kind != ChartKind::Bar {
        return None;
    }
    let plot = spec.plot_rect(area);
    let n = spec.category_count();
    if n == 0 || plot.width <= 0.0 {
        return None;
    }
    let hit_area = Rect {
        x: plot.x,
        y: plot.y,
        width: plot.width,
        height: (area.y + area.height - plot.y).max(0.0),
    };
    if !hit_area.contains(at) {
        return None;
    }
    let index = ((at.x - plot.x) / (plot.width / n as f32)).floor() as i32;
    (index >= 0 && (index as usize) < n).then_some(index as usize)
}

/// A chart of `kind`. Give it a size (`h-48`, `flex-1`…) and an accessible
/// name. Bars take [`Element::on_select`] with the category index.
pub fn chart<Msg>(kind: ChartKind) -> Element<Msg> {
    let cursor = match kind {
        ChartKind::Bar => " cursor-pointer",
        ChartKind::Sparkline => "",
    };
    Element::new(Kind::Chart(Box::new(ChartSpec::new(kind))))
        .role(Role::Image)
        .parts_root()
        .class(&format!(
            "relative overflow-hidden rounded-md bg-muted/30{cursor}"
        ))
}

impl<Msg> Element<Msg> {
    /// Charts only: names under the bars (one slot each). A sparkline
    /// ignores them.
    pub fn categories(mut self, names: impl IntoIterator<Item = impl Into<String>>) -> Self {
        let Kind::Chart(spec) = &mut self.kind else {
            return self;
        };
        spec.categories = names.into_iter().map(Into::into).collect();
        let kind = spec.kind;
        let categories = spec.categories.clone();
        let description = spec.accessible_description();
        if kind == ChartKind::Bar {
            self.children = category_labels(&categories);
        }
        if !description.is_empty() {
            self.description = Some(description);
        }
        self
    }

    /// Charts only: append a series. A sparkline draws the first one.
    pub fn series(mut self, name: impl Into<String>, values: &[f32]) -> Self {
        let Kind::Chart(spec) = &mut self.kind else {
            return self;
        };
        spec.series.push(ChartSeries {
            name: name.into(),
            values: values
                .iter()
                .copied()
                .map(|value| if value.is_finite() { value } else { 0.0 })
                .collect(),
        });
        let description = spec.accessible_description();
        if !description.is_empty() {
            self.description = Some(description);
        }
        self
    }

    /// Charts only: the highlighted category of a bar chart.
    pub fn selected(mut self, index: Option<usize>) -> Self {
        if let Kind::Chart(spec) = &mut self.kind {
            spec.selected = index;
        }
        self
    }
}

fn category_labels<Msg>(categories: &[String]) -> Vec<Element<Msg>> {
    if categories.is_empty() {
        return Vec::new();
    }
    vec![
        div()
            .part("labels")
            .class("absolute inset-x-0 bottom-0 flex px-2 h-5 pointer-events-none")
            .children(categories.iter().map(|name| {
                text(name.clone())
                    .class("flex-1 min-w-0 text-center text-xs text-muted-foreground truncate")
            })),
    ]
}

/// Paint `spec` inside `area` (the node's padding box).
pub(crate) fn paint(list: &mut DrawList, area: Rect, spec: &ChartSpec, theme: &Theme) {
    if area.width <= 0.0 || area.height <= 0.0 {
        return;
    }
    list.push_clip(area);
    match spec.kind {
        ChartKind::Bar => paint_bars(list, area, spec, theme),
        ChartKind::Sparkline => paint_sparkline(list, area, spec, theme),
    }
    list.pop_clip();
}

fn paint_bars(list: &mut DrawList, area: Rect, spec: &ChartSpec, theme: &Theme) {
    let choice = theme.color("choice").unwrap_or(theme.colors.secondary);
    for bar in spec.bars(area) {
        let color = if spec.selected == Some(bar.category) {
            choice
        } else {
            series_color(theme, bar.series)
        };
        list.quad(Quad {
            radius: BAR_RADIUS.min(bar.rect.width / 2.0).min(bar.rect.height),
            ..Quad::fill(bar.rect, color)
        });
    }
}

fn paint_sparkline(list: &mut DrawList, area: Rect, spec: &ChartSpec, theme: &Theme) {
    let points = spec.sparkline_points(area);
    let color = series_color(theme, 0);
    if points.is_empty() {
        return;
    }
    if points.len() == 1 {
        let point = points[0];
        let side = 4.0;
        list.quad(Quad {
            radius: side / 2.0,
            ..Quad::fill(
                Rect {
                    x: point.x - side / 2.0,
                    y: point.y - side / 2.0,
                    width: side,
                    height: side,
                },
                color,
            )
        });
        return;
    }
    let plot = spec.plot_rect(area);
    let mut fill = points.clone();
    if let (Some(first), Some(last)) = (points.first(), points.last()) {
        fill.push(Point {
            x: last.x,
            y: plot.y + plot.height,
        });
        fill.push(Point {
            x: first.x,
            y: plot.y + plot.height,
        });
        let mesh = Mesh::fill(std::slice::from_ref(&fill), 1.0);
        if !mesh.is_empty() {
            list.geometry(GeometryDraw::fill(
                Arc::new(mesh),
                Affine2::IDENTITY,
                fade(color, 0.2),
            ));
        }
    }
    let mut stroke = MeshBuilder::new(1.0);
    stroke.stroke(&points, false, StrokeStyle::ROUND);
    let mesh = stroke.build();
    if !mesh.is_empty() {
        list.geometry(GeometryDraw::stroke(
            Arc::new(mesh),
            Affine2::IDENTITY,
            color,
            SPARK_WIDTH,
        ));
    }
}

fn series_color(theme: &Theme, index: usize) -> Color {
    theme
        .color(SERIES_TOKENS[index % SERIES_TOKENS.len()])
        .unwrap_or(theme.colors.primary)
}

fn fade(color: Color, opacity: f32) -> Color {
    color.with_alpha((f32::from(color.a) * opacity.clamp(0.0, 1.0)).round() as u8)
}

fn value_extent(values: &[f32]) -> (f32, f32) {
    let mut min = f32::INFINITY;
    let mut max = f32::NEG_INFINITY;
    for value in values.iter().copied().filter(|value| value.is_finite()) {
        min = min.min(value);
        max = max.max(value);
    }
    if !min.is_finite() {
        return (0.0, 1.0);
    }
    if (max - min).abs() < f32::EPSILON {
        (min - 1.0, max + 1.0)
    } else {
        (min, max)
    }
}

fn format_value(value: f32) -> String {
    if !value.is_finite() {
        return "0".to_owned();
    }
    if value == value.round() && value.abs() < 1_000_000.0 {
        format!("{}", value as i64)
    } else {
        format!("{value}")
    }
}

#[cfg(test)]
mod tests {
    use super::{ChartKind, ChartSpec, hit};
    use crate::geometry::{Point, Rect};
    use crate::render::{DrawList, DrawOp};
    use crate::theme::{Density, Theme};

    fn area() -> Rect {
        Rect {
            x: 10.0,
            y: 20.0,
            width: 200.0,
            height: 100.0,
        }
    }

    fn bar_spec(values: &[f32]) -> ChartSpec {
        let mut spec = ChartSpec::new(ChartKind::Bar);
        spec.categories = (0..values.len()).map(|i| format!("c{i}")).collect();
        spec.series.push(super::ChartSeries {
            name: "s".into(),
            values: values.to_vec(),
        });
        spec
    }

    #[test]
    fn equal_values_share_height_and_leave_a_gap() {
        let spec = bar_spec(&[1.0, 1.0, 1.0]);
        let bars = spec.bars(area());
        assert_eq!(bars.len(), 3);
        assert!((bars[0].rect.height - bars[1].rect.height).abs() < 0.01);
        assert!(bars[0].rect.x + bars[0].rect.width < bars[1].rect.x);
        assert!(bars[1].rect.x + bars[1].rect.width < bars[2].rect.x);
    }

    #[test]
    fn a_taller_value_makes_a_taller_bar() {
        let spec = bar_spec(&[1.0, 3.0]);
        let bars = spec.bars(area());
        assert_eq!(bars.len(), 2);
        assert!(bars[1].rect.height > bars[0].rect.height * 2.0);
        assert!((bars[1].rect.height - bars[0].rect.height * 3.0).abs() < 0.01);
    }

    #[test]
    fn empty_or_zero_charts_do_not_panic() {
        let empty = ChartSpec::new(ChartKind::Bar);
        assert!(empty.bars(area()).is_empty());
        assert_eq!(hit(area(), &empty, Point { x: 50.0, y: 50.0 }), None);
        let zeros = bar_spec(&[0.0, 0.0]);
        assert!(zeros.bars(area()).is_empty());
        let spark = {
            let mut spec = ChartSpec::new(ChartKind::Sparkline);
            spec.series.push(super::ChartSeries {
                name: "t".into(),
                values: vec![],
            });
            spec
        };
        assert!(spark.sparkline_points(area()).is_empty());
    }

    #[test]
    fn a_click_hits_the_category_slot_not_just_the_bar() {
        let spec = bar_spec(&[1.0, 10.0, 1.0]);
        let plot = spec.plot_rect(area());
        let slot = plot.width / 3.0;
        let low = Point {
            x: plot.x + slot * 0.5,
            y: plot.y + plot.height - 2.0,
        };
        assert_eq!(hit(area(), &spec, low), Some(0));
        let mid = Point {
            x: plot.x + slot * 1.5,
            y: plot.y + plot.height / 2.0,
        };
        assert_eq!(hit(area(), &spec, mid), Some(1));
        assert_eq!(
            hit(
                area(),
                &spec,
                Point {
                    x: plot.x - 1.0,
                    y: plot.y,
                }
            ),
            None
        );
    }

    #[test]
    fn a_sparkline_runs_left_to_right() {
        let mut spec = ChartSpec::new(ChartKind::Sparkline);
        spec.series.push(super::ChartSeries {
            name: "t".into(),
            values: vec![0.0, 10.0, 5.0],
        });
        let points = spec.sparkline_points(area());
        assert_eq!(points.len(), 3);
        assert!(points[0].x < points[1].x && points[1].x < points[2].x);
        assert!(points[1].y < points[2].y && points[2].y < points[0].y);
    }

    #[test]
    fn paint_emits_a_quad_per_bar_and_geometry_for_a_sparkline() {
        let theme = Theme::kai(Density::Comfortable);
        let mut list = DrawList::new(theme.colors.background);
        super::paint(&mut list, area(), &bar_spec(&[1.0, 2.0, 3.0]), &theme);
        let quads = list
            .ops
            .iter()
            .filter(|op| matches!(op, DrawOp::Quad(_)))
            .count();
        assert_eq!(quads, 3);

        let mut spark = ChartSpec::new(ChartKind::Sparkline);
        spark.series.push(super::ChartSeries {
            name: "t".into(),
            values: vec![1.0, 3.0, 2.0, 5.0],
        });
        let mut list = DrawList::new(theme.colors.background);
        super::paint(&mut list, area(), &spark, &theme);
        assert!(
            list.ops.iter().any(|op| matches!(op, DrawOp::Geometry(_))),
            "sparkline strokes a mesh"
        );
    }

    #[test]
    fn accessible_description_lists_named_values() {
        let spec = bar_spec(&[4.0, 12.0]);
        assert_eq!(spec.accessible_description(), "c0 4, c1 12");
    }
}
