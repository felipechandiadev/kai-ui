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

#[cfg(test)]
mod tests {
    use super::*;

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
