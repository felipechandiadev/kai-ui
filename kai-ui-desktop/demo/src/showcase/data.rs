//! Data: Table, VirtualList, Chart, and the reference-only pages for Map and
//! Document (too big for a preview: they are shown in their section).

use std::rc::Rc;

use kai::components::{AlertVariant, ChartKind, Column, Table, alert, chart, virtual_list};
use kai::element::{Element, Role, div, text as label};

use super::code::{call, chain, lit, snippet, when};
use super::{
    Axis, Component, Group, PreviewMsg, REFERENCE, Set, Value, Variant, choice, text, toggle,
};

/// A row of the sample table.
#[derive(Clone, Debug)]
pub struct Commune {
    pub name: &'static str,
    pub region: &'static str,
    pub population: u32,
    pub coastal: bool,
}

const COMMUNES: [Commune; 8] = [
    Commune {
        name: "Antofagasta",
        region: "Antofagasta",
        population: 401_096,
        coastal: true,
    },
    Commune {
        name: "Concepción",
        region: "Biobío",
        population: 223_574,
        coastal: false,
    },
    Commune {
        name: "La Serena",
        region: "Coquimbo",
        population: 249_656,
        coastal: true,
    },
    Commune {
        name: "Puerto Montt",
        region: "Los Lagos",
        population: 245_902,
        coastal: true,
    },
    Commune {
        name: "Puente Alto",
        region: "Metropolitana",
        population: 645_909,
        coastal: false,
    },
    Commune {
        name: "Talca",
        region: "Maule",
        population: 236_724,
        coastal: false,
    },
    Commune {
        name: "Temuco",
        region: "Araucanía",
        population: 302_931,
        coastal: false,
    },
    Commune {
        name: "Valparaíso",
        region: "Valparaíso",
        population: 315_732,
        coastal: true,
    },
];

pub fn communes() -> Rc<[Commune]> {
    COMMUNES.to_vec().into()
}

pub fn columns() -> Vec<Column<Commune, PreviewMsg>> {
    vec![
        Column::text("Comuna", |c: &Commune| c.name.to_owned())
            .sortable()
            .flex(2.0),
        Column::text("Región", |c: &Commune| c.region.to_owned())
            .sortable()
            .filterable()
            .flex(2.0),
        Column::number("Habitantes", |c: &Commune| f64::from(c.population))
            .sortable()
            .flex(1.0),
        Column::boolean("Costera", |c: &Commune| c.coastal).flex(1.0),
    ]
}

pub const TABLE: Component = Component {
    id: "table",
    name: "Table",
    group: Group::Data,
    summary: "Tabla con búsqueda, filtros, orden, columnas fijas y exportación.",
    doc: "table.md",
    props: &[
        toggle("selectable", "Filas seleccionables", true),
        choice("row_height", "Alto de fila", &["28", "32", "40"], 1),
    ],
    preview: |props, cx| {
        let height = [28.0, 32.0, 40.0][props.choice("row_height").min(2)];
        let mut table = Table::new(communes(), columns(), &cx.live.table)
            .on_view(PreviewMsg::View)
            .row_height(height)
            .label("Comunas")
            .test_id(cx.id);
        if props.on("selectable") {
            table = table.selected(cx.live.row).on_select(PreviewMsg::Row);
        }
        div()
            .class("w-full h-80 flex flex-col")
            .child(table.build())
    },
    code: |props| {
        let height = ["28.0", "32.0", "40.0"][props.choice("row_height")];
        let columns = "let columnas = vec![\n    Column::text(\"Comuna\", |c: &Comuna| c.nombre.clone()).sortable(),\n    Column::text(\"Región\", |c: &Comuna| c.region.clone()).sortable().filterable(),\n    Column::number(\"Habitantes\", |c: &Comuna| c.habitantes as f64).sortable(),\n    Column::boolean(\"Costera\", |c: &Comuna| c.costera),\n];\n";
        snippet(
            &["Column", "Table"],
            columns.to_owned()
                + &chain(
                    "Table::new(self.comunas.clone(), columnas, &self.vista)",
                    [
                        call("on_view(Msg::Vista)"),
                        when(props.on("selectable"), "selected(self.fila)"),
                        when(props.on("selectable"), "on_select(Msg::Fila)"),
                        when(
                            props.choice("row_height") != 1,
                            format!("row_height({height})"),
                        ),
                        call("label(\"Comunas\")"),
                        call("build()"),
                    ],
                ),
        )
    },
    keyboard: &[
        "Con on_select, las filas son una parada de Tab: clic, ↑/↓, Inicio/Fin y RePág/AvPág eligen.",
        "→ y ← abren y cierran una fila expandible; en los filtros, Escape limpia.",
    ],
    accessibility: &[
        "Encabezados ColumnHeader, filas Row, celdas Cell.",
        "El menú de columnas responde a «mostrar menú» de VoiceOver.",
    ],
    demo: Some(("Tabla", "/table")),
    ..REFERENCE
};

pub const VIRTUAL_LIST: Component = Component {
    id: "virtual-list",
    name: "VirtualList",
    group: Group::Data,
    summary: "Lista que construye solo las filas visibles.",
    doc: "virtual-list.md",
    props: &[
        choice("count", "Filas", &["100", "10.000", "1.000.000"], 1),
        toggle("selecting", "Elegir con flechas", true),
    ],
    preview: |props, cx| {
        let count = [100, 10_000, 1_000_000][props.choice("count").min(2)];
        let selected = props.choice("row");
        let selecting = props.on("selecting");
        let render: Rc<dyn Fn(usize) -> Element<PreviewMsg>> = Rc::new(move |index| {
            let row = label(format!("Fila {}", index + 1))
                .class("h-8 px-3 flex items-center text-sm text-foreground selected:bg-choice/20");
            if selecting {
                row.role(Role::ListItem)
                    .pressed(index == selected)
                    .on_click(PreviewMsg::Set("row", Value::Choice(index)))
            } else {
                row
            }
        });
        let mut list = virtual_list(count, 32.0, render)
            .class("h-64 w-72 border border-border rounded-md")
            .label("Filas")
            .test_id(cx.id);
        if selecting {
            list = list.selecting_focus_group();
        }
        list
    },
    code: |props| {
        let count = ["100", "10_000", "1_000_000"][props.choice("count")];
        snippet(
            &["virtual_list"],
            chain(
                format!(
                    "virtual_list({count}, 32.0, Rc::new(move |i| {{\n    text(format!(\"Fila {{}}\", i + 1)).on_click(Msg::Fila(i))\n}}))"
                ),
                [
                    call("class(\"h-64\")"),
                    when(props.on("selecting"), "selecting_focus_group()"),
                ],
            ),
        )
    },
    keyboard: &["Con .selecting_focus_group() es una parada de Tab donde las flechas eligen."],
    accessibility: &["Filas sin rol se exponen como ListItem."],
    demo: Some(("Tabla", "/table")),
    ..REFERENCE
};

const MONTHS: [&str; 6] = ["Ene", "Feb", "Mar", "Abr", "May", "Jun"];
const SALES: [f32; 6] = [12.0, 18.0, 9.0, 22.0, 17.0, 25.0];
const KINDS: &[&str] = &["Bar", "Sparkline"];

pub const CHART: Component = Component {
    id: "chart",
    name: "Chart",
    group: Group::Data,
    summary: "Barras y sparkline.",
    doc: "chart.md",
    props: &[
        choice("kind", "Tipo", KINDS, 0),
        text("label", "Nombre accesible", "Ventas por mes"),
        choice(
            "selected",
            "Barra elegida",
            &["Ninguna", "Ene", "Feb", "Mar", "Abr", "May", "Jun"],
            0,
        ),
    ],
    matrix: &[Axis {
        title: "Tipos",
        variants: &[
            Variant {
                label: "Bar",
                set: &[Set::Choice("kind", 0)],
            },
            Variant {
                label: "Sparkline",
                set: &[Set::Choice("kind", 1)],
            },
        ],
    }],
    preview: |props, cx| {
        let bar = props.choice("kind") == 0;
        let plot = if bar {
            chart(ChartKind::Bar)
                .categories(MONTHS)
                .series("Ventas", &SALES)
                .selected(props.choice("selected").checked_sub(1))
                .on_select(|index| PreviewMsg::Set("selected", Value::Choice(index + 1)))
                .class("h-40 w-80")
        } else {
            chart(ChartKind::Sparkline)
                .series("Ventas", &SALES)
                .class("h-8 w-48")
        };
        plot.label(props.text("label")).test_id(cx.id)
    },
    code: |props| {
        let bar = props.choice("kind") == 0;
        let selected = match props.choice("selected").checked_sub(1) {
            Some(index) => format!("Some({index})"),
            None => "None".to_owned(),
        };
        snippet(
            &["chart", "ChartKind"],
            chain(
                format!("chart(ChartKind::{})", KINDS[props.choice("kind")]),
                [
                    when(
                        bar,
                        "categories([\"Ene\", \"Feb\", \"Mar\", \"Abr\", \"May\", \"Jun\"])",
                    ),
                    call("series(\"Ventas\", &[12.0, 18.0, 9.0, 22.0, 17.0, 25.0])"),
                    when(bar, format!("selected({selected}) // self.mes")),
                    when(bar, "on_select(Msg::Mes)"),
                    call(format!("label({})", lit(props.text("label")))),
                ],
            ),
        )
    },
    accessibility: &[
        "Rol Image nombrado con .label(..).",
        "La descripción accesible lista la primera serie («Ene 12, Feb 18…»).",
    ],
    demo: Some(("Gráficos", "/charts")),
    ..REFERENCE
};

fn reference(section: &str) -> Element<PreviewMsg> {
    alert(
        AlertVariant::Info,
        format!("Es muy grande para una vista previa: se usa de verdad en la sección «{section}»."),
    )
    .class("w-96")
}

pub const MAP: Component = Component {
    id: "map",
    name: "Map",
    group: Group::Data,
    summary: "Mapa vectorial con capas y marcadores.",
    doc: "map.md",
    preview: |_, cx| reference("Mapa").test_id(cx.id),
    code: |_| {
        "use kai::maps::map;\n\nmap(&self.vista)\n    .basemap(TileSource::pmtiles(\"assets/maps/parral.pmtiles\", MapStyle::protomaps()))\n    .on_view_change(Msg::Vista)\n    .label(\"Mapa de Parral\")\n    .class(\"flex-1 min-h-0\")".to_owned()
    },
    keyboard: &["Las flechas desplazan y + / - hacen zoom; tiene botones de zoom propios."],
    accessibility: &["Dale nombre accesible con .label(..)."],
    demo: Some(("Mapa", "/map")),
    ..REFERENCE
};

pub const DOCUMENT: Component = Component {
    id: "document",
    name: "Document",
    group: Group::Data,
    summary: "Editor de documentos tipo Word (feature document).",
    doc: "document.md",
    preview: |_, cx| reference("Documento").test_id(cx.id),
    code: |_| {
        "// Requiere la feature `document` (docs/componentes/document.md).\nuse kai::document::document;\n\ndocument()\n    .label(\"Informe\")\n    .markdown(&self.texto)\n    .on_change(Msg::Cambio)\n    .class(\"flex-1 min-h-0\")".to_owned()
    },
    keyboard: &[
        "Edición de texto completa: IME con preedit visible y portapapeles con formato.",
        "Cinta: pestañas document.ribbon.tab.<id> y comandos document.ribbon.cmd.<id>.",
    ],
    accessibility: &["Cursor de texto sobre el área del documento; la hoja se recorre con scroll."],
    demo: Some(("Documento", "/document")),
    ..REFERENCE
};
