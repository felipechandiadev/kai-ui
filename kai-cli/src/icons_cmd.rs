//! `kai icons add`: iconos propios de la app en `assets/icons/` (13, Paso 19).
//!
//! Copia un SVG (cuerpo 24×24, mismo formato que el catálogo Lucide de Kai) a
//! la carpeta de la app. La fuente puede ser un nombre Lucide ya embebido en
//! el crate `kai` (vía `cargo metadata`) o un archivo con `--from` / ruta +
//! `--name`. El runtime lo consume con `kai::element::icon_svg(include_str!(…))`.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use serde::Deserialize;

use crate::manifest;

#[derive(Debug, Clone, PartialEq)]
pub struct Opciones {
    /// Nombre kebab-case del archivo destino (`foo` → `assets/icons/foo.svg`).
    pub nombre: String,
    /// Origen explícito; si falta, se resuelve desde el catálogo del crate Kai.
    pub from: Option<PathBuf>,
}

pub fn run(opciones: &Opciones) -> Result<(), String> {
    let loaded = manifest::load_from_cwd()?;
    add_in_root(&loaded.root, opciones)
}

/// Núcleo testeable: escribe en `root/assets/icons/` sin depender del cwd.
pub fn add_in_root(root: &Path, opciones: &Opciones) -> Result<(), String> {
    validate_nombre(&opciones.nombre)?;
    let dest_dir = root.join("assets/icons");
    let dest = dest_dir.join(format!("{}.svg", opciones.nombre));
    if dest.is_file() {
        return Err(format!(
            "ya existe {}; bórralo o elige otro nombre",
            dest.display()
        ));
    }

    let source = resolve_source(root, opciones)?;
    let raw = fs::read_to_string(&source)
        .map_err(|e| format!("no se pudo leer {}: {e}", source.display()))?;
    let body = normalize_svg_body(&raw)?;

    fs::create_dir_all(&dest_dir)
        .map_err(|e| format!("no se pudo crear {}: {e}", dest_dir.display()))?;
    fs::write(&dest, body).map_err(|e| format!("no se pudo escribir {}: {e}", dest.display()))?;

    eprintln!(
        "kai icons add: {} → {}",
        source.display(),
        dest.strip_prefix(root).unwrap_or(&dest).display()
    );
    eprintln!(
        "  uso: icon_svg(include_str!(\"../assets/icons/{}.svg\"))",
        opciones.nombre
    );
    Ok(())
}

pub fn validate_nombre(nombre: &str) -> Result<(), String> {
    if nombre.is_empty() {
        return Err("nombre vacío".into());
    }
    let ok = nombre.chars().enumerate().all(|(i, c)| match c {
        'a'..='z' | '0'..='9' => true,
        '-' => i > 0,
        _ => false,
    }) && !nombre.starts_with('-')
        && !nombre.ends_with('-')
        && !nombre.contains("--");
    if !ok {
        return Err(format!(
            "nombre inválido: {nombre} (usa kebab-case Lucide, p. ej. circle-alert)"
        ));
    }
    Ok(())
}

fn resolve_source(root: &Path, opciones: &Opciones) -> Result<PathBuf, String> {
    if let Some(from) = &opciones.from {
        let path = if from.is_absolute() {
            from.clone()
        } else {
            root.join(from)
        };
        // También aceptar ruta relativa al cwd si no está bajo root.
        let path = if path.is_file() {
            path
        } else if from.is_file() {
            from.clone()
        } else {
            return Err(format!("no existe el SVG de origen: {}", from.display()));
        };
        return Ok(path);
    }
    let kai_icons = find_kai_icons_dir(root)?;
    let candidate = kai_icons.join(format!("{}.svg", opciones.nombre));
    if candidate.is_file() {
        return Ok(candidate);
    }
    Err(format!(
        "el icono `{}` no está en el catálogo de Kai ({}); \
         usa `kai icons add ./mi.svg --name {}` o `--from`",
        opciones.nombre,
        kai_icons.display(),
        opciones.nombre
    ))
}

/// Localiza `assets/icons` del paquete `kai-ui-desktop-prototype` vía metadata.
pub fn find_kai_icons_dir(app_root: &Path) -> Result<PathBuf, String> {
    let output = Command::new(cargo())
        .args(["metadata", "--format-version", "1"])
        .current_dir(app_root)
        .stderr(Stdio::piped())
        .output()
        .map_err(|e| format!("no se pudo ejecutar cargo metadata: {e}"))?;
    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr);
        return Err(format!(
            "cargo metadata falló en {}; ¿hay Cargo.toml con dependencia `kai`?\n{err}",
            app_root.display()
        ));
    }
    parse_kai_icons_dir(&output.stdout)
}

fn parse_kai_icons_dir(json: &[u8]) -> Result<PathBuf, String> {
    #[derive(Deserialize)]
    struct Metadata {
        packages: Vec<Package>,
    }
    #[derive(Deserialize)]
    struct Package {
        name: String,
        manifest_path: PathBuf,
        targets: Vec<Target>,
    }
    #[derive(Deserialize)]
    struct Target {
        name: String,
        kind: Vec<String>,
    }

    let metadata: Metadata = serde_json::from_slice(json)
        .map_err(|e| format!("cargo metadata inesperado: {e}"))?;
    for package in &metadata.packages {
        let is_kai = package.name == "kai-ui-desktop-prototype"
            || package
                .targets
                .iter()
                .any(|t| t.name == "kai" && t.kind.iter().any(|k| k == "lib"));
        if !is_kai {
            continue;
        }
        let root = package
            .manifest_path
            .parent()
            .ok_or_else(|| "manifiesto de kai sin carpeta padre".to_string())?;
        let icons = root.join("assets/icons");
        if icons.is_dir() {
            return Ok(icons);
        }
    }
    Err(
        "no se encontró el crate `kai` (kai-ui-desktop-prototype) en cargo metadata; \
         declara la dependencia en Cargo.toml o usa `--from`"
            .into(),
    )
}

fn cargo() -> String {
    std::env::var("CARGO").unwrap_or_else(|_| "cargo".into())
}

/// Deja solo los hijos del `<svg>` (formato del catálogo Kai). Si ya es cuerpo, se conserva.
pub fn normalize_svg_body(raw: &str) -> Result<String, String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err("SVG vacío".into());
    }
    let lower = trimmed.to_ascii_lowercase();
    if !lower.contains("<svg") {
        let mut body = trimmed.to_string();
        if !body.ends_with('\n') {
            body.push('\n');
        }
        return Ok(body);
    }
    let start = lower
        .find("<svg")
        .and_then(|i| lower[i..].find('>').map(|j| i + j + 1))
        .ok_or_else(|| "SVG mal formado: falta '>' de <svg>".to_string())?;
    let end_rel = lower[start..]
        .rfind("</svg>")
        .ok_or_else(|| "SVG mal formado: falta </svg>".to_string())?;
    let inner = trimmed[start..start + end_rel].trim();
    if inner.is_empty() {
        return Err("SVG sin hijos en el viewBox".into());
    }
    Ok(format!("{inner}\n"))
}

pub fn parse_args(mut args: impl Iterator<Item = String>) -> Result<Opciones, String> {
    let primero = args
        .next()
        .ok_or_else(|| "kai icons add necesita un nombre o un archivo .svg".to_string())?;
    if primero == "-h" || primero == "--help" {
        return Err(String::new());
    }

    let mut nombre: Option<String> = None;
    let mut from: Option<PathBuf> = None;

    if looks_like_svg_path(&primero) {
        from = Some(PathBuf::from(&primero));
    } else if primero.starts_with('-') {
        return Err(format!("argumento inesperado: {primero}"));
    } else {
        nombre = Some(primero);
    }

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--from" => {
                let path = args.next().ok_or("--from necesita una ruta")?;
                from = Some(PathBuf::from(path));
            }
            "--name" => {
                let n = args.next().ok_or("--name necesita un nombre")?;
                nombre = Some(n);
            }
            "-h" | "--help" => return Err(String::new()),
            otro => return Err(format!("argumento desconocido para kai icons add: {otro}")),
        }
    }

    let nombre = match nombre {
        Some(n) => n,
        None => {
            let path = from
                .as_ref()
                .ok_or("kai icons add necesita --name o un nombre Lucide")?;
            nombre_desde_path(path)?
        }
    };

    if from.is_none() && looks_like_svg_path(&nombre) {
        return Err("pasa el SVG como ruta y usa --name, o un nombre Lucide kebab-case".into());
    }

    Ok(Opciones { nombre, from })
}

fn looks_like_svg_path(s: &str) -> bool {
    s.ends_with(".svg") || s.contains('/') || s.contains('\\') || s.starts_with('.')
}

fn nombre_desde_path(path: &Path) -> Result<String, String> {
    let stem = path
        .file_stem()
        .and_then(|s| s.to_str())
        .ok_or_else(|| format!("no se pudo derivar --name de {}", path.display()))?;
    validate_nombre(stem)?;
    Ok(stem.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_root(tag: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("kai-icons-{tag}-{nanos}"));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn validate_nombre_acepta_kebab_lucide() {
        assert!(validate_nombre("search").is_ok());
        assert!(validate_nombre("circle-alert").is_ok());
        assert!(validate_nombre("a1").is_ok());
        assert!(validate_nombre("").is_err());
        assert!(validate_nombre("Search").is_err());
        assert!(validate_nombre("-x").is_err());
        assert!(validate_nombre("x-").is_err());
        assert!(validate_nombre("foo--bar").is_err());
        assert!(validate_nombre("foo bar").is_err());
    }

    #[test]
    fn normalize_extrae_hijos_de_documento_svg() {
        let full = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><circle cx="12" cy="12" r="10"/></svg>"#;
        assert_eq!(
            normalize_svg_body(full).unwrap(),
            "<circle cx=\"12\" cy=\"12\" r=\"10\"/>\n"
        );
        assert_eq!(
            normalize_svg_body("<path d=\"M1 1\"/>\n").unwrap(),
            "<path d=\"M1 1\"/>\n"
        );
    }

    #[test]
    fn parse_nombre_y_from() {
        assert_eq!(
            parse_args(["search".into()].into_iter()).unwrap(),
            Opciones {
                nombre: "search".into(),
                from: None,
            }
        );
        assert_eq!(
            parse_args(
                ["./mi.svg".into(), "--name".into(), "foo".into()]
                    .into_iter()
            )
            .unwrap(),
            Opciones {
                nombre: "foo".into(),
                from: Some(PathBuf::from("./mi.svg")),
            }
        );
        assert_eq!(
            parse_args(
                ["foo".into(), "--from".into(), "bar.svg".into()]
                    .into_iter()
            )
            .unwrap(),
            Opciones {
                nombre: "foo".into(),
                from: Some(PathBuf::from("bar.svg")),
            }
        );
    }

    #[test]
    fn add_from_file_crea_assets_icons() {
        let root = temp_root("from");
        fs::write(root.join("kai.toml"), "[app]\nname=\"t\"\nid=\"dev.kai.t\"\n").unwrap();
        let src = root.join("custom.svg");
        fs::write(
            &src,
            r#"<svg viewBox="0 0 24 24"><path d="M1 1h10"/></svg>"#,
        )
        .unwrap();
        add_in_root(
            &root,
            &Opciones {
                nombre: "mi-icono".into(),
                from: Some(src),
            },
        )
        .unwrap();
        let dest = root.join("assets/icons/mi-icono.svg");
        assert!(dest.is_file());
        assert_eq!(
            fs::read_to_string(&dest).unwrap().trim(),
            "<path d=\"M1 1h10\"/>"
        );
        let err = add_in_root(
            &root,
            &Opciones {
                nombre: "mi-icono".into(),
                from: Some(root.join("custom.svg")),
            },
        )
        .unwrap_err();
        assert!(err.contains("ya existe"), "{err}");
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn add_rechaza_nombre_invalido() {
        let root = temp_root("bad-name");
        fs::write(root.join("kai.toml"), "[app]\nname=\"t\"\nid=\"dev.kai.t\"\n").unwrap();
        let err = add_in_root(
            &root,
            &Opciones {
                nombre: "No Vale".into(),
                from: Some(root.join("x.svg")),
            },
        )
        .unwrap_err();
        assert!(err.contains("inválido"), "{err}");
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn run_exige_kai_toml() {
        let root = temp_root("no-toml");
        let prev = std::env::current_dir().unwrap();
        std::env::set_current_dir(&root).unwrap();
        let err = run(&Opciones {
            nombre: "search".into(),
            from: None,
        })
        .unwrap_err();
        let _ = std::env::set_current_dir(prev);
        assert!(
            err.contains("kai.toml") || err.contains("no hay"),
            "{err}"
        );
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn add_copia_desde_catalogo_local_simulado() {
        let root = temp_root("catalog");
        fs::write(root.join("kai.toml"), "[app]\nname=\"t\"\nid=\"dev.kai.t\"\n").unwrap();
        // Simular crate kai al lado y resolución vía --from (catálogo real necesita metadata).
        let kai_icons = root.join("fake-kai/assets/icons");
        fs::create_dir_all(&kai_icons).unwrap();
        fs::write(kai_icons.join("search.svg"), "<circle cx=\"11\" cy=\"11\" r=\"8\"/>\n")
            .unwrap();
        add_in_root(
            &root,
            &Opciones {
                nombre: "search".into(),
                from: Some(kai_icons.join("search.svg")),
            },
        )
        .unwrap();
        assert_eq!(
            fs::read_to_string(root.join("assets/icons/search.svg"))
                .unwrap()
                .trim(),
            "<circle cx=\"11\" cy=\"11\" r=\"8\"/>"
        );
        let _ = fs::remove_dir_all(&root);
    }
}
