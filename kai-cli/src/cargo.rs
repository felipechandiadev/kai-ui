//! What `kai dev` asks Cargo: which folders hold the app's sources
//! (`cargo metadata`) and the path of the binary it builds (`cargo build
//! --message-format=json-render-diagnostics`, whose diagnostics Cargo still
//! prints, rendered, on stderr).

use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::process::{Command, Stdio};

use serde::Deserialize;

/// What the loop needs from `cargo metadata`.
#[derive(Debug, PartialEq)]
pub struct Project {
    /// Sources of the package and of its path dependencies.
    pub watch: Vec<PathBuf>,
    pub target_directory: PathBuf,
}

#[derive(Deserialize)]
struct Metadata {
    packages: Vec<Package>,
    target_directory: PathBuf,
}

#[derive(Deserialize)]
struct Package {
    /// `None` for the workspace's packages and path dependencies, the code
    /// being edited; registry and git dependencies have one.
    source: Option<String>,
    manifest_path: PathBuf,
}

/// Run `cargo metadata` in the current folder.
pub fn project() -> Result<Project, String> {
    let output = Command::new(cargo())
        .args(["metadata", "--format-version", "1"])
        .stderr(Stdio::inherit())
        .output()
        .map_err(|error| format!("no se pudo ejecutar cargo: {error}"))?;
    if !output.status.success() {
        return Err("cargo metadata falló; ¿estás en un proyecto de Cargo?".into());
    }
    parse_metadata(&output.stdout)
}

fn parse_metadata(json: &[u8]) -> Result<Project, String> {
    let metadata: Metadata = serde_json::from_slice(json)
        .map_err(|error| format!("cargo metadata devolvió algo inesperado: {error}"))?;
    let mut watch = Vec::new();
    for package in metadata.packages.iter().filter(|p| p.source.is_none()) {
        let Some(root) = package.manifest_path.parent() else {
            continue;
        };
        for entry in ["src", "assets", "build.rs", "Cargo.toml", "kai.toml"] {
            watch.push(root.join(entry));
        }
    }
    watch.sort();
    watch.dedup();
    Ok(Project {
        watch,
        target_directory: metadata.target_directory,
    })
}

pub enum Failure {
    /// The code does not compile; Cargo already printed why.
    Build,
    Other(String),
}

/// Build the package in the current folder and return its binary: `bin`,
/// or the only one it has. `features` are passed as `--features a,b`.
pub fn build(bin: Option<&str>, features: &[String], release: bool) -> Result<PathBuf, Failure> {
    let mut command = Command::new(cargo());
    command.args(["build", "--message-format=json-render-diagnostics"]);
    if release {
        command.arg("--release");
    }
    if let Some(bin) = bin {
        command.args(["--bin", bin]);
    }
    if !features.is_empty() {
        command.args(["--features", &features.join(",")]);
    }
    let mut child = command
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .map_err(|error| Failure::Other(format!("no se pudo ejecutar cargo: {error}")))?;
    let mut binaries = Vec::new();
    if let Some(stdout) = child.stdout.take() {
        for line in BufReader::new(stdout).lines() {
            let Ok(line) = line else { break };
            binaries.extend(executable(&line));
        }
    }
    let status = child
        .wait()
        .map_err(|error| Failure::Other(format!("cargo build: {error}")))?;
    if !status.success() {
        return Err(Failure::Build);
    }
    pick(binaries, bin).map_err(Failure::Other)
}

/// Name and path of the binary in a `compiler-artifact` message.
fn executable(line: &str) -> Option<(String, PathBuf)> {
    #[derive(Deserialize)]
    struct Message {
        reason: String,
        target: Option<Target>,
        executable: Option<PathBuf>,
    }
    #[derive(Deserialize)]
    struct Target {
        name: String,
        kind: Vec<String>,
    }
    let message: Message = serde_json::from_str(line).ok()?;
    let target = message.target?;
    (message.reason == "compiler-artifact" && target.kind.iter().any(|kind| kind == "bin"))
        .then_some(())?;
    Some((target.name, message.executable?))
}

fn pick(binaries: Vec<(String, PathBuf)>, bin: Option<&str>) -> Result<PathBuf, String> {
    let mut binaries: Vec<_> = binaries
        .into_iter()
        .filter(|(name, _)| bin.is_none_or(|bin| bin == name))
        .collect();
    match binaries.len() {
        0 => Err("el paquete no tiene un binario que abrir".into()),
        1 => Ok(binaries.remove(0).1),
        _ => {
            let names: Vec<_> = binaries.iter().map(|(name, _)| name.as_str()).collect();
            Err(format!(
                "hay varios binarios ({}); elige uno con --bin",
                names.join(", ")
            ))
        }
    }
}

fn cargo() -> String {
    std::env::var("CARGO").unwrap_or_else(|_| "cargo".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_project_and_its_path_dependencies_are_watched() {
        let json = br#"{
            "packages": [
                {
                    "name": "dsp-desktop",
                    "source": null,
                    "manifest_path": "/dev/dsp-27/desktop/Cargo.toml"
                },
                {
                    "name": "kai-ui-desktop-prototype",
                    "source": null,
                    "manifest_path": "/dev/kai-ui/kai-ui-desktop/Cargo.toml"
                },
                {
                    "name": "serde",
                    "source": "registry+https://github.com/rust-lang/crates.io-index",
                    "manifest_path": "/home/.cargo/registry/src/serde-1.0.0/Cargo.toml"
                }
            ],
            "target_directory": "/dev/dsp-27/desktop/target",
            "version": 1
        }"#;
        let project = parse_metadata(json).unwrap();
        let rutas: Vec<_> = project
            .watch
            .iter()
            .map(|path| path.to_str().unwrap())
            .collect();
        assert_eq!(
            rutas,
            [
                "/dev/dsp-27/desktop/Cargo.toml",
                "/dev/dsp-27/desktop/assets",
                "/dev/dsp-27/desktop/build.rs",
                "/dev/dsp-27/desktop/kai.toml",
                "/dev/dsp-27/desktop/src",
                "/dev/kai-ui/kai-ui-desktop/Cargo.toml",
                "/dev/kai-ui/kai-ui-desktop/assets",
                "/dev/kai-ui/kai-ui-desktop/build.rs",
                "/dev/kai-ui/kai-ui-desktop/kai.toml",
                "/dev/kai-ui/kai-ui-desktop/src",
            ]
        );
        assert_eq!(
            project.target_directory,
            PathBuf::from("/dev/dsp-27/desktop/target")
        );
        assert!(parse_metadata(b"no es json").is_err());
    }

    #[test]
    fn only_binary_artifacts_have_an_executable() {
        let bin = r#"{"reason":"compiler-artifact","package_id":"path+file:///dev/app#0.1.0","target":{"kind":["bin"],"crate_types":["bin"],"name":"app","src_path":"/dev/app/src/main.rs"},"filenames":["/dev/app/target/debug/app"],"executable":"/dev/app/target/debug/app","fresh":false}"#;
        assert_eq!(
            executable(bin),
            Some(("app".into(), PathBuf::from("/dev/app/target/debug/app")))
        );
        let lib = r#"{"reason":"compiler-artifact","target":{"kind":["lib"],"name":"kai"},"executable":null}"#;
        assert_eq!(executable(lib), None);
        let build_script = r#"{"reason":"compiler-artifact","target":{"kind":["custom-build"],"name":"build-script-build"},"executable":null}"#;
        assert_eq!(executable(build_script), None);
        let finished = r#"{"reason":"build-finished","success":true}"#;
        assert_eq!(executable(finished), None);
        assert_eq!(executable("Compiling app v0.1.0"), None);
    }

    #[test]
    fn the_binary_is_the_only_one_or_the_one_asked_for() {
        let one = || vec![("app".to_string(), PathBuf::from("/t/app"))];
        assert_eq!(pick(one(), None), Ok(PathBuf::from("/t/app")));
        assert_eq!(pick(one(), Some("app")), Ok(PathBuf::from("/t/app")));
        assert!(pick(one(), Some("otra")).is_err());
        assert!(pick(Vec::new(), None).is_err());

        let dos = vec![
            ("app".to_string(), PathBuf::from("/t/app")),
            ("demo".to_string(), PathBuf::from("/t/demo")),
        ];
        assert_eq!(
            pick(dos.clone(), None),
            Err("hay varios binarios (app, demo); elige uno con --bin".into())
        );
        assert_eq!(pick(dos, Some("demo")), Ok(PathBuf::from("/t/demo")));
    }
}
