//! Ribbon Kai del documento, alimentado por el catálogo WordCraft (`CommandSpec`).
//!
//! No porta `wordcraft-ui-egui`: solo lee `id` / `label` / `location` / `shortcut`
//! (y `Session::why_disabled` cuando hay Session) del engine y pinta UI Kai.
//!
//! Diseño: [`docs/dev/17-document-ribbon.md`](../../docs/dev/17-document-ribbon.md).

use std::collections::BTreeMap;
use std::sync::OnceLock;

use wordcraft_engine::{CommandSpec, Registry, Session};

use crate::components::{button_sized, ButtonSize, ButtonVariant};
use crate::element::{div, text, Element, Role};

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
    pub label: String,
    pub shortcut: String,
}

/// Grupo bajo una pestaña (`Home › Font` → "Font").
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

/// Allowlist MVP: ids a mostrar. Labels/grupos salen del `CommandSpec`.
const MVP_HOME: &[&str] = &[
    "edit.cut",
    "edit.copy",
    "edit.paste",
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
    "select.all",
];

const MVP_INSERT: &[&str] = &[
    "insert.table",
    "insert.picture",
    "insert.pageBreak",
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
];

/// Review: proofing / tracking / changes sin dialogs de archivo ni texto libre.
/// Excluye `review.newComment` (exige `text`), compare/combine, restrict, speech UI.
const MVP_REVIEW: &[&str] = &[
    "review.spelling",
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
                    label: (*name).into(),
                    items,
                });
            }
        }
        for (label, items) in groups {
            group_list.push(RibbonGroup { label, items });
        }
        if group_list.is_empty() {
            continue;
        }
        out.push(RibbonTab {
            id: tab_label.to_ascii_lowercase(),
            label: tab_label.into(),
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
        "Insert" => vec!["Pages", "Tables", "Illustrations", "Text", "Links"],
        "Layout" => vec!["Page Setup", "Breaks", "Paragraph", "Arrange"],
        "Review" => vec!["Proofing", "Tracking", "Changes", "Comments"],
        "References" => vec!["Table of Contents", "Footnotes", "Captions", "Citations & Bibliography"],
        "View" => vec!["Views", "Show", "Zoom", "Dark Mode", "Immersive", "Page Movement"],
        _ => Vec::new(),
    }
}

fn item_from_spec(spec: &CommandSpec) -> RibbonItem {
    RibbonItem {
        cmd_id: spec.id.into(),
        label: spec.label.into(),
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
        _ => json!({}),
    }
}

/// ¿El comando está greyed-out en esta Session? (`Session::why_disabled`).
#[allow(dead_code)] // API pública para apps que enrutan Session al chrome.
pub fn cmd_disabled(session: &Session, id: &str) -> bool {
    session.why_disabled(id).is_some()
}

/// Builder del chrome ribbon (fuera del leaf `Kind::Document`).
pub struct DocumentRibbon<Msg> {
    mode: RibbonMode,
    active_tab: String,
    on_tab: Option<Box<dyn Fn(String) -> Msg>>,
    on_command: Option<Box<dyn Fn(String) -> Msg>>,
    disabled: Vec<String>,
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

    pub fn test_id(mut self, id: impl Into<String>) -> Self {
        self.test_id = Some(id.into());
        self
    }

    pub fn build(self) -> Element<Msg> {
        let tabs = ribbon_catalog(self.mode);
        if tabs.is_empty() {
            return div().class("hidden");
        }
        let active = if tabs.iter().any(|t| t.id == self.active_tab) {
            self.active_tab.clone()
        } else {
            tabs[0].id.clone()
        };

        let mut tab_row = div()
            .class("flex flex-row flex-wrap items-end gap-1 border-b border-border pb-0")
            .role(Role::TabList);
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

        let active_tab = tabs
            .iter()
            .find(|t| t.id == active)
            .cloned()
            .unwrap_or_else(|| tabs[0].clone());

        let mut groups_row = div()
            .class("flex flex-row flex-wrap items-start gap-3 pt-2")
            .role(Role::Toolbar)
            .test_id(format!("documento.ribbon.panel.{}", active_tab.id));

        for group in &active_tab.groups {
            let mut g = div().class("flex flex-col gap-1 min-w-0");
            g = g.child(
                text(&group.label).class("text-xs text-muted-foreground px-1"),
            );
            let mut items = div().class("flex flex-row flex-wrap items-center gap-1");
            for item in &group.items {
                let disabled = self.disabled.iter().any(|d| d == &item.cmd_id);
                let mut btn = button_sized(
                    item.label.clone(),
                    ButtonVariant::Outlined,
                    ButtonSize::Sm,
                )
                .disabled(disabled)
                .test_id(format!("documento.ribbon.cmd.{}", item.cmd_id));
                if !item.shortcut.is_empty() {
                    btn = btn.tooltip(item.shortcut.clone());
                }
                if let Some(ref on_cmd) = self.on_command {
                    let msg = on_cmd(item.cmd_id.clone());
                    btn = btn.on_click(msg);
                }
                items = items.child(btn);
            }
            g = g.child(items);
            groups_row = groups_row.child(g);
        }

        let mut root = div()
            .class("flex flex-col gap-1 w-full rounded-md border border-border bg-card p-2")
            .role(Role::Group)
            .children([tab_row, groups_row]);
        if let Some(id) = self.test_id {
            root = root.test_id(id);
        } else {
            root = root.test_id("documento.ribbon");
        }
        root
    }
}

/// Atajo: ribbon Home+Insert con handlers.
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
        let ids: Vec<_> = home
            .groups
            .iter()
            .flat_map(|g| g.items.iter().map(|i| i.cmd_id.as_str()))
            .collect();
        assert!(ids.contains(&"format.bold"), "{ids:?}");
        assert!(ids.contains(&"edit.undo"), "{ids:?}");
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
}
