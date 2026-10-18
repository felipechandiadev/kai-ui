//! Icon masks: SVG icons rasterized with resvg at their physical pixel size
//! and packed into a single-channel atlas. The quad shader multiplies the
//! quad's fill by the mask coverage, so icons take any color.

use std::collections::HashMap;

use resvg::{tiny_skia, usvg};

use crate::icons::{self, Icon};

/// Side of the square `R8Unorm` atlas texture, in pixels.
pub const ATLAS_SIZE: u32 = 1024;
/// Empty pixels kept around each mask.
const PADDING: u32 = 1;

/// One rasterization of an icon: size in physical pixels and stroke width in
/// hundredths of a view-box unit.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct MaskKey {
    pub icon: Icon,
    pub px: u16,
    pub stroke: u16,
}

impl MaskKey {
    pub fn new(icon: Icon, px: f32, stroke_width: f32) -> Self {
        Self {
            icon,
            px: px.round().clamp(1.0, f32::from(u16::MAX)) as u16,
            stroke: (stroke_width * 100.0)
                .round()
                .clamp(0.0, f32::from(u16::MAX)) as u16,
        }
    }

    pub fn stroke_width(self) -> f32 {
        f32::from(self.stroke) / 100.0
    }
}

/// Atlas region in pixels.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AtlasRect {
    pub x: u32,
    pub y: u32,
    pub size: u32,
}

/// Coverage (alpha) of `icon` rendered into a `px` × `px` square.
pub fn rasterize(icon: Icon, px: u32, stroke_width: f32) -> Option<Vec<u8>> {
    let tree =
        usvg::Tree::from_str(&icons::svg(icon, stroke_width), &usvg::Options::default()).ok()?;
    let mut pixmap = tiny_skia::Pixmap::new(px, px)?;
    let scale = px as f32 / 24.0;
    resvg::render(
        &tree,
        tiny_skia::Transform::from_scale(scale, scale),
        &mut pixmap.as_mut(),
    );
    Some(pixmap.pixels().iter().map(|pixel| pixel.alpha()).collect())
}

#[derive(Clone, Copy, Debug)]
struct Shelf {
    y: u32,
    height: u32,
    x: u32,
}

/// Shelf-packed cache of icon masks. Rasterized masks wait in
/// [`MaskAtlas::take_uploads`] until the backend copies them to the GPU.
#[derive(Debug)]
pub struct MaskAtlas {
    size: u32,
    shelves: Vec<Shelf>,
    next_y: u32,
    entries: HashMap<MaskKey, AtlasRect>,
    uploads: Vec<(AtlasRect, Vec<u8>)>,
}

impl Default for MaskAtlas {
    fn default() -> Self {
        Self::new(ATLAS_SIZE)
    }
}

impl MaskAtlas {
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

    fn allocate(&mut self, size: u32) -> Option<AtlasRect> {
        let padded = size + PADDING * 2;
        if padded > self.size {
            return None;
        }
        let limit = self.size;
        let shelf = self
            .shelves
            .iter_mut()
            .filter(|shelf| shelf.height >= padded && shelf.x + padded <= limit)
            .min_by_key(|shelf| shelf.height);
        let shelf = match shelf {
            Some(shelf) => shelf,
            None => {
                if self.next_y + padded > self.size {
                    return None;
                }
                self.shelves.push(Shelf {
                    y: self.next_y,
                    height: padded,
                    x: 0,
                });
                self.next_y += padded;
                self.shelves.last_mut()?
            }
        };
        let rect = AtlasRect {
            x: shelf.x + PADDING,
            y: shelf.y + PADDING,
            size,
        };
        shelf.x += padded;
        Some(rect)
    }

    fn insert(&mut self, key: MaskKey) -> Option<Option<AtlasRect>> {
        if let Some(rect) = self.entries.get(&key) {
            return Some(Some(*rect));
        }
        let px = u32::from(key.px);
        let Some(rect) = self.allocate(px) else {
            // Too large for any atlas: skip it. Otherwise the atlas is full.
            return (px + PADDING * 2 > self.size).then_some(None);
        };
        let Some(mask) = rasterize(key.icon, px, key.stroke_width()) else {
            return Some(None);
        };
        self.entries.insert(key, rect);
        self.uploads.push((rect, mask));
        Some(Some(rect))
    }

    /// Regions for every key of a frame. When the atlas fills up it is
    /// emptied and repacked with this frame's keys only, so all returned
    /// regions are valid together. Keys that cannot be rasterized are absent.
    pub fn prepare(&mut self, keys: &[MaskKey]) -> HashMap<MaskKey, AtlasRect> {
        for attempt in 0..2 {
            let mut regions = HashMap::new();
            let mut full = false;
            for key in keys {
                match self.insert(*key) {
                    Some(Some(rect)) => {
                        regions.insert(*key, rect);
                    }
                    Some(None) => {}
                    None => {
                        full = true;
                        break;
                    }
                }
            }
            if !full || attempt == 1 {
                return regions;
            }
            self.clear();
        }
        HashMap::new()
    }

    /// Masks rasterized since the last call, to copy into the texture.
    pub fn take_uploads(&mut self) -> Vec<(AtlasRect, Vec<u8>)> {
        std::mem::take(&mut self.uploads)
    }
}

#[cfg(test)]
mod tests {
    use super::{AtlasRect, MaskAtlas, MaskKey, rasterize};
    use crate::icons::{DEFAULT_STROKE_WIDTH, Icon};

    #[test]
    fn every_catalog_icon_parses_and_rasterizes_with_coverage() {
        for icon in Icon::ALL {
            let mask = rasterize(*icon, 24, DEFAULT_STROKE_WIDTH)
                .unwrap_or_else(|| panic!("{} rasterizes", icon.name()));
            assert_eq!(mask.len(), 24 * 24);
            assert!(mask.iter().any(|alpha| *alpha > 200), "{}", icon.name());
            assert!(mask.contains(&0), "{}", icon.name());
            assert_eq!(Icon::from_name(icon.name()), Some(*icon));
        }
        let thin = rasterize(Icon::Minus, 24, 1.0).unwrap();
        let thick = rasterize(Icon::Minus, 24, 3.0).unwrap();
        let ink = |mask: &[u8]| mask.iter().map(|alpha| u32::from(*alpha)).sum::<u32>();
        assert!(
            ink(&thick) > ink(&thin) * 2,
            "stroke width changes coverage"
        );
    }

    #[test]
    fn atlas_packs_caches_and_repacks_when_full() {
        // Room for 2 × 2 masks of 20px plus padding.
        let mut atlas = MaskAtlas::new(48);
        let eye = MaskKey::new(Icon::Eye, 20.0, 2.0);
        let search = MaskKey::new(Icon::Search, 20.0, 2.0);
        let regions = atlas.prepare(&[eye, search, eye]);
        assert_eq!(regions.len(), 2);
        assert_eq!(
            regions[&eye],
            AtlasRect {
                x: 1,
                y: 1,
                size: 20
            }
        );
        assert_eq!(
            regions[&search],
            AtlasRect {
                x: 23,
                y: 1,
                size: 20
            }
        );
        assert_eq!(atlas.take_uploads().len(), 2);

        assert_eq!(atlas.prepare(&[search])[&search], regions[&search]);
        assert!(
            atlas.take_uploads().is_empty(),
            "cached masks are not re-uploaded"
        );

        let others: Vec<MaskKey> = [Icon::Plus, Icon::Minus, Icon::X, Icon::User]
            .into_iter()
            .map(|icon| MaskKey::new(icon, 20.0, 2.0))
            .collect();
        let regions = atlas.prepare(&others);
        assert_eq!(regions.len(), 4, "atlas was emptied and repacked");
        assert_eq!(atlas.len(), 4);
        assert_eq!(atlas.take_uploads().len(), 4);

        let huge = MaskKey::new(Icon::Eye, 100.0, 2.0);
        assert!(atlas.prepare(&[huge, eye]).contains_key(&eye));
        assert!(!atlas.is_empty());
    }
}
