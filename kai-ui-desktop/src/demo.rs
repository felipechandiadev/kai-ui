//! Kai UI Desktop demo, shared by the `kai-ui-desktop-prototype` binary
//! (`cargo run`) and `examples/demo.rs` (`cargo run --example demo`).

use std::path::PathBuf;
use std::rc::Rc;
use std::sync::LazyLock;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::background::{BackgroundEffect, BackgroundKind, BackgroundMotion};
use crate::color::Color;
use crate::commands::{
    Action, Availability, CommandError, CommandRegistry, PALETTE, Query, Safety, Shortcut,
    describe_json,
};
use crate::components::{
    AlertVariant, Avatar, AvatarSize, BadgeVariant, BezelStyle, Button, ButtonSize, ButtonVariant,
    CHOICE_ACTIVE, CHOICE_HOVER, ChartKind, Column, Dialog, DialogWidth, DotProgress, DotSize,
    FieldBezel, FieldDensity, FieldVariant, IconButtonSize, IconButtonVariant, LabelLayout,
    MediaSlot, MediaVariant, SearchVariant, Sidebar, SidebarGroup, SidebarItem, SwitchDensity,
    Table, TableView, Tabs, TextField, Toast, ViewChange, alert, avatar, badge, button,
    button_sized, caption, card, card_actions, card_content, card_header, chart, checkbox,
    checkbox_with_options, color_field, column_menu, dot_progress, group_box, heading, icon_button,
    icon_button_sized, paragraph, search_field, segmented, segmented_with_options, select,
    select_with_options, stat, switch, switch_labeled, switch_with_options, text_area, toast_key,
    toast_stack, toast_timer, toolbar, toolbar_item, toolbar_search, toolbar_separator,
    toolbar_spacer, toolbar_title, validate_image_file, virtual_list,
};
use crate::element::{ControlSize, ControlState, Element, Role, div, icon, image, text};
use crate::document::{document, DocumentChange, DocumentCmd};
#[cfg(feature = "document")]
use crate::document::{document_cmd_for, document_ribbon, RibbonMode};
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
    seccion: Seccion,
    /// The sidebar is a rail of icons.
    sidebar_collapsed: bool,
    /// The first window size has set `sidebar_collapsed`.
    sized: bool,
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
    permisos: [bool; 3],
    /// Index into [`PAISES`].
    pais: Option<usize>,
    /// Index into [`ALINEACIONES`].
    alineacion: usize,
    /// Contenedores: the selected tab.
    pestana: usize,
    /// Tabla: the toolbar search, the clients, what the table shows of
    /// them (search, filters, sort) and the selected one (index into
    /// `clientes`), and whether an Excel file is being written.
    query: String,
    clientes: Rc<[Cliente]>,
    tabla: TableView,
    cliente: Option<usize>,
    exportando: bool,
    /// Multimedia: the three image fields, the URL typed and the one
    /// loaded in the avatar.
    foto: Imagen,
    portada: Imagen,
    logo: Imagen,
    url: String,
    avatar_url: Option<String>,
    /// Diálogos y avisos.
    dialogo: Option<Dialogo>,
    toasts: Vec<Toast<Msg>>,
    next_toast: u64,
    /// Mapa: what it shows, and whether a new view glides there.
    vista: MapView,
    animar_vista: bool,
    /// Mapa: the DSP sectors (their outlines change when edited), whether
    /// they are drawn, the code of the one selected and the last request
    /// to frame a box (with its serial).
    sectores: Vec<Sector>,
    mostrar_areas: bool,
    area: Option<String>,
    encuadre: Option<(LatLonBounds, u64)>,
    /// Mapa: the sector being edited and its draft.
    edicion: Option<(String, PolygonDraft)>,
    /// Botones: the last wait that ended, and how many background syncs
    /// run.
    espera: Option<&'static str>,
    sincronizando: u32,
    /// Gráficos: the selected bar (month index).
    mes: Option<usize>,
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
    doc_archivo_busy: bool,
    /// Pestaña activa del ribbon Documento (`home` / `insert`).
    #[cfg_attr(not(feature = "document"), allow(dead_code))]
    doc_ribbon_tab: String,
}

/// How long the «Acciones con espera» buttons wait.
const ESPERA: std::time::Duration = std::time::Duration::from_millis(1500);

impl Default for Demo {
    fn default() -> Self {
        let clientes: Rc<[Cliente]> = (1..=CLIENTES).map(cliente).collect();
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
            seccion: Seccion::Inicio,
            sidebar_collapsed: false,
            sized: false,
            background: BackgroundKind::None,
            background_motion: BackgroundMotion::Static,
            pin: String::new(),
            show_pin: false,
            notes: String::new(),
            lookup: String::new(),
            wifi: true,
            permisos: [true, false, true],
            pais: Some(0),
            alineacion: 0,
            pestana: 0,
            query: String::new(),
            tabla: TableView::new(&clientes, &columnas()),
            clientes,
            cliente: None,
            exportando: false,
            foto: Imagen::with(sample_portrait()),
            portada: Imagen::default(),
            logo: Imagen::with(sample_landscape()),
            url: String::new(),
            avatar_url: None,
            dialogo: None,
            toasts: Vec::new(),
            next_toast: 1,
            vista: VISTA_INICIAL,
            animar_vista: true,
            sectores: SECTORES.clone(),
            mostrar_areas: true,
            area: None,
            encuadre: None,
            edicion: None,
            espera: None,
            sincronizando: 0,
            mes: None,
            doc_markdown: demo_doc_markdown(),
            doc_cmd: None,
            doc_cmd_serial: 0,
            doc_load: None,
            doc_load_serial: 0,
            doc_pending_export: None,
            doc_archivo_busy: false,
            doc_ribbon_tab: "home".into(),
        }
    }
}

/// Plaza de Armas de Parral, where the map starts.
const PLAZA_DE_ARMAS: LatLon = LatLon::new(-36.1415, -71.8265);
const VISTA_INICIAL: MapView = MapView::new(PLAZA_DE_ARMAS, 13.0);

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
const MARGEN_SECTOR: f64 = 0.2;
/// Demo chart: six months of clicks and a longer trend.
const MESES: [&str; 6] = ["Ene", "Feb", "Mar", "Abr", "May", "Jun"];
const CLICS_MES: [f32; 6] = [4.0, 12.0, 7.0, 15.0, 9.0, 11.0];
const TENDENCIA: [f32; 12] = [
    2.0, 5.0, 4.0, 8.0, 6.0, 9.0, 7.0, 12.0, 10.0, 14.0, 11.0, 16.0,
];

/// An area of operation of DSP.
#[derive(Clone, Debug, PartialEq)]
struct Sector {
    code: String,
    nombre: String,
    tipo: String,
    capa: MapLayer,
    /// Its color as typed in Mapa: hex, or empty for the color of its type.
    color: String,
}

/// The five starting colors of DSP's area types.
const PALETA_DSP: [(&str, Color); 5] = [
    ("Azul", Color::hex(0x1e73ae)),
    ("Naranjo", Color::hex(0xc45c26)),
    ("Verde", Color::hex(0x2d6a4f)),
    ("Violeta", Color::hex(0x7c3aed)),
    ("Ámbar", Color::hex(0xb45309)),
];

/// The color of an area type, as in DSP's `operation_area_types`.
fn color_del_tipo(tipo: &str) -> Color {
    match tipo {
        "territorio" => Color::hex(0x1e73ae),
        "barrio" => Color::hex(0x2d6a4f),
        _ => Color::hex(0xc45c26),
    }
}

impl Sector {
    /// Its outline, without repeating the first vertex.
    fn anillo(&self) -> Vec<LatLon> {
        self.capa.outer_ring()
    }

    /// Take `anillo` as its new outline; a ring a layer cannot be made
    /// of leaves it as it was.
    fn con_anillo(&mut self, anillo: &[LatLon]) {
        if let Ok(capa) = MapLayer::ring(anillo) {
            self.capa = capa;
        }
    }

    /// The color typed for it, or else the one of its type (also while
    /// the typed hex is incomplete).
    fn color(&self) -> Color {
        Color::parse_hex(&self.color).unwrap_or_else(|| color_del_tipo(&self.tipo))
    }

    /// Background class of its color.
    fn swatch(&self) -> String {
        format!("bg-[{}]", self.color().to_hex())
    }
}

static SECTORES: LazyLock<Vec<Sector>> = LazyLock::new(|| {
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
        .map(|(capa, properties)| {
            let tipo = text(&properties, "type");
            Sector {
                code: text(&properties, "code"),
                nombre: text(&properties, "name"),
                color: color_del_tipo(&tipo).to_hex(),
                tipo,
                capa,
            }
        })
        .collect()
});

/// The sector `code` as the seed draws it.
fn sector_inicial(code: &str) -> Option<&'static Sector> {
    SECTORES.iter().find(|sector| sector.code == code)
}

/// Area in hectares with a decimal comma.
fn hectareas(area: f64) -> String {
    format!("{area:.1} ha").replace('.', ",")
}

/// Largest file the image fields take, in megabytes.
const MAX_IMAGEN_MB: u64 = 5;

/// An image field of «Multimedia»: what it shows and the last file
/// refused.
#[derive(Clone, Debug, Default, PartialEq)]
struct Imagen {
    actual: Option<ImageSource>,
    error: Option<String>,
}

impl Imagen {
    fn with(source: ImageSource) -> Self {
        Self {
            actual: Some(source),
            error: None,
        }
    }

    /// Show `path` if it is an image of at most [`MAX_IMAGEN_MB`].
    fn set(&mut self, path: PathBuf) {
        match validate_image_file(&path, MAX_IMAGEN_MB) {
            Ok(()) => *self = Self::with(ImageSource::Path(path)),
            Err(error) => self.error = Some(error),
        }
    }
}

/// The image fields of «Multimedia».
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Campo {
    Foto,
    Portada,
    Logo,
}

/// Rows of the client table.
const CLIENTES: u32 = 10_000;

/// Pause in the typing of a column filter before the table refreshes.
const PAUSA_FILTRO: std::time::Duration = std::time::Duration::from_millis(250);

/// One client of the generated table: number, name, city, plan, amount.
#[derive(Debug, PartialEq)]
struct Cliente {
    numero: u32,
    nombre: String,
    ciudad: &'static str,
    plan: &'static str,
    monto: u32,
}

/// Client `numero` (1-based), the same every time.
fn cliente(numero: u32) -> Cliente {
    const NOMBRES: [&str; 12] = [
        "Ana", "Bruno", "Carla", "Diego", "Elena", "Felipe", "Gabriela", "Hugo", "Inés", "Javier",
        "Laura", "Matías",
    ];
    const APELLIDOS: [&str; 10] = [
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
    const CIUDADES: [&str; 8] = [
        "Santiago",
        "Valparaíso",
        "Concepción",
        "La Serena",
        "Antofagasta",
        "Temuco",
        "Puerto Montt",
        "Arica",
    ];
    const PLANES: [&str; 3] = ["Básico", "Pro", "Empresa"];
    let n = numero as usize;
    let mix = n.wrapping_mul(2_654_435_761) >> 7;
    Cliente {
        numero,
        nombre: format!(
            "{} {}",
            NOMBRES[n % NOMBRES.len()],
            APELLIDOS[(n / NOMBRES.len() + mix) % APELLIDOS.len()]
        ),
        ciudad: CIUDADES[mix % CIUDADES.len()],
        plan: PLANES[(mix / 7) % PLANES.len()],
        monto: (mix % 9_000 + 100) as u32 * 10,
    }
}

/// Columns of the client table: number, name with its avatar, city, plan
/// as a badge, amount and the row's actions.
fn columnas() -> Vec<Column<Cliente, Msg>> {
    vec![
        Column::number("N.º", |c: &Cliente| f64::from(c.numero))
            .fixed(72.0)
            .sortable(),
        Column::text("Nombre", |c: &Cliente| c.nombre.clone())
            .render(|c: &Cliente| {
                div().class("flex items-center gap-2 min-w-0").children([
                    Avatar::new(c.nombre.clone()).size(AvatarSize::Xs).build(),
                    text(c.nombre.as_str()).class("truncate"),
                ])
            })
            .flex(2.0)
            .min_width(160.0)
            .sortable()
            .filterable(),
        Column::text("Ciudad", |c: &Cliente| c.ciudad.to_owned())
            .flex(1.5)
            .sortable()
            .filterable()
            .hideable(),
        Column::text("Plan", |c: &Cliente| c.plan.to_owned())
            .render(|c: &Cliente| badge(c.plan, plan_badge(c.plan)))
            .fixed(104.0)
            .sortable()
            .filterable()
            .hideable(),
        Column::number("Monto", |c: &Cliente| f64::from(c.monto))
            .format(|monto| format!("$ {monto}"))
            .fixed(112.0)
            .sortable()
            .filterable()
            .hideable(),
        Column::actions(|c: &Cliente| {
            icon_button_sized(
                Icon::Eye,
                format!("Ver {}", c.nombre),
                IconButtonVariant::Text,
                IconButtonSize::Sm,
            )
            .on_click(Msg::VerCliente(c.numero as usize - 1))
            .test_id(format!("tabla.ver.{}", c.numero))
        })
        .fixed(64.0),
    ]
}

/// Panel of an expanded client row: what the row leaves out, and a button.
fn detalle_cliente(c: &Cliente) -> Element<Msg> {
    div()
        .class("flex items-center justify-between gap-3 h-full")
        .children([
            div().class("flex flex-col gap-0.5 min-w-0").children([
                text(format!("Cliente N.º {} de {}", c.numero, c.ciudad))
                    .class("text-sm font-medium truncate"),
                caption(format!("Plan {} · $ {} al mes", c.plan, c.monto)),
            ]),
            button_sized("Ver ficha", ButtonVariant::Outlined, ButtonSize::Sm)
                .on_click(Msg::VerCliente(c.numero as usize - 1))
                .test_id(format!("tabla.ficha.{}", c.numero)),
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
const PAISES: [&str; 4] = ["Chile", "Argentina", "Perú", "Uruguay"];
/// Options of the alignment segmented control (Selección).
const ALINEACIONES: [&str; 3] = ["Izquierda", "Centro", "Derecha"];
/// Permissions under «Todos» (Selección).
const PERMISOS: [&str; 3] = ["Leer", "Escribir", "Compartir"];

/// The open dialog of «Diálogos y avisos».
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Dialogo {
    Guardar,
    Eliminar,
}

/// Variant of a toast, for `avisos.mostrar`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
enum VarianteAviso {
    Exito,
    Error,
    Advertencia,
    Info,
}

impl VarianteAviso {
    const ALL: [VarianteAviso; 4] = [Self::Exito, Self::Error, Self::Advertencia, Self::Info];

    fn alert(self) -> AlertVariant {
        match self {
            Self::Exito => AlertVariant::Success,
            Self::Error => AlertVariant::Error,
            Self::Advertencia => AlertVariant::Warning,
            Self::Info => AlertVariant::Info,
        }
    }

    fn key(self) -> &'static str {
        match self {
            Self::Exito => "exito",
            Self::Error => "error",
            Self::Advertencia => "advertencia",
            Self::Info => "info",
        }
    }

    /// Button label, toast title and message.
    fn texts(self) -> (&'static str, &'static str, &'static str) {
        match self {
            Self::Exito => ("Éxito", "Cambios guardados", "Se guardaron hace un momento"),
            Self::Error => (
                "Error",
                "No se pudo enviar",
                "Revisa la conexión y reintenta",
            ),
            Self::Advertencia => ("Advertencia", "Espacio casi lleno", "Quedan 200 MB libres"),
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
struct EstadoDev {
    seccion: Option<Seccion>,
    sidebar_collapsed: Option<bool>,
    /// Code of the selected sector in Mapa.
    area: Option<String>,
}

/// Sections of the sidebar, in its order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
enum Seccion {
    /// Fondo de ventana, contador, formulario y estadísticas.
    Inicio,
    /// Variantes de TextField.
    Campos,
    /// Botones, tamaños e IconButtons.
    Botones,
    /// Switches, checkboxes, selects y controles segmentados.
    Seleccion,
    /// Cards, group boxes, estadísticas y texto.
    Contenedores,
    /// Badges y alertas.
    Badges,
    /// Avatares, imágenes y campos de imagen (foto, portada, logo).
    Multimedia,
    /// Tabla de 10 000 clientes y lista virtualizada.
    Tabla,
    /// Mapa interactivo de Parral.
    Mapa,
    /// Gráficos de barras y sparkline.
    Graficos,
    /// Mini procesador de texto (esqueleto Document).
    Documento,
    /// Diálogos, avisos (toasts) y la paleta de comandos.
    Dialogos,
}

impl Seccion {
    const ALL: [Seccion; 12] = [
        Self::Inicio,
        Self::Campos,
        Self::Botones,
        Self::Seleccion,
        Self::Contenedores,
        Self::Badges,
        Self::Multimedia,
        Self::Tabla,
        Self::Mapa,
        Self::Graficos,
        Self::Documento,
        Self::Dialogos,
    ];

    fn index(self) -> usize {
        Self::ALL.iter().position(|s| *s == self).unwrap_or(0)
    }

    fn key(self) -> &'static str {
        match self {
            Self::Inicio => "inicio",
            Self::Campos => "campos",
            Self::Botones => "botones",
            Self::Seleccion => "seleccion",
            Self::Contenedores => "contenedores",
            Self::Badges => "badges",
            Self::Multimedia => "multimedia",
            Self::Tabla => "tabla",
            Self::Mapa => "mapa",
            Self::Graficos => "graficos",
            Self::Documento => "documento",
            Self::Dialogos => "dialogos",
        }
    }

    /// Sidebar row.
    fn item(self) -> SidebarItem {
        let (label, glyph) = match self {
            Self::Inicio => ("Inicio", Icon::House),
            Self::Campos => ("Campos", Icon::TextCursorInput),
            Self::Botones => ("Botones", Icon::MousePointerClick),
            Self::Seleccion => ("Selección", Icon::ToggleLeft),
            Self::Contenedores => ("Contenedores", Icon::LayoutGrid),
            Self::Badges => ("Badges y alertas", Icon::Tag),
            Self::Multimedia => ("Multimedia", Icon::Image),
            Self::Tabla => ("Tabla", Icon::Table),
            Self::Mapa => ("Mapa", Icon::Map),
            Self::Graficos => ("Gráficos", Icon::LayoutDashboard),
            Self::Documento => ("Documento", Icon::FileText),
            Self::Dialogos => ("Diálogos y avisos", Icon::MessageSquare),
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
    IrA(Seccion),
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
    TogglePermiso(Option<usize>),
    SetPais(usize),
    SetAlineacion(usize),
    SetPestana(usize),
    /// The toolbar search, which filters the table.
    Query(String),
    /// Sort, filter, hide columns or expand rows of the table.
    Tabla(ViewChange),
    /// Recompute the table's rows after a pause in a filter's typing.
    RefrescarTabla,
    /// A client of the table, by its index in `clientes`.
    SelectRow(usize),
    /// The «Ver» action of a client's row.
    VerCliente(usize),
    /// Ask where to save the rows the table shows as an Excel file.
    ExportarTabla,
    /// Where the save panel said (`None`: cancelled).
    GuardarExcel(Option<PathBuf>),
    /// The Excel file was written (its name and rows) or not.
    TablaExportada(Result<(String, usize), String>),
    OpenDialog(Dialogo),
    CloseDialog,
    /// The dialog's default button.
    ConfirmDialog,
    ShowToast(VarianteAviso),
    CloseToast(u64),
    /// Opens the file panel for a field.
    ElegirImagen(Campo),
    /// The panel's answer, or a file dropped on the field.
    ImagenElegida(Campo, Option<PathBuf>),
    /// Loads the avatar from a URL; empty, back to the icon.
    CargarUrl(String),
    /// A gesture on the map ended there.
    MapaMovido(MapView),
    /// Shows `vista` on the map, gliding there when `animar`.
    FijarVista {
        vista: MapView,
        animar: bool,
    },
    /// Selects the sector with this code and frames it.
    SeleccionarArea(String),
    /// Moves the map to show all of the box.
    Encuadrar(LatLonBounds),
    ToggleAreas,
    /// The selected bar of the chart, by month index.
    Mes(usize),
    /// Documento: contenido tipado / formateado.
    Documento(DocumentChange),
    /// Documento: comando de toolbar / Action / ribbon.
    DocCmd(DocumentCmd),
    /// Documento: cambiar pestaña del ribbon.
    #[cfg_attr(not(feature = "document"), allow(dead_code))]
    DocRibbonTab(String),
    /// Documento: panel Abrir `.docx`.
    DocAbrir,
    /// Ruta elegida al abrir (`None`: cancelado).
    DocAbrirElegido(Option<PathBuf>),
    /// Bytes leídos del `.docx` (o error).
    DocBytesLeidos(Result<Vec<u8>, String>),
    /// Documento: panel Guardar `.docx`.
    DocGuardar,
    DocGuardarEn(Option<PathBuf>),
    /// Documento: panel Exportar PDF.
    DocExportarPdf,
    DocExportarPdfEn(Option<PathBuf>),
    /// Escritura / export terminó.
    DocArchivoListo(Result<String, String>),
    /// What the color field of the selected sector holds.
    ColorSector(String),
    /// Starts editing the outline of the sector with this code.
    EditarArea(String),
    /// A change to the draft of the sector being edited (`Finish` keeps
    /// it if valid, `Cancel` drops it).
    Edicion(PolygonEdit),
    /// Replaces the outline of a sector (an agent's edit, already valid).
    GuardarGeometria {
        code: String,
        anillo: Vec<LatLon>,
    },
    /// Botones: wait [`ESPERA`] on a task; the button that sent it is busy
    /// meanwhile.
    Esperar(&'static str),
    /// The wait of that name ended.
    EsperaLista(&'static str),
    /// Save, then reload: two chained tasks.
    GuardarYRecargar,
    Recargar,
    /// A wait whose button stays usable.
    Sincronizar,
    Sincronizado,
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
    fn encuadrar(&mut self, bounds: LatLonBounds) {
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
        let serial = self.encuadre.map_or(1, |(_, serial)| serial + 1);
        self.encuadre = Some((bounds, serial));
        self.animar_vista = true;
    }

    fn sector(&self, code: &str) -> Option<&Sector> {
        self.sectores.iter().find(|sector| sector.code == code)
    }

    /// Select sector `code`, show the sectors and frame it with a margin.
    fn seleccionar(&mut self, code: &str) {
        let Some(bounds) = self.sector(code).map(|s| s.capa.bounds()) else {
            return;
        };
        self.area = Some(code.to_owned());
        self.mostrar_areas = true;
        self.encuadrar(bounds.padded(MARGEN_SECTOR));
    }

    /// Give sector `code` the outline `anillo`.
    fn guardar_anillo(&mut self, code: &str, anillo: &[LatLon]) {
        if let Some(sector) = self.sectores.iter_mut().find(|s| s.code == code) {
            sector.con_anillo(anillo);
        }
    }

    fn editar(&mut self, edit: PolygonEdit) {
        let Some((code, draft)) = &mut self.edicion else {
            return;
        };
        match edit {
            PolygonEdit::Finish if draft.is_valid() => {
                let (code, anillo) = (code.clone(), draft.ring().to_vec());
                self.edicion = None;
                self.guardar_anillo(&code, &anillo);
            }
            PolygonEdit::Cancel => self.edicion = None,
            edit => {
                draft.apply(edit);
            }
        }
    }
}

/// Saves the rows the client table shows (search, filters, sort; no
/// hidden columns) as an Excel file where the person chooses.
#[derive(Clone, Deserialize, JsonSchema)]
struct ExportarTabla;

impl Action for ExportarTabla {
    type App = Demo;
    const ID: &'static str = "tabla.exportar";
    const TITLE: &'static str = "Exportar a Excel";
    fn description() -> &'static str {
        "Guarda en un Excel las filas que muestra la tabla de clientes, con su búsqueda, \
         filtros y orden"
    }
    fn available(demo: &Demo) -> Availability {
        if demo.exportando {
            Availability::disabled("Ya se está exportando")
        } else if demo.tabla.is_empty() {
            Availability::disabled("No hay filas que exportar")
        } else {
            Availability::Enabled
        }
    }
    fn into_msg(self) -> Msg {
        Msg::ExportarTabla
    }
}

/// Waits a moment on a task: its buttons, menu item and palette row are
/// busy until it ends.
#[derive(Clone, Deserialize, JsonSchema)]
struct SimularEspera;

impl Action for SimularEspera {
    type App = Demo;
    const ID: &'static str = "botones.simular_espera";
    const TITLE: &'static str = "Simular espera";
    fn description() -> &'static str {
        "Espera un momento en una tarea; mientras, el comando está en curso"
    }
    fn into_msg(self) -> Msg {
        Msg::Esperar("comando")
    }
}

#[derive(Clone, Deserialize, JsonSchema)]
struct Incrementar;

impl Action for Incrementar {
    type App = Demo;
    const ID: &'static str = "contador.incrementar";
    const TITLE: &'static str = "Incrementar";
    fn description() -> &'static str {
        "Suma un clic al contador"
    }
    fn into_msg(self) -> Msg {
        Msg::Increment
    }
}

/// Adds `cantidad` clicks at once; only scripts and agents call it.
#[derive(Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct Sumar {
    #[schemars(description = "Clics que se suman")]
    cantidad: u32,
}

impl Action for Sumar {
    type App = Demo;
    const ID: &'static str = "contador.sumar";
    const TITLE: &'static str = "Sumar";
    fn description() -> &'static str {
        "Suma varios clics al contador"
    }
    fn into_msg(self) -> Msg {
        Msg::Add(self.cantidad)
    }
}

#[derive(Clone, Deserialize, JsonSchema)]
struct Reiniciar;

impl Action for Reiniciar {
    type App = Demo;
    const ID: &'static str = "contador.reiniciar";
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
struct Enviar;

impl Action for Enviar {
    type App = Demo;
    const ID: &'static str = "form.enviar";
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
struct Limpiar;

impl Action for Limpiar {
    type App = Demo;
    const ID: &'static str = "form.limpiar";
    const TITLE: &'static str = "Limpiar";
    fn description() -> &'static str {
        "Borra el nombre, el correo, el rol y los términos"
    }
    fn into_msg(self) -> Msg {
        Msg::Clear
    }
}

#[derive(Clone, Deserialize, JsonSchema)]
struct AlternarContrasena;

impl Action for AlternarContrasena {
    type App = Demo;
    const ID: &'static str = "form.alternar_contrasena";
    const TITLE: &'static str = "Mostrar u ocultar contraseña";
    fn into_msg(self) -> Msg {
        Msg::TogglePassword
    }
}

/// Shows a section, as a click on the sidebar does.
#[derive(Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct IrA {
    #[schemars(description = "Sección que se muestra")]
    seccion: Seccion,
}

impl Action for IrA {
    type App = Demo;
    const ID: &'static str = "ui.ir_a";
    const TITLE: &'static str = "Ir a sección";
    fn description() -> &'static str {
        "Muestra una sección de la demo, como un clic en la barra lateral"
    }
    fn into_msg(self) -> Msg {
        Msg::IrA(self.seccion)
    }
}

#[derive(Clone, Deserialize, JsonSchema)]
struct AlternarSidebar;

impl Action for AlternarSidebar {
    type App = Demo;
    const ID: &'static str = "ui.alternar_sidebar";
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
struct MostrarAviso {
    #[schemars(description = "Variante del aviso")]
    variante: VarianteAviso,
}

impl Action for MostrarAviso {
    type App = Demo;
    const ID: &'static str = "avisos.mostrar";
    const TITLE: &'static str = "Mostrar aviso";
    fn description() -> &'static str {
        "Muestra un aviso (toast) que se cierra solo a los 4 segundos"
    }
    fn into_msg(self) -> Msg {
        Msg::ShowToast(self.variante)
    }
}

/// Shows the image at a URL in the avatar of «Multimedia».
#[derive(Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct CargarUrl {
    #[schemars(
        description = "Dirección http(s) de una imagen png, jpg o webp; vacía, \
                              vuelve al ícono"
    )]
    url: String,
}

impl Action for CargarUrl {
    type App = Demo;
    const ID: &'static str = "multimedia.cargar_url";
    const TITLE: &'static str = "Cargar imagen desde URL";
    fn description() -> &'static str {
        "Descarga una imagen y la muestra en el avatar de la sección Multimedia"
    }
    fn into_msg(self) -> Msg {
        Msg::CargarUrl(self.url)
    }
}

/// Moves the map of «Mapa», as a drag or the zoom buttons would.
#[derive(Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct FijarVista {
    #[schemars(description = "Centro del mapa (grados)")]
    center: LatLon,
    #[schemars(description = "Zoom de 0 (el mundo entero) a 20 (una calle)")]
    zoom: f64,
    #[schemars(description = "Deslizarse hasta la vista (por omisión) o saltar")]
    #[serde(default)]
    animate: Option<bool>,
}

impl Action for FijarVista {
    type App = Demo;
    const ID: &'static str = "mapa.set_view";
    const TITLE: &'static str = "Mover el mapa";
    fn description() -> &'static str {
        "Centra el mapa de la sección Mapa en un punto, con el zoom dado"
    }
    fn into_msg(self) -> Msg {
        Msg::FijarVista {
            vista: MapView::new(self.center, self.zoom),
            animar: self.animate.unwrap_or(true),
        }
    }
}


/// Documento: negrita (WordCraft `format.bold`).
#[derive(Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct DocNegrita;

impl Action for DocNegrita {
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
struct DocCursiva;

impl Action for DocCursiva {
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
struct DocLista;

impl Action for DocLista {
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
struct DocDeshacer;

impl Action for DocDeshacer {
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
struct DocRehacer;

impl Action for DocRehacer {
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
struct DocTabla;

impl Action for DocTabla {
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
struct DocImagen;

impl Action for DocImagen {
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
struct DocAbrir;

impl Action for DocAbrir {
    type App = Demo;
    const ID: &'static str = "documento.abrir";
    const TITLE: &'static str = "Abrir documento";
    fn description() -> &'static str {
        "Abre un archivo .docx en el editor de Documento (WordCraft)"
    }
    fn available(demo: &Demo) -> Availability {
        if demo.doc_archivo_busy {
            Availability::disabled("Ya hay una operación de archivo en curso")
        } else {
            Availability::Enabled
        }
    }
    fn into_msg(self) -> Msg {
        Msg::DocAbrir
    }
}

#[derive(Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct DocGuardar;

impl Action for DocGuardar {
    type App = Demo;
    const ID: &'static str = "documento.guardar";
    const TITLE: &'static str = "Guardar documento";
    fn description() -> &'static str {
        "Guarda el documento como .docx donde elijas"
    }
    fn available(demo: &Demo) -> Availability {
        if demo.doc_archivo_busy {
            Availability::disabled("Ya hay una operación de archivo en curso")
        } else {
            Availability::Enabled
        }
    }
    fn into_msg(self) -> Msg {
        Msg::DocGuardar
    }
}

#[derive(Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct DocExportarPdf;

impl Action for DocExportarPdf {
    type App = Demo;
    const ID: &'static str = "documento.exportar_pdf";
    const TITLE: &'static str = "Exportar PDF";
    fn description() -> &'static str {
        "Exporta el documento a PDF (WordCraft / craft-pdf) donde elijas"
    }
    fn available(demo: &Demo) -> Availability {
        if demo.doc_archivo_busy {
            Availability::disabled("Ya hay una operación de archivo en curso")
        } else {
            Availability::Enabled
        }
    }
    fn into_msg(self) -> Msg {
        Msg::DocExportarPdf
    }
}

/// Frames a box on the map of «Mapa».
#[derive(Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct Encuadrar {
    #[schemars(description = "Caja a mostrar entera (grados)")]
    bounds: LatLonBounds,
}

impl Action for Encuadrar {
    type App = Demo;
    const ID: &'static str = "mapa.fit_bounds";
    const TITLE: &'static str = "Encuadrar el mapa";
    fn description() -> &'static str {
        "Mueve el mapa de la sección Mapa hasta mostrar entera una caja de latitud y longitud"
    }
    fn into_msg(self) -> Msg {
        Msg::Encuadrar(self.bounds)
    }
}

/// Selects a DSP sector of «Mapa», as a click on it or on the list would.
#[derive(Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct SeleccionarArea {
    #[schemars(
        description = "Código del sector: AOP26000001 (Plaza de Armas), AOP26000002 \
                       (Catillo), AOP26000003 (Villa San Pedro), AOP26000004 (Palma Rosa) \
                       o AOP26000005 (Terminal de Buses)"
    )]
    code: String,
}

impl Action for SeleccionarArea {
    type App = Demo;
    const ID: &'static str = "areas.seleccionar";
    const TITLE: &'static str = "Seleccionar sector";
    fn description() -> &'static str {
        "Selecciona un sector de DSP en el mapa de la sección Mapa y lo encuadra con un 20 % \
         de margen; un código desconocido, o un sector en edición, no cambia nada"
    }
    fn into_msg(self) -> Msg {
        Msg::SeleccionarArea(self.code)
    }
}

/// The outline an agent proposes for a sector, before it is checked.
#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct GeometriaPropuesta {
    #[schemars(description = "Código del sector, como en areas.seleccionar")]
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
#[serde(try_from = "GeometriaPropuesta")]
struct EditarGeometria {
    code: String,
    anillo: Vec<LatLon>,
}

impl TryFrom<GeometriaPropuesta> for EditarGeometria {
    type Error = String;

    fn try_from(propuesta: GeometriaPropuesta) -> Result<Self, String> {
        if sector_inicial(&propuesta.code).is_none() {
            return Err(format!("no hay un sector {}", propuesta.code));
        }
        let draft = PolygonDraft::from_ring(
            propuesta
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
            code: propuesta.code,
            anillo: draft.ring().to_vec(),
        })
    }
}

impl Action for EditarGeometria {
    type App = Demo;
    const ID: &'static str = "areas.editar_geometria";
    const TITLE: &'static str = "Editar la geometría de un sector";
    fn description() -> &'static str {
        "Reemplaza el polígono de un sector de DSP en el mapa de la sección Mapa, lo selecciona y \
         lo encuadra; una geometría inválida es un error y no cambia nada"
    }
    fn into_msg(self) -> Msg {
        Msg::GuardarGeometria {
            code: self.code,
            anillo: self.anillo,
        }
    }
}

/// What the map of «Mapa» shows, as of the last gesture.
#[derive(Deserialize, JsonSchema)]
struct VistaDelMapa;

impl Query for VistaDelMapa {
    type App = Demo;
    type Output = MapView;
    const ID: &'static str = "mapa.get_view";
    const TITLE: &'static str = "Vista del mapa";
    fn run(demo: &Demo, _: Self) -> Result<MapView, CommandError> {
        Ok(demo.vista)
    }
}

/// The last name sent, if any.
#[derive(Deserialize, JsonSchema)]
struct Saludo;

impl Query for Saludo {
    type App = Demo;
    type Output = Option<String>;
    const ID: &'static str = "form.ultimo_envio";
    const TITLE: &'static str = "Último envío";
    fn run(demo: &Demo, _: Self) -> Result<Option<String>, CommandError> {
        Ok(demo.submissions.last().cloned())
    }
}

impl App for Demo {
    type Msg = Msg;

    fn commands(registry: &mut CommandRegistry<Self>) {
        registry
            .action::<Incrementar>()
            .shortcut(Shortcut::cmd(Key::Char('i')))
            .menu("Contador")
            .expose();
        registry.action::<Sumar>().expose();
        registry
            .action::<Reiniciar>()
            .shortcut(Shortcut::cmd(Key::Char('0')))
            .menu("Contador")
            .expose();
        registry
            .action::<Enviar>()
            .menu("Formulario")
            .returns::<Saludo>()
            .expose();
        registry.action::<Limpiar>().menu("Formulario").expose();
        // Showing the password is the user's business, not an agent's.
        registry.action::<AlternarContrasena>().menu("Formulario");
        registry.query::<Saludo>().expose();
        // It takes a section, so it has no menu item nor shortcut.
        registry.action::<IrA>().expose();
        registry
            .action::<AlternarSidebar>()
            .shortcut(Shortcut::ctrl_cmd(Key::Char('s')))
            .menu("Ver")
            .expose();
        // It takes a variant: no menu item, as `ui.ir_a`.
        registry.action::<MostrarAviso>().expose();
        registry.action::<CargarUrl>().expose();
        registry.action::<FijarVista>().expose();
        registry.action::<Encuadrar>().expose();
        registry.action::<SeleccionarArea>().expose();
        registry.action::<EditarGeometria>().expose();
        registry.query::<VistaDelMapa>().expose();
        registry.action::<SimularEspera>().menu("Botones").expose();
        registry.action::<ExportarTabla>().menu("Archivo").expose();
        registry.action::<DocNegrita>().menu("Documento").expose();
        registry.action::<DocCursiva>().menu("Documento").expose();
        registry.action::<DocLista>().menu("Documento").expose();
        registry.action::<DocTabla>().menu("Documento").expose();
        registry.action::<DocImagen>().menu("Documento").expose();
        registry.action::<DocAbrir>().menu("Documento").expose();
        registry.action::<DocGuardar>().menu("Documento").expose();
        registry.action::<DocExportarPdf>().menu("Documento").expose();
        registry.action::<DocDeshacer>().menu("Documento").expose();
        registry.action::<DocRehacer>().menu("Documento").expose();
    }

    fn resized(&self, size: Size) -> Option<Msg> {
        (!self.sized).then_some(Msg::InitialWidth {
            narrow: size.width < 640.0,
        })
    }

    fn dev_state(&self) -> Option<serde_json::Value> {
        serde_json::to_value(EstadoDev {
            seccion: Some(self.seccion),
            sidebar_collapsed: Some(self.sidebar_collapsed),
            area: self.area.clone(),
        })
        .ok()
    }

    fn restore_dev_state(&mut self, state: serde_json::Value) {
        let Ok(estado) = serde_json::from_value::<EstadoDev>(state) else {
            return;
        };
        if let Some(seccion) = estado.seccion {
            self.seccion = seccion;
        }
        if let Some(collapsed) = estado.sidebar_collapsed {
            self.sidebar_collapsed = collapsed;
            // The first window size must not undo the restored sidebar.
            self.sized = true;
        }
        if let Some(code) = estado.area {
            self.seleccionar(&code);
            self.animar_vista = false;
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
        match msg {
            Msg::ShowToast(variante) => {
                let (_, title, message) = variante.texts();
                let mut toast = Toast::new(0, variante.alert(), title).message(message);
                if variante == VarianteAviso::Error {
                    toast = toast.action("Reintentar", Msg::ShowToast(VarianteAviso::Exito));
                }
                return self.show_toast(toast);
            }
            Msg::CloseToast(id) => {
                self.toasts.retain(|toast| toast.id != id);
                return Effects::cancel(toast_key(id));
            }
            Msg::ConfirmDialog => {
                let toast = match self.dialogo.take() {
                    Some(Dialogo::Guardar) => {
                        let (_, title, message) = VarianteAviso::Exito.texts();
                        Toast::new(0, AlertVariant::Success, title).message(message)
                    }
                    Some(Dialogo::Eliminar) => {
                        Toast::new(0, AlertVariant::Info, "Borrador eliminado")
                            .message("Se movió a la papelera")
                    }
                    None => return Effects::none(),
                };
                return self.show_toast(toast);
            }
            Msg::Esperar(name) => {
                let mut effect = Effects::perform(
                    || std::thread::sleep(ESPERA),
                    move |()| Msg::EsperaLista(name),
                );
                if let Some(invocation) = current_invocation() {
                    effect = effect.completes(invocation);
                }
                return effect;
            }
            Msg::EsperaLista(name) => self.espera = Some(name),
            Msg::GuardarYRecargar => {
                return Effects::perform(|| std::thread::sleep(ESPERA / 2), |()| Msg::Recargar);
            }
            Msg::Recargar => {
                return Effects::perform(
                    || std::thread::sleep(ESPERA / 2),
                    |()| Msg::EsperaLista("guardar y recargar"),
                );
            }
            Msg::Sincronizar => {
                self.sincronizando += 1;
                return Effects::perform(|| std::thread::sleep(ESPERA), |()| Msg::Sincronizado);
            }
            Msg::Sincronizado => self.sincronizando = self.sincronizando.saturating_sub(1),
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
            Msg::ElegirImagen(campo) => {
                let title = match campo {
                    Campo::Foto => "Elegir foto de perfil",
                    Campo::Portada => "Elegir portada",
                    Campo::Logo => "Elegir logo",
                };
                return Effects::pick_file(PickFile::images(title), move |path| {
                    Msg::ImagenElegida(campo, path)
                });
            }
            Msg::ImagenElegida(campo, Some(path)) => match campo {
                Campo::Foto => self.foto.set(path),
                Campo::Portada => self.portada.set(path),
                Campo::Logo => self.logo.set(path),
            },
            Msg::ImagenElegida(_, None) => {}
            Msg::CargarUrl(url) => {
                let url = url.trim().to_owned();
                self.avatar_url = (!url.is_empty()).then(|| url.clone());
                self.url = url;
            }
            Msg::TogglePassword => self.show_password = !self.show_password,
            Msg::ToggleNotifications => self.notifications = !self.notifications,
            Msg::SetRole(index) => self.role = Some(index),
            Msg::ToggleTerms => self.terms = !self.terms,
            Msg::IrA(seccion) => self.seccion = seccion,
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
            Msg::TogglePermiso(Some(index)) => self.permisos[index] = !self.permisos[index],
            Msg::TogglePermiso(None) => {
                let all = !self.permisos.iter().all(|on| *on);
                self.permisos = [all; 3];
            }
            Msg::SetPais(index) => self.pais = Some(index),
            Msg::SetAlineacion(index) => self.alineacion = index,
            Msg::SetPestana(index) => self.pestana = index,
            Msg::Query(query) => {
                self.tabla.query.clone_from(&query);
                self.query = query;
                self.tabla.refresh(&self.clientes, &columnas());
            }
            Msg::Tabla(change) => {
                let typing = matches!(change, ViewChange::Filter(..));
                self.tabla.apply(change);
                if typing {
                    return Effects::after(PAUSA_FILTRO, Msg::RefrescarTabla).keyed("filtro");
                }
                self.tabla.refresh(&self.clientes, &columnas());
            }
            Msg::RefrescarTabla => self.tabla.refresh(&self.clientes, &columnas()),
            Msg::SelectRow(index) => self.cliente = Some(index),
            Msg::VerCliente(index) => {
                let Some(cliente) = self.clientes.get(index) else {
                    return Effects::none();
                };
                let toast = Toast::new(0, AlertVariant::Info, cliente.nombre.as_str()).message(
                    format!("Cliente N.º {} de {}", cliente.numero, cliente.ciudad),
                );
                return self.show_toast(toast);
            }
            Msg::ExportarTabla => {
                if self.exportando || self.tabla.is_empty() {
                    return Effects::none();
                }
                let request = SaveFile::new("Exportar a Excel", "clientes.xlsx").types(&["xlsx"]);
                return Effects::save_file(request, Msg::GuardarExcel);
            }
            Msg::GuardarExcel(None) => {}
            Msg::GuardarExcel(Some(path)) => {
                self.exportando = true;
                let sheet = self.tabla.export(&self.clientes, &columnas());
                return Effects::perform(
                    move || {
                        crate::xlsx::write(&sheet, "Clientes", &path)?;
                        let name = path.file_name().map_or_else(
                            || path.display().to_string(),
                            |name| name.to_string_lossy().into_owned(),
                        );
                        Ok((name, sheet.rows.len()))
                    },
                    Msg::TablaExportada,
                );
            }
            Msg::TablaExportada(result) => {
                self.exportando = false;
                let toast = match result {
                    Ok((name, rows)) => Toast::new(0, AlertVariant::Success, "Excel exportado")
                        .message(format!("{rows} filas en {name}")),
                    Err(error) => {
                        Toast::new(0, AlertVariant::Error, "No se exportó").message(error)
                    }
                };
                return self.show_toast(toast);
            }
            Msg::OpenDialog(dialogo) => self.dialogo = Some(dialogo),
            Msg::CloseDialog => self.dialogo = None,
            Msg::MapaMovido(vista) => self.vista = vista,
            Msg::FijarVista { vista, animar } => {
                self.vista = vista.clamped();
                self.animar_vista = animar;
            }
            Msg::SeleccionarArea(code) => {
                if self.edicion.is_none() {
                    self.seleccionar(&code);
                }
            }
            Msg::Encuadrar(bounds) => self.encuadrar(bounds),
            Msg::ToggleAreas => self.mostrar_areas = !self.mostrar_areas,
            Msg::Mes(index) => self.mes = Some(index),
            Msg::Documento(change) => {
                self.doc_markdown = change.markdown;
                #[cfg(feature = "document")]
                if let Some(export) = change.export
                    && let Some((path, kind)) = self.doc_pending_export.take()
                {
                    self.doc_archivo_busy = true;
                    let doc = (*export.0).clone();
                    return Effects::perform(
                        move || write_document_file(doc, path, kind),
                        Msg::DocArchivoListo,
                    );
                }
                #[cfg(not(feature = "document"))]
                if self.doc_pending_export.take().is_some() {
                    return Effects::perform(
                        || Err("Compila con --features document".into()),
                        Msg::DocArchivoListo,
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
            Msg::DocAbrir => {
                if self.doc_archivo_busy {
                    return Effects::none();
                }
                let request = PickFile::new("Abrir documento").types(&["docx"]);
                return Effects::pick_file(request, Msg::DocAbrirElegido);
            }
            Msg::DocAbrirElegido(None) => {}
            Msg::DocAbrirElegido(Some(path)) => {
                self.doc_archivo_busy = true;
                return Effects::perform(
                    move || std::fs::read(&path).map_err(|e| e.to_string()),
                    Msg::DocBytesLeidos,
                );
            }
            Msg::DocBytesLeidos(result) => {
                self.doc_archivo_busy = false;
                match result {
                    Ok(bytes) => {
                        self.doc_load_serial = self.doc_load_serial.saturating_add(1);
                        self.doc_load = Some((self.doc_load_serial, bytes.into()));
                        let toast = Toast::new(0, AlertVariant::Success, "Documento abierto")
                            .message("Se cargó el archivo .docx");
                        return self.show_toast(toast);
                    }
                    Err(error) => {
                        let toast = Toast::new(0, AlertVariant::Error, "No se abrió")
                            .message(error);
                        return self.show_toast(toast);
                    }
                }
            }
            Msg::DocGuardar => {
                if self.doc_archivo_busy {
                    return Effects::none();
                }
                let request =
                    SaveFile::new("Guardar documento", "informe.docx").types(&["docx"]);
                return Effects::save_file(request, Msg::DocGuardarEn);
            }
            Msg::DocGuardarEn(None) => {}
            Msg::DocGuardarEn(Some(path)) => {
                self.doc_pending_export = Some((path, DocExportKind::Docx));
                self.doc_cmd_serial = self.doc_cmd_serial.saturating_add(1);
                self.doc_cmd = Some((DocumentCmd::TakeDocument, self.doc_cmd_serial));
            }
            Msg::DocExportarPdf => {
                if self.doc_archivo_busy {
                    return Effects::none();
                }
                let request =
                    SaveFile::new("Exportar PDF", "informe.pdf").types(&["pdf"]);
                return Effects::save_file(request, Msg::DocExportarPdfEn);
            }
            Msg::DocExportarPdfEn(None) => {}
            Msg::DocExportarPdfEn(Some(path)) => {
                self.doc_pending_export = Some((path, DocExportKind::Pdf));
                self.doc_cmd_serial = self.doc_cmd_serial.saturating_add(1);
                self.doc_cmd = Some((DocumentCmd::TakeDocument, self.doc_cmd_serial));
            }
            Msg::DocArchivoListo(result) => {
                self.doc_archivo_busy = false;
                let toast = match result {
                    Ok(name) => Toast::new(0, AlertVariant::Success, "Archivo listo")
                        .message(name),
                    Err(error) => {
                        Toast::new(0, AlertVariant::Error, "No se pudo escribir").message(error)
                    }
                };
                return self.show_toast(toast);
            }
            Msg::ColorSector(color) => {
                let code = self.area.clone();
                if let Some(sector) = self
                    .sectores
                    .iter_mut()
                    .find(|s| Some(&s.code) == code.as_ref())
                {
                    sector.color = color;
                }
            }
            Msg::EditarArea(code) => {
                if self.edicion.is_none()
                    && let Some(anillo) = self.sector(&code).map(Sector::anillo)
                    && let Ok(draft) = PolygonDraft::from_ring(anillo)
                {
                    self.seleccionar(&code);
                    self.edicion = Some((code, draft));
                }
            }
            Msg::Edicion(edit) => self.editar(edit),
            Msg::GuardarGeometria { code, anillo } => {
                if self
                    .edicion
                    .as_ref()
                    .is_some_and(|(editing, _)| *editing == code)
                {
                    self.edicion = None;
                }
                self.guardar_anillo(&code, &anillo);
                if self.edicion.is_none() {
                    self.seleccionar(&code);
                }
            }
        }
        Effects::none()
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
            .on_command(AlternarSidebar)
            .test_id("barra.sidebar"),
            toolbar_title("Kai UI Desktop", Some(&version)),
            toolbar_spacer().test_id("barra.espacio"),
            toolbar_item(Icon::Plus, "Sumar clic")
                .on_command(Incrementar)
                .test_id("barra.sumar"),
            toolbar_separator().test_id("barra.separador"),
            toolbar_search(
                search_field(&self.query, Msg::Query)
                    .placeholder("Filtrar clientes")
                    .label("Filtrar clientes")
                    .on_submit(Msg::IrA(Seccion::Tabla))
                    .test_id("barra.buscar")
                    .build(),
            ),
        ]);

        let alert_banner = alert(
            AlertVariant::Info,
            "Componentes sincronizados con la biblioteca web de Kai (React/TypeScript)",
        );

        let counter = card()
            .class("w-full max-w-3xl")
            .test_id("contador")
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
                    button("Reiniciar", ButtonVariant::Outlined).on_command(Reiniciar),
                    button("Incrementar", ButtonVariant::Primary).on_command(Incrementar),
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
                div()
                    .class("grid grid-cols-1 sm:grid-cols-2 gap-4")
                    .children([
                        // Return here activates «Enviar», the default button.
                        TextField::new("Nombre", &self.name)
                            .test_id("form.nombre")
                            .start_icon(Icon::User)
                            .on_input(Msg::NameChanged)
                            .build(),
                        TextField::new("Correo", &self.email)
                            .test_id("form.correo")
                            .start_icon(Icon::Search)
                            .on_input(Msg::EmailChanged)
                            .on_submit(Msg::Submit)
                            .build(),
                        select(&ROLES, self.role, "Elegir rol", Msg::SetRole)
                            .label("Rol")
                            .test_id("form.rol"),
                        checkbox(
                            "Acepto los términos",
                            if self.terms {
                                ControlState::On
                            } else {
                                ControlState::Off
                            },
                            Msg::ToggleTerms,
                        )
                        .test_id("form.terminos"),
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
                button("Limpiar", ButtonVariant::Text).on_command(Limpiar),
                button("Enviar", ButtonVariant::Primary)
                    .on_command(Enviar)
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
                        .test_id("botones.secondary")
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

        let cards = match self.seccion {
            Seccion::Inicio => vec![
                self.background_panel(),
                div().class("w-full max-w-3xl").child(alert_banner),
                counter,
                form,
                stats,
            ],
            Seccion::Campos => vec![self.fields(), self.more_fields()],
            Seccion::Botones => vec![
                buttons_section,
                self.waiting_buttons(),
                self.appkit_buttons(),
                self.control_sizes(),
            ],
            Seccion::Seleccion => vec![self.selection()],
            Seccion::Contenedores => self.containers(),
            Seccion::Badges => vec![badges_and_alerts],
            Seccion::Multimedia => vec![self.avatars(), self.image_fields()],
            Seccion::Tabla => vec![self.client_table(), self.simple_list()],
            Seccion::Mapa => vec![self.mapa()],
            Seccion::Graficos => self.graficos(),
            Seccion::Documento => vec![self.documento()],
            Seccion::Dialogos => vec![self.dialogs_and_toasts()],
        };

        let content = if self.seccion == Seccion::Documento {
            // Pages metaphor: ribbon + archivo + full-height canvas (no card / scroll shell).
            div()
                .class("flex-1 flex flex-col min-h-0 min-w-0 bg-under-page")
                .key(self.seccion.key())
                .test_id("contenido")
                .children(cards)
        } else {
            // Keyed by section: each one keeps its own scroll.
            div()
                .class("flex-1 overflow-y-auto")
                .key(self.seccion.key())
                .test_id("contenido")
                .child(
                    div()
                        .class("flex flex-col items-center gap-6 p-6")
                        .children(cards),
                )
        };
        let mut layers = vec![
            header,
            div()
                .class("flex-1 flex min-h-0")
                .children([self.sidebar(), content]),
            toast_stack(&self.toasts, Msg::CloseToast),
        ];
        if let Some(dialogo) = self.dialogo {
            layers.push(self.dialog(dialogo));
        }
        div()
            // `neutral` is systemGray in AppKit: use the window background.
            .class("relative w-full h-full flex flex-col bg-neutral appkit:bg-window")
            .role(Role::Main)
            .children(layers)
    }
}

impl Demo {
    fn sidebar(&self) -> Element<Msg> {
        let mut inicio = Seccion::Inicio.item();
        if !self.submissions.is_empty() {
            inicio = inicio.badge(self.submissions.len().to_string());
        }
        let groups = [
            SidebarGroup::titled("Demo", [inicio]),
            SidebarGroup::titled(
                "Componentes",
                [
                    Seccion::Campos,
                    Seccion::Botones,
                    Seccion::Seleccion,
                    Seccion::Contenedores,
                    Seccion::Badges,
                    Seccion::Multimedia,
                ]
                .map(Seccion::item),
            ),
            SidebarGroup::titled(
                "Datos",
                [
                    Seccion::Tabla.item(),
                    Seccion::Mapa.item(),
                    Seccion::Graficos.item(),
                    Seccion::Documento.item(),
                ],
            ),
            SidebarGroup::titled("Superposiciones", [Seccion::Dialogos.item()]),
        ];
        Sidebar::new(&groups, self.seccion.index(), |index| {
            Msg::IrA(Seccion::ALL[index])
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
        let status = match (self.espera, self.sincronizando) {
            (_, 1..) => format!("Sincronizando ({})…", self.sincronizando),
            (Some(name), _) => format!("Terminó: {name}"),
            (None, _) => "Pulsa un botón: espera 1,5 s".to_owned(),
        };
        card()
            .class("w-full max-w-3xl")
            .test_id("botones.espera")
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
                                .on_click(Msg::Esperar(name))
                                .test_id(format!("espera.{}", name.to_lowercase()))
                        })),
                    div().class("flex flex-wrap items-center gap-3").children([
                        Button::new("Guardar y recargar")
                            .accent(true)
                            .on_click(Msg::GuardarYRecargar)
                            .build()
                            .test_id("espera.recargar"),
                        Button::new("Sincronizar en segundo plano")
                            .busy_feedback(false)
                            .on_click(Msg::Sincronizar)
                            .build()
                            .test_id("espera.sincronizar"),
                        button("Simular espera (comando)", ButtonVariant::Outlined)
                            .on_command(SimularEspera),
                        Button::new("Cargando")
                            .loading(true)
                            .build()
                            .test_id("espera.cargando"),
                    ]),
                    div()
                        .class("flex items-center gap-6 pt-2 text-primary")
                        .children([
                            caption("DotProgress:"),
                            DotProgress::new()
                                .size(DotSize::Sm)
                                .test_id("espera.puntos.sm")
                                .build(),
                            dot_progress().test_id("espera.puntos.md"),
                            DotProgress::new()
                                .size(DotSize::Lg)
                                .test_id("espera.puntos.lg")
                                .build(),
                        ]),
                    caption(status).role(Role::Status).test_id("espera.estado"),
                ]),
            ])
    }

    /// Buttons of the AppKit API (`Button::new`): the three bezels, accent,
    /// destructive, cancel (Escape) and disabled, and an `icon_button`.
    fn appkit_buttons(&self) -> Element<Msg> {
        let built = |button: Button<Msg>, id: &str| button.build().test_id(id);
        card()
            .class("w-full max-w-3xl")
            .test_id("botones.appkit")
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
                            "botones.push",
                        ),
                        built(
                            Button::new("Bordered")
                                .bezel(BezelStyle::Bordered)
                                .on_click(Msg::Increment),
                            "botones.bordered",
                        ),
                        built(
                            Button::new("Borderless")
                                .bezel(BezelStyle::Borderless)
                                .on_click(Msg::Increment),
                            "botones.borderless",
                        ),
                    ]),
                    div().class("flex flex-wrap items-center gap-3").children([
                        built(
                            Button::new("Acento").accent(true).on_click(Msg::Increment),
                            "botones.acento",
                        ),
                        built(
                            Button::new("Destructivo")
                                .destructive(true)
                                .on_click(Msg::Reset),
                            "botones.destructivo",
                        ),
                        built(
                            Button::new("Cancelar (Esc)")
                                .cancel_button()
                                .on_click(Msg::Reset),
                            "botones.cancelar",
                        ),
                        built(
                            Button::new("Pequeño")
                                .control_size(ControlSize::Small)
                                .disabled(true),
                            "botones.pequeno",
                        ),
                        icon_button(Icon::Settings, "Ajustes", IconButtonVariant::Secondary)
                            .on_click(Msg::Increment)
                            .test_id("botones.icono"),
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
            .test_id("campos.mas")
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
                                .test_id("campos.pin")
                                .password(self.show_pin, Msg::TogglePin)
                                .on_input(edit(Field::Pin))
                                .build(),
                            TextField::new("Usuario", &self.lookup)
                                .test_id("campos.redondeado")
                                .bezel(FieldBezel::Rounded)
                                .placeholder("Placeholder en color primario")
                                .placeholder_color("primary/60")
                                .on_input(edit(Field::Lookup))
                                .build(),
                            TextField::new("Sitio", "kai.cl")
                                .test_id("campos.adornos")
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
                                .test_id("campos.busqueda")
                                .build(),
                        ]),
                    text_area("Notas", &self.notes)
                        .test_id("campos.notas")
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
        let all = match self.permisos.iter().filter(|on| **on).count() {
            0 => ControlState::Off,
            3 => ControlState::On,
            _ => ControlState::Mixed,
        };
        let label = |content: &str| {
            text(content).class("text-sm font-medium text-foreground appkit:text-label")
        };
        let mut checks = vec![
            checkbox_with_options("Todos", all, Msg::TogglePermiso(None), false)
                .test_id("seleccion.todos"),
        ];
        for (index, name) in PERMISOS.iter().enumerate() {
            checks.push(
                checkbox_with_options(
                    name,
                    state(self.permisos[index]),
                    Msg::TogglePermiso(Some(index)),
                    false,
                )
                .class("ml-6")
                .test_id(format!("seleccion.permiso.{index}")),
            );
        }
        checks.push(
            checkbox_with_options(
                "Administrar",
                ControlState::Off,
                Msg::TogglePermiso(None),
                true,
            )
            .class("ml-6")
            .test_id("seleccion.administrar"),
        );
        card()
            .class("w-full max-w-3xl")
            .test_id("seleccion")
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
                                .test_id("seleccion.wifi"),
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
                            .test_id("seleccion.bluetooth"),
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
                                &PAISES,
                                self.pais,
                                "Elegir país",
                                Msg::SetPais,
                                false,
                            )
                            .label("País")
                            .test_id("seleccion.pais"),
                            select_with_options(&PAISES, None, "Sin región", Msg::SetPais, true)
                                .label("Región")
                                .test_id("seleccion.region"),
                        ]),
                    div().class("flex flex-wrap items-center gap-3").children([
                        label("Alineación"),
                        segmented_with_options(
                            &ALINEACIONES,
                            self.alineacion,
                            Msg::SetAlineacion,
                            false,
                        )
                        .label("Alineación")
                        .test_id("seleccion.alineacion"),
                        segmented_with_options(&["Día", "Semana"], 0, |_| Msg::Increment, true)
                            .label("Vista")
                            .test_id("seleccion.vista"),
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
                .test_id("contenedores.card")
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
                    .test_id("contenedores.grupo"),
                    group_box(
                        None,
                        [caption("Un group box sin título agrupa sin encabezado.")],
                    )
                    .test_id("contenedores.sin_titulo"),
                ]),
            Tabs::new(self.pestana, Msg::SetPestana)
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
                .test_id("contenedores.pestanas"),
            div()
                .class("grid grid-cols-3 gap-4 w-full max-w-3xl")
                .test_id("contenedores.stats")
                .children([
                    stat("Clientes", CLIENTES.to_string(), Some("en la tabla")),
                    stat("Filtrados", self.tabla.len().to_string(), None),
                    stat("Avisos", self.toasts.len().to_string(), Some("abiertos")),
                ]),
        ]
    }

    /// Tabla: ten thousand generated clients, sorted by the header and
    /// filtered by the toolbar search.
    fn client_table(&self) -> Element<Msg> {
        let summary = match self.cliente.and_then(|index| self.clientes.get(index)) {
            Some(cliente) => format!("Seleccionado: {}", cliente.nombre),
            None => "Elige una fila con un clic o con ↑ ↓".to_owned(),
        };
        let shown = if !self.tabla.is_filtered() {
            format!("{} clientes", self.tabla.len())
        } else if self.query.trim().is_empty() {
            format!("{} de {CLIENTES} filtrados", self.tabla.len())
        } else {
            format!(
                "{} de {CLIENTES} con «{}»",
                self.tabla.len(),
                self.query.trim()
            )
        };
        let filtrando = self.tabla.filtering;
        let filtrar = icon_button_sized(
            if filtrando {
                Icon::FunnelX
            } else {
                Icon::Funnel
            },
            "Filtrar",
            if self.tabla.has_filters() {
                IconButtonVariant::Primary
            } else {
                IconButtonVariant::Outlined
            },
            IconButtonSize::Sm,
        )
        .tooltip(if filtrando {
            "Quitar los filtros"
        } else {
            "Filtrar por columna"
        })
        .pressed(filtrando)
        .on_click(Msg::Tabla(ViewChange::Filtering(!filtrando)))
        .test_id("tabla.filtrar");
        let columnas_boton = column_menu(
            icon_button_sized(
                Icon::Columns3,
                "Columnas",
                IconButtonVariant::Outlined,
                IconButtonSize::Sm,
            )
            .tooltip("Mostrar u ocultar columnas")
            .test_id("tabla.columnas"),
            &columnas(),
            &self.tabla,
            Msg::Tabla,
        );
        card().class("w-full max-w-3xl").test_id("tabla").children([
            card_header(
                "Clientes",
                Some("10 000 filas; solo se construyen las visibles"),
            ),
            card_content().class("gap-3").children([
                div()
                    .class("flex items-center justify-between gap-3")
                    .children([
                        caption(shown).role(Role::Status).test_id("tabla.total"),
                        div().class("flex items-center gap-3").children([
                            caption(summary).test_id("tabla.seleccion"),
                            filtrar,
                            columnas_boton,
                            button_sized("Exportar", ButtonVariant::Outlined, ButtonSize::Sm)
                                .on_command(ExportarTabla)
                                .busy(self.exportando),
                        ]),
                    ]),
                Table::new(Rc::clone(&self.clientes), columnas(), &self.tabla)
                    .on_view(Msg::Tabla)
                    .selected(self.cliente)
                    .on_select(Msg::SelectRow)
                    .expandable(detalle_cliente, 64.0)
                    .label("Clientes")
                    .test_id("tabla.clientes")
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
            .test_id("multimedia.avatares")
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
                        .test_id("multimedia.avatar_url"),
                        div().class("flex flex-1 flex-col gap-2 min-w-0").children([
                            caption("Desde una URL (multimedia.cargar_url):"),
                            div().class("flex items-center gap-3").children([
                                TextField::new("URL de la imagen", &self.url)
                                    .placeholder("https://…/foto.png")
                                    .on_input(|value| Msg::Edit(Field::Url, value))
                                    .on_submit(Msg::CargarUrl(self.url.clone()))
                                    .test_id("multimedia.url")
                                    .build()
                                    .class("flex-1"),
                                button("Cargar", ButtonVariant::Outlined)
                                    .on_command(CargarUrl { url })
                                    .test_id("multimedia.cargar"),
                            ]),
                        ]),
                    ]),
                    row(
                        "image(src):",
                        vec![
                            image(sample_landscape())
                                .alt("Paisaje")
                                .class("w-48 rounded-md")
                                .test_id("multimedia.paisaje"),
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
        let field = |campo: Campo, variant: MediaVariant, imagen: &Imagen, id: &str| {
            MediaSlot::new(variant)
                .current(imagen.actual.clone())
                .on_pick(Msg::ElegirImagen(campo))
                .on_drop(move |path| Msg::ImagenElegida(campo, Some(path)))
                .error(imagen.error.clone())
                .test_id(id)
        };
        let labeled = |label: &str, slot: Element<Msg>| {
            div()
                .class("flex flex-col items-center gap-2 min-w-0")
                .children([caption(label), slot])
        };
        card()
            .class("w-full max-w-3xl")
            .test_id("multimedia.campos")
            .children([
                card_header(
                    "Campos de imagen",
                    Some(
                        format!(
                            "Clic para elegir o arrastra desde Finder; png, jpg o webp de \
                             hasta {MAX_IMAGEN_MB} MB"
                        )
                        .as_str(),
                    ),
                ),
                card_content().class("gap-6").children([
                    div().class("grid grid-cols-2 gap-6").children([
                        labeled(
                            "Foto de perfil",
                            field(
                                Campo::Foto,
                                MediaVariant::Avatar(AvatarSize::Md),
                                &self.foto,
                                "multimedia.foto",
                            )
                            .build(),
                        ),
                        labeled(
                            "Logo",
                            field(
                                Campo::Logo,
                                MediaVariant::Logo,
                                &self.logo,
                                "multimedia.logo",
                            )
                            .build(),
                        ),
                    ]),
                    labeled(
                        "Portada",
                        field(
                            Campo::Portada,
                            MediaVariant::Banner,
                            &self.portada,
                            "multimedia.portada",
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
                .test_id("tabla.lista")
                .class("h-48 rounded-md border border-border appkit:border-separator"),
            ),
        ])
    }

    /// Mapa: Parral with a pin on the plaza, the DSP sectors over it, a
    /// list that frames them, and where the map is.
    fn mapa(&self) -> Element<Msg> {
        let vista = self.vista;
        let seleccionada = self.area.as_deref();
        let editando = self.edicion.as_ref().map(|(code, _)| code.as_str());
        let capas = self
            .sectores
            .iter()
            .filter(|sector| self.mostrar_areas && editando != Some(sector.code.as_str()))
            .map(|sector| {
                let color = sector.color();
                sector
                    .capa
                    .clone()
                    .id(&sector.code)
                    .fill(color, 0.12)
                    .stroke(color, 2.0)
                    .label(&sector.nombre)
                    .selected(seleccionada == Some(sector.code.as_str()))
            });
        let mut mapa = map(&vista)
            .basemap(TileSource::pmtiles(PARRAL_PMTILES, MapStyle::protomaps()))
            .layers(capas)
            .animate_view(self.animar_vista)
            .on_view_change(Msg::MapaMovido);
        mapa = match &self.edicion {
            Some((code, draft)) => {
                let color = self
                    .sector(code)
                    .map_or(Color::hex(0xc45c26), Sector::color);
                mapa.polygon_editor(draft, color, Msg::Edicion)
            }
            None => mapa.on_area_click(Msg::SeleccionarArea),
        };
        let mut mapa = mapa
            .label("Mapa de Parral")
            .test_id("mapa.vista")
            .class("flex-1 min-w-0 h-96 border border-border")
            .children([marker(
                PLAZA_DE_ARMAS,
                Anchor::Bottom,
                icon(Icon::MapPin).class("size-8 text-primary"),
            )
            .alt("Plaza de Armas")]);
        if let Some((bounds, serial)) = self.encuadre {
            mapa = mapa.fit_bounds(bounds, serial);
        }
        let lista = div()
            .role(Role::List)
            .label("Sectores")
            .test_id("mapa.sectores")
            .class("flex flex-col gap-1 w-44")
            .children(self.sectores.iter().map(|sector| {
                let elegido = seleccionada == Some(sector.code.as_str());
                div()
                    .role(Role::Button)
                    .pressed(elegido)
                    .on_click(Msg::SeleccionarArea(sector.code.clone()))
                    .test_id(format!("mapa.sector.{}", sector.code))
                    .class(&format!(
                        "flex items-center gap-2 h-8 px-2 rounded-md border text-sm \
                         text-foreground cursor-pointer focus-visible:ring-2 \
                         focus-visible:ring-primary/40 {}",
                        if elegido {
                            format!("{CHOICE_ACTIVE} font-medium")
                        } else {
                            CHOICE_HOVER.to_owned()
                        }
                    ))
                    .children([
                        div().class(&format!("size-3 shrink-0 rounded-sm {}", sector.swatch())),
                        text(&sector.nombre),
                    ])
            }));
        let estado = match (
            &self.edicion,
            self.area.as_deref().and_then(|c| self.sector(c)),
        ) {
            (Some(_), Some(sector)) => format!(
                "Editando {}: arrastra los vértices, haz clic entre dos para agregar uno; \
                 Supr borra el vértice elegido y ⌘Z deshace",
                sector.nombre
            ),
            (None, Some(sector)) => format!(
                "Sector {} · {} · {}",
                sector.nombre,
                sector.code,
                hectareas(crate::maps::editor::area_m2(&sector.anillo()) / 10_000.0)
            ),
            _ => "Elige un sector en la lista o en el mapa".to_owned(),
        };
        let edicion = self.edicion.as_ref().map(|(_, draft)| {
            let cruza = draft.crossing().is_some();
            div()
                .class("flex items-center gap-3")
                .test_id("mapa.edicion")
                .children([
                    caption(format!("Superficie {}", hectareas(draft.area_ha())))
                        .role(Role::Status)
                        .test_id("mapa.superficie"),
                    if cruza {
                        caption("Los bordes se cruzan")
                            .class("text-error")
                            .role(Role::Alert)
                            .test_id("mapa.cruce")
                    } else {
                        div()
                    },
                    div().class("flex-1"),
                    button("Cancelar", ButtonVariant::Outlined)
                        .on_click(Msg::Edicion(PolygonEdit::Cancel))
                        .test_id("mapa.cancelar"),
                    button("Terminar", ButtonVariant::Primary)
                        .on_click(Msg::Edicion(PolygonEdit::Finish))
                        .disabled(!draft.is_valid())
                        .test_id("mapa.terminar"),
                ])
        });
        let color = self
            .area
            .as_deref()
            .and_then(|code| self.sector(code))
            .filter(|_| self.edicion.is_none())
            .map(|sector| {
                div().class("flex w-80").child(
                    color_field("Color del sector", &sector.color, Msg::ColorSector)
                        .palette(PALETA_DSP)
                        .allow_empty(true)
                        .test_id("mapa.color")
                        .build(),
                )
            });
        let editar = button("Editar sector", ButtonVariant::Outlined)
            .on_click(Msg::EditarArea(self.area.clone().unwrap_or_default()))
            .disabled(self.area.is_none() || self.edicion.is_some())
            .test_id("mapa.editar");
        card().class("w-full max-w-3xl").test_id("mapa").children([
            card_header(
                "Parral",
                Some("Arrastra, usa la rueda o pellizca; doble clic acerca y ⇧ doble clic aleja"),
            ),
            card_content().class("gap-3").children(
                [div().class("flex gap-3").children([
                    mapa,
                    div().class("flex flex-col gap-3 w-44").children([
                        lista,
                        editar,
                        switch_labeled(
                            "Sectores",
                            self.mostrar_areas,
                            Msg::ToggleAreas,
                            false,
                            SwitchDensity::Compact,
                        )
                        .test_id("mapa.mostrar_sectores"),
                    ]),
                ])]
                .into_iter()
                .chain(edicion)
                .chain(color)
                .chain([
                    caption(estado).test_id("mapa.sector"),
                    div()
                        .class("flex items-center justify-between gap-3")
                        .children([
                            caption(format!(
                                "{:.4}, {:.4} · zoom {:.1}",
                                vista.center.lat, vista.center.lon, vista.zoom
                            ))
                            .role(Role::Status)
                            .test_id("mapa.posicion"),
                            button("Volver a la plaza", ButtonVariant::Outlined)
                                .on_click(Msg::FijarVista {
                                    vista: VISTA_INICIAL,
                                    animar: true,
                                })
                                .test_id("mapa.volver"),
                        ]),
                ]),
            ),
        ])
    }

    /// Gráficos: a bar chart of monthly clicks and a sparkline of the trend.
    fn graficos(&self) -> Vec<Element<Msg>> {
        let elegido = self.mes.map(|index| {
            format!(
                "Seleccionado: {} ({})",
                MESES[index], CLICS_MES[index] as i32
            )
        });
        vec![
            card()
                .class("w-full max-w-3xl")
                .test_id("graficos.barras")
                .children([
                    card_header("Barras", Some("Clics por mes; elige una barra")),
                    card_content().children([
                        chart(ChartKind::Bar)
                            .categories(MESES)
                            .series("Clics", &CLICS_MES)
                            .selected(self.mes)
                            .on_select(Msg::Mes)
                            .label("Clics por mes")
                            .test_id("graficos.barras.plot")
                            .class("h-48 w-full"),
                        caption(elegido.as_deref().unwrap_or("Elige una barra")),
                    ]),
                ]),
            card()
                .class("w-full max-w-3xl")
                .test_id("graficos.sparkline")
                .children([
                    card_header("Sparkline", Some("Tendencia compacta, sin ejes")),
                    card_content().class("flex items-center gap-4").children([
                        stat("Tendencia", "16", Some("último mes")),
                        chart(ChartKind::Sparkline)
                            .series("Tendencia", &TENDENCIA)
                            .label("Tendencia")
                            .test_id("graficos.sparkline.plot")
                            .class("h-8 flex-1"),
                    ]),
                ]),
        ]
    }


    #[cfg(feature = "document")]
    fn documento_ribbon_chrome(&self) -> Element<Msg> {
        document_ribbon()
            .mode(RibbonMode::Full)
            .active_tab(self.doc_ribbon_tab.clone())
            .on_tab(Msg::DocRibbonTab)
            .on_command(|id| Msg::DocCmd(document_cmd_for(&id)))
            .leading([
                toolbar_item(Icon::FolderOpen, "Abrir…")
                    .on_command(DocAbrir)
                    .test_id("documento.abrir"),
                toolbar_item(Icon::Save, "Guardar…")
                    .on_command(DocGuardar)
                    .test_id("documento.guardar"),
                toolbar_item(Icon::FileDown, "Exportar PDF…")
                    .on_command(DocExportarPdf)
                    .test_id("documento.exportar_pdf"),
            ])
            .test_id("documento.ribbon")
            .build()
    }

    #[cfg(not(feature = "document"))]
    fn documento_ribbon_chrome(&self) -> Element<Msg> {
        div()
            .class("flex flex-col gap-1")
            .test_id("documento.ribbon")
            .children([
                div()
                    .class("flex flex-row items-center gap-0.5")
                    .role(Role::Toolbar)
                    .test_id("documento.archivo")
                    .children([
                        toolbar_item(Icon::FolderOpen, "Abrir…")
                            .on_command(DocAbrir)
                            .test_id("documento.abrir"),
                        toolbar_item(Icon::Save, "Guardar…")
                            .on_command(DocGuardar)
                            .test_id("documento.guardar"),
                        toolbar_item(Icon::FileDown, "Exportar PDF…")
                            .on_command(DocExportarPdf)
                            .test_id("documento.exportar_pdf"),
                    ]),
                text("Compila con --features document para el ribbon WordCraft.")
                    .class("text-sm text-muted-foreground"),
            ])
    }

    /// Documento: metáfora Pages — ribbon (QAT Archivo + tabs) + canvas Letter.
    fn documento(&self) -> Element<Msg> {
        let mut editor = document()
            .label("Informe de demostración")
            .placeholder_text("Escribe aquí…")
            .markdown(&self.doc_markdown)
            .on_change(Msg::Documento)
            .class("flex-1 min-h-0 w-full")
            .test_id("documento.editor");
        if let Some((cmd, serial)) = &self.doc_cmd {
            editor = editor.pending_cmd(cmd.clone(), *serial);
        }
        if let Some((serial, bytes)) = &self.doc_load {
            editor = editor.pending_load(bytes.clone(), *serial);
        }
        div()
            .class("flex flex-col flex-1 min-h-0 w-full")
            .test_id("documento")
            .children([
                div()
                    .class(
                        "flex-none flex flex-col px-3 pt-2 pb-2 bg-window border-b border-separator",
                    )
                    .child(self.documento_ribbon_chrome()),
                editor,
            ])
    }

    fn dialogs_and_toasts(&self) -> Element<Msg> {
        card()
            .class("w-full max-w-3xl")
            .test_id("dialogos")
            .children([
                card_header(
                    "Diálogos y avisos",
                    Some("Modales con foco atrapado, avisos que se cierran solos y la paleta"),
                ),
                card_content().class("gap-4").children([
                    div().class("flex flex-wrap items-center gap-3").children([
                        caption("Diálogos:"),
                        button("Guardar cambios…", ButtonVariant::Outlined)
                            .on_click(Msg::OpenDialog(Dialogo::Guardar))
                            .test_id("dialogos.guardar"),
                        button("Eliminar borrador…", ButtonVariant::Danger)
                            .on_click(Msg::OpenDialog(Dialogo::Eliminar))
                            .test_id("dialogos.eliminar"),
                    ]),
                    div()
                        .class("flex flex-wrap items-center gap-3")
                        .child(caption("Avisos:"))
                        .children(VarianteAviso::ALL.map(|variante| {
                            button(variante.texts().0, ButtonVariant::Secondary)
                                .on_click(Msg::ShowToast(variante))
                                .test_id(format!("avisos.{}", variante.key()))
                        })),
                    div().class("flex flex-wrap items-center gap-3").children([
                        caption("Comandos:"),
                        Button::new("Abrir la paleta (⇧⌘P)")
                            .bezel(BezelStyle::Borderless)
                            .build()
                            .run_command(PALETTE)
                            .test_id("dialogos.paleta"),
                    ]),
                ]),
            ])
    }

    /// The open dialog.
    fn dialog(&self, dialogo: Dialogo) -> Element<Msg> {
        let dialog = match dialogo {
            Dialogo::Guardar => Dialog::new("¿Guardar los cambios?")
                .message("Los cambios quedarán disponibles para todo el equipo.")
                .default_button("Guardar", Msg::ConfirmDialog),
            Dialogo::Eliminar => Dialog::new("¿Eliminar el borrador?")
                .message("Esta acción no se puede deshacer.")
                .destructive(true)
                .width(DialogWidth::Sm)
                .default_button("Eliminar", Msg::ConfirmDialog),
        };
        dialog
            .cancel_button("Cancelar", Msg::CloseDialog)
            .test_id("dialogo")
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
                        .test_id("fondo.efecto"),
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
                div()
                    .class("grid grid-cols-1 sm:grid-cols-2 gap-x-4 gap-y-5")
                    .children([
                        TextField::new("Buscar", &self.search)
                            .placeholder("Buscar productos")
                            .start_icon(Icon::Search)
                            .on_input(edit(Field::Search))
                            .build(),
                        TextField::new("Contraseña", &self.password)
                            .test_id("form.password")
                            .password_command(self.show_password, AlternarContrasena)
                            .required(true)
                            .helper_text("Mínimo 8 caracteres")
                            .on_input(edit(Field::Password))
                            .build(),
                        TextField::new("Monto", &self.amount)
                            .start_symbol("$")
                            .select_on_focus(true)
                            .on_input(edit(Field::Amount))
                            .build(),
                        TextField::new("Descuento", &self.discount)
                            .end_symbol("%")
                            .on_input(edit(Field::Discount))
                            .build(),
                        TextField::new("Comuna", &self.commune)
                            .required(true)
                            .helper_text("Donde despacharemos el pedido")
                            .on_input(edit(Field::Commune))
                            .build(),
                        TextField::new("Código", &self.code)
                            .density(FieldDensity::Compact)
                            .placeholder("Ej: A-12")
                            .on_input(edit(Field::Code))
                            .build(),
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
                        TextField::new("Folio", "F-2026-0042")
                            .read_only(true)
                            .end_icon(Icon::CircleCheckBig)
                            .build(),
                        TextField::new("Sucursal", "Santiago Centro")
                            .disabled(true)
                            .build(),
                        autocomplete_shell,
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
    let demo = Demo {
        background,
        background_motion,
        ..Demo::default()
    };
    #[cfg(feature = "mcp")]
    if mcp {
        return runtime::run_with_mcp(demo, demo_theme());
    }
    runtime::run(demo, demo_theme())
}

/// Theme of the demo, chosen with `KAI_THEME=kai|appkit|appkit-dark`
/// (default AppKit current light). `KAI_THEME=kai` keeps the Kai theme.
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
        Ok("appkit" | "appkit-light") | Ok("") | Err(_) => {
            Theme::appkit_current(Appearance::Light)
        }
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
        Campo, CargarUrl, Dialogo, ImageSource, LatLon, LatLonBounds, MapView, PLAZA_DE_ARMAS,
        SECTORES, SimularEspera, VISTA_INICIAL, VarianteAviso, ViewChange, sector_inicial,
    };

    use cosmic_text::{
        Action, Attrs, Buffer, Edit, Editor, FontSystem, Metrics, Motion, Selection, Shaping,
    };

    use super::{
        AlternarContrasena, Demo, Enviar, Field, Incrementar, Limpiar, Msg, Reiniciar, Saludo,
        Seccion,
    };
    use serde_json::json;

    use crate::background::{BackgroundEffect, BackgroundKind, BackgroundMotion};
    use crate::commands::{
        Availability, CommandError, DIALECT, Origin, Outcome, PALETTE, describe_json,
    };
    use crate::components::SortDirection;
    use crate::element::{ControlState, FocusOrder, KeyEquivalent, Kind, Role, WidgetId};
    use crate::geometry::{Rect, Size};
    use crate::layout::testing::FixedMeasure;
    use crate::layout::{Interaction, Layout, LayoutInput, ScrollOffsets, StyleCache};
    use crate::menu::{MenuEntry, menu_model};

    impl Demo {
        /// The default demo showing `seccion`.
        fn at(seccion: Seccion) -> Self {
            Self {
                seccion,
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
    use crate::testing::{By, Confirm, Key, Modifiers, MsgSource, Screen, TestApp, layout_of};
    use crate::theme::{AppKitGeneration, Appearance, Density, Theme};

    /// The demo's tree as the session shows it: built, with its images
    /// decoded.
    fn build_view(demo: &Demo) -> crate::element::Element<Msg> {
        let mut tree = crate::runtime::build_view(demo);
        ImageStore::new(std::sync::Arc::new(FakeImageLoader::new())).load_now(&mut tree);
        tree
    }

    /// Two-field rows (`grid-cols-1 sm:grid-cols-2`): side by side from
    /// 640 pt, one column below, never wider than the window (the fields
    /// keep their 180 pt minimum).
    #[test]
    fn field_rows_stack_in_a_narrow_window() {
        use crate::theme::MixStyle;
        let rows = [
            (Demo::default(), ("Nombre", "Correo")),
            (Demo::at(Seccion::Campos), ("Buscar", "Contraseña")),
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
                                .test_id("enviar"),
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
                    let button = screen.get_by_test_id("enviar").bounds();
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
        let name = find(&demo, By::test_id("form.nombre.input"));
        let order = FocusOrder::from_tree(&build_view(&demo));
        let mut model = UiModel::default();
        dispatch(&mut model, &order, Intent::SetFocus(Some(name)));
        let empty = dispatch(&mut model, &order, return_key.clone());
        assert_eq!(empty.activated, None, "«Enviar» is disabled without a name");

        demo.apply(Msg::NameChanged("Ana".to_owned()));
        let send = find(&demo, By::test_id("form.enviar"));
        let order = FocusOrder::from_tree(&build_view(&demo));
        let result = dispatch(&mut model, &order, return_key.clone());
        assert_eq!(result.activated.map(|a| a.widget), Some(send));

        // «Correo» keeps its on_submit.
        let email = find(&demo, By::test_id("form.correo.input"));
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
        let inicio = [
            "barra.sidebar",
            "barra.buscar.input",
            "nav.0",
            "fondo.efecto.0",
            "contador.incrementar",
            "form.nombre.input",
            "form.correo.input",
            "form.rol",
            "form.terminos",
            "form.notificaciones.track",
            "form.limpiar",
        ];
        assert_eq!(tab_stops(&mut TestApp::new(Demo::default())), inicio);

        let campos = tab_stops(&mut TestApp::new(Demo::at(Seccion::Campos)));
        assert_eq!(campos[2], "nav.1", "the sidebar enters at the selection");
        let password = campos.iter().position(|stop| stop == "form.password.input");
        assert_eq!(
            campos[password.unwrap() + 1],
            "form.password.toggle",
            "the eye follows its field"
        );

        let mut glow = Demo::default();
        glow.apply(Msg::SetBackground(BackgroundKind::Glow));
        let stops = tab_stops(&mut TestApp::new(glow));
        assert_eq!(
            stops.len(),
            inicio.len() + 1,
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
        let content = screen.get_by_test_id("contenido").node();
        let scroll = content.scroll.expect("content scrolls");
        assert!(scroll.max > 0.0, "the small window overflows");
        let header = screen.get(By::role(Role::Toolbar)).node();
        assert_eq!(header.bounds.y, 0.0, "the toolbar stays fixed");
        assert_eq!(content.clip, None);
    }

    /// Full layout + paint of the demo view with real cosmic-text
    /// measurement and no cache; widths alternate so text re-wraps every
    /// frame. Run with:
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
        let send = screen.get_by_test_id("form.enviar");
        assert_eq!(send.role(), Role::Button);
        assert_eq!(send.name().as_deref(), Some("Enviar"));
        assert_eq!(
            screen.get(By::role(Role::Button).name("Enviar")).id(),
            send.id()
        );
        assert_eq!(
            screen.get(By::role(Role::Button).name("Limpiar")).test_id(),
            Some("form.limpiar")
        );
        assert_eq!(
            screen.get_by_label("Nombre").test_id(),
            Some("form.nombre.input")
        );
        assert_eq!(
            screen.get_by_label("Correo").test_id(),
            Some("form.correo.input")
        );
        assert_eq!(
            screen.get_by_test_id("barra.sumar").name().as_deref(),
            Some("Sumar clic")
        );
        assert_eq!(
            screen
                .get_by_test_id("barra.buscar.input")
                .name()
                .as_deref(),
            Some("Filtrar clientes")
        );
        // Sidebar rows are parts of `nav`, by position.
        for (index, seccion) in Seccion::ALL.into_iter().enumerate() {
            let row = screen.get(sidebar_row(&seccion.item().label));
            assert_eq!(row.test_id(), Some(format!("nav.{index}").as_str()));
        }

        // Every section: unique ids, and its controls reachable by id.
        for seccion in Seccion::ALL {
            let tree = build_view(&Demo::at(seccion));
            assert_eq!(tree.validate_ids(), Vec::<String>::new(), "{seccion:?}");
        }
        let layout = layout_of(&build_view(&Demo::at(Seccion::Campos)));
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
            Some("campos.pin.input")
        );
        assert_eq!(
            screen.get_by_test_id("campos.notas.input").role(),
            Role::TextArea
        );

        let layout = layout_of(&build_view(&Demo::at(Seccion::Seleccion)));
        let screen = Screen::new(&layout);
        assert_eq!(
            screen.get_by_test_id("seleccion.todos").role(),
            Role::Checkbox
        );
        assert_eq!(
            screen.get_by_test_id("seleccion.pais").role(),
            Role::PopUpButton
        );
        assert_eq!(
            screen
                .get_by_test_id("seleccion.alineacion.1")
                .name()
                .as_deref(),
            Some("Centro")
        );

        let layout = layout_of(&build_view(&Demo::at(Seccion::Tabla)));
        let screen = Screen::new(&layout);
        assert_eq!(screen.get_by_test_id("tabla.clientes").role(), Role::Table);
        assert_eq!(screen.get_by_test_id("tabla.lista").role(), Role::List);
    }

    #[test]
    fn typing_a_name_enables_send_and_the_click_submits_it() {
        let mut app = TestApp::new(Demo::default());
        let send = || By::test_id("form.enviar");
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
        app.click(By::test_id("form.rol"));
        app.click(By::test_id("kai.menu.1"));
        app.click(By::test_id("form.terminos"));
        assert_eq!(app.messages(), [Msg::SetRole(1), Msg::ToggleTerms]);
        assert_eq!((app.app().role, app.app().terms), (Some(1), true));
        app.screen().get(By::text("Desarrollo"));
        assert_eq!(
            app.screen().get_by_test_id("form.terminos").role(),
            Role::Checkbox
        );

        app.click(By::test_id("form.limpiar"));
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

    /// Something only section `seccion` shows.
    fn landmark(seccion: Seccion) -> &'static str {
        match seccion {
            Seccion::Inicio => "contador",
            Seccion::Campos => "form.password",
            Seccion::Botones => "botones.secondary",
            Seccion::Seleccion => "seleccion",
            Seccion::Contenedores => "contenedores.card",
            Seccion::Badges => "badges",
            Seccion::Multimedia => "multimedia.campos",
            Seccion::Tabla => "tabla.clientes",
            Seccion::Mapa => "mapa.vista",
            Seccion::Graficos => "graficos.barras",
            Seccion::Documento => "documento.editor",
            Seccion::Dialogos => "dialogos",
        }
    }


    #[cfg(feature = "document")]
    #[test]
    fn document_types_bold_and_undo() {
        let mut demo = Demo::at(Seccion::Documento);
        demo.doc_markdown.clear();
        let mut app = TestApp::new(demo);
        let editor = || By::test_id("documento.editor");
        assert_eq!(
            app.screen().get(editor()).name().as_deref(),
            Some("Informe de demostración")
        );
        app.type_text(editor(), "Hola");
        assert_eq!(app.value(editor()), "Hola");
        app.click(By::test_id("documento.ribbon.cmd.select.all"));
        assert!(app.perform("format.bold"));
        assert_eq!(app.value(editor()), "Hola");
        assert!(app.perform("edit.undo"));
        assert_eq!(app.value(editor()), "Hola");
        assert!(app.perform("edit.undo"));
        assert_ne!(app.value(editor()), "Hola");
    }

    #[cfg(feature = "document")]
    #[test]
    fn document_multipage_scrolls_and_keeps_hit_target() {
        let mut app = TestApp::new(Demo::at(Seccion::Documento));
        let editor = || By::test_id("documento.editor");
        let before = app.value(editor());
        assert!(
            before.contains("Párrafo 40") || before.len() > 200,
            "demo should seed multi-page markdown, got len={}",
            before.len()
        );
        app.scroll_wheel(editor(), 480.0);
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
        let mut demo = Demo::at(Seccion::Documento);
        demo.doc_markdown.clear();
        let mut app = TestApp::new(demo);
        let editor = || By::test_id("documento.editor");
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
        let mut demo = Demo::at(Seccion::Documento);
        demo.doc_markdown.clear();
        let mut app = TestApp::new(demo);
        let editor = || By::test_id("documento.editor");
        app.type_text(editor(), "Ribbon");
        assert_eq!(app.value(editor()), "Ribbon");
        app.click(By::test_id("documento.ribbon.cmd.select.all"));
        app.click(By::test_id("documento.ribbon.cmd.format.bold"));
        assert_eq!(app.value(editor()), "Ribbon");
        app.click(By::test_id("documento.ribbon.tab.insert"));
        app.click(By::test_id("documento.ribbon.cmd.insert.table"));
        assert!(app.perform("edit.undo"));
        app.type_text(editor(), "!");
        assert!(app.value(editor()).contains('!'));
    }

    #[cfg(feature = "document")]
    #[test]
    fn document_ribbon_layout_review_view_tabs() {
        use crate::document::{ribbon_catalog, RibbonMode};
        let tabs = ribbon_catalog(RibbonMode::Full);
        assert!(tabs.iter().any(|t| t.id == "layout"));
        assert!(tabs.iter().any(|t| t.id == "review"));
        assert!(tabs.iter().any(|t| t.id == "view"));
        assert!(tabs.iter().any(|t| t.id == "references"));

        let mut demo = Demo::at(Seccion::Documento);
        demo.doc_markdown.clear();
        let mut app = TestApp::new(demo);
        let editor = || By::test_id("documento.editor");
        app.click(editor());
        app.type_text(editor(), "Tabs");

        app.click(By::test_id("documento.ribbon.tab.layout"));
        app.click(By::test_id("documento.ribbon.cmd.layout.orientation"));

        app.click(By::test_id("documento.ribbon.tab.review"));
        app.click(By::test_id("documento.ribbon.cmd.review.wordCount"));

        app.click(By::test_id("documento.ribbon.tab.view"));
        app.click(By::test_id("documento.ribbon.cmd.view.zoomIn"));

        app.click(By::test_id("documento.ribbon.tab.references"));
        app.click(By::test_id("documento.ribbon.cmd.references.footnote"));

        assert!(
            app.value(editor()).contains("Tabs") || !app.value(editor()).is_empty(),
            "editor should stay usable after ribbon cmds"
        );
    }

    #[cfg(feature = "document")]
    #[test]
    fn document_save_docx_via_file_panel() {
        let mut demo = Demo::at(Seccion::Documento);
        demo.doc_markdown = "Hola archivo".into();
        let mut app = TestApp::new(demo);
        app.click(By::test_id("documento.guardar"));
        let request = app.file_save_request().cloned().expect("save panel");
        assert_eq!(request.name, "informe.docx");
        assert_eq!(request.types, ["docx"]);

        let dir = std::env::temp_dir().join(format!("kai-doc-save-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("informe.docx");
        app.on_file_save(Some(path.clone()));
        app.run_until_idle();
        assert!(!app.app().doc_archivo_busy);
        let bytes = std::fs::read(&path).expect("docx written");
        assert!(bytes.starts_with(b"PK"), "docx zip");
        app.screen().get(By::text("Archivo listo"));
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[cfg(feature = "document")]
    #[test]
    fn document_open_docx_via_file_panel() {
        // Seed a .docx via Guardar, then Abrir into a fresh demo.
        let mut demo = Demo::at(Seccion::Documento);
        demo.doc_markdown = "Desde disco".into();
        let mut app = TestApp::new(demo);
        app.click(By::test_id("documento.guardar"));
        let dir = std::env::temp_dir().join(format!("kai-doc-open-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("seed.docx");
        app.on_file_save(Some(path.clone()));
        app.run_until_idle();
        assert!(path.exists());

        let mut demo = Demo::at(Seccion::Documento);
        demo.doc_markdown.clear();
        let mut app = TestApp::new(demo);
        app.click(By::test_id("documento.abrir"));
        let request = app.file_pick_request().cloned().expect("open panel");
        assert_eq!(request.types, ["docx"]);
        app.on_file_pick(Some(path.clone()));
        app.run_until_idle();
        assert!(!app.app().doc_archivo_busy);
        let value = app.value(By::test_id("documento.editor"));
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
        let mut demo = Demo::at(Seccion::Documento);
        demo.doc_markdown = "PDF demo".into();
        let mut app = TestApp::new(demo);
        assert!(app.perform("documento.exportar_pdf"));
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
        app.screen().get(By::test_id("form.nombre"));
        app.click(sidebar_row("Campos"));
        assert_eq!(app.messages(), [Msg::IrA(Seccion::Campos)]);
        app.screen().get(By::test_id("form.password"));
        assert!(app.screen().query(By::test_id("form.nombre")).is_none());
        assert!(app.is_focused(sidebar_row("Campos")));

        // ↓ walks every section in order, across the groups, and ↑ back.
        for pair in Seccion::ALL[1..].windows(2) {
            app.press(Key::ArrowDown, Modifiers::NONE);
            assert_eq!(app.app().seccion, pair[1]);
            app.screen().get(By::test_id(landmark(pair[1])));
            assert!(
                app.screen().query(By::test_id(landmark(pair[0]))).is_none(),
                "{:?} left",
                pair[0]
            );
        }
        for _ in 1..Seccion::ALL.len() {
            app.press(Key::ArrowUp, Modifiers::NONE);
        }
        assert_eq!(app.app().seccion, Seccion::Inicio);
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
        assert_eq!(titles, ["Demo", "Componentes", "Datos", "Superposiciones"]);
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
                "Diálogos y avisos",
            ]
        );
    }

    #[test]
    fn clicking_a_bar_selects_its_category() {
        let mut app = TestApp::new(Demo::at(Seccion::Graficos));
        let plot = app.screen().get(By::test_id("graficos.barras.plot"));
        assert_eq!(plot.name().as_deref(), Some("Clics por mes"));
        assert_eq!(
            plot.node().description.as_deref(),
            Some("Ene 4, Feb 12, Mar 7, Abr 15, May 9, Jun 11")
        );
        let width = plot.bounds().width;
        let height = plot.bounds().height;
        app.click_at(
            By::test_id("graficos.barras.plot"),
            width * 0.08,
            height * 0.5,
        );
        assert_eq!(app.messages(), [Msg::Mes(0)]);
        assert_eq!(app.app().mes, Some(0));
        app.screen().get(By::text("Seleccionado: Ene (4)"));
        app.click_at(
            By::test_id("graficos.barras.plot"),
            width * 0.92,
            height * 0.5,
        );
        assert_eq!(app.app().mes, Some(5));
        app.screen().get(By::text("Seleccionado: Jun (11)"));
    }

    #[test]
    fn waiting_buttons_show_the_dots_until_their_task_ends() {
        let mut app = TestApp::new(Demo::at(Seccion::Botones));
        let primary = By::test_id("espera.primary");
        let width = app.screen().get(primary.clone()).bounds().width;
        app.click(primary.clone());
        assert!(app.is_busy(primary.clone()));
        assert!(!app.is_busy(By::test_id("espera.secondary")));
        assert_eq!(app.screen().get(primary.clone()).bounds().width, width);
        assert_eq!(
            app.screen().get(primary.clone()).name().as_deref(),
            Some("Primary")
        );
        app.click(primary.clone());
        let waits = app.messages().iter();
        assert_eq!(
            waits.filter(|msg| **msg == Msg::Esperar("Primary")).count(),
            1
        );
        app.run_until_idle();
        assert!(!app.is_busy(primary));
        app.screen().get(By::text("Terminó: Primary"));
        assert!(app.is_busy(By::test_id("espera.cargando")), "loading(true)");
    }

    #[test]
    fn a_chained_save_and_a_background_sync_in_the_demo() {
        let mut app = TestApp::new(Demo::at(Seccion::Botones));
        app.click(By::test_id("espera.recargar"));
        app.run_next_tasks();
        assert!(app.messages().contains(&Msg::Recargar));
        assert!(app.is_busy(By::test_id("espera.recargar")));
        app.run_until_idle();
        assert!(!app.is_busy(By::test_id("espera.recargar")));
        app.screen().get(By::text("Terminó: guardar y recargar"));

        let sync = By::test_id("espera.sincronizar");
        app.click(sync.clone());
        app.click(sync.clone());
        assert!(!app.is_busy(sync));
        app.screen().get(By::text("Sincronizando (2)…"));
        app.run_until_idle();
        assert_eq!(app.app().sincronizando, 0);
    }

    #[test]
    fn the_wait_command_is_in_course_everywhere_while_it_runs() {
        const ID: &str = <SimularEspera as crate::commands::Action>::ID;
        let mut app = TestApp::new(Demo::at(Seccion::Botones));
        let bound = By::test_id(ID);
        assert!(app.perform(ID));
        assert!(app.is_busy(bound.clone()));
        assert_eq!(
            app.available::<SimularEspera>(),
            Availability::disabled(crate::session::COMMAND_BUSY)
        );
        assert!(!app.perform(ID));
        app.run_until_idle();
        assert!(!app.is_busy(bound));
        assert_eq!(app.available::<SimularEspera>(), Availability::Enabled);
    }

    /// Every public constructor of `components` shows up in the demo,
    /// found by role, name or test id in some section (an overlay after
    /// the click that opens it). Items that only feed a constructor (data,
    /// class strings, timers) are listed apart; a new public item fails
    /// here until it is in one list or the other.
    #[test]
    fn the_demo_shows_every_component() {
        use std::collections::BTreeSet;
        const SOURCES: [&str; 18] = [
            include_str!("components.rs"),
            include_str!("components/color.rs"),
            include_str!("components/dialog.rs"),
            include_str!("components/media.rs"),
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
        const NOT_WIDGETS: [&str; 32] = [
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
            "DocumentRun",
            "DocumentFill",
            "DocumentLine",
            "DocumentImage",
        ];
        use Seccion::*;
        let shown: Vec<(&[&str], Seccion, Option<&str>, By)> = vec![
            (&["toolbar"], Inicio, None, By::role(Role::Toolbar)),
            (&["toolbar_item"], Inicio, None, By::test_id("barra.sumar")),
            (&["toolbar_title"], Inicio, None, By::text("Kai UI Desktop")),
            (
                &["toolbar_spacer"],
                Inicio,
                None,
                By::test_id("barra.espacio"),
            ),
            (
                &["toolbar_separator"],
                Inicio,
                None,
                By::test_id("barra.separador"),
            ),
            (
                &["toolbar_search", "search_field", "SearchField"],
                Inicio,
                None,
                By::role(Role::SearchInput).name("Filtrar clientes"),
            ),
            (
                &["sidebar", "Sidebar"],
                Inicio,
                None,
                By::role(Role::Sidebar).name("Secciones"),
            ),
            (&["segmented"], Inicio, None, By::test_id("fondo.efecto")),
            (
                &["switch_labeled"],
                Inicio,
                None,
                By::role(Role::Switch).name("Notificaciones"),
            ),
            (
                &["checkbox"],
                Inicio,
                None,
                By::role(Role::Checkbox).name("Acepto los términos"),
            ),
            (&["select"], Inicio, None, By::test_id("form.rol")),
            (&["stat"], Inicio, None, By::text("Envíos")),
            (&["TextField"], Campos, None, By::test_id("form.password")),
            (
                &["text_area", "TextArea"],
                Campos,
                None,
                By::role(Role::TextArea).name("Notas"),
            ),
            (&["button"], Botones, None, By::test_id("botones.secondary")),
            (
                &["button_sized"],
                Botones,
                None,
                By::role(Role::Button).name("Grande"),
            ),
            (&["Button"], Botones, None, By::test_id("botones.bordered")),
            (
                &["DotProgress", "dot_progress"],
                Botones,
                None,
                By::role(Role::ProgressIndicator).name("Cargando"),
            ),
            (
                &["icon_button"],
                Botones,
                None,
                By::test_id("botones.icono"),
            ),
            (
                &["icon_button_sized"],
                Botones,
                None,
                By::role(Role::Button).name("Usuario"),
            ),
            (
                &["switch"],
                Seleccion,
                None,
                By::role(Role::Switch).name("Wi-Fi"),
            ),
            (
                &["switch_with_options"],
                Seleccion,
                None,
                By::test_id("seleccion.bluetooth"),
            ),
            (
                &["checkbox_with_options"],
                Seleccion,
                None,
                By::test_id("seleccion.todos"),
            ),
            (
                &["select_with_options"],
                Seleccion,
                None,
                By::test_id("seleccion.pais"),
            ),
            (
                &["select_menu"],
                Seleccion,
                Some("seleccion.pais"),
                By::role(Role::Menu),
            ),
            (
                &["segmented_with_options"],
                Seleccion,
                None,
                By::test_id("seleccion.alineacion"),
            ),
            (
                &["card", "card_header", "card_content", "card_actions"],
                Contenedores,
                None,
                By::test_id("contenedores.card"),
            ),
            (
                &["group_box"],
                Contenedores,
                None,
                By::test_id("contenedores.grupo"),
            ),
            (
                &["heading"],
                Contenedores,
                None,
                By::role(Role::Heading).name("Título de sección"),
            ),
            (
                &["paragraph"],
                Contenedores,
                None,
                By::role(Role::Paragraph),
            ),
            (
                &["caption"],
                Contenedores,
                None,
                By::text("Una leyenda, para notas y metadatos."),
            ),
            (&["Tabs"], Contenedores, None, By::role(Role::TabList)),
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
                By::test_id("multimedia.avatar_url"),
            ),
            (
                &["MediaSlot"],
                Multimedia,
                None,
                By::test_id("multimedia.portada.preview"),
            ),
            (
                &["alert"],
                Badges,
                None,
                By::text("Revisa los campos requeridos"),
            ),
            (
                &["Table"],
                Tabla,
                None,
                By::role(Role::Table).name("Clientes"),
            ),
            (
                &["virtual_list"],
                Tabla,
                None,
                By::role(Role::List).name("Elementos"),
            ),
            (
                &["column_menu"],
                Tabla,
                None,
                By::role(Role::Button).name("Columnas"),
            ),
            (
                &["map"],
                Mapa,
                None,
                By::role(Role::Group).name("Mapa de Parral"),
            ),
            (
                &["marker"],
                Mapa,
                None,
                By::role(Role::Image).name("Plaza de Armas"),
            ),
            (
                &["MapLayer", "polygons"],
                Mapa,
                None,
                By::role(Role::List).name("Sectores"),
            ),
            (
                &["color_field", "ColorField"],
                Mapa,
                Some("mapa.sector.AOP26000002"),
                By::role(Role::Group).name("Color del sector"),
            ),
            (
                &["chart"],
                Graficos,
                None,
                By::role(Role::Image).name("Clics por mes"),
            ),
            (
                &["document"],
                Documento,
                None,
                By::role(Role::TextArea).name("Informe de demostración"),
            ),
            (
                &["Dialog", "dialog"],
                Dialogos,
                Some("dialogos.guardar"),
                By::role(Role::Dialog).name("¿Guardar los cambios?"),
            ),
            (
                &["toast_stack"],
                Dialogos,
                Some("avisos.info"),
                By::test_id("kai.aviso.1"),
            ),
            (
                &["command_palette"],
                Dialogos,
                Some("dialogos.paleta"),
                By::test_id("kai.paleta"),
            ),
        ];
        let mut covered = BTreeSet::new();
        for (names, seccion, open, by) in shown {
            let mut app = TestApp::new(Demo::at(seccion));
            if let Some(id) = open {
                app.click(By::test_id(id));
            }
            assert!(
                !app.screen().all(by).is_empty(),
                "{names:?} no aparece en {seccion:?}"
            );
            covered.extend(names.iter().copied());
        }
        let mut editor = TestApp::new(Demo::at(Mapa));
        editar_catillo(&mut editor);
        assert!(!editor.screen().all(vertice(0)).is_empty());
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
        let foto = dir.join("portada.png");
        std::fs::write(&foto, png([200, 40, 40, 255])).unwrap();
        let mut app = TestApp::new(Demo::at(Seccion::Multimedia));
        let preview = |id: &str| By::test_id(format!("multimedia.{id}.preview"));
        assert_eq!(
            app.screen().get(preview("portada")).name().as_deref(),
            Some("Agregar imagen")
        );

        app.click(preview("portada"));
        let request = app.file_pick_request().expect("the panel is open");
        assert_eq!(request.title, "Elegir portada");
        app.on_file_pick(Some(foto.clone()));
        app.run_until_idle();
        assert_eq!(
            app.app().portada.actual,
            Some(ImageSource::Path(foto.clone()))
        );
        assert_eq!(
            app.screen().get(preview("portada")).name().as_deref(),
            Some("Cambiar imagen")
        );
        assert!(
            app.messages()
                .contains(&Msg::ImagenElegida(Campo::Portada, Some(foto)))
        );

        let before = app.app().foto.clone();
        app.on_file_pick(None);
        app.click(preview("foto"));
        assert_eq!(app.app().foto, before, "cancelled");

        app.drop_files(preview("logo"), [dir.join("nota.txt")]);
        assert_eq!(
            app.app().logo.error.as_deref(),
            Some("Tipo no permitido: png, jpg, jpeg, webp")
        );
        assert!(app.app().logo.actual.is_some(), "the logo stays");
        app.screen().get(By::test_id("multimedia.logo.error"));
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn cargar_url_puts_a_downloaded_image_in_the_avatar() {
        const URL: &str = "https://kai.test/ana.png";
        let mut app = TestApp::builder(Demo::at(Seccion::Multimedia))
            .images(FakeImageLoader::new().with(URL, png([20, 120, 220, 255])))
            .build();
        let image = By::test_id("multimedia.avatar_url.image");
        assert!(app.screen().query(image.clone()).is_none(), "the icon");

        let result = app.call("multimedia.cargar_url", json!({"url": URL}));
        assert!(result.is_ok(), "{result:?}");
        app.run_until_idle();
        assert_eq!(app.app().avatar_url.as_deref(), Some(URL));
        app.screen().get(image.clone());
        assert_eq!(app.app().url, URL, "the field shows it");

        app.type_text(By::test_id("multimedia.url"), " ");
        app.click(By::test_id("multimedia.cargar"));
        assert_eq!(app.app().avatar_url.as_deref(), Some(URL), "trimmed");
        app.invoke(CargarUrl { url: String::new() }).unwrap();
        assert!(app.screen().query(image).is_none(), "back to the icon");
    }

    #[test]
    fn the_table_shows_an_avatar_by_each_name() {
        let app = TestApp::new(Demo::at(Seccion::Tabla));
        let screen = app.screen();
        let row = screen.within(screen.get_by_test_id("tabla.clientes.row.0"));
        let nombre = super::cliente(1).nombre;
        row.get(By::role(Role::Image).name(&nombre));
        let names: Vec<_> = row
            .all(By::role(Role::Cell))
            .iter()
            .filter_map(|cell| cell.name())
            .collect();
        assert_eq!(
            names[2], nombre,
            "after the chevron and N.º; the avatar stays out of the cell's name"
        );
    }

    #[test]
    fn the_toolbar_search_filters_and_the_headers_sort_the_table() {
        let mut app = TestApp::new(Demo::default());
        let table = By::test_id("tabla.clientes");
        let count = |app: &TestApp<Demo>| app.screen().get(table.clone()).node().row_count;
        let cells = |app: &TestApp<Demo>, row: usize| -> Vec<String> {
            let screen = app.screen();
            screen
                .within(screen.get_by_test_id(&format!("tabla.clientes.row.{row}")))
                .all(By::role(Role::Cell))
                .iter()
                .filter_map(|cell| cell.name())
                .collect()
        };
        // Return in the search goes to the table.
        app.type_text(By::test_id("barra.buscar"), "temuco");
        app.press(Key::Enter, Modifiers::NONE);
        assert_eq!(app.app().seccion, Seccion::Tabla);
        let temuco = (1..=super::CLIENTES)
            .map(super::cliente)
            .filter(|c| {
                [c.nombre.as_str(), c.ciudad, c.plan]
                    .iter()
                    .any(|text| text.to_lowercase().contains("temuco"))
            })
            .count();
        assert!(temuco > 0 && temuco < super::CLIENTES as usize);
        assert_eq!(count(&app), Some(temuco));
        assert_eq!(cells(&app, 0)[3], "Temuco");
        assert!(
            app.screen()
                .get_by_test_id("tabla.total")
                .name()
                .unwrap()
                .contains("de 10000 con «temuco»")
        );

        // A sortable header sorts, and again reverses.
        let monto = || By::role(Role::ColumnHeader).name("Monto");
        app.click(monto());
        assert_eq!(app.app().tabla.sort, Some((4, SortDirection::Ascending)));
        let amount = |row: Vec<String>| -> u32 { row[5].trim_start_matches("$ ").parse().unwrap() };
        assert!(amount(cells(&app, 0)) <= amount(cells(&app, 1)));
        app.click(monto());
        assert_eq!(app.app().tabla.sort, Some((4, SortDirection::Descending)));
        assert!(amount(cells(&app, 0)) >= amount(cells(&app, 1)));

        // A click selects a client, which keeps its place through sorting.
        app.click(By::test_id("tabla.clientes.row.1"));
        let chosen = app.app().tabla.rows()[1];
        assert_eq!(app.app().cliente, Some(chosen));
        app.click(monto());
        assert_eq!(app.app().cliente, Some(chosen));
        let name = app.app().clientes[chosen].nombre.clone();
        assert_eq!(
            app.screen().get_by_test_id("tabla.seleccion").name(),
            Some(format!("Seleccionado: {name}"))
        );

        // Clearing the search shows the ten thousand again.
        app.click(By::test_id("barra.buscar.clear"));
        assert_eq!(count(&app), Some(10_000));
    }

    #[test]
    fn exporting_saves_the_rows_the_table_shows_where_the_person_says() {
        let mut app = TestApp::new(Demo::at(Seccion::Tabla));
        app.type_text(By::test_id("barra.buscar"), "temuco");
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
        assert!(!app.app().exportando);
        assert!(std::fs::read(&path).unwrap().starts_with(b"PK"));
        let rows = app.app().tabla.len();
        assert!(rows > 0 && rows < 10_000);
        app.screen().get(By::text("Excel exportado"));
        app.screen()
            .get(By::text(format!("{rows} filas en clientes.xlsx")));

        // The command does the same, from the menu, the palette or an agent.
        std::fs::remove_file(&path).unwrap();
        assert!(app.perform("tabla.exportar"));
        app.run_until_idle();
        assert!(path.exists());
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn the_columns_button_hides_a_column_and_the_search_still_looks_in_it() {
        let mut app = TestApp::new(Demo::at(Seccion::Tabla));
        let table = By::test_id("tabla.clientes");
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

        app.type_text(By::test_id("barra.buscar"), "temuco");
        let count = app.screen().get(table).node().row_count.unwrap();
        assert!(
            count > 0 && count < 10_000,
            "Temuco is in the hidden column"
        );
    }

    #[test]
    fn a_client_row_opens_its_detail_and_the_export_leaves_it_out() {
        let mut app = TestApp::new(Demo::at(Seccion::Tabla));
        app.click(By::test_id("tabla.clientes.row.0"));
        app.press(Key::ArrowRight, Modifiers::NONE);
        let cliente = super::cliente(1);
        let detail = app.screen().get(By::role(Role::Region));
        assert!(
            detail
                .name()
                .unwrap()
                .starts_with(&format!("1 {}", cliente.nombre))
        );
        let text = By::text(format!("Cliente N.º 1 de {}", cliente.ciudad));
        assert_eq!(app.screen().all(text.clone()).len(), 1);

        app.press(Key::Tab, Modifiers::NONE);
        assert!(app.is_focused(By::test_id("tabla.ficha.1")));
        app.press(Key::Space, Modifiers::NONE);
        assert_eq!(app.screen().all(text).len(), 2, "and the toast");
        let sheet = app
            .app()
            .tabla
            .export(&app.app().clientes, &super::columnas());
        assert_eq!(
            sheet.columns,
            ["N.º", "Nombre", "Ciudad", "Plan", "Monto"],
            "no chevron, panel or actions"
        );
    }

    #[test]
    fn the_filter_button_shows_a_field_per_column_and_typing_waits_for_a_pause() {
        let mut app = TestApp::new(Demo::at(Seccion::Tabla));
        let table = By::test_id("tabla.clientes");
        let count = |app: &TestApp<Demo>| app.screen().get(table.clone()).node().row_count;
        let filtrar = By::role(Role::Button).name("Filtrar");
        assert!(
            app.screen()
                .query(By::role(Role::SearchInput).name("Filtrar Ciudad"))
                .is_none()
        );

        app.click(filtrar.clone());
        assert_eq!(
            app.screen().get(filtrar.clone()).node().control_state,
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

        app.type_text(By::test_id("tabla.clientes.filter.2"), "temuco");
        assert_eq!(count(&app), Some(10_000), "the table waits for a pause");
        app.advance(super::PAUSA_FILTRO);
        app.type_text(By::test_id("tabla.clientes.filter.3"), "pro");
        app.advance(super::PAUSA_FILTRO);
        let expected = app
            .app()
            .clientes
            .iter()
            .filter(|c| c.ciudad == "Temuco" && c.plan == "Pro")
            .count();
        assert!(expected > 0);
        assert_eq!(count(&app), Some(expected));
        assert!(
            app.screen()
                .get_by_test_id("tabla.total")
                .name()
                .unwrap()
                .contains("filtrados")
        );

        // Turning the fields off clears the filters.
        app.click(filtrar);
        assert!(!app.app().tabla.filtering);
        assert!(app.app().tabla.filters.is_empty());
        assert_eq!(count(&app), Some(10_000));
    }

    #[test]
    fn dialogs_trap_focus_and_confirming_shows_a_toast() {
        let mut app = TestApp::new(Demo::at(Seccion::Dialogos));
        let dialog = || By::role(Role::Dialog);
        app.click(By::test_id("dialogos.guardar"));
        assert_eq!(
            app.screen().get(dialog()).name().as_deref(),
            Some("¿Guardar los cambios?")
        );
        assert!(app.is_focused(By::test_id("dialogo.cancel")));
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
        assert!(app.is_focused(By::test_id("dialogos.guardar")));
        assert!(app.app().toasts.is_empty(), "cancel shows nothing");

        app.click(By::test_id("dialogos.eliminar"));
        let default = By::test_id("dialogo.default");
        assert_eq!(
            app.screen().get(default.clone()).name().as_deref(),
            Some("Eliminar")
        );
        app.click(default);
        assert!(app.screen().query(dialog()).is_none());
        let screen = app.screen();
        let toast = screen.get_by_test_id("kai.aviso.1");
        assert_eq!(toast.role(), Role::Status);
        screen.within(toast).get(By::text("Borrador eliminado"));
    }

    #[test]
    fn toasts_close_on_their_own_or_with_their_button() {
        let mut app = TestApp::new(Demo::at(Seccion::Dialogos));
        app.click(By::test_id("avisos.exito"));
        app.click(By::test_id("avisos.error"));
        assert_eq!(
            app.screen().get_by_test_id("kai.aviso.1").role(),
            Role::Status
        );
        assert_eq!(
            app.screen().get_by_test_id("kai.aviso.2").role(),
            Role::Alert,
            "errors are read at once"
        );
        app.screen().get_by_test_id("kai.aviso.2.action");
        app.advance(crate::components::TOAST_DURATION);
        assert!(app.screen().query(By::test_id("kai.aviso.1")).is_none());
        assert!(
            app.screen().query(By::test_id("kai.aviso.2")).is_some(),
            "an error stays until closed"
        );
        app.click(By::test_id("kai.aviso.2.close"));
        assert!(app.app().toasts.is_empty());

        assert_eq!(
            app.call("avisos.mostrar", json!({"variante": "advertencia"})),
            Ok(json!(null))
        );
        assert_eq!(
            app.app().toasts[0].variant,
            crate::components::AlertVariant::Warning
        );
        assert!(
            app.call("avisos.mostrar", json!({"variante": "rojo"}))
                .is_err()
        );
        app.click(By::test_id("kai.aviso.3.close"));
        assert!(app.app().toasts.is_empty());
        assert_eq!(app.session().next_deadline(), None, "its timer is gone too");
    }

    #[test]
    fn the_palette_link_opens_the_palette() {
        let mut app = TestApp::new(Demo::at(Seccion::Dialogos));
        let link = By::test_id("dialogos.paleta");
        assert_eq!(
            app.screen().get(link.clone()).name().as_deref(),
            Some("Abrir la paleta (⇧⌘P)")
        );
        app.click(link.clone());
        app.screen().get_by_test_id("kai.paleta");
        assert!(app.is_focused(By::test_id("kai.paleta.buscar")));
        app.press(Key::Escape, Modifiers::NONE);
        assert!(app.screen().query(By::test_id("kai.paleta")).is_none());
        assert!(app.is_focused(link));
    }

    #[test]
    fn the_all_checkbox_is_mixed_and_sets_every_permission() {
        let mut app = TestApp::new(Demo::at(Seccion::Seleccion));
        let todos = By::test_id("seleccion.todos");
        let state = |app: &TestApp<Demo>, by: By| app.screen().get(by).node().control_state;
        assert_eq!(state(&app, todos.clone()), Some(ControlState::Mixed));
        app.click(todos.clone());
        assert_eq!(app.app().permisos, [true; 3]);
        assert_eq!(state(&app, todos.clone()), Some(ControlState::On));
        app.click(todos.clone());
        assert_eq!(app.app().permisos, [false; 3]);
        app.click(By::test_id("seleccion.permiso.1"));
        assert_eq!(state(&app, todos.clone()), Some(ControlState::Mixed));
        assert!(!app.is_enabled(By::test_id("seleccion.administrar")));
        assert!(!app.is_enabled(By::test_id("seleccion.bluetooth")));

        app.click(By::test_id("seleccion.pais"));
        app.click(By::role(Role::MenuItem).name("Perú"));
        assert_eq!(app.app().pais, Some(2));
        app.click(By::test_id("seleccion.alineacion.2"));
        assert_eq!(app.app().alineacion, 2);
        app.click(By::test_id("seleccion.wifi"));
        assert!(!app.app().wifi);
    }

    #[test]
    fn scripts_and_agents_change_the_section_with_ui_ir_a() {
        let mut app = TestApp::new(Demo::default());
        for seccion in Seccion::ALL.into_iter().rev() {
            assert_eq!(
                app.call("ui.ir_a", json!({"seccion": seccion.key()})),
                Ok(json!(null))
            );
            assert_eq!(app.app().seccion, seccion);
            app.screen().get(By::test_id(landmark(seccion)));
        }
        app.call("ui.ir_a", json!({"seccion": "botones"})).unwrap();
        assert_eq!(
            app.screen()
                .get(sidebar_row("Botones"))
                .node()
                .control_state,
            Some(ControlState::On)
        );
        assert!(app.call("ui.ir_a", json!({"seccion": "mapas"})).is_err());
        assert!(app.call("ui.ir_a", json!({})).is_err());
        assert_eq!(app.app().seccion, Seccion::Botones);
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
        let toggle = By::test_id("barra.sidebar");
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
        assert_eq!(app.call("ui.alternar_sidebar", json!({})), Ok(json!(null)));
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
        let mut before = Demo::at(Seccion::Mapa);
        before.seleccionar("AOP26000002");
        let state = before.dev_state().unwrap();

        let mut after = Demo::default();
        after.restore_dev_state(state);
        let app = TestApp::builder(after).viewport(560.0, 700.0).build();
        assert_eq!(app.app().seccion, Seccion::Mapa);
        assert!(!app.app().sidebar_collapsed);
        assert_eq!(app.app().area.as_deref(), Some("AOP26000002"));
        assert!(app.screen().query(By::test_id("mapa")).is_some());

        let mut older = Demo::default();
        older.restore_dev_state(json!({ "seccion": "tabla", "otra_cosa": 1 }));
        assert_eq!(older.seccion, Seccion::Tabla);
        assert!(!older.sized, "nothing said about the sidebar");
        let mut broken = Demo::default();
        broken.restore_dev_state(json!({ "seccion": "no_existe" }));
        assert_eq!(broken, Demo::default());
    }

    /// The content is keyed by section, so a section never inherits the
    /// scroll of the one before it.
    #[test]
    fn each_section_opens_at_the_top() {
        let short = |demo| TestApp::builder(demo).viewport(1024.0, 400.0).build();
        let password =
            |app: &TestApp<Demo>| app.screen().get(By::test_id("form.password")).bounds().y;
        let top = |app: &TestApp<Demo>| app.screen().get(By::test_id("form.limpiar")).bounds().y;
        let campos = password(&short(Demo::at(Seccion::Campos)));
        let mut app = short(Demo::default());
        let at_rest = top(&app);
        for _ in 0..20 {
            app.press(Key::Tab, Modifiers::NONE);
            if app.is_focused(By::test_id("form.limpiar")) {
                break;
            }
        }
        let scrolled = top(&app);
        assert!(scrolled < at_rest, "focusing «Limpiar» scrolls to it");
        app.click(sidebar_row("Campos"));
        assert_eq!(password(&app), campos);
        app.click(sidebar_row("Inicio"));
        assert_eq!(top(&app), at_rest);
        // Also after scrolling a long section with the wheel.
        app.click(sidebar_row("Tabla"));
        let table = |app: &TestApp<Demo>| app.screen().get_by_test_id("tabla").bounds().y;
        let fresh = table(&app);
        // Over the caption, outside the table's own scroll.
        app.scroll_wheel(By::test_id("tabla.total"), -200.0);
        assert!(table(&app) < fresh);
        app.click(sidebar_row("Badges y alertas"));
        app.click(sidebar_row("Tabla"));
        assert_eq!(table(&app), fresh);
    }

    #[test]
    fn return_in_the_name_field_submits_through_the_default_button() {
        let mut app = TestApp::new(Demo::default());
        app.type_text(By::test_id("form.nombre"), "Ana");
        app.press(Key::Enter, Modifiers::NONE);
        assert_eq!(app.messages().last(), Some(&Msg::Submit));
        assert_eq!(app.app().submissions, ["Ana"]);
    }

    #[test]
    fn tab_moves_through_the_form_in_order_showing_the_ring() {
        let mut app = TestApp::new(Demo::default());
        app.click(By::test_id("form.nombre"));
        assert!(app.is_focused(By::test_id("form.nombre")));
        assert!(!app.focus_visible(), "a click does not show the ring");

        app.press(Key::Tab, Modifiers::NONE);
        assert!(app.is_focused(By::test_id("form.correo")));
        assert!(app.focus_visible());

        let shift = Modifiers {
            shift: true,
            ..Modifiers::NONE
        };
        app.press(Key::Tab, shift);
        assert!(app.is_focused(By::test_id("form.nombre")));
    }

    #[test]
    fn the_password_eye_toggles_and_leaves_focus_in_the_field() {
        let mut app = TestApp::new(Demo::at(Seccion::Campos));
        app.type_text(By::test_id("form.password"), "abc");
        assert!(!app.app().show_password);
        app.click(By::test_id("form.password.toggle"));
        assert!(app.app().show_password);
        assert!(app.is_focused(By::test_id("form.password")));
        assert_eq!(app.value(By::test_id("form.password")), "abc");
    }

    #[test]
    fn availability_follows_update() {
        let mut app = TestApp::new(Demo::default());
        let ids: Vec<&str> = app.commands().iter().map(|c| c.id).collect();
        assert_eq!(
            ids,
            [
                "contador.incrementar",
                "contador.sumar",
                "contador.reiniciar",
                "form.enviar",
                "form.limpiar",
                "form.alternar_contrasena",
                "form.ultimo_envio",
                "ui.ir_a",
                "ui.alternar_sidebar",
                "avisos.mostrar",
                "multimedia.cargar_url",
                "mapa.set_view",
                "mapa.fit_bounds",
                "areas.seleccionar",
                "areas.editar_geometria",
                "mapa.get_view",
                "botones.simular_espera",
                "tabla.exportar",
                "format.bold",
                "format.italic",
                "para.bullets",
                "insert.table",
                "insert.picture",
                "documento.abrir",
                "documento.guardar",
                "documento.exportar_pdf",
                "edit.undo",
                "edit.redo",
                "kai.paleta",
            ]
        );
        assert_eq!(
            app.available::<Reiniciar>(),
            Availability::disabled("El contador ya está en cero")
        );
        app.click(By::test_id("barra.sumar"));
        assert!(app.available::<Reiniciar>().is_enabled());
        assert!(!app.available::<Enviar>().is_enabled());
        app.type_text(By::test_id("form.nombre"), "Ana");
        assert!(app.available::<Enviar>().is_enabled());
    }

    #[test]
    fn invoke_runs_enabled_actions_and_reports_disabled_ones() {
        let mut app = TestApp::new(Demo::default());
        let error = app.invoke(Reiniciar).unwrap_err();
        assert_eq!(error.message, "El contador ya está en cero");
        assert_eq!(app.messages(), [], "a disabled action sends nothing");

        app.invoke(Incrementar).unwrap();
        app.invoke(Incrementar).unwrap();
        assert_eq!(app.app().clicks, 2);
        let screen = app.screen();
        screen
            .within(screen.get_by_test_id("contador"))
            .get(By::role(Role::Status).name("2"));
        app.invoke(Reiniciar).unwrap();
        assert_eq!(app.messages(), [Msg::Increment, Msg::Increment, Msg::Reset]);
        assert_eq!(app.app().clicks, 0);
    }

    #[test]
    fn menu_bar_follows_the_registry() {
        let app = TestApp::new(Demo::default());
        let model = menu_model(app.commands());
        let titles: Vec<_> = model.iter().map(|group| group.title).collect();
        assert_eq!(
            titles,
            ["Contador", "Formulario", "Ver", "Botones", "Archivo", "Documento"]
        );
        assert_eq!(
            model[0].items[1],
            MenuEntry {
                id: "contador.reiniciar",
                title: "Reiniciar",
                enabled: false,
                shortcut: Some(KeyEquivalent::cmd(Key::Char('0'))),
            }
        );
        let ver: Vec<_> = model[2].items.iter().map(|item| item.id).collect();
        assert_eq!(ver, ["ui.alternar_sidebar", PALETTE]);
        assert_eq!(
            model[2].items[0].shortcut,
            Some(KeyEquivalent::ctrl_cmd(Key::Char('s')))
        );
        assert!(model[2].items[1].enabled);
    }

    const CMD: Modifiers = Modifiers {
        command: true,
        ..Modifiers::NONE
    };

    #[test]
    fn command_shortcut_sends_the_same_msg_as_the_button() {
        let mut app = TestApp::new(Demo::default());
        app.press(Key::Char('i'), CMD);
        assert_eq!(app.messages(), [Msg::Increment]);
        app.click(By::role(Role::Button).name("Incrementar"));
        assert_eq!(app.messages(), [Msg::Increment, Msg::Increment]);
        assert_eq!(app.app().clicks, 2);
    }

    #[test]
    fn disabled_command_shortcut_sends_nothing() {
        let mut app = TestApp::new(Demo::default());
        app.press(Key::Char('0'), CMD);
        assert_eq!(app.messages(), [], "nothing to reset");
        app.press(Key::Char('i'), CMD);
        app.press(Key::Char('0'), CMD);
        assert_eq!(app.messages(), [Msg::Increment, Msg::Reset]);
    }

    #[test]
    fn command_shortcuts_do_not_steal_copy_in_a_field() {
        let mut app = TestApp::new(Demo::default());
        app.type_text(By::test_id("form.nombre"), "Ana");
        let typed = app.messages().len();
        app.press(Key::A, CMD);
        app.press(Key::C, CMD);
        assert_eq!(app.clipboard().text().as_deref(), Some("Ana"));
        assert_eq!(app.messages().len(), typed, "copying sends nothing");
        // Cmd shortcuts still work while typing.
        app.press(Key::Char('i'), CMD);
        assert_eq!(app.messages().last(), Some(&Msg::Increment));
    }

    #[test]
    fn a_button_follows_its_command() {
        let mut app = TestApp::new(Demo::default());
        let reset = || By::test_id("contador.reiniciar");
        assert!(!app.is_enabled(reset()));
        assert_eq!(
            app.screen().get(reset()).node().description.as_deref(),
            Some("El contador ya está en cero")
        );
        assert_eq!(
            app.screen().get(reset()).name().as_deref(),
            Some("Reiniciar")
        );
        app.click(reset());
        assert_eq!(app.messages(), []);
        app.click(By::test_id("barra.sumar"));
        assert!(app.is_enabled(reset()));
        assert_eq!(app.screen().get(reset()).node().description, None);
        app.click(reset());
        assert_eq!(app.messages().last(), Some(&Msg::Reset));
        assert_eq!(app.app().clicks, 0);
    }

    const CMD_SHIFT: Modifiers = Modifiers {
        command: true,
        shift: true,
        ..Modifiers::NONE
    };

    fn row(id: &str) -> By {
        By::test_id(format!("kai.paleta.{id}"))
    }

    fn search() -> By {
        By::test_id("kai.paleta.buscar")
    }

    #[test]
    fn palette_opens_with_focus_in_search_and_filters() {
        let mut app = TestApp::new(Demo::default());
        app.press(Key::Char('p'), CMD_SHIFT);
        assert!(app.session().palette_open());
        assert!(app.is_focused(search()));
        assert_eq!(app.session().unknown_classes(), Vec::<&str>::new());
        let dialog = app
            .screen()
            .get(By::role(Role::Dialog).name("Paleta de comandos"))
            .test_id()
            .map(str::to_owned);
        assert_eq!(dialog.as_deref(), Some("kai.paleta"));
        let rows = |app: &TestApp<Demo>| -> Vec<String> {
            let screen = app.screen();
            let panel = screen.get(By::test_id("kai.paleta"));
            screen
                .within(panel)
                .all(By::role(Role::Row))
                .iter()
                .filter_map(|found| found.test_id().map(str::to_owned))
                .collect()
        };
        assert_eq!(
            rows(&app).len(),
            18,
            "every app action, not kai.* or queries"
        );
        app.type_text(search(), "REIN");
        assert_eq!(rows(&app), ["kai.paleta.contador.reiniciar"]);
        assert_eq!(app.value(search()), "REIN");
        app.type_text(search(), "zzz");
        assert_eq!(rows(&app), Vec::<String>::new());
        assert!(app.screen().query(By::text("Sin resultados")).is_some());
    }

    #[test]
    fn disabled_command_stays_disabled_in_button_palette_and_menu() {
        let mut app = TestApp::new(Demo::default());
        app.press(Key::Char('p'), CMD_SHIFT);
        let reset = row("contador.reiniciar");
        assert!(!app.is_enabled(By::test_id("contador.reiniciar")));
        assert!(!app.is_enabled(reset.clone()));
        assert_eq!(
            app.screen().get(reset).node().description.as_deref(),
            Some("El contador ya está en cero")
        );
        let menu = menu_model(app.commands());
        let entry = menu[0]
            .items
            .iter()
            .find(|item| item.id == "contador.reiniciar");
        assert_eq!(entry.map(|item| item.enabled), Some(false));
    }

    #[test]
    fn enter_runs_the_selected_command_only_when_enabled() {
        let mut app = TestApp::new(Demo::default());
        app.press(Key::Char('p'), CMD_SHIFT);
        app.type_text(search(), "rein");
        app.press(Key::Enter, Modifiers::NONE);
        assert_eq!(app.messages(), [], "Reiniciar is disabled");
        assert!(app.session().palette_open());

        app.press(Key::Escape, Modifiers::NONE);
        app.click(By::test_id("barra.sumar"));
        app.press(Key::Char('p'), CMD_SHIFT);
        assert_eq!(app.value(search()), "", "a new palette starts empty");
        app.type_text(search(), "rein");
        app.press(Key::Enter, Modifiers::NONE);
        assert_eq!(app.messages(), [Msg::Increment, Msg::Reset]);
        assert!(!app.session().palette_open());
        assert_eq!(app.app().clicks, 0);
    }

    #[test]
    fn arrows_move_the_selection() {
        let mut app = TestApp::new(Demo::default());
        app.press(Key::Char('p'), CMD_SHIFT);
        app.press(Key::ArrowUp, Modifiers::NONE);
        app.press(Key::Enter, Modifiers::NONE);
        assert_eq!(app.messages(), [Msg::Increment], "first row: Incrementar");
        app.press(Key::Char('p'), CMD_SHIFT);
        app.press(Key::ArrowDown, Modifiers::NONE);
        app.press(Key::Enter, Modifiers::NONE);
        assert_eq!(
            app.messages(),
            [Msg::Increment, Msg::Reset],
            "second row: Reiniciar"
        );
    }

    #[test]
    fn command_z_undoes_typing_in_a_field_and_the_app_hears_it() {
        let mut app = TestApp::new(Demo::default());
        let name = By::test_id("form.nombre");
        app.type_text(name.clone(), "Ana María");
        let command = Modifiers {
            command: true,
            ..Modifiers::NONE
        };
        app.press(Key::Char('z'), command);
        assert_eq!(app.app().name, "Ana ", "word by word");
        assert_eq!(app.value(name.clone()), "Ana ");
        app.press(Key::Char('z'), command);
        assert_eq!(app.app().name, "");
        app.press(Key::Char('z'), CMD_SHIFT);
        app.press(Key::Char('z'), CMD_SHIFT);
        assert_eq!(app.app().name, "Ana María");
    }

    #[test]
    fn assistive_tech_reads_the_selected_row_while_focus_stays_in_search() {
        let mut app = TestApp::new(Demo::default());
        app.press(Key::Char('p'), CMD_SHIFT);
        let active = |app: &TestApp<Demo>| {
            let search = app.screen().get(search()).id();
            let update = app.session().a11y_tree(1.0);
            let node = |id| {
                update
                    .nodes
                    .iter()
                    .find(|(node, _)| *node == id)
                    .map(|(_, node)| node.clone())
                    .unwrap()
            };
            let row = node(crate::a11y::node_id(search)).active_descendant();
            let row = row.map(|row| (row, node(row).role(), node(row).is_selected()));
            (row, update.focus == crate::a11y::node_id(search))
        };
        let first = app.screen().get(row("contador.incrementar")).id();
        let second = app.screen().get(row("contador.reiniciar")).id();
        assert_eq!(
            active(&app),
            (
                Some((
                    crate::a11y::node_id(first),
                    accesskit::Role::Row,
                    Some(true)
                )),
                true
            )
        );
        app.press(Key::ArrowDown, Modifiers::NONE);
        assert_eq!(
            active(&app),
            (
                Some((
                    crate::a11y::node_id(second),
                    accesskit::Role::Row,
                    Some(true)
                )),
                true
            ),
            "the row changes, the focus does not"
        );
        assert!(app.is_focused(search()));
        assert_eq!(
            app.screen()
                .get(row("contador.incrementar"))
                .node()
                .control_state,
            Some(crate::element::ControlState::Off)
        );
    }

    #[test]
    fn escape_gives_focus_back_and_app_ids_do_not_change() {
        let mut app = TestApp::new(Demo::default());
        let name = By::test_id("form.nombre");
        app.click(name.clone());
        let reset_id = app.screen().get(By::test_id("contador.reiniciar")).id();
        app.press(Key::Char('p'), CMD_SHIFT);
        assert!(!app.is_focused(name.clone()));
        assert_eq!(
            app.screen().get(By::test_id("contador.reiniciar")).id(),
            reset_id
        );
        app.press(Key::Escape, Modifiers::NONE);
        assert!(!app.session().palette_open());
        assert!(app.is_focused(name));
        assert!(app.screen().query(By::test_id("kai.paleta")).is_none());

        app.press(Key::Char('p'), CMD_SHIFT);
        app.press(Key::Char('p'), CMD_SHIFT);
        assert!(!app.session().palette_open(), "its shortcut toggles it");
    }

    #[test]
    fn tab_stays_inside_the_palette() {
        let mut app = TestApp::new(Demo::default());
        app.press(Key::Char('p'), CMD_SHIFT);
        for _ in 0..8 {
            app.press(Key::Tab, Modifiers::NONE);
            let focused = app.session().focused().expect("something has focus");
            let test_id = app
                .screen()
                .by_id(focused)
                .and_then(|found| found.test_id());
            assert!(
                test_id.is_some_and(|id| id.starts_with("kai.paleta.")),
                "focus left the palette: {test_id:?}"
            );
        }
    }

    #[test]
    fn clicks_run_a_row_or_close_on_the_backdrop() {
        let mut app = TestApp::new(Demo::default());
        app.press(Key::Char('p'), CMD_SHIFT);
        app.click(row("contador.incrementar"));
        assert_eq!(app.messages(), [Msg::Increment]);
        assert!(!app.session().palette_open());

        app.press(Key::Char('p'), CMD_SHIFT);
        app.click_point(crate::geometry::Point { x: 4.0, y: 4.0 });
        assert!(!app.session().palette_open());
        assert_eq!(
            app.messages(),
            [Msg::Increment],
            "the click hit nothing below"
        );
    }

    /// A way to run a command: as a script, with the mouse, the keyboard,
    /// the menu bar or the palette.
    #[derive(Clone, Copy, Debug)]
    enum Via {
        Invoke,
        /// `call(id, {})`, as an agent will.
        Json,
        Button(&'static str),
        Key(Key, Modifiers),
        Menu,
        /// Open the palette, type this and press Return.
        Palette(&'static str),
    }

    struct Case {
        id: &'static str,
        setup: fn(&mut TestApp<Demo>),
        invoke: fn(&mut TestApp<Demo>),
        vias: Vec<Via>,
    }

    /// Run `via` on a fresh demo after `setup`; the model and the messages
    /// sent by `via` alone.
    fn run_via(case: &Case, via: Via) -> (Demo, Vec<Msg>) {
        let mut app = TestApp::new(Demo::default());
        (case.setup)(&mut app);
        let before = app.messages().len();
        match via {
            Via::Invoke => (case.invoke)(&mut app),
            Via::Json => {
                let _ = app.call(case.id, json!({}));
            }
            Via::Button(test_id) => app.click(By::test_id(test_id)),
            Via::Key(key, modifiers) => app.press(key, modifiers),
            Via::Menu => {
                let menu = menu_model(app.commands());
                let entry = menu
                    .iter()
                    .flat_map(|group| &group.items)
                    .find(|e| e.id == case.id);
                assert!(entry.is_some(), "{} no está en el menú", case.id);
                app.perform(case.id);
            }
            Via::Palette(query) => {
                app.press(Key::Char('p'), CMD_SHIFT);
                app.type_text(search(), query);
                app.press(Key::Enter, Modifiers::NONE);
            }
        }
        (app.app().clone(), app.messages()[before..].to_vec())
    }

    fn type_name(app: &mut TestApp<Demo>) {
        app.type_text(By::test_id("form.nombre"), "Ana");
    }

    fn cases() -> Vec<Case> {
        vec![
            Case {
                id: "contador.incrementar",
                setup: |_| {},
                invoke: |app| app.invoke(Incrementar).unwrap(),
                vias: vec![
                    Via::Invoke,
                    Via::Json,
                    Via::Button("barra.sumar"),
                    Via::Button("contador.incrementar"),
                    Via::Key(Key::Char('i'), CMD),
                    Via::Menu,
                    Via::Palette("Incrementar"),
                ],
            },
            Case {
                id: "contador.reiniciar",
                setup: |app| app.invoke(Incrementar).unwrap(),
                invoke: |app| app.invoke(Reiniciar).unwrap(),
                vias: vec![
                    Via::Invoke,
                    Via::Json,
                    Via::Button("contador.reiniciar"),
                    Via::Key(Key::Char('0'), CMD),
                    Via::Menu,
                    Via::Palette("Reiniciar"),
                ],
            },
            Case {
                id: "form.enviar",
                setup: type_name,
                invoke: |app| app.invoke(Enviar).unwrap(),
                vias: vec![
                    Via::Invoke,
                    Via::Json,
                    Via::Button("form.enviar"),
                    // Return in «Nombre»: «Enviar» is the default button.
                    Via::Key(Key::Enter, Modifiers::NONE),
                    Via::Menu,
                    Via::Palette("Enviar"),
                ],
            },
            Case {
                id: "form.limpiar",
                setup: type_name,
                invoke: |app| app.invoke(Limpiar).unwrap(),
                vias: vec![
                    Via::Invoke,
                    Via::Json,
                    Via::Button("form.limpiar"),
                    Via::Menu,
                    Via::Palette("Limpiar"),
                ],
            },
            Case {
                id: "form.alternar_contrasena",
                // The password field is in «Campos».
                setup: |app| app.click(By::test_id("nav.1")),
                invoke: |app| app.invoke(AlternarContrasena).unwrap(),
                vias: vec![
                    Via::Invoke,
                    Via::Json,
                    Via::Button("form.password.toggle"),
                    Via::Menu,
                    Via::Palette("contraseña"),
                ],
            },
        ]
    }

    #[test]
    fn every_action_does_the_same_from_every_via() {
        for case in cases() {
            let mut setup = TestApp::new(Demo::default());
            (case.setup)(&mut setup);
            let before = setup.app().clone();
            let (expected, messages) = run_via(&case, Via::Invoke);
            assert_eq!(messages.len(), 1, "{}: un mensaje", case.id);
            assert_ne!(expected, before, "{} cambia el modelo", case.id);
            for &via in &case.vias[1..] {
                let (model, sent) = run_via(&case, via);
                assert_eq!(sent, messages, "{} vía {via:?}", case.id);
                assert_eq!(model, expected, "{} vía {via:?}", case.id);
            }
        }
    }

    #[test]
    fn disabled_actions_do_nothing_from_any_via() {
        let disabled = [
            Case {
                id: "contador.reiniciar",
                setup: |_| {},
                invoke: |app| {
                    let error = app.invoke(Reiniciar).unwrap_err();
                    assert_eq!(error.message, "El contador ya está en cero");
                },
                vias: vec![
                    Via::Invoke,
                    Via::Json,
                    Via::Button("contador.reiniciar"),
                    Via::Key(Key::Char('0'), CMD),
                    Via::Menu,
                    Via::Palette("Reiniciar"),
                ],
            },
            Case {
                id: "form.enviar",
                setup: |app| app.click(By::test_id("form.nombre")),
                invoke: |app| {
                    let error = app.invoke(Enviar).unwrap_err();
                    assert_eq!(error.message, "Falta el nombre");
                },
                vias: vec![
                    Via::Invoke,
                    Via::Json,
                    Via::Button("form.enviar"),
                    Via::Key(Key::Enter, Modifiers::NONE),
                    Via::Menu,
                    Via::Palette("Enviar"),
                ],
            },
        ];
        for case in disabled {
            let mut setup = TestApp::new(Demo::default());
            (case.setup)(&mut setup);
            let before = setup.app().clone();
            for &via in &case.vias {
                let (model, sent) = run_via(&case, via);
                assert_eq!(sent, [], "{} vía {via:?}", case.id);
                assert_eq!(model, before, "{} vía {via:?}", case.id);
            }
        }
    }

    #[test]
    fn json_calls_run_actions_and_queries() {
        let mut app = TestApp::new(Demo::default());
        assert_eq!(
            app.call("contador.sumar", json!({"cantidad": 3})),
            Ok(json!(null))
        );
        assert_eq!(app.app().clicks, 3);
        assert_eq!(app.messages(), [Msg::Add(3)]);
        let error = app.call("contador.sumar", json!({})).unwrap_err();
        assert_eq!(
            error.message,
            "entrada inválida para contador.sumar: missing field `cantidad`"
        );
        assert!(app.call("contador.sumar", json!({"cantidad": -1})).is_err());

        assert_eq!(app.call("form.ultimo_envio", json!({})), Ok(json!(null)));
        let error = app.call("form.enviar", json!({})).unwrap_err();
        assert_eq!(error.message, "Falta el nombre");
        app.type_text(By::test_id("form.nombre"), "Ana");
        // «Enviar» answers with «Último envío», read after its update.
        assert_eq!(app.call("form.enviar", json!({})), Ok(json!("Ana")));
        assert_eq!(app.call("form.ultimo_envio", json!({})), Ok(json!("Ana")));
        assert_eq!(app.call("form.limpiar", json!({})), Ok(json!(null)));
        assert!(app.call("contador.nada", json!({})).is_err());

        let exposed: Vec<&str> = app
            .commands()
            .iter()
            .filter(|command| command.exposed)
            .map(|command| command.id)
            .collect();
        assert_eq!(
            exposed,
            [
                "contador.incrementar",
                "contador.sumar",
                "contador.reiniciar",
                "form.enviar",
                "form.limpiar",
                "form.ultimo_envio",
                "ui.ir_a",
                "ui.alternar_sidebar",
                "avisos.mostrar",
                "multimedia.cargar_url",
                "mapa.set_view",
                "mapa.fit_bounds",
                "areas.seleccionar",
                "areas.editar_geometria",
                "mapa.get_view",
                "botones.simular_espera",
                "tabla.exportar",
                "format.bold",
                "format.italic",
                "para.bullets",
                "insert.table",
                "insert.picture",
                "documento.abrir",
                "documento.guardar",
                "documento.exportar_pdf",
                "edit.undo",
                "edit.redo",
            ]
        );
    }

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

        let mut demo = Demo::at(Seccion::Mapa);
        demo.vista = MapView::new(PLAZA_DE_ARMAS, 15.0);
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
            .find_test_id("mapa.vista")
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
        let mut app = TestApp::new(Demo::at(Seccion::Mapa));
        let vista = By::test_id("mapa.vista");
        app.drag_and_hold(vista.clone(), -100.0, 0.0);
        let [Msg::MapaMovido(moved)] = app.messages() else {
            panic!("{:?}", app.messages());
        };
        assert!(moved.center.lon > PLAZA_DE_ARMAS.lon, "{moved:?}");
        assert_eq!(app.app().vista, *moved);
        assert_eq!(
            app.call("mapa.get_view", json!({})),
            Ok(serde_json::to_value(app.app().vista).unwrap())
        );

        let terminal = MapView::new(LatLon::new(-36.155, -71.82), 15.0);
        let call = json!({
            "center": {"lat": -36.155, "lon": -71.82},
            "zoom": 15,
            "animate": false,
        });
        assert_eq!(app.call("mapa.set_view", call), Ok(json!(null)));
        assert_eq!(app.app().vista, terminal);
        assert!(!app.session().animating(), "it jumps");
        app.screen()
            .get(By::role(Role::Status).name("-36.1550, -71.8200 · zoom 15.0"));
        let pin = app
            .screen()
            .get(By::role(Role::Image).name("Plaza de Armas"))
            .bounds();
        let map = app.screen().get(vista.clone()).bounds();
        assert!(
            pin.y + pin.height < map.y + map.height / 2.0,
            "the plaza is north of the terminal"
        );

        app.click(By::test_id("mapa.volver"));
        assert!(app.session().animating(), "it glides back");
        app.advance(Duration::from_secs(1));
        assert_eq!(app.app().vista, VISTA_INICIAL);
        assert!(
            !app.messages()
                .iter()
                .skip(1)
                .any(|msg| matches!(msg, Msg::MapaMovido(_))),
            "the app's moves are not reported back"
        );
    }

    /// Where `at` falls in the window on the map of «Mapa».
    fn on_map(app: &TestApp<Demo>, at: LatLon) -> crate::geometry::Point {
        let id = app.screen().get_by_test_id("mapa.vista").id();
        let area = app.session().layout().node(id).unwrap().padding_box;
        let size = Size {
            width: area.width,
            height: area.height,
        };
        let local = crate::geo::Camera::new(app.app().vista, size).to_local(at);
        crate::geometry::Point {
            x: area.x + local.x,
            y: area.y + local.y,
        }
    }

    #[test]
    fn sectors_are_chosen_from_the_list_the_map_and_agents() {
        use crate::style::Cursor;
        let mut app = TestApp::new(Demo::at(Seccion::Mapa));
        let screen = app.screen();
        let lista = screen.get(By::role(Role::List).name("Sectores"));
        let nombres: Vec<String> = screen
            .within(lista)
            .all(By::role(Role::Button))
            .iter()
            .filter_map(|row| row.name())
            .collect();
        assert_eq!(
            nombres,
            [
                "Plaza de Armas",
                "Catillo",
                "Villa San Pedro",
                "Palma Rosa",
                "Terminal de Buses"
            ]
        );

        let catillo = sector_inicial("AOP26000002").unwrap();
        app.click(By::test_id("mapa.sector.AOP26000002"));
        assert_eq!(app.app().area.as_deref(), Some("AOP26000002"));
        assert!(app.session().animating(), "it glides to the sector");
        app.advance(Duration::from_secs(1));
        let vista = app.app().vista;
        assert!(vista.zoom > VISTA_INICIAL.zoom, "{vista:?}");
        let centro = catillo.capa.centroid();
        assert!((vista.center.lat - centro.lat).abs() < 1e-3, "{vista:?}");
        assert!((vista.center.lon - centro.lon).abs() < 1e-3, "{vista:?}");
        app.screen()
            .get(By::text("Sector Catillo · AOP26000002 · 191,7 ha"));
        assert_eq!(
            app.screen()
                .get_by_test_id("mapa.sector.AOP26000002")
                .node()
                .control_state,
            Some(ControlState::On)
        );

        let todos = SECTORES
            .iter()
            .map(|sector| sector.capa.bounds())
            .reduce(LatLonBounds::union)
            .unwrap();
        assert_eq!(
            app.call("mapa.fit_bounds", json!({ "bounds": todos })),
            Ok(json!(null))
        );
        app.advance(Duration::from_secs(1));
        let terminal = sector_inicial("AOP26000005").unwrap().capa.centroid();
        let punto = on_map(&app, terminal);
        app.move_pointer(punto);
        assert_eq!(app.mouse_cursor(), Cursor::Pointer);
        app.click_point(punto);
        assert_eq!(app.app().area.as_deref(), Some("AOP26000005"));
        assert!(
            app.messages()
                .contains(&Msg::SeleccionarArea("AOP26000005".to_owned()))
        );

        assert_eq!(
            app.call("areas.seleccionar", json!({ "code": "AOP99" })),
            Ok(json!(null))
        );
        assert_eq!(app.app().area.as_deref(), Some("AOP26000005"), "unknown");
        assert_eq!(
            app.call("areas.seleccionar", json!({ "code": "AOP26000001" })),
            Ok(json!(null))
        );
        assert_eq!(app.app().area.as_deref(), Some("AOP26000001"));

        app.click(By::role(Role::Switch).name("Sectores"));
        assert!(!app.app().mostrar_areas, "{:?}", app.messages().last());
        app.advance(Duration::from_secs(1));
        let plaza = on_map(&app, PLAZA_DE_ARMAS);
        app.move_pointer(plaza);
        assert_eq!(app.mouse_cursor(), Cursor::Default, "no sectors drawn");
    }

    const CATILLO: &str = "AOP26000002";

    fn anillo(app: &TestApp<Demo>, code: &str) -> Vec<LatLon> {
        app.app().sector(code).unwrap().anillo()
    }

    fn borrador(app: &TestApp<Demo>) -> Vec<LatLon> {
        app.app().edicion.as_ref().unwrap().1.ring().to_vec()
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
    fn editar_catillo(app: &mut TestApp<Demo>) {
        app.click(By::test_id(format!("mapa.sector.{CATILLO}")));
        app.click(By::test_id("mapa.editar"));
        app.advance(Duration::from_secs(1));
        assert!(app.app().edicion.is_some(), "{:?}", app.messages().last());
    }

    fn vertice(n: usize) -> By {
        By::test_id(format!("mapa.vista.vertex.{n}"))
    }

    /// The selected sector takes a color from the palette or typed; while
    /// the hex is incomplete, and with «Sin color», it shows its type's.
    #[test]
    fn the_selected_sector_changes_color_with_the_color_field() {
        let mut app = TestApp::new(Demo::at(Seccion::Mapa));
        let campo = By::role(Role::Group).name("Color del sector");
        assert!(app.screen().query(campo.clone()).is_none(), "no sector yet");
        app.click(By::test_id(format!("mapa.sector.{CATILLO}")));
        let catillo = |app: &TestApp<Demo>| app.app().sector(CATILLO).unwrap().clone();
        let del_tipo = catillo(&app).color();
        assert_eq!(app.value(By::test_id("mapa.color.hex")), del_tipo.to_hex());

        app.click(By::role(Role::Radio).name("Violeta"));
        assert_eq!(catillo(&app).color, "#7c3aed");
        assert_eq!(catillo(&app).swatch(), "bg-[#7c3aed]");

        app.click(By::test_id("mapa.color.clear"));
        app.type_text(By::test_id("mapa.color.hex"), "#12");
        assert_eq!(catillo(&app).color(), del_tipo);
        app.screen().get(By::test_id("mapa.color.error"));
        app.type_text(By::test_id("mapa.color.hex"), "3456");
        assert_eq!(catillo(&app).color(), super::Color::hex(0x123456));

        app.click(By::test_id("mapa.color.clear"));
        assert_eq!(catillo(&app).color(), del_tipo);
        app.click(By::test_id("mapa.editar"));
        assert!(
            app.screen().query(campo).is_none(),
            "hidden while the outline is edited"
        );
    }

    #[test]
    fn a_sector_is_edited_on_the_map_and_kept_only_on_finish() {
        let mut app = TestApp::new(Demo::at(Seccion::Mapa));
        let editar = By::test_id("mapa.editar");
        assert!(
            app.screen().get(editar.clone()).node().disabled,
            "nothing selected"
        );
        editar_catillo(&mut app);
        assert!(app.screen().get(editar.clone()).node().disabled, "editing");
        let antes = anillo(&app, CATILLO);
        let vista = app.app().vista;
        for (n, at) in antes.iter().enumerate() {
            let handle = app.screen().get(vertice(n)).bounds();
            let punto = on_map(&app, *at);
            assert!(
                (handle.x + handle.width / 2.0 - punto.x).abs() < 0.5
                    && (handle.y + handle.height / 2.0 - punto.y).abs() < 0.5,
                "vertex {n}: {handle:?} vs {punto:?}"
            );
        }
        let superficie = |app: &TestApp<Demo>| {
            app.screen()
                .get_by_test_id("mapa.superficie")
                .name()
                .unwrap()
        };
        assert_eq!(superficie(&app), "Superficie 191,7 ha");

        app.drag_and_hold(vertice(0), -30.0, 20.0);
        assert_eq!(app.app().vista, vista, "the map stays put");
        let movido = borrador(&app);
        assert_ne!(movido[0], antes[0]);
        assert_eq!(movido[1..], antes[1..]);
        assert_eq!(
            anillo(&app, CATILLO),
            antes,
            "the sector waits for «Terminar»"
        );
        assert_ne!(superficie(&app), "Superficie 191,7 ha");

        app.click(By::test_id("mapa.sector.AOP26000001"));
        assert_eq!(
            app.app().area.as_deref(),
            Some(CATILLO),
            "no switching mid-edit"
        );

        app.click(By::test_id("mapa.vista.midpoint.0"));
        assert_eq!(borrador(&app).len(), 5);
        app.click(By::test_id("mapa.terminar"));
        assert!(app.app().edicion.is_none());
        let guardado = anillo(&app, CATILLO);
        assert_eq!(guardado.len(), 5);
        assert_same_ring(&[guardado[0]], &[movido[0]]);
        assert!(!app.screen().get(editar.clone()).node().disabled);

        editar_catillo(&mut app);
        app.drag_and_hold(vertice(1), 15.0, 0.0);
        app.press(Key::Escape, Modifiers::NONE);
        assert!(app.app().edicion.is_none(), "Esc cancels");
        assert_same_ring(&anillo(&app, CATILLO), &guardado);

        editar_catillo(&mut app);
        app.drag_and_hold(vertice(1), 15.0, 0.0);
        app.press(
            Key::Char('z'),
            Modifiers {
                command: true,
                ..Modifiers::NONE
            },
        );
        assert_same_ring(&borrador(&app), &guardado);
        app.click(By::test_id("mapa.cancelar"));
        assert!(app.app().edicion.is_none());
        assert_same_ring(&anillo(&app, CATILLO), &guardado);
    }

    #[test]
    fn crossing_edges_block_finish_until_fixed() {
        let mut app = TestApp::new(Demo::at(Seccion::Mapa));
        editar_catillo(&mut app);
        let antes = anillo(&app, CATILLO);
        let terminar = By::test_id("mapa.terminar");
        assert!(!app.screen().get(terminar.clone()).node().disabled);
        assert!(app.screen().all(By::test_id("mapa.cruce")).is_empty());

        let (sur_oeste, sur_este, nor_este) = (
            on_map(&app, antes[0]),
            on_map(&app, antes[1]),
            on_map(&app, antes[2]),
        );
        let al_este = (
            sur_este.x - sur_oeste.x + 30.0,
            (nor_este.y - sur_oeste.y) / 2.0,
        );
        app.drag_and_hold(vertice(0), al_este.0, al_este.1);
        assert!(borrador(&app)[0] != antes[0]);
        assert!(app.screen().get(terminar.clone()).node().disabled);
        assert_eq!(
            app.screen().get_by_test_id("mapa.cruce").name().as_deref(),
            Some("Los bordes se cruzan")
        );
        app.press(Key::Enter, Modifiers::NONE);
        assert!(
            app.app().edicion.is_some(),
            "Return does not take a crossed draft"
        );

        app.press(
            Key::Char('z'),
            Modifiers {
                command: true,
                ..Modifiers::NONE
            },
        );
        assert!(!app.screen().get(terminar).node().disabled);
        app.press(Key::Enter, Modifiers::NONE);
        assert!(app.app().edicion.is_none(), "Return finishes from a vertex");
        assert_same_ring(&anillo(&app, CATILLO), &antes);
    }

    #[test]
    fn agents_replace_a_sector_outline_with_a_valid_ring() {
        let mut app = TestApp::new(Demo::at(Seccion::Mapa));
        let antes = anillo(&app, CATILLO);
        let error = |result: Result<serde_json::Value, CommandError>| {
            result.expect_err("debería fallar").message
        };
        let bow_tie = json!({
            "code": CATILLO,
            "coordinates": [[-71.84, -36.15], [-71.82, -36.13], [-71.82, -36.15], [-71.84, -36.13]],
        });
        let message = error(app.call("areas.editar_geometria", bow_tie));
        assert!(message.contains("se cruzan"), "{message}");
        let unknown = json!({
            "code": "AOP99",
            "coordinates": [[-71.84, -36.15], [-71.82, -36.15], [-71.82, -36.13]],
        });
        let message = error(app.call("areas.editar_geometria", unknown));
        assert!(message.contains("AOP99"), "{message}");
        let short = json!({ "code": CATILLO, "coordinates": [[-71.84, -36.15], [-71.82, -36.15]] });
        assert!(error(app.call("areas.editar_geometria", short)).contains("al menos 3"));
        assert_eq!(anillo(&app, CATILLO), antes, "nothing changed");

        let triangle = json!({
            "code": CATILLO,
            "coordinates": [[-71.84, -36.15], [-71.82, -36.15], [-71.83, -36.13], [-71.84, -36.15]],
        });
        assert_eq!(
            app.call("areas.editar_geometria", triangle),
            Ok(json!(null))
        );
        assert_same_ring(
            &anillo(&app, CATILLO),
            &[
                LatLon::new(-36.15, -71.84),
                LatLon::new(-36.15, -71.82),
                LatLon::new(-36.13, -71.83),
            ],
        );
        assert_eq!(app.app().area.as_deref(), Some(CATILLO), "and selects it");
    }

    fn agent(client: &str) -> Origin {
        Origin::Agent {
            client: client.to_owned(),
        }
    }

    fn denied() -> Option<Result<serde_json::Value, CommandError>> {
        Some(Err(CommandError::new("La persona rechazó «Reiniciar»")))
    }

    const DONE: Option<Result<serde_json::Value, CommandError>> = Some(Ok(serde_json::Value::Null));

    #[test]
    fn agents_confirm_destructive_actions_in_a_dialog() {
        let mut app = TestApp::new(Demo::default());
        app.invoke(Incrementar).unwrap();
        app.invoke(Incrementar).unwrap();

        let reply = app.agent_call("Cursor", "contador.reiniciar", json!({}));
        assert_eq!(reply.result(), None, "waits for the person");
        assert_eq!(app.app().clicks, 2);
        let screen = app.screen();
        let dialog = screen.get(By::role(Role::Dialog).name("¿Permitir «Reiniciar»?"));
        screen
            .within(dialog)
            .get(By::text("Cursor quiere ejecutar contador.reiniciar."));
        assert!(app.is_focused(By::test_id("kai.confirmar.rechazar")));
        assert_eq!(app.session().unknown_classes(), Vec::<&str>::new());
        assert!(!app.perform("contador.incrementar"), "the dialog is modal");
        app.press(Key::Char('p'), CMD_SHIFT);
        assert!(!app.session().palette_open());

        app.click(By::test_id("kai.confirmar.rechazar"));
        assert_eq!(reply.result(), denied());
        assert_eq!(app.app().clicks, 2);
        assert!(app.screen().query(By::role(Role::Dialog)).is_none());
        let last = app.audit_log().last().unwrap();
        assert_eq!(last.command, "contador.reiniciar");
        assert_eq!(last.origin, agent("Cursor"));
        assert_eq!(last.outcome, Outcome::Denied);

        let reply = app.agent_call("Cursor", "contador.reiniciar", json!({}));
        app.press(Key::Escape, Modifiers::NONE);
        assert_eq!(reply.result(), denied(), "Escape rejects");
        assert_eq!(app.app().clicks, 2);

        let reply = app.agent_call("Cursor", "contador.reiniciar", json!({}));
        app.click(By::test_id("kai.confirmar.una_vez"));
        assert_eq!(reply.result(), DONE);
        assert_eq!(app.app().clicks, 0);
        assert_eq!(app.audit_log().last().unwrap().outcome, Outcome::Done);

        app.invoke(Incrementar).unwrap();
        let reply = app.agent_call("Cursor", "contador.reiniciar", json!({}));
        assert_eq!(reply.result(), None, "allowed once asks again");
        app.click(By::test_id("kai.confirmar.sesion"));
        assert_eq!(reply.result(), DONE);
        app.invoke(Incrementar).unwrap();
        let reply = app.agent_call("Cursor", "contador.reiniciar", json!({}));
        assert_eq!(reply.result(), DONE, "allowed for the session");
        assert_eq!(app.app().clicks, 0);
    }

    #[test]
    fn an_agent_wait_command_answers_only_after_the_task_finishes() {
        let mut app = TestApp::new(Demo::at(Seccion::Botones));
        let reply = app.agent_call("Cursor", "botones.simular_espera", json!({}));
        assert!(reply.result().is_none(), "queda abierta mientras corre la espera");
        assert!(app.tasks_pending());
        app.run_until_idle();
        assert_eq!(reply.result(), Some(Ok(json!(null))));
        assert_eq!(app.app().espera, Some("comando"));
        assert_eq!(app.audit_log().last().unwrap().outcome, Outcome::Done);
    }

    #[test]
    fn agent_calls_wait_behind_the_dialog_and_can_be_cancelled() {
        let mut app = TestApp::new(Demo::default());
        app.invoke(Incrementar).unwrap();
        let reset = app.agent_call("Cursor", "contador.reiniciar", json!({}));
        let add = app.agent_call("Cursor", "contador.incrementar", json!({}));
        assert_eq!((reset.result(), add.result()), (None, None));
        assert_eq!(app.app().clicks, 1);

        app.cancel_agent_call(reset.id());
        assert!(reset.result().unwrap().is_err());
        assert!(!app.session().confirmation_open());
        assert_eq!(add.result(), DONE, "the queue moves on");
        assert_eq!(app.app().clicks, 2);
        let outcomes: Vec<_> = app
            .audit_log()
            .entries()
            .map(|entry| (entry.command.as_str(), entry.outcome.clone()))
            .collect();
        assert_eq!(
            outcomes,
            [
                ("contador.reiniciar", Outcome::Cancelled),
                ("contador.incrementar", Outcome::Done),
            ]
        );

        let first = app.agent_call("Cursor", "contador.reiniciar", json!({}));
        let second = app.agent_call("Cursor", "contador.reiniciar", json!({}));
        app.cancel_agent_call(second.id());
        assert!(second.result().unwrap().is_err());
        assert_eq!(first.result(), None);
        app.on_confirmation(Confirm::Deny);
        assert_eq!(first.result(), denied());
        assert_eq!(app.app().clicks, 2);
    }

    #[test]
    fn agent_calls_fail_before_asking_when_they_cannot_run() {
        let mut app = TestApp::new(Demo::default());
        let hidden = app.agent_call("Cursor", "form.alternar_contrasena", json!({}));
        assert_eq!(
            hidden.result(),
            Some(Err(CommandError::new(
                "form.alternar_contrasena no está expuesto a agentes"
            )))
        );
        let reset = app.agent_call("Cursor", "contador.reiniciar", json!({}));
        assert_eq!(
            reset.result(),
            Some(Err(CommandError::new("El contador ya está en cero")))
        );
        let bad = app.agent_call("Cursor", "contador.sumar", json!({"cantidad": "dos"}));
        assert!(bad.result().unwrap().is_err());
        assert!(!app.session().confirmation_open());
        let query = app.agent_call("Cursor", "form.ultimo_envio", json!({}));
        assert_eq!(query.result(), DONE);
        assert_eq!(app.audit_log().len(), 3, "queries are not audited");
        assert!(
            app.audit_log()
                .entries()
                .all(|entry| matches!(entry.outcome, Outcome::Failed(_)))
        );
    }

    #[test]
    fn people_and_scripts_never_see_the_dialog() {
        let mut app = TestApp::new(Demo::default());
        app.invoke(Incrementar).unwrap();
        app.click(By::test_id("contador.reiniciar"));
        assert_eq!(app.app().clicks, 0);
        assert!(!app.session().confirmation_open());
        assert!(app.audit_log().is_empty(), "the person is not audited");

        app.call("contador.sumar", json!({"cantidad": 3})).unwrap();
        app.call("contador.reiniciar", json!({})).unwrap();
        assert!(!app.session().confirmation_open());
        let entries: Vec<_> = app.audit_log().entries().collect();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].origin, Origin::Script);
        assert_eq!(entries[0].input, json!({"cantidad": 3}));
        assert_eq!(entries[1].outcome, Outcome::Done);
    }

    #[test]
    fn the_indicator_shows_the_agent_and_its_last_action() {
        let mut app = TestApp::new(Demo::default());
        assert!(app.screen().query(By::test_id("kai.agente")).is_none());
        app.agent_connected("Cursor");
        app.screen()
            .get(By::role(Role::Status).name("Agente: Cursor"));
        assert_eq!(app.session().unknown_classes(), Vec::<&str>::new());
        app.on_confirmation(Confirm::AllowOnce);
        app.agent_call("Cursor", "contador.incrementar", json!({}));
        app.agent_call("Cursor", "contador.reiniciar", json!({}));
        assert_eq!(app.app().clicks, 0);
        app.screen()
            .get(By::role(Role::Status).name("Agente: Cursor · Reiniciar"));
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

        app.hover(By::test_id("barra.sumar"));
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
            shown.contains("clic → Button #") && shown.contains("«Sumar clic» [barra.sumar]"),
            "{shown}"
        );
        let outline = screen.get(By::test_id("kai.inspector.outline")).bounds();
        let icon = screen
            .within(screen.get(By::test_id("barra.sumar")))
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

        app.click(By::test_id("barra.sumar"));
        assert_eq!(app.app().clicks, 1, "the inspector takes no clicks");

        app.press(Key::Char('p'), CMD_SHIFT);
        assert!(app.is_focused(By::test_id("kai.paleta.buscar")));
        app.press(Key::Escape, Modifiers::NONE);
        assert!(!app.session().palette_open());
        assert!(app.session().inspector_open());

        app.press(Key::Char('i'), CMD_OPTION);
        assert!(app.screen().query(By::test_id("kai.inspector")).is_none());
        assert!(app.dump().contains("[barra.sumar]"));
    }

    #[test]
    fn messages_record_where_they_came_from() {
        let mut app = TestApp::new(Demo::default());
        app.click(By::role(Role::Button).name("Incrementar"));
        app.press(Key::Char('i'), CMD);
        assert!(app.perform("contador.incrementar"));
        app.invoke(Incrementar).unwrap();
        app.call("contador.sumar", json!({"cantidad": 2})).unwrap();
        let button = app.screen().get(By::test_id("contador.incrementar")).id();
        app.accessibility_action(&accesskit::ActionRequest {
            action: accesskit::Action::Click,
            target_tree: accesskit::TreeId::ROOT,
            target_node: crate::a11y::node_id(button),
            data: None,
        });
        app.agent_call("kai-test", "contador.reiniciar", json!({}));
        app.click(By::test_id("kai.confirmar.una_vez"));

        assert_eq!(
            app.messages(),
            [
                Msg::Increment,
                Msg::Increment,
                Msg::Increment,
                Msg::Increment,
                Msg::Add(2),
                Msg::Increment,
                Msg::Reset,
            ]
        );
        assert_eq!(
            app.message_sources(),
            [
                MsgSource::Pointer,
                MsgSource::Keyboard,
                MsgSource::Menu,
                MsgSource::Script,
                MsgSource::Script,
                MsgSource::A11y,
                MsgSource::Agent("kai-test".into()),
            ],
            "an agent's call a click allowed still comes from the agent"
        );
    }

    #[cfg(feature = "mcp")]
    #[test]
    fn mcp_lists_the_exposed_commands_as_tools() {
        let mut app = TestApp::new(Demo::default());
        let mut client = app.mcp_client();
        assert!(client.initialized()["capabilities"]["tools"].is_object());
        let tools = client.tools();
        let described = crate::commands::describe::<Demo>();
        let exposed: Vec<&serde_json::Value> = described["commands"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|command| command["exposed"] == json!(true))
            .collect();
        assert_eq!(tools.len(), exposed.len());
        for (tool, command) in tools.iter().zip(exposed) {
            assert_eq!(tool["name"], command["id"]);
            assert_eq!(tool["title"], command["title"]);
            assert_eq!(tool["inputSchema"], command["inputSchema"]);
            let read_only = command["kind"] == "query";
            assert_eq!(tool["annotations"]["readOnlyHint"], json!(read_only));
            let destructive = command["safety"] == "destructive";
            assert_eq!(tool["annotations"]["destructiveHint"], json!(destructive));
            match command.get("outputSchema") {
                Some(output) if output["type"] == "object" => assert_eq!(
                    tool["outputSchema"], *output,
                    "{} entrega su objeto tal cual",
                    command["id"]
                ),
                Some(output) => assert_eq!(
                    tool["outputSchema"]["properties"]["result"], *output,
                    "{} envuelve su salida",
                    command["id"]
                ),
                None => assert!(tool.get("outputSchema").is_none()),
            }
        }
        let names: Vec<&str> = tools.iter().filter_map(|t| t["name"].as_str()).collect();
        assert!(!names.contains(&"form.alternar_contrasena"));
        assert!(
            client.close(),
            "el servidor termina cuando el cliente se va"
        );
    }

    #[cfg(feature = "mcp")]
    #[test]
    fn mcp_destructive_calls_wait_for_the_person() {
        use crate::testing::MCP_CLIENT;

        let mut app = TestApp::new(Demo::default());
        let mut client = app.mcp_client();
        client
            .app()
            .screen()
            .get(By::role(Role::Status).name(format!("Agente: {MCP_CLIENT}")));
        client
            .call_tool("contador.sumar", json!({"cantidad": 3}))
            .unwrap();
        client
            .app()
            .screen()
            .get(By::role(Role::Status).name(format!("Agente: {MCP_CLIENT} · Sumar")));

        let reset = client.send_call("contador.reiniciar", json!({}));
        let screen = client.app().screen();
        let dialog = screen.get(By::role(Role::Dialog).name("¿Permitir «Reiniciar»?"));
        screen.within(dialog).get(By::text(format!(
            "{MCP_CLIENT} quiere ejecutar contador.reiniciar."
        )));
        client.app().click(By::test_id("kai.confirmar.rechazar"));
        let result = client.finish(reset).unwrap();
        assert_eq!(result["isError"], json!(true));
        assert_eq!(
            result["content"][0]["text"],
            json!("La persona rechazó «Reiniciar»")
        );
        assert_eq!(client.app().app().clicks, 3);
        let last = client.app().audit_log().last().unwrap().clone();
        assert_eq!(last.origin, agent(MCP_CLIENT));
        assert_eq!(last.outcome, Outcome::Denied);

        let reset = client.send_call("contador.reiniciar", json!({}));
        client.cancel(reset);
        assert!(!client.app().confirmation_open());
        assert_eq!(client.app().app().clicks, 3);
        let last = client.app().audit_log().last().unwrap().outcome.clone();
        assert_eq!(last, Outcome::Cancelled);

        client.app().on_confirmation(Confirm::AllowOnce);
        let result = client.call_tool("contador.reiniciar", json!({})).unwrap();
        assert_eq!(result["isError"], json!(false));
        assert_eq!(client.app().app().clicks, 0);
        let last = client.app().audit_log().last().unwrap().clone();
        assert_eq!(
            (last.origin, last.outcome),
            (agent(MCP_CLIENT), Outcome::Done)
        );
        assert!(client.close());
    }

    #[cfg(feature = "mcp")]
    #[test]
    fn mcp_tool_calls_run_on_the_app() {
        let mut app = TestApp::new(Demo::default());
        let mut client = app.mcp_client();

        let result = client
            .call_tool("contador.sumar", json!({"cantidad": 2}))
            .unwrap();
        assert_eq!(result["isError"], json!(false));
        assert_eq!(client.app().app().clicks, 2);
        let screen = client.app().screen();
        screen
            .within(screen.get_by_test_id("contador"))
            .get(By::role(Role::Status).name("2"));

        let reset = client.send_call("contador.reiniciar", json!({}));
        assert!(client.app().confirmation_open());
        client.app().click(By::test_id("kai.confirmar.una_vez"));
        assert_eq!(client.finish(reset).unwrap()["isError"], json!(false));
        assert_eq!(client.app().app().clicks, 0);
        let result = client.call_tool("contador.reiniciar", json!({})).unwrap();
        assert_eq!(result["isError"], json!(true));
        assert_eq!(
            result["content"][0]["text"],
            json!("El contador ya está en cero"),
            "an action that cannot run fails without asking"
        );

        let result = client
            .call_tool("contador.sumar", json!({"cantidad": "dos"}))
            .unwrap();
        assert_eq!(result["isError"], json!(true));
        assert!(
            result["content"][0]["text"]
                .as_str()
                .unwrap()
                .starts_with("entrada inválida para contador.sumar")
        );

        let error = client
            .call_tool("form.alternar_contrasena", json!({}))
            .unwrap_err();
        assert!(error["message"].as_str().unwrap().contains("desconocida"));
        assert_eq!(client.app().app().clicks, 0);

        client.app().type_text(By::test_id("form.nombre"), "Ana");
        let result = client.call_tool("form.enviar", json!({})).unwrap();
        assert_eq!(result["structuredContent"], json!({"result": "Ana"}));
        let result = client.call_tool("form.ultimo_envio", json!({})).unwrap();
        assert_eq!(result["structuredContent"], json!({"result": "Ana"}));
        assert_eq!(result["content"][0]["text"], json!(r#"{"result":"Ana"}"#));
        assert!(client.close());
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
        assert!(ids.contains(&"contador.sumar") && ids.contains(&"form.ultimo_envio"));
        assert!(ids.iter().all(|id| !id.starts_with("kai.")), "{ids:?}");
    }

    #[test]
    fn actions_with_input_stay_out_of_menus_and_palette() {
        let mut app = TestApp::new(Demo::default());
        let sumar = app.commands().iter().find(|c| c.id == "contador.sumar");
        assert!(sumar.is_some_and(|c| c.needs_input));
        let menu = menu_model(app.commands());
        assert!(
            menu.iter()
                .flat_map(|g| &g.items)
                .all(|e| e.id != "contador.sumar")
        );
        app.press(Key::Char('p'), CMD_SHIFT);
        app.type_text(search(), "Sumar");
        assert!(app.screen().query(row("contador.sumar")).is_none());
        assert!(app.screen().query(By::text("Sin resultados")).is_some());
    }

    #[test]
    fn query_reads_the_model_without_msg() {
        let mut app = TestApp::new(Demo::default());
        assert_eq!(app.query(Saludo), Ok(None));
        app.type_text(By::test_id("form.nombre"), "Ana");
        app.invoke(Enviar).unwrap();
        let sent = app.messages().len();
        assert_eq!(app.query(Saludo), Ok(Some("Ana".to_owned())));
        assert_eq!(app.messages().len(), sent, "a query sends nothing");
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
        let content = Screen::new(&layout).get_by_test_id("contenido").id();
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
            let content = Screen::new(&layout).get_by_test_id("contenido");
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
            ..Demo::at(Seccion::Botones)
        };
        let filled = Demo {
            password: "kai-2026".to_owned(),
            ..Demo::at(Seccion::Campos)
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
                                    focused: Some(find(&demo, By::test_id("botones.secondary"))),
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
            ..Demo::at(Seccion::Botones)
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
            seccion: Seccion::Botones,
            ..typed
        };
        snapshot(
            out,
            "demo-variants-focus",
            &buttons,
            wide,
            2.0,
            &Interaction {
                focused: Some(find(&buttons, By::test_id("botones.secondary"))),
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
            ..Demo::at(Seccion::Campos)
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
            ..Demo::at(Seccion::Campos)
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

        for (name, seccion) in [
            ("demo-seleccion", Seccion::Seleccion),
            ("demo-contenedores", Seccion::Contenedores),
            ("demo-tabla", Seccion::Tabla),
            ("demo-mapa", Seccion::Mapa),
            ("demo-mapa-areas", Seccion::Mapa),
            ("demo-graficos", Seccion::Graficos),
            ("demo-documento", Seccion::Documento),
        ] {
            let mut demo = Demo::at(seccion);
            if seccion == Seccion::Tabla {
                demo.apply(Msg::Tabla(ViewChange::Sort(4, SortDirection::Descending)));
                let second = demo.tabla.rows()[1];
                demo.apply(Msg::SelectRow(second));
            }
            if name == "demo-mapa" {
                demo.apply(Msg::ToggleAreas);
            }
            snapshot(out, name, &demo, wide, 2.0, &Interaction::default(), &none);
        }

        // The view the map reports once it has framed Catillo.
        let mut seleccion = TestApp::builder(Demo::at(Seccion::Mapa))
            .viewport(wide.width, wide.height)
            .build();
        seleccion.click(By::test_id("mapa.sector.AOP26000002"));
        seleccion.advance(Duration::from_secs(1));
        snapshot(
            out,
            "demo-mapa-seleccion",
            seleccion.app(),
            wide,
            2.0,
            &Interaction::default(),
            &none,
        );

        // Catillo being edited, its north east corner pulled out.
        let mut edicion = TestApp::builder(Demo::at(Seccion::Mapa))
            .viewport(wide.width, wide.height)
            .build();
        edicion.click(By::test_id("mapa.sector.AOP26000002"));
        edicion.click(By::test_id("mapa.editar"));
        edicion.advance(Duration::from_secs(1));
        edicion.drag_and_hold(By::test_id("mapa.vista.vertex.2"), 40.0, -30.0);
        snapshot(
            out,
            "demo-mapa-edicion",
            edicion.app(),
            wide,
            2.0,
            &Interaction::default(),
            &none,
        );

        let mut media = Demo::at(Seccion::Multimedia);
        media.apply(Msg::ImagenElegida(
            Campo::Logo,
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
        let banner = find(&media, By::test_id("multimedia.portada.preview"));
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
        let logo = find(&media, By::test_id("multimedia.logo.preview"));
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

        let mut dialog = Demo::at(Seccion::Dialogos);
        dialog.apply(Msg::OpenDialog(Dialogo::Eliminar));
        snapshot(
            out,
            "demo-dialogo",
            &dialog,
            wide,
            2.0,
            &Interaction::default(),
            &none,
        );
        let mut toasts = Demo::at(Seccion::Dialogos);
        for variante in VarianteAviso::ALL {
            toasts.apply(Msg::ShowToast(variante));
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
        let botones = Demo::at(Seccion::Botones);
        let mut tree = build_view(&botones);
        for test_id in ["espera.primary", "espera.outlined", "espera.recargar"] {
            let id = find(&botones, By::test_id(test_id));
            tree.find_mut(id).unwrap().busy = true;
        }
        let image = snapshot::render_with_background(
            &Shot {
                tree: &tree,
                theme: &out.theme,
                logical: wide,
                scale: 2.0,
                interaction: &Interaction::default(),
                scroll: &scrolled_to(&out.theme, &botones, wide, "Acciones con espera"),
            },
            BackgroundEffect::NONE,
            0.3,
        )
        .expect("snapshot render");
        write_shot(out, "demo-botones-cargando", &image);

        let mut rail = Demo::at(Seccion::Tabla);
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
