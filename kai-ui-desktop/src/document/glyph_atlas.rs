//! Atlas de glifos WordCraft → [`Bitmap`] Kai (sin cosmic-text / TextStore).
//!
//! Rasteriza outlines vía `swash` a partir del [`FaceRef`] de WordCraft y
//! cachea máscaras RGBA coloreadas para pintarlas como [`crate::render::ImageDraw`].

use std::collections::HashMap;

use swash::FontRef;
use swash::scale::{Render, ScaleContext, Source, StrikeWith};
use wordcraft_fonts::FaceRef;

use crate::render::{Bitmap, Color};

/// Clave de cache: cara + glifo + tamaño cuantizado + color.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct GlyphKey {
    face_id: u32,
    gid: u32,
    /// `font_size * 64` redondeado (px de raster).
    size_q: u32,
    color: u32,
}

#[derive(Clone, Debug)]
struct CachedGlyph {
    bitmap: Bitmap,
    /// Bearing izquierdo (px del raster).
    left: i32,
    /// Distancia baseline → top del bitmap (px del raster).
    top: i32,
}

/// Cache de glifos rasterizados para el cuerpo del documento.
pub struct DocumentGlyphAtlas {
    scale: ScaleContext,
    cache: HashMap<GlyphKey, CachedGlyph>,
}

impl Default for DocumentGlyphAtlas {
    fn default() -> Self {
        Self::new()
    }
}

impl DocumentGlyphAtlas {
    pub fn new() -> Self {
        Self {
            scale: ScaleContext::new(),
            cache: HashMap::new(),
        }
    }

    #[allow(dead_code)]
    pub fn clear(&mut self) {
        self.cache.clear();
    }

    pub fn len(&self) -> usize {
        self.cache.len()
    }

    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.cache.is_empty()
    }

    /// Rasteriza (o reutiliza) un glifo coloreado. Devuelve bitmap + left/top
    /// respecto al baseline WordCraft, en píxeles del tamaño pedido.
    pub fn glyph(
        &mut self,
        face: FaceRef,
        gid: u32,
        font_size_px: f32,
        color: Color,
        synth_bold: bool,
    ) -> Option<(Bitmap, i32, i32)> {
        let size = font_size_px.max(1.0);
        let size_q = (size * 64.0).round() as u32;
        let key = GlyphKey {
            face_id: face.id(),
            gid,
            size_q,
            color: color_key(color),
        };
        if let Some(hit) = self.cache.get(&key) {
            return Some((hit.bitmap.clone(), hit.left, hit.top));
        }
        let cached = rasterize(&mut self.scale, face, gid, size, color, synth_bold)?;
        let out = (cached.bitmap.clone(), cached.left, cached.top);
        self.cache.insert(key, cached);
        Some(out)
    }
}

fn color_key(c: Color) -> u32 {
    (u32::from(c.r) << 24) | (u32::from(c.g) << 16) | (u32::from(c.b) << 8) | u32::from(c.a)
}

fn rasterize(
    scale: &mut ScaleContext,
    face: FaceRef,
    gid: u32,
    size: f32,
    color: Color,
    synth_bold: bool,
) -> Option<CachedGlyph> {
    let data = face.data();
    let font = FontRef::from_index(data, face.index() as usize)?;
    let mut scaler = scale.builder(font).size(size).hint(true).build();
    let sources = [
        Source::ColorOutline(0),
        Source::ColorBitmap(StrikeWith::BestFit),
        Source::Outline,
    ];
    // Default format is Alpha.
    let mut render = Render::new(&sources);
    if synth_bold {
        // ~2 % del em en píxeles (faux bold WordCraft).
        render.embolden(size * 0.02);
    }
    let image = render.render(&mut scaler, gid as u16)?;

    let w = image.placement.width;
    let h = image.placement.height;
    if w == 0 || h == 0 || image.data.is_empty() {
        return None;
    }

    let rgba = match image.content {
        swash::scale::image::Content::Mask => {
            let mut out = Vec::with_capacity((w * h * 4) as usize);
            for &a in &image.data {
                let alpha = ((u16::from(a) * u16::from(color.a)) / 255) as u8;
                out.extend_from_slice(&[color.r, color.g, color.b, alpha]);
            }
            out
        }
        swash::scale::image::Content::SubpixelMask => {
            let mut out = Vec::with_capacity((w * h * 4) as usize);
            for chunk in image.data.chunks(4) {
                let a = chunk.first().copied().unwrap_or(0);
                let alpha = ((u16::from(a) * u16::from(color.a)) / 255) as u8;
                out.extend_from_slice(&[color.r, color.g, color.b, alpha]);
            }
            out
        }
        swash::scale::image::Content::Color => {
            let mut out = image.data;
            if color.a < 255 {
                for px in out.chunks_exact_mut(4) {
                    px[3] = ((u16::from(px[3]) * u16::from(color.a)) / 255) as u8;
                }
            }
            out
        }
    };

    if rgba.len() != (w * h * 4) as usize {
        return None;
    }

    Some(CachedGlyph {
        bitmap: Bitmap::new(w, h, rgba),
        left: image.placement.left,
        top: image.placement.top,
    })
}
