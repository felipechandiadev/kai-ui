//! `kai doctor`: entorno y coherencia de `kai.toml` (13, Paso 18).

use std::process::Command;

use crate::manifest::{self, Loaded, MSRV};

pub fn run() -> Result<(), String> {
    let mut failures = 0u32;
    let mut warnings_found = 0u32;

    eprintln!("kai doctor: MSRV {MSRV}");

    match rustc_version() {
        Ok(version) => {
            if version_gte(&version, MSRV) {
                eprintln!("  ok  rustc {version} (≥ {MSRV})");
            } else {
                eprintln!("  ERR rustc {version} es menor que {MSRV}");
                failures += 1;
            }
        }
        Err(error) => {
            eprintln!("  ERR {error}");
            failures += 1;
        }
    }

    match cargo_version() {
        Ok(version) => eprintln!("  ok  cargo {version}"),
        Err(error) => {
            eprintln!("  ERR {error}");
            failures += 1;
        }
    }

    #[cfg(target_os = "macos")]
    {
        match xcode_clt() {
            Ok(path) => eprintln!("  ok  Xcode CLT ({path})"),
            Err(error) => {
                eprintln!("  ERR {error}");
                failures += 1;
            }
        }
    }
    #[cfg(not(target_os = "macos"))]
    {
        eprintln!("  —   Xcode CLT: no aplica fuera de macOS");
    }
    #[cfg(target_os = "macos")]
    check_distribution(&mut warnings_found);

    match manifest::load_from_cwd() {
        Ok(loaded) => {
            eprintln!("  ok  kai.toml en {}", loaded.path.display());
            check_manifest(&loaded, &mut failures, &mut warnings_found);
        }
        Err(error) => {
            eprintln!("  ERR {error}");
            failures += 1;
        }
    }

    check_sections(&mut failures);

    if failures == 0 && warnings_found == 0 {
        eprintln!("kai doctor: todo bien");
        Ok(())
    } else if failures == 0 {
        eprintln!("kai doctor: {warnings_found} aviso(s), sin errores");
        Ok(())
    } else {
        Err(format!("{failures} error(es), {warnings_found} aviso(s)"))
    }
}

fn check_manifest(loaded: &Loaded, failures: &mut u32, warnings_found: &mut u32) {
    let cargo_toml = loaded.root.join("Cargo.toml");
    if !cargo_toml.is_file() {
        eprintln!(
            "  ERR no hay Cargo.toml junto a {} (¿path incorrecto?)",
            loaded.path.display()
        );
        *failures += 1;
    } else {
        eprintln!("  ok  Cargo.toml en la misma carpeta");
    }

    if loaded.manifest.mcp.enabled && loaded.manifest.mcp.expose == "all" {
        eprintln!("  avs [mcp] enabled + expose = \"all\": expone todo el registro");
        *warnings_found += 1;
    }
    if loaded.manifest.mcp.enabled && loaded.manifest.mcp.expose == "none" {
        eprintln!("  avs [mcp] enabled = true pero expose = \"none\": no habrá herramientas");
        *warnings_found += 1;
    }

    if let Some(bin) = &loaded.manifest.dev.bin {
        eprintln!("  ok  [dev].bin = {bin}");
    }

    if let Some(icon) = &loaded.manifest.app.icon {
        check_icon(&loaded.root.join(icon), icon, failures, warnings_found);
    }
}

/// The icon `kai bundle` turns into `AppIcon.icns`: it must exist and be a
/// square image of at least 1024 px.
fn check_icon(path: &std::path::Path, shown: &str, failures: &mut u32, warnings_found: &mut u32) {
    if !path.is_file() {
        eprintln!("  ERR [app].icon = {shown}: el archivo no existe");
        *failures += 1;
        return;
    }
    #[cfg(target_os = "macos")]
    {
        let output = Command::new("sips")
            .args(["-g", "pixelWidth", "-g", "pixelHeight"])
            .arg(path)
            .output();
        let size = output
            .ok()
            .and_then(|output| parse_sips_size(&String::from_utf8_lossy(&output.stdout)));
        match size {
            Some((width, height)) => match icon_problem(width, height) {
                None => eprintln!("  ok  [app].icon = {shown} ({width}×{height})"),
                Some(problem) => {
                    eprintln!("  avs [app].icon = {shown}: {problem}");
                    *warnings_found += 1;
                }
            },
            None => {
                eprintln!("  avs [app].icon = {shown}: sips no pudo leer el tamaño de la imagen");
                *warnings_found += 1;
            }
        }
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = warnings_found;
        eprintln!("  ok  [app].icon = {shown}");
    }
}

/// `pixelWidth: 1024` and `pixelHeight: 1024` from `sips -g`.
fn parse_sips_size(output: &str) -> Option<(u32, u32)> {
    let value = |key: &str| {
        output
            .lines()
            .find_map(|line| line.trim().strip_prefix(key))
            .and_then(|rest| rest.trim_start_matches(':').trim().parse().ok())
    };
    Some((value("pixelWidth")?, value("pixelHeight")?))
}

/// Why an icon of this size will not make a good `.icns`, if it will not.
fn icon_problem(width: u32, height: u32) -> Option<String> {
    if width != height {
        return Some(format!("no es cuadrado ({width}×{height})"));
    }
    (width < 1024).then(|| format!("mide {width} px; se necesitan 1024 px o más"))
}

/// Names of the Developer ID Application identities in `security
/// find-identity -v -p codesigning`.
fn parse_developer_ids(output: &str) -> Vec<String> {
    output
        .lines()
        .filter_map(|line| {
            let start = line.find('"')? + 1;
            let end = line.rfind('"')?;
            let name = line.get(start..end)?;
            name.starts_with("Developer ID Application")
                .then(|| name.to_owned())
        })
        .collect()
}

/// Signing and notarizing, which `kai sign` and `kai notarize` need.
#[cfg(target_os = "macos")]
fn check_distribution(warnings_found: &mut u32) {
    let identities = Command::new("security")
        .args(["find-identity", "-v", "-p", "codesigning"])
        .output()
        .map(|output| parse_developer_ids(&String::from_utf8_lossy(&output.stdout)))
        .unwrap_or_default();
    if identities.is_empty() {
        eprintln!(
            "  avs sin identidad «Developer ID Application» en el llavero: `kai sign` solo \
             podrá firmar ad hoc (este Mac)"
        );
        *warnings_found += 1;
    } else {
        for name in identities {
            eprintln!("  ok  firma: {name}");
        }
    }
    let notary = Command::new("xcrun")
        .args(["--find", "notarytool"])
        .output()
        .is_ok_and(|output| output.status.success());
    if notary {
        eprintln!(
            "  ok  xcrun notarytool (el perfil se guarda con `xcrun notarytool store-credentials`)"
        );
    } else {
        eprintln!("  avs xcrun notarytool no está disponible: `kai notarize` no funcionará");
        *warnings_found += 1;
    }
}

fn rustc_version() -> Result<String, String> {
    let output = Command::new("rustc")
        .arg("--version")
        .output()
        .map_err(|error| format!("no se pudo ejecutar rustc: {error}"))?;
    if !output.status.success() {
        return Err("rustc --version falló".into());
    }
    parse_toolchain_version(&String::from_utf8_lossy(&output.stdout))
        .ok_or_else(|| "no se entendió rustc --version".into())
}

fn cargo_version() -> Result<String, String> {
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".into());
    let output = Command::new(cargo)
        .arg("--version")
        .output()
        .map_err(|error| format!("no se pudo ejecutar cargo: {error}"))?;
    if !output.status.success() {
        return Err("cargo --version falló".into());
    }
    parse_toolchain_version(&String::from_utf8_lossy(&output.stdout))
        .ok_or_else(|| "no se entendió cargo --version".into())
}

/// `rustc 1.89.0 (...)` → `1.89.0`
fn parse_toolchain_version(line: &str) -> Option<String> {
    line.split_whitespace().nth(1).map(|s| s.to_string())
}

/// Compara `1.89.0` ≥ `1.89` por componentes numéricos.
fn version_gte(have: &str, need: &str) -> bool {
    let parse = |s: &str| -> Vec<u64> {
        s.split('.')
            .filter_map(|p| {
                p.chars()
                    .take_while(|c| c.is_ascii_digit())
                    .collect::<String>()
                    .parse()
                    .ok()
            })
            .collect()
    };
    let a = parse(have);
    let b = parse(need);
    let len = a.len().max(b.len());
    for i in 0..len {
        let x = a.get(i).copied().unwrap_or(0);
        let y = b.get(i).copied().unwrap_or(0);
        if x != y {
            return x > y;
        }
    }
    true
}

#[cfg(target_os = "macos")]
fn xcode_clt() -> Result<String, String> {
    let output = Command::new("xcode-select")
        .arg("-p")
        .output()
        .map_err(|error| format!("no se pudo ejecutar xcode-select: {error}"))?;
    if !output.status.success() {
        return Err("no hay Xcode Command Line Tools; instala con `xcode-select --install`".into());
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

/// An app made of `src/sections/`: the same checks as its build.
fn check_sections(failures: &mut u32) {
    let root = std::path::Path::new(".");
    if !root.join("src/sections").exists() {
        return;
    }
    match kai_build::scan::scan(root) {
        Ok(tree) => {
            let pages: usize = tree.sections.iter().map(|s| s.pages.len()).sum();
            eprintln!(
                "  ok  src/sections: {} sección(es), {pages} página(s)",
                tree.sections.len()
            );
        }
        Err(errors) => {
            for error in errors {
                eprintln!("  ERR {error}");
                *failures += 1;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_icon_must_be_square_and_at_least_1024() {
        let sips = "/x/icon.png\n  pixelWidth: 1024\n  pixelHeight: 1024\n";
        assert_eq!(parse_sips_size(sips), Some((1024, 1024)));
        assert_eq!(parse_sips_size("nada"), None);
        assert_eq!(icon_problem(1024, 1024), None);
        assert_eq!(icon_problem(2048, 2048), None);
        assert!(icon_problem(1024, 512).unwrap().contains("cuadrado"));
        assert!(icon_problem(512, 512).unwrap().contains("1024"));
    }

    #[test]
    fn only_developer_id_application_identities_count() {
        let output = r#"  1) AAAA "Apple Development: Ana (X1)"
  2) BBBB "Developer ID Application: Acme SA (T1)"
  3) CCCC "Developer ID Installer: Acme SA (T1)"
     3 valid identities found
"#;
        assert_eq!(
            parse_developer_ids(output),
            ["Developer ID Application: Acme SA (T1)"]
        );
        assert!(parse_developer_ids("     0 valid identities found").is_empty());
    }

    #[test]
    fn msrv_comparison() {
        assert!(version_gte("1.89.0", "1.89"));
        assert!(version_gte("1.92.0", "1.89"));
        assert!(!version_gte("1.88.0", "1.89"));
        assert!(version_gte("1.89", "1.89.0"));
    }

    #[test]
    fn parses_rustc_line() {
        assert_eq!(
            parse_toolchain_version("rustc 1.92.0 (ded5c06cf 2025-12-08)"),
            Some("1.92.0".into())
        );
    }
}
