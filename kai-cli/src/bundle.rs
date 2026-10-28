//! `kai bundle`: an unsigned `<Name>.app` under `target/kai/` (13, Paso 20).
//!
//! Release build of the app, then `Contents/MacOS/<bin>`, `Info.plist` from
//! `kai.toml` and, when `[app].icon` is set, `AppIcon.icns` made with
//! macOS's own `sips` and `iconutil`. Signing and notarizing come after.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::cargo::{self, Failure};
use crate::manifest::{self, App};

#[derive(Debug, Default, PartialEq)]
pub struct Options {
    pub bin: Option<String>,
    /// Bundle the debug build (faster to make, only for trying the layout).
    pub debug: bool,
}

pub fn run(options: &Options) -> Result<PathBuf, String> {
    if !cfg!(target_os = "macos") {
        return Err("kai bundle solo funciona en macOS".into());
    }
    let loaded = manifest::load_from_cwd()?;
    let app = &loaded.manifest.app;
    let bin = options.bin.clone().or(loaded.manifest.dev.bin.clone());
    eprintln!("kai bundle: compilando {}…", app.name);
    let binary = cargo::build(
        bin.as_deref(),
        &loaded.manifest.dev.features,
        !options.debug,
    )
    .map_err(|failure| match failure {
        Failure::Build => "el código no compila".to_string(),
        Failure::Other(message) => message,
    })?;
    let project = cargo::project()?;
    let out = project.target_directory.join("kai");
    let bundle = out.join(format!("{}.app", folder_name(&app.name)));
    let icon = app.icon.as_ref().map(|icon| loaded.root.join(icon));
    assemble(&bundle, &binary, app, icon.as_deref())?;
    eprintln!("kai bundle: {}", bundle.display());
    Ok(bundle)
}

/// Build `bundle` from `binary`, replacing what was there.
fn assemble(bundle: &Path, binary: &Path, app: &App, icon: Option<&Path>) -> Result<(), String> {
    let executable = binary
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| format!("{} no es un binario", binary.display()))?;
    if bundle.exists() {
        fs::remove_dir_all(bundle)
            .map_err(|error| format!("no se pudo reemplazar {}: {error}", bundle.display()))?;
    }
    let contents = bundle.join("Contents");
    let macos = contents.join("MacOS");
    let resources = contents.join("Resources");
    for folder in [&macos, &resources] {
        fs::create_dir_all(folder)
            .map_err(|error| format!("no se pudo crear {}: {error}", folder.display()))?;
    }
    fs::copy(binary, macos.join(executable))
        .map_err(|error| format!("no se pudo copiar el binario: {error}"))?;
    let has_icon = match icon {
        Some(png) => {
            make_icns(png, &resources.join("AppIcon.icns"))?;
            true
        }
        None => false,
    };
    fs::write(
        contents.join("Info.plist"),
        info_plist(app, executable, has_icon),
    )
    .map_err(|error| format!("no se pudo escribir Info.plist: {error}"))
}

/// The folder name of the app: the name without what a path cannot hold.
fn folder_name(name: &str) -> String {
    name.chars()
        .map(|c| {
            if matches!(c, '/' | ':' | '\\') {
                '-'
            } else {
                c
            }
        })
        .collect()
}

fn xml_escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn info_plist(app: &App, executable: &str, has_icon: bool) -> String {
    let mut entries: Vec<(&str, String)> = vec![
        ("CFBundleName", app.name.clone()),
        ("CFBundleDisplayName", app.name.clone()),
        ("CFBundleIdentifier", app.id.clone()),
        ("CFBundleExecutable", executable.to_string()),
        ("CFBundlePackageType", "APPL".into()),
        ("CFBundleInfoDictionaryVersion", "6.0".into()),
        (
            "CFBundleShortVersionString",
            app.version.clone().unwrap_or_else(|| "0.1.0".into()),
        ),
        (
            "CFBundleVersion",
            app.version.clone().unwrap_or_else(|| "0.1.0".into()),
        ),
        (
            "LSMinimumSystemVersion",
            app.min_macos.clone().unwrap_or_else(|| "13.0".into()),
        ),
        ("NSPrincipalClass", "NSApplication".into()),
    ];
    if has_icon {
        entries.push(("CFBundleIconFile", "AppIcon".into()));
    }
    if let Some(copyright) = &app.copyright {
        entries.push(("NSHumanReadableCopyright", copyright.clone()));
    }
    if let Some(category) = &app.category {
        entries.push(("LSApplicationCategoryType", category.clone()));
    }
    let mut plist = String::from(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
         <!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\" \"http://www.apple.com/DTDs/PropertyList-1.0.dtd\">\n\
         <plist version=\"1.0\">\n<dict>\n",
    );
    for (key, value) in entries {
        plist.push_str(&format!(
            "\t<key>{key}</key>\n\t<string>{}</string>\n",
            xml_escape(&value)
        ));
    }
    plist.push_str("\t<key>NSHighResolutionCapable</key>\n\t<true/>\n</dict>\n</plist>\n");
    plist
}

/// `AppIcon.icns` from a square PNG of at least 1024 px: the iconset sizes
/// with `sips`, then `iconutil`.
fn make_icns(png: &Path, icns: &Path) -> Result<(), String> {
    let iconset = icns.with_extension("iconset");
    fs::create_dir_all(&iconset)
        .map_err(|error| format!("no se pudo crear {}: {error}", iconset.display()))?;
    for size in [16u32, 32, 128, 256, 512] {
        for (scale, suffix) in [(1, ""), (2, "@2x")] {
            let pixels = size * scale;
            let target = iconset.join(format!("icon_{size}x{size}{suffix}.png"));
            let status = Command::new("sips")
                .args(["-z", &pixels.to_string(), &pixels.to_string()])
                .arg(png)
                .arg("--out")
                .arg(&target)
                .output()
                .map_err(|error| format!("no se pudo ejecutar sips: {error}"))?;
            if !status.status.success() {
                return Err(format!(
                    "sips falló con {}: {}",
                    png.display(),
                    String::from_utf8_lossy(&status.stderr).trim()
                ));
            }
        }
    }
    let status = Command::new("iconutil")
        .args(["-c", "icns"])
        .arg(&iconset)
        .arg("-o")
        .arg(icns)
        .output()
        .map_err(|error| format!("no se pudo ejecutar iconutil: {error}"))?;
    let _ = fs::remove_dir_all(&iconset);
    if !status.status.success() {
        return Err(format!(
            "iconutil falló: {}",
            String::from_utf8_lossy(&status.stderr).trim()
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn app() -> App {
        App {
            name: "Kai & Ventas".into(),
            id: "cl.kai.ventas".into(),
            icon: None,
            version: None,
            copyright: None,
            category: None,
            min_macos: None,
        }
    }

    #[test]
    fn the_plist_names_the_app_and_escapes_xml() {
        let plist = info_plist(&app(), "ventas", false);
        assert!(plist.contains("<key>CFBundleIdentifier</key>\n\t<string>cl.kai.ventas</string>"));
        assert!(plist.contains("<string>Kai &amp; Ventas</string>"));
        assert!(plist.contains("<key>CFBundleExecutable</key>\n\t<string>ventas</string>"));
        assert!(plist.contains("<key>CFBundleShortVersionString</key>\n\t<string>0.1.0</string>"));
        assert!(plist.contains("<key>LSMinimumSystemVersion</key>\n\t<string>13.0</string>"));
        assert!(plist.contains("<key>NSHighResolutionCapable</key>\n\t<true/>"));
        assert!(!plist.contains("CFBundleIconFile"));
        assert!(!plist.contains("NSHumanReadableCopyright"));
    }

    #[test]
    fn optional_keys_appear_only_when_set() {
        let mut app = app();
        app.version = Some("2.1.0".into());
        app.copyright = Some("© 2026 Kai".into());
        app.category = Some("public.app-category.business".into());
        app.min_macos = Some("14.0".into());
        let plist = info_plist(&app, "ventas", true);
        assert!(plist.contains("<key>CFBundleIconFile</key>\n\t<string>AppIcon</string>"));
        assert!(plist.contains("<string>2.1.0</string>"));
        assert!(plist.contains("<string>© 2026 Kai</string>"));
        assert!(plist.contains("<string>public.app-category.business</string>"));
        assert!(plist.contains("<key>LSMinimumSystemVersion</key>\n\t<string>14.0</string>"));
    }

    #[test]
    fn the_folder_name_has_no_path_separators() {
        assert_eq!(folder_name("Kai/Ventas: 2"), "Kai-Ventas- 2");
        assert_eq!(folder_name("Kai Ventas"), "Kai Ventas");
    }

    #[test]
    fn the_bundle_has_the_binary_and_the_plist_and_is_replaced_on_rebuild() {
        let root = std::env::temp_dir().join(format!("kai-bundle-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        let binary = root.join("ventas");
        fs::write(&binary, b"#!/bin/sh\n").unwrap();
        let bundle = root.join("Kai Ventas.app");
        assemble(&bundle, &binary, &app(), None).unwrap();
        assert!(bundle.join("Contents/MacOS/ventas").is_file());
        assert!(bundle.join("Contents/Info.plist").is_file());
        assert!(bundle.join("Contents/Resources").is_dir());

        fs::write(bundle.join("Contents/stale"), b"x").unwrap();
        assemble(&bundle, &binary, &app(), None).unwrap();
        assert!(!bundle.join("Contents/stale").exists());
        let _ = fs::remove_dir_all(&root);
    }
}
