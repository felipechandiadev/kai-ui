//! `kai.toml`: manifiesto mínimo de una app Kai (13, Paso 17).
//!
//! Vive junto al `Cargo.toml` de la app (o se encuentra subiendo desde el cwd).
//! Ningún secreto: solo declaración. La CLI lo usa para `doctor`, `dev` y `test`.

use std::fs;
use std::path::{Path, PathBuf};

use serde::Deserialize;

/// MSRV de Kai (13 / `Cargo.toml` de `kai-ui-desktop` y `kai-cli`).
pub const MSRV: &str = "1.95";

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    pub app: App,
    #[serde(default)]
    pub theme: Theme,
    #[serde(default)]
    pub window: Vec<Window>,
    #[serde(default)]
    pub capabilities: Capabilities,
    #[serde(default)]
    pub maps: Option<Maps>,
    #[serde(default)]
    pub mcp: Mcp,
    /// Opciones de la CLI (`kai dev` / `kai test`). No las lee el runtime aún.
    #[serde(default)]
    pub dev: Dev,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct App {
    pub name: String,
    pub id: String,
    #[serde(default)]
    pub icon: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Theme {
    #[serde(default = "default_theme_kind")]
    pub kind: String,
    #[serde(default = "default_appearance")]
    pub appearance: String,
    #[serde(default = "default_density")]
    pub density: String,
}

fn default_theme_kind() -> String {
    "system".into()
}
fn default_appearance() -> String {
    "system".into()
}
fn default_density() -> String {
    "comfortable".into()
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            kind: default_theme_kind(),
            appearance: default_appearance(),
            density: default_density(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Window {
    pub id: String,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub size: Option<[u32; 2]>,
    #[serde(default, rename = "min-size")]
    pub min_size: Option<[u32; 2]>,
}

#[derive(Debug, Clone, PartialEq, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Capabilities {
    #[serde(default)]
    pub clipboard: bool,
    #[serde(default)]
    pub network: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Maps {
    #[serde(default, rename = "user-agent")]
    pub user_agent: Option<String>,
    #[serde(default, rename = "cache-max-mb")]
    pub cache_max_mb: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Mcp {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_expose")]
    pub expose: String,
    #[serde(default)]
    pub allow: Vec<String>,
    #[serde(default)]
    pub deny: Vec<String>,
    #[serde(default, rename = "require-confirmation")]
    pub require_confirmation: Vec<String>,
    #[serde(default = "default_transports")]
    pub transports: Vec<String>,
}

fn default_expose() -> String {
    "none".into()
}
fn default_transports() -> Vec<String> {
    vec!["stdio".into()]
}

impl Default for Mcp {
    fn default() -> Self {
        Self {
            enabled: false,
            expose: default_expose(),
            allow: Vec::new(),
            deny: Vec::new(),
            require_confirmation: Vec::new(),
            transports: default_transports(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Dev {
    /// Binario para `kai dev` / `kai test` cuando el paquete tiene varios.
    #[serde(default)]
    pub bin: Option<String>,
    /// Features de Cargo que `kai test` (y `kai dev` al compilar) activan.
    #[serde(default)]
    pub features: Vec<String>,
    /// Nombre del paquete Cargo (`-p`), si el manifiesto está en un workspace.
    #[serde(default)]
    pub package: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Loaded {
    pub path: PathBuf,
    /// Carpeta del manifiesto (= raíz de la app por defecto).
    pub root: PathBuf,
    pub manifest: Manifest,
}

/// Busca `kai.toml` en `start` y en sus padres.
pub fn find(start: &Path) -> Option<PathBuf> {
    let mut dir = start.to_path_buf();
    loop {
        let candidate = dir.join("kai.toml");
        if candidate.is_file() {
            return Some(candidate);
        }
        if !dir.pop() {
            return None;
        }
    }
}

/// Lee y valida un `kai.toml`.
pub fn load(path: &Path) -> Result<Loaded, String> {
    let texto = fs::read_to_string(path)
        .map_err(|error| format!("no se pudo leer {}: {error}", path.display()))?;
    let manifest = parse(&texto).map_err(|error| format!("{}: {error}", path.display()))?;
    let root = path
        .parent()
        .ok_or_else(|| format!("{} no tiene carpeta padre", path.display()))?
        .to_path_buf();
    validate(&manifest, &root)?;
    Ok(Loaded {
        path: path.to_path_buf(),
        root,
        manifest,
    })
}

/// Igual que [`load`], buscando desde el directorio actual.
pub fn load_from_cwd() -> Result<Loaded, String> {
    let cwd = std::env::current_dir().map_err(|error| format!("cwd: {error}"))?;
    let path = find(&cwd).ok_or_else(|| {
        String::from(
            "no hay kai.toml aquí ni en carpetas padre; crea uno (ver docs/dev/13 Paso 17) o ejecuta kai new",
        )
    })?;
    load(&path)
}

pub fn parse(texto: &str) -> Result<Manifest, String> {
    toml::from_str(texto).map_err(|error| format_toml_error(texto, error))
}

fn format_toml_error(texto: &str, error: toml::de::Error) -> String {
    let msg = error.message().to_string();
    match error.span() {
        Some(span) => {
            let line = texto[..span.start.min(texto.len())]
                .bytes()
                .filter(|&b| b == b'\n')
                .count()
                + 1;
            format!("línea {line}: {msg}")
        }
        None => msg,
    }
}

/// Semántica que serde no cubre: enums, icono existente, mcp seguro por defecto.
pub fn validate(manifest: &Manifest, root: &Path) -> Result<(), String> {
    if manifest.app.name.trim().is_empty() {
        return Err("[app].name no puede estar vacío".into());
    }
    if manifest.app.id.trim().is_empty() {
        return Err("[app].id no puede estar vacío".into());
    }
    if let Some(icon) = &manifest.app.icon {
        let path = root.join(icon);
        if !path.is_file() {
            return Err(format!(
                "[app].icon = \"{icon}\" no existe en {}",
                path.display()
            ));
        }
    }
    match manifest.theme.kind.as_str() {
        "kai" | "appkit" | "system" => {}
        otro => {
            return Err(format!(
                "[theme].kind = \"{otro}\" inválido; usa kai, appkit o system"
            ));
        }
    }
    match manifest.theme.appearance.as_str() {
        "light" | "dark" | "system" => {}
        otro => {
            return Err(format!(
                "[theme].appearance = \"{otro}\" inválido; usa light, dark o system"
            ));
        }
    }
    match manifest.theme.density.as_str() {
        "comfortable" | "compact" => {}
        otro => {
            return Err(format!(
                "[theme].density = \"{otro}\" inválido; usa comfortable o compact"
            ));
        }
    }
    for (i, window) in manifest.window.iter().enumerate() {
        if window.id.trim().is_empty() {
            return Err(format!("[[window]][{i}].id no puede estar vacío"));
        }
    }
    match manifest.mcp.expose.as_str() {
        "none" | "allowlist" | "all" => {}
        otro => {
            return Err(format!(
                "[mcp].expose = \"{otro}\" inválido; usa none, allowlist o all"
            ));
        }
    }
    if manifest.mcp.enabled && manifest.mcp.expose == "all" {
        // Aviso blando vía Err solo si también allow vacío? Doc: seguro por defecto.
        // Permitimos enabled+all pero doctor avisará; aquí solo error si transports vacío.
    }
    if manifest.mcp.transports.is_empty() {
        return Err("[mcp].transports no puede estar vacío".into());
    }
    for t in &manifest.mcp.transports {
        if t != "stdio" {
            return Err(format!(
                "[mcp].transports contiene \"{t}\"; por ahora solo se admite stdio"
            ));
        }
    }
    Ok(())
}

/// Texto de ejemplo para `kai new` y docs.
pub fn example_toml(name: &str, id: &str, bin: Option<&str>) -> String {
    let bin_line = match bin {
        Some(b) => format!("bin = \"{b}\"\n"),
        None => String::new(),
    };
    format!(
        r#"# Manifiesto Kai (docs/dev/13, Paso 17). Sin secretos.
[app]
name = "{name}"
id = "{id}"

[theme]
kind = "system"
appearance = "system"
density = "comfortable"

[mcp]
enabled = false
expose = "none"

[dev]
{bin_line}"#
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    const MINIMO: &str = r#"
[app]
name = "Demo"
id = "dev.kai.demo"
"#;

    #[test]
    fn parses_defaults() {
        let m = parse(MINIMO).unwrap();
        assert_eq!(m.app.name, "Demo");
        assert_eq!(m.theme.kind, "system");
        assert!(!m.mcp.enabled);
        assert_eq!(m.mcp.expose, "none");
        assert!(m.dev.bin.is_none());
    }

    #[test]
    fn parses_full_schema() {
        let full = r#"
[app]
name = "DSP"
id = "cl.parral.dsp"

[theme]
kind = "appkit"
appearance = "dark"
density = "compact"

[[window]]
id = "main"
title = "DSP"
size = [1280, 800]
min-size = [720, 480]

[capabilities]
clipboard = true
network = ["http://localhost:5080"]

[maps]
user-agent = "DSP/0.1"
cache-max-mb = 256

[mcp]
enabled = false
expose = "none"
allow = []
deny = []
require-confirmation = ["destructive"]
transports = ["stdio"]

[dev]
bin = "dsp-desktop"
features = ["maps"]
package = "dsp-desktop"
"#;
        let m = parse(full).unwrap();
        assert_eq!(m.dev.bin.as_deref(), Some("dsp-desktop"));
        assert_eq!(m.window[0].size, Some([1280, 800]));
        assert_eq!(m.window[0].min_size, Some([720, 480]));
        assert_eq!(m.maps.as_ref().unwrap().cache_max_mb, Some(256));
    }

    #[test]
    fn rejects_unknown_key() {
        assert!(parse("[app]\nname=\"x\"\nid=\"y\"\nfoo = 1\n").is_err());
    }

    #[test]
    fn validate_theme_and_missing_icon() {
        let mut m = parse(MINIMO).unwrap();
        m.theme.kind = "gtk".into();
        assert!(
            validate(&m, Path::new("/tmp"))
                .unwrap_err()
                .contains("kind")
        );
        m.theme.kind = "system".into();
        m.app.icon = Some("no-existe.png".into());
        assert!(
            validate(&m, Path::new("/tmp"))
                .unwrap_err()
                .contains("icon")
        );
    }

    #[test]
    fn find_walks_up() {
        let base = std::env::temp_dir().join(format!("kai-manifest-{}", std::process::id()));
        let nested = base.join("a/b");
        std::fs::create_dir_all(&nested).unwrap();
        std::fs::write(base.join("kai.toml"), MINIMO).unwrap();
        assert_eq!(find(&nested), Some(base.join("kai.toml")));
        let _ = std::fs::remove_dir_all(&base);
    }
}
