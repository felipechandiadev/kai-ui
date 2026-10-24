//! Persistencia: preferencias y estado de la app que sobreviven al cierre
//! (docs/dev/13-hoja-de-ruta-plataforma.md, Paso 16).
//!
//! Cada valor guardado es un tipo [`Stored`] con una clave (el nombre del
//! archivo) y una versión. [`Store`] lo escribe como JSON en la carpeta de
//! datos de la app, de forma atómica (archivo temporal + `rename`), y al leer
//! migra datos de versiones anteriores con [`Stored::migrate`]. Un archivo
//! corrupto o de una versión futura es un [`StoreError`], nunca un panic.
//!
//! Leer es barato y se hace al arrancar (antes de [`crate::runtime::run`] o en
//! `App::init`); escribir va fuera del hilo de UI con [`Store::save_effect`].
//!
//! ```ignore
//! #[derive(Serialize, Deserialize)]
//! struct Prefs { seccion: String }
//!
//! impl Stored for Prefs {
//!     const KEY: &'static str = "prefs";
//!     const VERSION: u32 = 1;
//! }
//!
//! let store = Store::for_app("dev.kai.ui-desktop")?;
//! let prefs: Option<Prefs> = store.load().unwrap_or_default();
//! // En update:
//! return store.save_effect(&prefs, Msg::Guardado);
//! ```

use std::fmt;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::{Value, json};

use crate::runtime::Effects;

/// Un valor que la app guarda entre ejecuciones.
pub trait Stored: Serialize + DeserializeOwned {
    /// Nombre del archivo, sin extensión (`prefs` → `prefs.json`). Solo
    /// letras, números, `-`, `_` y `.`.
    const KEY: &'static str;
    /// Versión del formato. Súbela cuando cambie de forma incompatible y
    /// convierte los datos viejos en [`Stored::migrate`].
    const VERSION: u32;

    /// Convierte `raw`, guardado con la versión `from` (menor que
    /// [`Stored::VERSION`]), al formato actual. Por defecto no sabe
    /// migrar: los datos viejos se descartan con un error.
    fn migrate(from: u32, raw: Value) -> Result<Value, String> {
        let _ = raw;
        Err(format!(
            "no hay migración de la versión {from} a la {}",
            Self::VERSION
        ))
    }
}

/// Por qué no se pudo leer o escribir un valor.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StoreError {
    /// Error del sistema de archivos.
    Io(String),
    /// El archivo no es JSON válido o no tiene la forma esperada.
    Corrupt(String),
    /// El archivo es de una versión más nueva que la que esta app conoce.
    FutureVersion { found: u32, known: u32 },
    /// [`Stored::migrate`] no pudo convertir los datos.
    Migration(String),
    /// No se encontró la carpeta de datos (sin `HOME`).
    NoDataDir,
}

impl fmt::Display for StoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(e) => write!(f, "error de archivo: {e}"),
            Self::Corrupt(e) => write!(f, "archivo dañado: {e}"),
            Self::FutureVersion { found, known } => write!(
                f,
                "el archivo es de la versión {found} y esta app conoce hasta la {known}"
            ),
            Self::Migration(e) => write!(f, "no se pudo migrar: {e}"),
            Self::NoDataDir => write!(f, "no se encontró la carpeta de datos (falta HOME)"),
        }
    }
}

impl std::error::Error for StoreError {}

/// Carpeta donde una app guarda sus valores [`Stored`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Store {
    dir: PathBuf,
}

impl Store {
    /// La carpeta de datos de la app `id` (como `dev.kai.ui-desktop`):
    /// `~/Library/Application Support/<id>` en macOS,
    /// `$XDG_DATA_HOME/<id>` (o `~/.local/share/<id>`) en otros sistemas.
    /// No crea la carpeta hasta el primer guardado.
    pub fn for_app(id: &str) -> Result<Self, StoreError> {
        let home = std::env::var_os("HOME").ok_or(StoreError::NoDataDir)?;
        let home = PathBuf::from(home);
        let base = if cfg!(target_os = "macos") {
            home.join("Library/Application Support")
        } else {
            std::env::var_os("XDG_DATA_HOME")
                .map(PathBuf::from)
                .unwrap_or_else(|| home.join(".local/share"))
        };
        Ok(Self::at(base.join(id)))
    }

    /// Una carpeta elegida por la app (o un directorio temporal en tests).
    pub fn at(dir: impl Into<PathBuf>) -> Self {
        Self { dir: dir.into() }
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }

    /// Ruta del archivo de `T`.
    pub fn path_of<T: Stored>(&self) -> PathBuf {
        debug_assert!(
            T::KEY
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.')),
            "Stored::KEY inválida: {}",
            T::KEY
        );
        self.dir.join(format!("{}.json", T::KEY))
    }

    /// El valor guardado de `T`: `Ok(None)` si nunca se guardó, migrado si
    /// es de una versión anterior.
    pub fn load<T: Stored>(&self) -> Result<Option<T>, StoreError> {
        let text = match fs::read_to_string(self.path_of::<T>()) {
            Ok(text) => text,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(StoreError::Io(e.to_string())),
        };
        decode::<T>(&text).map(Some)
    }

    /// Guarda `value` de forma atómica: un corte a mitad de escritura deja
    /// el archivo anterior intacto. Crea la carpeta si hace falta.
    pub fn save<T: Stored>(&self, value: &T) -> Result<(), StoreError> {
        let text = encode(value)?;
        write_atomic(&self.path_of::<T>(), text.as_bytes())
    }

    /// Borra el valor guardado de `T`, si existe.
    pub fn remove<T: Stored>(&self) -> Result<(), StoreError> {
        match fs::remove_file(self.path_of::<T>()) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(StoreError::Io(e.to_string())),
        }
    }

    /// [`Store::save`] en el pool de tareas, fuera del hilo de UI; el
    /// resultado vuelve como `to_msg(..)`. Guardados seguidos del mismo
    /// valor comparten la clave `kai.store.<KEY>`, así solo el último
    /// entrega su mensaje.
    pub fn save_effect<T, Msg>(
        &self,
        value: &T,
        to_msg: impl FnOnce(Result<(), StoreError>) -> Msg + 'static,
    ) -> Effects<Msg>
    where
        T: Stored,
    {
        let path = self.path_of::<T>();
        let encoded = encode(value);
        Effects::perform(
            move || encoded.and_then(|text| write_atomic(&path, text.as_bytes())),
            to_msg,
        )
        .keyed(format!("kai.store.{}", T::KEY))
    }
}

/// `{"version": N, "data": …}`.
fn encode<T: Stored>(value: &T) -> Result<String, StoreError> {
    let data = serde_json::to_value(value).map_err(|e| StoreError::Corrupt(e.to_string()))?;
    serde_json::to_string_pretty(&json!({ "version": T::VERSION, "data": data }))
        .map_err(|e| StoreError::Corrupt(e.to_string()))
}

fn decode<T: Stored>(text: &str) -> Result<T, StoreError> {
    let envelope: Value =
        serde_json::from_str(text).map_err(|e| StoreError::Corrupt(e.to_string()))?;
    let version = envelope
        .get("version")
        .and_then(Value::as_u64)
        .and_then(|v| u32::try_from(v).ok())
        .ok_or_else(|| StoreError::Corrupt("falta \"version\"".into()))?;
    let mut data = envelope
        .get("data")
        .cloned()
        .ok_or_else(|| StoreError::Corrupt("falta \"data\"".into()))?;
    if version > T::VERSION {
        return Err(StoreError::FutureVersion {
            found: version,
            known: T::VERSION,
        });
    }
    if version < T::VERSION {
        data = T::migrate(version, data).map_err(StoreError::Migration)?;
    }
    serde_json::from_value(data).map_err(|e| StoreError::Corrupt(e.to_string()))
}

/// Escribe en `<archivo>.tmp`, lo sincroniza a disco y lo renombra encima
/// del destino (`rename` es atómico dentro de un mismo volumen).
fn write_atomic(path: &Path, bytes: &[u8]) -> Result<(), StoreError> {
    let io = |e: std::io::Error| StoreError::Io(e.to_string());
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir).map_err(io)?;
    }
    let mut tmp_name = path.as_os_str().to_owned();
    tmp_name.push(".tmp");
    let tmp = PathBuf::from(tmp_name);
    let mut file = fs::File::create(&tmp).map_err(io)?;
    file.write_all(bytes).map_err(io)?;
    file.sync_all().map_err(io)?;
    drop(file);
    fs::rename(&tmp, path).map_err(io)
}

#[cfg(test)]
mod tests {
    use serde::{Deserialize, Serialize};
    use serde_json::{Value, json};

    use super::{Store, StoreError, Stored};

    #[derive(Debug, PartialEq, Serialize, Deserialize)]
    struct Prefs {
        section: String,
        zoom: f64,
    }

    impl Stored for Prefs {
        const KEY: &'static str = "prefs";
        const VERSION: u32 = 2;

        /// v1 guardaba `section` en inglés y sin zoom.
        fn migrate(from: u32, raw: Value) -> Result<Value, String> {
            match from {
                1 => Ok(json!({ "section": raw["section"], "zoom": 12.0 })),
                _ => Err(format!("versión {from} desconocida")),
            }
        }
    }

    fn temp_store(name: &str) -> Store {
        let dir = std::env::temp_dir().join(format!(
            "kai-store-{name}-{}-{:?}",
            std::process::id(),
            std::time::SystemTime::now()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        Store::at(dir)
    }

    #[test]
    fn missing_is_none_then_saved_values_come_back() {
        let store = temp_store("roundtrip");
        assert_eq!(store.load::<Prefs>(), Ok(None));
        let prefs = Prefs {
            section: "map".into(),
            zoom: 14.5,
        };
        store.save(&prefs).unwrap();
        assert_eq!(store.load::<Prefs>(), Ok(Some(prefs)));
        store.remove::<Prefs>().unwrap();
        assert_eq!(store.load::<Prefs>(), Ok(None));
    }

    #[test]
    fn old_versions_are_migrated() {
        let store = temp_store("migrate");
        std::fs::create_dir_all(store.dir()).unwrap();
        std::fs::write(
            store.path_of::<Prefs>(),
            r#"{"version": 1, "data": {"section": "table"}}"#,
        )
        .unwrap();
        assert_eq!(
            store.load::<Prefs>(),
            Ok(Some(Prefs {
                section: "table".into(),
                zoom: 12.0
            }))
        );
    }

    #[test]
    fn corrupt_and_future_files_are_errors_not_panics() {
        let store = temp_store("corrupt");
        std::fs::create_dir_all(store.dir()).unwrap();
        let path = store.path_of::<Prefs>();

        std::fs::write(&path, "{ esto no es json").unwrap();
        assert!(matches!(store.load::<Prefs>(), Err(StoreError::Corrupt(_))));

        std::fs::write(&path, r#"{"version": 2, "data": {"section": 5}}"#).unwrap();
        assert!(matches!(store.load::<Prefs>(), Err(StoreError::Corrupt(_))));

        std::fs::write(&path, r#"{"version": 9, "data": {}}"#).unwrap();
        assert_eq!(
            store.load::<Prefs>(),
            Err(StoreError::FutureVersion { found: 9, known: 2 })
        );

        std::fs::write(&path, r#"{"version": 0, "data": {}}"#).unwrap();
        assert!(matches!(
            store.load::<Prefs>(),
            Err(StoreError::Migration(_))
        ));
    }

    #[test]
    fn saving_is_atomic_and_leaves_no_temporary_file() {
        let store = temp_store("atomic");
        let first = Prefs {
            section: "home".into(),
            zoom: 1.0,
        };
        store.save(&first).unwrap();
        let second = Prefs {
            section: "map".into(),
            zoom: 2.0,
        };
        store.save(&second).unwrap();
        assert_eq!(store.load::<Prefs>(), Ok(Some(second)));
        let names: Vec<_> = std::fs::read_dir(store.dir())
            .unwrap()
            .map(|e| e.unwrap().file_name().into_string().unwrap())
            .collect();
        assert_eq!(names, ["prefs.json"]);
    }
}
