//! Kai UI Desktop demo, shared by the `kai-ui-desktop-prototype` binary
//! (`cargo run`) and `examples/demo.rs` (`cargo run --example demo`).

use std::path::PathBuf;
use std::rc::Rc;
use std::sync::LazyLock;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::background::{BackgroundEffect, BackgroundKind, BackgroundMotion};
use crate::cache::DiskCache;
use crate::color::Color;
use crate::commands::{
    Action, Availability, CommandError, CommandRegistry, PALETTE, Query, Safety, Shortcut,
    describe_json,
};
use crate::components::{
    AlertVariant, Avatar, AvatarSize, BadgeVariant, BezelStyle, Button, ButtonSize, ButtonVariant,
    CHOICE_ACTIVE, CHOICE_HOVER, ChartKind, Column, DataPage, Dialog, DialogWidth, DotProgress,
    DotSize, EmptyState, FieldBezel, FieldDensity, FieldKind, FieldVariant, FormPage,
    IconButtonSize, IconButtonVariant, LabelLayout, MasterDetailPage, MediaSlot, MediaVariant,
    PageHeader, SearchVariant, Sidebar, SidebarGroup, SidebarItem, SwitchDensity, Table, TableView,
    Tabs, TextField, Toast, ViewChange, alert, avatar, badge, button, button_sized, caption, card,
    card_actions, card_content, card_header, chart, checkbox, checkbox_with_options, color_field,
    column_menu, dot_progress, form_field_cell, form_full_cell, group_box, heading, icon_button,
    icon_button_sized, loading_placeholder, page, page_body, page_grid, page_grid_item, paragraph,
    search_field, segmented, segmented_with_options, select, select_with_options, stat, switch,
    switch_labeled, switch_with_options, text_area, toast_key, toast_stack, toast_timer, toolbar,
    toolbar_item, toolbar_search, toolbar_separator, toolbar_spacer, toolbar_title,
    validate_image_file, virtual_list,
};
use crate::document::{DocumentChange, DocumentCmd, document};
#[cfg(feature = "document")]
use crate::document::{RibbonMode, document_cmd_for, document_ribbon};
use crate::element::{ControlSize, ControlState, Element, Role, div, icon, image, text};
use crate::geo::{LatLon, LatLonBounds, MapView};
use crate::geometry::Size;
use crate::icons::Icon;
use crate::maps::editor::{PolygonDraft, PolygonEdit, crossing};
use crate::maps::layers::{MapLayer, polygons};
use crate::maps::style::MapStyle;
use crate::maps::tiles::TileSource;
use crate::maps::{Anchor, map, marker};
use crate::media::{ImageSource, sample_landscape, sample_portrait};
use crate::runtime::{self, App, Effects, PickFile, SaveFile, current_invocation};
use crate::store::{Store, StoreError, Stored};
use crate::theme::{
    AppKitGeneration, Appearance, Density, FieldLabels, MacVersion, MixStyle, SystemInfo, Theme,
};
use crate::ui_engine::Key;
use crate::version;

/// Demo model: everything the screen shows derives from it.
#[derive(Clone, Debug, PartialEq)]
struct Demo {
    clicks: u32,
    name: String,
    email: String,
    submissions: Vec<String>,
    search: String,
    password: String,
    show_password: bool,
    amount: String,
    discount: String,
    commune: String,
    code: String,
    quantity: String,
    autocomplete: String,
    notifications: bool,
    /// Form: index into [`ROLES`].
    role: Option<usize>,
    terms: bool,
    /// Section shown next to the sidebar.
    section: Section,
    /// The sidebar is a rail of icons.
    sidebar_collapsed: bool,
    /// The first window size has set `sidebar_collapsed`.
    sized: bool,
    /// Dónde se guardan las preferencias entre ejecuciones (`None` en
    /// tests: nada toca el disco).
    store: Option<Store>,
    /// Lo último guardado, para escribir solo cuando algo cambia.
    saved_preferences: Option<DemoPreferences>,
    /// Window background effect (top panel); `KAI_BG`, `KAI_BG_ANIM`.
    background: BackgroundKind,
    background_motion: BackgroundMotion,
    /// Campos: PIN (a plain `password` field), notes, the search field.
    pin: String,
    show_pin: bool,
    notes: String,
    lookup: String,
    /// Selección.
    wifi: bool,
    /// Permissions under «Todos» (mixed while some are on).
    permissions: [bool; 3],
    /// Index into [`COUNTRIES`].
    country: Option<usize>,
    /// Index into [`ALIGNMENTS`].
    alignment: usize,
    /// Contenedores: the selected tab.
    tab_index: usize,
    /// Tabla: the toolbar search, the clients, what the table shows of
    /// them (search, filters, sort) and the selected one (index into
    /// `clients`), and whether an Excel file is being written.
    query: String,
    clients: Rc<[Client]>,
    table_view: TableView,
    client: Option<usize>,
    exporting: bool,
    /// Multimedia: the three image fields, the URL typed and the one
    /// loaded in the avatar.
    photo: ImageSlot,
    cover: ImageSlot,
    logo: ImageSlot,
    url: String,
    avatar_url: Option<String>,
    /// Diálogos y avisos.
    dialog_kind: Option<DemoDialog>,
    toasts: Vec<Toast<Msg>>,
    next_toast: u64,
    /// Mapa: what it shows, and whether a new view glides there.
    view: MapView,
    animate_map: bool,
    /// Mapa: the DSP sectors (their outlines change when edited), whether
    /// they are drawn, the code of the one selected and the last request
    /// to frame a box (with its serial).
    sectors: Vec<Sector>,
    show_areas: bool,
    area: Option<String>,
    fit_request: Option<(LatLonBounds, u64)>,
    /// Mapa: the sector being edited and its draft.
    editing: Option<(String, PolygonDraft)>,
    /// Botones: the last wait that ended, and how many background syncs
    /// run.
    wait_ms: Option<&'static str>,
    syncing: u32,
    /// Gráficos: the selected bar (month index).
    month: Option<usize>,
    /// Documento: markdown ligero + último comando de toolbar.
    doc_markdown: String,
    doc_cmd: Option<(DocumentCmd, u64)>,
    doc_cmd_serial: u64,
    /// Carga `.docx` pendiente (bytes + serial).
    doc_load: Option<(u64, std::sync::Arc<[u8]>)>,
    doc_load_serial: u64,
    /// Tras elegir ruta de guardar/exportar, pedimos TakeDocument.
    doc_pending_export: Option<(std::path::PathBuf, DocExportKind)>,
    /// I/O de archivo en curso (botones Archivo).
    doc_file_busy: bool,
    /// Último archivo abierto/guardado (nombre para chrome Archivo).
    doc_file_name: Option<String>,
    /// Pestaña activa del ribbon Documento (`home` / `insert`).
    #[cfg_attr(not(feature = "document"), allow(dead_code))]
    doc_ribbon_tab: String,
    /// Ribbon toggles pressed (desde Session vía DocumentChange).
    #[cfg_attr(not(feature = "document"), allow(dead_code))]
    doc_ribbon_pressed: Vec<String>,
    /// Ribbon cmds disabled (`why_disabled`).
    #[cfg_attr(not(feature = "document"), allow(dead_code))]
    doc_ribbon_disabled: Vec<String>,
    /// Pestañas contextuales visibles (Tabla, Formato de imagen).
    #[cfg_attr(not(feature = "document"), allow(dead_code))]
    doc_ribbon_contexts: Vec<String>,
    /// Barra de comentario abierta («Nuevo comentario» en Revisar).
    doc_comment_open: bool,
    /// Barra Buscar y reemplazar visible (⌘F / lupa de la QAT).
    doc_find_open: bool,
    /// Find/Replace query (barra bajo el ribbon).
    doc_find: String,
    /// Texto de reemplazo.
    doc_replace: String,
    /// Draft de comentario (`review.newComment`).
    doc_comment_draft: String,
    /// Estado breve del editor (find/comments/proofing).
    doc_edit_status: String,
    /// Plantilla lista+detalle: índice del cliente seleccionado.
    template_index: usize,
    /// Catálogo vivo (Fase E): preset de tema, densidad y tamaño.
    catalog_preset: CatalogPreset,
    catalog_density: Density,
    catalog_size: ControlSize,
    catalog_sample: String,
    catalog_check: bool,
    catalog_switch: bool,
    catalog_select: Option<usize>,
    catalog_tab: usize,
}

/// How long the «Acciones con espera» buttons wait.
const WAIT: std::time::Duration = std::time::Duration::from_millis(1500);

impl Default for Demo {
    fn default() -> Self {
        let clients: Rc<[Client]> = (1..=CLIENT_COUNT).map(client).collect();
        Self {
            clicks: 0,
            name: String::new(),
            email: String::new(),
            submissions: Vec::new(),
            search: String::new(),
            password: String::new(),
            show_password: false,
            amount: String::new(),
            discount: String::new(),
            commune: String::new(),
            code: String::new(),
            quantity: String::new(),
            autocomplete: String::new(),
            notifications: true,
            role: None,
            terms: false,
            section: Section::Home,
            sidebar_collapsed: false,
            sized: false,
            background: BackgroundKind::None,
            background_motion: BackgroundMotion::Static,
            pin: String::new(),
            show_pin: false,
            notes: String::new(),
            lookup: String::new(),
            wifi: true,
            permissions: [true, false, true],
            country: Some(0),
            alignment: 0,
            tab_index: 0,
            query: String::new(),
            table_view: TableView::new(&clients, &table_columns()),
            clients,
            client: None,
            exporting: false,
            photo: ImageSlot::with(sample_portrait()),
            cover: ImageSlot::default(),
            logo: ImageSlot::with(sample_landscape()),
            url: String::new(),
            avatar_url: None,
            dialog_kind: None,
            toasts: Vec::new(),
            next_toast: 1,
            view: INITIAL_VIEW,
            animate_map: true,
            store: None,
            saved_preferences: None,
            sectors: SECTORS.clone(),
            show_areas: true,
            area: None,
            fit_request: None,
            editing: None,
            wait_ms: None,
            syncing: 0,
            month: None,
            doc_markdown: demo_doc_markdown(),
            doc_cmd: None,
            doc_cmd_serial: 0,
            doc_load: None,
            doc_load_serial: 0,
            doc_pending_export: None,
            doc_file_busy: false,
            doc_file_name: None,
            doc_ribbon_tab: "home".into(),
            doc_ribbon_pressed: Vec::new(),
            doc_ribbon_disabled: Vec::new(),
            doc_ribbon_contexts: Vec::new(),
            doc_comment_open: false,
            doc_find_open: false,
            doc_find: String::new(),
            doc_replace: String::new(),
            doc_comment_draft: String::new(),
            doc_edit_status: String::new(),
            template_index: 0,
            catalog_preset: CatalogPreset::GlassLight,
            catalog_density: Density::Comfortable,
            catalog_size: ControlSize::Large,
            catalog_sample: String::new(),
            catalog_check: true,
            catalog_switch: true,
            catalog_select: Some(0),
            catalog_tab: 0,
        }
    }
}

/// Plaza de Armas de Parral, where the map starts.
const MAIN_SQUARE: LatLon = LatLon::new(-36.1415, -71.8265);
const INITIAL_VIEW: MapView = MapView::new(MAIN_SQUARE, 13.0);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DocExportKind {
    Docx,
    Pdf,
}

/// Encode + write off the UI thread (`Effects::perform`).
#[cfg(feature = "document")]
fn write_document_file(
    doc: wordcraft_doc::Document,
    path: std::path::PathBuf,
    kind: DocExportKind,
) -> Result<String, String> {
    let bytes = match kind {
        DocExportKind::Docx => wordcraft_docx::write(&doc).map_err(|e| e.to_string())?,
        DocExportKind::Pdf => wordcraft_pdf::export(&doc, &wordcraft_pdf::PdfOptions::default())
            .map_err(|e| e.to_string())?,
    };
    std::fs::write(&path, &bytes).map_err(|e| e.to_string())?;
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.display().to_string());
    let label = match kind {
        DocExportKind::Docx => "DOCX",
        DocExportKind::Pdf => "PDF",
    };
    Ok(format!("{label}: {name}"))
}

fn demo_doc_markdown() -> String {
    let mut parts = vec![
        "# Informe de demostración".to_owned(),
        "Fase 1: varias páginas Letter, scroll del viewport y hit-test WordCraft.".to_owned(),
    ];
    for i in 0..52 {
        parts.push(format!(
            "Párrafo {i}. Texto de relleno para forzar paginación WordCraft en la demo."
        ));
    }
    parts.push("Fin del documento multi-página.".to_owned());
    parts.join("\n\n")
}

/// Protomaps extract of Parral (`scripts/extract-region.sh`).
const PARRAL_PMTILES: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/assets/maps/parral.pmtiles");
/// The five starting sectors of DSP Parral, as its seed draws them.
const DSP_AREAS: &str = include_str!("../assets/maps/dsp-areas.geojson");
/// Margin around a selected sector, as a fraction of its size per side.
const SECTOR_MARGIN: f64 = 0.2;
/// Demo chart: six months of clicks and a longer trend.
const MONTHS: [&str; 6] = ["Ene", "Feb", "Mar", "Abr", "May", "Jun"];
const CLICKS_PER_MONTH: [f32; 6] = [4.0, 12.0, 7.0, 15.0, 9.0, 11.0];
const TREND: [f32; 12] = [
    2.0, 5.0, 4.0, 8.0, 6.0, 9.0, 7.0, 12.0, 10.0, 14.0, 11.0, 16.0,
];

/// An area of operation of DSP.
#[derive(Clone, Debug, PartialEq)]
struct Sector {
    code: String,
    name: String,
    kind_name: String,
    layer: MapLayer,
    /// Its color as typed in Mapa: hex, or empty for the color of its type.
    color: String,
}

/// The five starting colors of DSP's area types.
const DSP_PALETTE: [(&str, Color); 5] = [
    ("Azul", Color::hex(0x1e73ae)),
    ("Naranjo", Color::hex(0xc45c26)),
    ("Verde", Color::hex(0x2d6a4f)),
    ("Violeta", Color::hex(0x7c3aed)),
    ("Ámbar", Color::hex(0xb45309)),
];

/// The color of an area type, as in DSP's `operation_area_types`.
fn color_for_kind(kind_name: &str) -> Color {
    match kind_name {
        "territorio" => Color::hex(0x1e73ae),
        "barrio" => Color::hex(0x2d6a4f),
        _ => Color::hex(0xc45c26),
    }
}

impl Sector {
    /// Its outline, without repeating the first vertex.
    fn ring_points(&self) -> Vec<LatLon> {
        self.layer.outer_ring()
    }

    /// Take `ring_points` as its new outline; a ring a layer cannot be made
    /// of leaves it as it was.
    fn with_ring(&mut self, ring_points: &[LatLon]) {
        if let Ok(layer) = MapLayer::ring(ring_points) {
            self.layer = layer;
        }
    }

    /// The color typed for it, or else the one of its type (also while
    /// the typed hex is incomplete).
    fn color(&self) -> Color {
        Color::parse_hex(&self.color).unwrap_or_else(|| color_for_kind(&self.kind_name))
    }

    /// Background class of its color.
    fn swatch(&self) -> String {
        format!("bg-[{}]", self.color().to_hex())
    }
}

static SECTORS: LazyLock<Vec<Sector>> = LazyLock::new(|| {
    let text = |properties: &geojson::JsonObject, key: &str| {
        properties
            .get(key)
            .and_then(|value| value.as_str())
            .unwrap_or_default()
            .to_owned()
    };
    polygons(DSP_AREAS)
        .expect("assets/maps/dsp-areas.geojson")
        .into_iter()
        .map(|(layer, properties)| {
            let kind_name = text(&properties, "type");
            Sector {
                code: text(&properties, "code"),
                name: text(&properties, "name"),
                color: color_for_kind(&kind_name).to_hex(),
                kind_name,
                layer,
            }
        })
        .collect()
});

/// The sector `code` as the seed draws it.
fn initial_sector(code: &str) -> Option<&'static Sector> {
    SECTORS.iter().find(|sector| sector.code == code)
}

/// Area in hectares with a decimal comma.
fn hectares(area: f64) -> String {
    format!("{area:.1} ha").replace('.', ",")
}

/// Largest file the image fields take, in megabytes.
const MAX_IMAGE_MB: u64 = 5;

/// An image field of «Multimedia»: what it shows and the last file
/// refused.
#[derive(Clone, Debug, Default, PartialEq)]
struct ImageSlot {
    actual: Option<ImageSource>,
    error: Option<String>,
}

impl ImageSlot {
    fn with(source: ImageSource) -> Self {
        Self {
            actual: Some(source),
            error: None,
        }
    }

    /// Show `path` if it is an image of at most [`MAX_IMAGE_MB`].
    fn set(&mut self, path: PathBuf) {
        match validate_image_file(&path, MAX_IMAGE_MB) {
            Ok(()) => *self = Self::with(ImageSource::Path(path)),
            Err(error) => self.error = Some(error),
        }
    }
}

/// The image fields of «Multimedia».
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ImageField {
    Photo,
    Cover,
    Logo,
}

/// Rows of the client table.
const CLIENT_COUNT: u32 = 10_000;

/// Pause in the typing of a column filter before the table refreshes.
const FILTER_PAUSE: std::time::Duration = std::time::Duration::from_millis(250);

/// One client of the generated table: number, name, city, plan, amount.
#[derive(Debug, PartialEq)]
struct Client {
    number: u32,
    name: String,
    city: &'static str,
    plan: &'static str,
    amount: u32,
}

/// Client `number` (1-based), the same every time.
fn client(number: u32) -> Client {
    const FIRST_NAMES: [&str; 12] = [
        "Ana", "Bruno", "Carla", "Diego", "Elena", "Felipe", "Gabriela", "Hugo", "Inés", "Javier",
        "Laura", "Matías",
    ];
    const SURNAMES: [&str; 10] = [
        "Rojas",
        "Muñoz",
        "Soto",
        "Contreras",
        "Silva",
        "Martínez",
        "Sepúlveda",
        "Morales",
        "Fuentes",
        "Vega",
    ];
    const CITIES: [&str; 8] = [
        "Santiago",
        "Valparaíso",
        "Concepción",
        "La Serena",
        "Antofagasta",
        "Temuco",
        "Puerto Montt",
        "Arica",
    ];
    const PLANS: [&str; 3] = ["Básico", "Pro", "Empresa"];
    let n = number as usize;
    let mix = n.wrapping_mul(2_654_435_761) >> 7;
    Client {
        number,
        name: format!(
            "{} {}",
            FIRST_NAMES[n % FIRST_NAMES.len()],
            SURNAMES[(n / FIRST_NAMES.len() + mix) % SURNAMES.len()]
        ),
        city: CITIES[mix % CITIES.len()],
        plan: PLANS[(mix / 7) % PLANS.len()],
        amount: (mix % 9_000 + 100) as u32 * 10,
    }
}

/// Columns of the client table: number, name with its avatar, city, plan
/// as a badge, amount and the row's actions.
fn table_columns() -> Vec<Column<Client, Msg>> {
    vec![
        Column::number("N.º", |c: &Client| f64::from(c.number))
            .fixed(72.0)
            .sortable(),
        Column::text("Nombre", |c: &Client| c.name.clone())
            .render(|c: &Client| {
                div().class("flex items-center gap-2 min-w-0").children([
                    Avatar::new(c.name.clone()).size(AvatarSize::Xs).build(),
                    text(c.name.as_str()).class("truncate"),
                ])
            })
            .flex(2.0)
            .min_width(160.0)
            .sortable()
            .filterable(),
        Column::text("Ciudad", |c: &Client| c.city.to_owned())
            .flex(1.5)
            .sortable()
            .filterable()
            .hideable(),
        Column::text("Plan", |c: &Client| c.plan.to_owned())
            .render(|c: &Client| badge(c.plan, plan_badge(c.plan)))
            .fixed(104.0)
            .sortable()
            .filterable()
            .hideable(),
        Column::number("Monto", |c: &Client| f64::from(c.amount))
            .format(|amount| format!("$ {amount}"))
            .fixed(112.0)
            .sortable()
            .filterable()
            .hideable(),
        Column::actions(|c: &Client| {
            icon_button_sized(
                Icon::Eye,
                format!("Ver {}", c.name),
                IconButtonVariant::Text,
                IconButtonSize::Sm,
            )
            .on_click(Msg::ShowClient(c.number as usize - 1))
            .test_id(format!("table.view.{}", c.number))
        })
        .fixed(64.0),
    ]
}

/// Panel of an expanded client row: what the row leaves out, and a button.
fn client_detail(c: &Client) -> Element<Msg> {
    div()
        .class("flex items-center justify-between gap-3 h-full")
        .children([
            div().class("flex flex-col gap-0.5 min-w-0").children([
                text(format!("Cliente N.º {} de {}", c.number, c.city))
                    .class("text-sm font-medium truncate"),
                caption(format!("Plan {} · $ {} al mes", c.plan, c.amount)),
            ]),
            button_sized("Ver ficha", ButtonVariant::Outlined, ButtonSize::Sm)
                .on_click(Msg::ShowClient(c.number as usize - 1))
                .test_id(format!("table.card.{}", c.number)),
        ])
}

/// Badge color of a client plan.
fn plan_badge(plan: &str) -> BadgeVariant {
    match plan {
        "Empresa" => BadgeVariant::Primary,
        "Pro" => BadgeVariant::Info,
        _ => BadgeVariant::Secondary,
    }
}

/// Options of the country select (Selección).
const COUNTRIES: [&str; 4] = ["Chile", "Argentina", "Perú", "Uruguay"];
/// Options of the alignment segmented control (Selección).
const ALIGNMENTS: [&str; 3] = ["Izquierda", "Centro", "Derecha"];
/// Permissions under «Todos» (Selección).
const PERMISSIONS: [&str; 3] = ["Leer", "Escribir", "Compartir"];

/// The open dialog of «Diálogos y avisos».
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DemoDialog {
    Save,
    Delete,
}

/// Variant of a toast, for `warnings_found.mostrar`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
enum ToastKind {
    Success,
    Error,
    Warning,
    Info,
}

impl ToastKind {
    const ALL: [ToastKind; 4] = [Self::Success, Self::Error, Self::Warning, Self::Info];

    fn alert(self) -> AlertVariant {
        match self {
            Self::Success => AlertVariant::Success,
            Self::Error => AlertVariant::Error,
            Self::Warning => AlertVariant::Warning,
            Self::Info => AlertVariant::Info,
        }
    }

    fn key(self) -> &'static str {
        match self {
            Self::Success => "success",
            Self::Error => "error",
            Self::Warning => "warning",
            Self::Info => "info",
        }
    }

    /// Button label, toast title and message.
    fn texts(self) -> (&'static str, &'static str, &'static str) {
        match self {
            Self::Success => ("Éxito", "Cambios guardados", "Se guardaron hace un momento"),
            Self::Error => (
                "Error",
                "No se pudo enviar",
                "Revisa la conexión y reintenta",
            ),
            Self::Warning => ("Advertencia", "Espacio casi lleno", "Quedan 200 MB libres"),
            Self::Info => (
                "Info",
                "Nueva versión",
                "Kai UI Desktop se actualizará al cerrar",
            ),
        }
    }
}

/// Initial background from `KAI_BG=none|mesh|aurora|glow` and
/// `KAI_BG_ANIM=0|1` (unset: none, static), plus a warning per unknown
/// value.
fn background_from_env(
    kind: Option<&str>,
    animated: Option<&str>,
) -> (BackgroundKind, BackgroundMotion, Vec<String>) {
    let mut warnings = Vec::new();
    let kind = match kind {
        None => BackgroundKind::None,
        Some(value) => BackgroundKind::parse(value).unwrap_or_else(|| {
            warnings.push(format!(
                "KAI_BG={value} desconocido (none, mesh, aurora, glow); se usa none"
            ));
            BackgroundKind::None
        }),
    };
    let motion = match animated.map(str::trim) {
        None | Some("" | "0") => BackgroundMotion::Static,
        Some("1") => BackgroundMotion::Animated,
        Some(other) => {
            warnings.push(format!("KAI_BG_ANIM={other} desconocido (0, 1); se usa 0"));
            BackgroundMotion::Static
        }
    };
    (kind, motion, warnings)
}

/// Options of the form's role select.
const ROLES: [&str; 3] = ["Diseño", "Desarrollo", "Producto"];

/// What the demo keeps across `kai dev` restarts. Every field is optional,
/// so a state saved by an older build still restores what it can.
#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(default)]
struct DevState {
    section: Option<Section>,
    sidebar_collapsed: Option<bool>,
    /// Code of the selected sector in Mapa.
    area: Option<String>,
}

/// Theme preset for the live catalog (Fase E). Swaps `App::theme` while
/// the Catálogo section is open.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CatalogPreset {
    AppKitLight,
    AppKitDark,
    GlassLight,
    GlassDark,
    Kai,
}

impl CatalogPreset {
    const ALL: [CatalogPreset; 5] = [
        Self::AppKitLight,
        Self::AppKitDark,
        Self::GlassLight,
        Self::GlassDark,
        Self::Kai,
    ];

    fn label(self) -> &'static str {
        match self {
            Self::AppKitLight => "AppKit",
            Self::AppKitDark => "AppKit Dark",
            Self::GlassLight => "Glass",
            Self::GlassDark => "Glass Dark",
            Self::Kai => "Kai",
        }
    }

    fn index(self) -> usize {
        Self::ALL.iter().position(|p| *p == self).unwrap_or(0)
    }
}

/// Tamaños del Catálogo vivo, en el orden de su control segmentado.
const CATALOG_SIZES: [ControlSize; 5] = [
    ControlSize::Mini,
    ControlSize::Small,
    ControlSize::Regular,
    ControlSize::Large,
    ControlSize::ExtraLarge,
];

/// Id de la demo (como en `kai.toml`): nombre de su carpeta de datos.
const APP_ID: &str = "dev.kai.ui-desktop";

/// Lo que la demo recuerda entre ejecuciones (Paso 16).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
struct DemoPreferences {
    section: Section,
    sidebar_collapsed: bool,
    map_view_state: MapView,
    /// Catálogo vivo: índices de tema, densidad y tamaño.
    catalog: [usize; 3],
}

impl Stored for DemoPreferences {
    const KEY: &'static str = "preferencias";
    const VERSION: u32 = 1;
}

/// Sections of the sidebar, in its order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
enum Section {
    /// Fondo de ventana, contador, formulario y estadísticas.
    Home,
    /// Variantes de TextField.
    Fields,
    /// Botones, tamaños e IconButtons.
    Buttons,
    /// Switches, checkboxes, selects y controles segmentados.
    Selection,
    /// Cards, group boxes, estadísticas y texto.
    Containers,
    /// Badges y alertas.
    Badges,
    /// Avatares, imágenes y campos de imagen (foto, portada, logo).
    Multimedia,
    /// Tabla de 10 000 clientes y lista virtualizada.
    Table,
    /// Mapa interactivo de Parral.
    Map,
    /// Gráficos de barras y sparkline.
    Charts,
    /// Mini procesador de texto (esqueleto Document).
    Document,
    /// Diálogos, avisos (toasts) y la paleta de comandos.
    Dialogs,
    /// Plantilla lista + detalle (master/detail).
    TemplateList,
    /// Plantilla formulario (densidad comfortable).
    TemplateForm,
    /// Plantilla vista de datos (densidad compact).
    TemplateData,
    /// Catálogo vivo: matriz filtrable tema × density × size.
    Catalog,
}

impl Section {
    const ALL: [Section; 16] = [
        Self::Home,
        Self::Fields,
        Self::Buttons,
        Self::Selection,
        Self::Containers,
        Self::Badges,
        Self::Multimedia,
        Self::Table,
        Self::Map,
        Self::Charts,
        Self::Document,
        Self::TemplateList,
        Self::TemplateForm,
        Self::TemplateData,
        Self::Catalog,
        Self::Dialogs,
    ];

    fn index(self) -> usize {
        Self::ALL.iter().position(|s| *s == self).unwrap_or(0)
    }

    fn key(self) -> &'static str {
        match self {
            Self::Home => "home",
            Self::Fields => "fields",
            Self::Buttons => "buttons",
            Self::Selection => "selection",
            Self::Containers => "containers",
            Self::Badges => "badges",
            Self::Multimedia => "multimedia",
            Self::Table => "table",
            Self::Map => "map",
            Self::Charts => "charts",
            Self::Document => "document",
            Self::Dialogs => "dialogs",
            Self::TemplateList => "template_list",
            Self::TemplateForm => "template_form",
            Self::TemplateData => "template_data",
            Self::Catalog => "catalog",
        }
    }

    /// Sidebar row.
    fn item(self) -> SidebarItem {
        let (label, glyph) = match self {
            Self::Home => ("Inicio", Icon::House),
            Self::Fields => ("Campos", Icon::TextCursorInput),
            Self::Buttons => ("Botones", Icon::MousePointerClick),
            Self::Selection => ("Selección", Icon::ToggleLeft),
            Self::Containers => ("Contenedores", Icon::LayoutGrid),
            Self::Badges => ("Badges y alertas", Icon::Tag),
            Self::Multimedia => ("Multimedia", Icon::Image),
            Self::Table => ("Tabla", Icon::Table),
            Self::Map => ("Mapa", Icon::Map),
            Self::Charts => ("Gráficos", Icon::LayoutDashboard),
            Self::Document => ("Documento", Icon::FileText),
            Self::Dialogs => ("Diálogos y avisos", Icon::MessageSquare),
            Self::TemplateList => ("Lista + detalle", Icon::Columns3),
            Self::TemplateForm => ("Formulario", Icon::TextCursorInput),
            Self::TemplateData => ("Vista de datos", Icon::Sheet),
            Self::Catalog => ("Catálogo vivo", Icon::LayoutTemplate),
        };
        SidebarItem::new(label).icon(glyph)
    }
}

/// Inputs of the TextField variants card.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Field {
    Search,
    Password,
    Amount,
    Discount,
    Commune,
    Code,
    Quantity,
    Autocomplete,
    Pin,
    Notes,
    Lookup,
    Url,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum Msg {
    Increment,
    Add(u32),
    Reset,
    NameChanged(String),
    EmailChanged(String),
    Submit,
    Clear,
    Edit(Field, String),
    TogglePassword,
    ToggleNotifications,
    SetRole(usize),
    ToggleTerms,
    GoTo(Section),
    /// Plantilla lista+detalle: seleccionar fila.
    TemplateSelect(usize),
    CatalogPreset(usize),
    CatalogDensity(usize),
    CatalogSize(usize),
    CatalogSample(String),
    /// Empty filter demo: clear the sample field.
    CatalogClearFilter,
    CatalogToggleCheck,
    CatalogToggleSwitch,
    CatalogSelect(usize),
    CatalogTab(usize),
    ToggleSidebar,
    /// First window size: whether it is narrower than `sm`.
    InitialWidth {
        narrow: bool,
    },
    SetBackground(BackgroundKind),
    ToggleBackgroundMotion,
    TogglePin,
    ToggleWifi,
    /// `None` is «Todos».
    TogglePermission(Option<usize>),
    SetCountry(usize),
    SetAlignment(usize),
    SetTab(usize),
    /// The toolbar search, which filters the table.
    Query(String),
    /// Sort, filter, hide columns or expand rows of the table.
    Table(ViewChange),
    /// Recompute the table's rows after a pause in a filter's typing.
    RefreshTable,
    /// A client of the table, by its index in `clients`.
    SelectRow(usize),
    /// The «Ver» action of a client's row.
    ShowClient(usize),
    /// Ask where to save the rows the table shows as an Excel file.
    ExportTable,
    /// Where the save panel said (`None`: cancelled).
    SaveExcel(Option<PathBuf>),
    /// The Excel file was written (its name and rows) or not.
    TableExported(Result<(String, usize), String>),
    OpenDialog(DemoDialog),
    CloseDialog,
    /// The dialog's default button.
    ConfirmDialog,
    ShowToast(ToastKind),
    CloseToast(u64),
    /// Opens the file panel for a field.
    PickImage(ImageField),
    /// The panel's answer, or a file dropped on the field.
    ImagePicked(ImageField, Option<PathBuf>),
    /// Loads the avatar from a URL; empty, back to the icon.
    LoadUrl(String),
    /// A gesture on the map ended there.
    MapMoved(MapView),
    /// Shows `view` on the map, gliding there when `animate`.
    SetMapView {
        view: MapView,
        animate: bool,
    },
    /// Selects the sector with this code and frames it.
    SelectArea(String),
    /// Moves the map to show all of the box.
    FitBounds(LatLonBounds),
    ToggleAreas,
    /// The selected bar of the chart, by month index.
    Month(usize),
    /// Documento: contenido tipado / formateado.
    Document(DocumentChange),
    /// Documento: comando de toolbar / Action / ribbon.
    DocCmd(DocumentCmd),
    /// Documento: cambiar pestaña del ribbon.
    #[cfg_attr(not(feature = "document"), allow(dead_code))]
    DocRibbonTab(String),
    /// Resultado de guardar las preferencias en disco.
    PreferencesSaved(Result<(), StoreError>),
    /// Documento: mostrar u ocultar la barra Buscar y reemplazar (⌘F).
    DocToggleFind,
    /// Documento: mostrar u ocultar la barra de nuevo comentario.
    #[cfg_attr(not(feature = "document"), allow(dead_code))]
    DocToggleComment,
    /// Documento: query de find.
    DocFindChanged(String),
    /// Documento: texto de replace.
    DocReplaceChanged(String),
    /// Documento: draft de comentario.
    DocCommentDraftChanged(String),
    /// Documento: panel Abrir `.docx`.
    DocOpen,
    /// Ruta elegida al abrir (`None`: cancelado).
    DocOpenChosen(Option<PathBuf>),
    /// Bytes leídos del `.docx` (o error).
    DocBytesRead(Result<Vec<u8>, String>),
    /// Documento: panel Guardar `.docx`.
    DocSave,
    DocSaveTo(Option<PathBuf>),
    /// Documento: panel Exportar PDF.
    DocExportPdf,
    DocExportPdfTo(Option<PathBuf>),
    /// Escritura / export terminó.
    DocFileReady(Result<String, String>),
    /// What the color field of the selected sector holds.
    ColorSector(String),
    /// Starts editing the outline of the sector with this code.
    EditArea(String),
    /// A change to the draft of the sector being edited (`Finish` keeps
    /// it if valid, `Cancel` drops it).
    Editing(PolygonEdit),
    /// Replaces the outline of a sector (an agent's edit, already valid).
    SaveGeometry {
        code: String,
        ring_points: Vec<LatLon>,
    },
    /// Botones: wait [`WAIT`] on a task; the button that sent it is busy
    /// meanwhile.
    Wait(&'static str),
    /// The wait of that name ended.
    WaitDone(&'static str),
    /// Save, then reload: two chained tasks.
    SaveAndReload,
    Reload,
    /// A wait whose button stays usable.
    Sync,
    Synced,
}

impl Demo {
    fn can_submit(&self) -> bool {
        !self.name.trim().is_empty()
    }

    fn clear_form(&mut self) {
        self.name.clear();
        self.email.clear();
        self.role = None;
        self.terms = false;
    }

    /// Add `toast` with the next id, and the timer that closes it.
    fn show_toast(&mut self, mut toast: Toast<Msg>) -> Effects<Msg> {
        let id = self.next_toast;
        self.next_toast += 1;
        toast.id = id;
        let timer = toast_timer(&toast, Msg::CloseToast(id));
        self.toasts.push(toast);
        timer
    }

    /// Ask the map to show `bounds` (a new request each time, so the same
    /// box frames again after a drag). Boxes with NaN are ignored.
    fn fit_to(&mut self, bounds: LatLonBounds) {
        let Some(bounds) = LatLonBounds::around([
            LatLon::new(bounds.south, bounds.west),
            LatLon::new(bounds.north, bounds.east),
        ])
        .filter(|b| {
            [b.south, b.west, b.north, b.east]
                .iter()
                .all(|v| v.is_finite())
        }) else {
            return;
        };
        let serial = self.fit_request.map_or(1, |(_, serial)| serial + 1);
        self.fit_request = Some((bounds, serial));
        self.animate_map = true;
    }

    fn sector(&self, code: &str) -> Option<&Sector> {
        self.sectors.iter().find(|sector| sector.code == code)
    }

    /// Select sector `code`, show the sectors and frame it with a margin.
    fn select_area(&mut self, code: &str) {
        let Some(bounds) = self.sector(code).map(|s| s.layer.bounds()) else {
            return;
        };
        self.area = Some(code.to_owned());
        self.show_areas = true;
        self.fit_to(bounds.padded(SECTOR_MARGIN));
    }

    /// Give sector `code` the outline `ring_points`.
    fn save_ring(&mut self, code: &str, ring_points: &[LatLon]) {
        if let Some(sector) = self.sectors.iter_mut().find(|s| s.code == code) {
            sector.with_ring(ring_points);
        }
    }

    fn edit_sector(&mut self, edit: PolygonEdit) {
        let Some((code, draft)) = &mut self.editing else {
            return;
        };
        match edit {
            PolygonEdit::Finish if draft.is_valid() => {
                let (code, ring_points) = (code.clone(), draft.ring().to_vec());
                self.editing = None;
                self.save_ring(&code, &ring_points);
            }
            PolygonEdit::Cancel => self.editing = None,
            edit => {
                draft.apply(edit);
            }
        }
    }
}

/// Saves the rows the client table shows (search, filters, sort; no
/// hidden columns) as an Excel file where the person chooses.
#[derive(Clone, Deserialize, JsonSchema)]
struct ExportTable;

impl Action for ExportTable {
    type App = Demo;
    const ID: &'static str = "table.export";
    const TITLE: &'static str = "Exportar a Excel";
    fn description() -> &'static str {
        "Guarda en un Excel las filas que muestra la tabla de clientes, con su búsqueda, \
         filtros y orden"
    }
    fn available(demo: &Demo) -> Availability {
        if demo.exporting {
            Availability::disabled("Ya se está exportando")
        } else if demo.table_view.is_empty() {
            Availability::disabled("No hay filas que exportar")
        } else {
            Availability::Enabled
        }
    }
    fn into_msg(self) -> Msg {
        Msg::ExportTable
    }
}

/// Waits a moment on a task: its buttons, menu item and palette row are
/// busy until it ends.
#[derive(Clone, Deserialize, JsonSchema)]
struct SimulateWait;

impl Action for SimulateWait {
    type App = Demo;
    const ID: &'static str = "buttons.simulate_wait";
    const TITLE: &'static str = "Simular espera";
    fn description() -> &'static str {
        "Espera un momento en una tarea; mientras, el comando está en curso"
    }
    fn into_msg(self) -> Msg {
        Msg::Wait("command")
    }
}

#[derive(Clone, Deserialize, JsonSchema)]
struct Increment;

impl Action for Increment {
    type App = Demo;
    const ID: &'static str = "counter.increment";
    const TITLE: &'static str = "Incrementar";
    fn description() -> &'static str {
        "Suma un clic al contador"
    }
    fn into_msg(self) -> Msg {
        Msg::Increment
    }
}

/// Adds `count` clicks at once; only scripts and agents call it.
#[derive(Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct Add {
    #[schemars(description = "Clics que se suman")]
    count: u32,
}

impl Action for Add {
    type App = Demo;
    const ID: &'static str = "counter.add";
    const TITLE: &'static str = "Sumar";
    fn description() -> &'static str {
        "Suma varios clics al contador"
    }
    fn into_msg(self) -> Msg {
        Msg::Add(self.count)
    }
}

#[derive(Clone, Deserialize, JsonSchema)]
struct Reset;

impl Action for Reset {
    type App = Demo;
    const ID: &'static str = "counter.reset";
    const TITLE: &'static str = "Reiniciar";
    /// The clicks are lost: an agent needs the person's confirmation.
    const SAFETY: Safety = Safety::Destructive;
    fn description() -> &'static str {
        "Vuelve el contador a cero"
    }
    fn available(demo: &Demo) -> Availability {
        if demo.clicks == 0 {
            Availability::disabled("El contador ya está en cero")
        } else {
            Availability::Enabled
        }
    }
    fn into_msg(self) -> Msg {
        Msg::Reset
    }
}

#[derive(Clone, Deserialize, JsonSchema)]
struct Submit;

impl Action for Submit {
    type App = Demo;
    const ID: &'static str = "form.submit";
    const TITLE: &'static str = "Enviar";
    fn description() -> &'static str {
        "Registra el nombre y limpia el formulario"
    }
    fn available(demo: &Demo) -> Availability {
        if demo.can_submit() {
            Availability::Enabled
        } else {
            Availability::disabled("Falta el nombre")
        }
    }
    fn into_msg(self) -> Msg {
        Msg::Submit
    }
}

#[derive(Clone, Deserialize, JsonSchema)]
struct ClearForm;

impl Action for ClearForm {
    type App = Demo;
    const ID: &'static str = "form.clear";
    const TITLE: &'static str = "Limpiar";
    fn description() -> &'static str {
        "Borra el nombre, el correo, el rol y los términos"
    }
    fn into_msg(self) -> Msg {
        Msg::Clear
    }
}

#[derive(Clone, Deserialize, JsonSchema)]
struct TogglePasswordAction;

impl Action for TogglePasswordAction {
    type App = Demo;
    const ID: &'static str = "form.toggle_password";
    const TITLE: &'static str = "Mostrar u ocultar contraseña";
    fn into_msg(self) -> Msg {
        Msg::TogglePassword
    }
}

/// Shows a section, as a click on the sidebar does.
#[derive(Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct GoTo {
    #[schemars(description = "Sección que se muestra")]
    section: Section,
}

impl Action for GoTo {
    type App = Demo;
    const ID: &'static str = "ui.go_to";
    const TITLE: &'static str = "Ir a sección";
    fn description() -> &'static str {
        "Muestra una sección de la demo, como un clic en la barra lateral"
    }
    fn into_msg(self) -> Msg {
        Msg::GoTo(self.section)
    }
}

#[derive(Clone, Deserialize, JsonSchema)]
struct ToggleSidebarAction;

impl Action for ToggleSidebarAction {
    type App = Demo;
    const ID: &'static str = "ui.toggle_sidebar";
    const TITLE: &'static str = "Mostrar u ocultar barra lateral";
    fn description() -> &'static str {
        "Colapsa la barra lateral a un riel de íconos o la expande"
    }
    fn into_msg(self) -> Msg {
        Msg::ToggleSidebar
    }
}

/// Shows a toast, as the buttons of «Diálogos y avisos» do.
#[derive(Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct ShowToastAction {
    #[schemars(description = "Variante del aviso")]
    kind: ToastKind,
}

impl Action for ShowToastAction {
    type App = Demo;
    const ID: &'static str = "toasts.show";
    const TITLE: &'static str = "Mostrar aviso";
    fn description() -> &'static str {
        "Muestra un aviso (toast) que se cierra solo a los 4 segundos"
    }
    fn into_msg(self) -> Msg {
        Msg::ShowToast(self.kind)
    }
}

/// Shows the image at a URL in the avatar of «Multimedia».
#[derive(Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct LoadUrl {
    #[schemars(
        description = "Dirección http(s) de una imagen png, jpg o webp; vacía, \
                              vuelve al ícono"
    )]
    url: String,
}

impl Action for LoadUrl {
    type App = Demo;
    const ID: &'static str = "media.load_url";
    const TITLE: &'static str = "Cargar imagen desde URL";
    fn description() -> &'static str {
        "Descarga una imagen y la muestra en el avatar de la sección Multimedia"
    }
    fn into_msg(self) -> Msg {
        Msg::LoadUrl(self.url)
    }
}

/// Moves the map of «Mapa», as a drag or the zoom buttons would.
#[derive(Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct SetMapView {
    #[schemars(description = "Centro del mapa (grados)")]
    center: LatLon,
    #[schemars(description = "Zoom de 0 (el mundo entero) a 20 (una calle)")]
    zoom: f64,
    #[schemars(description = "Deslizarse hasta la vista (por omisión) o saltar")]
    #[serde(default)]
    animate: Option<bool>,
}

impl Action for SetMapView {
    type App = Demo;
    const ID: &'static str = "map.set_view";
    const TITLE: &'static str = "Mover el mapa";
    fn description() -> &'static str {
        "Centra el mapa de la sección Mapa en un punto, con el zoom dado"
    }
    fn into_msg(self) -> Msg {
        Msg::SetMapView {
            view: MapView::new(self.center, self.zoom),
            animate: self.animate.unwrap_or(true),
        }
    }
}

/// Documento: negrita (WordCraft `format.bold`).
#[derive(Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct DocBold;

impl Action for DocBold {
    type App = Demo;
    const ID: &'static str = "format.bold";
    const TITLE: &'static str = "Negrita";
    fn description() -> &'static str {
        "Aplica o quita negrita en la selección del documento de la demo"
    }
    fn into_msg(self) -> Msg {
        Msg::DocCmd(DocumentCmd::Bold)
    }
}

#[derive(Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct DocItalic;

impl Action for DocItalic {
    type App = Demo;
    const ID: &'static str = "format.italic";
    const TITLE: &'static str = "Cursiva";
    fn description() -> &'static str {
        "Aplica o quita cursiva en la selección del documento de la demo"
    }
    fn into_msg(self) -> Msg {
        Msg::DocCmd(DocumentCmd::Italic)
    }
}

#[derive(Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct DocList;

impl Action for DocList {
    type App = Demo;
    const ID: &'static str = "para.bullets";
    const TITLE: &'static str = "Lista con viñetas";
    fn description() -> &'static str {
        "Convierte el párrafo del documento en una lista con viñetas"
    }
    fn into_msg(self) -> Msg {
        Msg::DocCmd(DocumentCmd::Bullets)
    }
}

#[derive(Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct DocUndo;

impl Action for DocUndo {
    type App = Demo;
    const ID: &'static str = "edit.undo";
    const TITLE: &'static str = "Deshacer";
    fn description() -> &'static str {
        "Deshace el último cambio del documento"
    }
    fn into_msg(self) -> Msg {
        Msg::DocCmd(DocumentCmd::Undo)
    }
}

#[derive(Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct DocRedo;

impl Action for DocRedo {
    type App = Demo;
    const ID: &'static str = "edit.redo";
    const TITLE: &'static str = "Rehacer";
    fn description() -> &'static str {
        "Rehace el último cambio deshecho del documento"
    }
    fn into_msg(self) -> Msg {
        Msg::DocCmd(DocumentCmd::Redo)
    }
}

#[derive(Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct DocTable;

impl Action for DocTable {
    type App = Demo;
    const ID: &'static str = "insert.table";
    const TITLE: &'static str = "Insertar tabla";
    fn description() -> &'static str {
        "Inserta una tabla 2×2 en el documento (WordCraft)"
    }
    fn into_msg(self) -> Msg {
        Msg::DocCmd(DocumentCmd::InsertTable)
    }
}

#[derive(Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct DocImage;

impl Action for DocImage {
    type App = Demo;
    const ID: &'static str = "insert.picture";
    const TITLE: &'static str = "Insertar imagen";
    fn description() -> &'static str {
        "Inserta una imagen de demostración en el documento (WordCraft + atlas Kai)"
    }
    fn into_msg(self) -> Msg {
        Msg::DocCmd(DocumentCmd::InsertPicture)
    }
}

#[derive(Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct DocOpen;

impl Action for DocOpen {
    type App = Demo;
    const ID: &'static str = "document.open";
    const TITLE: &'static str = "Abrir documento";
    fn description() -> &'static str {
        "Abre un archivo .docx en el editor de Documento (WordCraft)"
    }
    fn available(demo: &Demo) -> Availability {
        if demo.doc_file_busy {
            Availability::disabled("Ya hay una operación de archivo en curso")
        } else {
            Availability::Enabled
        }
    }
    fn into_msg(self) -> Msg {
        Msg::DocOpen
    }
}

#[derive(Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct DocSave;

impl Action for DocSave {
    type App = Demo;
    const ID: &'static str = "document.save";
    const TITLE: &'static str = "Guardar documento";
    fn description() -> &'static str {
        "Guarda el documento como .docx donde elijas"
    }
    fn available(demo: &Demo) -> Availability {
        if demo.doc_file_busy {
            Availability::disabled("Ya hay una operación de archivo en curso")
        } else {
            Availability::Enabled
        }
    }
    fn into_msg(self) -> Msg {
        Msg::DocSave
    }
}

#[derive(Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct DocExportPdf;

impl Action for DocExportPdf {
    type App = Demo;
    const ID: &'static str = "document.export_pdf";
    const TITLE: &'static str = "Exportar PDF";
    fn description() -> &'static str {
        "Exporta el documento a PDF (WordCraft / craft-pdf) donde elijas"
    }
    fn available(demo: &Demo) -> Availability {
        if demo.doc_file_busy {
            Availability::disabled("Ya hay una operación de archivo en curso")
        } else {
            Availability::Enabled
        }
    }
    fn into_msg(self) -> Msg {
        Msg::DocExportPdf
    }
}

#[derive(Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct DocFind;

impl Action for DocFind {
    type App = Demo;
    const ID: &'static str = "document.find";
    const TITLE: &'static str = "Buscar y reemplazar";
    fn description() -> &'static str {
        "Muestra u oculta la barra Buscar y reemplazar del documento"
    }
    fn into_msg(self) -> Msg {
        Msg::DocToggleFind
    }
}

/// Frames a box on the map of «Mapa».
#[derive(Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct FitBounds {
    #[schemars(description = "Caja a mostrar entera (grados)")]
    bounds: LatLonBounds,
}

impl Action for FitBounds {
    type App = Demo;
    const ID: &'static str = "map.fit_bounds";
    const TITLE: &'static str = "Encuadrar el mapa";
    fn description() -> &'static str {
        "Mueve el mapa de la sección Mapa hasta mostrar entera una caja de latitud y longitud"
    }
    fn into_msg(self) -> Msg {
        Msg::FitBounds(self.bounds)
    }
}

/// Selects a DSP sector of «Mapa», as a click on it or on the list would.
#[derive(Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct SelectArea {
    #[schemars(
        description = "Código del sector: AOP26000001 (Plaza de Armas), AOP26000002 \
                       (Catillo), AOP26000003 (Villa San Pedro), AOP26000004 (Palma Rosa) \
                       o AOP26000005 (Terminal de Buses)"
    )]
    code: String,
}

impl Action for SelectArea {
    type App = Demo;
    const ID: &'static str = "areas.select";
    const TITLE: &'static str = "Seleccionar sector";
    fn description() -> &'static str {
        "Selecciona un sector de DSP en el mapa de la sección Mapa y lo encuadra con un 20 % \
         de margen; un código desconocido, o un sector en edición, no cambia nada"
    }
    fn into_msg(self) -> Msg {
        Msg::SelectArea(self.code)
    }
}

/// The outline an agent proposes for a sector, before it is checked.
#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct ProposedGeometry {
    #[schemars(description = "Código del sector, como en areas.select")]
    code: String,
    #[schemars(
        description = "Anillo exterior del polígono como en GeoJSON: [longitud, latitud] por \
                       vértice, al menos 3, sin bordes que se crucen; el primero puede repetirse \
                       al final"
    )]
    coordinates: Vec<[f64; 2]>,
}

/// Replaces the outline of a DSP sector of «Mapa»; the input is checked
/// as it is read, so an invalid one is an input error.
#[derive(Clone, Deserialize, JsonSchema)]
#[serde(try_from = "ProposedGeometry")]
struct EditGeometry {
    code: String,
    ring_points: Vec<LatLon>,
}

impl TryFrom<ProposedGeometry> for EditGeometry {
    type Error = String;

    fn try_from(proposed: ProposedGeometry) -> Result<Self, String> {
        if initial_sector(&proposed.code).is_none() {
            return Err(format!("no hay un sector {}", proposed.code));
        }
        let draft = PolygonDraft::from_ring(
            proposed
                .coordinates
                .iter()
                .map(|[lon, lat]| LatLon::new(*lat, *lon)),
        )?;
        if let Some(bad) = draft
            .ring()
            .iter()
            .find(|at| at.lat.abs() > 90.0 || at.lon.abs() > 180.0)
        {
            return Err(format!("[{}, {}] está fuera del mundo", bad.lon, bad.lat));
        }
        if let Some((a, b)) = crossing(draft.ring()) {
            return Err(format!("los bordes {} y {} se cruzan", a + 1, b + 1));
        }
        Ok(Self {
            code: proposed.code,
            ring_points: draft.ring().to_vec(),
        })
    }
}

impl Action for EditGeometry {
    type App = Demo;
    const ID: &'static str = "areas.edit_geometry";
    const TITLE: &'static str = "Editar la geometría de un sector";
    fn description() -> &'static str {
        "Reemplaza el polígono de un sector de DSP en el mapa de la sección Mapa, lo selecciona y \
         lo encuadra; una geometría inválida es un error y no cambia nada"
    }
    fn into_msg(self) -> Msg {
        Msg::SaveGeometry {
            code: self.code,
            ring_points: self.ring_points,
        }
    }
}

/// What the map of «Mapa» shows, as of the last gesture.
#[derive(Deserialize, JsonSchema)]
struct MapViewQuery;

impl Query for MapViewQuery {
    type App = Demo;
    type Output = MapView;
    const ID: &'static str = "map.get_view";
    const TITLE: &'static str = "Vista del mapa";
    fn run(demo: &Demo, _: Self) -> Result<MapView, CommandError> {
        Ok(demo.view)
    }
}

/// The last name sent, if any.
#[derive(Deserialize, JsonSchema)]
struct LastSubmission;

impl Query for LastSubmission {
    type App = Demo;
    type Output = Option<String>;
    const ID: &'static str = "form.last_submission";
    const TITLE: &'static str = "Último envío";
    fn run(demo: &Demo, _: Self) -> Result<Option<String>, CommandError> {
        Ok(demo.submissions.last().cloned())
    }
}

impl App for Demo {
    type Msg = Msg;

    fn commands(registry: &mut CommandRegistry<Self>) {
        registry
            .action::<Increment>()
            .shortcut(Shortcut::cmd(Key::Char('i')))
            .menu("Contador")
            .expose();
        registry.action::<Add>().expose();
        registry
            .action::<Reset>()
            .shortcut(Shortcut::cmd(Key::Char('0')))
            .menu("Contador")
            .expose();
        registry
            .action::<Submit>()
            .menu("Formulario")
            .returns::<LastSubmission>()
            .expose();
        registry.action::<ClearForm>().menu("Formulario").expose();
        // Showing the password is the user's business, not an agent's.
        registry.action::<TogglePasswordAction>().menu("Formulario");
        registry.query::<LastSubmission>().expose();
        // It takes a section, so it has no menu item nor shortcut.
        registry.action::<GoTo>().expose();
        registry
            .action::<ToggleSidebarAction>()
            .shortcut(Shortcut::ctrl_cmd(Key::Char('s')))
            .menu("Ver")
            .expose();
        // It takes a variant: no menu item, as `ui.ir_a`.
        registry.action::<ShowToastAction>().expose();
        registry.action::<LoadUrl>().expose();
        registry.action::<SetMapView>().expose();
        registry.action::<FitBounds>().expose();
        registry.action::<SelectArea>().expose();
        registry.action::<EditGeometry>().expose();
        registry.query::<MapViewQuery>().expose();
        registry.action::<SimulateWait>().menu("Botones").expose();
        registry.action::<ExportTable>().menu("Archivo").expose();
        registry.action::<DocBold>().menu("Documento").expose();
        registry.action::<DocItalic>().menu("Documento").expose();
        registry.action::<DocList>().menu("Documento").expose();
        registry.action::<DocTable>().menu("Documento").expose();
        registry.action::<DocImage>().menu("Documento").expose();
        registry.action::<DocOpen>().menu("Documento").expose();
        registry.action::<DocSave>().menu("Documento").expose();
        registry.action::<DocExportPdf>().menu("Documento").expose();
        registry
            .action::<DocFind>()
            .shortcut(Shortcut::cmd(Key::Char('f')))
            .menu("Documento")
            .expose();
        registry.action::<DocUndo>().menu("Documento").expose();
        registry.action::<DocRedo>().menu("Documento").expose();
    }

    fn theme(&self) -> Option<Theme> {
        (self.section == Section::Catalog).then(|| self.catalog_theme())
    }

    fn resized(&self, size: Size) -> Option<Msg> {
        (!self.sized).then_some(Msg::InitialWidth {
            narrow: size.width < 640.0,
        })
    }

    fn image_cache(&self) -> Option<DiskCache> {
        DiskCache::for_app(APP_ID, "imagenes", DiskCache::DEFAULT_LIMIT)
    }

    fn dev_state(&self) -> Option<serde_json::Value> {
        serde_json::to_value(DevState {
            section: Some(self.section),
            sidebar_collapsed: Some(self.sidebar_collapsed),
            area: self.area.clone(),
        })
        .ok()
    }

    fn restore_dev_state(&mut self, state: serde_json::Value) {
        let Ok(state_value) = serde_json::from_value::<DevState>(state) else {
            return;
        };
        if let Some(section) = state_value.section {
            self.section = section;
        }
        if let Some(collapsed) = state_value.sidebar_collapsed {
            self.sidebar_collapsed = collapsed;
            // The first window size must not undo the restored sidebar.
            self.sized = true;
        }
        if let Some(code) = state_value.area {
            self.select_area(&code);
            self.animate_map = false;
        }
    }

    fn title(&self) -> String {
        let base = format!("Kai UI Desktop v{}", version::VERSION);
        if self.clicks == 0 {
            base
        } else {
            format!("{base} — {} clics", self.clicks)
        }
    }

    fn update(&mut self, msg: Msg) -> Effects<Msg> {
        let effects = self.update_model(msg);
        Effects::batch([effects, self.save_preferences()])
    }

    fn background(&self) -> BackgroundEffect {
        BackgroundEffect::new(self.background).with_motion(self.background_motion)
    }

    fn view(&self) -> Element<Msg> {
        let version = format!("v{}", version::VERSION);
        let header = toolbar([
            toolbar_item(
                Icon::PanelLeft,
                if self.sidebar_collapsed {
                    "Mostrar barra lateral"
                } else {
                    "Ocultar barra lateral"
                },
            )
            .on_command(ToggleSidebarAction)
            .test_id("bar.sidebar"),
            toolbar_title("Kai UI Desktop", Some(&version)),
            toolbar_spacer().test_id("bar.espacio"),
            toolbar_item(Icon::Plus, "Sumar clic")
                .on_command(Increment)
                .test_id("bar.add"),
            toolbar_separator().test_id("bar.separador"),
            toolbar_search(
                search_field(&self.query, Msg::Query)
                    .placeholder("Filtrar clientes")
                    .label("Filtrar clientes")
                    .on_submit(Msg::GoTo(Section::Table))
                    .test_id("bar.find")
                    .build(),
            ),
        ]);

        let alert_banner = alert(
            AlertVariant::Info,
            "Componentes sincronizados con la biblioteca web de Kai (React/TypeScript)",
        );

        let counter = card()
            .class("w-full max-w-3xl")
            .test_id("counter")
            .children([
                card_header(
                    "Contador reactivo",
                    Some("El estado vive en el modelo; cada mensaje vuelve a ejecutar la vista"),
                ),
                card_content().children([
                    div().class("flex items-end gap-2").children([
                        text(self.clicks.to_string())
                            .class("text-3xl font-bold text-foreground")
                            .role(Role::Status),
                        caption(if self.clicks == 1 { "clic" } else { "clics" }).class("pb-1"),
                    ]),
                    paragraph(
                        "Los botones usan las mismas clases que Kai web: hover, foco con anillo, \
                     estado activo y deshabilitado.",
                    ),
                ]),
                card_actions().children([
                    button("Reiniciar", ButtonVariant::Outlined).on_command(Reset),
                    button("Incrementar", ButtonVariant::Primary).on_command(Increment),
                ]),
            ]);

        let greeting = if self.name.trim().is_empty() {
            caption("Escribe tu nombre para ver la vista reaccionar.")
        } else {
            text(format!("Hola, {} 👋", self.name.trim()))
                .class("text-base font-medium text-primary")
        };
        let form = card().class("w-full max-w-3xl").test_id("form").children([
            card_header(
                "Formulario controlado",
                Some("Cada tecla emite un mensaje con el nuevo valor"),
            ),
            card_content().class("gap-4").children([
                page_grid().children([
                    // Return here activates «Enviar», the default button.
                    page_grid_item(6).child(
                        TextField::new("Nombre", &self.name)
                            .test_id("form.name")
                            .field_icon(FieldKind::Name)
                            .on_input(Msg::NameChanged)
                            .build(),
                    ),
                    page_grid_item(6).child(
                        TextField::new("Correo", &self.email)
                            .test_id("form.email")
                            .field_icon(FieldKind::Email)
                            .on_input(Msg::EmailChanged)
                            .on_submit(Msg::Submit)
                            .build(),
                    ),
                    page_grid_item(6).child(
                        select(&ROLES, self.role, "Elegir rol", Msg::SetRole)
                            .label("Rol")
                            .test_id("form.role"),
                    ),
                    page_grid_item(6).child(
                        checkbox(
                            "Acepto los términos",
                            if self.terms {
                                ControlState::On
                            } else {
                                ControlState::Off
                            },
                            Msg::ToggleTerms,
                        )
                        .test_id("form.terms"),
                    ),
                ]),
                div()
                    .class("flex items-center justify-between pt-1")
                    .children([
                        greeting.role(Role::Status),
                        switch_labeled(
                            "Notificaciones",
                            self.notifications,
                            Msg::ToggleNotifications,
                            false,
                            SwitchDensity::Compact,
                        )
                        .test_id("form.notificaciones"),
                    ]),
            ]),
            card_actions().children([
                button("Limpiar", ButtonVariant::Text).on_command(ClearForm),
                button("Enviar", ButtonVariant::Primary)
                    .on_command(Submit)
                    .default_button(true),
            ]),
        ]);

        let last = self
            .submissions
            .last()
            .map_or("Sin envíos todavía".to_owned(), |name| {
                format!("Último: {name}")
            });
        let stats = div()
            .class("grid grid-cols-3 gap-4 w-full max-w-3xl")
            .children([
                stat("Clics", self.clicks.to_string(), None),
                stat(
                    "Caracteres",
                    self.name.chars().count().to_string(),
                    Some("en el nombre"),
                ),
                stat("Envíos", self.submissions.len().to_string(), Some(&last)),
            ]);

        let badges_and_alerts = card()
            .class("w-full max-w-3xl")
            .test_id("badges")
            .children([
                card_header(
                    "Badges y Alertas",
                    Some("Indicadores de estado y mensajes del sistema"),
                ),
                card_content().class("gap-4").children([
                    div().class("flex flex-col gap-2").children([
                        caption("Badges sólidos:"),
                        div().class("flex flex-wrap items-center gap-2").children([
                            badge("Primary", BadgeVariant::Primary),
                            badge("Secondary", BadgeVariant::Secondary),
                            badge("Success", BadgeVariant::Success),
                            badge("Error", BadgeVariant::Error),
                            badge("Warning", BadgeVariant::Warning),
                            badge("Info", BadgeVariant::Info),
                        ]),
                        caption("Badges con borde:"),
                        div().class("flex flex-wrap items-center gap-2").children([
                            badge("Primary", BadgeVariant::PrimaryOutlined),
                            badge("Secondary", BadgeVariant::SecondaryOutlined),
                            badge("Success", BadgeVariant::SuccessOutlined),
                            badge("Error", BadgeVariant::ErrorOutlined),
                            badge("Warning", BadgeVariant::WarningOutlined),
                            badge("Info", BadgeVariant::InfoOutlined),
                        ]),
                    ]),
                    div().class("grid grid-cols-2 gap-3 pt-2").children([
                        alert(AlertVariant::Success, "Operación guardada con éxito"),
                        alert(AlertVariant::Warning, "Revisa los campos requeridos"),
                    ]),
                ]),
            ]);

        let buttons_section = card().class("w-full max-w-3xl").children([
            card_header(
                "Botones, Tamaños e IconButtons",
                Some("Variantes visuales, tamaños y botones de iconos"),
            ),
            card_content().class("gap-4").children([
                div().class("flex flex-wrap items-center gap-3").children([
                    button("Primary", ButtonVariant::Primary).on_click(Msg::Increment),
                    button("Secondary", ButtonVariant::Secondary)
                        .test_id("buttons.secondary")
                        .on_click(Msg::Increment),
                    button("Outlined", ButtonVariant::Outlined).on_click(Msg::Increment),
                    button("Text", ButtonVariant::Text).on_click(Msg::Increment),
                    button("Danger", ButtonVariant::Danger).on_click(Msg::Reset),
                    button("Deshabilitado", ButtonVariant::Primary)
                        .on_click(Msg::Increment)
                        .disabled(true),
                ]),
                div().class("flex items-center gap-3 pt-2").children([
                    caption("Tamaños:"),
                    button_sized("Pequeño", ButtonVariant::Primary, ButtonSize::Sm),
                    button_sized("Mediano", ButtonVariant::Primary, ButtonSize::Md),
                    button_sized("Grande", ButtonVariant::Primary, ButtonSize::Lg),
                ]),
                div().class("flex items-center gap-3 pt-2").children([
                    caption("IconButtons:"),
                    icon_button_sized(
                        Icon::Search,
                        "Buscar",
                        IconButtonVariant::Primary,
                        IconButtonSize::Md,
                    ),
                    icon_button_sized(
                        Icon::User,
                        "Usuario",
                        IconButtonVariant::Secondary,
                        IconButtonSize::Md,
                    ),
                    icon_button_sized(
                        Icon::Info,
                        "Info",
                        IconButtonVariant::Action,
                        IconButtonSize::Md,
                    ),
                    icon_button_sized(
                        Icon::Trash2,
                        "Eliminar",
                        IconButtonVariant::Outlined,
                        IconButtonSize::Md,
                    ),
                ]),
            ]),
        ]);

        let cards = match self.section {
            Section::Home => vec![
                self.background_panel(),
                div().class("w-full max-w-3xl").child(alert_banner),
                counter,
                form,
                stats,
            ],
            Section::Fields => vec![self.fields(), self.more_fields(), self.field_states()],
            Section::Buttons => vec![
                buttons_section,
                self.button_states(),
                self.waiting_buttons(),
                self.appkit_buttons(),
                self.control_sizes(),
            ],
            Section::Selection => vec![self.selection()],
            Section::Containers => self.containers(),
            Section::Badges => vec![badges_and_alerts],
            Section::Multimedia => vec![self.avatars(), self.image_fields()],
            Section::Table => vec![
                self.client_table(),
                self.simple_list(),
                self.patterns_empty_loading(),
            ],
            Section::Map => vec![self.map_view()],
            Section::Charts => self.charts_view(),
            Section::Document => vec![self.document_view()],
            Section::Dialogs => vec![self.dialogs_and_toasts()],
            Section::TemplateList => vec![self.template_list()],
            Section::TemplateForm => vec![self.template_form()],
            Section::TemplateData => vec![self.template_data()],
            Section::Catalog => vec![self.live_catalog()],
        };

        let content = if self.section == Section::Document {
            // Pages metaphor: ribbon + archivo + full-height canvas (no card / scroll shell).
            div()
                .class("flex-1 flex flex-col min-h-0 min-w-0 bg-under-page")
                .key(self.section.key())
                .test_id("content")
                .children(cards)
        } else {
            // Page shell (Fase B): Page → PageBody as one flex child of the
            // sidebar row (overflow-hidden on Page forces the body to scroll).
            page()
                .key(self.section.key())
                .test_id("content")
                .child(page_body().children(cards))
        };
        let mut layers = vec![
            header,
            div()
                .class("flex-1 flex min-h-0")
                .children([self.sidebar(), content]),
            toast_stack(&self.toasts, Msg::CloseToast),
        ];
        if let Some(dialog_kind) = self.dialog_kind {
            layers.push(self.dialog(dialog_kind));
        }
        div()
            // `neutral` is systemGray in AppKit: use the window background.
            .class("relative w-full h-full flex flex-col bg-neutral appkit:bg-window")
            .role(Role::Main)
            .children(layers)
    }
}

impl Demo {
    fn preferences(&self) -> DemoPreferences {
        DemoPreferences {
            section: self.section,
            sidebar_collapsed: self.sidebar_collapsed,
            map_view_state: self.view,
            catalog: [
                self.catalog_preset.index(),
                usize::from(self.catalog_density == Density::Compact),
                CATALOG_SIZES
                    .iter()
                    .position(|s| *s == self.catalog_size)
                    .unwrap_or(2),
            ],
        }
    }

    /// Restaura lo guardado. La ventana ya no pliega la barra lateral sola:
    /// manda lo que la persona dejó.
    fn apply_preferences(&mut self, p: &DemoPreferences) {
        self.section = p.section;
        self.sidebar_collapsed = p.sidebar_collapsed;
        self.sized = true;
        self.view = p.map_view_state;
        let [preset, density, size] = p.catalog;
        self.catalog_preset = CatalogPreset::ALL
            .get(preset)
            .copied()
            .unwrap_or(self.catalog_preset);
        self.catalog_density = if density == 1 {
            Density::Compact
        } else {
            Density::Comfortable
        };
        self.catalog_size = CATALOG_SIZES
            .get(size)
            .copied()
            .unwrap_or(self.catalog_size);
        self.saved_preferences = Some(p.clone());
    }

    /// Guarda las preferencias si cambiaron desde lo último guardado.
    fn save_preferences(&mut self) -> Effects<Msg> {
        let Some(store) = &self.store else {
            return Effects::none();
        };
        let current_prefs = self.preferences();
        if self.saved_preferences.as_ref() == Some(&current_prefs) {
            return Effects::none();
        }
        let effect = store.save_effect(&current_prefs, Msg::PreferencesSaved);
        self.saved_preferences = Some(current_prefs);
        effect
    }

    /// Todo el `update` de la demo; [`App::update`] le suma el guardado
    /// de preferencias.
    fn update_model(&mut self, msg: Msg) -> Effects<Msg> {
        match msg {
            Msg::ShowToast(kind) => {
                let (_, title, message) = kind.texts();
                let mut toast = Toast::new(0, kind.alert(), title).message(message);
                if kind == ToastKind::Error {
                    toast = toast.action("Reintentar", Msg::ShowToast(ToastKind::Success));
                }
                return self.show_toast(toast);
            }
            Msg::CloseToast(id) => {
                self.toasts.retain(|toast| toast.id != id);
                return Effects::cancel(toast_key(id));
            }
            Msg::ConfirmDialog => {
                let toast = match self.dialog_kind.take() {
                    Some(DemoDialog::Save) => {
                        let (_, title, message) = ToastKind::Success.texts();
                        Toast::new(0, AlertVariant::Success, title).message(message)
                    }
                    Some(DemoDialog::Delete) => {
                        Toast::new(0, AlertVariant::Info, "Borrador eliminado")
                            .message("Se movió a la papelera")
                    }
                    None => return Effects::none(),
                };
                return self.show_toast(toast);
            }
            Msg::Wait(name) => {
                let mut effect =
                    Effects::perform(|| std::thread::sleep(WAIT), move |()| Msg::WaitDone(name));
                if let Some(invocation) = current_invocation() {
                    effect = effect.completes(invocation);
                }
                return effect;
            }
            Msg::WaitDone(name) => self.wait_ms = Some(name),
            Msg::SaveAndReload => {
                return Effects::perform(|| std::thread::sleep(WAIT / 2), |()| Msg::Reload);
            }
            Msg::Reload => {
                return Effects::perform(
                    || std::thread::sleep(WAIT / 2),
                    |()| Msg::WaitDone("guardar y recargar"),
                );
            }
            Msg::Sync => {
                self.syncing += 1;
                return Effects::perform(|| std::thread::sleep(WAIT), |()| Msg::Synced);
            }
            Msg::Synced => self.syncing = self.syncing.saturating_sub(1),
            Msg::Increment => self.clicks += 1,
            Msg::Add(amount) => self.clicks = self.clicks.saturating_add(amount),
            Msg::Reset => self.clicks = 0,
            Msg::NameChanged(name) => self.name = name,
            Msg::EmailChanged(email) => self.email = email,
            Msg::Submit => {
                if self.can_submit() {
                    self.submissions.push(self.name.trim().to_owned());
                    self.clear_form();
                }
            }
            Msg::Clear => self.clear_form(),
            Msg::Edit(field, value) => {
                *match field {
                    Field::Search => &mut self.search,
                    Field::Password => &mut self.password,
                    Field::Amount => &mut self.amount,
                    Field::Discount => &mut self.discount,
                    Field::Commune => &mut self.commune,
                    Field::Code => &mut self.code,
                    Field::Quantity => &mut self.quantity,
                    Field::Autocomplete => &mut self.autocomplete,
                    Field::Pin => &mut self.pin,
                    Field::Notes => &mut self.notes,
                    Field::Lookup => &mut self.lookup,
                    Field::Url => &mut self.url,
                } = value;
            }
            Msg::PickImage(image_field) => {
                let title = match image_field {
                    ImageField::Photo => "Elegir foto de perfil",
                    ImageField::Cover => "Elegir portada",
                    ImageField::Logo => "Elegir logo",
                };
                return Effects::pick_file(PickFile::images(title), move |path| {
                    Msg::ImagePicked(image_field, path)
                });
            }
            Msg::ImagePicked(image_field, Some(path)) => match image_field {
                ImageField::Photo => self.photo.set(path),
                ImageField::Cover => self.cover.set(path),
                ImageField::Logo => self.logo.set(path),
            },
            Msg::ImagePicked(_, None) => {}
            Msg::LoadUrl(url) => {
                let url = url.trim().to_owned();
                self.avatar_url = (!url.is_empty()).then(|| url.clone());
                self.url = url;
            }
            Msg::TogglePassword => self.show_password = !self.show_password,
            Msg::ToggleNotifications => self.notifications = !self.notifications,
            Msg::SetRole(index) => self.role = Some(index),
            Msg::ToggleTerms => self.terms = !self.terms,
            Msg::GoTo(section) => self.section = section,
            Msg::TemplateSelect(index) => self.template_index = index,
            Msg::CatalogPreset(index) => {
                self.catalog_preset = CatalogPreset::ALL
                    .get(index)
                    .copied()
                    .unwrap_or(CatalogPreset::AppKitLight);
            }
            Msg::CatalogDensity(index) => {
                self.catalog_density = if index == 0 {
                    Density::Comfortable
                } else {
                    Density::Compact
                };
            }
            Msg::CatalogSize(index) => {
                self.catalog_size = CATALOG_SIZES
                    .get(index)
                    .copied()
                    .unwrap_or(ControlSize::Regular);
            }
            Msg::CatalogSample(value) => self.catalog_sample = value,
            Msg::CatalogClearFilter => self.catalog_sample.clear(),
            Msg::CatalogToggleCheck => self.catalog_check = !self.catalog_check,
            Msg::CatalogToggleSwitch => self.catalog_switch = !self.catalog_switch,
            Msg::CatalogSelect(index) => self.catalog_select = Some(index),
            Msg::CatalogTab(index) => self.catalog_tab = index,
            Msg::ToggleSidebar => self.sidebar_collapsed = !self.sidebar_collapsed,
            Msg::InitialWidth { narrow } => {
                self.sidebar_collapsed = narrow;
                self.sized = true;
            }
            Msg::SetBackground(kind) => self.background = kind,
            Msg::ToggleBackgroundMotion => {
                self.background_motion = match self.background_motion {
                    BackgroundMotion::Static => BackgroundMotion::Animated,
                    BackgroundMotion::Animated => BackgroundMotion::Static,
                };
            }
            Msg::TogglePin => self.show_pin = !self.show_pin,
            Msg::ToggleWifi => self.wifi = !self.wifi,
            Msg::TogglePermission(Some(index)) => {
                self.permissions[index] = !self.permissions[index]
            }
            Msg::TogglePermission(None) => {
                let all = !self.permissions.iter().all(|on| *on);
                self.permissions = [all; 3];
            }
            Msg::SetCountry(index) => self.country = Some(index),
            Msg::SetAlignment(index) => self.alignment = index,
            Msg::SetTab(index) => self.tab_index = index,
            Msg::Query(query) => {
                self.table_view.query.clone_from(&query);
                self.query = query;
                self.table_view.refresh(&self.clients, &table_columns());
            }
            Msg::Table(change) => {
                let typing = matches!(change, ViewChange::Filter(..));
                self.table_view.apply(change);
                if typing {
                    return Effects::after(FILTER_PAUSE, Msg::RefreshTable).keyed("filter");
                }
                self.table_view.refresh(&self.clients, &table_columns());
            }
            Msg::RefreshTable => self.table_view.refresh(&self.clients, &table_columns()),
            Msg::SelectRow(index) => self.client = Some(index),
            Msg::ShowClient(index) => {
                let Some(client) = self.clients.get(index) else {
                    return Effects::none();
                };
                let toast = Toast::new(0, AlertVariant::Info, client.name.as_str())
                    .message(format!("Cliente N.º {} de {}", client.number, client.city));
                return self.show_toast(toast);
            }
            Msg::ExportTable => {
                if self.exporting || self.table_view.is_empty() {
                    return Effects::none();
                }
                let request = SaveFile::new("Exportar a Excel", "clientes.xlsx").types(&["xlsx"]);
                return Effects::save_file(request, Msg::SaveExcel);
            }
            Msg::SaveExcel(None) => {}
            Msg::SaveExcel(Some(path)) => {
                self.exporting = true;
                let sheet = self.table_view.export(&self.clients, &table_columns());
                return Effects::perform(
                    move || {
                        crate::xlsx::write(&sheet, "Clientes", &path)?;
                        let name = path.file_name().map_or_else(
                            || path.display().to_string(),
                            |name| name.to_string_lossy().into_owned(),
                        );
                        Ok((name, sheet.rows.len()))
                    },
                    Msg::TableExported,
                );
            }
            Msg::TableExported(result) => {
                self.exporting = false;
                let toast = match result {
                    Ok((name, rows)) => Toast::new(0, AlertVariant::Success, "Excel exportado")
                        .message(format!("{rows} filas en {name}")),
                    Err(error) => {
                        Toast::new(0, AlertVariant::Error, "No se exportó").message(error)
                    }
                };
                return self.show_toast(toast);
            }
            Msg::OpenDialog(dialog_kind) => self.dialog_kind = Some(dialog_kind),
            Msg::CloseDialog => self.dialog_kind = None,
            Msg::MapMoved(view) => self.view = view,
            Msg::SetMapView { view, animate } => {
                self.view = view.clamped();
                self.animate_map = animate;
            }
            Msg::SelectArea(code) => {
                if self.editing.is_none() {
                    self.select_area(&code);
                }
            }
            Msg::FitBounds(bounds) => self.fit_to(bounds),
            Msg::ToggleAreas => self.show_areas = !self.show_areas,
            Msg::Month(index) => self.month = Some(index),
            Msg::Document(change) => {
                self.doc_markdown = change.markdown;
                #[cfg(feature = "document")]
                {
                    self.doc_ribbon_pressed = change.ribbon_pressed;
                    self.doc_ribbon_disabled = change.ribbon_disabled;
                    self.doc_ribbon_contexts = change.ribbon_contexts;
                    if !change.status.is_empty() {
                        self.doc_edit_status = change.status;
                    }
                }
                #[cfg(feature = "document")]
                if let Some(export) = change.export
                    && let Some((path, kind)) = self.doc_pending_export.take()
                {
                    self.doc_file_busy = true;
                    let doc = (*export.0).clone();
                    return Effects::perform(
                        move || write_document_file(doc, path, kind),
                        Msg::DocFileReady,
                    );
                }
                #[cfg(not(feature = "document"))]
                if self.doc_pending_export.take().is_some() {
                    return Effects::perform(
                        || Err("Compila con --features document".into()),
                        Msg::DocFileReady,
                    );
                }
            }
            Msg::DocCmd(cmd) => {
                self.doc_cmd_serial = self.doc_cmd_serial.saturating_add(1);
                self.doc_cmd = Some((cmd, self.doc_cmd_serial));
            }
            Msg::DocRibbonTab(id) => {
                self.doc_ribbon_tab = id;
            }
            Msg::DocToggleFind => self.doc_find_open = !self.doc_find_open,
            Msg::PreferencesSaved(Ok(())) => {}
            Msg::PreferencesSaved(Err(e)) => {
                eprintln!("No se guardaron las preferencias: {e}");
            }
            Msg::DocToggleComment => self.doc_comment_open = !self.doc_comment_open,
            Msg::DocFindChanged(v) => self.doc_find = v,
            Msg::DocReplaceChanged(v) => self.doc_replace = v,
            Msg::DocCommentDraftChanged(v) => self.doc_comment_draft = v,
            Msg::DocOpen => {
                if self.doc_file_busy {
                    return Effects::none();
                }
                let request = PickFile::new("Abrir documento").types(&["docx"]);
                return Effects::pick_file(request, Msg::DocOpenChosen);
            }
            Msg::DocOpenChosen(None) => {}
            Msg::DocOpenChosen(Some(path)) => {
                self.doc_file_busy = true;
                let name = path
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_else(|| path.display().to_string());
                self.doc_file_name = Some(name);
                return Effects::perform(
                    move || std::fs::read(&path).map_err(|e| e.to_string()),
                    Msg::DocBytesRead,
                );
            }
            Msg::DocBytesRead(result) => {
                self.doc_file_busy = false;
                match result {
                    Ok(bytes) => {
                        self.doc_load_serial = self.doc_load_serial.saturating_add(1);
                        self.doc_load = Some((self.doc_load_serial, bytes.into()));
                        let toast = Toast::new(0, AlertVariant::Success, "Documento abierto")
                            .message("Se cargó el archivo .docx");
                        return self.show_toast(toast);
                    }
                    Err(error) => {
                        let toast =
                            Toast::new(0, AlertVariant::Error, "No se abrió").message(error);
                        return self.show_toast(toast);
                    }
                }
            }
            Msg::DocSave => {
                if self.doc_file_busy {
                    return Effects::none();
                }
                let request = SaveFile::new("Guardar documento", "informe.docx").types(&["docx"]);
                return Effects::save_file(request, Msg::DocSaveTo);
            }
            Msg::DocSaveTo(None) => {}
            Msg::DocSaveTo(Some(path)) => {
                let name = path
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_else(|| path.display().to_string());
                self.doc_file_name = Some(name);
                self.doc_pending_export = Some((path, DocExportKind::Docx));
                self.doc_cmd_serial = self.doc_cmd_serial.saturating_add(1);
                self.doc_cmd = Some((DocumentCmd::TakeDocument, self.doc_cmd_serial));
            }
            Msg::DocExportPdf => {
                if self.doc_file_busy {
                    return Effects::none();
                }
                let request = SaveFile::new("Exportar PDF", "informe.pdf").types(&["pdf"]);
                return Effects::save_file(request, Msg::DocExportPdfTo);
            }
            Msg::DocExportPdfTo(None) => {}
            Msg::DocExportPdfTo(Some(path)) => {
                // No cambia el nombre del .docx activo; solo toast al terminar.
                self.doc_pending_export = Some((path, DocExportKind::Pdf));
                self.doc_cmd_serial = self.doc_cmd_serial.saturating_add(1);
                self.doc_cmd = Some((DocumentCmd::TakeDocument, self.doc_cmd_serial));
            }
            Msg::DocFileReady(result) => {
                self.doc_file_busy = false;
                let toast = match result {
                    Ok(name) => Toast::new(0, AlertVariant::Success, "Archivo listo").message(name),
                    Err(error) => {
                        Toast::new(0, AlertVariant::Error, "No se pudo escribir").message(error)
                    }
                };
                return self.show_toast(toast);
            }
            Msg::ColorSector(color) => {
                let code = self.area.clone();
                if let Some(sector) = self
                    .sectors
                    .iter_mut()
                    .find(|s| Some(&s.code) == code.as_ref())
                {
                    sector.color = color;
                }
            }
            Msg::EditArea(code) => {
                if self.editing.is_none()
                    && let Some(ring_points) = self.sector(&code).map(Sector::ring_points)
                    && let Ok(draft) = PolygonDraft::from_ring(ring_points)
                {
                    self.select_area(&code);
                    self.editing = Some((code, draft));
                }
            }
            Msg::Editing(edit) => self.edit_sector(edit),
            Msg::SaveGeometry { code, ring_points } => {
                if self
                    .editing
                    .as_ref()
                    .is_some_and(|(editing, _)| *editing == code)
                {
                    self.editing = None;
                }
                self.save_ring(&code, &ring_points);
                if self.editing.is_none() {
                    self.select_area(&code);
                }
            }
        }
        Effects::none()
    }
}

impl Demo {
    fn sidebar(&self) -> Element<Msg> {
        let mut home_view = Section::Home.item();
        if !self.submissions.is_empty() {
            home_view = home_view.badge(self.submissions.len().to_string());
        }
        let groups = [
            SidebarGroup::titled("Demo", [home_view]),
            SidebarGroup::titled(
                "Componentes",
                [
                    Section::Fields,
                    Section::Buttons,
                    Section::Selection,
                    Section::Containers,
                    Section::Badges,
                    Section::Multimedia,
                ]
                .map(Section::item),
            ),
            SidebarGroup::titled(
                "Datos",
                [
                    Section::Table.item(),
                    Section::Map.item(),
                    Section::Charts.item(),
                    Section::Document.item(),
                ],
            ),
            SidebarGroup::titled(
                "Plantillas",
                [
                    Section::TemplateList.item(),
                    Section::TemplateForm.item(),
                    Section::TemplateData.item(),
                ],
            ),
            SidebarGroup::titled("Catálogo", [Section::Catalog.item()]),
            SidebarGroup::titled("Superposiciones", [Section::Dialogs.item()]),
        ];
        Sidebar::new(&groups, self.section.index(), |index| {
            Msg::GoTo(Section::ALL[index])
        })
        .collapsed(self.sidebar_collapsed)
        .label("Secciones")
        .test_id("nav")
        .build()
        .class(if self.sidebar_collapsed {
            ""
        } else {
            "w-40 sm:w-52 shrink-0"
        })
        .class("border-r border-border/70 bg-background appkit:border-separator appkit:bg-fill-4")
    }

    fn catalog_theme(&self) -> Theme {
        let base = match self.catalog_preset {
            CatalogPreset::AppKitLight => {
                Theme::appkit_static(AppKitGeneration::Aqua11, Appearance::Light)
            }
            CatalogPreset::AppKitDark => {
                Theme::appkit_static(AppKitGeneration::Aqua11, Appearance::Dark)
            }
            CatalogPreset::GlassLight => {
                Theme::appkit_static(AppKitGeneration::Glass27, Appearance::Light)
            }
            CatalogPreset::GlassDark => {
                Theme::appkit_static(AppKitGeneration::Glass27, Appearance::Dark)
            }
            CatalogPreset::Kai => Theme::kai(self.catalog_density),
        };
        Theme {
            control_size: self.catalog_size,
            density: match self.catalog_preset {
                CatalogPreset::Kai => self.catalog_density,
                // Catalog lets density move independently of size for exploration.
                _ => self.catalog_density,
            },
            ..base
        }
    }

    /// Fase E — catálogo vivo: filtros tema × density × size + muestras.
    fn live_catalog(&self) -> Element<Msg> {
        let size = self.catalog_size;
        let field_density = match self.catalog_density {
            Density::Comfortable => FieldDensity::Default,
            Density::Compact => FieldDensity::Compact,
        };
        let preset_labels = [
            CatalogPreset::AppKitLight.label(),
            CatalogPreset::AppKitDark.label(),
            CatalogPreset::GlassLight.label(),
            CatalogPreset::GlassDark.label(),
            CatalogPreset::Kai.label(),
        ];
        let density_idx = match self.catalog_density {
            Density::Comfortable => 0,
            Density::Compact => 1,
        };
        let size_idx = size.index();
        let filters = card()
            .class("w-full max-w-4xl")
            .test_id("catalog.filters")
            .children([
                card_header(
                    "Filtros",
                    Some("Tema (App::theme mientras esta sección está abierta), densidad y ControlSize"),
                ),
                card_content().class("gap-4").children([
                    div().class("flex flex-col gap-1.5").children([
                        caption("Tema"),
                        segmented_with_options(
                            &preset_labels,
                            self.catalog_preset.index(),
                            Msg::CatalogPreset,
                            false,
                        )
                        .test_id("catalog.preset"),
                    ]),
                    div().class("flex flex-col gap-1.5").children([
                        caption("Densidad"),
                        segmented_with_options(
                            &["Comfortable", "Compact"],
                            density_idx,
                            Msg::CatalogDensity,
                            false,
                        )
                        .test_id("catalog.density"),
                    ]),
                    div().class("flex flex-col gap-1.5").children([
                        caption("Tamaño (ControlSize)"),
                        segmented_with_options(
                            &["Mini", "Small", "Regular", "Large", "XL"],
                            size_idx,
                            Msg::CatalogSize,
                            false,
                        )
                        .test_id("catalog.size"),
                    ]),
                ]),
            ]);

        let buttons = card()
            .class("w-full max-w-4xl")
            .test_id("catalog.buttons")
            .children([
                card_header(
                    "Button",
                    Some("Variantes + disabled + loading al tamaño filtrado"),
                ),
                card_content().class("gap-3").children([div()
                    .class("flex flex-wrap items-center gap-2")
                    .children([
                        button("Primary", ButtonVariant::Primary)
                            .control_size(size)
                            .on_click(Msg::Increment),
                        button("Secondary", ButtonVariant::Secondary)
                            .control_size(size)
                            .on_click(Msg::Increment),
                        button("Outlined", ButtonVariant::Outlined)
                            .control_size(size)
                            .on_click(Msg::Increment),
                        button("Danger", ButtonVariant::Danger)
                            .control_size(size)
                            .on_click(Msg::Increment),
                        button("Disabled", ButtonVariant::Primary)
                            .control_size(size)
                            .disabled(true),
                        Button::new("Loading")
                            .accent(true)
                            .loading(true)
                            .control_size(size)
                            .on_click(Msg::Increment)
                            .build(),
                    ])]),
            ]);

        let fields = card()
            .class("w-full max-w-4xl")
            .test_id("catalog.fields")
            .children([
                card_header(
                    "TextField",
                    Some(
                        "Default / required / helper / error / disabled · FieldDensity del filtro",
                    ),
                ),
                card_content()
                    .class("gap-4")
                    .children([page_grid().children([
                        page_grid_item(6).child(
                            TextField::new("Nombre", &self.catalog_sample)
                                .placeholder("Valor de ejemplo")
                                .field_icon(FieldKind::Name)
                                .density(field_density)
                                .on_input(Msg::CatalogSample)
                                .test_id("catalog.campo.default")
                                .build()
                                .control_size(size),
                        ),
                        page_grid_item(6).child(
                            TextField::new("Correo", "ana@example.com")
                                .field_icon(FieldKind::Email)
                                .density(field_density)
                                .required(true)
                                .test_id("catalog.campo.required")
                                .build()
                                .control_size(size),
                        ),
                        page_grid_item(6).child(
                            TextField::new("Búsqueda", "")
                                .placeholder("Escribe para filtrar")
                                .field_icon(FieldKind::Search)
                                .helper_text("Puedes usar palabras clave")
                                .density(field_density)
                                .test_id("catalog.campo.helper")
                                .build()
                                .control_size(size),
                        ),
                        page_grid_item(6).child(
                            TextField::new("Correo", "no-es-un-correo")
                                .field_icon(FieldKind::Email)
                                .helper_text("Introduce un correo válido")
                                .error(true)
                                .density(field_density)
                                .disabled(false)
                                .test_id("catalog.campo.error")
                                .build()
                                .control_size(size),
                        ),
                    ])]),
            ]);

        let switch_density = match self.catalog_density {
            Density::Comfortable => SwitchDensity::Default,
            Density::Compact => SwitchDensity::Compact,
        };
        let check_state = if self.catalog_check {
            ControlState::On
        } else {
            ControlState::Off
        };
        let choices = card()
            .class("w-full max-w-4xl")
            .test_id("catalog.eleccion")
            .children([
                card_header(
                    "Checkbox · Switch · Select",
                    Some("Estados útiles; checkbox/select respetan ControlSize del tema; switch usa SwitchDensity"),
                ),
                card_content().class("gap-4").children([
                    div().class("flex flex-col gap-2").children([
                        caption("Checkbox"),
                        div()
                            .class("flex flex-wrap items-center gap-4")
                            .children([
                                checkbox_with_options(
                                    "Activo",
                                    check_state,
                                    Msg::CatalogToggleCheck,
                                    false,
                                )
                                .test_id("catalog.check"),
                                checkbox_with_options(
                                    "Mixto",
                                    ControlState::Mixed,
                                    Msg::Increment,
                                    false,
                                )
                                .test_id("catalog.check.mixed"),
                                checkbox_with_options(
                                    "Deshabilitado",
                                    ControlState::On,
                                    Msg::Increment,
                                    true,
                                )
                                .test_id("catalog.check.disabled"),
                            ]),
                    ]),
                    div().class("flex flex-col gap-2").children([
                        caption("Switch"),
                        div()
                            .class("flex flex-wrap items-center gap-4")
                            .children([
                                switch_labeled(
                                    "Notificaciones",
                                    self.catalog_switch,
                                    Msg::CatalogToggleSwitch,
                                    false,
                                    switch_density,
                                )
                                .test_id("catalog.switch"),
                                switch_labeled(
                                    "Deshabilitado",
                                    true,
                                    Msg::Increment,
                                    true,
                                    switch_density,
                                )
                                .test_id("catalog.switch.disabled"),
                            ]),
                    ]),
                    div().class("flex flex-col gap-2 max-w-xs").children([
                        caption("Select"),
                        select_with_options(
                            &["Diseño", "Desarrollo", "Producto"],
                            self.catalog_select,
                            "Elegir rol",
                            Msg::CatalogSelect,
                            false,
                        )
                        .label("Rol")
                        .control_size(size)
                        .test_id("catalog.select"),
                        select_with_options(
                            &["Diseño", "Desarrollo", "Producto"],
                            Some(1),
                            "Elegir rol",
                            Msg::CatalogSelect,
                            true,
                        )
                        .label("Deshabilitado")
                        .control_size(size)
                        .test_id("catalog.select.disabled"),
                    ]),
                ]),
            ]);

        let badges = card()
            .class("w-full max-w-4xl")
            .test_id("catalog.badges")
            .children([
                card_header(
                    "Badge · Alert",
                    Some("Variantes sólidas y outlined (no pills WIP)"),
                ),
                card_content().class("gap-3").children([
                    div().class("flex flex-wrap items-center gap-2").children([
                        badge("Primary", BadgeVariant::Primary),
                        badge("Success", BadgeVariant::Success),
                        badge("Warning", BadgeVariant::Warning),
                        badge("Error", BadgeVariant::Error),
                        badge("Info", BadgeVariant::Info),
                        badge("Outlined", BadgeVariant::PrimaryOutlined),
                        badge("Success □", BadgeVariant::SuccessOutlined),
                    ]),
                    alert(AlertVariant::Info, "Alerta informativa de ejemplo"),
                    alert(AlertVariant::Warning, "Revisa los campos requeridos"),
                ]),
            ]);

        let progress_size = match size {
            ControlSize::Mini | ControlSize::Small => DotSize::Sm,
            ControlSize::Regular => DotSize::Md,
            ControlSize::Large | ControlSize::ExtraLarge => DotSize::Lg,
        };
        let feedback = card()
            .class("w-full max-w-4xl")
            .test_id("catalog.feedback")
            .children([
                card_header(
                    "DotProgress · Tabs",
                    Some("Progreso indeterminado (tamaño ≈ ControlSize) y pestañas"),
                ),
                card_content().class("gap-4").children([
                    div()
                        .class(
                            "flex flex-wrap items-center gap-6 text-foreground appkit:text-label",
                        )
                        .children([
                            div().class("flex flex-col gap-1").children([
                                caption("DotProgress"),
                                DotProgress::new()
                                    .size(progress_size)
                                    .label("Cargando catálogo")
                                    .test_id("catalog.progress")
                                    .build(),
                            ]),
                            div().class("flex flex-col gap-1").children([
                                caption("Sm (referencia)"),
                                DotProgress::new().size(DotSize::Sm).decorative().build(),
                            ]),
                        ]),
                    Tabs::new(self.catalog_tab, Msg::CatalogTab)
                        .tab(
                            "General",
                            paragraph("Contenido de la pestaña General bajo el tema filtrado."),
                        )
                        .tab("Avanzado", paragraph("Contenido de la pestaña Avanzado."))
                        .build()
                        .test_id("catalog.tabs"),
                ]),
            ]);

        let empties = card()
            .class("w-full max-w-4xl")
            .test_id("catalog.empty")
            .children([
                card_header(
                    "Empty filter vs empty resource",
                    Some("EmptyKind::Filter (búsqueda sin filas) · EmptyKind::Resource (colección vacía)"),
                ),
                card_content().class("gap-4").children([
                    page_grid().children([
                        page_grid_item(6).child(
                            div()
                                .class(
                                    "rounded-md border border-border appkit:border-separator \
                                     overflow-hidden bg-control",
                                )
                                .child(
                                    EmptyState::filter("No hay resultados")
                                        .description("Ningún ítem coincide con el filtro.")
                                        .action("Limpiar filtros", Msg::CatalogClearFilter)
                                        .test_id("catalog.empty.filter")
                                        .build(),
                                ),
                        ),
                        page_grid_item(6).child(
                            div()
                                .class(
                                    "rounded-md border border-border appkit:border-separator \
                                     overflow-hidden bg-control",
                                )
                                .child(
                                    EmptyState::resource("Sin elementos")
                                        .description("Todavía no hay nada en esta colección.")
                                        .action("Crear elemento", Msg::Increment)
                                        .test_id("catalog.empty.resource")
                                        .build(),
                                ),
                        ),
                    ]),
                ]),
            ]);

        let surface = |name: &str, class: &str| {
            div()
                .class(&format!(
                    "flex flex-col gap-1 min-h-20 rounded-md border border-border/60 \
                     appkit:border-separator p-3 {class}"
                ))
                .children([
                    caption(name),
                    text("Aa").class("text-sm text-foreground appkit:text-label"),
                ])
        };
        let layers = card()
            .class("w-full max-w-4xl")
            .test_id("catalog.capas")
            .children([
                card_header(
                    "Capas AppKit",
                    Some("window / under-page / control / fill — ver docs/dev/24-layering.md"),
                ),
                card_content()
                    .class("gap-3")
                    .children([page_grid().children([
                        page_grid_item(3).child(surface("bg-window", "bg-window")),
                        page_grid_item(3).child(surface("bg-under-page", "bg-under-page")),
                        page_grid_item(3).child(surface("bg-control", "bg-control")),
                        page_grid_item(3).child(surface("bg-fill-3", "bg-fill-3")),
                    ])]),
            ]);

        div()
            .class("flex flex-col gap-5 w-full")
            .test_id("catalog")
            .children([
                filters, buttons, fields, choices, badges, feedback, empties, layers,
            ])
    }

    /// Fase C — lista + detalle (master/detail) sobre PageGrid.
    fn template_list(&self) -> Element<Msg> {
        const CLIENT_COUNT: [(&str, &str, &str); 5] = [
            ("Ana Ruiz", "ana.ruiz@ejemplo.cl", "Activa"),
            ("Bruno Vega", "bruno.vega@ejemplo.cl", "Activo"),
            ("Carla Soto", "carla.soto@ejemplo.cl", "Pendiente"),
            ("Diego Núñez", "diego.nunez@ejemplo.cl", "Activo"),
            ("Elena Paredes", "elena.paredes@ejemplo.cl", "Inactiva"),
        ];
        let idx = self.template_index.min(CLIENT_COUNT.len() - 1);
        let list = div()
            .class(
                "flex flex-col gap-1 w-full rounded-lg border border-border/70 \
                 appkit:border-separator appkit:rounded-md overflow-hidden bg-card appkit:bg-control",
            )
            .part("items")
            .children(CLIENT_COUNT.iter().enumerate().map(|(i, (name, email, _))| {
                let selected = i == idx;
                div()
                    .class(if selected {
                        "flex flex-col gap-0.5 px-3 py-2 cursor-pointer \
                         bg-accent/15 appkit:bg-selected-content"
                    } else {
                        "flex flex-col gap-0.5 px-3 py-2 cursor-pointer \
                         hover:bg-muted/40 appkit:hover:bg-fill-3"
                    })
                    .role(Role::Button)
                    .on_click(Msg::TemplateSelect(i))
                    .test_id(format!("template.items.row.{i}"))
                    .children([
                        text(*name).class(
                            "text-sm font-medium text-foreground appkit:text-label",
                        ),
                        text(*email).class(
                            "text-xs text-muted-foreground appkit:text-secondary-label",
                        ),
                    ])
            }));
        let (name, email, state_value) = CLIENT_COUNT[idx];
        let detail = div()
            .class(
                "flex flex-col gap-3 w-full min-h-40 rounded-lg border border-border/70 \
                 appkit:border-separator appkit:rounded-md p-4 bg-card appkit:bg-control",
            )
            .part("panel")
            .children([
                heading(name),
                caption(format!("{email} · {state_value}")),
                paragraph(
                    "Panel de detalle: en una app real aquí van fichas, historial o acciones \
                     sobre el ítem seleccionado en la lista.",
                ),
                div().class("flex flex-wrap gap-2 pt-1").children([
                    button("Editar", ButtonVariant::Primary)
                        .on_click(Msg::Increment)
                        .test_id("template.items.edit"),
                    button("Archivar", ButtonVariant::Outlined)
                        .on_click(Msg::Reset)
                        .test_id("template.items.archive"),
                ]),
            ]);
        MasterDetailPage::new(list, detail)
            .header(
                PageHeader::new("Clientes")
                    .subtitle("Lista + detalle · span 4/8 · gutter gap-4 (step 4)")
                    .breadcrumbs(["Demo", "Plantillas", "Lista + detalle"])
                    .action(
                        button("Nuevo", ButtonVariant::Primary)
                            .on_click(Msg::Increment)
                            .test_id("template.items.new"),
                    ),
            )
            .list_span(4)
            .test_id("template.items")
            .build()
    }

    /// Fase C — formulario comfortable con PageGrid (spans 6/12).
    fn template_form(&self) -> Element<Msg> {
        FormPage::new()
            .header(
                PageHeader::new("Nuevo cliente")
                    .subtitle("Densidad comfortable · gap-5 (step 5) · campos span 6 / 12")
                    .breadcrumbs(["Demo", "Plantillas", "Formulario"]),
            )
            .fields([
                form_field_cell(
                    TextField::new("Nombre", &self.name)
                        .placeholder("Ana Ruiz")
                        .on_input(Msg::NameChanged)
                        .test_id("template.form.name")
                        .build(),
                ),
                form_field_cell(
                    TextField::new("Correo", &self.email)
                        .placeholder("ana@ejemplo.cl")
                        .on_input(Msg::EmailChanged)
                        .test_id("template.form.email")
                        .build(),
                ),
                form_full_cell(
                    TextField::new("Notas", &self.notes)
                        .placeholder("Contexto, preferencias…")
                        .on_input(|v| Msg::Edit(Field::Notes, v))
                        .test_id("template.form.notes")
                        .build(),
                ),
            ])
            .actions([
                button("Cancelar", ButtonVariant::Text)
                    .on_click(Msg::Clear)
                    .test_id("template.form.cancel"),
                button("Guardar", ButtonVariant::Primary)
                    .on_click(Msg::Submit)
                    .test_id("template.form.save"),
            ])
            .test_id("template.form")
            .build()
    }

    /// Fase C — vista de datos compacta (tabla / métricas densas).
    fn template_data(&self) -> Element<Msg> {
        let metrics = page_grid().children([
            page_grid_item(4).child(
                div()
                    .class(
                        "flex flex-col gap-1 rounded-md border border-border/70 \
                         appkit:border-separator p-3 bg-card appkit:bg-control",
                    )
                    .children([
                        caption("Ingresos"),
                        text("$ 128.4k")
                            .class("text-lg font-semibold text-foreground appkit:text-label"),
                    ]),
            ),
            page_grid_item(4).child(
                div()
                    .class(
                        "flex flex-col gap-1 rounded-md border border-border/70 \
                         appkit:border-separator p-3 bg-card appkit:bg-control",
                    )
                    .children([
                        caption("Conversiones"),
                        text("3.2 %")
                            .class("text-lg font-semibold text-foreground appkit:text-label"),
                    ]),
            ),
            page_grid_item(4).child(
                div()
                    .class(
                        "flex flex-col gap-1 rounded-md border border-border/70 \
                         appkit:border-separator p-3 bg-card appkit:bg-control",
                    )
                    .children([
                        caption("Tickets abiertos"),
                        text("47").class("text-lg font-semibold text-foreground appkit:text-label"),
                    ]),
            ),
        ]);
        let rows = [
            ("#1042", "Ana Ruiz", "Pagado", "$ 240"),
            ("#1043", "Bruno Vega", "Pendiente", "$ 180"),
            ("#1044", "Carla Soto", "Pagado", "$ 95"),
            ("#1045", "Diego Núñez", "Fallido", "$ 310"),
            ("#1046", "Elena Paredes", "Pagado", "$ 120"),
            ("#1047", "Felipe Chandía", "Pendiente", "$ 75"),
        ];
        let table = div()
            .class(
                "w-full rounded-md border border-border/70 appkit:border-separator \
                 overflow-hidden bg-card appkit:bg-control",
            )
            .part("table")
            .children({
                let mut kids: Vec<Element<Msg>> = vec![
                    div()
                        .class(
                            "grid grid-cols-4 gap-2 px-3 py-1.5 text-xs font-medium \
                         text-muted-foreground appkit:text-secondary-label \
                         bg-muted/30 appkit:bg-fill-3 border-b border-border/50 \
                         appkit:border-separator",
                        )
                        .children([
                            text("Orden"),
                            text("Cliente"),
                            text("Estado"),
                            text("Monto"),
                        ]),
                ];
                for (i, (sort_order, client, state_value, amount)) in rows.iter().enumerate() {
                    kids.push(
                        div()
                            .class(
                                "grid grid-cols-4 gap-2 px-3 py-1.5 text-sm \
                                 text-foreground appkit:text-label border-b \
                                 border-border/40 appkit:border-separator last:border-0",
                            )
                            .test_id(format!("template.data.row.{i}"))
                            .children([
                                text(*sort_order),
                                text(*client),
                                text(*state_value),
                                text(*amount),
                            ]),
                    );
                }
                kids
            });
        DataPage::new()
            .header(
                PageHeader::new("Operaciones")
                    .subtitle("Densidad compact · gap-3 (step 3) · métricas + tabla densa")
                    .breadcrumbs(["Demo", "Plantillas", "Vista de datos"])
                    .action(
                        button("Exportar", ButtonVariant::Outlined)
                            .on_click(Msg::ExportTable)
                            .test_id("template.data.export"),
                    ),
            )
            .child(metrics)
            .child(table)
            .test_id("template.data")
            .build()
    }

    /// Matriz de estados de Button: variantes + disabled + loading.
    fn button_states(&self) -> Element<Msg> {
        let row = |label: &str, children: Vec<Element<Msg>>| {
            div().class("flex flex-col gap-2").children([
                caption(label),
                div()
                    .class("flex flex-wrap items-center gap-3")
                    .children(children),
            ])
        };
        card()
            .class("w-full max-w-3xl")
            .test_id("buttons.estados")
            .children([
                card_header(
                    "Estados de Button",
                    Some(
                        "Primary / Secondary / Outlined / Danger, disabled y loading                          (hover nativo del tema)",
                    ),
                ),
                card_content().class("gap-4").children([
                    row(
                        "Variantes",
                        vec![
                            button("Primary", ButtonVariant::Primary),
                            button("Secondary", ButtonVariant::Secondary),
                            button("Outlined", ButtonVariant::Outlined),
                            button("Danger", ButtonVariant::Danger),
                        ],
                    ),
                    row(
                        "Disabled",
                        vec![
                            button("Primary", ButtonVariant::Primary).disabled(true),
                            button("Secondary", ButtonVariant::Secondary).disabled(true),
                            button("Outlined", ButtonVariant::Outlined).disabled(true),
                            button("Danger", ButtonVariant::Danger).disabled(true),
                        ],
                    ),
                    row(
                        "Loading",
                        vec![
                            Button::new("Primary")
                                .accent(true)
                                .loading(true)
                                .on_click(Msg::Increment)
                                .build(),
                            Button::new("Secondary")
                                .loading(true)
                                .on_click(Msg::Increment)
                                .build(),
                            button("Outlined", ButtonVariant::Outlined)
                                .on_click(Msg::Increment)
                                .busy(true),
                            Button::new("Danger")
                                .destructive(true)
                                .loading(true)
                                .on_click(Msg::Increment)
                                .build(),
                        ],
                    ),
                ]),
            ])
    }

    /// Buttons that start a task (`Effects::perform`): busy, with the dots
    /// over their label, until it ends. A chained save and reload, one
    /// that opts out, one bound to a command, one the app keeps loading,
    /// and `DotProgress` alone in its three sizes.
    fn waiting_buttons(&self) -> Element<Msg> {
        let variants = [
            ("Primary", ButtonVariant::Primary),
            ("Secondary", ButtonVariant::Secondary),
            ("Outlined", ButtonVariant::Outlined),
            ("Text", ButtonVariant::Text),
            ("Danger", ButtonVariant::Danger),
        ];
        let status = match (self.wait_ms, self.syncing) {
            (_, 1..) => format!("Sincronizando ({})…", self.syncing),
            (Some(name), _) => format!("Terminó: {name}"),
            (None, _) => "Pulsa un botón: espera 1,5 s".to_owned(),
        };
        card()
            .class("w-full max-w-3xl")
            .test_id("buttons.wait")
            .children([
                card_header(
                    "Acciones con espera",
                    Some(
                        "El botón que lanza una tarea muestra los puntos y no responde \
                         hasta que termina",
                    ),
                ),
                card_content().class("gap-4").children([
                    div()
                        .class("flex flex-wrap items-center gap-3")
                        .children(variants.map(|(name, variant)| {
                            button(name, variant)
                                .on_click(Msg::Wait(name))
                                .test_id(format!("wait.{}", name.to_lowercase()))
                        })),
                    div().class("flex flex-wrap items-center gap-3").children([
                        Button::new("Guardar y recargar")
                            .accent(true)
                            .on_click(Msg::SaveAndReload)
                            .build()
                            .test_id("wait.reload"),
                        Button::new("Sincronizar en segundo plano")
                            .busy_feedback(false)
                            .on_click(Msg::Sync)
                            .build()
                            .test_id("wait.sync"),
                        button("Simular espera (comando)", ButtonVariant::Outlined)
                            .on_command(SimulateWait),
                        Button::new("Cargando")
                            .loading(true)
                            .build()
                            .test_id("wait.cargando"),
                    ]),
                    div()
                        .class("flex items-center gap-6 pt-2 text-primary")
                        .children([
                            caption("DotProgress:"),
                            DotProgress::new()
                                .size(DotSize::Sm)
                                .test_id("wait.puntos.sm")
                                .build(),
                            dot_progress().test_id("espera.puntos.md"),
                            DotProgress::new()
                                .size(DotSize::Lg)
                                .test_id("wait.puntos.lg")
                                .build(),
                        ]),
                    caption(status).role(Role::Status).test_id("wait.state"),
                ]),
            ])
    }

    /// Buttons of the AppKit API (`Button::new`): the three bezels, accent,
    /// destructive, cancel (Escape) and disabled, and an `icon_button`.
    fn appkit_buttons(&self) -> Element<Msg> {
        let built = |button: Button<Msg>, id: &str| button.build().test_id(id);
        card()
            .class("w-full max-w-3xl")
            .test_id("buttons.appkit")
            .children([
                card_header(
                    "Button::new",
                    Some("Biseles push, bordered y borderless; acento, destructivo y Esc"),
                ),
                card_content().class("gap-4").children([
                    div().class("flex flex-wrap items-center gap-3").children([
                        built(
                            Button::new("Push")
                                .bezel(BezelStyle::Push)
                                .on_click(Msg::Increment),
                            "buttons.push",
                        ),
                        built(
                            Button::new("Bordered")
                                .bezel(BezelStyle::Bordered)
                                .on_click(Msg::Increment),
                            "buttons.bordered",
                        ),
                        built(
                            Button::new("Borderless")
                                .bezel(BezelStyle::Borderless)
                                .on_click(Msg::Increment),
                            "buttons.borderless",
                        ),
                    ]),
                    div().class("flex flex-wrap items-center gap-3").children([
                        built(
                            Button::new("Acento").accent(true).on_click(Msg::Increment),
                            "buttons.acento",
                        ),
                        built(
                            Button::new("Destructivo")
                                .destructive(true)
                                .on_click(Msg::Reset),
                            "buttons.destructivo",
                        ),
                        built(
                            Button::new("Cancelar (Esc)")
                                .cancel_button()
                                .on_click(Msg::Reset),
                            "buttons.cancel",
                        ),
                        built(
                            Button::new("Pequeño")
                                .control_size(ControlSize::Small)
                                .disabled(true),
                            "buttons.pequeno",
                        ),
                        icon_button(Icon::Settings, "Ajustes", IconButtonVariant::Secondary)
                            .on_click(Msg::Increment)
                            .test_id("buttons.icono"),
                    ]),
                ]),
            ])
    }

    /// Campos, second card: a plain `password` field, the AppKit rounded
    /// bezel, adornments, the placeholder color, a search field and a
    /// text area.
    fn more_fields(&self) -> Element<Msg> {
        let edit = |field: Field| move |value: String| Msg::Edit(field, value);
        card()
            .class("w-full max-w-3xl")
            .test_id("fields.mas")
            .children([
                card_header(
                    "Más campos",
                    Some("Bisel redondeado, adornos, color del placeholder, búsqueda y notas"),
                ),
                card_content().class("gap-4").children([
                    div()
                        .class("grid grid-cols-1 sm:grid-cols-2 gap-x-4 gap-y-5")
                        .children([
                            TextField::new("PIN", &self.pin)
                                .test_id("fields.pin")
                                .password(self.show_pin, Msg::TogglePin)
                                .on_input(edit(Field::Pin))
                                .build(),
                            TextField::new("Usuario", &self.lookup)
                                .test_id("fields.redondeado")
                                .bezel(FieldBezel::Rounded)
                                .placeholder("Placeholder en color primario")
                                .placeholder_color("primary/60")
                                .on_input(edit(Field::Lookup))
                                .build(),
                            TextField::new("Sitio", "kai.cl")
                                .test_id("fields.adornos")
                                .start_adornment(text("https://").class(
                                    "text-sm text-muted-foreground appkit:text-secondary-label",
                                ))
                                .end_adornment(
                                    icon(Icon::CircleCheckBig).class("size-4 text-success"),
                                )
                                .read_only(true)
                                .build(),
                            search_field(&self.lookup, edit(Field::Lookup))
                                .variant(SearchVariant::Field)
                                .placeholder("Buscar en la ayuda")
                                .label("Buscar en la ayuda")
                                .test_id("fields.busqueda")
                                .build(),
                        ]),
                    text_area("Notas", &self.notes)
                        .test_id("fields.notes")
                        .placeholder("Varias líneas; Retorno agrega una")
                        .auto_grow(3, 6)
                        .max_length(280)
                        .counter(true)
                        .helper_text("Crece hasta 6 líneas")
                        .on_input(edit(Field::Notes))
                        .build(),
                ]),
            ])
    }

    /// Selección: switches, checkboxes (one mixed), selects and segmented
    /// controls, enabled and disabled.
    fn selection(&self) -> Element<Msg> {
        let state = |on: bool| {
            if on {
                ControlState::On
            } else {
                ControlState::Off
            }
        };
        let all = match self.permissions.iter().filter(|on| **on).count() {
            0 => ControlState::Off,
            3 => ControlState::On,
            _ => ControlState::Mixed,
        };
        let label = |content: &str| {
            text(content).class("text-sm font-medium text-foreground appkit:text-label")
        };
        let mut checks = vec![
            checkbox_with_options("Todos", all, Msg::TogglePermission(None), false)
                .test_id("selection.all"),
        ];
        for (index, name) in PERMISSIONS.iter().enumerate() {
            checks.push(
                checkbox_with_options(
                    name,
                    state(self.permissions[index]),
                    Msg::TogglePermission(Some(index)),
                    false,
                )
                .class("ml-6")
                .test_id(format!("selection.permiso.{index}")),
            );
        }
        checks.push(
            checkbox_with_options(
                "Administrar",
                ControlState::Off,
                Msg::TogglePermission(None),
                true,
            )
            .class("ml-6")
            .test_id("selection.administrar"),
        );
        card()
            .class("w-full max-w-3xl")
            .test_id("selection")
            .children([
                card_header(
                    "Selección",
                    Some("Switch, checkbox con estado mixto, select y control segmentado"),
                ),
                card_content().class("gap-5").children([
                    div().class("flex flex-wrap items-center gap-6").children([
                        div().class("flex items-center gap-2").children([
                            switch(self.wifi, Msg::ToggleWifi)
                                .label("Wi-Fi")
                                .test_id("selection.wifi"),
                            label("Wi-Fi"),
                        ]),
                        div().class("flex items-center gap-2").children([
                            switch_with_options(
                                true,
                                Msg::ToggleWifi,
                                true,
                                SwitchDensity::Compact,
                            )
                            .label("Bluetooth")
                            .test_id("selection.bluetooth"),
                            label("Bluetooth (deshabilitado)"),
                        ]),
                    ]),
                    div()
                        .class("flex flex-col items-start gap-2")
                        .children(checks),
                    div()
                        .class("grid grid-cols-1 sm:grid-cols-2 gap-4")
                        .children([
                            select_with_options(
                                &COUNTRIES,
                                self.country,
                                "Elegir país",
                                Msg::SetCountry,
                                false,
                            )
                            .label("País")
                            .test_id("selection.country"),
                            select_with_options(
                                &COUNTRIES,
                                None,
                                "Sin región",
                                Msg::SetCountry,
                                true,
                            )
                            .label("Región")
                            .test_id("selection.region"),
                        ]),
                    div().class("flex flex-wrap items-center gap-3").children([
                        label("Alineación"),
                        segmented_with_options(
                            &ALIGNMENTS,
                            self.alignment,
                            Msg::SetAlignment,
                            false,
                        )
                        .label("Alineación")
                        .test_id("selection.alineacion"),
                        segmented_with_options(&["Día", "Semana"], 0, |_| Msg::Increment, true)
                            .label("Vista")
                            .test_id("selection.view"),
                    ]),
                ]),
            ])
    }

    /// Contenedores: a full card, group boxes, tabs, statistics and text
    /// styles.
    fn containers(&self) -> Vec<Element<Msg>> {
        vec![
            card()
                .class("w-full max-w-3xl")
                .test_id("containers.card")
                .children([
                    card_header("Card completa", Some("Encabezado, contenido y acciones")),
                    card_content().children([
                        heading("Título de sección"),
                        paragraph(
                            "Un párrafo de cuerpo: el texto largo se ajusta al ancho de la \
                             card y mantiene el interlineado del tema.",
                        ),
                        caption("Una leyenda, para notas y metadatos."),
                    ]),
                    card_actions().children([
                        button("Descartar", ButtonVariant::Text).on_click(Msg::Reset),
                        button("Aceptar", ButtonVariant::Primary).on_click(Msg::Increment),
                    ]),
                ]),
            div()
                .class("grid grid-cols-1 sm:grid-cols-2 gap-4 w-full max-w-3xl")
                .children([
                    group_box(
                        Some("Preferencias"),
                        [switch_labeled(
                            "Sincronizar",
                            self.wifi,
                            Msg::ToggleWifi,
                            false,
                            SwitchDensity::Default,
                        )],
                    )
                    .test_id("containers.grupo"),
                    group_box(
                        None,
                        [caption("Un group box sin título agrupa sin encabezado.")],
                    )
                    .test_id("containers.sin_titulo"),
                ]),
            Tabs::new(self.tab_index, Msg::SetTab)
                .tab(
                    "General",
                    paragraph("Las pestañas muestran un panel a la vez; ← y → cambian de pestaña."),
                )
                .tab(
                    "Cuenta",
                    switch_labeled(
                        "Sincronizar",
                        self.wifi,
                        Msg::ToggleWifi,
                        false,
                        SwitchDensity::Default,
                    ),
                )
                .tab(
                    "Avanzado",
                    caption("Tab entra al panel y Mayúsculas+Tab vuelve a la pestaña."),
                )
                .build()
                .class("w-full max-w-3xl")
                .test_id("containers.pestanas"),
            div()
                .class("grid grid-cols-3 gap-4 w-full max-w-3xl")
                .test_id("containers.stats")
                .children([
                    stat("Clientes", CLIENT_COUNT.to_string(), Some("en la tabla")),
                    stat("Filtrados", self.table_view.len().to_string(), None),
                    stat("Avisos", self.toasts.len().to_string(), Some("abiertos")),
                ]),
        ]
    }

    /// Tabla: ten thousand generated clients, sorted by the header and
    /// filtered by the toolbar search.
    fn client_table(&self) -> Element<Msg> {
        let summary = match self.client.and_then(|index| self.clients.get(index)) {
            Some(client) => format!("Seleccionado: {}", client.name),
            None => "Elige una fila con un clic o con ↑ ↓".to_owned(),
        };
        let shown = if !self.table_view.is_filtered() {
            format!("{} clientes", self.table_view.len())
        } else if self.query.trim().is_empty() {
            format!("{} de {CLIENT_COUNT} filtrados", self.table_view.len())
        } else {
            format!(
                "{} de {CLIENT_COUNT} con «{}»",
                self.table_view.len(),
                self.query.trim()
            )
        };
        let filtering_now = self.table_view.filtering;
        let filter_rows = icon_button_sized(
            if filtering_now {
                Icon::FunnelX
            } else {
                Icon::Funnel
            },
            "Filtrar",
            if self.table_view.has_filters() {
                IconButtonVariant::Primary
            } else {
                IconButtonVariant::Outlined
            },
            IconButtonSize::Sm,
        )
        .tooltip(if filtering_now {
            "Quitar los filtros"
        } else {
            "Filtrar por columna"
        })
        .pressed(filtering_now)
        .on_click(Msg::Table(ViewChange::Filtering(!filtering_now)))
        .test_id("table.filter");
        let columns_button = column_menu(
            icon_button_sized(
                Icon::Columns3,
                "Columnas",
                IconButtonVariant::Outlined,
                IconButtonSize::Sm,
            )
            .tooltip("Mostrar u ocultar columnas")
            .test_id("table.columns"),
            &table_columns(),
            &self.table_view,
            Msg::Table,
        );
        card().class("w-full max-w-3xl").test_id("table").children([
            card_header(
                "Clientes",
                Some("10 000 filas; solo se construyen las visibles"),
            ),
            card_content().class("gap-3").children([
                div()
                    .class("flex items-center justify-between gap-3")
                    .children([
                        caption(shown).role(Role::Status).test_id("table.total"),
                        div().class("flex items-center gap-3").children([
                            caption(summary).test_id("table.selection"),
                            filter_rows,
                            columns_button,
                            button_sized("Exportar", ButtonVariant::Outlined, ButtonSize::Sm)
                                .on_command(ExportTable)
                                .busy(self.exporting),
                        ]),
                    ]),
                Table::new(Rc::clone(&self.clients), table_columns(), &self.table_view)
                    .on_view(Msg::Table)
                    .selected(self.client)
                    .on_select(Msg::SelectRow)
                    .expandable(client_detail, 64.0)
                    .label("Clientes")
                    .test_id("table.clients")
                    .build()
                    .class("h-[360px] rounded-md border border-border appkit:border-separator"),
            ]),
        ])
    }

    /// Multimedia: avatars by size, with the `User` icon, an image or one
    /// from a URL, and a bare `image`.
    fn avatars(&self) -> Element<Msg> {
        let sizes = [
            AvatarSize::Xs,
            AvatarSize::Sm,
            AvatarSize::Md,
            AvatarSize::Lg,
        ];
        let row = |label: &str, avatars: Vec<Element<Msg>>| {
            div()
                .class("flex flex-wrap items-center gap-3")
                .child(caption(label).class("w-24"))
                .children(avatars)
        };
        let url = self.url.clone();
        card()
            .class("w-full max-w-3xl")
            .test_id("media.avatares")
            .children([
                card_header(
                    "Avatares",
                    Some(
                        "El ícono User de Lucide mientras no hay imagen; la imagen se recorta \
                         al círculo",
                    ),
                ),
                card_content().class("gap-4").children([
                    row(
                        "Sin imagen:",
                        ["Ana Rojas", "Bruno Soto", "Carla Vega", "Diego Silva"]
                            .into_iter()
                            .zip(sizes)
                            .map(|(name, size)| Avatar::new(name).size(size).build())
                            .collect(),
                    ),
                    row(
                        "Con imagen:",
                        sizes
                            .into_iter()
                            .map(|size| {
                                Avatar::new("Felipe Muñoz")
                                    .image(sample_portrait())
                                    .size(size)
                                    .build()
                            })
                            .collect(),
                    ),
                    div().class("flex items-center gap-4").children([
                        avatar(
                            "Imagen remota",
                            self.avatar_url.clone().map(ImageSource::Url),
                        )
                        .test_id("media.avatar_url"),
                        div().class("flex flex-1 flex-col gap-2 min-w-0").children([
                            caption("Desde una URL (media.load_url):"),
                            div().class("flex items-center gap-3").children([
                                TextField::new("URL de la imagen", &self.url)
                                    .placeholder("https://…/foto.png")
                                    .on_input(|value| Msg::Edit(Field::Url, value))
                                    .on_submit(Msg::LoadUrl(self.url.clone()))
                                    .test_id("media.url")
                                    .build()
                                    .class("flex-1"),
                                button("Cargar", ButtonVariant::Outlined)
                                    .on_command(LoadUrl { url })
                                    .test_id("media.load"),
                            ]),
                        ]),
                    ]),
                    row(
                        "image(src):",
                        vec![
                            image(sample_landscape())
                                .alt("Paisaje")
                                .class("w-48 rounded-md")
                                .test_id("media.paisaje"),
                            image(sample_landscape())
                                .alt("Paisaje recortado")
                                .class("size-24 rounded-md object-cover"),
                        ],
                    ),
                ]),
            ])
    }

    /// Multimedia: the profile photo, logo and banner fields: a click on
    /// the image opens the file panel, a file
    /// dropped from Finder replaces the image, and a refused one shows its
    /// error.
    fn image_fields(&self) -> Element<Msg> {
        let field =
            |image_field: ImageField, variant: MediaVariant, image_value: &ImageSlot, id: &str| {
                MediaSlot::new(variant)
                    .current(image_value.actual.clone())
                    .on_pick(Msg::PickImage(image_field))
                    .on_drop(move |path| Msg::ImagePicked(image_field, Some(path)))
                    .error(image_value.error.clone())
                    .test_id(id)
            };
        let labeled = |label: &str, slot: Element<Msg>| {
            div()
                .class("flex flex-col items-center gap-2 min-w-0")
                .children([caption(label), slot])
        };
        card()
            .class("w-full max-w-3xl")
            .test_id("media.fields")
            .children([
                card_header(
                    "Campos de imagen",
                    Some(
                        format!(
                            "Clic para elegir o arrastra desde Finder; png, jpg o webp de \
                             hasta {MAX_IMAGE_MB} MB"
                        )
                        .as_str(),
                    ),
                ),
                card_content().class("gap-6").children([
                    div().class("grid grid-cols-2 gap-6").children([
                        labeled(
                            "Foto de perfil",
                            field(
                                ImageField::Photo,
                                MediaVariant::Avatar(AvatarSize::Md),
                                &self.photo,
                                "media.photo",
                            )
                            .build(),
                        ),
                        labeled(
                            "Logo",
                            field(
                                ImageField::Logo,
                                MediaVariant::Logo,
                                &self.logo,
                                "media.logo",
                            )
                            .build(),
                        ),
                    ]),
                    labeled(
                        "Portada",
                        field(
                            ImageField::Cover,
                            MediaVariant::Banner,
                            &self.cover,
                            "media.cover",
                        )
                        .build(),
                    ),
                ]),
            ])
    }

    /// Tabla: a plain virtual list.
    fn simple_list(&self) -> Element<Msg> {
        card().class("w-full max-w-3xl").children([
            card_header("Lista virtualizada", Some("1 000 elementos de 28 pt")),
            card_content().child(
                virtual_list(
                    1_000,
                    28.0,
                    Rc::new(|index| {
                        div()
                            .class(
                                "flex items-center px-3 text-sm text-foreground \
                                 appkit:text-label",
                            )
                            .child(text(format!("Elemento {}", index + 1)))
                    }),
                )
                .label("Elementos")
                .test_id("table.items")
                .class("h-48 rounded-md border border-border appkit:border-separator"),
            ),
        ])
    }

    /// Mapa: Parral with a pin on the plaza, the DSP sectors over it, a
    /// list that frames them, and where the map is.
    fn map_view(&self) -> Element<Msg> {
        let view = self.view;
        let selected_one = self.area.as_deref();
        let editing_code = self.editing.as_ref().map(|(code, _)| code.as_str());
        let layers_list = self
            .sectors
            .iter()
            .filter(|sector| self.show_areas && editing_code != Some(sector.code.as_str()))
            .map(|sector| {
                let color = sector.color();
                sector
                    .layer
                    .clone()
                    .id(&sector.code)
                    .fill(color, 0.12)
                    .stroke(color, 2.0)
                    .label(&sector.name)
                    .selected(selected_one == Some(sector.code.as_str()))
            });
        let mut map_view = map(&view)
            .basemap(TileSource::pmtiles(PARRAL_PMTILES, MapStyle::protomaps()))
            .layers(layers_list)
            .animate_view(self.animate_map)
            .on_view_change(Msg::MapMoved);
        map_view = match &self.editing {
            Some((code, draft)) => {
                let color = self
                    .sector(code)
                    .map_or(Color::hex(0xc45c26), Sector::color);
                map_view.polygon_editor(draft, color, Msg::Editing)
            }
            None => map_view.on_area_click(Msg::SelectArea),
        };
        let mut map_view = map_view
            .label("Mapa de Parral")
            .test_id("map.view")
            .class("flex-1 min-w-0 h-96 border border-border")
            .children([marker(
                MAIN_SQUARE,
                Anchor::Bottom,
                icon(Icon::MapPin).class("size-8 text-primary"),
            )
            .alt("Plaza de Armas")]);
        if let Some((bounds, serial)) = self.fit_request {
            map_view = map_view.fit_bounds(bounds, serial);
        }
        let list_view = div()
            .role(Role::List)
            .label("Sectores")
            .test_id("map.sectors")
            .class("flex flex-col gap-1 w-44")
            .children(self.sectors.iter().map(|sector| {
                let chosen_one = selected_one == Some(sector.code.as_str());
                div()
                    .role(Role::Button)
                    .pressed(chosen_one)
                    .on_click(Msg::SelectArea(sector.code.clone()))
                    .test_id(format!("map.sector.{}", sector.code))
                    .class(&format!(
                        "flex items-center gap-2 h-8 px-2 rounded-md border text-sm \
                         text-foreground cursor-pointer focus-visible:ring-2 \
                         focus-visible:ring-primary/40 {}",
                        if chosen_one {
                            format!("{CHOICE_ACTIVE} font-medium")
                        } else {
                            CHOICE_HOVER.to_owned()
                        }
                    ))
                    .children([
                        div().class(&format!("size-3 shrink-0 rounded-sm {}", sector.swatch())),
                        text(&sector.name),
                    ])
            }));
        let state_value = match (
            &self.editing,
            self.area.as_deref().and_then(|c| self.sector(c)),
        ) {
            (Some(_), Some(sector)) => format!(
                "Editando {}: arrastra los vértices, haz clic entre dos para agregar uno; \
                 Supr borra el vértice elegido y ⌘Z deshace",
                sector.name
            ),
            (None, Some(sector)) => format!(
                "Sector {} · {} · {}",
                sector.name,
                sector.code,
                hectares(crate::maps::editor::area_m2(&sector.ring_points()) / 10_000.0)
            ),
            _ => "Elige un sector en la lista o en el mapa".to_owned(),
        };
        let editing = self.editing.as_ref().map(|(_, draft)| {
            let crosses = draft.crossing().is_some();
            div()
                .class("flex items-center gap-3")
                .test_id("map.edicion")
                .children([
                    caption(format!("Superficie {}", hectares(draft.area_ha())))
                        .role(Role::Status)
                        .test_id("map.superficie"),
                    if crosses {
                        caption("Los bordes se cruzan")
                            .class("text-error")
                            .role(Role::Alert)
                            .test_id("map.cruce")
                    } else {
                        div()
                    },
                    div().class("flex-1"),
                    button("Cancelar", ButtonVariant::Outlined)
                        .on_click(Msg::Editing(PolygonEdit::Cancel))
                        .test_id("map.cancel"),
                    button("Terminar", ButtonVariant::Primary)
                        .on_click(Msg::Editing(PolygonEdit::Finish))
                        .disabled(!draft.is_valid())
                        .test_id("map.finish"),
                ])
        });
        let color = self
            .area
            .as_deref()
            .and_then(|code| self.sector(code))
            .filter(|_| self.editing.is_none())
            .map(|sector| {
                div().class("flex w-80").child(
                    color_field("Color del sector", &sector.color, Msg::ColorSector)
                        .palette(DSP_PALETTE)
                        .allow_empty(true)
                        .test_id("map.color")
                        .build(),
                )
            });
        let edit_sector = button("Editar sector", ButtonVariant::Outlined)
            .on_click(Msg::EditArea(self.area.clone().unwrap_or_default()))
            .disabled(self.area.is_none() || self.editing.is_some())
            .test_id("map.edit");
        card().class("w-full max-w-3xl").test_id("map").children([
            card_header(
                "Parral",
                Some("Arrastra, usa la rueda o pellizca; doble clic acerca y ⇧ doble clic aleja"),
            ),
            card_content().class("gap-3").children(
                [div().class("flex gap-3").children([
                    map_view,
                    div().class("flex flex-col gap-3 w-44").children([
                        list_view,
                        edit_sector,
                        switch_labeled(
                            "Sectores",
                            self.show_areas,
                            Msg::ToggleAreas,
                            false,
                            SwitchDensity::Compact,
                        )
                        .test_id("map.mostrar_sectores"),
                    ]),
                ])]
                .into_iter()
                .chain(editing)
                .chain(color)
                .chain([
                    caption(state_value).test_id("map.sector"),
                    div()
                        .class("flex items-center justify-between gap-3")
                        .children([
                            caption(format!(
                                "{:.4}, {:.4} · zoom {:.1}",
                                view.center.lat, view.center.lon, view.zoom
                            ))
                            .role(Role::Status)
                            .test_id("map.posicion"),
                            button("Volver a la plaza", ButtonVariant::Outlined)
                                .on_click(Msg::SetMapView {
                                    view: INITIAL_VIEW,
                                    animate: true,
                                })
                                .test_id("map.volver"),
                        ]),
                ]),
            ),
        ])
    }

    /// Gráficos: a bar chart of monthly clicks and a sparkline of the trend.
    fn charts_view(&self) -> Vec<Element<Msg>> {
        let chosen_one = self.month.map(|index| {
            format!(
                "Seleccionado: {} ({})",
                MONTHS[index], CLICKS_PER_MONTH[index] as i32
            )
        });
        vec![
            card()
                .class("w-full max-w-3xl")
                .test_id("charts.bars")
                .children([
                    card_header("Barras", Some("Clics por mes; elige una barra")),
                    card_content().children([
                        chart(ChartKind::Bar)
                            .categories(MONTHS)
                            .series("Clics", &CLICKS_PER_MONTH)
                            .selected(self.month)
                            .on_select(Msg::Month)
                            .label("Clics por mes")
                            .test_id("charts.bars.plot")
                            .class("h-48 w-full"),
                        caption(chosen_one.as_deref().unwrap_or("Elige una barra")),
                    ]),
                ]),
            card()
                .class("w-full max-w-3xl")
                .test_id("charts.sparkline")
                .children([
                    card_header("Sparkline", Some("Tendencia compacta, sin ejes")),
                    card_content().class("flex items-center gap-4").children([
                        stat("Tendencia", "16", Some("último mes")),
                        chart(ChartKind::Sparkline)
                            .series("Tendencia", &TREND)
                            .label("Tendencia")
                            .test_id("charts.sparkline.plot")
                            .class("h-8 flex-1"),
                    ]),
                ]),
        ]
    }

    #[cfg(feature = "document")]
    fn document_ribbon_chrome(&self) -> Element<Msg> {
        document_ribbon()
            .mode(RibbonMode::Full)
            .active_tab(self.doc_ribbon_tab.clone())
            .pressed_ids(self.doc_ribbon_pressed.clone())
            .disabled_ids(self.doc_ribbon_disabled.clone())
            .contexts(self.doc_ribbon_contexts.clone())
            .on_tab(Msg::DocRibbonTab)
            // Un comentario necesita texto: el botón abre la barra que lo pide.
            .on_command(|id| match id.as_str() {
                "review.newComment" => Msg::DocToggleComment,
                _ => Msg::DocCmd(document_cmd_for(&id)),
            })
            .leading([
                toolbar_item(Icon::FolderOpen, "Abrir…")
                    .on_command(DocOpen)
                    .test_id("document.open"),
                toolbar_item(Icon::Save, "Guardar…")
                    .on_command(DocSave)
                    .test_id("document.save"),
                toolbar_item(Icon::FileDown, "Exportar PDF…")
                    .on_command(DocExportPdf)
                    .test_id("document.export_pdf"),
                toolbar_item(Icon::Search, "Buscar y reemplazar")
                    .on_command(DocFind)
                    .test_id("document.find"),
            ])
            .test_id("document.ribbon")
            .build()
    }

    #[cfg(not(feature = "document"))]
    fn document_ribbon_chrome(&self) -> Element<Msg> {
        div()
            .class("flex flex-col gap-1")
            .test_id("document.ribbon")
            .children([
                div()
                    .class("flex flex-row items-center gap-0.5")
                    .role(Role::Toolbar)
                    .test_id("document.file")
                    .children([
                        toolbar_item(Icon::FolderOpen, "Abrir…")
                            .on_command(DocOpen)
                            .test_id("document.open"),
                        toolbar_item(Icon::Save, "Guardar…")
                            .on_command(DocSave)
                            .test_id("document.save"),
                        toolbar_item(Icon::FileDown, "Exportar PDF…")
                            .on_command(DocExportPdf)
                            .test_id("document.export_pdf"),
                    ]),
                text("Compila con --features document para el ribbon WordCraft.")
                    .class("text-sm text-muted-foreground"),
            ])
    }

    /// Documento: metáfora Pages — ribbon (QAT Archivo + tabs) + canvas Letter.
    fn document_view(&self) -> Element<Msg> {
        let mut editor = document()
            .label("Informe de demostración")
            .placeholder_text("Escribe aquí…")
            .markdown(&self.doc_markdown)
            .on_change(Msg::Document)
            .class("flex-1 min-h-0 w-full")
            .test_id("document.editor");
        if let Some((cmd, serial)) = &self.doc_cmd {
            editor = editor.pending_cmd(cmd.clone(), *serial);
        }
        if let Some((serial, bytes)) = &self.doc_load {
            editor = editor.pending_load(bytes.clone(), *serial);
        }
        let mut chrome = vec![self.document_ribbon_chrome()];
        if self.doc_find_open {
            chrome.push(self.document_find_bar());
        }
        if self.doc_comment_open {
            chrome.push(self.document_comment_bar());
        }
        let status = match &self.doc_file_name {
            Some(name) => name.clone(),
            None => "Sin guardar".into(),
        };
        let status = if self.doc_edit_status.is_empty() {
            status
        } else {
            format!("{status} · {}", self.doc_edit_status)
        };
        div()
            .class("flex flex-col flex-1 min-h-0 w-full")
            .test_id("document")
            .children([
                div()
                    .class("flex-none flex flex-col px-3 pt-2 pb-1 bg-window border-b border-separator gap-1")
                    .children(chrome),
                editor,
                // Barra de estado (como la de Word): archivo y resultado de buscar/revisar.
                div()
                    .class("flex-none flex flex-row items-center px-3 py-1 bg-window border-t border-separator")
                    .children([text(status)
                        .class("text-xs text-muted-foreground")
                        .test_id("document.status")]),
            ])
    }

    /// Buscar y reemplazar en una sola fila, oculta hasta ⌘F (como Pages).
    fn document_find_bar(&self) -> Element<Msg> {
        let find = self.doc_find.clone();
        let replace = self.doc_replace.clone();
        div()
            .class("flex flex-row items-center gap-1 py-1")
            .test_id("document.findbar")
            .children([
                search_field(&self.doc_find, Msg::DocFindChanged)
                    .label("Buscar en el documento")
                    .on_submit(Msg::DocCmd(DocumentCmd::Find { text: find.clone() }))
                    .test_id("document.find.query")
                    .build(),
                search_field(&self.doc_replace, Msg::DocReplaceChanged)
                    .placeholder("Reemplazar con")
                    .label("Reemplazar con")
                    .test_id("document.find.replace")
                    .build(),
                toolbar_item(Icon::ChevronUp, "Anterior")
                    .on_click(Msg::DocCmd(DocumentCmd::FindPrevious))
                    .test_id("document.find.prev"),
                toolbar_item(Icon::ChevronDown, "Siguiente")
                    .on_click(Msg::DocCmd(DocumentCmd::FindNext))
                    .test_id("document.find.next"),
                button("Reemplazar", ButtonVariant::Text)
                    .on_click(Msg::DocCmd(DocumentCmd::Replace {
                        text: find.clone(),
                        with: replace.clone(),
                    }))
                    .test_id("document.find.replace_one"),
                button("Todo", ButtonVariant::Text)
                    .on_click(Msg::DocCmd(DocumentCmd::ReplaceAll {
                        text: find,
                        with: replace,
                    }))
                    .test_id("document.find.replace_all"),
                toolbar_item(Icon::X, "Cerrar búsqueda")
                    .on_click(Msg::DocToggleFind)
                    .test_id("document.find.close"),
            ])
    }

    /// Texto del nuevo comentario, en una fila bajo el ribbon (como el
    /// globo de Word): Enter o «Comentar» lo inserta en la selección.
    fn document_comment_bar(&self) -> Element<Msg> {
        let insert = Msg::DocCmd(DocumentCmd::NewComment {
            text: self.doc_comment_draft.clone(),
        });
        div()
            .class("flex flex-row items-center gap-1 py-1")
            .test_id("document.commentbar")
            .children([
                search_field(&self.doc_comment_draft, Msg::DocCommentDraftChanged)
                    .placeholder("Escribe un comentario")
                    .label("Comentario")
                    .on_submit(insert.clone())
                    .test_id("document.comment.draft")
                    .build(),
                button("Comentar", ButtonVariant::Text)
                    .on_click(insert)
                    .test_id("document.comment.new"),
                toolbar_item(Icon::X, "Cerrar comentario")
                    .on_click(Msg::DocToggleComment)
                    .test_id("document.comment.close"),
            ])
    }

    fn dialogs_and_toasts(&self) -> Element<Msg> {
        card()
            .class("w-full max-w-3xl")
            .test_id("dialogs")
            .children([
                card_header(
                    "Diálogos y avisos",
                    Some("Modales con foco atrapado, avisos que se cierran solos y la paleta"),
                ),
                card_content().class("gap-4").children([
                    div().class("flex flex-wrap items-center gap-3").children([
                        caption("Diálogos:"),
                        button("Guardar cambios…", ButtonVariant::Outlined)
                            .on_click(Msg::OpenDialog(DemoDialog::Save))
                            .test_id("dialogs.save"),
                        button("Eliminar borrador…", ButtonVariant::Danger)
                            .on_click(Msg::OpenDialog(DemoDialog::Delete))
                            .test_id("dialogs.delete"),
                    ]),
                    div()
                        .class("flex flex-wrap items-center gap-3")
                        .child(caption("Avisos:"))
                        .children(ToastKind::ALL.map(|kind| {
                            button(kind.texts().0, ButtonVariant::Secondary)
                                .on_click(Msg::ShowToast(kind))
                                .test_id(format!("toasts.{}", kind.key()))
                        })),
                    div().class("flex flex-wrap items-center gap-3").children([
                        caption("Comandos:"),
                        Button::new("Abrir la paleta (⇧⌘P)")
                            .bezel(BezelStyle::Borderless)
                            .build()
                            .run_command(PALETTE)
                            .test_id("dialogs.palette"),
                    ]),
                ]),
            ])
    }

    /// The open dialog.
    fn dialog(&self, dialog_kind: DemoDialog) -> Element<Msg> {
        let dialog = match dialog_kind {
            DemoDialog::Save => Dialog::new("¿Guardar los cambios?")
                .message("Los cambios quedarán disponibles para todo el equipo.")
                .default_button("Guardar", Msg::ConfirmDialog),
            DemoDialog::Delete => Dialog::new("¿Eliminar el borrador?")
                .message("Esta acción no se puede deshacer.")
                .destructive(true)
                .width(DialogWidth::Sm)
                .default_button("Eliminar", Msg::ConfirmDialog),
        };
        dialog
            .cancel_button("Cancelar", Msg::CloseDialog)
            .test_id("dialog")
            .build()
    }

    /// Card with every control size (`NSControl.ControlSize`, mini … extra
    /// large) for the main button variants, and the three icon-button
    /// sizes (Kai 28 / 40 / 48 pt, also in mix C).
    /// Top panel: window background effect and its motion, applied live.
    fn background_panel(&self) -> Element<Msg> {
        let options = [
            (BackgroundKind::None, "Ninguno"),
            (BackgroundKind::Mesh, "Mesh"),
            (BackgroundKind::Aurora, "Aurora"),
            (BackgroundKind::Glow, "Glow"),
        ];
        let animated = self.background_motion == BackgroundMotion::Animated;
        card().class("w-full max-w-3xl").children([
            card_header(
                "Fondo de ventana",
                Some("Efecto bajo el contenido; estático o animado"),
            ),
            card_content().child(
                div()
                    .class("flex flex-wrap items-center justify-between gap-3")
                    .children([
                        segmented(
                            &options.map(|(_, label)| label),
                            options
                                .iter()
                                .position(|(kind, _)| *kind == self.background)
                                .unwrap_or(0),
                            |index| Msg::SetBackground(options[index].0),
                        )
                        .label("Efecto")
                        .test_id("background.efecto"),
                        switch_labeled(
                            "Animado",
                            animated,
                            Msg::ToggleBackgroundMotion,
                            self.background == BackgroundKind::None,
                            SwitchDensity::Default,
                        ),
                    ]),
            ),
        ])
    }

    fn control_sizes(&self) -> Element<Msg> {
        let sizes = [
            (ControlSize::Mini, "Mini"),
            (ControlSize::Small, "Small"),
            (ControlSize::Regular, "Regular"),
            (ControlSize::Large, "Large"),
            (ControlSize::ExtraLarge, "Extra large"),
        ];
        let row = |name: &str, variant: ButtonVariant| {
            div()
                .class("flex flex-wrap items-center gap-3")
                .child(caption(name).class("w-20 shrink-0"))
                .children(sizes.map(|(size, label)| {
                    button(label, variant)
                        .control_size(size)
                        .on_click(Msg::Increment)
                }))
        };
        let icons = |name: &str, variant: IconButtonVariant| {
            div()
                .class("flex flex-wrap items-center gap-3")
                .child(caption(name).class("w-20 shrink-0"))
                .children(
                    [
                        (IconButtonSize::Sm, Icon::Search),
                        (IconButtonSize::Md, Icon::User),
                        (IconButtonSize::Lg, Icon::Trash2),
                    ]
                    .map(|(size, kind)| icon_button_sized(kind, name, variant, size)),
                )
        };
        card().class("w-full max-w-3xl").children([
            card_header(
                "Tamaños de control",
                Some("Mini, small, regular, large y extra large; IconButtons Sm / Md / Lg"),
            ),
            card_content().class("gap-3").children([
                row("Primary", ButtonVariant::Primary),
                row("Secondary", ButtonVariant::Secondary),
                row("Outlined", ButtonVariant::Outlined),
                row("Text", ButtonVariant::Text),
                div().class("flex flex-col gap-3 pt-2").children([
                    icons("Primary", IconButtonVariant::Primary),
                    icons("Secondary", IconButtonVariant::Secondary),
                    icons("Outlined", IconButtonVariant::Outlined),
                    icons("Action", IconButtonVariant::Action),
                ]),
            ]),
        ])
    }

    /// Matriz de estados TextField (demo kit): placeholders neutros AppKit.
    fn field_states(&self) -> Element<Msg> {
        let cell = |label: &str, field: Element<Msg>| {
            div()
                .class("flex flex-col gap-1.5 min-w-0")
                .children([caption(label).class("text-secondary-label"), field])
        };
        card()
            .class("w-full max-w-3xl")
            .test_id("fields.estados")
            .children([
                card_header(
                    "Estados de TextField",
                    Some(
                        "Default, foco (focus-within), disabled, required, helper y error —                          placeholders neutros",
                    ),
                ),
                card_content().class("gap-4").children([
                    div()
                        .class("grid grid-cols-1 sm:grid-cols-2 gap-x-4 gap-y-5")
                        .children([
                            cell(
                                "Default",
                                TextField::new("Nombre", "")
                                    .placeholder("Valor de ejemplo")
                                    .field_icon(FieldKind::Name)
                                    .test_id("estados.default")
                                    .build(),
                            ),
                            cell(
                                "Con valor (foco vía Tab)",
                                TextField::new("Correo", "ana@example.com")
                                    .field_icon(FieldKind::Email)
                                    .test_id("estados.focus")
                                    .build(),
                            ),
                            cell(
                                "Disabled",
                                TextField::new("Teléfono", "+56 9 1234 5678")
                                    .field_icon(FieldKind::Phone)
                                    .disabled(true)
                                    .test_id("estados.disabled")
                                    .build(),
                            ),
                            cell(
                                "Required",
                                TextField::new("Usuario", "")
                                    .placeholder("Requerido")
                                    .field_icon(FieldKind::User)
                                    .required(true)
                                    .test_id("estados.required")
                                    .build(),
                            ),
                            cell(
                                "Helper text",
                                TextField::new("Búsqueda", "")
                                    .placeholder("Escribe para filtrar")
                                    .field_icon(FieldKind::Search)
                                    .helper_text("Puedes usar palabras clave")
                                    .test_id("estados.helper")
                                    .build(),
                            ),
                            cell(
                                "Error",
                                TextField::new("Correo", "no-es-un-correo")
                                    .field_icon(FieldKind::Email)
                                    .helper_text("Introduce un correo válido")
                                    .error(true)
                                    .test_id("estados.error")
                                    .build(),
                            ),
                        ]),
                ]),
            ])
    }

    /// EmptyState + loading placeholder (patrones de datos).
    fn patterns_empty_loading(&self) -> Element<Msg> {
        card()
            .class("w-full max-w-3xl")
            .test_id("patrones.empty_loading")
            .children([
                card_header(
                    "Empty y Loading",
                    Some("Patrones reutilizables para listas y tablas vacías o en carga"),
                ),
                card_content().class("gap-4").children([
                    caption("Empty filter — la consulta no devolvió filas"),
                    div()
                        .class(
                            "rounded-md border border-border appkit:border-separator overflow-hidden",
                        )
                        .test_id("patrones.empty.filter")
                        .child(
                            EmptyState::filter("No hay resultados")
                                .description("Prueba otro filtro o límpialo.")
                                .action("Limpiar filtros", Msg::Clear)
                                .test_id("patrones.empty.filter.state")
                                .build(),
                        ),
                    caption("Empty resource — la colección nunca tuvo ítems"),
                    div()
                        .class(
                            "rounded-md border border-border appkit:border-separator overflow-hidden",
                        )
                        .test_id("patrones.empty.resource")
                        .child(
                            EmptyState::resource("Sin elementos")
                                .description("Todavía no hay nada aquí.")
                                .action("Crear elemento", Msg::Increment)
                                .test_id("patrones.empty.state")
                                .build(),
                        ),
                    div()
                        .class(
                            "rounded-md border border-border appkit:border-separator overflow-hidden",
                        )
                        .test_id("patrones.loading")
                        .child(loading_placeholder("Cargando filas…")),
                ]),
            ])
    }

    /// Card showing the TextField props implemented so far.
    fn fields(&self) -> Element<Msg> {
        let edit = |field: Field| move |value: String| Msg::Edit(field, value);
        let contrast_bar = div()
            .class("flex items-center gap-4 px-4 py-3 rounded-md bg-primary")
            .children([
                text("Kai").class("text-base font-semibold text-white whitespace-nowrap"),
                TextField::new("Buscar en Kai", &self.search)
                    .variant(FieldVariant::Contrast)
                    .always_show_label(true)
                    .start_icon(Icon::Search)
                    .on_input(edit(Field::Search))
                    .build(),
            ]);

        let autocomplete_shell = div()
            .class(
                "relative flex items-center w-full rounded-md border border-border px-3 \
                 focus-within:border-primary bg-background",
            )
            .children([
                icon(Icon::Search).class("size-4 text-muted-foreground mr-1 shrink-0"),
                TextField::new("Buscar cliente...", &self.autocomplete)
                    .variant(FieldVariant::Autocomplete)
                    .density(FieldDensity::Compact)
                    .on_input(edit(Field::Autocomplete))
                    .build(),
            ]);

        card().class("w-full max-w-3xl").children([
            card_header(
                "Variantes de TextField",
                Some(
                    "Iconos Lucide, adornments, password, requerido, compacto, solo lectura, \
                     deshabilitado y autocomplete",
                ),
            ),
            card_content().class("gap-4").children([
                page_grid().children([
                    page_grid_item(6).child(
                        TextField::new("Buscar", &self.search)
                            .placeholder("Texto de búsqueda")
                            .field_icon(FieldKind::Search)
                            .on_input(edit(Field::Search))
                            .build(),
                    ),
                    page_grid_item(6).child(
                        TextField::new("Contraseña", &self.password)
                            .test_id("form.password")
                            .field_icon(FieldKind::Password)
                            .password_command(self.show_password, TogglePasswordAction)
                            .required(true)
                            .helper_text("Mínimo 8 caracteres")
                            .on_input(edit(Field::Password))
                            .build(),
                    ),
                    page_grid_item(6).child(
                        TextField::new("Monto", &self.amount)
                            .start_symbol("$")
                            .select_on_focus(true)
                            .on_input(edit(Field::Amount))
                            .build(),
                    ),
                    page_grid_item(6).child(
                        TextField::new("Descuento", &self.discount)
                            .end_symbol("%")
                            .on_input(edit(Field::Discount))
                            .build(),
                    ),
                    page_grid_item(6).child(
                        TextField::new("Comuna", &self.commune)
                            .required(true)
                            .helper_text("Donde despacharemos el pedido")
                            .on_input(edit(Field::Commune))
                            .build(),
                    ),
                    page_grid_item(6).child(
                        TextField::new("Código", &self.code)
                            .density(FieldDensity::Compact)
                            .placeholder("Ej: A-12")
                            .on_input(edit(Field::Code))
                            .build(),
                    ),
                    page_grid_item(6).child(
                        TextField::new("Cantidad", &self.quantity)
                            .density(FieldDensity::Compact)
                            .label_layout(LabelLayout::Inline)
                            .inline_leading(
                                icon(Icon::Info)
                                    .class("size-4 text-secondary appkit:text-secondary-label"),
                            )
                            .placeholder("0")
                            .on_input(edit(Field::Quantity))
                            .build(),
                    ),
                    page_grid_item(6).child(
                        TextField::new("Folio", "F-2026-0042")
                            .read_only(true)
                            .end_icon(Icon::CircleCheckBig)
                            .build(),
                    ),
                    page_grid_item(6).child(
                        TextField::new("Sucursal", "Santiago Centro")
                            .disabled(true)
                            .build(),
                    ),
                    page_grid_item(6).child(autocomplete_shell),
                ]),
                contrast_bar,
            ]),
        ])
    }
}

/// Opens the demo window. Reads `KAI_THEME`, `KAI_HOVER`, `KAI_MIX`,
/// `KAI_LABEL`, `KAI_BG` and `KAI_BG_ANIM`. With `--describe-commands` it
/// prints the commands and their schemas (`mcp-schema.json`) instead.
pub fn run_from_env() -> Result<(), Box<dyn std::error::Error>> {
    let flag = |name: &str| std::env::args().skip(1).any(|arg| arg == name);
    if flag("--describe-commands") {
        println!("{}", describe_json::<Demo>());
        return Ok(());
    }
    let mcp = flag("--mcp");
    if mcp && !cfg!(feature = "mcp") {
        return Err("--mcp necesita compilar con --features mcp".into());
    }
    if !cfg!(target_os = "macos") {
        return Err("Kai UI Desktop comienza como prototipo para macOS".into());
    }
    let (background, background_motion, warnings) = background_from_env(
        std::env::var("KAI_BG").ok().as_deref(),
        std::env::var("KAI_BG_ANIM").ok().as_deref(),
    );
    for warning in warnings {
        eprintln!("{warning}");
    }
    let mut demo = Demo {
        background,
        background_motion,
        ..Demo::default()
    };
    // Preferencias de la ejecución anterior; un problema solo se avisa.
    match Store::for_app(APP_ID) {
        Ok(store) => {
            match store.load::<DemoPreferences>() {
                Ok(Some(prefs)) => demo.apply_preferences(&prefs),
                Ok(None) => {}
                Err(e) => eprintln!("Preferencias ignoradas ({}): {e}", store.dir().display()),
            }
            demo.store = Some(store);
        }
        Err(e) => eprintln!("Sin preferencias: {e}"),
    }
    #[cfg(feature = "mcp")]
    if mcp {
        return runtime::run_with_mcp(demo, demo_theme());
    }
    runtime::run(demo, demo_theme())
}

/// Theme of the demo, chosen with `KAI_THEME=kai|appkit|appkit-dark`
/// (default: Glass 27 light, large controls, comfortable density).
/// `KAI_THEME=kai` keeps the Kai theme.
/// `KAI_THEME`: `appkit[-dark]` follows this Mac ([`Theme::appkit_current`]);
/// `aqua11[-dark]` and `glass27[-dark]` force a generation. `KAI_HOVER=0`
/// turns hover off (strict AppKit). `KAI_MIX=appkit|a|b|c` picks the mixed
/// candidate of the AppKit themes (default `c`); Kai ignores it.
/// `KAI_LABEL=near|appkit|top|tab` places AppKit field labels: above,
/// small and close (default), the original AppKit label, on the border
/// like Kai, or in a tab on the border.
fn demo_theme() -> Theme {
    let forced = |generation, appearance| Theme::appkit_static(generation, appearance);
    let theme = match std::env::var("KAI_THEME").as_deref() {
        // Por defecto: Glass claro, controles grandes y densidad cómoda.
        Ok("") | Err(_) => Theme {
            control_size: ControlSize::Large,
            density: Density::Comfortable,
            ..forced(AppKitGeneration::Glass27, Appearance::Light)
        },
        Ok("appkit" | "appkit-light") => Theme::appkit_current(Appearance::Light),
        Ok("appkit-dark") => Theme::appkit_current(Appearance::Dark),
        Ok("aqua11") => forced(AppKitGeneration::Aqua11, Appearance::Light),
        Ok("aqua11-dark") => forced(AppKitGeneration::Aqua11, Appearance::Dark),
        Ok("glass27") => forced(AppKitGeneration::Glass27, Appearance::Light),
        Ok("glass27-dark") => forced(AppKitGeneration::Glass27, Appearance::Dark),
        Ok("kai") => Theme::kai(Density::Comfortable),
        Ok(other) => {
            eprintln!(
                "KAI_THEME={other} desconocido (kai, appkit, appkit-dark, aqua11, aqua11-dark, \
                 glass27, glass27-dark); se usa appkit"
            );
            Theme::appkit_current(Appearance::Light)
        }
    };
    let theme = theme.with_hover(std::env::var("KAI_HOVER").as_deref() != Ok("0"));
    let mix = match std::env::var("KAI_MIX").as_deref() {
        Ok("appkit" | "strict") => MixStyle::AppKit,
        Ok("a" | "A") => MixStyle::A,
        Ok("b" | "B") => MixStyle::B,
        Ok("c" | "C" | "") | Err(_) => MixStyle::C,
        Ok(other) => {
            eprintln!("KAI_MIX={other} desconocido (appkit, a, b, c); se usa c");
            MixStyle::C
        }
    };
    let theme = theme.with_mix(mix);
    // Experiment: where AppKit fields put their label.
    let labels = match std::env::var("KAI_LABEL").as_deref() {
        Ok("appkit") => FieldLabels::AppKit,
        Ok("top") => FieldLabels::Top,
        Ok("tab") => FieldLabels::Tab,
        Ok("near" | "") | Err(_) => FieldLabels::Near,
        Ok(other) => {
            eprintln!("KAI_LABEL={other} desconocido (near, appkit, top, tab); se usa near");
            FieldLabels::Near
        }
    };
    let theme = theme.with_field_labels(labels);
    if theme.is_appkit() {
        let system = SystemInfo::detect();
        let show = |v: Option<MacVersion>| v.map_or("?".to_owned(), |v| v.to_string());
        eprintln!(
            "tema {:?} {:?}, mezcla {:?}, hover {}, etiquetas {:?} (macOS {}, SDK {}: nativo {:?})",
            theme.kind,
            theme.appearance,
            theme.mix,
            if theme.hover_effects { "sí" } else { "no" },
            theme.field_labels,
            show(system.os),
            show(system.sdk),
            system.generation()
        );
    }
    theme
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use super::{
        DemoDialog, INITIAL_VIEW, ImageField, ImageSource, LatLon, LatLonBounds, LoadUrl,
        MAIN_SQUARE, MapView, SECTORS, SimulateWait, ToastKind, ViewChange, initial_sector,
    };

    use cosmic_text::{
        Action, Attrs, Buffer, Edit, Editor, FontSystem, Metrics, Motion, Selection, Shaping,
    };

    use super::{Demo, Field, Msg, Section};
    use serde_json::json;

    use crate::background::{BackgroundEffect, BackgroundKind, BackgroundMotion};
    use crate::commands::{Availability, CommandError, DIALECT, describe_json};
    use crate::components::SortDirection;
    use crate::element::{ControlState, FocusOrder, Kind, Role, WidgetId};
    use crate::geometry::{Rect, Size};
    use crate::layout::testing::FixedMeasure;
    use crate::layout::{Interaction, Layout, LayoutInput, ScrollOffsets, StyleCache};

    impl Demo {
        /// The default demo showing `section`.
        fn at(section: Section) -> Self {
            Self {
                section,
                ..Self::default()
            }
        }

        /// `update`, dropping its effects.
        fn apply(&mut self, msg: Msg) {
            let _ = self.update(msg);
        }
    }
    use crate::media::{FakeImageLoader, ImageStore};
    use crate::render::paint::{PaintInput, paint};
    use crate::render::text::TextStore;
    use crate::runtime::App;
    use crate::snapshot::{self, Shot};
    use crate::testing::{By, Key, Modifiers, Screen, TestApp, layout_of};
    use crate::theme::{AppKitGeneration, Appearance, Density, Theme};

    /// The demo's tree as the session shows it: built, with its images
    /// decoded.
    fn build_view(demo: &Demo) -> crate::element::Element<Msg> {
        let mut tree = crate::runtime::build_view(demo);
        ImageStore::new(std::sync::Arc::new(FakeImageLoader::new())).load_now(&mut tree);
        tree
    }

    /// Two-field rows on PageGrid span 6: side by side from `md` (and half
    /// of the 8-col `sm` grid), stacked full-width below 640 pt; never wider
    /// than the window (the fields keep their 180 pt minimum).
    #[test]
    fn field_rows_stack_in_a_narrow_window() {
        use crate::theme::MixStyle;
        let rows = [
            (Demo::default(), ("Nombre", "Correo")),
            (Demo::at(Section::Fields), ("Buscar", "Contraseña")),
        ];
        let themes = [
            Theme::kai(Density::Comfortable),
            Theme::appkit_static(AppKitGeneration::Aqua11, Appearance::Light).with_mix(MixStyle::C),
            Theme::appkit_static(AppKitGeneration::Glass27, Appearance::Dark),
        ];
        for theme in themes {
            let at = |width: f32| {
                rows.each_ref()
                    .map(|(demo, pair)| row_bounds(&theme, demo, *pair, width))
            };
            for (first, second) in at(1024.0) {
                assert_eq!(first.y, second.y, "{:?}", theme.kind);
                assert!(second.x >= first.x + first.width);
            }
            for width in [639.0, 420.0, 400.0] {
                for (first, second) in at(width) {
                    assert_eq!(first.x, second.x, "{width} {:?}", theme.kind);
                    assert!(second.y >= first.y + first.height);
                    assert!(second.x + second.width <= width, "{width} {:?}", theme.kind);
                }
            }
        }
    }

    /// Bounds of the shells (the 180 pt minimum grid cells) of two fields.
    fn row_bounds(theme: &Theme, demo: &Demo, (a, b): (&str, &str), width: f32) -> (Rect, Rect) {
        let tree = build_view(demo);
        let layout = Layout::compute(
            &tree,
            &LayoutInput {
                viewport: Size {
                    width,
                    height: 700.0,
                },
                theme,
                interaction: &Interaction::default(),
                scroll: &ScrollOffsets::new(),
            },
            &mut StyleCache::default(),
            &mut FixedMeasure,
        );
        let bounds = |label| {
            let input = find(demo, By::label(label));
            // The field's shell: the input's grid cell ancestor.
            let path = tree.path_to(input).unwrap();
            let cell = path.iter().rev().find(|id| {
                layout
                    .node(**id)
                    .is_some_and(|n| n.style.min_width == crate::style::Length::Px(180.0))
            });
            layout.bounds(*cell.unwrap()).unwrap()
        };
        (bounds(a), bounds(b))
    }

    /// A field and a button in an `items-end` row (mix C, `near` label
    /// above the field): same height, same bottom, for regular fields with
    /// regular buttons and compact fields with small ones, on Aqua11 and
    /// Glass27, light and dark.
    #[test]
    fn mix_c_buttons_line_up_with_fields_in_an_items_end_row() {
        use crate::components::{ButtonVariant, FieldDensity, TextField, button};
        use crate::element::{ControlSize, Element, div};
        for generation in [AppKitGeneration::Aqua11, AppKitGeneration::Glass27] {
            for appearance in [Appearance::Light, Appearance::Dark] {
                let theme = Theme::appkit_static(generation, appearance);
                assert_eq!(theme.mix, crate::theme::MixStyle::AppKit);
                let theme = theme.with_mix(crate::theme::MixStyle::C);
                for (density, size, height) in [
                    (FieldDensity::Default, ControlSize::Regular, 36.0),
                    (FieldDensity::Compact, ControlSize::Small, 32.0),
                ] {
                    let mut tree: Element<Msg> = div().class("w-full h-full p-4").child(
                        div().class("flex items-end gap-2").children([
                            TextField::new("Campo", "x")
                                .test_id("campo")
                                .density(density)
                                .build(),
                            button("Enviar", ButtonVariant::Primary)
                                .control_size(size)
                                .test_id("submit"),
                        ]),
                    );
                    tree.assign_ids();
                    let layout = Layout::compute(
                        &tree,
                        &LayoutInput {
                            viewport: Size {
                                width: 600.0,
                                height: 200.0,
                            },
                            theme: &theme,
                            interaction: &Interaction::default(),
                            scroll: &ScrollOffsets::new(),
                        },
                        &mut StyleCache::default(),
                        &mut FixedMeasure,
                    );
                    let screen = Screen::new(&layout);
                    let field = screen.get_by_test_id("campo.shell").bounds();
                    let button = screen.get_by_test_id("submit").bounds();
                    let at = format!("{generation:?} {appearance:?} {size:?}");
                    assert_eq!(field.height, height, "{at}");
                    assert_eq!(button.height, height, "{at}");
                    assert_eq!(button.y + button.height, field.y + field.height, "{at}");
                }
            }
        }
    }

    fn layout(demo: &Demo, viewport: Size, styles: &mut StyleCache) -> Layout {
        Layout::compute(
            &build_view(demo),
            &LayoutInput {
                viewport,
                theme: &Theme::kai(Density::Comfortable),
                interaction: &Interaction::default(),
                scroll: &ScrollOffsets::new(),
            },
            styles,
            &mut FixedMeasure,
        )
    }

    #[test]
    fn update_follows_messages_and_view_reflects_the_model() {
        let mut demo = Demo::default();
        demo.apply(Msg::Increment);
        demo.apply(Msg::Increment);
        assert_eq!(demo.clicks, 2);
        assert_eq!(
            demo.title(),
            format!("Kai UI Desktop v{} — 2 clics", crate::version::VERSION)
        );
        demo.apply(Msg::Submit);
        assert!(demo.submissions.is_empty(), "empty names are not submitted");
        demo.apply(Msg::NameChanged("  Ana ".to_owned()));
        let view = build_view(&demo);
        let mut texts = Vec::new();
        view.walk(&mut |element| {
            if let Kind::Text(text) = &element.kind {
                texts.push(text.clone());
            }
        });
        assert!(texts.contains(&"Hola, Ana 👋".to_owned()));
        assert!(texts.contains(&"2".to_owned()));
        demo.apply(Msg::Submit);
        assert_eq!(demo.submissions, vec!["Ana".to_owned()]);
        assert!(demo.name.is_empty());
        demo.apply(Msg::Reset);
        assert_eq!(demo.clicks, 0);
    }

    #[test]
    fn return_in_name_field_activates_the_default_button() {
        use crate::ui_engine::{Intent, Key, Modifiers, TextIntent, UiModel, dispatch};

        let return_key = Intent::Text(TextIntent::KeyDown {
            key: Key::Enter,
            modifiers: Modifiers::NONE,
            repeat: false,
        });
        let mut demo = Demo::default();
        let name = find(&demo, By::test_id("form.name.input"));
        let order = FocusOrder::from_tree(&build_view(&demo));
        let mut model = UiModel::default();
        dispatch(&mut model, &order, Intent::SetFocus(Some(name)));
        let empty = dispatch(&mut model, &order, return_key.clone());
        assert_eq!(empty.activated, None, "«Enviar» is disabled without a name");

        demo.apply(Msg::NameChanged("Ana".to_owned()));
        let send = find(&demo, By::test_id("form.submit"));
        let order = FocusOrder::from_tree(&build_view(&demo));
        let result = dispatch(&mut model, &order, return_key.clone());
        assert_eq!(result.activated.map(|a| a.widget), Some(send));

        // «Correo» keeps its on_submit.
        let email = find(&demo, By::test_id("form.email.input"));
        dispatch(&mut model, &order, Intent::SetFocus(Some(email)));
        let result = dispatch(&mut model, &order, return_key);
        assert_eq!(result.activated, None);
        assert_eq!(result.text_actions.len(), 1);
    }

    #[test]
    fn clicked_buttons_show_no_focus_ring_but_fields_keep_their_focus_look() {
        use crate::ui_engine::{Intent, Key, Modifiers, TextIntent, UiModel, dispatch};

        let tab = |shift| {
            Intent::Text(TextIntent::KeyDown {
                key: Key::Tab,
                modifiers: Modifiers {
                    shift,
                    ..Modifiers::NONE
                },
                repeat: false,
            })
        };
        let demo = Demo::default();
        let tree = build_view(&demo);
        let order = FocusOrder::from_tree(&tree);
        let theme = Theme::kai(Density::Comfortable);
        let mut styles = StyleCache::default();
        let mut compute = |model: &UiModel| {
            Layout::compute(
                &tree,
                &LayoutInput {
                    viewport: Size {
                        width: 1024.0,
                        height: 768.0,
                    },
                    theme: &theme,
                    interaction: &Interaction {
                        focused: *model.focused.get(),
                        focus_visible: *model.focus_visible.get(),
                        ..Interaction::default()
                    },
                    scroll: &ScrollOffsets::new(),
                },
                &mut styles,
                &mut FixedMeasure,
            )
        };
        let button = find(&demo, By::role(Role::Button).name("Incrementar"));
        let ring = |layout: &Layout| layout.node(button).unwrap().style.ring.width;
        let mut model = UiModel::default();

        dispatch(&mut model, &order, Intent::PointerDown(Some(button)));
        dispatch(&mut model, &order, Intent::PointerUp(Some(button)));
        assert_eq!(*model.focused.get(), Some(button));
        assert_eq!(ring(&compute(&model)), 0.0, "mouse click: no ring");

        dispatch(&mut model, &order, tab(false));
        dispatch(&mut model, &order, tab(true));
        assert_eq!(*model.focused.get(), Some(button));
        assert_eq!(ring(&compute(&model)), 2.0, "Tab: ring");

        // A clicked text field looks exactly as one reached with Tab.
        let name = find(&demo, By::label("Nombre"));
        let path = tree.path_to(name).unwrap();
        let field_look = |layout: &Layout| {
            path.iter()
                .map(|id| format!("{:?}", layout.node(*id).unwrap().style))
                .collect::<Vec<_>>()
        };
        let unfocused = field_look(&compute(&UiModel::default()));
        dispatch(&mut model, &order, Intent::PointerDown(Some(name)));
        assert!(!*model.focus_visible.get());
        let clicked = field_look(&compute(&model));
        dispatch(&mut model, &order, Intent::SetFocus(Some(name)));
        assert!(*model.focus_visible.get());
        assert_eq!(clicked, field_look(&compute(&model)));
        assert_ne!(clicked, unfocused, "the focused field is highlighted");
    }

    #[test]
    fn demo_focus_order_follows_document_order_and_skips_disabled() {
        // Toolbar items (one stop), its search, the sidebar (one stop),
        // the background effects (one stop; «Animado» starts disabled),
        // «Incrementar» («Reiniciar» starts disabled) and the form
        // («Enviar» starts disabled).
        let home_view = [
            "bar.sidebar",
            "bar.find.input",
            "nav.0",
            "background.efecto.0",
            "counter.increment",
            "form.name.input",
            "form.email.input",
            "form.role",
            "form.terms",
            "form.notificaciones.track",
            "form.clear",
        ];
        assert_eq!(tab_stops(&mut TestApp::new(Demo::default())), home_view);

        let fields_list = tab_stops(&mut TestApp::new(Demo::at(Section::Fields)));
        assert_eq!(
            fields_list[2], "nav.1",
            "the sidebar enters at the selection"
        );
        let password = fields_list
            .iter()
            .position(|stop| stop == "form.password.input");
        assert_eq!(
            fields_list[password.unwrap() + 1],
            "form.password.toggle",
            "the eye follows its field"
        );

        let mut glow = Demo::default();
        glow.apply(Msg::SetBackground(BackgroundKind::Glow));
        let stops = tab_stops(&mut TestApp::new(glow));
        assert_eq!(
            stops.len(),
            home_view.len() + 1,
            "with an effect «Animado» joins"
        );
        assert_eq!(stops[4], "Animado");
    }

    /// Tab stops from the start of the window until they wrap around:
    /// each one's `test_id`, or its name without one.
    fn tab_stops(app: &mut TestApp<Demo>) -> Vec<String> {
        let mut stops: Vec<String> = Vec::new();
        for _ in 0..60 {
            app.press(Key::Tab, Modifiers::NONE);
            let focused = app.session().focused().expect("Tab focuses something");
            let screen = app.screen();
            let found = screen.by_id(focused).unwrap();
            let stop = found
                .test_id()
                .map(str::to_owned)
                .or_else(|| found.name())
                .unwrap_or_default();
            if stops.first() == Some(&stop) {
                return stops;
            }
            stops.push(stop);
        }
        panic!("Tab does not wrap around: {stops:?}");
    }

    /// The background panel sets the effect and its motion; the app hands
    /// them to the runtime, `None` (static) by default.
    #[test]
    fn background_panel_messages_drive_the_window_background() {
        let mut demo = Demo::default();
        assert_eq!(demo.background(), BackgroundEffect::NONE);
        assert!(!demo.background().is_visible());
        demo.apply(Msg::SetBackground(BackgroundKind::Aurora));
        assert_eq!(demo.background().kind, BackgroundKind::Aurora);
        assert!(!demo.background().is_animated());
        demo.apply(Msg::ToggleBackgroundMotion);
        assert!(demo.background().is_animated());
        demo.apply(Msg::SetBackground(BackgroundKind::None));
        assert!(
            !demo.background().is_animated(),
            "no effect, nothing to animate"
        );
        demo.apply(Msg::ToggleBackgroundMotion);
        assert_eq!(demo.background_motion, BackgroundMotion::Static);
        // The selected option is pressed; clicking another sends its kind.
        demo.apply(Msg::SetBackground(BackgroundKind::Mesh));
        let tree = build_view(&demo);
        let mut options = Vec::new();
        tree.walk(&mut |element| {
            if let Some(Msg::SetBackground(kind)) = &element.handlers.on_click {
                options.push((*kind, element.is_pressed()));
            }
        });
        assert_eq!(
            options,
            [
                (BackgroundKind::None, false),
                (BackgroundKind::Mesh, true),
                (BackgroundKind::Aurora, false),
                (BackgroundKind::Glow, false),
            ]
        );
    }

    #[test]
    fn background_env_parses_kind_and_motion_and_warns_on_unknown_values() {
        use super::background_from_env;
        assert_eq!(
            background_from_env(None, None),
            (BackgroundKind::None, BackgroundMotion::Static, vec![])
        );
        assert_eq!(
            background_from_env(Some("aurora"), Some("1")),
            (BackgroundKind::Aurora, BackgroundMotion::Animated, vec![])
        );
        assert_eq!(
            background_from_env(Some("GLOW"), Some("0")).0,
            BackgroundKind::Glow
        );
        let (kind, motion, warnings) = background_from_env(Some("bokeh"), Some("si"));
        assert_eq!(
            (kind, motion),
            (BackgroundKind::None, BackgroundMotion::Static)
        );
        assert_eq!(warnings.len(), 2, "{warnings:?}");
        assert!(warnings[0].starts_with("KAI_BG=bokeh"));
    }

    #[test]
    fn demo_uses_only_supported_classes_and_fills_the_window() {
        let mut styles = StyleCache::default();
        let mut demo = Demo::default();
        demo.apply(Msg::NameChanged("Ana".to_owned()));
        let layout = layout(
            &demo,
            Size {
                width: 420.0,
                height: 340.0,
            },
            &mut styles,
        );
        assert!(styles.unknown().is_empty(), "{:?}", styles.unknown());
        let root = layout.root().unwrap();
        assert_eq!((root.bounds.width, root.bounds.height), (420.0, 340.0));
        let screen = Screen::new(&layout);
        let content = screen.get_by_test_id("content").node();
        let scroll = content.scroll.expect("content scrolls");
        assert!(scroll.max > 0.0, "the small window overflows");
        let header = screen.get(By::role(Role::Toolbar)).node();
        assert_eq!(header.bounds.y, 0.0, "the toolbar stays fixed");
        assert_eq!(content.clip, None);
    }

    /// Full layout + paint of the demo view with real cosmic-text
    /// measurement and no cache; widths alternate so text re-wraps every
    /// frame. Run with:
    /// Cost of [`Element::map`] on whole demo views, against building them:
    /// `cargo test --release -- --ignored --nocapture bench_element_map`
    #[test]
    #[ignore = "benchmark; run explicitly in release mode"]
    fn bench_element_map() {
        #[derive(Clone, Debug)]
        struct Wrapped(#[allow(dead_code)] Msg);

        fn count(element: &crate::element::Element<Wrapped>) -> usize {
            1 + element.children.iter().map(count).sum::<usize>()
        }
        fn percentile(samples: &mut [Duration], p: f64) -> Duration {
            samples.sort();
            samples[((samples.len() - 1) as f64 * p) as usize]
        }

        const ITERATIONS: usize = 300;
        for section in [
            Section::Home,
            Section::Fields,
            Section::Table,
            Section::Map,
            Section::Catalog,
        ] {
            let demo = Demo::at(section);
            let mut build = Vec::with_capacity(ITERATIONS);
            let mut mapped = Vec::with_capacity(ITERATIONS);
            let mut nodes = 0;
            for _ in 0..ITERATIONS {
                let start = Instant::now();
                let tree = demo.view();
                build.push(start.elapsed());
                let start = Instant::now();
                let tree = tree.map(Wrapped);
                mapped.push(start.elapsed());
                nodes = count(&tree);
            }
            let (b50, m50) = (percentile(&mut build, 0.5), percentile(&mut mapped, 0.5));
            let (b95, m95) = (percentile(&mut build, 0.95), percentile(&mut mapped, 0.95));
            println!(
                "{section:?}: {nodes} nodos · view p50 {b50:?} p95 {b95:?} · map p50 {m50:?} p95 {m95:?} ({:.1} % de view)",
                100.0 * m50.as_secs_f64() / b50.as_secs_f64()
            );
        }
    }

    /// `cargo test --release -- --ignored --nocapture bench_layout_and_paint`
    #[test]
    #[ignore = "benchmark; run explicitly in release mode"]
    fn bench_layout_and_paint() {
        const ITERATIONS: usize = 400;
        const WIDTHS: [f32; 4] = [1024.0, 800.0, 600.0, 420.0];
        let theme = Theme::kai(Density::Comfortable);
        let mut demo = Demo::default();
        demo.apply(Msg::NameChanged("Ana".to_owned()));
        let mut text = TextStore::new(FontSystem::new());
        let mut styles = StyleCache::default();

        let mut samples: Vec<Duration> = Vec::with_capacity(ITERATIONS);
        let mut ops = 0;
        for index in 0..ITERATIONS {
            let viewport = Size {
                width: WIDTHS[index % WIDTHS.len()],
                height: 720.0,
            };
            let start = Instant::now();
            let tree = build_view(&demo);
            let layout = Layout::compute(
                &tree,
                &LayoutInput {
                    viewport,
                    theme: &theme,
                    interaction: &Interaction::default(),
                    scroll: &ScrollOffsets::new(),
                },
                &mut styles,
                &mut text,
            );
            let list = paint(&PaintInput {
                layout: &layout,
                theme: &theme,
                focused: None,
                window: Default::default(),
                text_fields: &[],
                background: Default::default(),
                time: None,
                documents: &[],
                caret_visible: true,
            });
            samples.push(start.elapsed());
            ops += list.ops.len();
        }
        samples.sort();
        let total: Duration = samples.iter().sum();
        eprintln!(
            "view+layout+paint: {ITERATIONS} iteraciones, media {:?}, p50 {:?}, p95 {:?}, máx {:?} ({} ops/frame)",
            total / ITERATIONS as u32,
            samples[ITERATIONS / 2],
            samples[ITERATIONS * 95 / 100],
            samples[ITERATIONS - 1],
            ops / ITERATIONS,
        );
        assert!(ops > 0);
    }

    /// The single element of the demo's view matching `by`.
    #[track_caller]
    fn find(demo: &Demo, by: By) -> WidgetId {
        let layout = layout_of(&build_view(demo));
        Screen::new(&layout).get(by).id()
    }

    /// Header block of the card titled `title`.
    fn card_header_of(demo: &Demo, title: &str) -> WidgetId {
        let layout = layout_of(&build_view(demo));
        let screen = Screen::new(&layout);
        screen.get_by_text(title).parent().unwrap().id()
    }

    const BUTTONS_CARD: &str = "Botones, Tamaños e IconButtons";

    #[test]
    fn test_ids_are_unique_and_point_at_the_controls() {
        let tree = build_view(&Demo::default());
        assert_eq!(tree.validate_ids(), Vec::<String>::new());
        let layout = layout_of(&tree);
        let screen = Screen::new(&layout);
        let send = screen.get_by_test_id("form.submit");
        assert_eq!(send.role(), Role::Button);
        assert_eq!(send.name().as_deref(), Some("Enviar"));
        assert_eq!(
            screen.get(By::role(Role::Button).name("Enviar")).id(),
            send.id()
        );
        assert_eq!(
            screen.get(By::role(Role::Button).name("Limpiar")).test_id(),
            Some("form.clear")
        );
        assert_eq!(
            screen.get_by_label("Nombre").test_id(),
            Some("form.name.input")
        );
        assert_eq!(
            screen.get_by_label("Correo").test_id(),
            Some("form.email.input")
        );
        assert_eq!(
            screen.get_by_test_id("bar.add").name().as_deref(),
            Some("Sumar clic")
        );
        assert_eq!(
            screen.get_by_test_id("bar.find.input").name().as_deref(),
            Some("Filtrar clientes")
        );
        // Sidebar rows are parts of `nav`, by position.
        for (index, section) in Section::ALL.into_iter().enumerate() {
            let row = screen.get(sidebar_row(&section.item().label));
            assert_eq!(row.test_id(), Some(format!("nav.{index}").as_str()));
        }

        // Every section: unique ids, and its controls reachable by id.
        for section in Section::ALL {
            let tree = build_view(&Demo::at(section));
            assert_eq!(tree.validate_ids(), Vec::<String>::new(), "{section:?}");
        }
        let layout = layout_of(&build_view(&Demo::at(Section::Fields)));
        let screen = Screen::new(&layout);
        assert_eq!(
            screen.get_by_label("Contraseña").test_id(),
            Some("form.password.input")
        );
        let form = screen.within(screen.get_by_test_id("form.password"));
        assert_eq!(
            form.get(By::role(Role::Button).name("Mostrar contraseña"))
                .test_id(),
            Some("form.password.toggle")
        );
        assert_eq!(
            screen.get_by_label("PIN").test_id(),
            Some("fields.pin.input")
        );
        assert_eq!(
            screen.get_by_test_id("fields.notes.input").role(),
            Role::TextArea
        );

        let layout = layout_of(&build_view(&Demo::at(Section::Selection)));
        let screen = Screen::new(&layout);
        assert_eq!(
            screen.get_by_test_id("selection.all").role(),
            Role::Checkbox
        );
        assert_eq!(
            screen.get_by_test_id("selection.country").role(),
            Role::PopUpButton
        );
        assert_eq!(
            screen
                .get_by_test_id("selection.alineacion.1")
                .name()
                .as_deref(),
            Some("Centro")
        );

        let layout = layout_of(&build_view(&Demo::at(Section::Table)));
        let screen = Screen::new(&layout);
        assert_eq!(screen.get_by_test_id("table.clients").role(), Role::Table);
        assert_eq!(screen.get_by_test_id("table.items").role(), Role::List);
    }

    #[test]
    fn typing_a_name_enables_send_and_the_click_submits_it() {
        let mut app = TestApp::new(Demo::default());
        let send = || By::test_id("form.submit");
        assert!(!app.is_enabled(send()));
        app.click(send());
        assert_eq!(app.messages(), [], "a disabled button sends nothing");

        app.type_text(By::label("Nombre"), "Ana");
        assert_eq!(app.value(By::label("Nombre")), "Ana");
        assert_eq!(
            app.messages(),
            ["A", "An", "Ana"].map(|name| Msg::NameChanged(name.to_owned()))
        );
        assert!(app.is_enabled(send()));
        app.screen().get(By::text("Hola, Ana 👋"));

        app.click(send());
        assert_eq!(app.messages().last(), Some(&Msg::Submit));
        assert_eq!(app.app().submissions, ["Ana"]);
        assert_eq!(app.value(By::label("Nombre")), "");
        assert!(!app.is_enabled(send()));
        app.screen().get(By::text("Último: Ana"));
    }

    #[test]
    fn the_form_picks_a_role_and_accepts_the_terms_and_clear_resets_them() {
        let mut app = TestApp::new(Demo::default());
        app.screen().get(By::text("Elegir rol"));
        app.click(By::test_id("form.role"));
        app.click(By::test_id("kai.menu.1"));
        app.click(By::test_id("form.terms"));
        assert_eq!(app.messages(), [Msg::SetRole(1), Msg::ToggleTerms]);
        assert_eq!((app.app().role, app.app().terms), (Some(1), true));
        app.screen().get(By::text("Desarrollo"));
        assert_eq!(
            app.screen().get_by_test_id("form.terms").role(),
            Role::Checkbox
        );

        app.click(By::test_id("form.clear"));
        assert_eq!((app.app().role, app.app().terms), (None, false));
        app.screen().get(By::text("Elegir rol"));
    }

    /// The effect is a segmented control: one Tab stop, arrows move between
    /// effects and Space applies the focused one.
    #[test]
    fn background_effects_are_one_segmented_tab_stop() {
        let mut app = TestApp::new(Demo::default());
        let segment = |name: &str| By::role(Role::Radio).name(name);
        // Header button, sidebar, its collapse button, then the effects.
        for _ in 0..4 {
            app.press(Key::Tab, Modifiers::NONE);
        }
        assert!(app.is_focused(segment("Ninguno")));
        app.press(Key::ArrowRight, Modifiers::NONE);
        app.press(Key::Space, Modifiers::NONE);
        assert_eq!(app.app().background, BackgroundKind::Mesh);
        app.press(Key::Tab, Modifiers::NONE);
        let focused = app.session().focused().unwrap();
        let segments: Vec<_> = app
            .screen()
            .all(By::role(Role::Radio))
            .iter()
            .map(|segment| segment.id())
            .collect();
        assert_eq!(segments.len(), 4);
        assert!(!segments.contains(&focused), "Tab leaves the group");
    }

    fn sidebar_row(name: &str) -> By {
        By::role(Role::SidebarItem).name(name)
    }

    /// Something only section `section` shows.
    fn landmark(section: Section) -> &'static str {
        match section {
            Section::Home => "counter",
            Section::Fields => "form.password",
            Section::Buttons => "buttons.secondary",
            Section::Selection => "selection",
            Section::Containers => "containers.card",
            Section::Badges => "badges",
            Section::Multimedia => "media.fields",
            Section::Table => "table.clients",
            Section::Map => "map.view",
            Section::Charts => "charts.bars",
            Section::Document => "document.editor",
            Section::Dialogs => "dialogs",
            Section::TemplateList => "template.items",
            Section::TemplateForm => "template.form",
            Section::TemplateData => "template.data",
            Section::Catalog => "catalog",
        }
    }

    #[cfg(feature = "document")]
    #[test]
    fn document_types_bold_and_undo() {
        let mut demo = Demo::at(Section::Document);
        demo.doc_markdown.clear();
        let mut app = TestApp::new(demo);
        let editor = || By::test_id("document.editor");
        assert_eq!(
            app.screen().get(editor()).name().as_deref(),
            Some("Informe de demostración")
        );
        app.type_text(editor(), "Hola");
        assert_eq!(app.value(editor()), "Hola");
        app.click(By::test_id("document.ribbon.cmd.select.all"));
        assert!(app.perform("format.bold"));
        assert_eq!(app.value(editor()), "Hola");
        assert!(app.perform("edit.undo"));
        assert_eq!(app.value(editor()), "Hola");
        assert!(app.perform("edit.undo"));
        assert_ne!(app.value(editor()), "Hola");
    }

    #[cfg(feature = "document")]
    #[test]
    fn document_pointer_drag_selects_text() {
        use crate::geometry::Point;
        let mut demo = Demo::at(Section::Document);
        demo.doc_markdown.clear();
        let mut app = TestApp::new(demo);
        let editor = || By::test_id("document.editor");
        app.type_text(editor(), "Hola mundo seleccionable");
        assert_eq!(app.value(editor()), "Hola mundo seleccionable");

        // El texto del cuerpo comienza tras el margen superior e izquierdo (72pt).
        let start = app.document_page_to_local(editor(), Point { x: 72.0, y: 72.0 });
        let end = app.document_page_to_local(editor(), Point { x: 180.0, y: 72.0 });
        app.drag_at(editor(), start.x, start.y, end.x - start.x, 0.0);
        assert!(
            !app.document_selection_collapsed(editor()),
            "expected non-empty selection after pointer drag across text"
        );
    }

    #[cfg(feature = "document")]
    #[test]
    fn document_shift_arrows_extend_selection() {
        let mut demo = Demo::at(Section::Document);
        demo.doc_markdown.clear();
        let mut app = TestApp::new(demo);
        let editor = || By::test_id("document.editor");
        app.type_text(editor(), "abcdef");
        app.press(Key::ArrowLeft, Modifiers::NONE);
        app.press(Key::ArrowLeft, Modifiers::NONE);
        app.press(Key::ArrowLeft, Modifiers::NONE);
        assert!(app.document_selection_collapsed(editor()));
        app.press(
            Key::ArrowRight,
            Modifiers {
                shift: true,
                ..Modifiers::NONE
            },
        );
        app.press(
            Key::ArrowRight,
            Modifiers {
                shift: true,
                ..Modifiers::NONE
            },
        );
        assert!(
            !app.document_selection_collapsed(editor()),
            "Shift+Arrow should extend Document selection"
        );
    }

    #[cfg(feature = "document")]
    #[test]
    fn document_clipboard_copy_cut_paste_plain() {
        use crate::render::DrawOp;
        use crate::testing::MemoryClipboard;
        let clipboard = MemoryClipboard::new();
        let mut demo = Demo::at(Section::Document);
        demo.doc_markdown.clear();
        let mut app = TestApp::builder(demo).clipboard(clipboard.clone()).build();
        let editor = || By::test_id("document.editor");
        app.type_text(editor(), "ClipDoc");
        assert_eq!(app.value(editor()), "ClipDoc");
        app.click(By::test_id("document.ribbon.cmd.select.all"));
        app.press(Key::C, CMD);
        assert_eq!(clipboard.text().as_deref(), Some("ClipDoc"));
        app.press(Key::X, CMD);
        assert!(
            app.value(editor()).is_empty() || !app.value(editor()).contains("ClipDoc"),
            "cut should clear: {:?}",
            app.value(editor())
        );
        assert_eq!(clipboard.text().as_deref(), Some("ClipDoc"));
        app.press(Key::V, CMD);
        assert!(
            app.value(editor()).contains("ClipDoc"),
            "paste plain: {:?}",
            app.value(editor())
        );
        // Cuerpo pinta glifos como ImageDraw (atlas), no DrawOp::Text de runs.
        let list = app.draw_list();
        let images = list
            .ops
            .iter()
            .filter(|op| matches!(op, DrawOp::Image(_)))
            .count();
        assert!(
            images > 0,
            "expected glyph/image draws after paste, got {}",
            images
        );
    }

    #[cfg(feature = "document")]
    #[test]
    fn document_clipboard_copy_preserves_bold_via_html() {
        use crate::testing::MemoryClipboard;
        let clipboard = MemoryClipboard::new();
        let mut demo = Demo::at(Section::Document);
        demo.doc_markdown.clear();
        let mut app = TestApp::builder(demo).clipboard(clipboard.clone()).build();
        let editor = || By::test_id("document.editor");
        app.type_text(editor(), "RichDoc");
        app.click(By::test_id("document.ribbon.cmd.select.all"));
        assert!(app.perform("format.bold"));
        app.press(Key::C, CMD);
        assert_eq!(clipboard.text().as_deref(), Some("RichDoc"));
        assert!(
            clipboard.html().is_some_and(|h| h.contains("RichDoc")),
            "⌘C must publish HTML for rich interop: {:?}",
            clipboard.html()
        );
        // Vaciar el editor y pegar: el markdown debe conservar bold.
        app.press(Key::X, CMD);
        app.press(Key::V, CMD);
        let md = app.app().doc_markdown.clone();
        assert!(
            md.contains("**") && md.contains("RichDoc"),
            "paste after cut should keep bold in persisted markdown: {md:?}"
        );
    }

    #[cfg(feature = "document")]
    #[test]
    fn document_multipage_scrolls_and_keeps_hit_target() {
        let mut app = TestApp::new(Demo::at(Section::Document));
        let editor = || By::test_id("document.editor");
        let before = app.value(editor());
        assert!(
            before.contains("Párrafo 40") || before.len() > 200,
            "demo should seed multi-page markdown, got len={}",
            before.len()
        );
        app.scroll_wheel(editor(), -480.0);
        app.click(editor());
        app.type_text(editor(), "¶");
        let after = app.value(editor());
        assert!(
            after.contains('¶') || after.contains("¶"),
            "typing after scroll should insert; after_len={}",
            after.len()
        );
        assert_ne!(after, before);
    }

    #[cfg(feature = "document")]
    #[test]
    fn document_inserts_table_and_picture_from_toolbar() {
        let mut demo = Demo::at(Section::Document);
        demo.doc_markdown.clear();
        let mut app = TestApp::new(demo);
        let editor = || By::test_id("document.editor");
        app.click(editor());
        assert!(app.perform("insert.table"));
        assert!(app.perform("insert.picture"));
        // Undo picture then table keeps the editor responsive.
        assert!(app.perform("edit.undo"));
        assert!(app.perform("edit.undo"));
        app.type_text(editor(), "ok");
        assert!(app.value(editor()).contains("ok"));
    }

    #[cfg(feature = "document")]
    #[test]
    fn document_ribbon_bold_and_insert_table() {
        let mut demo = Demo::at(Section::Document);
        demo.doc_markdown.clear();
        let mut app = TestApp::new(demo);
        let editor = || By::test_id("document.editor");
        app.type_text(editor(), "Ribbon");
        assert_eq!(app.value(editor()), "Ribbon");
        app.click(By::test_id("document.ribbon.cmd.select.all"));
        app.click(By::test_id("document.ribbon.cmd.format.bold"));
        assert_eq!(app.value(editor()), "Ribbon");
        app.click(By::test_id("document.ribbon.tab.insert"));
        app.click(By::test_id("document.ribbon.cmd.insert.table"));
        assert!(app.perform("edit.undo"));
        app.type_text(editor(), "!");
        assert!(app.value(editor()).contains('!'));
    }

    #[test]
    fn preferences_survive_a_restart() {
        use crate::store::Store;

        let dir = std::env::temp_dir().join(format!("kai-demo-prefs-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let demo = Demo {
            store: Some(Store::at(&dir)),
            ..Demo::default()
        };
        let mut app = TestApp::new(demo);
        app.call("ui.go_to", serde_json::json!({ "section": "map" }))
            .expect("ui.go_to");
        app.run_until_idle();
        let saved: super::DemoPreferences = Store::at(&dir)
            .load()
            .expect("archivo legible")
            .expect("preferencias guardadas");
        assert_eq!(saved.section, Section::Map);

        // Una ejecución nueva arranca donde quedó la anterior.
        let mut other_one = Demo::default();
        other_one.apply_preferences(&saved);
        assert_eq!(other_one.section, Section::Map);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[cfg(feature = "document")]
    #[test]
    fn document_contextual_tabs_follow_table_and_picture() {
        let mut demo = Demo::at(Section::Document);
        demo.doc_markdown.clear();
        let mut app = TestApp::new(demo);
        let editor = || By::test_id("document.editor");
        let tab = |id: &str| By::test_id(format!("document.ribbon.tab.{id}"));
        app.type_text(editor(), "Texto");
        assert!(app.screen().try_get(tab("table")).is_err());
        assert!(app.screen().try_get(tab("picture")).is_err());

        app.click(tab("insert"));
        app.click(By::test_id("document.ribbon.cmd.insert.table"));
        assert!(
            app.screen().try_get(tab("table")).is_ok(),
            "caret en la tabla"
        );
        app.click(tab("table"));
        app.click(By::test_id("document.ribbon.cmd.table.style:TableGrid"));

        app.click(tab("insert"));
        app.click(By::test_id("document.ribbon.cmd.insert.picture"));
        assert!(
            app.screen().try_get(tab("picture")).is_ok(),
            "imagen seleccionada"
        );
        app.click(tab("picture"));
        app.click(By::test_id("document.ribbon.cmd.arrange.wrap:square"));
    }

    #[cfg(feature = "document")]
    #[test]
    fn document_ribbon_collapses_with_the_window_width() {
        let at = |width: f32| {
            TestApp::builder(Demo::at(Section::Document))
                .viewport(width, 800.0)
                .build()
        };
        let shown = |app: &TestApp<Demo>, by: By| app.screen().query(by).is_some();
        let gallery = || By::test_id("document.ribbon.cmd.para.heading2");
        let styles = || By::test_id("document.ribbon.styles");
        let paste_label = || By::text("Pegar");
        let group_label = || By::text("Portapapeles");

        let wide = at(1400.0);
        assert!(shown(&wide, gallery()) && !shown(&wide, styles()));
        assert!(shown(&wide, paste_label()) && shown(&wide, group_label()));

        // < xl: la galería pasa a un desplegable.
        let large = at(1100.0);
        assert!(!shown(&large, gallery()) && shown(&large, styles()));
        assert!(shown(&large, paste_label()));

        // < lg: los botones grandes quedan solo con icono.
        let medium = at(900.0);
        assert!(!shown(&medium, paste_label()) && shown(&medium, group_label()));
        assert!(shown(
            &medium,
            By::test_id("document.ribbon.cmd.edit.paste")
        ));

        // < md: sin etiquetas de grupo.
        let narrow = at(700.0);
        assert!(!shown(&narrow, group_label()));
    }

    #[cfg(feature = "document")]
    #[test]
    fn document_new_comment_asks_for_its_text() {
        let mut app = TestApp::new(Demo::at(Section::Document));
        let bar = || By::test_id("document.commentbar");
        assert!(app.screen().try_get(bar()).is_err());
        app.click(By::test_id("document.ribbon.tab.review"));
        app.click(By::test_id("document.ribbon.cmd.review.newComment"));
        assert!(app.screen().try_get(bar()).is_ok());
        app.click(By::test_id("document.comment.close"));
        assert!(app.screen().try_get(bar()).is_err());
    }

    #[cfg(feature = "document")]
    #[test]
    fn document_ribbon_shows_bold_pressed_and_cut_disabled() {
        let mut demo = Demo::at(Section::Document);
        demo.doc_markdown.clear();
        let mut app = TestApp::new(demo);
        let editor = || By::test_id("document.editor");
        // Sin selección: cut disabled en chrome.
        assert!(
            app.app()
                .doc_ribbon_disabled
                .iter()
                .any(|id| id == "edit.cut")
                || app.app().doc_ribbon_disabled.is_empty(),
            // vacío hasta el primer DocumentChange; tipar dispara sync
            "before edit disabled may be empty"
        );
        app.type_text(editor(), "Vivo");
        assert!(
            app.app()
                .doc_ribbon_disabled
                .iter()
                .any(|id| id == "edit.cut"),
            "cut disabled without selection: {:?}",
            app.app().doc_ribbon_disabled
        );
        app.click(By::test_id("document.ribbon.cmd.select.all"));
        app.click(By::test_id("document.ribbon.cmd.format.bold"));
        assert!(
            app.app()
                .doc_ribbon_pressed
                .iter()
                .any(|id| id == "format.bold"),
            "bold must show pressed: {:?}",
            app.app().doc_ribbon_pressed
        );
        assert!(
            !app.app()
                .doc_ribbon_disabled
                .iter()
                .any(|id| id == "edit.cut"),
            "cut enabled with selection: {:?}",
            app.app().doc_ribbon_disabled
        );
        // El botón del ribbon queda pressed en el árbol.
        let tree = build_view(app.app());
        let mut bold_pressed = false;
        tree.walk(&mut |el| {
            if el.test_id.as_deref() == Some("document.ribbon.cmd.format.bold") {
                bold_pressed = el.is_pressed();
            }
        });
        assert!(bold_pressed, "ribbon bold button should be pressed");
    }

    #[cfg(feature = "document")]
    #[test]
    fn document_ribbon_layout_review_view_tabs() {
        use crate::document::{RibbonMode, ribbon_catalog};
        let tabs = ribbon_catalog(RibbonMode::Full);
        assert!(tabs.iter().any(|t| t.id == "layout"));
        assert!(tabs.iter().any(|t| t.id == "review"));
        assert!(tabs.iter().any(|t| t.id == "view"));
        assert!(tabs.iter().any(|t| t.id == "references"));

        let mut demo = Demo::at(Section::Document);
        demo.doc_markdown.clear();
        let mut app = TestApp::new(demo);
        let editor = || By::test_id("document.editor");
        app.click(editor());
        app.type_text(editor(), "Tabs");

        app.click(By::test_id("document.ribbon.tab.layout"));
        app.click(By::test_id("document.ribbon.cmd.layout.orientation"));

        app.click(By::test_id("document.ribbon.tab.review"));
        app.click(By::test_id("document.ribbon.cmd.review.wordCount"));

        app.click(By::test_id("document.ribbon.tab.view"));
        app.click(By::test_id("document.ribbon.cmd.view.zoomIn"));

        app.click(By::test_id("document.ribbon.tab.references"));
        app.click(By::test_id("document.ribbon.cmd.references.footnote"));

        assert!(
            app.value(editor()).contains("Tabs") || !app.value(editor()).is_empty(),
            "editor should stay usable after ribbon cmds"
        );
    }

    #[cfg(feature = "document")]
    #[test]
    fn document_save_docx_via_file_panel() {
        let mut demo = Demo::at(Section::Document);
        demo.doc_markdown = "Hola archivo".into();
        let mut app = TestApp::new(demo);
        app.click(By::test_id("document.save"));
        let request = app.file_save_request().cloned().expect("save panel");
        assert_eq!(request.name, "informe.docx");
        assert_eq!(request.types, ["docx"]);

        let dir = std::env::temp_dir().join(format!("kai-doc-save-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("informe.docx");
        app.on_file_save(Some(path.clone()));
        app.run_until_idle();
        assert!(!app.app().doc_file_busy);
        let bytes = std::fs::read(&path).expect("docx written");
        assert!(bytes.starts_with(b"PK"), "docx zip");
        app.screen().get(By::text("Archivo listo"));
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[cfg(feature = "document")]
    #[test]
    fn document_open_docx_via_file_panel() {
        // Seed a .docx via Guardar, then Abrir into a fresh demo.
        let mut demo = Demo::at(Section::Document);
        demo.doc_markdown = "Desde disco".into();
        let mut app = TestApp::new(demo);
        app.click(By::test_id("document.save"));
        let dir = std::env::temp_dir().join(format!("kai-doc-open-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("seed.docx");
        app.on_file_save(Some(path.clone()));
        app.run_until_idle();
        assert!(path.exists());

        let mut demo = Demo::at(Section::Document);
        demo.doc_markdown.clear();
        let mut app = TestApp::new(demo);
        app.click(By::test_id("document.open"));
        let request = app.file_pick_request().cloned().expect("open panel");
        assert_eq!(request.types, ["docx"]);
        app.on_file_pick(Some(path.clone()));
        app.run_until_idle();
        assert!(!app.app().doc_file_busy);
        let value = app.value(By::test_id("document.editor"));
        assert!(
            value.contains("Desde disco"),
            "opened docx should restore text; got {value:?}"
        );
        app.screen().get(By::text("Documento abierto"));
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[cfg(feature = "document")]
    #[test]
    fn document_export_pdf_via_file_panel() {
        let mut demo = Demo::at(Section::Document);
        demo.doc_markdown = "PDF demo".into();
        let mut app = TestApp::new(demo);
        assert!(app.perform("document.export_pdf"));
        let request = app.file_save_request().cloned().expect("pdf panel");
        assert_eq!(request.name, "informe.pdf");
        assert_eq!(request.types, ["pdf"]);
        let dir = std::env::temp_dir().join(format!("kai-doc-pdf-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("informe.pdf");
        app.on_file_save(Some(path.clone()));
        app.run_until_idle();
        let bytes = std::fs::read(&path).expect("pdf written");
        assert!(bytes.starts_with(b"%PDF"), "got {} bytes", bytes.len());
        app.screen().get(By::text("Archivo listo"));
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn the_sidebar_shows_one_section_at_a_time() {
        let mut app = TestApp::new(Demo::default());
        app.screen().get(By::test_id("form.name"));
        app.click(sidebar_row("Campos"));
        assert_eq!(app.messages(), [Msg::GoTo(Section::Fields)]);
        app.screen().get(By::test_id("form.password"));
        assert!(app.screen().query(By::test_id("form.name")).is_none());
        assert!(app.is_focused(sidebar_row("Campos")));

        // ↓ walks every section in order, across the groups, and ↑ back.
        for pair in Section::ALL[1..].windows(2) {
            app.press(Key::ArrowDown, Modifiers::NONE);
            assert_eq!(app.app().section, pair[1]);
            app.screen().get(By::test_id(landmark(pair[1])));
            assert!(
                app.screen().query(By::test_id(landmark(pair[0]))).is_none(),
                "{:?} left",
                pair[0]
            );
        }
        for _ in 1..Section::ALL.len() {
            app.press(Key::ArrowUp, Modifiers::NONE);
        }
        assert_eq!(app.app().section, Section::Home);
        assert!(app.is_focused(sidebar_row("Inicio")));
    }

    /// Groups of the sidebar and their rows, in order.
    #[test]
    fn the_sidebar_groups_every_section() {
        let app = TestApp::new(Demo::default());
        let screen = app.screen();
        let nav = screen.within(screen.get_by_test_id("nav"));
        let titles: Vec<_> = nav
            .all(By::role(Role::Heading))
            .iter()
            .filter_map(|heading| heading.name())
            .collect();
        assert_eq!(
            titles,
            [
                "Demo",
                "Componentes",
                "Datos",
                "Plantillas",
                "Catálogo",
                "Superposiciones",
            ]
        );
        let rows: Vec<_> = nav
            .all(By::role(Role::SidebarItem))
            .iter()
            .filter_map(|row| row.name())
            .collect();
        assert_eq!(
            rows,
            [
                "Inicio",
                "Campos",
                "Botones",
                "Selección",
                "Contenedores",
                "Badges y alertas",
                "Multimedia",
                "Tabla",
                "Mapa",
                "Gráficos",
                "Documento",
                "Lista + detalle",
                "Formulario",
                "Vista de datos",
                "Catálogo vivo",
                "Diálogos y avisos",
            ]
        );
    }

    #[test]
    fn clicking_a_bar_selects_its_category() {
        let mut app = TestApp::new(Demo::at(Section::Charts));
        let plot = app.screen().get(By::test_id("charts.bars.plot"));
        assert_eq!(plot.name().as_deref(), Some("Clics por mes"));
        assert_eq!(
            plot.node().description.as_deref(),
            Some("Ene 4, Feb 12, Mar 7, Abr 15, May 9, Jun 11")
        );
        let width = plot.bounds().width;
        let height = plot.bounds().height;
        app.click_at(By::test_id("charts.bars.plot"), width * 0.08, height * 0.5);
        assert_eq!(app.messages(), [Msg::Month(0)]);
        assert_eq!(app.app().month, Some(0));
        app.screen().get(By::text("Seleccionado: Ene (4)"));
        app.click_at(By::test_id("charts.bars.plot"), width * 0.92, height * 0.5);
        assert_eq!(app.app().month, Some(5));
        app.screen().get(By::text("Seleccionado: Jun (11)"));
    }

    #[test]
    fn waiting_buttons_show_the_dots_until_their_task_ends() {
        let mut app = TestApp::new(Demo::at(Section::Buttons));
        let primary = By::test_id("wait.primary");
        let width = app.screen().get(primary.clone()).bounds().width;
        app.click(primary.clone());
        assert!(app.is_busy(primary.clone()));
        assert!(!app.is_busy(By::test_id("wait.secondary")));
        assert_eq!(app.screen().get(primary.clone()).bounds().width, width);
        assert_eq!(
            app.screen().get(primary.clone()).name().as_deref(),
            Some("Primary")
        );
        app.click(primary.clone());
        let waits = app.messages().iter();
        assert_eq!(waits.filter(|msg| **msg == Msg::Wait("Primary")).count(), 1);
        app.run_until_idle();
        assert!(!app.is_busy(primary));
        app.screen().get(By::text("Terminó: Primary"));
        assert!(app.is_busy(By::test_id("wait.cargando")), "loading(true)");
    }

    #[test]
    fn a_chained_save_and_a_background_sync_in_the_demo() {
        let mut app = TestApp::new(Demo::at(Section::Buttons));
        app.click(By::test_id("wait.reload"));
        app.run_next_tasks();
        assert!(app.messages().contains(&Msg::Reload));
        assert!(app.is_busy(By::test_id("wait.reload")));
        app.run_until_idle();
        assert!(!app.is_busy(By::test_id("wait.reload")));
        app.screen().get(By::text("Terminó: guardar y recargar"));

        let sync = By::test_id("wait.sync");
        app.click(sync.clone());
        app.click(sync.clone());
        assert!(!app.is_busy(sync));
        app.screen().get(By::text("Sincronizando (2)…"));
        app.run_until_idle();
        assert_eq!(app.app().syncing, 0);
    }

    #[test]
    fn the_wait_command_is_in_course_everywhere_while_it_runs() {
        const ID: &str = <SimulateWait as crate::commands::Action>::ID;
        let mut app = TestApp::new(Demo::at(Section::Buttons));
        let bound = By::test_id(ID);
        assert!(app.perform(ID));
        assert!(app.is_busy(bound.clone()));
        assert_eq!(
            app.available::<SimulateWait>(),
            Availability::disabled(crate::session::COMMAND_BUSY)
        );
        assert!(!app.perform(ID));
        app.run_until_idle();
        assert!(!app.is_busy(bound));
        assert_eq!(app.available::<SimulateWait>(), Availability::Enabled);
    }

    /// Every public constructor of `components` shows up in the demo,
    /// found by role, name or test id in some section (an overlay after
    /// the click that opens it). Items that only feed a constructor (data,
    /// class strings, timers) are listed apart; a new public item fails
    /// here until it is in one list or the other.
    #[test]
    fn the_demo_shows_every_component() {
        use std::collections::BTreeSet;
        const SOURCES: [&str; 20] = [
            include_str!("components.rs"),
            include_str!("components/empty_state.rs"),
            include_str!("components/color.rs"),
            include_str!("components/dialog.rs"),
            include_str!("components/media.rs"),
            include_str!("components/page_templates.rs"),
            include_str!("components/progress.rs"),
            include_str!("components/search.rs"),
            include_str!("components/sidebar.rs"),
            include_str!("components/table.rs"),
            include_str!("components/table/view.rs"),
            include_str!("components/tabs.rs"),
            include_str!("components/text_area.rs"),
            include_str!("components/toast.rs"),
            include_str!("components/toolbar.rs"),
            include_str!("charts/mod.rs"),
            include_str!("maps/mod.rs"),
            include_str!("maps/layers.rs"),
            include_str!("maps/editor.rs"),
            include_str!("document/mod.rs"),
        ];
        const NOT_WIDGETS: [&str; 34] = [
            "ChartSpec",
            "ChartSeries",
            "MapSpec",
            "Marker",
            "FitRequest",
            "paint",
            "crossing",
            "area_m2",
            "validate_image_file",
            "themed",
            "appkit_button_classes",
            "button_classes",
            "badge_classes",
            "toast_key",
            "toast_timer",
            "SidebarItem",
            "SidebarGroup",
            "Column",
            "TableView",
            "Date",
            "Sheet",
            "Toast",
            "PaletteItem",
            "MenuFrame",
            "DocumentSpec",
            "DocumentChange",
            "DocumentCmdRequest",
            "DocumentPaint",
            "DocumentGlyph",
            "DocumentFill",
            "DocumentLine",
            "DocumentImage",
            "master_list_span_classes",
            "master_detail_span_classes",
        ];
        use Section::*;
        let shown: Vec<(&[&str], Section, Option<&str>, By)> = vec![
            (&["toolbar"], Home, None, By::role(Role::Toolbar)),
            (&["toolbar_item"], Home, None, By::test_id("bar.add")),
            (&["toolbar_title"], Home, None, By::text("Kai UI Desktop")),
            (&["toolbar_spacer"], Home, None, By::test_id("bar.espacio")),
            (
                &["toolbar_separator"],
                Home,
                None,
                By::test_id("bar.separador"),
            ),
            (
                &["toolbar_search", "search_field", "SearchField"],
                Home,
                None,
                By::role(Role::SearchInput).name("Filtrar clientes"),
            ),
            (
                &["sidebar", "Sidebar"],
                Home,
                None,
                By::role(Role::Sidebar).name("Secciones"),
            ),
            (&["segmented"], Home, None, By::test_id("background.efecto")),
            (
                &["switch_labeled"],
                Home,
                None,
                By::role(Role::Switch).name("Notificaciones"),
            ),
            (
                &["checkbox"],
                Home,
                None,
                By::role(Role::Checkbox).name("Acepto los términos"),
            ),
            (&["select"], Home, None, By::test_id("form.role")),
            (&["stat"], Home, None, By::text("Envíos")),
            (&["TextField"], Fields, None, By::test_id("form.password")),
            (
                &["text_area", "TextArea"],
                Fields,
                None,
                By::role(Role::TextArea).name("Notas"),
            ),
            (&["button"], Buttons, None, By::test_id("buttons.secondary")),
            (
                &["button_sized"],
                Buttons,
                None,
                By::role(Role::Button).name("Grande"),
            ),
            (&["Button"], Buttons, None, By::test_id("buttons.bordered")),
            (
                &["DotProgress", "dot_progress"],
                Buttons,
                None,
                By::role(Role::ProgressIndicator).name("Cargando"),
            ),
            (
                &["icon_button"],
                Buttons,
                None,
                By::test_id("buttons.icono"),
            ),
            (
                &["icon_button_sized"],
                Buttons,
                None,
                By::role(Role::Button).name("Usuario"),
            ),
            (
                &["switch"],
                Selection,
                None,
                By::role(Role::Switch).name("Wi-Fi"),
            ),
            (
                &["switch_with_options"],
                Selection,
                None,
                By::test_id("selection.bluetooth"),
            ),
            (
                &["checkbox_with_options"],
                Selection,
                None,
                By::test_id("selection.all"),
            ),
            (
                &["select_with_options"],
                Selection,
                None,
                By::test_id("selection.country"),
            ),
            (
                &["select_menu"],
                Selection,
                Some("selection.country"),
                By::role(Role::Menu),
            ),
            (
                &["segmented_with_options"],
                Selection,
                None,
                By::test_id("selection.alineacion"),
            ),
            (
                &["card", "card_header", "card_content", "card_actions"],
                Containers,
                None,
                By::test_id("containers.card"),
            ),
            (
                &["group_box"],
                Containers,
                None,
                By::test_id("containers.grupo"),
            ),
            (
                &["heading"],
                Containers,
                None,
                By::role(Role::Heading).name("Título de sección"),
            ),
            (&["paragraph"], Containers, None, By::role(Role::Paragraph)),
            (
                &["caption"],
                Containers,
                None,
                By::text("Una leyenda, para notas y metadatos."),
            ),
            (&["Tabs"], Containers, None, By::role(Role::TabList)),
            (&["badge"], Badges, None, By::text("Warning")),
            (
                &["Avatar"],
                Multimedia,
                None,
                By::role(Role::Image).name("Ana Rojas"),
            ),
            (
                &["avatar"],
                Multimedia,
                None,
                By::test_id("media.avatar_url"),
            ),
            (
                &["MediaSlot"],
                Multimedia,
                None,
                By::test_id("media.cover.preview"),
            ),
            (
                &["alert"],
                Badges,
                None,
                By::text("Revisa los campos requeridos"),
            ),
            (
                &["Table"],
                Table,
                None,
                By::role(Role::Table).name("Clientes"),
            ),
            (
                &["virtual_list"],
                Table,
                None,
                By::role(Role::List).name("Elementos"),
            ),
            (
                &["column_menu"],
                Table,
                None,
                By::role(Role::Button).name("Columnas"),
            ),
            (
                &["map"],
                Map,
                None,
                By::role(Role::Group).name("Mapa de Parral"),
            ),
            (
                &["marker"],
                Map,
                None,
                By::role(Role::Image).name("Plaza de Armas"),
            ),
            (
                &["MapLayer", "polygons"],
                Map,
                None,
                By::role(Role::List).name("Sectores"),
            ),
            (
                &["color_field", "ColorField"],
                Map,
                Some("map.sector.AOP26000002"),
                By::role(Role::Group).name("Color del sector"),
            ),
            (
                &["chart"],
                Charts,
                None,
                By::role(Role::Image).name("Clics por mes"),
            ),
            (
                &["document"],
                Document,
                None,
                By::role(Role::TextArea).name("Informe de demostración"),
            ),
            (
                &["Dialog", "dialog"],
                Dialogs,
                Some("dialogs.save"),
                By::role(Role::Dialog).name("¿Guardar los cambios?"),
            ),
            (
                &["toast_stack"],
                Dialogs,
                Some("toasts.info"),
                By::test_id("kai.toast.1"),
            ),
            (
                &["command_palette"],
                Dialogs,
                Some("dialogs.palette"),
                By::test_id("kai.palette"),
            ),
            (
                &["MasterDetailPage", "PageHeader", "breadcrumb_trail"],
                TemplateList,
                None,
                By::test_id("template.items"),
            ),
            (
                &["FormPage", "form_field_cell", "form_full_cell"],
                TemplateForm,
                None,
                By::test_id("template.form"),
            ),
            (
                &["DataPage"],
                TemplateData,
                None,
                By::test_id("template.data"),
            ),
            (&["EmptyState"], Catalog, None, By::test_id("catalog.empty")),
            (
                &["loading_placeholder"],
                Table,
                None,
                By::test_id("patrones.loading"),
            ),
        ];
        let mut covered = BTreeSet::new();
        for (names, section, open, by) in shown {
            let mut app = TestApp::new(Demo::at(section));
            if let Some(id) = open {
                app.click(By::test_id(id));
            }
            assert!(
                !app.screen().all(by).is_empty(),
                "{names:?} no aparece en {section:?}"
            );
            covered.extend(names.iter().copied());
        }
        let mut editor = TestApp::new(Demo::at(Map));
        edit_catillo(&mut editor);
        assert!(!editor.screen().all(vertex(0)).is_empty());
        covered.insert("PolygonDraft");

        let public: BTreeSet<&str> = SOURCES
            .iter()
            .flat_map(|source| source.lines())
            .filter_map(|line| {
                line.strip_prefix("pub fn ")
                    .or_else(|| line.strip_prefix("pub struct "))
            })
            .filter_map(|rest| {
                rest.split(|c: char| !(c.is_alphanumeric() || c == '_'))
                    .next()
            })
            .collect();
        let accounted: BTreeSet<&str> = covered.into_iter().chain(NOT_WIDGETS).collect();
        assert_eq!(public, accounted);
    }

    /// An 8×8 PNG of one color.
    fn png(color: [u8; 4]) -> Vec<u8> {
        let mut bytes = Vec::new();
        image::RgbaImage::from_pixel(8, 8, image::Rgba(color))
            .write_to(
                &mut std::io::Cursor::new(&mut bytes),
                image::ImageFormat::Png,
            )
            .unwrap();
        bytes
    }

    #[test]
    fn image_fields_take_picked_and_dropped_files_and_refuse_others() {
        let dir = std::env::temp_dir().join(format!("kai-demo-media-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let photo = dir.join("portada.png");
        std::fs::write(&photo, png([200, 40, 40, 255])).unwrap();
        let mut app = TestApp::new(Demo::at(Section::Multimedia));
        let preview = |id: &str| By::test_id(format!("media.{id}.preview"));
        assert_eq!(
            app.screen().get(preview("cover")).name().as_deref(),
            Some("Agregar imagen")
        );

        app.click(preview("cover"));
        let request = app.file_pick_request().expect("the panel is open");
        assert_eq!(request.title, "Elegir portada");
        app.on_file_pick(Some(photo.clone()));
        app.run_until_idle();
        assert_eq!(
            app.app().cover.actual,
            Some(ImageSource::Path(photo.clone()))
        );
        assert_eq!(
            app.screen().get(preview("cover")).name().as_deref(),
            Some("Cambiar imagen")
        );
        assert!(
            app.messages()
                .contains(&Msg::ImagePicked(ImageField::Cover, Some(photo)))
        );

        let before = app.app().photo.clone();
        app.on_file_pick(None);
        app.click(preview("photo"));
        assert_eq!(app.app().photo, before, "cancelled");

        app.drop_files(preview("logo"), [dir.join("nota.txt")]);
        assert_eq!(
            app.app().logo.error.as_deref(),
            Some("Tipo no permitido: png, jpg, jpeg, webp")
        );
        assert!(app.app().logo.actual.is_some(), "the logo stays");
        app.screen().get(By::test_id("media.logo.error"));
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn load_url_puts_a_downloaded_image_in_the_avatar() {
        const URL: &str = "https://kai.test/ana.png";
        let mut app = TestApp::builder(Demo::at(Section::Multimedia))
            .images(FakeImageLoader::new().with(URL, png([20, 120, 220, 255])))
            .build();
        let image = By::test_id("media.avatar_url.image");
        assert!(app.screen().query(image.clone()).is_none(), "the icon");

        let result = app.call("media.load_url", json!({"url": URL}));
        assert!(result.is_ok(), "{result:?}");
        app.run_until_idle();
        assert_eq!(app.app().avatar_url.as_deref(), Some(URL));
        app.screen().get(image.clone());
        assert_eq!(app.app().url, URL, "the field shows it");

        app.type_text(By::test_id("media.url"), " ");
        app.click(By::test_id("media.load"));
        assert_eq!(app.app().avatar_url.as_deref(), Some(URL), "trimmed");
        app.invoke(LoadUrl { url: String::new() }).unwrap();
        assert!(app.screen().query(image).is_none(), "back to the icon");
    }

    #[test]
    fn the_table_shows_an_avatar_by_each_name() {
        let app = TestApp::new(Demo::at(Section::Table));
        let screen = app.screen();
        let row = screen.within(screen.get_by_test_id("table.clients.row.0"));
        let name = super::client(1).name;
        row.get(By::role(Role::Image).name(&name));
        let names: Vec<_> = row
            .all(By::role(Role::Cell))
            .iter()
            .filter_map(|cell| cell.name())
            .collect();
        assert_eq!(
            names[2], name,
            "after the chevron and N.º; the avatar stays out of the cell's name"
        );
    }

    #[test]
    fn the_toolbar_search_filters_and_the_headers_sort_the_table() {
        let mut app = TestApp::new(Demo::default());
        let table = By::test_id("table.clients");
        let count = |app: &TestApp<Demo>| app.screen().get(table.clone()).node().row_count;
        let cells = |app: &TestApp<Demo>, row: usize| -> Vec<String> {
            let screen = app.screen();
            screen
                .within(screen.get_by_test_id(&format!("table.clients.row.{row}")))
                .all(By::role(Role::Cell))
                .iter()
                .filter_map(|cell| cell.name())
                .collect()
        };
        // Return in the search goes to the table.
        app.type_text(By::test_id("bar.find"), "temuco");
        app.press(Key::Enter, Modifiers::NONE);
        assert_eq!(app.app().section, Section::Table);
        let temuco = (1..=super::CLIENT_COUNT)
            .map(super::client)
            .filter(|c| {
                [c.name.as_str(), c.city, c.plan]
                    .iter()
                    .any(|text| text.to_lowercase().contains("temuco"))
            })
            .count();
        assert!(temuco > 0 && temuco < super::CLIENT_COUNT as usize);
        assert_eq!(count(&app), Some(temuco));
        assert_eq!(cells(&app, 0)[3], "Temuco");
        assert!(
            app.screen()
                .get_by_test_id("table.total")
                .name()
                .unwrap()
                .contains("de 10000 con «temuco»")
        );

        // A sortable header sorts, and again reverses.
        let amount_header = || By::role(Role::ColumnHeader).name("Monto");
        app.click(amount_header());
        assert_eq!(
            app.app().table_view.sort,
            Some((4, SortDirection::Ascending))
        );
        let amount = |row: Vec<String>| -> u32 { row[5].trim_start_matches("$ ").parse().unwrap() };
        assert!(amount(cells(&app, 0)) <= amount(cells(&app, 1)));
        app.click(amount_header());
        assert_eq!(
            app.app().table_view.sort,
            Some((4, SortDirection::Descending))
        );
        assert!(amount(cells(&app, 0)) >= amount(cells(&app, 1)));

        // A click selects a client, which keeps its place through sorting.
        app.click(By::test_id("table.clients.row.1"));
        let chosen = app.app().table_view.rows()[1];
        assert_eq!(app.app().client, Some(chosen));
        app.click(amount_header());
        assert_eq!(app.app().client, Some(chosen));
        let name = app.app().clients[chosen].name.clone();
        assert_eq!(
            app.screen().get_by_test_id("table.selection").name(),
            Some(format!("Seleccionado: {name}"))
        );

        // Clearing the search shows the ten thousand again.
        app.click(By::test_id("bar.find.clear"));
        assert_eq!(count(&app), Some(10_000));
    }

    #[test]
    fn exporting_saves_the_rows_the_table_shows_where_the_person_says() {
        let mut app = TestApp::new(Demo::at(Section::Table));
        app.type_text(By::test_id("bar.find"), "temuco");
        app.click(By::role(Role::ColumnHeader).name("Monto"));
        app.click(By::role(Role::Button).name("Exportar"));
        let request = app.file_save_request().cloned().expect("the save panel");
        assert_eq!(request.name, "clientes.xlsx");
        assert_eq!(request.types, ["xlsx"]);

        let dir = std::env::temp_dir().join(format!("kai-demo-export-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("clientes.xlsx");
        app.on_file_save(Some(path.clone()));
        app.run_until_idle();
        assert!(!app.app().exporting);
        assert!(std::fs::read(&path).unwrap().starts_with(b"PK"));
        let rows = app.app().table_view.len();
        assert!(rows > 0 && rows < 10_000);
        app.screen().get(By::text("Excel exportado"));
        app.screen()
            .get(By::text(format!("{rows} filas en clientes.xlsx")));

        // The command does the same, from the menu, the palette or an agent.
        std::fs::remove_file(&path).unwrap();
        assert!(app.perform("table.export"));
        app.run_until_idle();
        assert!(path.exists());
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn the_columns_button_hides_a_column_and_the_search_still_looks_in_it() {
        let mut app = TestApp::new(Demo::at(Section::Table));
        let table = By::test_id("table.clients");
        app.click(By::role(Role::Button).name("Columnas"));
        let items: Vec<_> = app
            .screen()
            .all(By::role(Role::MenuItemCheckBox))
            .iter()
            .filter_map(|item| item.name())
            .collect();
        assert_eq!(items, ["Ciudad", "Plan", "Monto"], "N.º and Nombre stay");
        app.click(By::role(Role::MenuItemCheckBox).name("Ciudad"));
        app.press(Key::Escape, Modifiers::NONE);
        assert!(app.is_focused(By::role(Role::Button).name("Columnas")));
        let screen = app.screen();
        let headers: Vec<_> = screen
            .within(screen.get(table.clone()))
            .all(By::role(Role::ColumnHeader))
            .iter()
            .filter_map(|header| header.name())
            .collect();
        assert_eq!(
            headers,
            ["Detalle", "N.º", "Nombre", "Plan", "Monto", "Acciones"]
        );

        app.type_text(By::test_id("bar.find"), "temuco");
        let count = app.screen().get(table).node().row_count.unwrap();
        assert!(
            count > 0 && count < 10_000,
            "Temuco is in the hidden column"
        );
    }

    #[test]
    fn a_client_row_opens_its_detail_and_the_export_leaves_it_out() {
        let mut app = TestApp::new(Demo::at(Section::Table));
        app.click(By::test_id("table.clients.row.0"));
        app.press(Key::ArrowRight, Modifiers::NONE);
        let client = super::client(1);
        let detail = app.screen().get(By::role(Role::Region));
        assert!(
            detail
                .name()
                .unwrap()
                .starts_with(&format!("1 {}", client.name))
        );
        let text = By::text(format!("Cliente N.º 1 de {}", client.city));
        assert_eq!(app.screen().all(text.clone()).len(), 1);

        app.press(Key::Tab, Modifiers::NONE);
        assert!(app.is_focused(By::test_id("table.card.1")));
        app.press(Key::Space, Modifiers::NONE);
        assert_eq!(app.screen().all(text).len(), 2, "and the toast");
        let sheet = app
            .app()
            .table_view
            .export(&app.app().clients, &super::table_columns());
        assert_eq!(
            sheet.columns,
            ["N.º", "Nombre", "Ciudad", "Plan", "Monto"],
            "no chevron, panel or actions"
        );
    }

    #[test]
    fn the_filter_button_shows_a_field_per_column_and_typing_waits_for_a_pause() {
        let mut app = TestApp::new(Demo::at(Section::Table));
        let table = By::test_id("table.clients");
        let count = |app: &TestApp<Demo>| app.screen().get(table.clone()).node().row_count;
        let filter_rows = By::role(Role::Button).name("Filtrar");
        assert!(
            app.screen()
                .query(By::role(Role::SearchInput).name("Filtrar Ciudad"))
                .is_none()
        );

        app.click(filter_rows.clone());
        assert_eq!(
            app.screen().get(filter_rows.clone()).node().control_state,
            Some(ControlState::On)
        );
        let screen = app.screen();
        let fields: Vec<_> = screen
            .within(screen.get(table.clone()))
            .all(By::role(Role::SearchInput))
            .iter()
            .filter_map(|field| field.name())
            .collect();
        assert_eq!(
            fields,
            [
                "Filtrar Nombre",
                "Filtrar Ciudad",
                "Filtrar Plan",
                "Filtrar Monto"
            ],
            "N.º is not filterable"
        );

        app.type_text(By::test_id("table.clients.filter.2"), "temuco");
        assert_eq!(count(&app), Some(10_000), "the table waits for a pause");
        app.advance(super::FILTER_PAUSE);
        app.type_text(By::test_id("table.clients.filter.3"), "pro");
        app.advance(super::FILTER_PAUSE);
        let expected = app
            .app()
            .clients
            .iter()
            .filter(|c| c.city == "Temuco" && c.plan == "Pro")
            .count();
        assert!(expected > 0);
        assert_eq!(count(&app), Some(expected));
        assert!(
            app.screen()
                .get_by_test_id("table.total")
                .name()
                .unwrap()
                .contains("filtrados")
        );

        // Turning the fields off clears the filters.
        app.click(filter_rows);
        assert!(!app.app().table_view.filtering);
        assert!(app.app().table_view.filters.is_empty());
        assert_eq!(count(&app), Some(10_000));
    }

    #[test]
    fn dialogs_trap_focus_and_confirming_shows_a_toast() {
        let mut app = TestApp::new(Demo::at(Section::Dialogs));
        let dialog = || By::role(Role::Dialog);
        app.click(By::test_id("dialogs.save"));
        assert_eq!(
            app.screen().get(dialog()).name().as_deref(),
            Some("¿Guardar los cambios?")
        );
        assert!(app.is_focused(By::test_id("dialog.cancel")));
        for _ in 0..3 {
            app.press(Key::Tab, Modifiers::NONE);
            let focused = app.session().focused().unwrap();
            let screen = app.screen();
            let inside = screen
                .within(screen.get(dialog()))
                .all(By::role(Role::Button));
            assert!(inside.iter().any(|button| button.id() == focused));
        }
        app.press(Key::Escape, Modifiers::NONE);
        assert!(app.screen().query(dialog()).is_none());
        assert!(app.is_focused(By::test_id("dialogs.save")));
        assert!(app.app().toasts.is_empty(), "cancel shows nothing");

        app.click(By::test_id("dialogs.delete"));
        let default = By::test_id("dialog.default");
        assert_eq!(
            app.screen().get(default.clone()).name().as_deref(),
            Some("Eliminar")
        );
        app.click(default);
        assert!(app.screen().query(dialog()).is_none());
        let screen = app.screen();
        let toast = screen.get_by_test_id("kai.toast.1");
        assert_eq!(toast.role(), Role::Status);
        screen.within(toast).get(By::text("Borrador eliminado"));
    }

    #[test]
    fn toasts_close_on_their_own_or_with_their_button() {
        let mut app = TestApp::new(Demo::at(Section::Dialogs));
        app.click(By::test_id("toasts.success"));
        app.click(By::test_id("toasts.error"));
        assert_eq!(
            app.screen().get_by_test_id("kai.toast.1").role(),
            Role::Status
        );
        assert_eq!(
            app.screen().get_by_test_id("kai.toast.2").role(),
            Role::Alert,
            "errors are read at once"
        );
        app.screen().get_by_test_id("kai.toast.2.action");
        app.advance(crate::components::TOAST_DURATION);
        assert!(app.screen().query(By::test_id("kai.toast.1")).is_none());
        assert!(
            app.screen().query(By::test_id("kai.toast.2")).is_some(),
            "an error stays until closed"
        );
        app.click(By::test_id("kai.toast.2.close"));
        assert!(app.app().toasts.is_empty());

        assert_eq!(
            app.call("toasts.show", json!({"kind": "warning"})),
            Ok(json!(null))
        );
        assert_eq!(
            app.app().toasts[0].variant,
            crate::components::AlertVariant::Warning
        );
        assert!(app.call("toasts.show", json!({"kind": "red"})).is_err());
        app.click(By::test_id("kai.toast.3.close"));
        assert!(app.app().toasts.is_empty());
        assert_eq!(app.session().next_deadline(), None, "its timer is gone too");
    }

    #[test]
    fn the_all_checkbox_is_mixed_and_sets_every_permission() {
        let mut app = TestApp::new(Demo::at(Section::Selection));
        let all_items = By::test_id("selection.all");
        let state = |app: &TestApp<Demo>, by: By| app.screen().get(by).node().control_state;
        assert_eq!(state(&app, all_items.clone()), Some(ControlState::Mixed));
        app.click(all_items.clone());
        assert_eq!(app.app().permissions, [true; 3]);
        assert_eq!(state(&app, all_items.clone()), Some(ControlState::On));
        app.click(all_items.clone());
        assert_eq!(app.app().permissions, [false; 3]);
        app.click(By::test_id("selection.permiso.1"));
        assert_eq!(state(&app, all_items.clone()), Some(ControlState::Mixed));
        assert!(!app.is_enabled(By::test_id("selection.administrar")));
        assert!(!app.is_enabled(By::test_id("selection.bluetooth")));

        app.click(By::test_id("selection.country"));
        app.click(By::role(Role::MenuItem).name("Perú"));
        assert_eq!(app.app().country, Some(2));
        app.click(By::test_id("selection.alineacion.2"));
        assert_eq!(app.app().alignment, 2);
        app.click(By::test_id("selection.wifi"));
        assert!(!app.app().wifi);
    }

    #[test]
    fn scripts_and_agents_change_the_section_with_ui_go_to() {
        let mut app = TestApp::new(Demo::default());
        for section in Section::ALL.into_iter().rev() {
            assert_eq!(
                app.call("ui.go_to", json!({"section": section.key()})),
                Ok(json!(null))
            );
            assert_eq!(app.app().section, section);
            app.screen().get(By::test_id(landmark(section)));
        }
        app.call("ui.go_to", json!({"section": "buttons"})).unwrap();
        assert_eq!(
            app.screen()
                .get(sidebar_row("Botones"))
                .node()
                .control_state,
            Some(ControlState::On)
        );
        assert!(app.call("ui.go_to", json!({"section": "maps"})).is_err());
        assert!(app.call("ui.go_to", json!({})).is_err());
        assert_eq!(app.app().section, Section::Buttons);
    }

    /// ⌃⌘S, the toolbar button and `ui.alternar_sidebar` collapse it to
    /// the rail and back; a window narrower than `sm` starts with it
    /// collapsed.
    #[test]
    fn the_sidebar_collapses_to_a_rail_by_shortcut_button_or_command() {
        let mut app = TestApp::new(Demo::default());
        let width = |app: &TestApp<Demo>| app.screen().get_by_test_id("nav").bounds().width;
        assert!(width(&app) > 48.0);
        let ctrl_cmd = Modifiers {
            control: true,
            command: true,
            ..Modifiers::NONE
        };
        app.press(Key::Char('s'), ctrl_cmd);
        assert_eq!(app.messages(), [Msg::ToggleSidebar]);
        assert_eq!(width(&app), 48.0);
        assert_eq!(
            app.screen()
                .get(By::role(Role::SidebarItem).name("Campos"))
                .role(),
            Role::SidebarItem,
            "rows keep their names in the rail"
        );
        let toggle = By::test_id("bar.sidebar");
        assert_eq!(
            app.screen().get(toggle.clone()).name().as_deref(),
            Some("Mostrar barra lateral")
        );
        app.click(toggle.clone());
        assert!(width(&app) > 48.0);
        assert_eq!(
            app.screen().get(toggle).name().as_deref(),
            Some("Ocultar barra lateral")
        );
        assert_eq!(app.call("ui.toggle_sidebar", json!({})), Ok(json!(null)));
        assert!(app.app().sidebar_collapsed);

        let narrow = TestApp::builder(Demo::default())
            .viewport(560.0, 700.0)
            .build();
        assert!(narrow.app().sidebar_collapsed);
        assert_eq!(width(&narrow), 48.0);
        let wide = TestApp::builder(Demo::default())
            .viewport(1024.0, 700.0)
            .build();
        assert!(!wide.app().sidebar_collapsed);
    }

    /// `kai dev` restarts keep the section, the sidebar and the selected
    /// sector, even over a window width that would collapse the sidebar.
    #[test]
    fn dev_restarts_keep_section_sidebar_and_sector() {
        let mut before = Demo::at(Section::Map);
        before.select_area("AOP26000002");
        let state = before.dev_state().unwrap();

        let mut after = Demo::default();
        after.restore_dev_state(state);
        let app = TestApp::builder(after).viewport(560.0, 700.0).build();
        assert_eq!(app.app().section, Section::Map);
        assert!(!app.app().sidebar_collapsed);
        assert_eq!(app.app().area.as_deref(), Some("AOP26000002"));
        assert!(app.screen().query(By::test_id("map")).is_some());

        let mut older = Demo::default();
        older.restore_dev_state(json!({ "section": "table", "other_thing": 1 }));
        assert_eq!(older.section, Section::Table);
        assert!(!older.sized, "nothing said about the sidebar");
        let mut broken = Demo::default();
        broken.restore_dev_state(json!({ "section": "missing" }));
        assert_eq!(broken, Demo::default());
    }

    /// The content is keyed by section, so a section never inherits the
    /// scroll of the one before it.
    #[test]
    fn each_section_opens_at_the_top() {
        let short = |demo| TestApp::builder(demo).viewport(1024.0, 400.0).build();
        let password =
            |app: &TestApp<Demo>| app.screen().get(By::test_id("form.password")).bounds().y;
        let top = |app: &TestApp<Demo>| app.screen().get(By::test_id("form.clear")).bounds().y;
        let fields_list = password(&short(Demo::at(Section::Fields)));
        let mut app = short(Demo::default());
        let at_rest = top(&app);
        for _ in 0..20 {
            app.press(Key::Tab, Modifiers::NONE);
            if app.is_focused(By::test_id("form.clear")) {
                break;
            }
        }
        let scrolled = top(&app);
        assert!(scrolled < at_rest, "focusing «Limpiar» scrolls to it");
        app.click(sidebar_row("Campos"));
        assert_eq!(password(&app), fields_list);
        app.click(sidebar_row("Inicio"));
        assert_eq!(top(&app), at_rest);
        // Also after scrolling a long section with the wheel.
        app.click(sidebar_row("Tabla"));
        let table = |app: &TestApp<Demo>| app.screen().get_by_test_id("table").bounds().y;
        let fresh = table(&app);
        // Over the caption, outside the table's own scroll.
        app.scroll_wheel(By::test_id("table.total"), -200.0);
        assert!(table(&app) < fresh);
        app.click(sidebar_row("Badges y alertas"));
        app.click(sidebar_row("Tabla"));
        assert_eq!(table(&app), fresh);
    }

    #[test]
    fn return_in_the_name_field_submits_through_the_default_button() {
        let mut app = TestApp::new(Demo::default());
        app.type_text(By::test_id("form.name"), "Ana");
        app.press(Key::Enter, Modifiers::NONE);
        assert_eq!(app.messages().last(), Some(&Msg::Submit));
        assert_eq!(app.app().submissions, ["Ana"]);
    }

    #[test]
    fn tab_moves_through_the_form_in_order_showing_the_ring() {
        let mut app = TestApp::new(Demo::default());
        app.click(By::test_id("form.name"));
        assert!(app.is_focused(By::test_id("form.name")));
        assert!(!app.focus_visible(), "a click does not show the ring");

        app.press(Key::Tab, Modifiers::NONE);
        assert!(app.is_focused(By::test_id("form.email")));
        assert!(app.focus_visible());

        let shift = Modifiers {
            shift: true,
            ..Modifiers::NONE
        };
        app.press(Key::Tab, shift);
        assert!(app.is_focused(By::test_id("form.name")));
    }

    #[test]
    fn the_password_eye_toggles_and_leaves_focus_in_the_field() {
        let mut app = TestApp::new(Demo::at(Section::Fields));
        app.type_text(By::test_id("form.password"), "abc");
        assert!(!app.app().show_password);
        app.click(By::test_id("form.password.toggle"));
        assert!(app.app().show_password);
        assert!(app.is_focused(By::test_id("form.password")));
        assert_eq!(app.value(By::test_id("form.password")), "abc");
    }

    #[cfg(feature = "document")]
    const CMD: Modifiers = Modifiers {
        command: true,
        ..Modifiers::NONE
    };

    const CMD_SHIFT: Modifiers = Modifiers {
        command: true,
        shift: true,
        ..Modifiers::NONE
    };

    /// Frames of the «Mapa» section while a drag pans Parral at zoom 15:
    /// view, layout with real text, tiles, labels and paint. Under 4 ms per
    /// frame in release. Run with
    /// `cargo test --release --lib -- --ignored --nocapture bench_map_pan`.
    #[test]
    #[ignore = "benchmark; run explicitly in release mode"]
    fn bench_map_pan() {
        use crate::geometry::Point;
        use crate::session::{Measure, Session};
        use crate::testing::MemoryClipboard;
        use std::time::{Duration, Instant};

        let mut demo = Demo::at(Section::Map);
        demo.view = MapView::new(MAIN_SQUARE, 15.0);
        let mut session = Session::new(
            demo,
            Theme::kai(Density::Comfortable),
            Size {
                width: 1024.0,
                height: 720.0,
            },
            Measure::Real,
            Box::new(MemoryClipboard::default()),
        );
        session.prepare_frame();
        let limit = Instant::now() + Duration::from_secs(20);
        while session.tasks_pending() {
            assert!(Instant::now() < limit, "las teselas no cargaron");
            session.wait_for_tasks(Duration::from_millis(50));
            session.prepare_frame();
        }
        let map = session
            .tree()
            .find_test_id("map.view")
            .map(|map| map.id)
            .unwrap();
        let bounds = session.layout().bounds(map).unwrap();
        let start = Point {
            x: bounds.x + bounds.width / 2.0,
            y: bounds.y + bounds.height / 2.0,
        };
        let spec = |session: &Session<Demo>| match &session.layout().node(map).unwrap().content {
            crate::layout::Content::Map(spec) => spec.clone(),
            other => panic!("{other:?}"),
        };
        let before = spec(&session);
        assert!(!before.tiles.tiles.is_empty() && !before.labels.is_empty());
        session.pointer_moved(Some(start));
        session.pointer_clicks(true, 1);
        session.prepare_frame();
        let mut samples: Vec<Duration> = Vec::new();
        let mut offset = 0.0;
        for frame in 0..400 {
            offset += if (frame / 50) % 2 == 0 { -4.0 } else { 4.0 };
            let started = Instant::now();
            session.pointer_moved(Some(Point {
                x: start.x + offset,
                y: start.y + offset / 3.0,
            }));
            session.prepare_frame();
            let list = session.paint_list();
            samples.push(started.elapsed());
            assert!(!list.ops.is_empty());
            if frame == 49 {
                let during = spec(&session);
                assert_ne!(during.view, before.view, "the drag pans");
                let spots = |labels: &[crate::maps::labels::MapLabel]| -> Vec<_> {
                    labels.iter().map(|label| (label.text, label.at)).collect()
                };
                assert_eq!(
                    spots(&during.labels),
                    spots(&before.labels),
                    "labels ride along"
                );
            }
        }
        session.pointer_button(false);
        samples.sort();
        let p50 = samples[samples.len() / 2];
        let p95 = samples[samples.len() * 95 / 100];
        println!("pan del mapa: p50 {p50:?}, p95 {p95:?} por frame");
        if !cfg!(debug_assertions) {
            assert!(p95 < Duration::from_millis(4), "p95 {p95:?}");
        }
    }

    #[test]
    fn the_map_follows_gestures_and_agents() {
        let mut app = TestApp::new(Demo::at(Section::Map));
        let view = By::test_id("map.view");
        app.drag_and_hold(view.clone(), -100.0, 0.0);
        let [Msg::MapMoved(moved)] = app.messages() else {
            panic!("{:?}", app.messages());
        };
        assert!(moved.center.lon > MAIN_SQUARE.lon, "{moved:?}");
        assert_eq!(app.app().view, *moved);
        assert_eq!(
            app.call("map.get_view", json!({})),
            Ok(serde_json::to_value(app.app().view).unwrap())
        );

        let terminal = MapView::new(LatLon::new(-36.155, -71.82), 15.0);
        let call = json!({
            "center": {"lat": -36.155, "lon": -71.82},
            "zoom": 15,
            "animate": false,
        });
        assert_eq!(app.call("map.set_view", call), Ok(json!(null)));
        assert_eq!(app.app().view, terminal);
        assert!(!app.session().animating(), "it jumps");
        app.screen()
            .get(By::role(Role::Status).name("-36.1550, -71.8200 · zoom 15.0"));
        let pin = app
            .screen()
            .get(By::role(Role::Image).name("Plaza de Armas"))
            .bounds();
        let map = app.screen().get(view.clone()).bounds();
        assert!(
            pin.y + pin.height < map.y + map.height / 2.0,
            "the plaza is north of the terminal"
        );

        app.click(By::test_id("map.volver"));
        assert!(app.session().animating(), "it glides back");
        app.advance(Duration::from_secs(1));
        assert_eq!(app.app().view, INITIAL_VIEW);
        assert!(
            !app.messages()
                .iter()
                .skip(1)
                .any(|msg| matches!(msg, Msg::MapMoved(_))),
            "the app's moves are not reported back"
        );
    }

    /// Where `at` falls in the window on the map of «Mapa».
    fn on_map(app: &TestApp<Demo>, at: LatLon) -> crate::geometry::Point {
        let id = app.screen().get_by_test_id("map.view").id();
        let area = app.session().layout().node(id).unwrap().padding_box;
        let size = Size {
            width: area.width,
            height: area.height,
        };
        let local = crate::geo::Camera::new(app.app().view, size).to_local(at);
        crate::geometry::Point {
            x: area.x + local.x,
            y: area.y + local.y,
        }
    }

    #[test]
    fn sectors_are_chosen_from_the_list_the_map_and_agents() {
        use crate::style::Cursor;
        let mut app = TestApp::new(Demo::at(Section::Map));
        let screen = app.screen();
        let list_view = screen.get(By::role(Role::List).name("Sectores"));
        let names: Vec<String> = screen
            .within(list_view)
            .all(By::role(Role::Button))
            .iter()
            .filter_map(|row| row.name())
            .collect();
        assert_eq!(
            names,
            [
                "Plaza de Armas",
                "Catillo",
                "Villa San Pedro",
                "Palma Rosa",
                "Terminal de Buses"
            ]
        );

        let catillo_ring = initial_sector("AOP26000002").unwrap();
        app.click(By::test_id("map.sector.AOP26000002"));
        assert_eq!(app.app().area.as_deref(), Some("AOP26000002"));
        assert!(app.session().animating(), "it glides to the sector");
        app.advance(Duration::from_secs(1));
        let view = app.app().view;
        assert!(view.zoom > INITIAL_VIEW.zoom, "{view:?}");
        let middle = catillo_ring.layer.centroid();
        assert!((view.center.lat - middle.lat).abs() < 1e-3, "{view:?}");
        assert!((view.center.lon - middle.lon).abs() < 1e-3, "{view:?}");
        app.screen()
            .get(By::text("Sector Catillo · AOP26000002 · 191,7 ha"));
        assert_eq!(
            app.screen()
                .get_by_test_id("map.sector.AOP26000002")
                .node()
                .control_state,
            Some(ControlState::On)
        );

        let all_items = SECTORS
            .iter()
            .map(|sector| sector.layer.bounds())
            .reduce(LatLonBounds::union)
            .unwrap();
        assert_eq!(
            app.call("map.fit_bounds", json!({ "bounds": all_items })),
            Ok(json!(null))
        );
        app.advance(Duration::from_secs(1));
        let terminal = initial_sector("AOP26000005").unwrap().layer.centroid();
        let point_value = on_map(&app, terminal);
        app.move_pointer(point_value);
        assert_eq!(app.mouse_cursor(), Cursor::Pointer);
        app.click_point(point_value);
        assert_eq!(app.app().area.as_deref(), Some("AOP26000005"));
        assert!(
            app.messages()
                .contains(&Msg::SelectArea("AOP26000005".to_owned()))
        );

        assert_eq!(
            app.call("areas.select", json!({ "code": "AOP99" })),
            Ok(json!(null))
        );
        assert_eq!(app.app().area.as_deref(), Some("AOP26000005"), "unknown");
        assert_eq!(
            app.call("areas.select", json!({ "code": "AOP26000001" })),
            Ok(json!(null))
        );
        assert_eq!(app.app().area.as_deref(), Some("AOP26000001"));

        app.click(By::role(Role::Switch).name("Sectores"));
        assert!(!app.app().show_areas, "{:?}", app.messages().last());
        app.advance(Duration::from_secs(1));
        let square = on_map(&app, MAIN_SQUARE);
        app.move_pointer(square);
        assert_eq!(app.mouse_cursor(), Cursor::Default, "no sectors drawn");
    }

    const CATILLO_POLYGON: &str = "AOP26000002";

    fn ring_points(app: &TestApp<Demo>, code: &str) -> Vec<LatLon> {
        app.app().sector(code).unwrap().ring_points()
    }

    fn draft_area(app: &TestApp<Demo>) -> Vec<LatLon> {
        app.app().editing.as_ref().unwrap().1.ring().to_vec()
    }

    #[track_caller]
    fn assert_same_ring(actual: &[LatLon], expected: &[LatLon]) {
        assert_eq!(actual.len(), expected.len(), "{actual:?} vs {expected:?}");
        for (a, e) in actual.iter().zip(expected) {
            assert!(
                (a.lat - e.lat).abs() < 1e-9 && (a.lon - e.lon).abs() < 1e-9,
                "{actual:?} vs {expected:?}"
            );
        }
    }

    /// Select Catillo, press «Editar sector» and let the map frame it.
    fn edit_catillo(app: &mut TestApp<Demo>) {
        app.click(By::test_id(format!("map.sector.{CATILLO_POLYGON}")));
        app.click(By::test_id("map.edit"));
        app.advance(Duration::from_secs(1));
        assert!(app.app().editing.is_some(), "{:?}", app.messages().last());
    }

    fn vertex(n: usize) -> By {
        By::test_id(format!("map.view.vertex.{n}"))
    }

    /// The selected sector takes a color from the palette or typed; while
    /// the hex is incomplete, and with «Sin color», it shows its type's.
    #[test]
    fn the_selected_sector_changes_color_with_the_color_field() {
        let mut app = TestApp::new(Demo::at(Section::Map));
        let image_field = By::role(Role::Group).name("Color del sector");
        assert!(
            app.screen().query(image_field.clone()).is_none(),
            "no sector yet"
        );
        app.click(By::test_id(format!("map.sector.{CATILLO_POLYGON}")));
        let catillo_ring = |app: &TestApp<Demo>| app.app().sector(CATILLO_POLYGON).unwrap().clone();
        let for_kind = catillo_ring(&app).color();
        assert_eq!(app.value(By::test_id("map.color.hex")), for_kind.to_hex());

        app.click(By::role(Role::Radio).name("Violeta"));
        assert_eq!(catillo_ring(&app).color, "#7c3aed");
        assert_eq!(catillo_ring(&app).swatch(), "bg-[#7c3aed]");

        app.click(By::test_id("map.color.clear"));
        app.type_text(By::test_id("map.color.hex"), "#12");
        assert_eq!(catillo_ring(&app).color(), for_kind);
        app.screen().get(By::test_id("map.color.error"));
        app.type_text(By::test_id("map.color.hex"), "3456");
        assert_eq!(catillo_ring(&app).color(), super::Color::hex(0x123456));

        app.click(By::test_id("map.color.clear"));
        assert_eq!(catillo_ring(&app).color(), for_kind);
        app.click(By::test_id("map.edit"));
        assert!(
            app.screen().query(image_field).is_none(),
            "hidden while the outline is edited"
        );
    }

    #[test]
    fn a_sector_is_edited_on_the_map_and_kept_only_on_finish() {
        let mut app = TestApp::new(Demo::at(Section::Map));
        let edit_sector = By::test_id("map.edit");
        assert!(
            app.screen().get(edit_sector.clone()).node().disabled,
            "nothing selected"
        );
        edit_catillo(&mut app);
        assert!(
            app.screen().get(edit_sector.clone()).node().disabled,
            "editing"
        );
        let before_change = ring_points(&app, CATILLO_POLYGON);
        let view = app.app().view;
        for (n, at) in before_change.iter().enumerate() {
            let handle = app.screen().get(vertex(n)).bounds();
            let point_value = on_map(&app, *at);
            assert!(
                (handle.x + handle.width / 2.0 - point_value.x).abs() < 0.5
                    && (handle.y + handle.height / 2.0 - point_value.y).abs() < 0.5,
                "vertex {n}: {handle:?} vs {point_value:?}"
            );
        }
        let surface_area = |app: &TestApp<Demo>| {
            app.screen()
                .get_by_test_id("map.superficie")
                .name()
                .unwrap()
        };
        assert_eq!(surface_area(&app), "Superficie 191,7 ha");

        app.drag_and_hold(vertex(0), -30.0, 20.0);
        assert_eq!(app.app().view, view, "the map stays put");
        let moved_view = draft_area(&app);
        assert_ne!(moved_view[0], before_change[0]);
        assert_eq!(moved_view[1..], before_change[1..]);
        assert_eq!(
            ring_points(&app, CATILLO_POLYGON),
            before_change,
            "the sector waits for «Terminar»"
        );
        assert_ne!(surface_area(&app), "Superficie 191,7 ha");

        app.click(By::test_id("map.sector.AOP26000001"));
        assert_eq!(
            app.app().area.as_deref(),
            Some(CATILLO_POLYGON),
            "no switching mid-edit"
        );

        app.click(By::test_id("map.view.midpoint.0"));
        assert_eq!(draft_area(&app).len(), 5);
        app.click(By::test_id("map.finish"));
        assert!(app.app().editing.is_none());
        let saved_one = ring_points(&app, CATILLO_POLYGON);
        assert_eq!(saved_one.len(), 5);
        assert_same_ring(&[saved_one[0]], &[moved_view[0]]);
        assert!(!app.screen().get(edit_sector.clone()).node().disabled);

        edit_catillo(&mut app);
        app.drag_and_hold(vertex(1), 15.0, 0.0);
        app.press(Key::Escape, Modifiers::NONE);
        assert!(app.app().editing.is_none(), "Esc cancels");
        assert_same_ring(&ring_points(&app, CATILLO_POLYGON), &saved_one);

        edit_catillo(&mut app);
        app.drag_and_hold(vertex(1), 15.0, 0.0);
        app.press(
            Key::Char('z'),
            Modifiers {
                command: true,
                ..Modifiers::NONE
            },
        );
        assert_same_ring(&draft_area(&app), &saved_one);
        app.click(By::test_id("map.cancel"));
        assert!(app.app().editing.is_none());
        assert_same_ring(&ring_points(&app, CATILLO_POLYGON), &saved_one);
    }

    #[test]
    fn crossing_edges_block_finish_until_fixed() {
        let mut app = TestApp::new(Demo::at(Section::Map));
        edit_catillo(&mut app);
        let before_change = ring_points(&app, CATILLO_POLYGON);
        let finish_edit = By::test_id("map.finish");
        assert!(!app.screen().get(finish_edit.clone()).node().disabled);
        assert!(app.screen().all(By::test_id("map.cruce")).is_empty());

        let (south_west, south_east, north_east) = (
            on_map(&app, before_change[0]),
            on_map(&app, before_change[1]),
            on_map(&app, before_change[2]),
        );
        let to_the_east = (
            south_east.x - south_west.x + 30.0,
            (north_east.y - south_west.y) / 2.0,
        );
        app.drag_and_hold(vertex(0), to_the_east.0, to_the_east.1);
        assert!(draft_area(&app)[0] != before_change[0]);
        assert!(app.screen().get(finish_edit.clone()).node().disabled);
        assert_eq!(
            app.screen().get_by_test_id("map.cruce").name().as_deref(),
            Some("Los bordes se cruzan")
        );
        app.press(Key::Enter, Modifiers::NONE);
        assert!(
            app.app().editing.is_some(),
            "Return does not take a crossed draft"
        );

        app.press(
            Key::Char('z'),
            Modifiers {
                command: true,
                ..Modifiers::NONE
            },
        );
        assert!(!app.screen().get(finish_edit).node().disabled);
        app.press(Key::Enter, Modifiers::NONE);
        assert!(app.app().editing.is_none(), "Return finishes from a vertex");
        assert_same_ring(&ring_points(&app, CATILLO_POLYGON), &before_change);
    }

    #[test]
    fn agents_replace_a_sector_outline_with_a_valid_ring() {
        let mut app = TestApp::new(Demo::at(Section::Map));
        let before_change = ring_points(&app, CATILLO_POLYGON);
        let error = |result: Result<serde_json::Value, CommandError>| {
            result.expect_err("debería fallar").message
        };
        let bow_tie = json!({
            "code": CATILLO_POLYGON,
            "coordinates": [[-71.84, -36.15], [-71.82, -36.13], [-71.82, -36.15], [-71.84, -36.13]],
        });
        let message = error(app.call("areas.edit_geometry", bow_tie));
        assert!(message.contains("se cruzan"), "{message}");
        let unknown = json!({
            "code": "AOP99",
            "coordinates": [[-71.84, -36.15], [-71.82, -36.15], [-71.82, -36.13]],
        });
        let message = error(app.call("areas.edit_geometry", unknown));
        assert!(message.contains("AOP99"), "{message}");
        let short =
            json!({ "code": CATILLO_POLYGON, "coordinates": [[-71.84, -36.15], [-71.82, -36.15]] });
        assert!(error(app.call("areas.edit_geometry", short)).contains("al menos 3"));
        assert_eq!(
            ring_points(&app, CATILLO_POLYGON),
            before_change,
            "nothing changed"
        );

        let triangle = json!({
            "code": CATILLO_POLYGON,
            "coordinates": [[-71.84, -36.15], [-71.82, -36.15], [-71.83, -36.13], [-71.84, -36.15]],
        });
        assert_eq!(app.call("areas.edit_geometry", triangle), Ok(json!(null)));
        assert_same_ring(
            &ring_points(&app, CATILLO_POLYGON),
            &[
                LatLon::new(-36.15, -71.84),
                LatLon::new(-36.15, -71.82),
                LatLon::new(-36.13, -71.83),
            ],
        );
        assert_eq!(
            app.app().area.as_deref(),
            Some(CATILLO_POLYGON),
            "and selects it"
        );
    }

    const CMD_OPTION: Modifiers = Modifiers {
        command: true,
        option: true,
        ..Modifiers::NONE
    };

    #[test]
    fn the_inspector_describes_what_is_under_the_pointer() {
        let mut app = TestApp::new(Demo::default());
        assert!(app.screen().query(By::test_id("kai.inspector")).is_none());
        app.press(Key::Char('i'), CMD_OPTION);
        assert!(app.session().inspector_open());
        assert_eq!(app.messages(), [], "⌘⌥I is not ⌘I");
        app.screen().get(By::test_id("kai.inspector.panel"));
        assert!(
            app.screen()
                .query(By::test_id("kai.inspector.outline"))
                .is_none()
        );

        app.hover(By::test_id("bar.add"));
        let screen = app.screen();
        let panel = screen.get(By::test_id("kai.inspector.panel"));
        let shown: Vec<String> = screen
            .within(panel)
            .all(By::role(Role::Label))
            .iter()
            .filter_map(|line| line.name())
            .collect();
        let shown = shown.join("\n");
        assert!(
            shown.starts_with("Image #"),
            "the icon is under the pointer: {shown}"
        );
        assert!(
            shown.contains("clic → Button #") && shown.contains("«Sumar clic» [bar.add]"),
            "{shown}"
        );
        let outline = screen.get(By::test_id("kai.inspector.outline")).bounds();
        let icon = screen
            .within(screen.get(By::test_id("bar.add")))
            .get(By::role(Role::Image))
            .bounds();
        assert_eq!(
            (outline.x, outline.y, outline.width, outline.height),
            (
                icon.x.round(),
                icon.y.round(),
                icon.width.round(),
                icon.height.round()
            )
        );
        assert_eq!(app.session().unknown_classes(), Vec::<&str>::new());

        app.click(By::test_id("bar.add"));
        assert_eq!(app.app().clicks, 1, "the inspector takes no clicks");

        app.press(Key::Char('p'), CMD_SHIFT);
        assert!(app.is_focused(By::test_id("kai.palette.find")));
        app.press(Key::Escape, Modifiers::NONE);
        assert!(!app.session().palette_open());
        assert!(app.session().inspector_open());

        app.press(Key::Char('i'), CMD_OPTION);
        assert!(app.screen().query(By::test_id("kai.inspector")).is_none());
        assert!(app.dump().contains("[bar.add]"));
    }

    /// Lines that differ between `expected` and `actual`, numbered; `None`
    /// when they are equal.
    fn line_diff(expected: &str, actual: &str) -> Option<String> {
        let expected: Vec<&str> = expected.lines().collect();
        let actual: Vec<&str> = actual.lines().collect();
        let mut diff = String::new();
        for line in 0..expected.len().max(actual.len()) {
            let (old, new) = (expected.get(line), actual.get(line));
            if old != new {
                for (sign, text) in [("-", old), ("+", new)] {
                    if let Some(text) = text {
                        diff.push_str(&format!("{:>4} {sign} {text}\n", line + 1));
                    }
                }
            }
        }
        (!diff.is_empty()).then_some(diff)
    }

    /// `mcp-schema.json` is what the demo publishes to agents; a change
    /// must be deliberate. `KAI_UPDATE_SCHEMA=1 cargo test mcp_schema`
    /// rewrites it.
    #[test]
    fn mcp_schema_matches_snapshot() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("mcp-schema.json");
        let actual = describe_json::<Demo>() + "\n";
        if std::env::var_os("KAI_UPDATE_SCHEMA").is_some() {
            std::fs::write(&path, &actual).expect("write mcp-schema.json");
            return;
        }
        let expected = std::fs::read_to_string(&path).unwrap_or_default();
        if let Some(diff) = line_diff(&expected, &actual) {
            panic!(
                "mcp-schema.json cambió; si es a propósito, KAI_UPDATE_SCHEMA=1 cargo test \
                 mcp_schema\n{diff}"
            );
        }
        let document: serde_json::Value = serde_json::from_str(&actual).unwrap();
        assert_eq!(document["$schema"], DIALECT);
        let ids: Vec<&str> = document["commands"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|command| command["id"].as_str())
            .collect();
        assert!(ids.contains(&"counter.add") && ids.contains(&"form.last_submission"));
        assert!(ids.iter().all(|id| !id.starts_with("kai.")), "{ids:?}");
    }

    /// Scroll offsets that bring the card titled `title` to the top of the
    /// content area, measured with real text.
    fn scrolled_to(theme: &Theme, demo: &Demo, viewport: Size, title: &str) -> ScrollOffsets {
        let tree = build_view(demo);
        let mut text = TextStore::new(FontSystem::new());
        let layout = Layout::compute(
            &tree,
            &LayoutInput {
                viewport,
                theme,
                interaction: &Interaction::default(),
                scroll: &ScrollOffsets::new(),
            },
            &mut StyleCache::default(),
            &mut text,
        );
        let content = Screen::new(&layout).get_by_test_id("content").id();
        let card = card_header_of(demo, title);
        let top = layout.bounds(card).unwrap().y - layout.bounds(content).unwrap().y;
        ScrollOffsets::from([(
            content,
            crate::geometry::Point {
                x: 0.0,
                y: (top - 40.0).max(0.0),
            },
        )])
    }

    /// Where a set of captures goes and with which theme.
    struct Out {
        theme: Theme,
        /// Relative to the crate root.
        dir: &'static str,
        /// Appended to every file name.
        suffix: &'static str,
        /// Also write a PNG next to each BMP (with `--features snapshots`).
        png: bool,
    }

    /// Render the demo offscreen exactly as the runtime does and save it as
    /// a BMP under `out.dir`.
    fn snapshot(
        out: &Out,
        name: &str,
        demo: &Demo,
        logical: Size,
        scale: f32,
        interaction: &Interaction,
        scroll: &ScrollOffsets,
    ) {
        let tree = build_view(demo);
        let image = snapshot::render(&Shot {
            tree: &tree,
            theme: &out.theme,
            logical,
            scale,
            interaction,
            scroll,
        })
        .expect("snapshot render");
        write_shot(out, name, &image);
    }

    fn write_shot(out: &Out, name: &str, image: &snapshot::Image) {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(out.dir);
        std::fs::create_dir_all(&dir).unwrap();
        let file = format!("{name}{}", out.suffix);
        snapshot::write_bmp(&dir.join(format!("{file}.bmp")), image).unwrap();
        #[cfg(feature = "snapshots")]
        if out.png {
            snapshot::write_png(&dir.join(format!("{file}.png")), image).unwrap();
        }
        // PNG output needs `--features snapshots`.
        #[cfg(not(feature = "snapshots"))]
        let _ = out.png;
    }

    /// Writes `target/snapshots/*.bmp` for visual review. Needs a GPU:
    /// `cargo test -- --ignored snapshot_demo`
    #[test]
    #[ignore = "needs a GPU; writes images for visual review"]
    fn snapshot_demo() {
        render_demo_shots(&Out {
            theme: Theme::kai(Density::Comfortable),
            dir: "target/snapshots",
            suffix: "",
            png: false,
        });
    }

    /// The same captures with the static AppKit theme, light and dark, as
    /// BMP and PNG under `target/snapshots/appkit/` (not part of the
    /// baseline). `cargo test --features snapshots -- --ignored snapshots_appkit`
    #[test]
    #[ignore = "needs a GPU; writes images for visual review"]
    fn snapshots_appkit() {
        for (generation, appearance, suffix) in [
            (AppKitGeneration::Aqua11, Appearance::Light, ""),
            (AppKitGeneration::Aqua11, Appearance::Dark, "-dark"),
            (AppKitGeneration::Glass27, Appearance::Light, "-glass27"),
            (AppKitGeneration::Glass27, Appearance::Dark, "-glass27-dark"),
        ] {
            render_demo_shots(&Out {
                theme: Theme::appkit_static(generation, appearance),
                dir: "target/snapshots/appkit",
                suffix,
                png: true,
            });
        }
    }

    /// Mixed-theme candidates side by side, for Felipe to choose:
    /// `target/snapshots/mix/mix-compare-{light,dark}.png` (rows Kai |
    /// strict AppKit | A | B | C; columns buttons card | fields card) plus
    /// each candidate's variants-focus and fields shots.
    /// `cargo test --features snapshots -- --ignored snapshots_mix`
    #[cfg(feature = "snapshots")]
    #[test]
    #[ignore = "needs a GPU; writes images for visual review"]
    fn snapshots_mix() {
        use crate::element::{div, text};
        use crate::snapshot::Image;
        use crate::theme::MixStyle;

        const SCALE: f32 = 2.0;
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("target/snapshots/mix");
        std::fs::create_dir_all(&dir).unwrap();
        fn render(
            theme: &Theme,
            demo: &Demo,
            logical: Size,
            interaction: &Interaction,
            scroll: &ScrollOffsets,
        ) -> Image {
            let tree = build_view(demo);
            snapshot::render(&Shot {
                tree: &tree,
                theme,
                logical,
                scale: SCALE,
                interaction,
                scroll,
            })
            .expect("snapshot render")
        }
        // Card `title` at the top of an 800 × `height` crop, below the header.
        let section = |theme: &Theme, demo: &Demo, title: &str, height: f32| -> Image {
            let view = Size {
                width: 800.0,
                height: height + 160.0,
            };
            let scroll = scrolled_to(theme, demo, view, title);
            let full = render(theme, demo, view, &Interaction::default(), &scroll);
            let tree = build_view(demo);
            let mut text = TextStore::new(FontSystem::new());
            let layout = Layout::compute(
                &tree,
                &LayoutInput {
                    viewport: view,
                    theme,
                    interaction: &Interaction::default(),
                    scroll: &scroll,
                },
                &mut StyleCache::default(),
                &mut text,
            );
            let content = Screen::new(&layout).get_by_test_id("content");
            let top = content.bounds().y + 28.0;
            crop(&full, (top * SCALE) as u32, (height * SCALE) as u32)
        };
        let label = |caption: &str| -> Image {
            let mut tree: crate::element::Element<Msg> = div()
                .class("w-full h-full flex items-center px-4 bg-neutral")
                .child(text(caption).class("text-lg font-semibold text-foreground"));
            tree.assign_ids();
            snapshot::render(&Shot {
                tree: &tree,
                theme: &Theme::kai(Density::Comfortable),
                logical: Size {
                    width: 1600.0,
                    height: 36.0,
                },
                scale: SCALE,
                interaction: &Interaction::default(),
                scroll: &ScrollOffsets::new(),
            })
            .expect("label render")
        };
        let demo = Demo {
            clicks: 3,
            ..Demo::at(Section::Buttons)
        };
        let filled = Demo {
            password: "kai-2026".to_owned(),
            ..Demo::at(Section::Fields)
        };
        for (generation, appearance, name) in [
            (AppKitGeneration::Aqua11, Appearance::Light, "light"),
            (AppKitGeneration::Glass27, Appearance::Dark, "dark"),
        ] {
            let strict = Theme::appkit_static(generation, appearance);
            let rows = [
                ("Kai (referencia)", Theme::kai(Density::Comfortable), "kai"),
                ("AppKit estricto (KAI_MIX=appkit)", strict, "appkit"),
                (
                    "A · cerca de AppKit (KAI_MIX=a)",
                    strict.with_mix(MixStyle::A),
                    "a",
                ),
                (
                    "B · intermedio (KAI_MIX=b)",
                    strict.with_mix(MixStyle::B),
                    "b",
                ),
                (
                    "C · cerca de Kai (KAI_MIX=c, por defecto)",
                    strict.with_mix(MixStyle::C),
                    "c",
                ),
            ];
            let mut sheet: Vec<Image> = Vec::new();
            for (caption, theme, tag) in &rows {
                let buttons = section(theme, &demo, BUTTONS_CARD, 300.0);
                let fields = section(theme, &filled, "Variantes de TextField", 470.0);
                sheet.push(label(&format!("{caption} — {generation:?} {appearance:?}")));
                sheet.push(side_by_side(&buttons, &fields));
                if *tag != "kai" {
                    let wide = Size {
                        width: 1024.0,
                        height: 720.0,
                    };
                    let shots = [
                        (
                            "demo-variants-focus",
                            render(
                                theme,
                                &demo,
                                wide,
                                &Interaction {
                                    focused: Some(find(&demo, By::test_id("buttons.secondary"))),
                                    focus_visible: true,
                                    ..Interaction::default()
                                },
                                &scrolled_to(theme, &demo, wide, BUTTONS_CARD),
                            ),
                        ),
                        ("demo-fields", {
                            let view = Size {
                                width: 1024.0,
                                height: 640.0,
                            };
                            let scroll =
                                scrolled_to(theme, &filled, view, "Variantes de TextField");
                            render(theme, &filled, view, &Interaction::default(), &scroll)
                        }),
                    ];
                    for (shot, image) in shots {
                        let path = dir.join(format!("{shot}-{name}-{tag}.png"));
                        snapshot::write_png(&path, &image).unwrap();
                    }
                }
            }
            let path = dir.join(format!("mix-compare-{name}.png"));
            snapshot::write_png(&path, &stack(&sheet)).unwrap();
            println!("{}", path.display());
        }
    }

    /// Renders for Felipe's review under `target/snapshots/renders/`
    /// (`KAI_RENDER_TAG` names the form-row sheet, default `after`):
    /// the form row at wide and narrow widths.
    /// `cargo test --features snapshots -- --ignored snapshots_renders`
    #[cfg(feature = "snapshots")]
    #[test]
    #[ignore = "needs a GPU; writes images for visual review"]
    fn snapshots_renders() {
        use crate::components::{TextField, card, card_content};
        use crate::element::{Element, div, text};
        use crate::theme::{FieldLabels, MixStyle};
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("target/snapshots/renders");
        std::fs::create_dir_all(&dir).unwrap();
        let tag = std::env::var("KAI_RENDER_TAG").unwrap_or_else(|_| "after".to_owned());
        let demo = Demo {
            name: "Ana".to_owned(),
            ..Demo::default()
        };
        let appkit = |appearance| {
            Theme::appkit_static(AppKitGeneration::Aqua11, appearance).with_mix(MixStyle::C)
        };
        // Form row: Kai and mix C, wide | narrow.
        let mut rows = Vec::new();
        for theme in [Theme::kai(Density::Comfortable), appkit(Appearance::Light)] {
            let shot = |width: f32| {
                let view = Size {
                    width,
                    height: 420.0,
                };
                let scroll = scrolled_to(&theme, &demo, view, "Formulario controlado");
                render_image(&theme, &demo, view, &Interaction::default(), &scroll)
            };
            rows.push(side_by_side(&shot(1024.0), &shot(420.0)));
        }
        let path = dir.join(format!("form-row-{tag}.png"));
        snapshot::write_png(&path, &stack(&rows)).unwrap();
        println!("{}", path.display());

        // Whole demo, mix C light (white window), tall enough to show
        // every card.
        let whole = Size {
            width: 1024.0,
            height: 3000.0,
        };
        let image = render_image(
            &appkit(Appearance::Light),
            &demo,
            whole,
            &Interaction::default(),
            &ScrollOffsets::new(),
        );
        let path = dir.join("demo-white-light.png");
        snapshot::write_png(&path, &image).unwrap();
        println!("{}", path.display());

        // Control sizes card: Kai, mix C light and dark.
        let clicks = Demo {
            clicks: 3,
            ..Demo::at(Section::Buttons)
        };
        for (name, theme) in [
            ("kai", Theme::kai(Density::Comfortable)),
            ("appkit-light", appkit(Appearance::Light)),
            ("appkit-dark", appkit(Appearance::Dark)),
        ] {
            let image = card_shot(&theme, &clicks, "Tamaños de control", 900.0, 700.0);
            let path = dir.join(format!("sizes-{name}.png"));
            snapshot::write_png(&path, &image).unwrap();
            println!("{}", path.display());
        }

        // Mix C field labels, appkit | top | near (default) | tab, on a group box
        // whose fill differs from the fields': empty, filled, focused
        // (empty) and required filled.
        for (name, appearance) in [("light", Appearance::Light), ("dark", Appearance::Dark)] {
            let theme = appkit(appearance);
            let columns = [
                ("KAI_LABEL=appkit", FieldLabels::AppKit),
                ("KAI_LABEL=top", FieldLabels::Top),
                ("near (default)", FieldLabels::Near),
                ("KAI_LABEL=tab", FieldLabels::Tab),
            ]
            .map(|(title, labels)| {
                let mut tree: Element<Msg> = div()
                    .class("w-full h-full flex flex-col gap-3 p-4 bg-window")
                    .children([
                        text(title).class("text-headline text-label"),
                        card()
                            .class("w-full")
                            .child(card_content().class("gap-4").children([
                                TextField::new("Nombre", "").build(),
                                TextField::new("Correo", "ana@kai.cl").build(),
                                TextField::new("Ciudad", "").build(),
                                TextField::new("Comuna", "Ñuñoa").required(true).build(),
                            ])),
                    ]);
                tree.assign_ids();
                let mut focused = None;
                tree.walk(&mut |e| {
                    if matches!(e.kind, Kind::Input { .. }) && e.label.as_deref() == Some("Ciudad")
                    {
                        focused = Some(e.id);
                    }
                });
                snapshot::render(&Shot {
                    tree: &tree,
                    theme: &theme.with_field_labels(labels),
                    logical: Size {
                        width: 300.0,
                        height: 400.0,
                    },
                    scale: 2.0,
                    interaction: &Interaction {
                        focused,
                        ..Interaction::default()
                    },
                    scroll: &ScrollOffsets::new(),
                })
                .expect("labels render")
            });
            let sheet = columns[1..]
                .iter()
                .fold(columns[0].clone(), |sheet, column| {
                    side_by_side(&sheet, column)
                });
            let path = dir.join(format!("labels-compare-{name}.png"));
            snapshot::write_png(&path, &sheet).unwrap();
            println!("{}", path.display());
        }

        // Mix C field (near label) next to every button size, in an
        // `items-end` row and an `items-center` row, regular and compact.
        for (name, appearance) in [
            ("field-button-align", Appearance::Light),
            ("field-button-align-dark", Appearance::Dark),
        ] {
            use crate::components::{ButtonVariant, FieldDensity, button};
            use crate::element::ControlSize;
            let theme = appkit(appearance);
            let row = |items: &str, density: FieldDensity, variant: ButtonVariant| {
                div()
                    .class(&format!("flex gap-2 {items}"))
                    .child(
                        div()
                            .class("w-[220px]")
                            .child(TextField::new("Campo", "").density(density).build()),
                    )
                    .children(
                        ControlSize::ALL
                            .map(|size| button(format!("{size:?}"), variant).control_size(size)),
                    )
            };
            let mut tree: Element<Msg> = div()
                .class("w-full h-full flex flex-col gap-4 p-4 bg-window")
                .children(["items-end", "items-center"].map(|items| {
                    div().class("flex flex-col gap-3").children([
                        text(format!("{items} (campo normal y compacto)"))
                            .class("text-headline text-label"),
                        row(items, FieldDensity::Default, ButtonVariant::Primary),
                        row(items, FieldDensity::Default, ButtonVariant::Outlined),
                        row(items, FieldDensity::Compact, ButtonVariant::Primary),
                        row(items, FieldDensity::Compact, ButtonVariant::Outlined),
                    ])
                }));
            tree.assign_ids();
            let image = snapshot::render(&Shot {
                tree: &tree,
                theme: &theme,
                logical: Size {
                    width: 760.0,
                    height: 640.0,
                },
                scale: 2.0,
                interaction: &Interaction::default(),
                scroll: &ScrollOffsets::new(),
            })
            .expect("field-button render");
            let path = dir.join(format!("{name}.png"));
            snapshot::write_png(&path, &image).unwrap();
            println!("{}", path.display());
        }
    }

    /// Background effects under the demo, as the runtime draws them, under
    /// `target/snapshots/renders/`: `bg-{none,mesh,aurora,glow}-{kai,
    /// appkit-light,appkit-dark}.png` (static frames, top of the page with
    /// the panel), `bg-sheet-*.png` side by side, and `bg-{kind}-t{s}.png`
    /// frames of the animated effects at a few times.
    /// `cargo test --features snapshots -- --ignored snapshots_backgrounds`
    #[cfg(feature = "snapshots")]
    #[test]
    #[ignore = "needs a GPU; writes images for visual review"]
    fn snapshots_backgrounds() {
        use crate::background::{BackgroundKind, BackgroundMotion};
        use crate::theme::MixStyle;
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("target/snapshots/renders");
        std::fs::create_dir_all(&dir).unwrap();
        let view = Size {
            width: 1024.0,
            height: 720.0,
        };
        let shot = |theme: &Theme, demo: &Demo, seconds: f32| {
            let tree = build_view(demo);
            snapshot::render_with_background(
                &Shot {
                    tree: &tree,
                    theme,
                    logical: view,
                    scale: 2.0,
                    interaction: &Interaction::default(),
                    scroll: &ScrollOffsets::new(),
                },
                demo.background(),
                seconds,
            )
            .expect("background render")
        };
        let mix = |appearance| {
            Theme::appkit_static(AppKitGeneration::Aqua11, appearance).with_mix(MixStyle::C)
        };
        let themes = [
            ("kai", Theme::kai(Density::Comfortable)),
            ("appkit-light", mix(Appearance::Light)),
            ("appkit-dark", mix(Appearance::Dark)),
        ];
        for (name, theme) in &themes {
            let mut row: Option<crate::snapshot::Image> = None;
            for kind in BackgroundKind::ALL {
                let demo = Demo {
                    background: kind,
                    ..Demo::default()
                };
                let image = shot(theme, &demo, 0.0);
                let path = dir.join(format!("bg-{}-{name}.png", kind.name()));
                snapshot::write_png(&path, &image).unwrap();
                println!("{}", path.display());
                row = Some(match row {
                    None => image,
                    Some(row) => side_by_side(&row, &image),
                });
            }
            let path = dir.join(format!("bg-sheet-{name}.png"));
            snapshot::write_png(&path, &row.unwrap()).unwrap();
            println!("{}", path.display());
        }
        // Motion: the animated effects at 0, 4 and 8 s of a 16 s cycle.
        let theme = mix(Appearance::Light);
        for kind in [
            BackgroundKind::Mesh,
            BackgroundKind::Aurora,
            BackgroundKind::Glow,
        ] {
            let demo = Demo {
                background: kind,
                background_motion: BackgroundMotion::Animated,
                ..Demo::default()
            };
            assert!(demo.background().is_animated());
            let frames: Vec<_> = [0.0, 4.0, 8.0]
                .into_iter()
                .map(|seconds| {
                    let image = shot(&theme, &demo, seconds);
                    let path = dir.join(format!("bg-{}-t{seconds}.png", kind.name()));
                    snapshot::write_png(&path, &image).unwrap();
                    println!("{}", path.display());
                    image
                })
                .collect();
            assert_ne!(frames[0], frames[1], "{kind:?} moves");
            let sheet = side_by_side(&side_by_side(&frames[0], &frames[1]), &frames[2]);
            let path = dir.join(format!("bg-{}-motion.png", kind.name()));
            snapshot::write_png(&path, &sheet).unwrap();
            println!("{}", path.display());
        }
        // Static effects ignore the time.
        let still = Demo {
            background: BackgroundKind::Aurora,
            ..Demo::default()
        };
        assert_eq!(shot(&theme, &still, 0.0), shot(&theme, &still, 5.0));
    }

    /// The card titled `title`, `width` × `height` points from just above
    /// its top (below the header), at 2x.
    #[cfg(feature = "snapshots")]
    fn card_shot(
        theme: &Theme,
        demo: &Demo,
        title: &str,
        width: f32,
        height: f32,
    ) -> crate::snapshot::Image {
        let view = Size {
            width,
            height: height + 160.0,
        };
        let scroll = scrolled_to(theme, demo, view, title);
        let full = render_image(theme, demo, view, &Interaction::default(), &scroll);
        let tree = build_view(demo);
        let mut text = TextStore::new(FontSystem::new());
        let layout = Layout::compute(
            &tree,
            &LayoutInput {
                viewport: view,
                theme,
                interaction: &Interaction::default(),
                scroll: &scroll,
            },
            &mut StyleCache::default(),
            &mut text,
        );
        // 28 pt above the title: the card's top edge plus a margin.
        let header = Screen::new(&layout).get(By::role(Role::Header)).bounds();
        let title = layout.bounds(card_header_of(demo, title)).unwrap().y - 28.0;
        let top = title.max(header.y + header.height);
        crop(&full, (top * 2.0) as u32, (height * 2.0) as u32)
    }

    /// The demo at `logical` (2x), as the runtime draws it.
    #[cfg(feature = "snapshots")]
    fn render_image(
        theme: &Theme,
        demo: &Demo,
        logical: Size,
        interaction: &Interaction,
        scroll: &ScrollOffsets,
    ) -> crate::snapshot::Image {
        let tree = build_view(demo);
        snapshot::render(&Shot {
            tree: &tree,
            theme,
            logical,
            scale: 2.0,
            interaction,
            scroll,
        })
        .expect("snapshot render")
    }

    /// Rows `top..top + height` of `image` (clamped).
    #[cfg(feature = "snapshots")]
    fn crop(image: &crate::snapshot::Image, top: u32, height: u32) -> crate::snapshot::Image {
        let top = top.min(image.height);
        let height = height.min(image.height - top);
        let row = (image.width * 4) as usize;
        let start = top as usize * row;
        crate::snapshot::Image {
            width: image.width,
            height,
            rgba: image.rgba[start..start + height as usize * row].to_vec(),
        }
    }

    /// `a` and `b` next to each other, top-aligned, on white.
    #[cfg(feature = "snapshots")]
    fn side_by_side(
        a: &crate::snapshot::Image,
        b: &crate::snapshot::Image,
    ) -> crate::snapshot::Image {
        let (width, height) = (a.width + b.width, a.height.max(b.height));
        let mut rgba = vec![255; (width * height * 4) as usize];
        for (image, x0) in [(a, 0), (b, a.width)] {
            for y in 0..image.height {
                let src = (y * image.width * 4) as usize;
                let dst = ((y * width + x0) * 4) as usize;
                rgba[dst..dst + (image.width * 4) as usize]
                    .copy_from_slice(&image.rgba[src..src + (image.width * 4) as usize]);
            }
        }
        crate::snapshot::Image {
            width,
            height,
            rgba,
        }
    }

    /// `images` one below the other, left-aligned, on white.
    #[cfg(feature = "snapshots")]
    fn stack(images: &[crate::snapshot::Image]) -> crate::snapshot::Image {
        let width = images.iter().map(|i| i.width).max().unwrap_or(0);
        let mut rgba = Vec::new();
        let mut height = 0;
        for image in images {
            for y in 0..image.height {
                let src = (y * image.width * 4) as usize;
                rgba.extend_from_slice(&image.rgba[src..src + (image.width * 4) as usize]);
                rgba.resize(rgba.len() + ((width - image.width) * 4) as usize, 255);
            }
            height += image.height;
        }
        crate::snapshot::Image {
            width,
            height,
            rgba,
        }
    }

    fn render_demo_shots(out: &Out) {
        let wide = Size {
            width: 1024.0,
            height: 720.0,
        };
        let idle = Demo::default();
        let none = ScrollOffsets::new();
        snapshot(
            out,
            "demo-idle",
            &idle,
            wide,
            2.0,
            &Interaction::default(),
            &none,
        );

        let mut typed = Demo {
            clicks: 3,
            ..Demo::default()
        };
        typed.apply(Msg::NameChanged("Ana".to_owned()));
        let hovered_button = find(&typed, By::role(Role::Button).name("Incrementar"));
        let field = find(&typed, By::label("Nombre"));
        let tree = build_view(&typed);
        snapshot(
            out,
            "demo-interaction",
            &typed,
            wide,
            2.0,
            &Interaction {
                hovered: tree.path_to(hovered_button).unwrap(),
                focused: Some(field),
                ..Interaction::default()
            },
            &none,
        );
        snapshot(
            out,
            "demo-narrow",
            &typed,
            Size {
                width: 420.0,
                height: 560.0,
            },
            2.0,
            &Interaction::default(),
            &none,
        );

        let buttons = Demo {
            section: Section::Buttons,
            ..typed
        };
        snapshot(
            out,
            "demo-variants-focus",
            &buttons,
            wide,
            2.0,
            &Interaction {
                focused: Some(find(&buttons, By::test_id("buttons.secondary"))),
                // Reached with Tab: the ring is drawn.
                focus_visible: true,
                ..Interaction::default()
            },
            &scrolled_to(&out.theme, &buttons, wide, BUTTONS_CARD),
        );

        let fields_view = Size {
            width: 1024.0,
            height: 640.0,
        };
        let title = "Variantes de TextField";
        let masked = Demo {
            password: "kai-2026".to_owned(),
            ..Demo::at(Section::Fields)
        };
        snapshot(
            out,
            "demo-fields",
            &masked,
            fields_view,
            2.0,
            &Interaction::default(),
            &scrolled_to(&out.theme, &masked, fields_view, title),
        );
        let mut filled = Demo {
            show_password: true,
            ..Demo::at(Section::Fields)
        };
        for (field, value) in [
            (Field::Search, "zapatillas"),
            (Field::Password, "kai-2026"),
            (Field::Amount, "15000"),
            (Field::Code, "A-12"),
            (Field::Quantity, "3"),
        ] {
            filled.apply(Msg::Edit(field, value.to_owned()));
        }
        let password = find(&filled, By::test_id("form.password.input"));
        let tree = build_view(&filled);
        snapshot(
            out,
            "demo-fields-focus",
            &filled,
            fields_view,
            2.0,
            &Interaction {
                focused: Some(password),
                hovered: tree.path_to(password).unwrap(),
                ..Interaction::default()
            },
            &scrolled_to(&out.theme, &filled, fields_view, title),
        );

        for (name, section) in [
            ("demo-seleccion", Section::Selection),
            ("demo-contenedores", Section::Containers),
            ("demo-tabla", Section::Table),
            ("demo-mapa", Section::Map),
            ("demo-mapa-areas", Section::Map),
            ("demo-graficos", Section::Charts),
            ("demo-documento", Section::Document),
        ] {
            let mut demo = Demo::at(section);
            if section == Section::Table {
                demo.apply(Msg::Table(ViewChange::Sort(4, SortDirection::Descending)));
                let second = demo.table_view.rows()[1];
                demo.apply(Msg::SelectRow(second));
            }
            if name == "demo-mapa" {
                demo.apply(Msg::ToggleAreas);
            }
            snapshot(out, name, &demo, wide, 2.0, &Interaction::default(), &none);
        }

        // The view the map reports once it has framed Catillo.
        let mut selection_view = TestApp::builder(Demo::at(Section::Map))
            .viewport(wide.width, wide.height)
            .build();
        selection_view.click(By::test_id("map.sector.AOP26000002"));
        selection_view.advance(Duration::from_secs(1));
        snapshot(
            out,
            "demo-mapa-seleccion",
            selection_view.app(),
            wide,
            2.0,
            &Interaction::default(),
            &none,
        );

        // Catillo being edited, its north east corner pulled out.
        let mut editing = TestApp::builder(Demo::at(Section::Map))
            .viewport(wide.width, wide.height)
            .build();
        editing.click(By::test_id("map.sector.AOP26000002"));
        editing.click(By::test_id("map.edit"));
        editing.advance(Duration::from_secs(1));
        editing.drag_and_hold(By::test_id("map.view.vertex.2"), 40.0, -30.0);
        snapshot(
            out,
            "demo-mapa-edicion",
            editing.app(),
            wide,
            2.0,
            &Interaction::default(),
            &none,
        );

        let mut media = Demo::at(Section::Multimedia);
        media.apply(Msg::ImagePicked(
            ImageField::Logo,
            Some("/no/existe/logo.gif".into()),
        ));
        snapshot(
            out,
            "demo-multimedia",
            &media,
            wide,
            2.0,
            &Interaction::default(),
            &none,
        );
        let banner = find(&media, By::test_id("media.cover.preview"));
        let tree = build_view(&media);
        snapshot(
            out,
            "demo-multimedia-drag",
            &media,
            wide,
            2.0,
            &Interaction {
                hovered: tree.path_to(banner).unwrap(),
                file_drag: true,
                ..Interaction::default()
            },
            &scrolled_to(&out.theme, &media, wide, "Campos de imagen"),
        );
        let logo = find(&media, By::test_id("media.logo.preview"));
        snapshot(
            out,
            "demo-multimedia-hover",
            &media,
            wide,
            2.0,
            &Interaction {
                hovered: tree.path_to(logo).unwrap(),
                ..Interaction::default()
            },
            &scrolled_to(&out.theme, &media, wide, "Campos de imagen"),
        );

        let mut dialog = Demo::at(Section::Dialogs);
        dialog.apply(Msg::OpenDialog(DemoDialog::Delete));
        snapshot(
            out,
            "demo-dialogo",
            &dialog,
            wide,
            2.0,
            &Interaction::default(),
            &none,
        );
        let mut toasts = Demo::at(Section::Dialogs);
        for kind in ToastKind::ALL {
            toasts.apply(Msg::ShowToast(kind));
        }
        snapshot(
            out,
            "demo-avisos",
            &toasts,
            wide,
            2.0,
            &Interaction::default(),
            &none,
        );

        // Two buttons waiting on their tasks and the dots alone, with the
        // wave caught mid-jump.
        let buttons_view = Demo::at(Section::Buttons);
        let mut tree = build_view(&buttons_view);
        for test_id in ["wait.primary", "wait.outlined", "wait.reload"] {
            let id = find(&buttons_view, By::test_id(test_id));
            tree.find_mut(id).unwrap().busy = true;
        }
        let image = snapshot::render_with_background(
            &Shot {
                tree: &tree,
                theme: &out.theme,
                logical: wide,
                scale: 2.0,
                interaction: &Interaction::default(),
                scroll: &scrolled_to(&out.theme, &buttons_view, wide, "Acciones con espera"),
            },
            BackgroundEffect::NONE,
            0.3,
        )
        .expect("snapshot render");
        write_shot(out, "demo-botones-cargando", &image);

        let mut rail = Demo::at(Section::Table);
        rail.apply(Msg::ToggleSidebar);
        snapshot(
            out,
            "demo-sidebar-colapsada",
            &rail,
            wide,
            2.0,
            &Interaction::default(),
            &none,
        );
    }

    #[test]
    fn editor_inserts_unicode_and_selects_text() {
        let mut font_system = FontSystem::new();
        let mut buffer = Buffer::new(&mut font_system, Metrics::new(15.0, 22.0));
        buffer.set_text("", &Attrs::new(), Shaping::Advanced, None);
        let mut editor = Editor::new(buffer);
        editor.insert_string("Kai 😀", None);
        editor.shape_as_needed(&mut font_system, false);
        editor.action(&mut font_system, Action::Motion(Motion::BufferStart));
        let anchor = editor.cursor();
        editor.set_selection(Selection::Normal(anchor));
        editor.action(&mut font_system, Action::Motion(Motion::BufferEnd));
        assert_eq!(editor.copy_selection().as_deref(), Some("Kai 😀"));
        editor.insert_string("Kai UI", None);
        let value = editor.with_buffer(|buffer| buffer.lines[0].text().to_owned());
        assert_eq!(value, "Kai UI");
    }
}
