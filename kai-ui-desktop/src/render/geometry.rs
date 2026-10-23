//! Vector geometry for [`DrawOp::Geometry`](super::DrawOp::Geometry):
//! triangle meshes built with `lyon_tessellation` and drawn by
//! `geometry.wgsl` under MSAA 4×.
//!
//! A mesh lives in its own coordinates (a tile's 0..4096, an area relative
//! to its origin…) and a [`GeometryDraw`] places it with an [`Affine2`].
//! Stroke vertices sit on the path and carry the extrusion normal; the
//! shader moves them `normal × width / 2` after the transform, so a stroke
//! keeps its width in screen pixels at any zoom and panning or zooming never
//! re-tessellates or re-uploads the mesh.

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use lyon_tessellation::geom::point;
use lyon_tessellation::path::Path;
use lyon_tessellation::{
    BuffersBuilder, FillOptions, FillRule, FillTessellator, FillVertex, LineCap, LineJoin,
    StrokeOptions, StrokeTessellator, StrokeVertex, VertexBuffers,
};

use super::Color;
use crate::geometry::{Point, Rect};

/// Maps mesh coordinates to logical window points:
/// `x' = a·x + c·y + tx`, `y' = b·x + d·y + ty`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Affine2 {
    pub a: f32,
    pub b: f32,
    pub c: f32,
    pub d: f32,
    pub tx: f32,
    pub ty: f32,
}

impl Affine2 {
    pub const IDENTITY: Self = Self::scale_translate(1.0, 0.0, 0.0);

    /// Uniform `scale`, then a move by `(tx, ty)`.
    pub const fn scale_translate(scale: f32, tx: f32, ty: f32) -> Self {
        Self {
            a: scale,
            b: 0.0,
            c: 0.0,
            d: scale,
            tx,
            ty,
        }
    }

    pub fn apply(self, p: Point) -> Point {
        Point {
            x: self.a * p.x + self.c * p.y + self.tx,
            y: self.b * p.x + self.d * p.y + self.ty,
        }
    }

    /// Bounding box of `rect` after the transform.
    pub fn apply_rect(self, rect: Rect) -> Rect {
        let corners = [
            self.apply(Point {
                x: rect.x,
                y: rect.y,
            }),
            self.apply(Point {
                x: rect.x + rect.width,
                y: rect.y,
            }),
            self.apply(Point {
                x: rect.x,
                y: rect.y + rect.height,
            }),
            self.apply(Point {
                x: rect.x + rect.width,
                y: rect.y + rect.height,
            }),
        ];
        let (mut left, mut top) = (f32::INFINITY, f32::INFINITY);
        let (mut right, mut bottom) = (f32::NEG_INFINITY, f32::NEG_INFINITY);
        for p in corners {
            left = left.min(p.x);
            top = top.min(p.y);
            right = right.max(p.x);
            bottom = bottom.max(p.y);
        }
        Rect {
            x: left,
            y: top,
            width: right - left,
            height: bottom - top,
        }
    }
}

/// One mesh vertex: its position and, for strokes, the extrusion normal
/// (unit length, longer at miter joins; zero for fills).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
pub struct GeoVertex {
    pub pos: [f32; 2],
    pub normal: [f32; 2],
}

/// How strokes meet at corners.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Join {
    #[default]
    Miter,
    Round,
    Bevel,
}

/// How open strokes end.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Cap {
    #[default]
    Butt,
    Round,
    Square,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct StrokeStyle {
    pub join: Join,
    pub cap: Cap,
}

impl StrokeStyle {
    pub const ROUND: Self = Self {
        join: Join::Round,
        cap: Cap::Round,
    };
}

/// An immutable triangle mesh. Its id keys the renderer's GPU buffers, so a
/// mesh is uploaded once however many frames draw it.
#[derive(Debug)]
pub struct Mesh {
    id: u64,
    vertices: Vec<GeoVertex>,
    indices: Vec<u32>,
    bounds: Option<Rect>,
}

impl PartialEq for Mesh {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

static NEXT_MESH_ID: AtomicU64 = AtomicU64::new(1);

impl Mesh {
    pub fn new(vertices: Vec<GeoVertex>, indices: Vec<u32>) -> Self {
        let bounds = vertices.split_first().map(|(first, rest)| {
            let [mut left, mut top] = first.pos;
            let (mut right, mut bottom) = (left, top);
            for vertex in rest {
                let [x, y] = vertex.pos;
                left = left.min(x);
                top = top.min(y);
                right = right.max(x);
                bottom = bottom.max(y);
            }
            Rect {
                x: left,
                y: top,
                width: right - left,
                height: bottom - top,
            }
        });
        Self {
            id: NEXT_MESH_ID.fetch_add(1, Ordering::Relaxed),
            vertices,
            indices,
            bounds,
        }
    }

    /// The filled interior of closed `rings` (even-odd: inner rings are
    /// holes). `pixel` is how many mesh units one screen pixel spans at the
    /// expected zoom (see [`MeshBuilder::new`]).
    pub fn fill(rings: &[Vec<Point>], pixel: f32) -> Self {
        let mut builder = MeshBuilder::new(pixel);
        builder.fill(rings);
        builder.build()
    }

    /// Strokes along `lines` (closed when `closed`), to be drawn with a
    /// screen width in [`GeometryDraw::width`].
    pub fn stroke(lines: &[Vec<Point>], closed: bool, style: StrokeStyle, pixel: f32) -> Self {
        let mut builder = MeshBuilder::new(pixel);
        for line in lines {
            builder.stroke(line, closed, style);
        }
        builder.build()
    }

    pub fn id(&self) -> u64 {
        self.id
    }

    pub fn vertices(&self) -> &[GeoVertex] {
        &self.vertices
    }

    pub fn indices(&self) -> &[u32] {
        &self.indices
    }

    /// Bounding box of the vertex positions, before extrusion; `None` when
    /// empty.
    pub fn bounds(&self) -> Option<Rect> {
        self.bounds
    }

    pub fn is_empty(&self) -> bool {
        self.indices.is_empty()
    }

    /// GPU memory of the vertex and index buffers.
    pub fn byte_size(&self) -> usize {
        std::mem::size_of_val(self.vertices.as_slice())
            + std::mem::size_of_val(self.indices.as_slice())
    }
}

/// Accumulates fills or strokes of many features into one mesh (one
/// draw call). Each call tessellates on its own, so separate polygons never
/// cancel each other out.
pub struct MeshBuilder {
    buffers: VertexBuffers<GeoVertex, u32>,
    pixel: f32,
    fill: FillTessellator,
    stroke: StrokeTessellator,
}

impl MeshBuilder {
    /// `pixel`: mesh units per screen pixel at the expected zoom. It sets
    /// the tessellation tolerance and how finely round joins and caps are
    /// cut; any positive value draws correctly.
    pub fn new(pixel: f32) -> Self {
        Self {
            buffers: VertexBuffers::new(),
            pixel: if pixel.is_finite() && pixel > 0.0 {
                pixel
            } else {
                1.0
            },
            fill: FillTessellator::new(),
            stroke: StrokeTessellator::new(),
        }
    }

    /// Fill one polygon: its outer ring and holes. Rings close by
    /// themselves; a repeated first point is fine.
    pub fn fill(&mut self, rings: &[Vec<Point>]) -> &mut Self {
        let Some(path) = path(rings.iter().map(Vec::as_slice), true) else {
            return self;
        };
        let options = FillOptions::tolerance(self.pixel * 0.05).with_fill_rule(FillRule::EvenOdd);
        // A self-intersecting ring that lyon rejects is skipped.
        let _ = self.fill.tessellate_path(
            &path,
            &options,
            &mut BuffersBuilder::new(&mut self.buffers, |vertex: FillVertex| GeoVertex {
                pos: [vertex.position().x, vertex.position().y],
                normal: [0.0, 0.0],
            }),
        );
        self
    }

    /// Stroke one line (a ring when `closed`).
    pub fn stroke(&mut self, line: &[Point], closed: bool, style: StrokeStyle) -> &mut Self {
        let Some(path) = path(std::iter::once(line), closed) else {
            return self;
        };
        let cap = match style.cap {
            Cap::Butt => LineCap::Butt,
            Cap::Round => LineCap::Round,
            Cap::Square => LineCap::Square,
        };
        let join = match style.join {
            Join::Miter => LineJoin::Miter,
            Join::Round => LineJoin::Round,
            Join::Bevel => LineJoin::Bevel,
        };
        // Tessellated one pixel wide: the shader extrudes the normals to the
        // drawn width.
        let options = StrokeOptions::tolerance(self.pixel * 0.02)
            .with_line_width(self.pixel)
            .with_line_cap(cap)
            .with_line_join(join);
        let _ = self.stroke.tessellate_path(
            &path,
            &options,
            &mut BuffersBuilder::new(&mut self.buffers, |vertex: StrokeVertex| {
                let on_path = vertex.position_on_path();
                let normal = vertex.normal();
                GeoVertex {
                    pos: [on_path.x, on_path.y],
                    normal: [normal.x, normal.y],
                }
            }),
        );
        self
    }

    pub fn is_empty(&self) -> bool {
        self.buffers.indices.is_empty()
    }

    pub fn build(self) -> Mesh {
        Mesh::new(self.buffers.vertices, self.buffers.indices)
    }
}

/// A lyon path of `lines`; `None` when none has two points.
fn path<'a>(lines: impl Iterator<Item = &'a [Point]>, closed: bool) -> Option<Path> {
    let mut builder = Path::builder();
    let mut any = false;
    for line in lines {
        let mut points = line
            .iter()
            .filter(|p| p.x.is_finite() && p.y.is_finite())
            .map(|p| point(p.x, p.y));
        let Some(first) = points.next() else {
            continue;
        };
        let rest: Vec<_> = points.collect();
        let rest = if closed && rest.last() == Some(&first) {
            &rest[..rest.len() - 1]
        } else {
            &rest[..]
        };
        if rest.is_empty() {
            continue;
        }
        builder.begin(first);
        for p in rest {
            builder.line_to(*p);
        }
        builder.end(closed);
        any = true;
    }
    any.then(|| builder.build())
}

/// A mesh placed in the window and painted with one color. `width` is the
/// stroke width in logical points, `0` for fills.
#[derive(Clone, Debug, PartialEq)]
pub struct GeometryDraw {
    pub mesh: Arc<Mesh>,
    pub transform: Affine2,
    pub color: Color,
    pub width: f32,
}

impl GeometryDraw {
    pub fn fill(mesh: Arc<Mesh>, transform: Affine2, color: Color) -> Self {
        Self {
            mesh,
            transform,
            color,
            width: 0.0,
        }
    }

    pub fn stroke(mesh: Arc<Mesh>, transform: Affine2, color: Color, width: f32) -> Self {
        Self {
            mesh,
            transform,
            color,
            width,
        }
    }

    /// Area the draw can touch, in logical points: the transformed bounds
    /// plus room for miter joins (lyon limits them to 4 half widths).
    pub fn extent(&self) -> Option<Rect> {
        let rect = self.transform.apply_rect(self.mesh.bounds()?);
        let grow = self.width.max(0.0) * 2.0;
        Some(Rect {
            x: rect.x - grow,
            y: rect.y - grow,
            width: rect.width + grow * 2.0,
            height: rect.height + grow * 2.0,
        })
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::{Affine2, GeometryDraw, Mesh, MeshBuilder, StrokeStyle};
    use crate::color::Color;
    use crate::geometry::{Point, Rect};

    fn pts(points: &[(f32, f32)]) -> Vec<Point> {
        points.iter().map(|&(x, y)| Point { x, y }).collect()
    }

    #[test]
    fn a_square_fills_with_two_triangles_and_closes_its_ring() {
        let square = pts(&[(0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (0.0, 10.0)]);
        let mesh = Mesh::fill(std::slice::from_ref(&square), 1.0);
        assert_eq!(mesh.vertices().len(), 4);
        assert_eq!(mesh.indices().len(), 6);
        assert!(mesh.vertices().iter().all(|v| v.normal == [0.0, 0.0]));
        assert_eq!(
            mesh.bounds(),
            Some(Rect {
                x: 0.0,
                y: 0.0,
                width: 10.0,
                height: 10.0
            })
        );
        // The repeated first point of a closed GeoJSON ring adds nothing.
        let mut closed = square.clone();
        closed.push(square[0]);
        let again = Mesh::fill(&[closed], 1.0);
        assert_eq!(again.indices().len(), 6);
        assert_ne!(mesh.id(), again.id(), "every mesh has its own id");
    }

    #[test]
    fn holes_are_cut_out_of_fills() {
        let outer = pts(&[(0.0, 0.0), (30.0, 0.0), (30.0, 30.0), (0.0, 30.0)]);
        let hole = pts(&[(10.0, 10.0), (20.0, 10.0), (20.0, 20.0), (10.0, 20.0)]);
        let mesh = Mesh::fill(&[outer, hole], 1.0);
        assert_eq!(mesh.vertices().len(), 8);
        assert_eq!(mesh.indices().len(), 8 * 3, "a square ring is 8 triangles");
        let area: f32 = mesh
            .indices()
            .chunks(3)
            .map(|t| {
                let [a, b, c] = [0, 1, 2].map(|i| mesh.vertices()[t[i] as usize].pos);
                ((b[0] - a[0]) * (c[1] - a[1]) - (c[0] - a[0]) * (b[1] - a[1])).abs() / 2.0
            })
            .sum();
        assert!((area - 800.0).abs() < 0.01, "{area}");
    }

    #[test]
    fn strokes_sit_on_the_path_and_carry_unit_normals() {
        let line = pts(&[(0.0, 0.0), (100.0, 0.0)]);
        let mesh = Mesh::stroke(&[line], false, StrokeStyle::default(), 1.0);
        assert_eq!(mesh.vertices().len(), 4, "a butt segment is one quad");
        assert_eq!(mesh.indices().len(), 6);
        for vertex in mesh.vertices() {
            assert_eq!(vertex.pos[1], 0.0, "vertices stay on the path");
            assert_eq!(vertex.normal[0], 0.0);
            assert_eq!(vertex.normal[1].abs(), 1.0);
        }
        // A closed ring has no caps: four corners, two vertices each.
        let ring = pts(&[
            (0.0, 0.0),
            (10.0, 0.0),
            (10.0, 10.0),
            (0.0, 10.0),
            (0.0, 0.0),
        ]);
        let closed = Mesh::stroke(&[ring], true, StrokeStyle::default(), 1.0);
        assert_eq!(closed.indices().len(), 8 * 3);
        assert!(
            closed
                .vertices()
                .iter()
                .all(|v| (v.normal[0].hypot(v.normal[1]) - 2f32.sqrt()).abs() < 1e-4),
            "square miters are √2 long"
        );
        let round = Mesh::stroke(
            &[pts(&[(0.0, 0.0), (100.0, 0.0)])],
            false,
            StrokeStyle::ROUND,
            1.0,
        );
        assert!(round.vertices().len() > 4, "round caps add vertices");
    }

    #[test]
    fn degenerate_input_makes_empty_meshes() {
        assert!(Mesh::fill(&[pts(&[(1.0, 1.0)])], 1.0).is_empty());
        assert!(Mesh::fill(&[], 1.0).is_empty());
        assert!(
            Mesh::stroke(
                &[pts(&[(f32::NAN, 0.0)])],
                false,
                StrokeStyle::default(),
                1.0
            )
            .is_empty()
        );
        assert_eq!(Mesh::fill(&[], 1.0).bounds(), None);
        let mut builder = MeshBuilder::new(0.0);
        assert!(builder.is_empty());
        builder.fill(&[pts(&[(0.0, 0.0), (1.0, 0.0), (0.0, 1.0)])]);
        assert!(!builder.is_empty());
    }

    #[test]
    fn draws_extend_their_transformed_bounds_by_the_stroke() {
        let mesh = Arc::new(Mesh::fill(
            &[pts(&[(0.0, 0.0), (10.0, 0.0), (10.0, 10.0)])],
            1.0,
        ));
        let transform = Affine2::scale_translate(2.0, 5.0, 7.0);
        assert_eq!(
            transform.apply(Point { x: 1.0, y: 1.0 }),
            Point { x: 7.0, y: 9.0 }
        );
        let fill = GeometryDraw::fill(mesh.clone(), transform, Color::rgb(0, 0, 0));
        assert_eq!(
            fill.extent(),
            Some(Rect {
                x: 5.0,
                y: 7.0,
                width: 20.0,
                height: 20.0
            })
        );
        let stroke = GeometryDraw::stroke(mesh, transform, Color::rgb(0, 0, 0), 3.0);
        assert_eq!(stroke.extent().map(|r| r.x), Some(-1.0));
    }
}
