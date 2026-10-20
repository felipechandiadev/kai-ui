//! Which tiles a map draws and their loading: [`TileStore`] asks for the
//! tiles a camera sees (center first), cancels those that leave the view,
//! shows an ancestor while a tile loads, keeps meshes under a byte budget
//! (least recently used go first) and reveals at most
//! [`TileStore::PER_FRAME`] new tiles per frame, so GPU uploads spread
//! over frames. Loading itself (read, gunzip, MVT, lyon) runs elsewhere:
//! [`TileStore::frame`] hands out [`TileLoad`] jobs for the task pool.

use std::any::Any;
use std::collections::{HashMap, HashSet, VecDeque};
use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use super::mvt::{Bucket, TileMeshes, tessellate};
use super::pmtiles::PmTiles;
use crate::geo::{Camera, TileId};

/// Where a map's tiles come from: a PMTiles archive of vector tiles, and
/// the buckets its style draws.
#[derive(Clone)]
pub struct TileSource {
    path: Arc<Path>,
    buckets: Arc<[Bucket]>,
}

impl TileSource {
    pub fn pmtiles(path: impl Into<PathBuf>, buckets: Arc<[Bucket]>) -> Self {
        Self {
            path: Arc::from(path.into()),
            buckets,
        }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn buckets(&self) -> &[Bucket] {
        &self.buckets
    }

    fn key(&self) -> SourceKey {
        SourceKey(
            Arc::clone(&self.path),
            Arc::as_ptr(&self.buckets).cast::<Bucket>() as usize,
        )
    }
}

impl PartialEq for TileSource {
    fn eq(&self, other: &Self) -> bool {
        self.key() == other.key()
    }
}

impl Eq for TileSource {}

impl fmt::Debug for TileSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "TileSource({})", self.path.display())
    }
}

/// An archive and a style: tiles of one differ from another's.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct SourceKey(Arc<Path>, usize);

/// The data zoom that serves view zoom `zoom`: tiles show about 512
/// points wide (as MapLibre does with these tiles), and past the
/// archive's `max_zoom` the deepest tiles are scaled up.
pub fn data_zoom(zoom: f64, max_zoom: u8) -> u8 {
    (zoom.round() - 1.0).clamp(0.0, f64::from(max_zoom)) as u8
}

/// A tile to draw: the data of `tile`, only inside `clip` when it stands
/// in for a tile still loading.
#[derive(Clone, Debug)]
pub struct PlacedTile {
    pub tile: TileId,
    pub clip: Option<TileId>,
    pub meshes: Arc<TileMeshes>,
}

impl PartialEq for PlacedTile {
    fn eq(&self, other: &Self) -> bool {
        self.tile == other.tile
            && self.clip == other.clip
            && Arc::ptr_eq(&self.meshes, &other.meshes)
    }
}

impl Eq for PlacedTile {}

/// What a map draws this frame: stand-ins first, then loaded tiles.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TileFrame {
    pub tiles: Vec<PlacedTile>,
    /// Some tile is loading or waits to be revealed: more frames to come.
    pub pending: bool,
    /// Loaded tiles wait to be revealed: the next frame shows more.
    pub revealing: bool,
}

/// Work for the task pool: load `tile` of `source`.
pub struct TileLoad {
    pub source: SourceKey,
    pub tile: TileId,
    pub run: Box<dyn FnOnce() -> Box<dyn Any + Send> + Send>,
}

/// What a [`TileLoad`] yields: meshes, nothing (no tile there) or why it
/// failed.
pub type TileResult = Result<Option<TileMeshes>, String>;

enum Slot {
    Loading(Arc<AtomicBool>),
    Ready {
        meshes: Arc<TileMeshes>,
        used: u64,
        shown: bool,
    },
    /// The archive has nothing here, or it failed.
    Empty,
}

struct Archive {
    reader: Result<Arc<PmTiles>, String>,
    buckets: Arc<[Bucket]>,
    tiles: HashMap<TileId, Slot>,
    /// Loaded, not yet shown, oldest first.
    fresh: VecDeque<TileId>,
    bytes: usize,
}

impl Archive {
    fn open(source: &TileSource) -> Self {
        Self {
            reader: PmTiles::open(&source.path)
                .map(Arc::new)
                .map_err(|error| format!("{}: {error}", source.path.display())),
            buckets: Arc::clone(&source.buckets),
            tiles: HashMap::new(),
            fresh: VecDeque::new(),
            bytes: 0,
        }
    }

    fn shown(&self, tile: TileId) -> Option<&Arc<TileMeshes>> {
        match self.tiles.get(&tile) {
            Some(Slot::Ready {
                meshes,
                shown: true,
                ..
            }) => Some(meshes),
            _ => None,
        }
    }

    fn touch(&mut self, tile: TileId, now: u64) {
        if let Some(Slot::Ready { used, .. }) = self.tiles.get_mut(&tile) {
            *used = now;
        }
    }
}

/// Tiles of every map, by source.
pub struct TileStore {
    archives: HashMap<SourceKey, Archive>,
    budget: usize,
    tick: u64,
}

impl Default for TileStore {
    fn default() -> Self {
        Self::new(Self::BUDGET)
    }
}

impl TileStore {
    /// Mesh bytes kept by default.
    pub const BUDGET: usize = 96 << 20;
    /// Newly loaded tiles revealed per frame.
    pub const PER_FRAME: usize = 6;

    pub fn new(budget: usize) -> Self {
        Self {
            archives: HashMap::new(),
            budget,
            tick: 0,
        }
    }

    /// Why `source` cannot be read, if it cannot.
    pub fn error(&self, source: &TileSource) -> Option<&str> {
        self.archives
            .get(&source.key())
            .and_then(|archive| archive.reader.as_ref().err())
            .map(String::as_str)
    }

    /// Mesh bytes held for `source`.
    pub fn bytes(&self, source: &TileSource) -> usize {
        self.archives
            .get(&source.key())
            .map_or(0, |archive| archive.bytes)
    }

    /// Tiles of `source` loading now.
    pub fn loading(&self, source: &TileSource) -> Vec<TileId> {
        let mut tiles: Vec<TileId> = self
            .archives
            .get(&source.key())
            .map(|archive| {
                archive
                    .tiles
                    .iter()
                    .filter(|(_, slot)| matches!(slot, Slot::Loading(_)))
                    .map(|(tile, _)| *tile)
                    .collect()
            })
            .unwrap_or_default();
        tiles.sort();
        tiles
    }

    /// What a map of `source` seen by `camera` draws now, and the loads
    /// to start (center first). Loads of tiles out of view are cancelled.
    pub fn frame(&mut self, source: &TileSource, camera: Camera) -> (TileFrame, Vec<TileLoad>) {
        self.tick += 1;
        let now = self.tick;
        let key = source.key();
        let archive = self
            .archives
            .entry(key.clone())
            .or_insert_with(|| Archive::open(source));
        let Ok(reader) = archive.reader.clone() else {
            return (TileFrame::default(), Vec::new());
        };
        if !(camera.size.width > 0.0 && camera.size.height > 0.0) {
            return (TileFrame::default(), Vec::new());
        }
        let header = reader.header();
        let z = data_zoom(camera.view.zoom, header.max_zoom).max(header.min_zoom);
        let wanted = camera.visible_tiles(z);

        for _ in 0..Self::PER_FRAME {
            let Some(tile) = archive.fresh.pop_front() else {
                break;
            };
            if let Some(Slot::Ready { shown, .. }) = archive.tiles.get_mut(&tile) {
                *shown = true;
            }
        }

        let mut loads = Vec::new();
        let mut exact = Vec::new();
        let mut standins = Vec::new();
        let mut pending = !archive.fresh.is_empty();
        let mut asked = HashSet::new();
        for &tile in &wanted {
            if let Some(meshes) = archive.shown(tile) {
                exact.push(PlacedTile {
                    tile,
                    clip: None,
                    meshes: Arc::clone(meshes),
                });
                archive.touch(tile, now);
                continue;
            }
            // Up from the tile: the first one not known to be empty is
            // the data to wait for (outside the archive's area that is an
            // ancestor), and the first one shown stands in meanwhile.
            let mut waiting = false;
            let mut current = Some(tile);
            while let Some(at) = current {
                if at != tile
                    && let Some(meshes) = archive.shown(at)
                {
                    standins.push(PlacedTile {
                        tile: at,
                        clip: Some(tile),
                        meshes: Arc::clone(meshes),
                    });
                    archive.touch(at, now);
                    break;
                }
                if !waiting {
                    match archive.tiles.get(&at) {
                        None => {
                            let cancel = Arc::new(AtomicBool::new(false));
                            archive.tiles.insert(at, Slot::Loading(Arc::clone(&cancel)));
                            loads.push(load(&key, at, &reader, &archive.buckets, cancel));
                            asked.insert(at);
                            waiting = true;
                        }
                        Some(Slot::Loading(_)) => {
                            asked.insert(at);
                            waiting = true;
                        }
                        Some(Slot::Ready { .. }) => waiting = true,
                        Some(Slot::Empty) => {}
                    }
                    pending |= waiting;
                }
                current = at.parent();
            }
        }
        standins.sort_by_key(|placed| placed.tile.z);
        standins.dedup_by(|a, b| a.tile == b.tile && a.clip == b.clip);

        archive.tiles.retain(|tile, slot| match slot {
            Slot::Loading(cancel) if !asked.contains(tile) => {
                cancel.store(true, Ordering::Relaxed);
                false
            }
            _ => true,
        });

        evict(archive, self.budget, now);
        standins.extend(exact);
        (
            TileFrame {
                tiles: standins,
                pending,
                revealing: !archive.fresh.is_empty(),
            },
            loads,
        )
    }

    /// A load landed. Results of cancelled loads are dropped.
    pub fn finish(&mut self, source: &SourceKey, tile: TileId, result: TileResult) {
        let Some(archive) = self.archives.get_mut(source) else {
            return;
        };
        if !matches!(archive.tiles.get(&tile), Some(Slot::Loading(_))) {
            return;
        }
        let slot = match result {
            Ok(Some(meshes)) => {
                archive.bytes += meshes.byte_size();
                archive.fresh.push_back(tile);
                Slot::Ready {
                    meshes: Arc::new(meshes),
                    used: self.tick,
                    shown: false,
                }
            }
            Ok(None) => Slot::Empty,
            Err(error) => {
                eprintln!("No se pudo cargar la tesela {tile:?}: {error}");
                Slot::Empty
            }
        };
        archive.tiles.insert(tile, slot);
    }
}

/// Drop the least recently used tiles until `archive` fits `budget`;
/// tiles used in frame `now` stay.
fn evict(archive: &mut Archive, budget: usize, now: u64) {
    if archive.bytes <= budget {
        return;
    }
    let mut ready: Vec<(u64, TileId, usize)> = archive
        .tiles
        .iter()
        .filter_map(|(tile, slot)| match slot {
            Slot::Ready { used, meshes, .. } if *used < now => {
                Some((*used, *tile, meshes.byte_size()))
            }
            _ => None,
        })
        .collect();
    ready.sort();
    for (_, tile, bytes) in ready {
        if archive.bytes <= budget {
            break;
        }
        archive.tiles.remove(&tile);
        archive.fresh.retain(|fresh| *fresh != tile);
        archive.bytes -= bytes;
    }
}

fn load(
    source: &SourceKey,
    tile: TileId,
    reader: &Arc<PmTiles>,
    buckets: &Arc<[Bucket]>,
    cancel: Arc<AtomicBool>,
) -> TileLoad {
    let reader = Arc::clone(reader);
    let buckets = Arc::clone(buckets);
    TileLoad {
        source: source.clone(),
        tile,
        run: Box::new(move || {
            let result: TileResult = if cancel.load(Ordering::Relaxed) {
                Err("cancelada".to_owned())
            } else {
                reader
                    .tile(tile)
                    .map_err(|error| error.to_string())
                    .and_then(|bytes| {
                        bytes
                            .map(|bytes| tessellate(bytes, tile.z, &buckets))
                            .transpose()
                    })
            };
            Box::new(result) as Box<dyn Any + Send>
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geo::{LatLon, MapView};
    use crate::geometry::Size;
    use crate::maps::mvt::encode::{Feature, Geom, square, tile};
    use crate::maps::pmtiles::write_archive;

    const PLAZA: LatLon = LatLon::new(-36.1415, -71.8265);
    const SIZE: Size = Size {
        width: 600.0,
        height: 400.0,
    };
    const BUCKETS: [Bucket; 1] = [Bucket::fill("earth", &[])];

    fn land() -> Vec<u8> {
        tile(&[(
            "earth",
            vec![Feature {
                kind: "earth",
                name: None,
                geom: Geom::Polygon(vec![square(0, 0, 4096)]),
            }],
        )])
    }

    /// An archive with land in every tile around the plaza from zoom 10 to
    /// 13, and only the zoom 0 tile elsewhere.
    fn source(name: &str) -> TileSource {
        let mut tiles = vec![(TileId::new(0, 0, 0), land())];
        for z in 10..=13 {
            let camera = Camera::new(MapView::new(PLAZA, f64::from(z) + 1.0), SIZE);
            for tile in camera.visible_tiles(z) {
                for dx in 0..3 {
                    tiles.push((TileId::new(z, tile.x + dx, tile.y), land()));
                }
            }
        }
        tiles.sort_by_key(|(tile, _)| *tile);
        tiles.dedup_by_key(|(tile, _)| *tile);
        let path =
            std::env::temp_dir().join(format!("kai-tiles-{name}-{}.pmtiles", std::process::id()));
        std::fs::write(&path, write_archive(&tiles, 13)).unwrap();
        TileSource::pmtiles(path, Arc::from(BUCKETS.as_slice()))
    }

    fn camera(zoom: f64) -> Camera {
        Camera::new(MapView::new(PLAZA, zoom), SIZE)
    }

    fn run(store: &mut TileStore, loads: Vec<TileLoad>) {
        for load in loads {
            let result = *(load.run)().downcast::<TileResult>().unwrap();
            store.finish(&load.source, load.tile, result);
        }
    }

    /// Frames until nothing is pending; returns the last one.
    fn settle(store: &mut TileStore, source: &TileSource, camera: Camera) -> TileFrame {
        for _ in 0..20 {
            let (frame, loads) = store.frame(source, camera);
            if !frame.pending {
                return frame;
            }
            run(store, loads);
        }
        panic!("las teselas no terminaron de cargar");
    }

    #[test]
    fn data_zoom_shows_tiles_about_512_points_wide_and_overzooms() {
        assert_eq!(data_zoom(14.0, 15), 13);
        assert_eq!(data_zoom(14.6, 15), 14);
        assert_eq!(data_zoom(0.2, 15), 0);
        assert_eq!(data_zoom(19.0, 15), 15);
    }

    #[test]
    fn visible_tiles_load_center_first_and_then_draw() {
        let source = source("centro");
        let mut store = TileStore::default();
        let view = camera(14.0);
        let (frame, loads) = store.frame(&source, view);
        assert!(frame.tiles.is_empty() && frame.pending);
        let wanted = view.visible_tiles(13);
        let asked: Vec<TileId> = loads.iter().map(|load| load.tile).collect();
        assert_eq!(asked, wanted, "center first");
        let (_, again) = store.frame(&source, view);
        assert!(again.is_empty(), "a tile loads once");

        run(&mut store, loads);
        let frame = settle(&mut store, &source, view);
        let drawn: Vec<TileId> = frame.tiles.iter().map(|placed| placed.tile).collect();
        assert_eq!(drawn, wanted);
        assert!(frame.tiles.iter().all(|placed| placed.clip.is_none()));
        assert!(store.bytes(&source) > 0);
    }

    #[test]
    fn new_tiles_are_revealed_a_few_per_frame() {
        let source = source("revelar");
        let mut store = TileStore::default();
        let view = Camera::new(
            MapView::new(PLAZA, 14.0),
            Size {
                width: 1600.0,
                height: 1200.0,
            },
        );
        let (_, loads) = store.frame(&source, view);
        let count = loads.len();
        assert!(count > TileStore::PER_FRAME, "{count} tiles");
        run(&mut store, loads);
        let (frame, _) = store.frame(&source, view);
        assert_eq!(frame.tiles.len(), TileStore::PER_FRAME);
        assert!(frame.pending && frame.revealing);
    }

    #[test]
    fn an_ancestor_stands_in_while_a_tile_loads() {
        let source = source("ancestro");
        let mut store = TileStore::default();
        settle(&mut store, &source, camera(12.0));
        let (frame, loads) = store.frame(&source, camera(14.0));
        assert!(!loads.is_empty());
        assert!(frame.pending);
        assert!(!frame.tiles.is_empty());
        for placed in &frame.tiles {
            assert_eq!(placed.tile.z, 11);
            let clip = placed.clip.expect("a stand-in is clipped to its tile");
            assert_eq!(clip.z, 13);
            assert_eq!(clip.ancestor(11), Some(placed.tile));
        }
    }

    #[test]
    fn past_the_deepest_zoom_tiles_scale_up_and_outside_the_data_the_root_shows() {
        let source = source("sobrezoom");
        let mut store = TileStore::default();
        let frame = settle(&mut store, &source, camera(18.0));
        assert!(frame.tiles.iter().all(|placed| placed.tile.z == 13));
        assert!(!frame.tiles.is_empty());

        let far = Camera::new(MapView::new(LatLon::new(40.0, 10.0), 14.0), SIZE);
        let frame = settle(&mut store, &source, far);
        assert!(!frame.tiles.is_empty());
        assert!(
            frame
                .tiles
                .iter()
                .all(|placed| placed.tile == TileId::new(0, 0, 0) && placed.clip.is_some())
        );
    }

    #[test]
    fn tiles_that_leave_the_view_stop_loading() {
        let source = source("cancelar");
        let mut store = TileStore::default();
        let (_, loads) = store.frame(&source, camera(14.0));
        assert!(!store.loading(&source).is_empty());
        let (_, far_loads) = store.frame(&source, camera(11.0));
        let far: Vec<TileId> = far_loads.iter().map(|load| load.tile).collect();
        assert_eq!(store.loading(&source), {
            let mut far = far.clone();
            far.sort();
            far
        });
        // The cancelled loads skip their work and land for nothing.
        for load in loads {
            let result = *(load.run)().downcast::<TileResult>().unwrap();
            assert_eq!(result.as_ref().err().map(String::as_str), Some("cancelada"));
            store.finish(&load.source, load.tile, result);
        }
        assert_eq!(store.loading(&source).len(), far.len());
    }

    #[test]
    fn the_least_recently_used_tiles_leave_over_budget() {
        let source = source("lru");
        let mut store = TileStore::new(usize::MAX);
        settle(&mut store, &source, camera(12.0));
        let one_zoom = store.bytes(&source);
        store.budget = one_zoom;
        settle(&mut store, &source, camera(14.0));
        assert!(store.bytes(&source) <= one_zoom.max(1) * 2);
        // The zoom 11 tiles went; those of zoom 13 stayed.
        let (frame, loads) = store.frame(&source, camera(14.0));
        assert!(loads.is_empty());
        assert!(frame.tiles.iter().all(|placed| placed.tile.z == 13));
        let (_, reloads) = store.frame(&source, camera(12.0));
        assert!(!reloads.is_empty(), "evicted tiles load again");
    }

    #[test]
    fn a_missing_archive_draws_nothing_and_says_why() {
        let source = TileSource::pmtiles("/no/existe.pmtiles", Arc::from(BUCKETS.as_slice()));
        let mut store = TileStore::default();
        let (frame, loads) = store.frame(&source, camera(14.0));
        assert_eq!(frame, TileFrame::default());
        assert!(loads.is_empty());
        assert!(
            store
                .error(&source)
                .is_some_and(|error| error.contains("existe"))
        );
    }
}
