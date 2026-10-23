//! Ribbon Kai del documento, alimentado por el catálogo WordCraft (`CommandSpec`).
//!
//! No porta `wordcraft-ui-egui`: solo lee `id` / `location` / `shortcut`
//! (`Session::why_disabled` + `format.state` / párrafo para pressed) del engine y pinta UI Kai
//! estilo toolbar AppKit: iconos + tooltip en español, labels de grupo
//! discretos y separadores verticales.
//!
//! Diseño: [`docs/dev/17-document-ribbon.md`](../../docs/dev/17-document-ribbon.md).

use std::collections::BTreeMap;
use std::sync::OnceLock;

use wordcraft_engine::{CommandSpec, Registry, Session};

use crate::components::{
    button_sized, themed, toolbar_item, toolbar_separator, ButtonSize, ButtonVariant,
};
use crate::element::{div, text, Element, Role};
use crate::icons::Icon;

use super::DocumentCmd;

/// Qué pestañas del catálogo WordCraft expone el ribbon.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum RibbonMode {
    /// Sin chrome de ribbon.
    None,
    /// Solo Home (font / paragraph / clipboard / undo).
    HomeOnly,
    /// Home + Insert (MVP original).
    HomeInsert,
    /// Home + Insert + Layout + Review + References + View.
    #[default]
    Full,
}

/// Ítem clicable: un `CommandSpec.id` WordCraft.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RibbonItem {
    pub cmd_id: String,
    /// Etiqueta UI (español); el id interno sigue en inglés.
    pub label: String,
    pub shortcut: String,
}

/// Grupo bajo una pestaña (`Inicio › Fuente` → "Fuente").
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RibbonGroup {
    pub label: String,
    pub items: Vec<RibbonItem>,
}

/// Pestaña del ribbon (`home`, `insert`, …).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RibbonTab {
    pub id: String,
    pub label: String,
    pub groups: Vec<RibbonGroup>,
}

/// Allowlist MVP: ids a mostrar. Labels/grupos salen del `CommandSpec` + ES.
const MVP_HOME: &[&str] = &[
    "edit.cut",
    "edit.copy",
    "edit.paste",
    "edit.pasteText",
    "edit.pasteMerge",
    "edit.formatPainter",
    "edit.undo",
    "edit.redo",
    "format.bold",
    "format.italic",
    "format.underline",
    "format.strikethrough",
    "format.growFont",
    "format.shrinkFont",
    "format.clear",
    "para.bullets",
    "para.numbering",
    "para.alignLeft",
    "para.alignCenter",
    "para.alignRight",
    "para.justify",
    "para.normal",
    "para.heading1",
    "para.heading2",
    "para.heading3",
    "select.all",
];

const MVP_INSERT: &[&str] = &[
    "insert.table",
    "insert.picture",
    "insert.pageBreak",
    "insert.header",
    "insert.footer",
    "insert.pageNumber",
    "insert.editHeader",
    "insert.editFooter",
    "insert.closeHeader",
];

/// Layout: Page Setup / Breaks que corren con `{}` o params por defecto (ver
/// [`default_run_params`]). Excluye `layout.pageSetup` (abre dialog WC-UI) y
/// Arrange (requiere selección de objeto).
const MVP_LAYOUT: &[&str] = &[
    "layout.margins",
    "layout.orientation",
    "layout.size",
    "layout.columns",
    "layout.hyphenation",
    "layout.lineNumbers",
    "layout.textDirection",
    "layout.sectionRtl",
    "text.columnBreak",
    "layout.break",
    "arrange.wrap",
    "arrange.bringForward",
    "arrange.sendBackward",
];

/// Table Design/Layout mínimo (requiere caret en tabla; `why_disabled` greya).
const MVP_TABLE: &[&str] = &[
    "table.style",
    "table.look",
    "table.shading",
    "table.borders",
    "table.selectTable",
];

/// Review: proofing / tracking / changes sin dialogs de archivo.
/// `review.newComment` exige `text` → barra de comentarios de la demo
/// ([`DocumentCmd::NewComment`]), no el ribbon. Excluye compare/combine,
/// restrict, speech UI.
const MVP_REVIEW: &[&str] = &[
    "review.spelling",
    "review.proofing",
    "review.wordCount",
    "review.trackChanges",
    "review.markup",
    "review.nextChange",
    "review.previousChange",
    "review.accept",
    "review.reject",
    "review.acceptAll",
    "review.rejectAll",
    "review.nextComment",
    "review.previousComment",
];

/// References: TOC / footnotes mínimos. Excluye citations/sources/researcher
/// (params de fuentes o UI de investigación).
const MVP_REFERENCES: &[&str] = &[
    "references.toc",
    "references.updateToc",
    "references.removeToc",
    "references.footnote",
    "references.endnote",
    "references.nextFootnote",
];

/// View: zoom / vistas / show toggles. Excluye window/macros/organizer y
/// `view.interfaceTheme` (exige value); darkMode queda (toggle).
const MVP_VIEW: &[&str] = &[
    "view.printLayout",
    "view.webLayout",
    "view.draft",
    "view.ruler",
    "view.gridlines",
    "view.navigationPane",
    "view.hideWhiteSpace",
    "view.zoomIn",
    "view.zoomOut",
    "view.zoom100",
    "view.onePage",
    "view.pageWidth",
    "view.multiplePages",
    "view.darkMode",
];

fn wc_registry() -> &'static Registry {
    static REG: OnceLock<Registry> = OnceLock::new();
    REG.get_or_init(wordcraft_engine::cmd::registry)
}

/// Tabs construidos desde el Registry WordCraft según [`RibbonMode`].
pub fn ribbon_catalog(mode: RibbonMode) -> Vec<RibbonTab> {
    match mode {
        RibbonMode::None => Vec::new(),
        RibbonMode::HomeOnly => build_tabs(&[("Home", MVP_HOME)]),
        RibbonMode::HomeInsert => build_tabs(&[("Home", MVP_HOME), ("Insert", MVP_INSERT)]),
        RibbonMode::Full => build_tabs(&[
            ("Home", MVP_HOME),
            ("Insert", MVP_INSERT),
            ("Layout", MVP_LAYOUT),
            ("Table", MVP_TABLE),
            ("Review", MVP_REVIEW),
            ("References", MVP_REFERENCES),
            ("View", MVP_VIEW),
        ]),
    }
}

fn build_tabs(want: &[(&str, &[&str])]) -> Vec<RibbonTab> {
    let reg = wc_registry();
    let mut out = Vec::new();
    for &(tab_label, ids) in want {
        let mut groups: BTreeMap<String, Vec<RibbonItem>> = BTreeMap::new();
        for id in ids {
            let Some(spec) = reg.get(id) else {
                continue;
            };
            let group = group_for_tab(tab_label, spec);
            groups
                .entry(group)
                .or_default()
                .push(item_from_spec(spec));
        }
        // Orden de grupos preferido en Home.
        let order = preferred_group_order(tab_label);
        let mut group_list: Vec<RibbonGroup> = Vec::new();
        for name in &order {
            if let Some(items) = groups.remove(*name) {
                group_list.push(RibbonGroup {
                    label: localize_group(name),
                    items,
                });
            }
        }
        for (label, items) in groups {
            group_list.push(RibbonGroup {
                label: localize_group(&label),
                items,
            });
        }
        if group_list.is_empty() {
            continue;
        }
        out.push(RibbonTab {
            id: tab_label.to_ascii_lowercase(),
            label: localize_tab(tab_label),
            groups: group_list,
        });
    }
    out
}

/// Grupo UI para un spec bajo la pestaña pedida (QAT undo/redo → Editing).
fn group_for_tab(tab_label: &str, spec: &CommandSpec) -> String {
    if matches!(spec.id, "edit.undo" | "edit.redo") && tab_label == "Home" {
        return "Editing".into();
    }
    let (tab, group) = parse_location(spec.location);
    if tab.eq_ignore_ascii_case(tab_label) {
        if group.is_empty() {
            "General".into()
        } else {
            group
        }
    } else if spec.location.eq_ignore_ascii_case("Quick Access Toolbar") && tab_label == "Home"
    {
        "Editing".into()
    } else {
        // Allowlist id pertenece a otra location: aún así muéstralo en General.
        "General".into()
    }
}

fn preferred_group_order(tab: &str) -> Vec<&'static str> {
    match tab {
        "Home" => vec![
            "Clipboard",
            "Font",
            "Paragraph",
            "Editing",
            "Styles",
        ],
        "Insert" => vec!["Pages", "Tables", "Illustrations", "Header & Footer", "Text", "Links"],
        "Layout" => vec!["Page Setup", "Breaks", "Paragraph", "Arrange"],
        "Review" => vec!["Proofing", "Tracking", "Changes", "Comments"],
        "References" => vec!["Table of Contents", "Footnotes", "Captions", "Citations & Bibliography"],
        "View" => vec!["Views", "Show", "Zoom", "Dark Mode", "Immersive", "Page Movement"],
        "Table" => vec!["Table Styles", "Table Style Options", "Borders"],
        _ => Vec::new(),
    }
}

fn item_from_spec(spec: &CommandSpec) -> RibbonItem {
    RibbonItem {
        cmd_id: spec.id.into(),
        label: localize_cmd(spec.id).unwrap_or(spec.label).into(),
        shortcut: spec.shortcut.into(),
    }
}

/// `Home › Font › Underline` → (`Home`, `Font`).
pub fn parse_location(location: &str) -> (String, String) {
    let parts: Vec<&str> = location
        .split('›')
        .map(str::trim)
        .filter(|p| !p.is_empty())
        .collect();
    match parts.as_slice() {
        [] => (String::new(), String::new()),
        [tab] => ((*tab).into(), String::new()),
        [tab, group, ..] => ((*tab).into(), (*group).into()),
    }
}

/// Mapea un id WordCraft al [`DocumentCmd`] que la Session Kai ya sabe despachar
/// (params demo para tabla/imagen).
pub fn document_cmd_for(id: &str) -> DocumentCmd {
    match id {
        "format.bold" => DocumentCmd::Bold,
        "format.italic" => DocumentCmd::Italic,
        "para.bullets" => DocumentCmd::Bullets,
        "edit.undo" => DocumentCmd::Undo,
        "edit.redo" => DocumentCmd::Redo,
        "insert.table" => DocumentCmd::InsertTable,
        "insert.picture" => DocumentCmd::InsertPicture,
        "edit.pasteText" => DocumentCmd::PastePlain,
        "edit.pasteMerge" => DocumentCmd::PasteMerge,
        "edit.paste" => DocumentCmd::PasteKeepSource,
        "arrange.wrap" => DocumentCmd::Wrap {
            mode: "square".into(),
        },
        "table.style" => DocumentCmd::TableStyle {
            style: "GridTable1Light".into(),
        },
        other => DocumentCmd::Raw(other.into()),
    }
}

/// Params JSON por defecto al ejecutar un id desde el ribbon (sin dialogs).
///
/// La mayoría usa `{}`. Algunos cmds de Layout exigen un preset/`kind` mínimo
/// para no fallar con Params; nunca abren UI egui/rfd.
pub fn default_run_params(id: &str) -> serde_json::Value {
    use serde_json::json;
    match id {
        "layout.margins" => json!({ "preset": "normal" }),
        "layout.columns" => json!({ "count": 2 }),
        "layout.break" => json!({ "kind": "page" }),
        "layout.size" => json!({ "name": "Letter" }),
        "insert.header" => json!({ "preset": "blank" }),
        "insert.footer" => json!({ "preset": "pageNumber" }),
        "insert.pageNumber" => json!({ "position": "bottom", "align": "center" }),
        "arrange.wrap" => json!({ "wrap": "square" }),
        "table.style" => json!({ "style": "GridTable1Light" }),
        "table.look" => json!({ "bandedRows": true, "headerRow": true }),
        "table.shading" => json!({ "color": "D9E2F3" }),
        "table.borders" => json!({ "kind": "all", "width": 0.5, "color": "000000" }),
        _ => json!({}),
    }
}

/// ¿El comando está greyed-out en esta Session? (`Session::why_disabled`).
#[allow(dead_code)] // API pública para apps que enrutan Session al chrome.
pub fn cmd_disabled(session: &Session, id: &str) -> bool {
    session.why_disabled(id).is_some()
}

/// Estado vivo del ribbon (pressed / disabled) derivado de la Session.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RibbonChrome {
    pub pressed: Vec<String>,
    pub disabled: Vec<String>,
}


/// Calcula pressed (desde `format.state` + párrafo/caret + view) y disabled
/// (`Session::why_disabled`) para el catálogo del `mode`.
pub fn ribbon_chrome_for(session: &mut Session, mode: RibbonMode) -> RibbonChrome {
    let mut pressed: Vec<String> = Vec::new();
    if let Ok(state) = session.run("format.state", &serde_json::json!({})) {
        let on = |key: &str| state.get(key).and_then(|v| v.as_bool()) == Some(true);
        if on("bold") {
            pressed.push("format.bold".into());
        }
        if on("italic") {
            pressed.push("format.italic".into());
        }
        if on("underline") {
            pressed.push("format.underline".into());
        }
        if on("strike") {
            pressed.push("format.strikethrough".into());
        }
        if on("subscript") {
            pressed.push("format.subscript".into());
        }
        if on("superscript") {
            pressed.push("format.superscript".into());
        }
        // Gallery mínima de estilos de párrafo.
        match state.get("style").and_then(|v| v.as_str()).unwrap_or("Normal") {
            "Normal" => pressed.push("para.normal".into()),
            "Heading1" => pressed.push("para.heading1".into()),
            "Heading2" => pressed.push("para.heading2".into()),
            "Heading3" => pressed.push("para.heading3".into()),
            _ => {}
        }
    }

    let focus = session.sel.focus.clone();
    if let Some(para) = session.doc.para_at(&focus) {
        let resolved = session.doc.styles.resolve_para(&para.props);
        let visual = resolved.align.visual(resolved.bidi);
        match visual {
            wordcraft_doc::props::Align::Left => pressed.push("para.alignLeft".into()),
            wordcraft_doc::props::Align::Center => pressed.push("para.alignCenter".into()),
            wordcraft_doc::props::Align::Right => pressed.push("para.alignRight".into()),
            wordcraft_doc::props::Align::Justify => pressed.push("para.justify".into()),
            wordcraft_doc::props::Align::Distribute => {}
        }
        if let Some(n) = resolved.numbering.filter(|n| n.num != 0) {
            if let Some(level) = session.doc.numbering.level(n.num, n.level) {
                use wordcraft_doc::section::NumFormat;
                if level.format == NumFormat::Bullet {
                    pressed.push("para.bullets".into());
                } else if level.format != NumFormat::None {
                    pressed.push("para.numbering".into());
                }
            }
        }
    }

    if session.view.dark_mode {
        pressed.push("view.darkMode".into());
    }

    if session.view.proofing {
        pressed.push("review.proofing".into());
    }

    if session.painter.is_some() {
        pressed.push("edit.formatPainter".into());
    }

    // Solo ids que el catálogo puede mostrar (evita pressed huérfanos).
    let catalog = ribbon_catalog(mode);
    let catalog_ids: std::collections::BTreeSet<&str> = catalog
        .iter()
        .flat_map(|t| t.groups.iter())
        .flat_map(|g| g.items.iter())
        .map(|i| i.cmd_id.as_str())
        .collect();
    pressed.retain(|id| catalog_ids.contains(id.as_str()));
    pressed.sort();
    pressed.dedup();

    let mut disabled = Vec::new();
    for id in &catalog_ids {
        if cmd_disabled(session, id) {
            disabled.push((*id).to_string());
        }
    }
    disabled.sort();
    RibbonChrome { pressed, disabled }
}


/// Icono Lucide para un id WordCraft, si hay uno razonable en el catálogo Kai.
pub fn icon_for_cmd(id: &str) -> Option<Icon> {
    Some(match id {
        // Clipboard / edit
        "edit.cut" => Icon::Scissors,
        "edit.copy" => Icon::Copy,
        "edit.paste" => Icon::ClipboardPaste,
        "edit.pasteText" => Icon::Type,
        "edit.pasteMerge" => Icon::Clipboard,
        "edit.formatPainter" => Icon::Highlighter,
        "edit.undo" => Icon::Undo,
        "edit.redo" => Icon::Redo,
        // Font
        "format.bold" => Icon::Bold,
        "format.italic" => Icon::Italic,
        "format.underline" => Icon::Underline,
        "format.strikethrough" => Icon::Strikethrough,
        "format.growFont" => Icon::AArrowUp,
        "format.shrinkFont" => Icon::AArrowDown,
        "format.clear" => Icon::RemoveFormatting,
        // Paragraph
        "para.bullets" => Icon::List,
        "para.numbering" => Icon::ListOrdered,
        "para.alignLeft" => Icon::AlignLeft,
        "para.alignCenter" => Icon::AlignCenter,
        "para.alignRight" => Icon::AlignRight,
        "para.justify" => Icon::AlignJustify,
        "para.normal" => Icon::Type,
        "para.heading1" => Icon::BookType,
        "para.heading2" => Icon::BookMarked,
        "para.heading3" => Icon::Baseline,
        "select.all" => Icon::Baseline,
        // Insert
        "insert.table" => Icon::Table,
        "insert.picture" => Icon::ImagePlus,
        "arrange.wrap" => Icon::Layers,
        "arrange.bringForward" => Icon::ArrowUp,
        "arrange.sendBackward" => Icon::ArrowDown,
        "table.style" => Icon::Table,
        "table.look" => Icon::Settings,
        "table.shading" => Icon::Highlighter,
        "table.borders" => Icon::Grid3x3,
        "table.selectTable" => Icon::Table,
        "insert.pageBreak" => Icon::SeparatorHorizontal,
        "insert.header" => Icon::PanelLeft,
        "insert.footer" => Icon::LayoutDashboard,
        "insert.pageNumber" => Icon::Hash,
        "insert.editHeader" => Icon::PanelLeft,
        "insert.editFooter" => Icon::LayoutDashboard,
        "insert.closeHeader" => Icon::XCircle,
        // Layout
        "layout.margins" => Icon::Expand,
        "layout.orientation" => Icon::RectangleHorizontal,
        "layout.size" => Icon::Maximize,
        "layout.columns" => Icon::Columns3,
        "layout.hyphenation" => Icon::Type,
        "layout.lineNumbers" => Icon::Hash,
        "layout.textDirection" => Icon::AlignLeft,
        "layout.sectionRtl" => Icon::CornerDownLeft,
        "text.columnBreak" => Icon::Columns3,
        "layout.break" => Icon::SeparatorHorizontal,
        // Review
        "review.spelling" => Icon::SpellCheck2,
        "review.proofing" => Icon::SpellCheck2,
        "review.wordCount" => Icon::ScanText,
        "review.trackChanges" => Icon::Highlighter,
        "review.markup" => Icon::Eye,
        "review.nextChange" => Icon::ChevronRight,
        "review.previousChange" => Icon::CornerDownLeft,
        "review.accept" => Icon::Check,
        "review.reject" => Icon::X,
        "review.acceptAll" => Icon::CheckCheck,
        "review.rejectAll" => Icon::Ban,
        "review.nextComment" => Icon::MessageSquare,
        "review.previousComment" => Icon::Reply,
        // References
        "references.toc" => Icon::ListTodo,
        "references.updateToc" => Icon::Redo,
        "references.removeToc" => Icon::Trash2,
        "references.footnote" => Icon::BookMarked,
        "references.endnote" => Icon::BookType,
        "references.nextFootnote" => Icon::ChevronRight,
        // View
        "view.printLayout" => Icon::Printer,
        "view.webLayout" => Icon::LayoutTemplate,
        "view.draft" => Icon::FileText,
        "view.ruler" => Icon::Ruler,
        "view.gridlines" => Icon::Grid3x3,
        "view.navigationPane" => Icon::PanelLeft,
        "view.hideWhiteSpace" => Icon::Minimize,
        "view.zoomIn" => Icon::ZoomIn,
        "view.zoomOut" => Icon::ZoomOut,
        "view.zoom100" => Icon::Maximize,
        "view.onePage" => Icon::Sheet,
        "view.pageWidth" => Icon::RectangleHorizontal,
        "view.multiplePages" => Icon::Files,
        "view.darkMode" => Icon::Moon,
        // File QAT (demo)
        "file.open" => Icon::FolderOpen,
        "file.save" => Icon::Save,
        "file.exportPdf" => Icon::FileDown,
        _ => return None,
    })
}

fn localize_tab(en: &str) -> String {
    match en {
        "Home" => "Inicio",
        "Insert" => "Insertar",
        "Layout" => "Diseño",
        "Review" => "Revisar",
        "References" => "Referencias",
        "View" => "Vista",
        other => other,
    }
    .to_string()
}

fn localize_group(en: &str) -> String {
    match en {
        "Clipboard" => "Portapapeles",
        "Font" => "Fuente",
        "Paragraph" => "Párrafo",
        "Editing" => "Edición",
        "Styles" => "Estilos",
        "Pages" => "Páginas",
        "Tables" => "Tablas",
        "Illustrations" => "Ilustraciones",
        "Text" => "Texto",
        "Links" => "Vínculos",
        "Page Setup" => "Configurar página",
        "Breaks" => "Saltos",
        "Arrange" => "Organizar",
        "Proofing" => "Revisión",
        "Tracking" => "Seguimiento",
        "Changes" => "Cambios",
        "Comments" => "Comentarios",
        "Table of Contents" => "Tabla de contenido",
        "Footnotes" => "Notas al pie",
        "Captions" => "Títulos",
        "Citations & Bibliography" => "Citas y bibliografía",
        "Views" => "Vistas",
        "Show" => "Mostrar",
        "Zoom" => "Zoom",
        "Dark Mode" => "Modo oscuro",
        "Immersive" => "Inmersivo",
        "Page Movement" => "Movimiento de página",
        "General" => "General",
        other => other,
    }
    .to_string()
}

/// Etiqueta corta en español para tooltip / accesibilidad / fallback de texto.
pub fn localize_cmd(id: &str) -> Option<&'static str> {
    Some(match id {
        "edit.cut" => "Cortar",
        "edit.copy" => "Copiar",
        "edit.paste" => "Pegar",
        "edit.pasteText" => "Pegar texto",
        "edit.pasteMerge" => "Pegar combinar",
        "edit.formatPainter" => "Copiar formato",
        "edit.undo" => "Deshacer",
        "edit.redo" => "Rehacer",
        "format.bold" => "Negrita",
        "format.italic" => "Cursiva",
        "format.underline" => "Subrayado",
        "format.strikethrough" => "Tachado",
        "format.growFont" => "Aumentar tamaño",
        "format.shrinkFont" => "Reducir tamaño",
        "format.clear" => "Borrar formato",
        "para.bullets" => "Viñetas",
        "para.numbering" => "Numeración",
        "para.alignLeft" => "Alinear a la izquierda",
        "para.alignCenter" => "Centrar",
        "para.alignRight" => "Alinear a la derecha",
        "para.justify" => "Justificar",
        "para.normal" => "Normal",
        "para.heading1" => "Título 1",
        "para.heading2" => "Título 2",
        "para.heading3" => "Título 3",
        "select.all" => "Seleccionar todo",
        "insert.table" => "Tabla",
        "insert.picture" => "Imagen",
        "arrange.wrap" => "Ajustar texto",
        "arrange.bringForward" => "Traer adelante",
        "arrange.sendBackward" => "Enviar atrás",
        "table.style" => "Estilo de tabla",
        "table.look" => "Opciones de estilo",
        "table.shading" => "Sombreado",
        "table.borders" => "Bordes",
        "table.selectTable" => "Seleccionar tabla",
        "insert.pageBreak" => "Salto de página",
        "insert.header" => "Encabezado",
        "insert.footer" => "Pie de página",
        "insert.pageNumber" => "Número de página",
        "insert.editHeader" => "Editar encabezado",
        "insert.editFooter" => "Editar pie",
        "insert.closeHeader" => "Cerrar encabezado/pie",
        "layout.margins" => "Márgenes",
        "layout.orientation" => "Orientación",
        "layout.size" => "Tamaño",
        "layout.columns" => "Columnas",
        "layout.hyphenation" => "Guiones",
        "layout.lineNumbers" => "Números de línea",
        "layout.textDirection" => "Dirección del texto",
        "layout.sectionRtl" => "Sección RTL",
        "text.columnBreak" => "Salto de columna",
        "layout.break" => "Salto",
        "review.spelling" => "Ortografía",
        "review.proofing" => "Revisar al escribir",
        "review.wordCount" => "Contar palabras",
        "review.trackChanges" => "Control de cambios",
        "review.markup" => "Marcas",
        "review.nextChange" => "Cambio siguiente",
        "review.previousChange" => "Cambio anterior",
        "review.accept" => "Aceptar",
        "review.reject" => "Rechazar",
        "review.acceptAll" => "Aceptar todo",
        "review.rejectAll" => "Rechazar todo",
        "review.nextComment" => "Comentario siguiente",
        "review.previousComment" => "Comentario anterior",
        "references.toc" => "Tabla de contenido",
        "references.updateToc" => "Actualizar TOC",
        "references.removeToc" => "Quitar TOC",
        "references.footnote" => "Nota al pie",
        "references.endnote" => "Nota al final",
        "references.nextFootnote" => "Nota siguiente",
        "view.printLayout" => "Diseño de impresión",
        "view.webLayout" => "Diseño web",
        "view.draft" => "Borrador",
        "view.ruler" => "Regla",
        "view.gridlines" => "Líneas de cuadrícula",
        "view.navigationPane" => "Panel de navegación",
        "view.hideWhiteSpace" => "Ocultar espacio en blanco",
        "view.zoomIn" => "Acercar",
        "view.zoomOut" => "Alejar",
        "view.zoom100" => "100 %",
        "view.onePage" => "Una página",
        "view.pageWidth" => "Ancho de página",
        "view.multiplePages" => "Varias páginas",
        "view.darkMode" => "Modo oscuro",
        _ => return None,
    })
}

fn item_tooltip(item: &RibbonItem) -> String {
    if item.shortcut.is_empty() {
        item.label.clone()
    } else {
        format!("{} ({})", item.label, item.shortcut)
    }
}

/// Builder del chrome ribbon (fuera del leaf `Kind::Document`).
pub struct DocumentRibbon<Msg> {
    mode: RibbonMode,
    active_tab: String,
    on_tab: Option<Box<dyn Fn(String) -> Msg>>,
    on_command: Option<Box<dyn Fn(String) -> Msg>>,
    disabled: Vec<String>,
    pressed: Vec<String>,
    /// Acciones de archivo (QAT) a la izquierda de las pestañas.
    leading: Vec<Element<Msg>>,
    test_id: Option<String>,
}

impl<Msg> Default for DocumentRibbon<Msg> {
    fn default() -> Self {
        Self {
            mode: RibbonMode::Full,
            active_tab: "home".into(),
            on_tab: None,
            on_command: None,
            disabled: Vec::new(),
            pressed: Vec::new(),
            leading: Vec::new(),
            test_id: None,
        }
    }
}

impl<Msg: 'static> DocumentRibbon<Msg> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn mode(mut self, mode: RibbonMode) -> Self {
        self.mode = mode;
        self
    }

    pub fn active_tab(mut self, id: impl Into<String>) -> Self {
        self.active_tab = id.into();
        self
    }

    pub fn on_tab(mut self, f: impl Fn(String) -> Msg + 'static) -> Self {
        self.on_tab = Some(Box::new(f));
        self
    }

    pub fn on_command(mut self, f: impl Fn(String) -> Msg + 'static) -> Self {
        self.on_command = Some(Box::new(f));
        self
    }

    pub fn disabled_ids(mut self, ids: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.disabled = ids.into_iter().map(Into::into).collect();
        self
    }

    /// Toggles activos (bold/italic/align/…) — `Element::pressed`.
    pub fn pressed_ids(mut self, ids: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.pressed = ids.into_iter().map(Into::into).collect();
        self
    }

    /// Botones QAT (p. ej. Abrir / Guardar / Exportar) a la izquierda de las tabs.
    pub fn leading(mut self, items: impl IntoIterator<Item = Element<Msg>>) -> Self {
        self.leading = items.into_iter().collect();
        self
    }

    pub fn test_id(mut self, id: impl Into<String>) -> Self {
        self.test_id = Some(id.into());
        self
    }

    pub fn build(self) -> Element<Msg> {
        let tabs = ribbon_catalog(self.mode);
        if tabs.is_empty() && self.leading.is_empty() {
            return div().class("hidden");
        }
        let active = if tabs.iter().any(|t| t.id == self.active_tab) {
            self.active_tab.clone()
        } else if let Some(first) = tabs.first() {
            first.id.clone()
        } else {
            String::new()
        };

        let mut tab_row = div()
            .class(&themed(
                "flex flex-row flex-wrap items-center gap-1.5 border-b border-border pb-1",
                "flex flex-row flex-wrap items-center gap-1.5 border-b border-separator pb-1",
            ))
            .role(Role::TabList);

        if !self.leading.is_empty() {
            let mut qat = div()
                .class("flex flex-row items-center gap-0.5 shrink-0")
                .role(Role::Toolbar)
                .test_id("documento.archivo");
            for item in self.leading {
                qat = qat.child(item);
            }
            tab_row = tab_row.child(qat);
            if !tabs.is_empty() {
                tab_row = tab_row.child(toolbar_separator());
            }
        }

        for tab in &tabs {
            let selected = tab.id == active;
            let variant = if selected {
                ButtonVariant::Primary
            } else {
                ButtonVariant::Text
            };
            let mut btn = button_sized(tab.label.clone(), variant, ButtonSize::Sm)
                .role(Role::Tab)
                .test_id(format!("documento.ribbon.tab.{}", tab.id));
            if let Some(ref on_tab) = self.on_tab {
                let id = tab.id.clone();
                let msg = on_tab(id);
                btn = btn.on_click(msg);
            }
            tab_row = tab_row.child(btn);
        }

        let mut children = vec![tab_row];

        if let Some(active_tab) = tabs.iter().find(|t| t.id == active).cloned()
            .or_else(|| tabs.first().cloned())
        {
            let mut groups_row = div()
                .class(&themed(
                    "flex flex-row flex-wrap items-stretch gap-2 pt-1.5",
                    "flex flex-row flex-wrap items-stretch gap-2 pt-1.5",
                ))
                .role(Role::Toolbar)
                .test_id(format!("documento.ribbon.panel.{}", active_tab.id));

            for (gi, group) in active_tab.groups.iter().enumerate() {
                if gi > 0 {
                    groups_row = groups_row.child(
                        div().class(&themed(
                            "w-px self-stretch min-h-8 mx-0.5 shrink-0 bg-border",
                            "w-px self-stretch min-h-8 mx-0.5 shrink-0 bg-separator",
                        )),
                    );
                }
                let mut g = div().class("flex flex-col gap-0.5 min-w-0 justify-end");
                g = g.child(
                    text(&group.label).class(&themed(
                        "text-[11px] leading-none text-muted-foreground px-1",
                        "text-[11px] leading-none text-secondary-label px-1",
                    )),
                );
                let mut items = div().class("flex flex-row flex-wrap items-center gap-0.5");
                for item in &group.items {
                    let disabled = self.disabled.iter().any(|d| d == &item.cmd_id);
                    let pressed = self.pressed.iter().any(|d| d == &item.cmd_id);
                    let tip = item_tooltip(item);
                    let mut btn = if let Some(glyph) = icon_for_cmd(&item.cmd_id) {
                        toolbar_item(glyph, tip)
                            .class("disabled:opacity-40")
                            .disabled(disabled)
                            .pressed(pressed)
                            .test_id(format!("documento.ribbon.cmd.{}", item.cmd_id))
                    } else {
                        // Fallback: texto corto ES si no hay icono razonable.
                        button_sized(item.label.clone(), ButtonVariant::Text, ButtonSize::Sm)
                            .tooltip(tip)
                            .class("disabled:opacity-40")
                            .disabled(disabled)
                            .pressed(pressed)
                            .test_id(format!("documento.ribbon.cmd.{}", item.cmd_id))
                    };
                    if let Some(ref on_cmd) = self.on_command {
                        let msg = on_cmd(item.cmd_id.clone());
                        btn = btn.on_click(msg);
                    }
                    items = items.child(btn);
                }
                g = g.child(items);
                groups_row = groups_row.child(g);
            }
            children.push(groups_row);
        }

        let mut root = div()
            .class(&themed(
                "flex flex-col gap-0 w-full rounded-md border border-border bg-card px-2 py-1.5",
                "flex flex-col gap-0 w-full rounded-md border border-separator bg-window px-2 py-1.5",
            ))
            .role(Role::Group)
            .children(children);
        if let Some(id) = self.test_id {
            root = root.test_id(id);
        } else {
            root = root.test_id("documento.ribbon");
        }
        root
    }
}

/// Atajo: ribbon Full con handlers.
pub fn document_ribbon<Msg: 'static>() -> DocumentRibbon<Msg> {
    DocumentRibbon::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_location_splits_tab_and_group() {
        assert_eq!(
            parse_location("Home › Font"),
            ("Home".into(), "Font".into())
        );
        assert_eq!(
            parse_location("Insert › Tables"),
            ("Insert".into(), "Tables".into())
        );
        assert_eq!(
            parse_location("Home › Font › Underline"),
            ("Home".into(), "Font".into())
        );
    }

    #[test]
    fn home_insert_catalog_has_bold_and_table() {
        let tabs = ribbon_catalog(RibbonMode::HomeInsert);
        assert!(tabs.iter().any(|t| t.id == "home"));
        assert!(tabs.iter().any(|t| t.id == "insert"));
        let home = tabs.iter().find(|t| t.id == "home").unwrap();
        assert_eq!(home.label, "Inicio");
        let ids: Vec<_> = home
            .groups
            .iter()
            .flat_map(|g| g.items.iter().map(|i| i.cmd_id.as_str()))
            .collect();
        assert!(ids.contains(&"format.bold"), "{ids:?}");
        assert!(ids.contains(&"edit.undo"), "{ids:?}");
        let bold = home
            .groups
            .iter()
            .flat_map(|g| g.items.iter())
            .find(|i| i.cmd_id == "format.bold")
            .unwrap();
        assert_eq!(bold.label, "Negrita");
        assert_eq!(icon_for_cmd("format.bold"), Some(Icon::Bold));
        let insert = tabs.iter().find(|t| t.id == "insert").unwrap();
        let iids: Vec<_> = insert
            .groups
            .iter()
            .flat_map(|g| g.items.iter().map(|i| i.cmd_id.as_str()))
            .collect();
        assert!(iids.contains(&"insert.table"), "{iids:?}");
        assert!(iids.contains(&"insert.pageBreak"), "{iids:?}");
    }

    #[test]
    fn full_catalog_has_layout_review_view_and_references() {
        let tabs = ribbon_catalog(RibbonMode::Full);
        let ids: Vec<_> = tabs.iter().map(|t| t.id.as_str()).collect();
        for want in ["home", "insert", "layout", "review", "references", "view"] {
            assert!(ids.contains(&want), "missing tab {want} in {ids:?}");
        }
        let tab_cmds = |id: &str| -> Vec<String> {
            tabs.iter()
                .find(|t| t.id == id)
                .unwrap()
                .groups
                .iter()
                .flat_map(|g| g.items.iter().map(|i| i.cmd_id.clone()))
                .collect()
        };
        assert!(tab_cmds("layout").contains(&"layout.orientation".into()));
        assert!(tab_cmds("layout").len() >= 5, "{:?}", tab_cmds("layout"));
        assert!(tab_cmds("review").contains(&"review.spelling".into()));
        assert!(tab_cmds("review").contains(&"review.trackChanges".into()));
        assert!(tab_cmds("review").len() >= 5, "{:?}", tab_cmds("review"));
        assert!(tab_cmds("view").contains(&"view.zoomIn".into()));
        assert!(tab_cmds("view").contains(&"view.printLayout".into()));
        assert!(tab_cmds("view").len() >= 5, "{:?}", tab_cmds("view"));
        assert!(tab_cmds("references").contains(&"references.footnote".into()));
        assert!(tab_cmds("references").contains(&"references.toc".into()));
    }

    #[test]
    fn default_run_params_cover_layout_presets() {
        assert_eq!(
            default_run_params("layout.margins")["preset"],
            "normal"
        );
        assert_eq!(default_run_params("layout.columns")["count"], 2);
        assert_eq!(default_run_params("view.zoomIn"), serde_json::json!({}));
    }

    #[test]
    fn document_cmd_for_maps_known_ids() {
        assert_eq!(document_cmd_for("format.bold"), DocumentCmd::Bold);
        assert_eq!(
            document_cmd_for("insert.table"),
            DocumentCmd::InsertTable
        );
        assert_eq!(
            document_cmd_for("format.underline"),
            DocumentCmd::Raw("format.underline".into())
        );
    }

    #[test]
    fn specs_exist_in_wordcraft_registry() {
        let reg = wc_registry();
        let all = MVP_HOME
            .iter()
            .chain(MVP_INSERT.iter())
            .chain(MVP_LAYOUT.iter())
            .chain(MVP_REVIEW.iter())
            .chain(MVP_REFERENCES.iter())
            .chain(MVP_VIEW.iter());
        for id in all {
            assert!(reg.get(id).is_some(), "missing CommandSpec `{id}`");
        }
    }

    #[test]
    fn home_groups_are_localized_and_ordered() {
        let home = ribbon_catalog(RibbonMode::HomeOnly)
            .into_iter()
            .find(|t| t.id == "home")
            .unwrap();
        let labels: Vec<_> = home.groups.iter().map(|g| g.label.as_str()).collect();
        assert!(
            labels.starts_with(&["Portapapeles", "Fuente", "Párrafo"]),
            "{labels:?}"
        );
    }

    #[test]
    fn mvp_cmds_have_icon_or_es_label() {
        let all = MVP_HOME
            .iter()
            .chain(MVP_INSERT.iter())
            .chain(MVP_LAYOUT.iter())
            .chain(MVP_REVIEW.iter())
            .chain(MVP_REFERENCES.iter())
            .chain(MVP_VIEW.iter());
        for id in all {
            assert!(
                icon_for_cmd(id).is_some() || localize_cmd(id).is_some(),
                "cmd `{id}` needs icon or ES label"
            );
        }
    }

    #[test]
    fn ribbon_chrome_bold_pressed_and_cut_disabled_without_selection() {
        use wordcraft_doc::Document;
        use wordcraft_engine::Session;
        let mut s = Session::new(Document::from_text("Hola"));
        let chrome = ribbon_chrome_for(&mut s, RibbonMode::HomeOnly);
        assert!(
            !chrome.pressed.iter().any(|id| id == "format.bold"),
            "plain text is not bold: {:?}",
            chrome.pressed
        );
        assert!(
            chrome.disabled.iter().any(|id| id == "edit.cut"),
            "cut needs a selection: {:?}",
            chrome.disabled
        );
        s.run("select.all", &serde_json::json!({})).unwrap();
        s.run("format.bold", &serde_json::json!({})).unwrap();
        let chrome = ribbon_chrome_for(&mut s, RibbonMode::HomeOnly);
        assert!(
            chrome.pressed.iter().any(|id| id == "format.bold"),
            "bold selection must press format.bold: {:?}",
            chrome.pressed
        );
        assert!(
            !chrome.disabled.iter().any(|id| id == "edit.cut"),
            "cut enabled with selection: {:?}",
            chrome.disabled
        );
    }

    #[test]
    fn ribbon_chrome_align_center_pressed() {
        use wordcraft_doc::Document;
        use wordcraft_engine::Session;
        let mut s = Session::new(Document::from_text("Centro"));
        s.run("para.alignCenter", &serde_json::json!({})).unwrap();
        let chrome = ribbon_chrome_for(&mut s, RibbonMode::HomeOnly);
        assert!(
            chrome.pressed.iter().any(|id| id == "para.alignCenter"),
            "center align: {:?}",
            chrome.pressed
        );
        assert!(
            !chrome.pressed.iter().any(|id| id == "para.alignLeft"),
            "left should not be pressed: {:?}",
            chrome.pressed
        );
    }

    #[test]
    fn home_catalog_includes_format_painter_and_styles() {
        let home = ribbon_catalog(RibbonMode::HomeOnly)
            .into_iter()
            .find(|t| t.id == "home")
            .expect("home tab");
        let ids: Vec<&str> = home
            .groups
            .iter()
            .flat_map(|g| g.items.iter())
            .map(|i| i.cmd_id.as_str())
            .collect();
        assert!(ids.contains(&"edit.formatPainter"), "{ids:?}");
        assert!(ids.contains(&"para.heading1"), "{ids:?}");
        assert!(ids.contains(&"para.normal"), "{ids:?}");
        let styles = home
            .groups
            .iter()
            .find(|g| g.label == "Estilos")
            .expect("Estilos group");
        assert!(styles.items.len() >= 4, "{styles:?}");
    }

    #[test]
    fn ribbon_chrome_format_painter_pressed_when_armed() {
        use wordcraft_doc::Document;
        use wordcraft_engine::Session;
        let mut s = Session::new(Document::from_text("hola"));
        let _ = s.run("edit.formatPainter", &serde_json::json!({}));
        assert!(s.painter.is_some());
        let chrome = ribbon_chrome_for(&mut s, RibbonMode::HomeOnly);
        assert!(
            chrome.pressed.iter().any(|id| id == "edit.formatPainter"),
            "pressed={:?}",
            chrome.pressed
        );
    }

    #[test]
    fn ribbon_chrome_heading1_pressed() {
        use wordcraft_doc::Document;
        use wordcraft_engine::Session;
        let mut s = Session::new(Document::from_text("hola"));
        let _ = s.run("para.heading1", &serde_json::json!({}));
        let chrome = ribbon_chrome_for(&mut s, RibbonMode::HomeOnly);
        assert!(
            chrome.pressed.iter().any(|id| id == "para.heading1"),
            "pressed={:?}",
            chrome.pressed
        );
        assert!(
            !chrome.pressed.iter().any(|id| id == "para.normal"),
            "Normal should not stay pressed: {:?}",
            chrome.pressed
        );
    }


    #[test]
    fn full_ribbon_includes_paso6_cmds() {
        let tabs = ribbon_catalog(RibbonMode::Full);
        let ids: Vec<&str> = tabs
            .iter()
            .flat_map(|t| t.groups.iter())
            .flat_map(|g| g.items.iter())
            .map(|i| i.cmd_id.as_str())
            .collect();
        for need in [
            "edit.pasteText",
            "edit.pasteMerge",
            "arrange.wrap",
            "table.style",
            "insert.header",
            "insert.editHeader",
            "insert.closeHeader",
        ] {
            assert!(ids.contains(&need), "missing {need} in {ids:?}");
        }
        assert!(
            tabs.iter().any(|t| t.id == "table"),
            "Table tab missing: {:?}",
            tabs.iter().map(|t| t.id.as_str()).collect::<Vec<_>>()
        );
    }

}
