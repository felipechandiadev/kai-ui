//! `kai doctor`: entorno y coherencia de `kai.toml` (13, Paso 18).

use std::process::Command;

use crate::manifest::{self, Loaded, MSRV};

pub fn run() -> Result<(), String> {
    let mut fallos = 0u32;
    let mut avisos = 0u32;

    eprintln!("kai doctor: MSRV {MSRV}");

    match rustc_version() {
        Ok(ver) => {
            if version_gte(&ver, MSRV) {
                eprintln!("  ok  rustc {ver} (≥ {MSRV})");
            } else {
                eprintln!("  ERR rustc {ver} es menor que {MSRV}");
                fallos += 1;
            }
        }
        Err(error) => {
            eprintln!("  ERR {error}");
            fallos += 1;
        }
    }

    match cargo_version() {
        Ok(ver) => eprintln!("  ok  cargo {ver}"),
        Err(error) => {
            eprintln!("  ERR {error}");
            fallos += 1;
        }
    }

    #[cfg(target_os = "macos")]
    {
        match xcode_clt() {
            Ok(path) => eprintln!("  ok  Xcode CLT ({path})"),
            Err(error) => {
                eprintln!("  ERR {error}");
                fallos += 1;
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
            check_manifest(&loaded, &mut fallos, &mut avisos);
        }
        Err(error) => {
            eprintln!("  ERR {error}");
            fallos += 1;
        }
    }

    if fallos == 0 && avisos == 0 {
        eprintln!("kai doctor: todo bien");
        Ok(())
    } else if fallos == 0 {
        eprintln!("kai doctor: {avisos} aviso(s), sin errores");
        Ok(())
    } else {
        Err(format!("{fallos} error(es), {avisos} aviso(s)"))
    }
}

fn check_manifest(loaded: &Loaded, fallos: &mut u32, avisos: &mut u32) {
    let cargo_toml = loaded.root.join("Cargo.toml");
    if !cargo_toml.is_file() {
        eprintln!(
            "  ERR no hay Cargo.toml junto a {} (¿path incorrecto?)",
            loaded.path.display()
        );
        *fallos += 1;
    } else {
        eprintln!("  ok  Cargo.toml en la misma carpeta");
    }

    if loaded.manifest.mcp.enabled && loaded.manifest.mcp.expose == "all" {
        eprintln!("  avs [mcp] enabled + expose = \"all\": expone todo el registro");
        *avisos += 1;
    }
    if loaded.manifest.mcp.enabled && loaded.manifest.mcp.expose == "none" {
        eprintln!("  avs [mcp] enabled = true pero expose = \"none\": no habrá herramientas");
        *avisos += 1;
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
