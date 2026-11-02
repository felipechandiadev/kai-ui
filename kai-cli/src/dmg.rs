//! `kai dmg`: a `.dmg` with the app and a shortcut to `/Applications`, made
//! with macOS's own `hdiutil` (13, Paso 20).
//!
//! To distribute, sign the app first (`kai sign`), make the disk image from
//! it and notarize that: `kai bundle && kai sign --identity … && kai dmg`.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::bundle;
use crate::manifest;

#[derive(Debug, Default, PartialEq)]
pub struct Options {
    /// A bundle other than `target/kai/<Name>.app`.
    pub bundle: Option<PathBuf>,
}

pub fn run(options: &Options) -> Result<PathBuf, String> {
    if !cfg!(target_os = "macos") {
        return Err("kai dmg solo funciona en macOS".into());
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
    let dmg = make(&bundle, &loaded.manifest.app.name)?;
    eprintln!("kai dmg: {}", dmg.display());
    Ok(dmg)
}

/// `<bundle folder>/<Name>.dmg`, next to the app.
pub fn dmg_path(bundle: &Path) -> PathBuf {
    bundle.with_extension("dmg")
}

/// Build the disk image of `bundle`, replacing an earlier one.
pub fn make(bundle: &Path, volume: &str) -> Result<PathBuf, String> {
    let dmg = dmg_path(bundle);
    let staging = bundle.with_extension("dmg-staging");
    let _ = fs::remove_dir_all(&staging);
    fs::create_dir_all(&staging)
        .map_err(|error| format!("no se pudo crear {}: {error}", staging.display()))?;
    let result = stage_and_pack(bundle, &staging, &dmg, volume);
    let _ = fs::remove_dir_all(&staging);
    result.map(|()| dmg)
}

fn stage_and_pack(bundle: &Path, staging: &Path, dmg: &Path, volume: &str) -> Result<(), String> {
    let name = bundle
        .file_name()
        .ok_or_else(|| format!("{} no es un bundle", bundle.display()))?;
    // `ditto` keeps the signature, the symlinks and the extended attributes.
    let status = Command::new("ditto")
        .arg(bundle)
        .arg(staging.join(name))
        .status()
        .map_err(|error| format!("no se pudo ejecutar ditto: {error}"))?;
    if !status.success() {
        return Err("ditto no pudo copiar el .app".into());
    }
    std::os::unix::fs::symlink("/Applications", staging.join("Applications"))
        .map_err(|error| format!("no se pudo enlazar /Applications: {error}"))?;
    let _ = fs::remove_file(dmg);
    let output = Command::new("hdiutil")
        .args([
            "create",
            "-volname",
            volume,
            "-format",
            "UDZO",
            "-ov",
            "-srcfolder",
        ])
        .arg(staging)
        .arg(dmg)
        .output()
        .map_err(|error| format!("no se pudo ejecutar hdiutil: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "hdiutil falló: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_image_sits_next_to_the_app() {
        assert_eq!(
            dmg_path(Path::new("/t/kai/Demo.app")),
            PathBuf::from("/t/kai/Demo.dmg")
        );
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn the_image_holds_the_app_and_a_link_to_applications() {
        let root = std::env::temp_dir().join(format!("kai-dmg-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        let app = root.join("Demo.app/Contents/MacOS");
        fs::create_dir_all(&app).unwrap();
        fs::write(app.join("demo"), "#!/bin/sh\n").unwrap();
        let dmg = make(&root.join("Demo.app"), "Demo").unwrap();
        assert!(dmg.is_file());
        assert!(!root.join("Demo.dmg-staging").exists(), "staging removed");

        let mount = root.join("mnt");
        fs::create_dir_all(&mount).unwrap();
        let attach = Command::new("hdiutil")
            .args(["attach", "-nobrowse", "-readonly", "-mountpoint"])
            .arg(&mount)
            .arg(&dmg)
            .output()
            .unwrap();
        assert!(attach.status.success(), "{attach:?}");
        let has_app = mount.join("Demo.app/Contents/MacOS/demo").is_file();
        let has_link = fs::read_link(mount.join("Applications")).ok();
        let _ = Command::new("hdiutil").arg("detach").arg(&mount).output();
        let _ = fs::remove_dir_all(&root);
        assert!(has_app, "the app is inside");
        assert_eq!(has_link, Some(PathBuf::from("/Applications")));
    }
}
