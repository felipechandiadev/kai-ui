//! Backend-independent draw operations.
//!
//! Paint code produces a [`DrawList`] in logical window coordinates and in
//! painter's order (later ops cover earlier ones, text included); a backend
//! (currently [`wgpu_backend::WgpuRenderer`]) turns it into GPU work.
//! [`resolve`] splits the list into alternating quad, image, geometry and
//! text [`Batch`]es that keep that order and carries each op's effective
//! clip.

pub mod geometry;
pub mod icons;
pub mod images;
pub mod paint;
pub mod text;
pub mod wgpu_backend;

use crate::background::BackgroundLayer;
use crate::element::WidgetId;
use crate::geometry::{ClipRect, Point, Rect};
use crate::icons::Icon;
pub use geometry::{Affine2, GeometryDraw, Mesh};
pub use images::Bitmap;

pub use crate::color::{Color, srgb_to_linear};

/// Which text of a widget a buffer holds.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TextPart {
    /// Text element content.
    Content,
    /// Input value, including IME preedit.
    Value,
    Placeholder,
    /// A map label, by a hash of its kind and name.
    MapLabel(u64),
}

/// Text buffer key in the renderer's text store.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct TextId {
    pub widget: WidgetId,
    pub part: TextPart,
}

impl TextId {
    pub const fn content(widget: WidgetId) -> Self {
        Self {
            widget,
            part: TextPart::Content,
        }
    }

    pub const fn value(widget: WidgetId) -> Self {
        Self {
            widget,
            part: TextPart::Value,
        }
    }

    pub const fn placeholder(widget: WidgetId) -> Self {
        Self {
            widget,
            part: TextPart::Placeholder,
        }
    }

    pub const fn map_label(map: WidgetId, key: u64) -> Self {
        Self {
            widget: map,
            part: TextPart::MapLabel(key),
        }
    }
}

/// Coverage mask that shapes a quad: the quad's fill shows only where the
/// icon is drawn.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Mask {
    pub icon: Icon,
    /// In units of the icon's 24×24 view box.
    pub stroke_width: f32,
}

/// A rounded rectangle with optional border, a blurred shape (shadow), or
/// an icon filled with one color.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Quad {
    pub rect: Rect,
    pub fill: Color,
    pub border: Color,
    /// Inside `rect`, as CSS borders with `box-sizing: border-box`.
    pub border_width: f32,
    pub radius: f32,
    /// Edge softness in logical points; `> 0` draws a shadow and ignores the
    /// border.
    pub blur: f32,
    /// Draw only the icon's coverage; radius, border and blur are ignored.
    pub mask: Option<Mask>,
}

impl Quad {
    pub const fn fill(rect: Rect, fill: Color) -> Self {
        Self {
            rect,
            fill,
            border: TRANSPARENT,
            border_width: 0.0,
            radius: 0.0,
            blur: 0.0,
            mask: None,
        }
    }

    pub const fn icon(rect: Rect, fill: Color, mask: Mask) -> Self {
        Self {
            mask: Some(mask),
            ..Self::fill(rect, fill)
        }
    }

    /// Area the quad can touch, including blur.
    pub fn extent(&self) -> Rect {
        let grow = self.blur;
        Rect {
            x: self.rect.x - grow,
            y: self.rect.y - grow,
            width: self.rect.width + grow * 2.0,
            height: self.rect.height + grow * 2.0,
        }
    }
}

pub const TRANSPARENT: Color = Color::rgba(0, 0, 0, 0);

/// A raster image stretched over `rect`.
#[derive(Clone, Debug, PartialEq)]
pub struct ImageDraw {
    pub rect: Rect,
    pub bitmap: Bitmap,
    /// Part of the bitmap shown, as fractions of its size (`0..1`); a crop
    /// for `object-cover`.
    pub source: Rect,
    /// Corner radius of `rect`, in logical points; `rect` is rounded like a
    /// quad (`rounded-full` makes a circle).
    pub radius: f32,
    pub opacity: f32,
}

impl ImageDraw {
    /// The whole of `bitmap` over `rect`, square and opaque.
    pub fn new(rect: Rect, bitmap: Bitmap) -> Self {
        Self {
            rect,
            bitmap,
            source: Rect {
                x: 0.0,
                y: 0.0,
                width: 1.0,
                height: 1.0,
            },
            radius: 0.0,
            opacity: 1.0,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum DrawOp {
    Quad(Quad),
    Image(ImageDraw),
    /// A triangle mesh (fill or screen-width stroke), antialiased by MSAA.
    Geometry(GeometryDraw),
    /// Intersect the current clip with `clip` until the matching `PopClip`.
    /// A rounded clip keeps an SDF mask so children follow the curve.
    PushClip(ClipRect),
    PopClip,
    Text {
        id: TextId,
        origin: Point,
        color: Color,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub struct DrawList {
    pub clear: Color,
    /// Window background effect, drawn over `clear` and under every op.
    pub background: Option<BackgroundLayer>,
    pub ops: Vec<DrawOp>,
}

impl DrawList {
    pub fn new(clear: Color) -> Self {
        Self {
            clear,
            background: None,
            ops: Vec::new(),
        }
    }

    pub fn quad(&mut self, quad: Quad) {
        self.ops.push(DrawOp::Quad(quad));
    }

    pub fn fill_rect(&mut self, rect: Rect, color: Color) {
        self.quad(Quad::fill(rect, color));
    }

    pub fn image(&mut self, image: ImageDraw) {
        self.ops.push(DrawOp::Image(image));
    }

    pub fn geometry(&mut self, draw: GeometryDraw) {
        self.ops.push(DrawOp::Geometry(draw));
    }

    pub fn push_clip(&mut self, rect: Rect) {
        self.push_clip_rect(ClipRect::sharp(rect));
    }

    pub fn push_clip_rect(&mut self, clip: ClipRect) {
        self.ops.push(DrawOp::PushClip(clip));
    }

    pub fn pop_clip(&mut self) {
        self.ops.push(DrawOp::PopClip);
    }

    pub fn text(&mut self, id: TextId, origin: Point, color: Color) {
        self.ops.push(DrawOp::Text { id, origin, color });
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ClippedQuad {
    pub quad: Quad,
    /// Effective clip; `None` means unclipped (whole surface).
    pub clip: Option<ClipRect>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TextDraw {
    pub id: TextId,
    pub origin: Point,
    pub color: Color,
    /// Effective clip; `None` means unclipped (whole surface).
    /// Text shaping uses the AABB only; the rounded mask is ignored here.
    pub clip: Option<ClipRect>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ClippedImage {
    pub image: ImageDraw,
    /// Effective clip; `None` means unclipped (whole surface).
    pub clip: Option<ClipRect>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ClippedGeometry {
    pub draw: GeometryDraw,
    /// Effective clip; `None` means unclipped (whole surface).
    pub clip: Option<ClipRect>,
}

/// Consecutive ops of one kind; batches are drawn in order.
#[derive(Clone, Debug, PartialEq)]
pub enum Batch {
    Quads(Vec<ClippedQuad>),
    Images(Vec<ClippedImage>),
    Geometry(Vec<ClippedGeometry>),
    Texts(Vec<TextDraw>),
}

#[derive(Clone, Copy)]
enum Clip {
    Unclipped,
    Visible(ClipRect),
    Empty,
}

impl Clip {
    fn clip(self) -> Option<Option<ClipRect>> {
        match self {
            Self::Unclipped => Some(None),
            Self::Visible(clip) => Some(Some(clip)),
            Self::Empty => None,
        }
    }
}

/// Resolve clips and group ops into ordered batches. Fully clipped and
/// fully transparent ops are dropped.
pub fn resolve(list: &DrawList) -> Vec<Batch> {
    let mut batches: Vec<Batch> = Vec::new();
    let mut stack = vec![Clip::Unclipped];
    for op in &list.ops {
        let current = *stack.last().unwrap_or(&Clip::Unclipped);
        match op {
            DrawOp::PushClip(rect) => stack.push(match current {
                Clip::Unclipped => Clip::Visible(*rect),
                Clip::Visible(clip) => clip.intersection(*rect).map_or(Clip::Empty, Clip::Visible),
                Clip::Empty => Clip::Empty,
            }),
            DrawOp::PopClip => {
                if stack.len() > 1 {
                    stack.pop();
                }
            }
            DrawOp::Quad(quad) => {
                let Some(clip) = current.clip() else {
                    continue;
                };
                let invisible =
                    quad.fill.a == 0 && (quad.border.a == 0 || quad.border_width <= 0.0);
                let clipped_out =
                    clip.is_some_and(|clip| quad.extent().intersection(clip.rect).is_none());
                if invisible || clipped_out {
                    continue;
                }
                let item = ClippedQuad { quad: *quad, clip };
                match batches.last_mut() {
                    Some(Batch::Quads(quads)) => quads.push(item),
                    _ => batches.push(Batch::Quads(vec![item])),
                }
            }
            DrawOp::Image(image) => {
                let Some(clip) = current.clip() else {
                    continue;
                };
                let empty = image.rect.width <= 0.0 || image.rect.height <= 0.0;
                let clipped_out =
                    clip.is_some_and(|clip| image.rect.intersection(clip.rect).is_none());
                if image.opacity <= 0.0 || empty || clipped_out {
                    continue;
                }
                let item = ClippedImage {
                    image: image.clone(),
                    clip,
                };
                match batches.last_mut() {
                    Some(Batch::Images(images)) => images.push(item),
                    _ => batches.push(Batch::Images(vec![item])),
                }
            }
            DrawOp::Geometry(draw) => {
                let Some(clip) = current.clip() else {
                    continue;
                };
                let Some(extent) = draw.extent() else {
                    continue;
                };
                let clipped_out = clip.is_some_and(|clip| extent.intersection(clip.rect).is_none());
                if draw.color.a == 0 || draw.mesh.is_empty() || clipped_out {
                    continue;
                }
                let item = ClippedGeometry {
                    draw: draw.clone(),
                    clip,
                };
                match batches.last_mut() {
                    Some(Batch::Geometry(draws)) => draws.push(item),
                    _ => batches.push(Batch::Geometry(vec![item])),
                }
            }
            DrawOp::Text { id, origin, color } => {
                let Some(clip) = current.clip() else {
                    continue;
                };
                if color.a == 0 {
                    continue;
                }
                let item = TextDraw {
                    id: *id,
                    origin: *origin,
                    color: *color,
                    clip,
                };
                match batches.last_mut() {
                    Some(Batch::Texts(texts)) => texts.push(item),
                    _ => batches.push(Batch::Texts(vec![item])),
                }
            }
        }
    }
    batches
}

#[cfg(test)]
mod tests {
    use super::{Batch, Color, DrawList, Quad, TextId, resolve};
    use crate::element::WidgetId;
    use crate::geometry::{ClipRect, Point, Rect};

    const TEXT: TextId = TextId::content(WidgetId(1));

    const fn rect(x: f32, y: f32, width: f32, height: f32) -> Rect {
        Rect {
            x,
            y,
            width,
            height,
        }
    }

    #[test]
    fn batches_keep_painter_order_between_quads_and_text() {
        let red = Color::rgb(255, 0, 0);
        let mut list = DrawList::new(Color::rgb(0, 0, 0));
        list.fill_rect(rect(0.0, 0.0, 10.0, 10.0), red);
        list.fill_rect(rect(5.0, 0.0, 10.0, 10.0), red);
        list.text(TEXT, Point::default(), red);
        list.text(TEXT, Point::default(), red);
        list.fill_rect(rect(0.0, 0.0, 10.0, 10.0), red);
        list.text(TEXT, Point::default(), red);
        let batches = resolve(&list);
        let shape: Vec<(bool, usize)> = batches
            .iter()
            .map(|batch| match batch {
                Batch::Quads(quads) => (true, quads.len()),
                Batch::Images(_) | Batch::Geometry(_) => unreachable!("no images here"),
                Batch::Texts(texts) => (false, texts.len()),
            })
            .collect();
        assert_eq!(shape, vec![(true, 2), (false, 2), (true, 1), (false, 1)]);
    }

    #[test]
    fn nested_clips_intersect_restore_and_drop_hidden_ops() {
        let red = Color::rgb(255, 0, 0);
        let mut list = DrawList::new(Color::rgb(0, 0, 0));
        list.push_clip(rect(0.0, 0.0, 100.0, 100.0));
        list.push_clip(rect(50.0, 50.0, 100.0, 100.0));
        list.fill_rect(rect(0.0, 0.0, 80.0, 80.0), red);
        list.fill_rect(rect(0.0, 0.0, 10.0, 10.0), red);
        list.text(TEXT, Point { x: 1.0, y: 2.0 }, red);
        list.push_clip(rect(200.0, 200.0, 10.0, 10.0));
        list.text(TEXT, Point::default(), red);
        list.pop_clip();
        list.pop_clip();
        list.fill_rect(rect(0.0, 0.0, 10.0, 10.0), Color::rgba(0, 0, 0, 0));
        list.pop_clip();
        list.pop_clip();
        list.quad(Quad {
            blur: 4.0,
            ..Quad::fill(rect(200.0, 200.0, 5.0, 5.0), red)
        });

        let batches = resolve(&list);
        assert_eq!(batches.len(), 3);
        let Batch::Quads(quads) = &batches[0] else {
            panic!("quads first");
        };
        assert_eq!(quads.len(), 1, "quad outside the clip is dropped");
        assert_eq!(
            quads[0].clip,
            Some(ClipRect::sharp(rect(50.0, 50.0, 50.0, 50.0)))
        );
        assert_eq!(
            quads[0].quad.rect,
            rect(0.0, 0.0, 80.0, 80.0),
            "clip is per instance"
        );
        let Batch::Texts(texts) = &batches[1] else {
            panic!("then text");
        };
        assert_eq!(texts.len(), 1, "text inside an empty clip is dropped");
        let Batch::Quads(last) = &batches[2] else {
            panic!("transparent quad dropped, shadow kept");
        };
        assert_eq!(last[0].clip, None);
        assert_eq!(last[0].quad.extent(), rect(196.0, 196.0, 13.0, 13.0));
    }

    #[test]
    fn images_batch_between_quads_and_text_and_drop_when_hidden() {
        use super::ImageDraw;
        use super::images::test_pattern;
        let red = Color::rgb(255, 0, 0);
        let photo = test_pattern(8, 8);
        let mut list = DrawList::new(Color::rgb(0, 0, 0));
        list.fill_rect(rect(0.0, 0.0, 10.0, 10.0), red);
        list.image(ImageDraw::new(rect(0.0, 0.0, 40.0, 40.0), photo.clone()));
        list.image(ImageDraw {
            radius: 20.0,
            ..ImageDraw::new(rect(50.0, 0.0, 40.0, 40.0), photo.clone())
        });
        list.text(TEXT, Point::default(), red);
        list.image(ImageDraw {
            opacity: 0.0,
            ..ImageDraw::new(rect(0.0, 0.0, 40.0, 40.0), photo.clone())
        });
        list.image(ImageDraw::new(rect(0.0, 0.0, 0.0, 40.0), photo.clone()));
        list.push_clip(rect(0.0, 0.0, 30.0, 30.0));
        list.image(ImageDraw::new(
            rect(100.0, 100.0, 10.0, 10.0),
            photo.clone(),
        ));
        list.image(ImageDraw::new(rect(20.0, 20.0, 40.0, 40.0), photo.clone()));
        list.pop_clip();
        list.fill_rect(rect(0.0, 0.0, 10.0, 10.0), red);

        let batches = resolve(&list);
        let shape: Vec<(&str, usize)> = batches
            .iter()
            .map(|batch| match batch {
                Batch::Quads(quads) => ("quads", quads.len()),
                Batch::Images(images) => ("images", images.len()),
                Batch::Geometry(_) => unreachable!("no geometry here"),
                Batch::Texts(texts) => ("texts", texts.len()),
            })
            .collect();
        assert_eq!(
            shape,
            [
                ("quads", 1),
                ("images", 2),
                ("texts", 1),
                ("images", 1),
                ("quads", 1)
            ],
            "transparent, empty and clipped-out images are dropped"
        );
        let Batch::Images(clipped) = &batches[3] else {
            panic!("images");
        };
        assert_eq!(
            clipped[0].clip,
            Some(ClipRect::sharp(rect(0.0, 0.0, 30.0, 30.0)))
        );
        assert_eq!(clipped[0].image.bitmap, photo);
        let Batch::Images(first) = &batches[1] else {
            panic!("images");
        };
        assert_eq!(first[1].image.radius, 20.0);
    }

    #[test]
    fn geometry_batches_between_quads_and_images_and_drops_when_hidden() {
        use super::images::test_pattern;
        use super::{Affine2, GeometryDraw, ImageDraw, Mesh};
        use std::sync::Arc;
        let red = Color::rgb(255, 0, 0);
        let triangle = Arc::new(Mesh::fill(
            &[vec![
                Point { x: 0.0, y: 0.0 },
                Point { x: 10.0, y: 0.0 },
                Point { x: 0.0, y: 10.0 },
            ]],
            1.0,
        ));
        let at = |x: f32| Affine2::scale_translate(1.0, x, 0.0);
        let mut list = DrawList::new(Color::rgb(0, 0, 0));
        list.fill_rect(rect(0.0, 0.0, 10.0, 10.0), red);
        list.geometry(GeometryDraw::fill(triangle.clone(), at(0.0), red));
        list.geometry(GeometryDraw::stroke(triangle.clone(), at(5.0), red, 2.0));
        list.image(ImageDraw::new(rect(0.0, 0.0, 4.0, 4.0), test_pattern(2, 2)));
        list.geometry(GeometryDraw::fill(
            triangle.clone(),
            at(0.0),
            Color::rgba(255, 0, 0, 0),
        ));
        list.geometry(GeometryDraw::fill(
            Arc::new(Mesh::fill(&[], 1.0)),
            at(0.0),
            red,
        ));
        list.push_clip(rect(0.0, 0.0, 20.0, 20.0));
        list.geometry(GeometryDraw::fill(triangle.clone(), at(100.0), red));
        list.geometry(GeometryDraw::stroke(triangle.clone(), at(22.0), red, 2.0));
        list.pop_clip();
        list.fill_rect(rect(0.0, 0.0, 10.0, 10.0), red);

        let batches = resolve(&list);
        let shape: Vec<(&str, usize)> = batches
            .iter()
            .map(|batch| match batch {
                Batch::Quads(quads) => ("quads", quads.len()),
                Batch::Images(images) => ("images", images.len()),
                Batch::Geometry(draws) => ("geometry", draws.len()),
                Batch::Texts(texts) => ("texts", texts.len()),
            })
            .collect();
        assert_eq!(
            shape,
            [
                ("quads", 1),
                ("geometry", 2),
                ("images", 1),
                ("geometry", 1),
                ("quads", 1)
            ],
            "transparent, empty and clipped-out geometry is dropped; a stroke \
             just past the clip still reaches into it"
        );
        let Batch::Geometry(clipped) = &batches[3] else {
            panic!("geometry");
        };
        assert_eq!(
            clipped[0].clip,
            Some(ClipRect::sharp(rect(0.0, 0.0, 20.0, 20.0)))
        );
        assert_eq!(clipped[0].draw.width, 2.0);
    }

    #[test]
    fn rounded_clip_keeps_the_sdf_mask_through_resolve() {
        let mut list = DrawList::new(Color::rgb(255, 255, 255));
        list.push_clip_rect(ClipRect::rounded(rect(0.0, 0.0, 100.0, 100.0), 20.0));
        list.fill_rect(rect(0.0, 0.0, 100.0, 100.0), Color::rgb(0, 0, 0));
        list.pop_clip();
        let batches = resolve(&list);
        let Batch::Quads(quads) = &batches[0] else {
            panic!("expected quads");
        };
        let clip = quads[0].clip.expect("clipped");
        assert!(clip.round.is_some());
        assert!((clip.round.unwrap().radius - 20.0).abs() < f32::EPSILON);
        list = DrawList::new(Color::rgb(255, 255, 255));
        list.push_clip_rect(ClipRect::rounded(rect(0.0, 0.0, 100.0, 100.0), 20.0));
        list.push_clip(rect(10.0, 10.0, 40.0, 40.0));
        list.fill_rect(rect(0.0, 0.0, 100.0, 100.0), Color::rgb(0, 0, 0));
        list.pop_clip();
        list.pop_clip();
        let batches = resolve(&list);
        let Batch::Quads(quads) = &batches[0] else {
            panic!("expected quads");
        };
        let clip = quads[0].clip.expect("nested clip");
        assert_eq!(clip.rect.x, 10.0);
        assert!(
            clip.round.is_some(),
            "sharp child clip keeps the parent rounded mask"
        );
    }
}
