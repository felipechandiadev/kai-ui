//! Changes in the watched folders, by polling modification times: no
//! file-system events, so it behaves the same on every platform and over
//! network or synced folders.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

/// Modification time of every file under the watched paths.
pub type Snapshot = BTreeMap<PathBuf, SystemTime>;

/// Walk `roots` (folders or single files). Missing paths are skipped, and
/// so are hidden entries and `target` folders.
pub fn snapshot(roots: &[PathBuf]) -> Snapshot {
    let mut files = Snapshot::new();
    for root in roots {
        visit(root, &mut files);
    }
    files
}

fn visit(path: &Path, files: &mut Snapshot) {
    let Ok(metadata) = std::fs::metadata(path) else {
        return;
    };
    if metadata.is_file() {
        if let Ok(modified) = metadata.modified() {
            files.insert(path.to_owned(), modified);
        }
        return;
    }
    let Ok(entries) = std::fs::read_dir(path) else {
        return;
    };
    for entry in entries.flatten() {
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if name.starts_with('.') || name == "target" {
            continue;
        }
        visit(&entry.path(), files);
    }
}

/// Files created, modified or deleted between two snapshots.
pub fn changes(before: &Snapshot, after: &Snapshot) -> Vec<PathBuf> {
    let mut changed: Vec<PathBuf> = after
        .iter()
        .filter(|(path, modified)| before.get(*path) != Some(modified))
        .map(|(path, _)| path.clone())
        .collect();
    changed.extend(
        before
            .keys()
            .filter(|path| !after.contains_key(*path))
            .cloned(),
    );
    changed.sort();
    changed
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;

    struct TempDir(PathBuf);

    impl TempDir {
        fn new(name: &str) -> Self {
            let path =
                std::env::temp_dir().join(format!("kai-cli-watch-{}-{name}", std::process::id()));
            let _ = std::fs::remove_dir_all(&path);
            std::fs::create_dir_all(&path).unwrap();
            Self(path)
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    /// Give the file a later mtime than any earlier write, whatever the
    /// resolution of the file system.
    fn touch_later(path: &Path) {
        let file = std::fs::File::options().write(true).open(path).unwrap();
        file.set_modified(SystemTime::now() + Duration::from_secs(5))
            .unwrap();
    }

    #[test]
    fn creating_modifying_and_deleting_files_are_changes() {
        let dir = TempDir::new("cambios");
        let src = dir.0.join("src");
        std::fs::create_dir_all(src.join("vistas")).unwrap();
        std::fs::write(src.join("main.rs"), "fn main() {}").unwrap();
        std::fs::write(dir.0.join("Cargo.toml"), "[package]").unwrap();
        let roots = vec![src.clone(), dir.0.join("Cargo.toml"), dir.0.join("assets")];

        let start = snapshot(&roots);
        assert_eq!(start.len(), 2, "the missing assets folder is skipped");
        assert_eq!(changes(&start, &snapshot(&roots)), Vec::<PathBuf>::new());

        let nueva = src.join("vistas/mapa.rs");
        std::fs::write(&nueva, "pub fn vista() {}").unwrap();
        let created = snapshot(&roots);
        assert_eq!(changes(&start, &created), std::slice::from_ref(&nueva));

        touch_later(&src.join("main.rs"));
        let modified = snapshot(&roots);
        assert_eq!(changes(&created, &modified), [src.join("main.rs")]);

        std::fs::remove_file(&nueva).unwrap();
        assert_eq!(changes(&modified, &snapshot(&roots)), [nueva]);
    }

    #[test]
    fn hidden_entries_and_target_folders_are_ignored() {
        let dir = TempDir::new("ignorados");
        std::fs::create_dir_all(dir.0.join("target/debug")).unwrap();
        std::fs::create_dir_all(dir.0.join(".git")).unwrap();
        std::fs::write(dir.0.join("target/debug/app"), "").unwrap();
        std::fs::write(dir.0.join(".git/HEAD"), "").unwrap();
        std::fs::write(dir.0.join(".main.rs.swp"), "").unwrap();
        std::fs::write(dir.0.join("lib.rs"), "").unwrap();
        let files = snapshot(std::slice::from_ref(&dir.0));
        assert_eq!(files.keys().collect::<Vec<_>>(), [&dir.0.join("lib.rs")]);
    }
}
