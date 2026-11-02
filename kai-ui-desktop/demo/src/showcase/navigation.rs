//! Navigation and page: Sidebar, Tabs, Page and the page templates.

use kai::components::{
    ButtonVariant, PageHeader, Sidebar, SidebarGroup, SidebarItem, Tabs, button, caption, card,
    card_content, page_grid, page_grid_item, paragraph,
};
use kai::element::{div, text as label};
use kai::icons::Icon;

use super::code::{call, chain, lit, snippet, when};
use super::{
    Axis, Component, Group, PreviewMsg, REFERENCE, Set, Value, Variant, choice, text, toggle,
};

const SECTIONS: &[&str] = &["Inicio", "Clientes", "Ventas", "Ajustes"];

pub const SIDEBAR: Component = Component {
    id: "sidebar",
    name: "Sidebar",
    group: Group::Navigation,
    summary: "Lista de secciones, con riel plegado.",
    doc: "sidebar.md",
    props: &[
        choice("selected", "Elegida", SECTIONS, 1),
        toggle("collapsed", "Plegada (riel)", false),
        text("badge", "Badge de Clientes", "12"),
    ],
    matrix: &[Axis {
        title: "Ancho",
        variants: &[
            Variant {
                label: "Expandida",
                set: &[Set::Bool("collapsed", false)],
            },
            Variant {
                label: "Riel",
                set: &[Set::Bool("collapsed", true)],
            },
        ],
    }],
    preview: |props, cx| {
        let mut clients = SidebarItem::new("Clientes").icon(Icon::Users);
        if !props.text("badge").is_empty() {
            clients = clients.badge(props.text("badge"));
        }
        let groups = [
            SidebarGroup::untitled([SidebarItem::new("Inicio").icon(Icon::House)]),
            SidebarGroup::titled(
                "Negocio",
                [clients, SidebarItem::new("Ventas").icon(Icon::Wallet)],
            ),
            SidebarGroup::titled("Cuenta", [SidebarItem::new("Ajustes").icon(Icon::Settings)]),
        ];
        div().class("h-64 flex").child(
            Sidebar::new(&groups, props.choice("selected"), |index| {
                PreviewMsg::Set("selected", Value::Choice(index))
            })
            .collapsed(props.on("collapsed"))
            .label("Secciones")
            .test_id(cx.id)
            .build(),
        )
    },
    code: |props| {
        let badge = if props.text("badge").is_empty() {
            String::new()
        } else {
            format!(".badge({})", lit(props.text("badge")))
        };
        let groups = format!(
            "let grupos = [\n    SidebarGroup::untitled([SidebarItem::new(\"Inicio\").icon(Icon::House)]),\n    SidebarGroup::titled(\"Negocio\", [\n        SidebarItem::new(\"Clientes\").icon(Icon::Users){badge},\n        SidebarItem::new(\"Ventas\").icon(Icon::Wallet),\n    ]),\n    SidebarGroup::titled(\"Cuenta\", [SidebarItem::new(\"Ajustes\").icon(Icon::Settings)]),\n];\n"
        );
        snippet(
            &["Sidebar", "SidebarGroup", "SidebarItem"],
            groups
                + &chain(
                    format!(
                        "Sidebar::new(&grupos, self.seccion /* {} */, Msg::Seccion)",
                        props.choice("selected")
                    ),
                    [
                        when(props.on("collapsed"), "collapsed(true)"),
                        call("label(\"Secciones\")"),
                        call("build()"),
                    ],
                ),
        )
    },
    keyboard: &[
        "Una parada de Tab que entra por la fila elegida.",
        "↑ y ↓ cambian la selección (envían on_select como un clic).",
    ],
    accessibility: &[
        "Rol Sidebar nombrado con .label(..); cada fila es un SidebarItem con su texto.",
        "Plegada, cada fila muestra su título en un tooltip; nombres y selección se mantienen.",
    ],
    demo: Some(("Inicio (barra lateral)", "/home")),
    ..REFERENCE
};

pub const TABS: Component = Component {
    id: "tabs",
    name: "Tabs",
    group: Group::Navigation,
    summary: "Pestañas sobre un panel.",
    doc: "tabs.md",
    props: &[
        choice("selected", "Elegida", &["Resumen", "Actividad", "Notas"], 0),
        choice("count", "Pestañas", &["2", "3"], 1),
    ],
    preview: |props, cx| {
        let count = props.choice("count") + 2;
        let mut tabs = Tabs::new(props.choice("selected").min(count - 1), |index| {
            PreviewMsg::Set("selected", Value::Choice(index))
        })
        .tab("Resumen", paragraph("Cliente desde 2021, plan Equipo."))
        .tab("Actividad", paragraph("Última compra: 12 de abril."));
        if count > 2 {
            tabs = tabs.tab("Notas", paragraph("Prefiere que lo llamen en la tarde."));
        }
        div().class("w-96").test_id(cx.id).child(tabs.build())
    },
    code: |props| {
        let count = props.choice("count") + 2;
        snippet(
            &["Tabs", "paragraph"],
            chain(
                format!(
                    "Tabs::new(self.pestana /* {} */, Msg::Pestana)",
                    props.choice("selected").min(count - 1)
                ),
                [
                    call("tab(\"Resumen\", paragraph(\"Cliente desde 2021, plan Equipo.\"))"),
                    call("tab(\"Actividad\", paragraph(\"Última compra: 12 de abril.\"))"),
                    when(
                        count > 2,
                        "tab(\"Notas\", paragraph(\"Prefiere que lo llamen en la tarde.\"))",
                    ),
                    call("build()"),
                ],
            ),
        )
    },
    keyboard: &[
        "La fila es una parada de Tab que entra por la pestaña elegida; ← y → eligen.",
        "Tab sigue al panel.",
    ],
    accessibility: &["AccessKit ve un TabList de Tabs y un TabPanel nombrado como la pestaña."],
    demo: Some(("Contenedores", "/containers")),
    ..REFERENCE
};

const SPANS: &[&str] = &["3", "4", "6", "12"];

pub const PAGE: Component = Component {
    id: "page",
    name: "Page y PageGrid",
    group: Group::Navigation,
    summary: "Contenedor de sección y grilla de 4/8/12 columnas.",
    doc: "page.md",
    props: &[choice("span", "Columnas por tarjeta", SPANS, 1)],
    matrix: &[Axis {
        title: "Columnas por tarjeta",
        variants: &[
            Variant {
                label: "3",
                set: &[Set::Choice("span", 0)],
            },
            Variant {
                label: "6",
                set: &[Set::Choice("span", 2)],
            },
            Variant {
                label: "12",
                set: &[Set::Choice("span", 3)],
            },
        ],
    }],
    preview: |props, cx| {
        let span: u8 = SPANS[props.choice("span")].parse().unwrap_or(4);
        let items = (1..=4).map(|n| {
            page_grid_item(span)
                .child(card().child(card_content().child(label(format!("Tarjeta {n}")))))
        });
        div()
            .class("w-full")
            .test_id(cx.id)
            .child(page_grid().children(items))
    },
    code: |props| {
        let span = SPANS[props.choice("span")];
        snippet(
            &["page", "page_body", "page_grid", "page_grid_item"],
            format!(
                "page().child(page_body().child(page_grid().children([\n    page_grid_item({span}).child(ventas),\n    page_grid_item({span}).child(clientes),\n])))"
            ),
        )
    },
    accessibility: &[
        "La grilla no agrega roles: el orden de lectura es el del código.",
        "La grilla tiene 4, 8 o 12 columnas según el ancho (docs/dev/21-page-layout.md).",
    ],
    demo: Some(("Contenedores", "/containers")),
    ..REFERENCE
};

pub const PAGE_TEMPLATES: Component = Component {
    id: "page-templates",
    name: "Plantillas de página",
    group: Group::Navigation,
    summary: "PageHeader, lista + detalle y formulario.",
    doc: "page-templates.md",
    props: &[
        text("title", "Título", "Clientes"),
        text("subtitle", "Subtítulo", "128 registros"),
        toggle("breadcrumbs", "Migas", true),
        toggle("action", "Acción", true),
    ],
    preview: |props, cx| {
        let mut header = PageHeader::new(props.text("title")).test_id(cx.id);
        if !props.text("subtitle").is_empty() {
            header = header.subtitle(props.text("subtitle"));
        }
        if props.on("breadcrumbs") {
            header = header.breadcrumbs(["Inicio", "Negocio"]);
        }
        if props.on("action") {
            header = header.action(
                button("Nuevo cliente", ButtonVariant::Primary)
                    .on_click(PreviewMsg::Event("Nuevo cliente")),
            );
        }
        div().class("w-full flex flex-col gap-2").children([
            header.build(),
            caption("MasterDetailPage y FormPage llevan este encabezado sobre su contenido."),
        ])
    },
    code: |props| {
        snippet(
            &["PageHeader", "button", "ButtonVariant"],
            chain(
                format!("PageHeader::new({})", lit(props.text("title"))),
                [
                    when(
                        !props.text("subtitle").is_empty(),
                        format!("subtitle({})", lit(props.text("subtitle"))),
                    ),
                    when(
                        props.on("breadcrumbs"),
                        "breadcrumbs([\"Inicio\", \"Negocio\"])",
                    ),
                    when(
                        props.on("action"),
                        "action(button(\"Nuevo cliente\", ButtonVariant::Primary).on_click(Msg::Nuevo))",
                    ),
                    call("build()"),
                ],
            ),
        )
    },
    accessibility: &["El título de PageHeader es el Heading de la sección."],
    demo: Some(("Plantillas: Lista + detalle", "/template-list")),
    ..REFERENCE
};
