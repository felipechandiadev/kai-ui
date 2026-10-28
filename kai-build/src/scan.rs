//! Reads `src/sections/` and `src/state.rs` and checks the conventions
//! (docs/dev/26 §5.1). `kai doctor` runs the same checks.

use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

/// A folder name in a route.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Segment {
    /// `customers`, `sales_report`.
    Static(String),
    /// `[id]`: a value of the route.
    Param(String),
}

impl Segment {
    pub fn name(&self) -> &str {
        match self {
            Self::Static(name) | Self::Param(name) => name,
        }
    }
}

/// A `page.rs`: one route.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PageNode {
    /// From the section folder down: `[customers, [id]]`.
    pub segments: Vec<Segment>,
    pub file: PathBuf,
}

impl PageNode {
    /// The section's own page (`sections/<name>/page.rs`).
    pub fn is_section_root(&self) -> bool {
        self.segments.len() == 1
    }

    /// The route's parameter, if it has one.
    pub fn param(&self) -> Option<&str> {
        self.segments.iter().find_map(|segment| match segment {
            Segment::Param(name) => Some(name.as_str()),
            Segment::Static(_) => None,
        })
    }
}

/// A `layout.rs`: the frame around the pages of its folder and below.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LayoutNode {
    /// From the section folder down: `[customers]`, `[customers, new]`.
    pub segments: Vec<Segment>,
    pub file: PathBuf,
}

/// A folder of `src/sections/`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Section {
    pub name: String,
    pub section_file: PathBuf,
    /// Its pages, the section's own first, then by path.
    pub pages: Vec<PageNode>,
    /// Its layouts, the outermost first (docs/dev/28).
    pub layouts: Vec<LayoutNode>,
}

/// What `kai-build` generates the app from.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AppTree {
    pub state_file: PathBuf,
    /// `src/layout.rs`, around everything to the right of the sidebar.
    pub root_layout: Option<PathBuf>,
    /// By folder name.
    pub sections: Vec<Section>,
}

/// A convention not followed, with the file or folder at fault. The
/// message is for the person (Spanish), with what to do.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScanError {
    pub path: PathBuf,
    pub message: String,
}

impl fmt::Display for ScanError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.path.display(), self.message)
    }
}

fn error(path: &Path, message: impl Into<String>) -> ScanError {
    ScanError {
        path: path.to_path_buf(),
        message: message.into(),
    }
}

/// `snake_case` in English letters: a valid Rust module name.
pub fn is_valid_name(name: &str) -> bool {
    let mut chars = name.chars();
    chars.next().is_some_and(|c| c.is_ascii_lowercase())
        && chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
        && !name.ends_with('_')
        && !name.contains("__")
        && !RESERVED.contains(&name)
}

/// Rust keywords and names the generated code uses itself.
const RESERVED: &[&str] = &[
    "as", "break", "const", "crate", "else", "enum", "fn", "for", "if", "impl", "in", "let",
    "loop", "match", "mod", "move", "mut", "pub", "ref", "return", "self", "static", "struct",
    "super", "trait", "type", "use", "where", "while", "async", "await", "dyn", "page", "section",
    "layout", "state", "app",
];

/// Read the app whose `Cargo.toml` is in `app_root`.
pub fn scan(app_root: &Path) -> Result<AppTree, Vec<ScanError>> {
    let src = app_root.join("src");
    let sections_dir = src.join("sections");
    let state_file = src.join("state.rs");
    let mut errors = Vec::new();
    if !state_file.is_file() {
        errors.push(error(
            &state_file,
            "falta: define ahí `pub struct State`, el estado que comparten las páginas",
        ));
    }
    let root_layout = Some(src.join("layout.rs")).filter(|file| file.is_file());
    if let Some(file) = &root_layout {
        check_outlet(file, &mut errors);
    }
    let mut sections = Vec::new();
    match sorted_dirs(&sections_dir) {
        Err(_) => errors.push(error(
            &sections_dir,
            "falta la carpeta: cada sección de la app es una carpeta dentro de src/sections",
        )),
        Ok(dirs) if dirs.is_empty() => errors.push(error(
            &sections_dir,
            "está vacía: crea una sección con `kai new section <nombre>`",
        )),
        Ok(dirs) => {
            for dir in dirs {
                if let Some(section) = scan_section(&dir, &mut errors) {
                    sections.push(section);
                }
            }
        }
    }
    if errors.is_empty() {
        Ok(AppTree {
            state_file,
            root_layout,
            sections,
        })
    } else {
        Err(errors)
    }
}

fn scan_section(dir: &Path, errors: &mut Vec<ScanError>) -> Option<Section> {
    let name = folder_name(dir);
    if !is_valid_name(&name) {
        errors.push(error(
            dir,
            "nombre de sección inválido: usa snake_case en inglés (customers, sales_report)",
        ));
        return None;
    }
    let section_file = dir.join("section.rs");
    if !section_file.is_file() {
        errors.push(error(
            &section_file,
            "falta: cada sección declara `pub const SECTION: SectionMeta` (título, icono, orden, grupo)",
        ));
    }
    if !dir.join("page.rs").is_file() {
        errors.push(error(
            &dir.join("page.rs"),
            "falta: la página propia de la sección, con `pub struct Screen` que implementa Page",
        ));
    }
    let mut pages = Vec::new();
    let mut layouts = Vec::new();
    collect_pages(
        dir,
        vec![Segment::Static(name.clone())],
        &mut pages,
        &mut layouts,
        errors,
    );
    pages.sort_by_key(|page| (page.segments.len(), path_key(&page.segments)));
    layouts.sort_by_key(|layout| (layout.segments.len(), path_key(&layout.segments)));
    Some(Section {
        name,
        section_file,
        pages,
        layouts,
    })
}

fn path_key(segments: &[Segment]) -> String {
    segments
        .iter()
        .map(Segment::name)
        .collect::<Vec<_>>()
        .join("/")
}

fn collect_pages(
    dir: &Path,
    segments: Vec<Segment>,
    pages: &mut Vec<PageNode>,
    layouts: &mut Vec<LayoutNode>,
    errors: &mut Vec<ScanError>,
) {
    let page = dir.join("page.rs");
    if page.is_file() {
        pages.push(PageNode {
            segments: segments.clone(),
            file: page,
        });
    }
    let layout = dir.join("layout.rs");
    if layout.is_file() {
        if segments.iter().any(|s| matches!(s, Segment::Param(_))) {
            errors.push(error(
                &layout,
                "un layout dentro de una carpeta [parámetro] aún no se admite: ponlo en la carpeta de arriba",
            ));
        } else {
            check_outlet(&layout, errors);
            layouts.push(LayoutNode {
                segments: segments.clone(),
                file: layout,
            });
        }
    }
    let Ok(children) = sorted_dirs(dir) else {
        return;
    };
    let params: Vec<&PathBuf> = children
        .iter()
        .filter(|child| folder_name(child).starts_with('['))
        .collect();
    if params.len() > 1 {
        errors.push(error(
            dir,
            "tiene más de una carpeta de parámetro ([...]): una ruta no sabría cuál elegir",
        ));
    }
    for child in &children {
        let name = folder_name(child);
        let segment = match name.strip_prefix('[').and_then(|n| n.strip_suffix(']')) {
            Some(param) if is_valid_name(param) => {
                if segments.iter().any(|s| matches!(s, Segment::Param(_))) {
                    errors.push(error(child, "una ruta admite un solo parámetro por ahora"));
                    continue;
                }
                Segment::Param(param.to_owned())
            }
            Some(_) => {
                errors.push(error(
                    child,
                    "parámetro inválido: usa [snake_case] en inglés, como [id] o [customer_id]",
                ));
                continue;
            }
            None if is_valid_name(&name) => Segment::Static(name),
            None => {
                errors.push(error(
                    child,
                    "nombre de carpeta inválido: usa snake_case en inglés",
                ));
                continue;
            }
        };
        let mut below = segments.clone();
        below.push(segment);
        let before = pages.len();
        collect_pages(child, below, pages, layouts, errors);
        if pages.len() == before {
            errors.push(error(
                child,
                "no tiene ninguna page.rs: borra la carpeta o agrega su página",
            ));
        }
    }
}

/// A layout without `outlet()` would hide its pages.
fn check_outlet(file: &Path, errors: &mut Vec<ScanError>) {
    let has_outlet = fs::read_to_string(file).is_ok_and(|code| code.contains("outlet("));
    if !has_outlet {
        errors.push(error(
            file,
            "falta `outlet()` en su vista: es el hueco donde va el contenido (docs/dev/28)",
        ));
    }
}

fn folder_name(dir: &Path) -> String {
    dir.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default()
}

/// Subfolders of `dir`, by name (not in the order the file system lists
/// them, so the output is the same on every machine).
fn sorted_dirs(dir: &Path) -> std::io::Result<Vec<PathBuf>> {
    let mut dirs: Vec<PathBuf> = fs::read_dir(dir)?
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.is_dir())
        .filter(|path| !folder_name(path).starts_with('.'))
        .collect();
    dirs.sort();
    Ok(dirs)
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::{Path, PathBuf};

    use super::{Segment, is_valid_name, scan};

    pub(crate) fn temp_app(name: &str, files: &[&str]) -> PathBuf {
        let root = std::env::temp_dir().join(format!("kai-build-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        for file in files {
            let path = root.join(file);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, "").unwrap();
        }
        root
    }

    fn messages(root: &Path) -> Vec<String> {
        scan(root)
            .unwrap_err()
            .iter()
            .map(|e| {
                let path = e.path.strip_prefix(root).unwrap_or(&e.path);
                format!(
                    "{}: {}",
                    path.display(),
                    e.message.split(':').next().unwrap()
                )
            })
            .collect()
    }

    #[test]
    fn reads_sections_pages_and_parameters_in_a_stable_order() {
        let root = temp_app(
            "ok",
            &[
                "src/state.rs",
                "src/sections/sales/section.rs",
                "src/sections/sales/page.rs",
                "src/sections/customers/section.rs",
                "src/sections/customers/page.rs",
                "src/sections/customers/[id]/page.rs",
                "src/sections/customers/new/page.rs",
            ],
        );
        let tree = scan(&root).unwrap();
        let names: Vec<_> = tree.sections.iter().map(|s| s.name.as_str()).collect();
        assert_eq!(names, ["customers", "sales"]);
        let paths: Vec<Vec<String>> = tree.sections[0]
            .pages
            .iter()
            .map(|page| page.segments.iter().map(|s| s.name().to_owned()).collect())
            .collect();
        assert_eq!(
            paths,
            [
                vec!["customers"],
                vec!["customers", "id"],
                vec!["customers", "new"]
            ]
        );
        assert_eq!(
            tree.sections[0].pages[1].segments[1],
            Segment::Param("id".into())
        );
        assert_eq!(tree.sections[0].pages[1].param(), Some("id"));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn says_what_is_missing_and_what_to_do() {
        let root = temp_app(
            "missing",
            &[
                "src/sections/customers/page.rs",
                "src/sections/Ventas/page.rs",
                "src/sections/customers/[Id]/page.rs",
                "src/sections/customers/empty/notes.md",
            ],
        );
        assert_eq!(
            messages(&root),
            [
                "src/state.rs: falta",
                "src/sections/Ventas: nombre de sección inválido",
                "src/sections/customers/section.rs: falta",
                "src/sections/customers/[Id]: parámetro inválido",
                "src/sections/customers/empty: no tiene ninguna page.rs",
            ]
        );
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn one_parameter_per_route_and_per_folder() {
        let root = temp_app(
            "params",
            &[
                "src/state.rs",
                "src/sections/customers/section.rs",
                "src/sections/customers/page.rs",
                "src/sections/customers/[id]/page.rs",
                "src/sections/customers/[id]/[invoice]/page.rs",
                "src/sections/customers/[name]/page.rs",
            ],
        );
        assert_eq!(
            messages(&root),
            [
                "src/sections/customers: tiene más de una carpeta de parámetro ([...])",
                "src/sections/customers/[id]/[invoice]: una ruta admite un solo parámetro por ahora",
            ]
            .map(|m| m.split(':').take(2).collect::<Vec<_>>().join(":"))
        );
        let _ = fs::remove_dir_all(root);
    }

    fn write(root: &Path, file: &str, code: &str) {
        let path = root.join(file);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, code).unwrap();
    }

    const FRAME: &str = "fn view() { outlet() }";

    #[test]
    fn reads_the_root_layout_and_the_layouts_of_each_folder() {
        let root = temp_app(
            "layouts",
            &[
                "src/state.rs",
                "src/sections/customers/section.rs",
                "src/sections/customers/page.rs",
                "src/sections/customers/new/page.rs",
                "src/sections/sales/section.rs",
                "src/sections/sales/page.rs",
            ],
        );
        write(&root, "src/layout.rs", FRAME);
        write(&root, "src/sections/customers/layout.rs", FRAME);
        write(&root, "src/sections/customers/new/layout.rs", FRAME);
        let tree = scan(&root).unwrap();
        assert_eq!(tree.root_layout, Some(root.join("src/layout.rs")));
        let layouts: Vec<Vec<String>> = tree.sections[0]
            .layouts
            .iter()
            .map(|l| l.segments.iter().map(|s| s.name().to_owned()).collect())
            .collect();
        assert_eq!(layouts, [vec!["customers"], vec!["customers", "new"]]);
        assert!(tree.sections[1].layouts.is_empty());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn a_layout_needs_its_outlet_and_cannot_sit_in_a_parameter_folder() {
        let root = temp_app(
            "layout-errors",
            &[
                "src/state.rs",
                "src/sections/customers/section.rs",
                "src/sections/customers/page.rs",
                "src/sections/customers/[id]/page.rs",
            ],
        );
        write(&root, "src/layout.rs", "fn view() {}");
        write(&root, "src/sections/customers/layout.rs", FRAME);
        write(&root, "src/sections/customers/[id]/layout.rs", FRAME);
        assert_eq!(
            messages(&root),
            [
                "src/layout.rs: falta `outlet()` en su vista",
                "src/sections/customers/[id]/layout.rs: un layout dentro de una carpeta [parámetro] aún no se admite",
            ]
        );
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn names_are_snake_case_english_and_not_reserved() {
        assert!(is_valid_name("customers"));
        assert!(is_valid_name("sales_report2"));
        for bad in [
            "Customers",
            "sales-report",
            "_x",
            "x_",
            "a__b",
            "año",
            "page",
            "layout",
            "fn",
            "",
        ] {
            assert!(!is_valid_name(bad), "{bad}");
        }
    }
}
