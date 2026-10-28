//! `kai notarize`: Apple's notary service and `stapler` (13, Paso 20).
//!
//! Needs a bundle signed with a Developer ID (`kai sign --identity …`) and a
//! `notarytool` profile in the keychain (`xcrun notarytool store-credentials`).
//! Credentials never go in `kai.toml` nor on this command's line.

use std::path::PathBuf;
use std::process::Command;

use crate::bundle;
use crate::manifest;

#[derive(Debug, Default, PartialEq)]
pub struct Options {
    /// Name given to `notarytool store-credentials`.
    pub profile: Option<String>,
    pub bundle: Option<PathBuf>,
}

pub fn run(options: &Options) -> Result<(), String> {
    if !cfg!(target_os = "macos") {
        return Err("kai notarize solo funciona en macOS".into());
    }
    let profile = options.profile.as_deref().ok_or(
        "falta --profile NOMBRE (el perfil que guardaste con xcrun notarytool store-credentials)",
    )?;
    let loaded = manifest::load_from_cwd()?;
    let bundle = match &options.bundle {
        Some(path) => path.clone(),
        None => bundle::bundle_path(&loaded.manifest.app)?,
    };
    if !bundle.is_dir() {
        return Err(format!(
            "no existe {}; ejecuta kai bundle y kai sign antes",
            bundle.display()
        ));
    }
    let signature = output("codesign", &["-dv".as_ref(), bundle.as_os_str()], true)?;
    check_signature(&signature)?;

    let zip = bundle.with_extension("zip");
    let _ = std::fs::remove_file(&zip);
    output(
        "ditto",
        &[
            "-c".as_ref(),
            "-k".as_ref(),
            "--keepParent".as_ref(),
            bundle.as_os_str(),
            zip.as_os_str(),
        ],
        false,
    )?;
    eprintln!("kai notarize: enviando a Apple (puede tardar unos minutos)…");
    let submitted = output(
        "xcrun",
        &[
            "notarytool".as_ref(),
            "submit".as_ref(),
            zip.as_os_str(),
            "--keychain-profile".as_ref(),
            profile.as_ref(),
            "--wait".as_ref(),
        ],
        false,
    );
    let _ = std::fs::remove_file(&zip);
    let log = submitted?;
    if !accepted(&log) {
        return Err(format!(
            "Apple no aceptó la app; revisa con xcrun notarytool log:\n{log}"
        ));
    }
    output(
        "xcrun",
        &["stapler".as_ref(), "staple".as_ref(), bundle.as_os_str()],
        false,
    )?;
    eprintln!("kai notarize: aceptada y grapada: {}", bundle.display());
    Ok(())
}

/// Ad hoc signatures, or none, can't be notarized.
fn check_signature(details: &str) -> Result<(), String> {
    if details.contains("Signature=adhoc") || !details.contains("Authority=Developer ID") {
        return Err(
            "la app no está firmada con un Developer ID; ejecuta kai sign --identity \"Developer ID Application: …\""
                .into(),
        );
    }
    if !details.contains("flags=0x10000(runtime)") {
        return Err(
            "la firma no usa el hardened runtime ([macos].hardened-runtime = true) y Apple la rechazaría"
                .into(),
        );
    }
    Ok(())
}

fn accepted(log: &str) -> bool {
    log.lines().any(|line| line.trim() == "status: Accepted")
}

/// Run a tool; its stdout, or stderr when `codesign -d`-style tools use it.
fn output(program: &str, args: &[&std::ffi::OsStr], stderr: bool) -> Result<String, String> {
    let result = Command::new(program)
        .args(args)
        .output()
        .map_err(|error| format!("no se pudo ejecutar {program}: {error}"))?;
    let out = String::from_utf8_lossy(&result.stdout).into_owned();
    let err = String::from_utf8_lossy(&result.stderr).into_owned();
    if !result.status.success() {
        return Err(format!("{program} falló: {}", err.trim()));
    }
    Ok(if stderr { err } else { out })
}

#[cfg(test)]
mod tests {
    use super::*;

    const GOOD: &str = "Authority=Developer ID Application: Kai (ABCDE12345)\nCodeDirectory v=20500 size=1 flags=0x10000(runtime) hashes=1\n";

    #[test]
    fn only_a_developer_id_with_the_runtime_passes() {
        assert!(check_signature(GOOD).is_ok());
        assert!(check_signature("Signature=adhoc\nflags=0x2(adhoc)").is_err());
        assert!(
            check_signature("Authority=Developer ID Application: Kai\nflags=0x0(none)").is_err()
        );
        assert!(check_signature("code object is not signed at all").is_err());
    }

    #[test]
    fn only_an_accepted_submission_is_good() {
        assert!(accepted("  id: abc\n  status: Accepted\n"));
        assert!(!accepted("  status: Invalid\n"));
        assert!(!accepted("status: Accepted but not really? no"));
    }
}
