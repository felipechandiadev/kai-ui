//! Images of the element tree ([`image`](crate::element::image)): where
//! they come from ([`ImageSource`]), how their bytes are read
//! ([`ImageLoader`]: files, and URLs with the `net` feature) and the
//! session's cache of decoded ones ([`ImageStore`]).
//!
//! The view only names a source. The session reads and decodes it on the
//! task pool ([`Effects::perform`](crate::runtime::Effects::perform)'s),
//! scaled down to [`MAX_IMAGE_SIDE`], and fills it into the tree once it is
//! ready; until then, or when it fails, the image draws nothing, so a
//! component puts its placeholder underneath. The app keeps no loading
//! state. Tests read from a [`FakeImageLoader`], never from the network.

use std::collections::HashMap;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use crate::element::{Element, Kind};
pub use crate::render::images::{Bitmap, MAX_IMAGE_SIDE};

/// Where an image comes from. Two sources are the same image when they
/// name the same file or URL, share the same bytes (the same `Arc`) or the
/// same [`Bitmap`].
#[derive(Clone)]
pub enum ImageSource {
    /// A file, read on the task pool.
    Path(PathBuf),
    /// `http(s)://…`, fetched on the task pool (feature `net`).
    Url(String),
    /// Encoded bytes (PNG, JPEG or WebP) already in memory.
    Bytes(Arc<[u8]>),
    /// Pixels made by the app (straight-alpha RGBA8); ready at once.
    Rgba(Bitmap),
}

impl ImageSource {
    pub fn path(path: impl Into<PathBuf>) -> Self {
        Self::Path(path.into())
    }

    pub fn url(url: impl Into<String>) -> Self {
        Self::Url(url.into())
    }

    pub fn bytes(bytes: impl Into<Arc<[u8]>>) -> Self {
        Self::Bytes(bytes.into())
    }

    /// `width × height` straight-alpha RGBA8 pixels, row by row from the
    /// top.
    pub fn rgba(width: u32, height: u32, pixels: impl Into<Arc<[u8]>>) -> Self {
        Self::Rgba(Bitmap::new(width, height, pixels))
    }
}

impl From<PathBuf> for ImageSource {
    fn from(path: PathBuf) -> Self {
        Self::Path(path)
    }
}

impl From<&Path> for ImageSource {
    fn from(path: &Path) -> Self {
        Self::Path(path.to_path_buf())
    }
}

impl From<Bitmap> for ImageSource {
    fn from(bitmap: Bitmap) -> Self {
        Self::Rgba(bitmap)
    }
}

impl PartialEq for ImageSource {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Path(a), Self::Path(b)) => a == b,
            (Self::Url(a), Self::Url(b)) => a == b,
            (Self::Bytes(a), Self::Bytes(b)) => Arc::ptr_eq(a, b),
            (Self::Rgba(a), Self::Rgba(b)) => a == b,
            _ => false,
        }
    }
}

impl Eq for ImageSource {}

impl Hash for ImageSource {
    fn hash<H: Hasher>(&self, state: &mut H) {
        std::mem::discriminant(self).hash(state);
        match self {
            Self::Path(path) => path.hash(state),
            Self::Url(url) => url.hash(state),
            Self::Bytes(bytes) => (bytes.as_ptr() as usize).hash(state),
            Self::Rgba(bitmap) => bitmap.id().hash(state),
        }
    }
}

impl fmt::Debug for ImageSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Path(path) => write!(f, "Path({})", path.display()),
            Self::Url(url) => write!(f, "Url({url})"),
            Self::Bytes(bytes) => write!(f, "Bytes({} bytes)", bytes.len()),
            Self::Rgba(bitmap) => write!(f, "Rgba({bitmap:?})"),
        }
    }
}

/// A decoded image: its pixels (at most [`MAX_IMAGE_SIDE`] on the longest
/// side) and the size of the original, which layout uses as the image's
/// natural size.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LoadedImage {
    pub bitmap: Bitmap,
    pub width: u32,
    pub height: u32,
}

/// Reads the bytes of a [`ImageSource::Path`] or [`ImageSource::Url`]. It
/// runs on the task pool, so it may block.
pub trait ImageLoader: Send + Sync {
    fn read(&self, source: &ImageSource) -> Result<Vec<u8>, String>;
}

/// Largest download [`SystemLoader`] accepts.
pub const MAX_DOWNLOAD: u64 = 20 * 1024 * 1024;

/// Files from disk; URLs with `ureq` (feature `net`), 20 s and
/// [`MAX_DOWNLOAD`] at most.
#[derive(Clone, Copy, Debug, Default)]
pub struct SystemLoader;

impl ImageLoader for SystemLoader {
    fn read(&self, source: &ImageSource) -> Result<Vec<u8>, String> {
        match source {
            ImageSource::Path(path) => read_file(path),
            ImageSource::Url(url) => fetch(url),
            ImageSource::Bytes(bytes) => Ok(bytes.to_vec()),
            ImageSource::Rgba(_) => Err("una imagen RGBA no se lee".to_owned()),
        }
    }
}

fn read_file(path: &Path) -> Result<Vec<u8>, String> {
    std::fs::read(path).map_err(|error| format!("No se pudo leer {}: {error}", path.display()))
}

#[cfg(feature = "net")]
fn fetch(url: &str) -> Result<Vec<u8>, String> {
    use std::time::Duration;
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(20)))
        .user_agent(concat!("kai-ui-desktop/", env!("CARGO_PKG_VERSION")))
        .build()
        .into();
    let mut response = agent
        .get(url)
        .call()
        .map_err(|error| format!("No se pudo descargar {url}: {error}"))?;
    response
        .body_mut()
        .with_config()
        .limit(MAX_DOWNLOAD)
        .read_to_vec()
        .map_err(|error| format!("No se pudo descargar {url}: {error}"))
}

#[cfg(not(feature = "net"))]
fn fetch(url: &str) -> Result<Vec<u8>, String> {
    Err(format!("Sin red (feature `net`): {url}"))
}

type Answers = HashMap<String, Result<Vec<u8>, String>>;

/// Loader for tests: answers from a table of sources, reads files that are
/// not in it from disk and fails every other URL, without a network. Clones
/// share the table and the count of reads.
#[derive(Clone, Default)]
pub struct FakeImageLoader {
    answers: Arc<Mutex<Answers>>,
    reads: Arc<AtomicUsize>,
}

impl FakeImageLoader {
    pub fn new() -> Self {
        Self::default()
    }

    fn key(source: &ImageSource) -> Option<String> {
        match source {
            ImageSource::Path(path) => Some(path.display().to_string()),
            ImageSource::Url(url) => Some(url.clone()),
            _ => None,
        }
    }

    fn answers(&self) -> std::sync::MutexGuard<'_, Answers> {
        self.answers
            .lock()
            .unwrap_or_else(|poison| poison.into_inner())
    }

    /// Answer `url_or_path` with `bytes`.
    pub fn with(self, url_or_path: impl Into<String>, bytes: impl Into<Vec<u8>>) -> Self {
        self.answers().insert(url_or_path.into(), Ok(bytes.into()));
        self
    }

    /// Answer `url_or_path` with `error`.
    pub fn failing(self, url_or_path: impl Into<String>, error: impl Into<String>) -> Self {
        self.answers().insert(url_or_path.into(), Err(error.into()));
        self
    }

    /// Paths and URLs read so far.
    pub fn reads(&self) -> usize {
        self.reads.load(Ordering::SeqCst)
    }
}

impl ImageLoader for FakeImageLoader {
    fn read(&self, source: &ImageSource) -> Result<Vec<u8>, String> {
        self.reads.fetch_add(1, Ordering::SeqCst);
        let key = Self::key(source);
        if let Some(answer) = key
            .as_ref()
            .and_then(|key| self.answers().get(key).cloned())
        {
            return answer;
        }
        match source {
            ImageSource::Path(path) => read_file(path),
            ImageSource::Url(url) => Err(format!("Sin respuesta de prueba para {url}")),
            ImageSource::Bytes(bytes) => Ok(bytes.to_vec()),
            ImageSource::Rgba(_) => Err("una imagen RGBA no se lee".to_owned()),
        }
    }
}

/// Decode PNG, JPEG or WebP bytes, scaled down to [`MAX_IMAGE_SIDE`].
pub fn decode(bytes: &[u8]) -> Result<LoadedImage, String> {
    let image = image::load_from_memory(bytes)
        .map_err(|error| format!("No se pudo decodificar la imagen: {error}"))?;
    let (width, height) = (image.width(), image.height());
    let image = if width.max(height) > MAX_IMAGE_SIDE {
        image.thumbnail(MAX_IMAGE_SIDE, MAX_IMAGE_SIDE)
    } else {
        image
    };
    let rgba = image.into_rgba8();
    let (scaled_width, scaled_height) = rgba.dimensions();
    Ok(LoadedImage {
        bitmap: Bitmap::new(scaled_width, scaled_height, rgba.into_raw()),
        width,
        height,
    })
}

/// Read (with `loader`) and decode `source`; this is the task the session
/// runs on the pool.
pub fn load(loader: &dyn ImageLoader, source: &ImageSource) -> Result<LoadedImage, String> {
    match source {
        ImageSource::Rgba(bitmap) => Ok(ready(bitmap)),
        ImageSource::Bytes(bytes) => decode(bytes),
        _ => decode(&loader.read(source)?),
    }
}

fn ready(bitmap: &Bitmap) -> LoadedImage {
    LoadedImage {
        bitmap: bitmap.clone(),
        width: bitmap.width(),
        height: bitmap.height(),
    }
}

/// Where an image of the [`ImageStore`] is.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ImageState {
    /// Seen in a view; its load has not started.
    Wanted,
    Loading,
    Ready(LoadedImage),
    Failed(String),
}

/// Entries kept before the ones not seen lately are dropped.
const STORE_CAPACITY: usize = 256;

struct Entry {
    state: ImageState,
    /// [`ImageStore::resolve`] call that last saw this source.
    seen: u64,
}

/// The images of the views so far, by source: wanted, loading, decoded or
/// failed. [`ImageStore::resolve`] fills the decoded ones into a tree; the
/// session starts the loads ([`ImageStore::take_wanted`]) and stores the
/// results ([`ImageStore::finish`]). Beyond `STORE_CAPACITY` entries,
/// the ones out of the views longest are dropped (and load again if they
/// come back).
pub struct ImageStore {
    loader: Arc<dyn ImageLoader>,
    entries: HashMap<ImageSource, Entry>,
    generation: u64,
}

impl Default for ImageStore {
    fn default() -> Self {
        Self::new(Arc::new(SystemLoader))
    }
}

impl ImageStore {
    pub fn new(loader: Arc<dyn ImageLoader>) -> Self {
        Self {
            loader,
            entries: HashMap::new(),
            generation: 0,
        }
    }

    pub fn loader(&self) -> Arc<dyn ImageLoader> {
        Arc::clone(&self.loader)
    }

    /// Read new images with `loader` from now on.
    pub fn set_loader(&mut self, loader: Arc<dyn ImageLoader>) {
        self.loader = loader;
    }

    pub fn state(&self, source: &ImageSource) -> Option<&ImageState> {
        self.entries.get(source).map(|entry| &entry.state)
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Fill every decoded image of `tree` in, and note the sources it has
    /// not seen yet as wanted ([`ImageSource::Rgba`] is ready at once).
    /// Returns whether some image is wanted.
    pub fn resolve<Msg>(&mut self, tree: &mut Element<Msg>) -> bool {
        self.generation += 1;
        let generation = self.generation;
        let entries = &mut self.entries;
        let mut wanted = false;
        tree.walk_mut(&mut |element| {
            if let Kind::Image { source, loaded } = &mut element.kind {
                let entry = entries.entry(source.clone()).or_insert_with(|| Entry {
                    state: match source {
                        ImageSource::Rgba(bitmap) => ImageState::Ready(ready(bitmap)),
                        _ => ImageState::Wanted,
                    },
                    seen: generation,
                });
                entry.seen = generation;
                wanted |= entry.state == ImageState::Wanted;
                *loaded = match &entry.state {
                    ImageState::Ready(image) => Some(image.clone()),
                    _ => None,
                };
            }
        });
        self.evict();
        wanted
    }

    fn evict(&mut self) {
        if self.entries.len() <= STORE_CAPACITY {
            return;
        }
        let mut idle: Vec<(u64, ImageSource)> = self
            .entries
            .iter()
            .filter(|(_, entry)| entry.seen < self.generation && entry.state != ImageState::Loading)
            .map(|(source, entry)| (entry.seen, source.clone()))
            .collect();
        idle.sort_by_key(|(seen, _)| *seen);
        let excess = self.entries.len() - STORE_CAPACITY;
        for (_, source) in idle.into_iter().take(excess) {
            self.entries.remove(&source);
        }
    }

    /// The wanted sources, now loading.
    pub fn take_wanted(&mut self) -> Vec<ImageSource> {
        let mut wanted = Vec::new();
        for (source, entry) in &mut self.entries {
            if entry.state == ImageState::Wanted {
                entry.state = ImageState::Loading;
                wanted.push(source.clone());
            }
        }
        wanted
    }

    /// Store the result of loading `source`.
    pub fn finish(&mut self, source: &ImageSource, result: Result<LoadedImage, String>) {
        if let Some(entry) = self.entries.get_mut(source) {
            entry.state = match result {
                Ok(image) => ImageState::Ready(image),
                Err(error) => {
                    eprintln!("Imagen {source:?}: {error}");
                    ImageState::Failed(error)
                }
            };
        }
    }

    /// Load every image of `tree` here and now and fill them in: for
    /// captures and tools, which have no task pool to wait for.
    pub fn load_now<Msg>(&mut self, tree: &mut Element<Msg>) {
        if self.resolve(tree) {
            for source in self.take_wanted() {
                let result = load(self.loader.as_ref(), &source);
                self.finish(&source, result);
            }
            self.resolve(tree);
        }
    }
}

/// `width × height` opaque pixels from `color(x, y)` (fractions 0..1).
fn paint(width: u32, height: u32, color: impl Fn(f32, f32) -> [f32; 3]) -> Bitmap {
    let mut pixels = Vec::with_capacity(width as usize * height as usize * 4);
    for y in 0..height {
        for x in 0..width {
            let rgb = color(
                (x as f32 + 0.5) / width as f32,
                (y as f32 + 0.5) / height as f32,
            );
            pixels.extend(rgb.map(|channel| (channel.clamp(0.0, 1.0) * 255.0).round() as u8));
            pixels.push(255);
        }
    }
    Bitmap::new(width, height, pixels)
}

fn mix(a: [f32; 3], b: [f32; 3], t: f32) -> [f32; 3] {
    [0, 1, 2].map(|i| a[i] + (b[i] - a[i]) * t.clamp(0.0, 1.0))
}

/// A drawn portrait (head and shoulders on a gradient), made once: for
/// the demo and the gallery, without files or network.
#[doc(hidden)]
pub fn sample_portrait() -> ImageSource {
    static PORTRAIT: std::sync::OnceLock<Bitmap> = std::sync::OnceLock::new();
    let bitmap = PORTRAIT.get_or_init(|| {
        paint(160, 160, |x, y| {
            let background = mix([0.11, 0.45, 0.68], [0.04, 0.24, 0.36], y);
            let skin = [0.96, 0.80, 0.66];
            let head = ((x - 0.5).powi(2) + (y - 0.40).powi(2)).sqrt() < 0.18;
            let shoulders = ((x - 0.5) / 0.36).powi(2) + ((y - 1.0) / 0.34).powi(2) < 1.0;
            if head {
                skin
            } else if shoulders {
                [0.93, 0.95, 0.97]
            } else {
                background
            }
        })
    });
    ImageSource::Rgba(bitmap.clone())
}

/// A drawn 16:9 landscape (sky, sun and hills), made once.
#[doc(hidden)]
pub fn sample_landscape() -> ImageSource {
    static LANDSCAPE: std::sync::OnceLock<Bitmap> = std::sync::OnceLock::new();
    let bitmap = LANDSCAPE.get_or_init(|| {
        paint(480, 270, |x, y| {
            let sun = ((x - 0.74) / 0.5625).hypot(y - 0.3) < 0.09;
            let far = 0.62 + 0.08 * (x * 7.0).sin();
            let near = 0.78 + 0.06 * (x * 4.0 + 1.0).cos();
            if y > near {
                mix([0.30, 0.55, 0.25], [0.18, 0.38, 0.16], (y - near) * 4.0)
            } else if y > far {
                [0.42, 0.62, 0.52]
            } else if sun {
                [1.0, 0.86, 0.45]
            } else {
                mix([0.55, 0.78, 0.95], [0.95, 0.85, 0.75], y / far)
            }
        })
    });
    ImageSource::Rgba(bitmap.clone())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::element::{div, image};

    /// A `width × height` PNG of one color.
    pub(crate) fn png(width: u32, height: u32, rgba: [u8; 4]) -> Vec<u8> {
        let mut out = std::io::Cursor::new(Vec::new());
        image::RgbaImage::from_pixel(width, height, image::Rgba(rgba))
            .write_to(&mut out, image::ImageFormat::Png)
            .unwrap();
        out.into_inner()
    }

    #[test]
    fn sources_are_equal_by_name_or_by_shared_bytes() {
        let bytes: Arc<[u8]> = Arc::from(vec![1u8, 2, 3]);
        assert_eq!(ImageSource::path("/a.png"), Path::new("/a.png").into());
        assert_ne!(ImageSource::path("/a.png"), ImageSource::url("/a.png"));
        assert_eq!(
            ImageSource::Bytes(Arc::clone(&bytes)),
            ImageSource::Bytes(bytes)
        );
        assert_ne!(
            ImageSource::bytes(vec![1u8, 2, 3]),
            ImageSource::bytes(vec![1u8, 2, 3]),
            "same bytes, not the same image"
        );
        assert_eq!(
            format!("{:?}", ImageSource::bytes(vec![0u8; 4])),
            "Bytes(4 bytes)"
        );
    }

    #[test]
    fn sample_images_are_made_once() {
        assert_eq!(sample_portrait(), sample_portrait());
        assert_ne!(sample_portrait(), sample_landscape());
        let ImageSource::Rgba(landscape) = sample_landscape() else {
            unreachable!()
        };
        assert_eq!((landscape.width(), landscape.height()), (480, 270));
    }

    #[test]
    fn decoding_keeps_the_natural_size_and_scales_large_images_down() {
        let small = decode(&png(3, 2, [10, 20, 30, 255])).unwrap();
        assert_eq!((small.width, small.height), (3, 2));
        assert_eq!(small.bitmap.pixels()[..4], [10, 20, 30, 255]);
        let large = decode(&png(2048, 512, [0, 0, 0, 255])).unwrap();
        assert_eq!((large.width, large.height), (2048, 512));
        assert_eq!((large.bitmap.width(), large.bitmap.height()), (1024, 256));
        assert!(decode(b"no es una imagen").is_err());
    }

    #[test]
    fn the_fake_loader_answers_from_its_table_without_a_network() {
        let loader = FakeImageLoader::new()
            .with("https://kai.test/a.png", png(1, 1, [255; 4]))
            .failing("https://kai.test/roto.png", "404");
        let ok = load(&loader, &ImageSource::url("https://kai.test/a.png")).unwrap();
        assert_eq!((ok.width, ok.height), (1, 1));
        assert_eq!(
            load(&loader, &ImageSource::url("https://kai.test/roto.png")),
            Err("404".to_owned())
        );
        assert!(load(&loader, &ImageSource::url("https://kai.test/otra.png")).is_err());
        assert!(load(&loader, &ImageSource::path("/no/existe.png")).is_err());
        assert_eq!(loader.reads(), 4);
    }

    #[test]
    fn the_store_fills_ready_images_in_and_wants_the_others_once() {
        let pixels = ImageSource::rgba(1, 1, vec![1, 2, 3, 255]);
        let photo = ImageSource::url("https://kai.test/a.png");
        let loader = FakeImageLoader::new().with("https://kai.test/a.png", png(4, 2, [9; 4]));
        let mut store = ImageStore::new(Arc::new(loader.clone()));
        let mut tree: Element<()> = div().children([image(pixels.clone()), image(photo.clone())]);
        assert!(store.resolve(&mut tree));
        let loaded = |tree: &Element<()>, index: usize| match &tree.children[index].kind {
            Kind::Image { loaded, .. } => loaded.clone(),
            _ => unreachable!(),
        };
        assert!(loaded(&tree, 0).is_some(), "RGBA is ready at once");
        assert!(loaded(&tree, 1).is_none());
        assert_eq!(store.take_wanted(), std::slice::from_ref(&photo));
        assert_eq!(store.state(&photo), Some(&ImageState::Loading));
        assert!(!store.resolve(&mut tree), "loading is not wanted again");
        assert!(store.take_wanted().is_empty());

        let result = load(loader.as_ref_loader(), &photo);
        store.finish(&photo, result);
        store.resolve(&mut tree);
        let photo_image = loaded(&tree, 1).unwrap();
        assert_eq!((photo_image.width, photo_image.height), (4, 2));
        assert_eq!(loader.reads(), 1);

        let mut again: Element<()> = image(ImageSource::url("https://kai.test/roto.png"));
        store.load_now(&mut again);
        assert!(matches!(
            store.state(&ImageSource::url("https://kai.test/roto.png")),
            Some(ImageState::Failed(_))
        ));
    }

    #[test]
    fn the_store_drops_the_images_out_of_view_longest() {
        let mut store = ImageStore::new(Arc::new(FakeImageLoader::new()));
        for index in 0..STORE_CAPACITY + 10 {
            let mut tree: Element<()> =
                image(ImageSource::url(format!("https://kai.test/{index}")));
            store.resolve(&mut tree);
        }
        assert_eq!(store.len(), STORE_CAPACITY);
        assert!(
            store
                .state(&ImageSource::url("https://kai.test/0"))
                .is_none()
        );
        assert!(
            store
                .state(&ImageSource::url(format!(
                    "https://kai.test/{}",
                    STORE_CAPACITY + 9
                )))
                .is_some()
        );
    }

    impl FakeImageLoader {
        fn as_ref_loader(&self) -> &dyn ImageLoader {
            self
        }
    }

    mod in_a_view {
        use super::png;
        use crate::element::{Element, Role, div, image};
        use crate::media::{FakeImageLoader, ImageSource};
        use crate::render::DrawOp;
        use crate::runtime::SimpleApp;
        use crate::testing::{By, TestApp};

        const PHOTO: &str = "https://kai.test/foto.png";

        struct Photos {
            count: usize,
            classes: &'static str,
        }

        impl SimpleApp for Photos {
            type Msg = ();
            fn title(&self) -> String {
                "Fotos".into()
            }
            fn update(&mut self, (): ()) {}
            fn view(&self) -> Element<()> {
                div().children((0..self.count).map(|index| {
                    image(ImageSource::url(PHOTO))
                        .class(self.classes)
                        .alt(format!("Foto {index}"))
                }))
            }
        }

        fn photos(count: usize, classes: &'static str, loader: FakeImageLoader) -> TestApp<Photos> {
            TestApp::builder(Photos { count, classes })
                .images(loader)
                .build()
        }

        fn images_drawn(app: &TestApp<Photos>) -> usize {
            app.draw_list()
                .ops
                .iter()
                .filter(|op| matches!(op, DrawOp::Image(_)))
                .count()
        }

        #[test]
        fn an_image_loads_off_the_ui_thread_and_keeps_its_aspect_ratio() {
            let loader = FakeImageLoader::new().with(PHOTO, png(40, 20, [200, 0, 0, 255]));
            let mut app = photos(1, "w-20", loader);
            let photo = By::role(Role::Image).name("Foto 0");
            assert_eq!(app.screen().get(photo.clone()).bounds().height, 0.0);
            assert_eq!(images_drawn(&app), 0, "nothing to draw while loading");
            assert!(app.tasks_pending());

            app.run_until_idle();
            let bounds = app.screen().get(photo).bounds();
            assert_eq!((bounds.width, bounds.height), (80.0, 40.0));
            assert_eq!(images_drawn(&app), 1);
        }

        #[test]
        fn without_classes_an_image_takes_its_natural_size() {
            let loader = FakeImageLoader::new().with(PHOTO, png(30, 12, [0, 0, 200, 255]));
            let mut app = photos(1, "self-start", loader);
            app.run_until_idle();
            let bounds = app.screen().get(By::role(Role::Image)).bounds();
            assert_eq!((bounds.width, bounds.height), (30.0, 12.0));
        }

        #[test]
        fn a_failed_image_draws_nothing_and_is_not_retried() {
            let loader = FakeImageLoader::new().failing(PHOTO, "404");
            let mut app = photos(1, "w-20 h-20", loader.clone());
            app.run_until_idle();
            assert!(!app.tasks_pending());
            assert_eq!(images_drawn(&app), 0);
            let bounds = app.screen().get(By::role(Role::Image)).bounds();
            assert_eq!((bounds.width, bounds.height), (80.0, 80.0), "the box stays");
            app.click(By::role(Role::Image));
            app.run_until_idle();
            assert_eq!(loader.reads(), 1);
        }

        #[test]
        fn the_same_source_is_read_and_decoded_once() {
            let loader = FakeImageLoader::new().with(PHOTO, png(8, 8, [0, 200, 0, 255]));
            let mut app = photos(3, "w-10 h-10", loader.clone());
            app.run_until_idle();
            assert_eq!(images_drawn(&app), 3);
            app.click(By::role(Role::Image).name("Foto 1"));
            app.run_until_idle();
            assert_eq!(loader.reads(), 1);
        }

        #[test]
        fn named_images_are_images_and_unnamed_ones_are_decorative() {
            let app = TestApp::new(Photos {
                count: 1,
                classes: "w-10 h-10",
            });
            assert_eq!(
                app.screen().get(By::role(Role::Image)).name().as_deref(),
                Some("Foto 0")
            );
            let decorative: Element<()> = image(ImageSource::url(PHOTO));
            assert_eq!(decorative.role, None);
        }
    }
}
