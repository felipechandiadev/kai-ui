//! Content and status: cards, figures, text, badges, alerts, avatar,
//! empty state and progress.

use kai::components::{
    AlertVariant, Avatar, AvatarSize, BadgeVariant, ButtonVariant, DotProgress, DotSize,
    EmptyState, alert, badge, button, caption, card, card_actions, card_content, card_header,
    group_box, heading, paragraph, stat,
};
use kai::element::{Element, div};
use kai::icons::Icon;

use super::code::{call, chain, lit, option, snippet, when};
use super::{
    Axis, Component, Cx, Group, PreviewMsg, Props, REFERENCE, Set, Variant, choice, text, toggle,
};

pub const CARD: Component = Component {
    id: "card",
    name: "Card",
    group: Group::Content,
    summary: "Tarjeta con encabezado, contenido y acciones.",
    doc: "card.md",
    props: &[
        text("title", "Título", "Ventas de abril"),
        text("subtitle", "Subtítulo", "Comparado con marzo"),
        text(
            "body",
            "Contenido",
            "Subieron un 12 % gracias a la campaña de otoño.",
        ),
        toggle("actions", "Acciones", true),
    ],
    preview: card_preview,
    code: card_code,
    accessibility: &[
        "El título del encabezado tiene rol Heading.",
        "La tarjeta no agrega paradas de Tab: solo las tienen sus controles.",
    ],
    demo: Some(("Contenedores", "/containers")),
    ..REFERENCE
};

fn card_preview(props: &Props, cx: &Cx<'_>) -> Element<PreviewMsg> {
    let subtitle = props.text("subtitle");
    let mut parts = vec![
        card_header(
            props.text("title"),
            (!subtitle.is_empty()).then_some(subtitle),
        ),
        card_content().child(paragraph(props.text("body"))),
    ];
    if props.on("actions") {
        parts.push(card_actions().child(
            button("Ver detalle", ButtonVariant::Text).on_click(PreviewMsg::Event("Ver detalle")),
        ));
    }
    card().class("w-80").test_id(cx.id).children(parts)
}

fn card_code(props: &Props) -> String {
    let mut lines = vec![
        format!(
            "    card_header({}, {}),",
            lit(props.text("title")),
            option(props.text("subtitle"))
        ),
        format!(
            "    card_content().child(paragraph({})),",
            lit(props.text("body"))
        ),
    ];
    let mut uses = vec!["card", "card_content", "card_header", "paragraph"];
    if props.on("actions") {
        lines.push(
            "    card_actions().child(button(\"Ver detalle\", ButtonVariant::Text).on_click(Msg::Detalle)),"
                .into(),
        );
        uses.extend(["card_actions", "button", "ButtonVariant"]);
    }
    snippet(
        &uses,
        format!("card().children([\n{}\n])", lines.join("\n")),
    )
}

pub const GROUP_BOX: Component = Component {
    id: "group-box",
    name: "GroupBox",
    group: Group::Content,
    summary: "Caja con título (NSBox) para agrupar controles.",
    doc: "group-box.md",
    props: &[
        text("title", "Título", "Privacidad"),
        toggle("titled", "Con título", true),
    ],
    preview: |props, cx| {
        let title = props.on("titled").then(|| props.text("title"));
        div().class("w-80").test_id(cx.id).child(group_box(
            title,
            [
                paragraph("Tus datos se guardan en este Mac."),
                caption("Nunca se envían a terceros."),
            ],
        ))
    },
    code: |props| {
        let title = if props.on("titled") {
            option(props.text("title"))
        } else {
            "None".to_owned()
        };
        snippet(
            &["caption", "group_box", "paragraph"],
            format!(
                "group_box(\n    {title},\n    [\n        paragraph(\"Tus datos se guardan en este Mac.\"),\n        caption(\"Nunca se envían a terceros.\"),\n    ],\n)"
            ),
        )
    },
    accessibility: &["El título es un Heading que nombra el grupo."],
    demo: Some(("Contenedores", "/containers")),
    ..REFERENCE
};

pub const STAT: Component = Component {
    id: "stat",
    name: "Stat",
    group: Group::Content,
    summary: "Cifra con etiqueta y nota.",
    doc: "stat.md",
    props: &[
        text("label", "Etiqueta", "Clientes activos"),
        text("value", "Cifra", "1.284"),
        text("hint", "Nota", "+38 esta semana"),
    ],
    preview: |props, cx| {
        let hint = props.text("hint");
        div().test_id(cx.id).child(stat(
            props.text("label"),
            props.text("value"),
            (!hint.is_empty()).then_some(hint),
        ))
    },
    code: |props| {
        snippet(
            &["stat"],
            format!(
                "stat({}, {}, {})",
                lit(props.text("label")),
                lit(props.text("value")),
                option(props.text("hint"))
            ),
        )
    },
    accessibility: &["Se lee en orden: etiqueta, cifra y nota."],
    demo: Some(("Gráficos", "/charts")),
    ..REFERENCE
};

pub const TYPOGRAPHY: Component = Component {
    id: "tipografia",
    name: "Tipografía",
    group: Group::Content,
    summary: "heading, paragraph y caption con la escala del tema.",
    doc: "tipografia.md",
    props: &[
        choice("kind", "Elemento", &["heading", "paragraph", "caption"], 0),
        text("content", "Texto", "Resumen del trimestre"),
    ],
    matrix: &[Axis {
        title: "Elementos",
        variants: &[
            Variant {
                label: "heading",
                set: &[Set::Choice("kind", 0)],
            },
            Variant {
                label: "paragraph",
                set: &[Set::Choice("kind", 1)],
            },
            Variant {
                label: "caption",
                set: &[Set::Choice("kind", 2)],
            },
        ],
    }],
    preview: |props, cx| {
        let content = props.text("content");
        match props.choice("kind") {
            0 => heading(content),
            1 => paragraph(content),
            _ => caption(content),
        }
        .test_id(cx.id)
    },
    code: |props| {
        let kind = ["heading", "paragraph", "caption"][props.choice("kind")];
        snippet(&[kind], format!("{kind}({})", lit(props.text("content"))))
    },
    accessibility: &[
        "heading tiene rol Heading: VoiceOver lo usa para saltar entre secciones.",
        "paragraph tiene rol Paragraph; caption es texto secundario.",
    ],
    demo: Some(("Contenedores", "/containers")),
    ..REFERENCE
};

const BADGES: &[&str] = &[
    "Primary",
    "Secondary",
    "Success",
    "Error",
    "Warning",
    "Info",
];
const ALL_BADGES: [BadgeVariant; 12] = [
    BadgeVariant::Primary,
    BadgeVariant::Secondary,
    BadgeVariant::Success,
    BadgeVariant::Error,
    BadgeVariant::Warning,
    BadgeVariant::Info,
    BadgeVariant::PrimaryOutlined,
    BadgeVariant::SecondaryOutlined,
    BadgeVariant::SuccessOutlined,
    BadgeVariant::ErrorOutlined,
    BadgeVariant::WarningOutlined,
    BadgeVariant::InfoOutlined,
];

const fn badge_cell(label: &'static str, color: usize) -> Variant {
    Variant {
        label,
        set: match color {
            0 => &[Set::Choice("color", 0)],
            1 => &[Set::Choice("color", 1)],
            2 => &[Set::Choice("color", 2)],
            3 => &[Set::Choice("color", 3)],
            4 => &[Set::Choice("color", 4)],
            _ => &[Set::Choice("color", 5)],
        },
    }
}

pub const BADGE: Component = Component {
    id: "badge",
    name: "Badge",
    group: Group::Content,
    summary: "Etiqueta de estado en píldora: seis colores, relleno o contorno.",
    doc: "badge.md",
    props: &[
        text("label", "Texto", "Activo"),
        choice("color", "Color", BADGES, 2),
        toggle("outlined", "Contorno", false),
    ],
    matrix: &[
        Axis {
            title: "Colores",
            variants: &[
                badge_cell("Primary", 0),
                badge_cell("Secondary", 1),
                badge_cell("Success", 2),
                badge_cell("Error", 3),
                badge_cell("Warning", 4),
                badge_cell("Info", 5),
            ],
        },
        Axis {
            title: "Relleno",
            variants: &[
                Variant {
                    label: "Relleno",
                    set: &[Set::Bool("outlined", false)],
                },
                Variant {
                    label: "Contorno",
                    set: &[Set::Bool("outlined", true)],
                },
            ],
        },
    ],
    preview: |props, cx| {
        let index = props.choice("color").min(5) + if props.on("outlined") { 6 } else { 0 };
        badge(props.text("label"), ALL_BADGES[index]).test_id(cx.id)
    },
    code: |props| {
        snippet(
            &["badge", "BadgeVariant"],
            format!(
                "badge({}, BadgeVariant::{}{})",
                lit(props.text("label")),
                BADGES[props.choice("color")],
                if props.on("outlined") { "Outlined" } else { "" }
            ),
        )
    },
    accessibility: &[
        "El color no es la única señal: el texto del badge dice el estado.",
        "El contraste de cada variante está medido en docs/dev/20-contraste-wcag.md.",
    ],
    demo: Some(("Badges y alertas", "/badges")),
    ..REFERENCE
};

const ALERTS: &[&str] = &["Success", "Info", "Warning", "Error"];

pub const ALERT: Component = Component {
    id: "alert",
    name: "Alert",
    group: Group::Content,
    summary: "Mensaje con icono y color semántico.",
    doc: "alert.md",
    props: &[
        choice("variant", "Variante", ALERTS, 1),
        text("message", "Mensaje", "Los cambios se guardan solos."),
    ],
    matrix: &[Axis {
        title: "Variantes",
        variants: &[
            Variant {
                label: "Success",
                set: &[Set::Choice("variant", 0)],
            },
            Variant {
                label: "Info",
                set: &[Set::Choice("variant", 1)],
            },
            Variant {
                label: "Warning",
                set: &[Set::Choice("variant", 2)],
            },
            Variant {
                label: "Error",
                set: &[Set::Choice("variant", 3)],
            },
        ],
    }],
    preview: |props, cx| {
        alert(
            alert_variant(props.choice("variant")),
            props.text("message"),
        )
        .class("w-80")
        .test_id(cx.id)
    },
    code: |props| {
        snippet(
            &["alert", "AlertVariant"],
            format!(
                "alert(AlertVariant::{}, {})",
                ALERTS[props.choice("variant")],
                lit(props.text("message"))
            ),
        )
    },
    accessibility: &[
        "Rol Status: el lector lo anuncia sin mover el foco.",
        "El icono acompaña al color; el mensaje dice lo que pasa.",
    ],
    demo: Some(("Badges y alertas", "/badges")),
    ..REFERENCE
};

pub fn alert_variant(index: usize) -> AlertVariant {
    [
        AlertVariant::Success,
        AlertVariant::Info,
        AlertVariant::Warning,
        AlertVariant::Error,
    ][index.min(3)]
}

pub const AVATAR_SIZES: &[&str] = &["Xs", "Sm", "Md", "Lg"];

pub const AVATAR: Component = Component {
    id: "avatar",
    name: "Avatar",
    group: Group::Content,
    summary: "Foto o inicial de una persona.",
    doc: "avatar.md",
    props: &[
        text("name", "Nombre", "Ana Rojas"),
        choice("size", "Tamaño", AVATAR_SIZES, 2),
    ],
    matrix: &[Axis {
        title: "Tamaños",
        variants: &[
            Variant {
                label: "Xs",
                set: &[Set::Choice("size", 0)],
            },
            Variant {
                label: "Sm",
                set: &[Set::Choice("size", 1)],
            },
            Variant {
                label: "Md",
                set: &[Set::Choice("size", 2)],
            },
            Variant {
                label: "Lg",
                set: &[Set::Choice("size", 3)],
            },
        ],
    }],
    preview: |props, cx| {
        let size = [
            AvatarSize::Xs,
            AvatarSize::Sm,
            AvatarSize::Md,
            AvatarSize::Lg,
        ][props.choice("size").min(3)];
        Avatar::new(props.text("name"))
            .size(size)
            .test_id(cx.id)
            .build()
    },
    code: |props| {
        snippet(
            &["Avatar", "AvatarSize"],
            chain(
                format!("Avatar::new({})", lit(props.text("name"))),
                [
                    call("image(self.foto.clone())"),
                    when(
                        props.choice("size") != 2,
                        format!("size(AvatarSize::{})", AVATAR_SIZES[props.choice("size")]),
                    ),
                    call("build()"),
                ],
            ),
        )
    },
    accessibility: &[
        "Rol Image con el nombre de la persona.",
        "Sin foto muestra la inicial; sin nombre se lee «Usuario».",
    ],
    demo: Some(("Multimedia", "/media")),
    ..REFERENCE
};

const EMPTY_KINDS: &[&str] = &["Vacío", "Sin resultados", "Recurso"];

pub const EMPTY_STATE: Component = Component {
    id: "empty-state",
    name: "EmptyState",
    group: Group::Content,
    summary: "Superficie vacía con icono, texto y acción.",
    doc: "empty-state.md",
    props: &[
        choice("kind", "Tipo", EMPTY_KINDS, 0),
        text("title", "Título", "Aún no hay clientes"),
        text(
            "description",
            "Descripción",
            "Agrega el primero para empezar.",
        ),
        text("action", "Acción", "Agregar cliente"),
    ],
    preview: |props, cx| {
        let title = props.text("title");
        let mut state = match props.choice("kind") {
            0 => EmptyState::new(Icon::Users, title),
            1 => EmptyState::filter(title),
            _ => EmptyState::resource(title),
        }
        .description(props.text("description"))
        .test_id(cx.id);
        if !props.text("action").is_empty() {
            state = state.action(props.text("action"), PreviewMsg::Event("action"));
        }
        div().class("w-96").child(state.build())
    },
    code: |props| {
        let head = match props.choice("kind") {
            0 => format!("EmptyState::new(Icon::Users, {})", lit(props.text("title"))),
            1 => format!("EmptyState::filter({})", lit(props.text("title"))),
            _ => format!("EmptyState::resource({})", lit(props.text("title"))),
        };
        snippet(
            &["EmptyState"],
            chain(
                head,
                [
                    call(format!("description({})", lit(props.text("description")))),
                    when(
                        !props.text("action").is_empty(),
                        format!("action({}, Msg::Nuevo)", lit(props.text("action"))),
                    ),
                    call("build()"),
                ],
            ),
        )
    },
    accessibility: &[
        "Rol Status nombrado por el título: se anuncia cuando aparece.",
        "La acción es un botón normal, la siguiente parada de Tab.",
    ],
    demo: Some(("Tabla", "/table")),
    ..REFERENCE
};

const DOT_SIZES: &[&str] = &["Sm", "Md", "Lg"];

pub const DOT_PROGRESS: Component = Component {
    id: "dot-progress",
    name: "DotProgress",
    group: Group::Content,
    summary: "Progreso indeterminado en puntos.",
    doc: "dot-progress.md",
    props: &[
        choice("size", "Tamaño", DOT_SIZES, 1),
        choice("count", "Puntos", &["3", "4", "5"], 0),
        text("label", "Nombre accesible", "Cargando clientes"),
    ],
    matrix: &[Axis {
        title: "Tamaños",
        variants: &[
            Variant {
                label: "Sm",
                set: &[Set::Choice("size", 0)],
            },
            Variant {
                label: "Md",
                set: &[Set::Choice("size", 1)],
            },
            Variant {
                label: "Lg",
                set: &[Set::Choice("size", 2)],
            },
        ],
    }],
    preview: |props, cx| {
        let size = [DotSize::Sm, DotSize::Md, DotSize::Lg][props.choice("size").min(2)];
        DotProgress::new()
            .size(size)
            .count(props.choice("count") + 3)
            .label(props.text("label"))
            .test_id(cx.id)
            .build()
    },
    code: |props| {
        snippet(
            &["DotProgress", "DotSize"],
            chain(
                "DotProgress::new()",
                [
                    when(
                        props.choice("size") != 1,
                        format!("size(DotSize::{})", DOT_SIZES[props.choice("size")]),
                    ),
                    when(
                        props.choice("count") != 0,
                        format!("count({})", props.choice("count") + 3),
                    ),
                    call(format!("label({})", lit(props.text("label")))),
                    call("build()"),
                ],
            ),
        )
    },
    accessibility: &[
        "Rol ProgressIndicator con su label (por defecto «Cargando»).",
        "decorative() lo deja sin rol cuando el estado ocupado es del elemento de alrededor.",
    ],
    demo: Some(("Botones (estado ocupado)", "/buttons")),
    ..REFERENCE
};
