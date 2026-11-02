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
    /// No se encontró la carpeta de datos. [`Store::for_app`] ya no lo
    /// devuelve (usa la carpeta temporal); queda para las apps que lo
    /// comparan.
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
    /// `~/Library/Application Support/<id>` en macOS, `%APPDATA%\<id>` en
    /// Windows, `$XDG_DATA_HOME/<id>` (o `~/.local/share/<id>`) en otros
    /// sistemas. Sin esas variables (un proceso lanzado sin `HOME`) usa
    /// `<temp>/<id>`: no persiste entre reinicios del sistema, pero la app
    /// arranca y guarda igual. Hoy nunca devuelve error; el `Result` se
    /// mantiene por compatibilidad. No crea la carpeta hasta el primer
    /// guardado.
    pub fn for_app(id: &str) -> Result<Self, StoreError> {
        let os = if cfg!(target_os = "macos") {
            DataOs::Mac
        } else if cfg!(windows) {
            DataOs::Windows
        } else {
            DataOs::Other
        };
        Ok(Self::at(data_dir_from(id, os, |name| {
            std::env::var_os(name)
        })))
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DataOs {
    Mac,
    Windows,
    Other,
}

/// [`Store::for_app`] with the OS and the environment as parameters, so
/// tests cover every platform. Empty variables count as missing.
fn data_dir_from(
    id: &str,
    os: DataOs,
    var: impl Fn(&str) -> Option<std::ffi::OsString>,
) -> PathBuf {
    let var = |name: &str| {
        var(name)
            .filter(|value| !value.is_empty())
            .map(PathBuf::from)
    };
    let found = match os {
        DataOs::Mac => var("HOME").map(|home| home.join("Library/Application Support")),
        DataOs::Windows => var("APPDATA").or_else(|| var("LOCALAPPDATA")),
        DataOs::Other => {
            var("XDG_DATA_HOME").or_else(|| var("HOME").map(|home| home.join(".local/share")))
        }
    };
    found.unwrap_or_else(std::env::temp_dir).join(id)
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

    use std::ffi::OsString;
    use std::path::PathBuf;

    use super::{DataOs, Store, StoreError, Stored, data_dir_from};

    fn env(pairs: &[(&str, &str)]) -> impl Fn(&str) -> Option<OsString> {
        let pairs: Vec<(String, String)> = pairs
            .iter()
            .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
            .collect();
        move |name| {
            pairs
                .iter()
                .find(|(k, _)| k == name)
                .map(|(_, v)| OsString::from(v))
        }
    }

    #[test]
    fn the_data_folder_follows_each_platform() {
        let id = "dev.kai.prueba";
        assert_eq!(
            data_dir_from(id, DataOs::Mac, env(&[("HOME", "/Users/ana")])),
            PathBuf::from("/Users/ana/Library/Application Support").join(id)
        );
        assert_eq!(
            data_dir_from(
                id,
                DataOs::Windows,
                env(&[("APPDATA", "C:/Users/ana/AppData/Roaming")])
            ),
            PathBuf::from("C:/Users/ana/AppData/Roaming").join(id)
        );
        assert_eq!(
            data_dir_from(id, DataOs::Windows, env(&[("LOCALAPPDATA", "C:/Local")])),
            PathBuf::from("C:/Local").join(id)
        );
        assert_eq!(
            data_dir_from(id, DataOs::Other, env(&[("HOME", "/home/ana")])),
            PathBuf::from("/home/ana/.local/share").join(id)
        );
        assert_eq!(
            data_dir_from(
                id,
                DataOs::Other,
                env(&[("XDG_DATA_HOME", "/datos"), ("HOME", "/home/ana")])
            ),
            PathBuf::from("/datos").join(id)
        );
    }

    #[test]
    fn without_home_the_data_folder_is_in_temp() {
        let id = "dev.kai.prueba";
        for os in [DataOs::Mac, DataOs::Windows, DataOs::Other] {
            assert_eq!(
                data_dir_from(id, os, env(&[("HOME", "")])),
                std::env::temp_dir().join(id),
                "{os:?}"
            );
        }
    }

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
