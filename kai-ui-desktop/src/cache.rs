//! Caché en disco con límite de tamaño, para lo que se descarga de la red
//! (imágenes por URL hoy; teselas HTTP más adelante, docs/dev/10-mapas.md
//! §4.4). Paso 16 de docs/dev/13-hoja-de-ruta-plataforma.md.
//!
//! Cada entrada es un archivo en la carpeta de la caché, nombrado por un
//! hash de su clave (la URL). Leer una entrada actualiza su fecha de
//! modificación; al escribir, si la carpeta pasa del límite, se borran las
//! entradas usadas hace más tiempo (LRU por fecha). Escribir es atómico
//! (archivo temporal + `rename`), así una entrada nunca queda a medias.
//!
//! Es una caché: cualquier error al leer cuenta como que no estaba, y uno
//! al escribir solo se informa. A diferencia de [`crate::store`], lo que
//! hay aquí se puede borrar en cualquier momento (macOS limpia
//! `~/Library/Caches` cuando falta espacio).

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::SystemTime;

/// Carpeta de caché con un tope en bytes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DiskCache {
    dir: PathBuf,
    limit: u64,
}

impl DiskCache {
    /// Tope por defecto: 200 MB.
    pub const DEFAULT_LIMIT: u64 = 200 * 1024 * 1024;

    /// `~/Library/Caches/<app_id>/<name>` en macOS, `$XDG_CACHE_HOME`
    /// (o `~/.cache`) en otros sistemas. No crea la carpeta hasta la
    /// primera escritura. `None` sin `HOME`.
    pub fn for_app(app_id: &str, name: &str, limit: u64) -> Option<Self> {
        let home = PathBuf::from(std::env::var_os("HOME")?);
        let base = if cfg!(target_os = "macos") {
            home.join("Library/Caches")
        } else {
            std::env::var_os("XDG_CACHE_HOME")
                .map(PathBuf::from)
                .unwrap_or_else(|| home.join(".cache"))
        };
        Some(Self::at(base.join(app_id).join(name), limit))
    }

    /// Una carpeta elegida por la app (o temporal, en tests).
    pub fn at(dir: impl Into<PathBuf>, limit: u64) -> Self {
        Self {
            dir: dir.into(),
            limit,
        }
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }

    pub fn limit(&self) -> u64 {
        self.limit
    }

    fn path_of(&self, key: &str) -> PathBuf {
        self.dir.join(format!("{:016x}", fnv1a(key.as_bytes())))
    }

    /// Los bytes guardados con `key`, si están; marca la entrada como
    /// recién usada.
    pub fn get(&self, key: &str) -> Option<Vec<u8>> {
        let path = self.path_of(key);
        let bytes = fs::read(&path).ok()?;
        if let Ok(file) = fs::File::options().write(true).open(&path) {
            let _ = file.set_modified(SystemTime::now());
        }
        Some(bytes)
    }

    /// Guarda `bytes` con `key` y recorta la carpeta al límite. Una entrada
    /// más grande que el límite no se guarda.
    pub fn put(&self, key: &str, bytes: &[u8]) -> std::io::Result<()> {
        if bytes.len() as u64 > self.limit {
            return Ok(());
        }
        fs::create_dir_all(&self.dir)?;
        let path = self.path_of(key);
        // Un nombre temporal por escritura: varios hilos del pool pueden
        // guardar a la vez.
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let tmp = self.dir.join(format!(
            ".tmp-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let mut file = fs::File::create(&tmp)?;
        file.write_all(bytes)?;
        file.sync_all()?;
        drop(file);
        fs::rename(&tmp, &path)?;
        self.trim()
    }

    /// Bytes que ocupan las entradas ahora.
    pub fn size(&self) -> u64 {
        self.entries().iter().map(|e| e.size).sum()
    }

    /// Borra las entradas usadas hace más tiempo hasta quedar bajo el límite.
    pub fn trim(&self) -> std::io::Result<()> {
        let mut entries = self.entries();
        let mut total: u64 = entries.iter().map(|e| e.size).sum();
        if total <= self.limit {
            return Ok(());
        }
        entries.sort_by_key(|e| e.used);
        for entry in entries {
            if total <= self.limit {
                break;
            }
            // Otra escritura pudo borrarla ya: no es un error.
            if fs::remove_file(&entry.path).is_ok() {
                total = total.saturating_sub(entry.size);
            }
        }
        Ok(())
    }

    /// Borra toda la caché.
    pub fn clear(&self) -> std::io::Result<()> {
        match fs::remove_dir_all(&self.dir) {
            Err(e) if e.kind() != std::io::ErrorKind::NotFound => Err(e),
            _ => Ok(()),
        }
    }

    fn entries(&self) -> Vec<Entry> {
        let Ok(dir) = fs::read_dir(&self.dir) else {
            return Vec::new();
        };
        dir.flatten()
            .filter(|e| !e.file_name().to_string_lossy().starts_with(".tmp-"))
            .filter_map(|e| {
                let meta = e.metadata().ok()?;
                meta.is_file().then(|| Entry {
                    path: e.path(),
                    size: meta.len(),
                    used: meta.modified().unwrap_or(SystemTime::UNIX_EPOCH),
                })
            })
            .collect()
    }
}

struct Entry {
    path: PathBuf,
    size: u64,
    used: SystemTime,
}

/// FNV-1a de 64 bits: estable entre ejecuciones y versiones de Rust (a
/// diferencia de `DefaultHasher`), suficiente para nombrar archivos.
fn fnv1a(bytes: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, SystemTime};

    use super::DiskCache;

    fn temp_cache(name: &str, limit: u64) -> DiskCache {
        let dir = std::env::temp_dir().join(format!("kai-cache-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        DiskCache::at(dir, limit)
    }

    /// Fecha de uso de `key`, para ordenar entradas sin dormir el test.
    fn age(cache: &DiskCache, key: &str, seconds_ago: u64) {
        let file = std::fs::File::options()
            .write(true)
            .open(cache.path_of(key))
            .unwrap();
        file.set_modified(SystemTime::now() - Duration::from_secs(seconds_ago))
            .unwrap();
    }

    #[test]
    fn stores_and_reads_back_by_key() {
        let cache = temp_cache("roundtrip", 1024);
        assert_eq!(cache.get("https://kai.test/a.png"), None);
        cache.put("https://kai.test/a.png", b"abc").unwrap();
        assert_eq!(
            cache.get("https://kai.test/a.png").as_deref(),
            Some(&b"abc"[..])
        );
        assert_eq!(cache.get("https://kai.test/b.png"), None);
        cache.clear().unwrap();
    }

    #[test]
    fn stays_under_its_limit_dropping_the_least_recently_used() {
        let cache = temp_cache("lru", 25);
        cache.put("a", &[1; 10]).unwrap();
        cache.put("b", &[2; 10]).unwrap();
        age(&cache, "a", 300);
        age(&cache, "b", 200);
        // Leer `a` la vuelve la más reciente: la que sale es `b`.
        assert!(cache.get("a").is_some());
        cache.put("c", &[3; 10]).unwrap();
        assert!(cache.size() <= 25);
        assert!(cache.get("a").is_some());
        assert_eq!(cache.get("b"), None);
        assert!(cache.get("c").is_some());
        cache.clear().unwrap();
    }

    #[test]
    fn an_entry_larger_than_the_limit_is_not_kept() {
        let cache = temp_cache("huge", 8);
        cache.put("grande", &[0; 9]).unwrap();
        assert_eq!(cache.get("grande"), None);
        assert_eq!(cache.size(), 0);
    }

    #[test]
    fn writing_leaves_no_temporary_files() {
        let cache = temp_cache("tmp", 1024);
        cache.put("x", b"1").unwrap();
        cache.put("x", b"2").unwrap();
        assert_eq!(cache.get("x").as_deref(), Some(&b"2"[..]));
        let names = std::fs::read_dir(cache.dir()).unwrap().count();
        assert_eq!(names, 1);
        cache.clear().unwrap();
    }
}
