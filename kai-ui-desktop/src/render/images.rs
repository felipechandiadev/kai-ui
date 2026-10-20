//! Raster images ([`DrawOp::Image`](super::DrawOp::Image)): decoded
//! [`Bitmap`]s packed into one RGBA atlas that `image.wgsl` samples with a
//! linear filter. Pixels go in premultiplied, scaled down to
//! [`MAX_IMAGE_SIDE`] first, so a photo never takes more than a corner of
//! the atlas.

use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

/// Side of the square `Rgba8Unorm` image atlas, in texels.
pub const IMAGE_ATLAS_SIZE: u32 = 4096;
/// Longest side an image keeps in the atlas; larger ones are scaled down.
pub const MAX_IMAGE_SIDE: u32 = 1024;
/// Empty texels kept around each image, so filtering never reads a
/// neighbor.
const PADDING: u32 = 1;

/// Decoded pixels: straight-alpha RGBA8, sRGB-encoded, row by row from the
/// top. Clones share the pixels; every [`Bitmap::new`] gets its own id,
/// which is what the atlas and `==` go by.
#[derive(Clone)]
pub struct Bitmap {
    id: u64,
    width: u32,
    height: u32,
    pixels: Arc<[u8]>,
}

impl Bitmap {
    /// `pixels` must hold `width × height × 4` bytes.
    pub fn new(width: u32, height: u32, pixels: impl Into<Arc<[u8]>>) -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(1);
        let pixels = pixels.into();
        assert_eq!(
            pixels.len(),
            width as usize * height as usize * 4,
            "un Bitmap de {width}×{height} lleva {} bytes",
            width as usize * height as usize * 4
        );
        Self {
            id: NEXT.fetch_add(1, Ordering::Relaxed),
            width,
            height,
            pixels,
        }
    }

    pub fn id(&self) -> u64 {
        self.id
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    pub fn pixels(&self) -> &[u8] {
        &self.pixels
    }
}

impl PartialEq for Bitmap {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl fmt::Debug for Bitmap {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Bitmap#{}({}×{})", self.id, self.width, self.height)
    }
}

/// Size `width × height` takes once its longest side is at most `max`,
/// keeping the aspect ratio (each side at least 1).
pub fn fitted_size(width: u32, height: u32, max: u32) -> (u32, u32) {
    let longest = width.max(height);
    if longest <= max {
        return (width, height);
    }
    let scale = max as f64 / longest as f64;
    (
        ((width as f64 * scale).round() as u32).clamp(1, max),
        ((height as f64 * scale).round() as u32).clamp(1, max),
    )
}

/// Straight-alpha RGBA scaled to `to_width × to_height` by averaging the
/// source pixels under each target pixel (premultiplied while averaging,
/// so transparent pixels do not darken the edges), returned premultiplied.
pub fn scale_premultiplied(
    pixels: &[u8],
    width: u32,
    height: u32,
    to_width: u32,
    to_height: u32,
) -> Vec<u8> {
    let mut out = Vec::with_capacity(to_width as usize * to_height as usize * 4);
    let x_ratio = width as f64 / to_width as f64;
    let y_ratio = height as f64 / to_height as f64;
    for ty in 0..to_height {
        let y0 = (ty as f64 * y_ratio) as u32;
        let y1 = (((ty + 1) as f64 * y_ratio).ceil() as u32).clamp(y0 + 1, height);
        for tx in 0..to_width {
            let x0 = (tx as f64 * x_ratio) as u32;
            let x1 = (((tx + 1) as f64 * x_ratio).ceil() as u32).clamp(x0 + 1, width);
            let mut sum = [0u64; 4];
            for y in y0..y1 {
                let row = (y * width) as usize * 4;
                for x in x0..x1 {
                    let px = &pixels[row + x as usize * 4..][..4];
                    let alpha = u64::from(px[3]);
                    sum[0] += u64::from(px[0]) * alpha;
                    sum[1] += u64::from(px[1]) * alpha;
                    sum[2] += u64::from(px[2]) * alpha;
                    sum[3] += alpha;
                }
            }
            let count = u64::from((y1 - y0) * (x1 - x0));
            let channel = |total: u64| ((total + count * 255 / 2) / (count * 255)) as u8;
            out.extend_from_slice(&[
                channel(sum[0]),
                channel(sum[1]),
                channel(sum[2]),
                ((sum[3] + count / 2) / count) as u8,
            ]);
        }
    }
    out
}

/// Atlas region of an image, in texels.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ImageRegion {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

#[derive(Clone, Copy, Debug)]
struct Shelf {
    y: u32,
    height: u32,
    x: u32,
}

/// Shelf-packed cache of images by [`Bitmap::id`]. Scaled, premultiplied
/// pixels wait in [`ImageAtlas::take_uploads`] until the backend copies
/// them to the GPU.
#[derive(Debug)]
pub struct ImageAtlas {
    size: u32,
    shelves: Vec<Shelf>,
    next_y: u32,
    entries: HashMap<u64, ImageRegion>,
    uploads: Vec<(ImageRegion, Vec<u8>)>,
}

impl Default for ImageAtlas {
    fn default() -> Self {
        Self::new(IMAGE_ATLAS_SIZE)
    }
}

impl ImageAtlas {
    pub fn new(size: u32) -> Self {
        Self {
            size,
            shelves: Vec::new(),
            next_y: 0,
            entries: HashMap::new(),
            uploads: Vec::new(),
        }
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    fn clear(&mut self) {
        self.shelves.clear();
        self.next_y = 0;
        self.entries.clear();
        self.uploads.clear();
    }

    fn allocate(&mut self, width: u32, height: u32) -> Option<ImageRegion> {
        let (padded_width, padded_height) = (width + PADDING * 2, height + PADDING * 2);
        if padded_width > self.size || padded_height > self.size {
            return None;
        }
        let limit = self.size;
        let shelf = self
            .shelves
            .iter_mut()
            .filter(|shelf| shelf.height >= padded_height && shelf.x + padded_width <= limit)
            .min_by_key(|shelf| shelf.height);
        let shelf = match shelf {
            Some(shelf) => shelf,
            None => {
                if self.next_y + padded_height > self.size {
                    return None;
                }
                self.shelves.push(Shelf {
                    y: self.next_y,
                    height: padded_height,
                    x: 0,
                });
                self.next_y += padded_height;
                self.shelves.last_mut()?
            }
        };
        let region = ImageRegion {
            x: shelf.x + PADDING,
            y: shelf.y + PADDING,
            width,
            height,
        };
        shelf.x += padded_width;
        Some(region)
    }

    /// `None` when the atlas is full.
    fn insert(&mut self, bitmap: &Bitmap) -> Option<ImageRegion> {
        if let Some(region) = self.entries.get(&bitmap.id) {
            return Some(*region);
        }
        let max = MAX_IMAGE_SIDE.min(self.size - PADDING * 2);
        let (width, height) = fitted_size(bitmap.width, bitmap.height, max);
        let region = self.allocate(width, height)?;
        let pixels =
            scale_premultiplied(&bitmap.pixels, bitmap.width, bitmap.height, width, height);
        self.entries.insert(bitmap.id, region);
        self.uploads.push((region, pixels));
        Some(region)
    }

    /// Regions for the images of a frame. When the atlas fills up it is
    /// emptied and repacked with this frame's images only (the ones not
    /// drawn lately go first), so all returned regions are valid
    /// together; what still does not fit is left out.
    pub fn prepare(&mut self, bitmaps: &[&Bitmap]) -> HashMap<u64, ImageRegion> {
        let mut regions = HashMap::new();
        let mut full = false;
        for bitmap in bitmaps {
            match self.insert(bitmap) {
                Some(region) => {
                    regions.insert(bitmap.id, region);
                }
                None => {
                    full = true;
                    break;
                }
            }
        }
        if !full {
            return regions;
        }
        self.clear();
        regions.clear();
        for bitmap in bitmaps {
            if let Some(region) = self.insert(bitmap) {
                regions.insert(bitmap.id, region);
            }
        }
        regions
    }

    /// Images packed since the last call, to copy into the texture.
    pub fn take_uploads(&mut self) -> Vec<(ImageRegion, Vec<u8>)> {
        std::mem::take(&mut self.uploads)
    }
}

/// A test image: a horizontal hue ramp over a vertical fade to white, with
/// a dark 4 × 4 checker on top, `width × height`, opaque.
pub fn test_pattern(width: u32, height: u32) -> Bitmap {
    let mut pixels = Vec::with_capacity(width as usize * height as usize * 4);
    for y in 0..height {
        for x in 0..width {
            let t = x as f32 / width.max(1) as f32;
            let v = y as f32 / height.max(1) as f32;
            let ramp = |phase: f32| {
                let c = ((t + phase) * std::f32::consts::TAU).cos() * 0.5 + 0.5;
                ((c * (1.0 - v) + v) * 255.0) as u8
            };
            let mut px = [ramp(0.0), ramp(1.0 / 3.0), ramp(2.0 / 3.0), 255];
            if (x / (width / 4).max(1) + y / (height / 4).max(1)).is_multiple_of(2) {
                px[..3]
                    .iter_mut()
                    .for_each(|c| *c = (*c as f32 * 0.8) as u8);
            }
            pixels.extend_from_slice(&px);
        }
    }
    Bitmap::new(width, height, pixels)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn solid(width: u32, height: u32, px: [u8; 4]) -> Bitmap {
        Bitmap::new(width, height, px.repeat(width as usize * height as usize))
    }

    #[test]
    fn bitmaps_are_equal_by_identity_and_check_their_size() {
        let a = solid(2, 2, [1, 2, 3, 255]);
        let b = solid(2, 2, [1, 2, 3, 255]);
        assert_eq!(a, a.clone());
        assert_ne!(a, b, "same pixels, another image");
        assert_eq!(format!("{a:?}"), format!("Bitmap#{}(2×2)", a.id()));
        let wrong = std::panic::catch_unwind(|| Bitmap::new(2, 2, vec![0u8; 3]));
        assert!(wrong.is_err());
    }

    #[test]
    fn large_images_fit_the_longest_side_keeping_the_ratio() {
        assert_eq!(fitted_size(800, 600, 1024), (800, 600));
        assert_eq!(fitted_size(4000, 3000, 1024), (1024, 768));
        assert_eq!(fitted_size(3000, 4000, 1024), (768, 1024));
        assert_eq!(fitted_size(5000, 2, 1024), (1024, 1));
    }

    #[test]
    fn scaling_averages_premultiplied_pixels() {
        // Opaque red next to a transparent pixel: the average is half
        // as opaque and still red, not darkened toward black.
        let pixels = [255, 0, 0, 255, 0, 0, 0, 0];
        assert_eq!(scale_premultiplied(&pixels, 2, 1, 1, 1), [128, 0, 0, 128]);
        // At the same size it only premultiplies.
        let pixels = [200, 100, 50, 128];
        assert_eq!(scale_premultiplied(&pixels, 1, 1, 1, 1), [100, 50, 25, 128]);
        let big = solid(8, 4, [10, 20, 30, 255]);
        assert_eq!(
            scale_premultiplied(big.pixels(), 8, 4, 2, 1),
            [10, 20, 30, 255, 10, 20, 30, 255]
        );
    }

    #[test]
    fn the_atlas_packs_caches_and_repacks_when_full() {
        let mut atlas = ImageAtlas::new(48);
        let a = solid(20, 10, [255; 4]);
        let b = solid(20, 20, [255; 4]);
        let regions = atlas.prepare(&[&a, &b, &a]);
        assert_eq!(
            regions[&a.id()],
            ImageRegion {
                x: 1,
                y: 1,
                width: 20,
                height: 10
            }
        );
        assert_eq!(regions[&b.id()].y, 13, "a taller image opens a shelf");
        let uploads = atlas.take_uploads();
        assert_eq!(uploads.len(), 2);
        assert_eq!(uploads[0].1.len(), 20 * 10 * 4);

        assert_eq!(atlas.prepare(&[&b])[&b.id()], regions[&b.id()]);
        assert!(atlas.take_uploads().is_empty(), "cached images stay");

        let c = solid(40, 30, [255; 4]);
        let regions = atlas.prepare(&[&c, &b]);
        assert_eq!(
            regions.len(),
            1,
            "repacked with this frame's images; b no longer fits"
        );
        assert_eq!(regions[&c.id()].y, 1);
        assert_eq!(atlas.len(), 1);

        let huge = solid(60, 5, [255; 4]);
        let regions = atlas.prepare(&[&huge]);
        assert_eq!(
            (regions[&huge.id()].width, regions[&huge.id()].height),
            (46, 4),
            "scaled down to the atlas"
        );
    }

    #[test]
    fn the_test_pattern_is_opaque_and_varied() {
        let bitmap = test_pattern(16, 8);
        assert_eq!(bitmap.pixels().len(), 16 * 8 * 4);
        assert!(bitmap.pixels().chunks(4).all(|px| px[3] == 255));
        assert_ne!(bitmap.pixels()[..4], bitmap.pixels()[8 * 4..][..4]);
    }
}
