//! `kai sign`: `codesign` over the bundle `kai bundle` made (13, Paso 20).
//!
//! Without `--identity` it signs ad hoc (`-`): enough to run the app on this
//! Mac, not to distribute it. A Developer ID identity adds the hardened
//! runtime and a secure timestamp, which notarizing requires.

use std::path::{Path, PathBuf};
use std::process::Command;

use crate::bundle;
use crate::manifest::{self, Macos};

#[derive(Debug, Default, PartialEq)]
pub struct Options {
    /// `Developer ID Application: …`; `None` signs ad hoc.
    pub identity: Option<String>,
    /// A bundle other than `target/kai/<Name>.app`.
    pub bundle: Option<PathBuf>,
}

pub fn run(options: &Options) -> Result<PathBuf, String> {
    if !cfg!(target_os = "macos") {
        return Err("kai sign solo funciona en macOS".into());
    }
    let loaded = manifest::load_from_cwd()?;
    let bundle = match &options.bundle {
        Some(path) => path.clone(),
        None => bundle::bundle_path(&loaded.manifest.app)?,
    };
    if !bundle.is_dir() {
        return Err(format!(
            "no existe {}; ejecuta kai bundle antes",
            bundle.display()
        ));
    }
    let entitlements = loaded
        .manifest
        .macos
        .entitlements
        .as_ref()
        .map(|path| loaded.root.join(path));
    let identity = options.identity.as_deref().unwrap_or("-");
    for target in signing_order(&bundle)? {
        let args = codesign_args(
            identity,
            &loaded.manifest.macos,
            entitlements.as_deref(),
            &target,
        );
        let output = Command::new("codesign")
            .args(&args)
            .output()
            .map_err(|error| format!("no se pudo ejecutar codesign: {error}"))?;
        if !output.status.success() {
            return Err(format!(
                "codesign falló con {}: {}",
                target.display(),
                String::from_utf8_lossy(&output.stderr).trim()
            ));
        }
    }
    verify(&bundle)?;
    if identity == "-" {
        eprintln!(
            "kai sign: firmada ad hoc (solo para este Mac); para distribuir usa --identity \"Developer ID Application: …\""
        );
    }
    eprintln!("kai sign: {}", bundle.display());
    Ok(bundle)
}

/// Inside out: each executable under `Contents/MacOS`, then the bundle.
fn signing_order(bundle: &Path) -> Result<Vec<PathBuf>, String> {
    let macos = bundle.join("Contents/MacOS");
    let mut order = Vec::new();
    for entry in std::fs::read_dir(&macos)
        .map_err(|error| format!("no se pudo leer {}: {error}", macos.display()))?
    {
        let path = entry.map_err(|error| error.to_string())?.path();
        if path.is_file() {
            order.push(path);
        }
    }
    order.sort();
    order.push(bundle.to_path_buf());
    Ok(order)
}

fn codesign_args(
    identity: &str,
    macos: &Macos,
    entitlements: Option<&Path>,
    target: &Path,
) -> Vec<String> {
    let mut args = vec!["--force".to_string()];
    if identity != "-" {
        // Notarizing needs the hardened runtime and a secure timestamp.
        if macos.hardened_runtime {
            args.extend(["--options".into(), "runtime".into()]);
        }
        args.push("--timestamp".into());
    }
    if let Some(entitlements) = entitlements {
        args.extend(["--entitlements".into(), entitlements.display().to_string()]);
    }
    args.extend(["--sign".into(), identity.to_string()]);
    args.push(target.display().to_string());
    args
}

fn verify(bundle: &Path) -> Result<(), String> {
    let output = Command::new("codesign")
        .args(["--verify", "--strict", "--verbose=2"])
        .arg(bundle)
        .output()
        .map_err(|error| format!("no se pudo ejecutar codesign: {error}"))?;
    if output.status.success() {
        Ok(())
    } else {
        Err(format!(
            "la firma no verifica: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ad_hoc_signing_skips_runtime_and_timestamp() {
        let args = codesign_args("-", &Macos::default(), None, Path::new("/t/App.app"));
        assert_eq!(args, ["--force", "--sign", "-", "/t/App.app"]);
    }

    #[test]
    fn a_developer_id_gets_runtime_timestamp_and_entitlements() {
        let args = codesign_args(
            "Developer ID Application: Kai",
            &Macos::default(),
            Some(Path::new("/t/e.plist")),
            Path::new("/t/App.app"),
        );
        assert_eq!(
            args,
            [
                "--force",
                "--options",
                "runtime",
                "--timestamp",
                "--entitlements",
                "/t/e.plist",
                "--sign",
                "Developer ID Application: Kai",
                "/t/App.app"
            ]
        );
        let relaxed = Macos {
            hardened_runtime: false,
            ..Macos::default()
        };
        let args = codesign_args("X", &relaxed, None, Path::new("/t/App.app"));
        assert!(!args.contains(&"runtime".to_string()));
        assert!(args.contains(&"--timestamp".to_string()));
    }

    #[test]
    fn executables_are_signed_before_the_bundle() {
        let root = std::env::temp_dir().join(format!("kai-sign-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let bundle = root.join("App.app");
        std::fs::create_dir_all(bundle.join("Contents/MacOS")).unwrap();
        std::fs::write(bundle.join("Contents/MacOS/b"), b"").unwrap();
        std::fs::write(bundle.join("Contents/MacOS/a"), b"").unwrap();
        let order = signing_order(&bundle).unwrap();
        assert_eq!(
            order,
            [
                bundle.join("Contents/MacOS/a"),
                bundle.join("Contents/MacOS/b"),
                bundle.clone()
            ]
        );
        let _ = std::fs::remove_dir_all(&root);
    }
}
