//! Actions: Button, IconButton, Toolbar and CommandPalette.

use kai::components::{
    ButtonSize, ButtonVariant, IconButtonSize, IconButtonVariant, button_sized, caption,
    icon_button_sized, search_field, toolbar, toolbar_item, toolbar_search, toolbar_separator,
    toolbar_spacer, toolbar_title,
};
use kai::element::{Element, div};
use kai::icons::Icon;

use super::code::{call, chain, lit, option, snippet, when};
use super::{
    Axis, Component, Cx, Group, PreviewMsg, Props, REFERENCE, Set, Value, Variant, choice, text,
    toggle,
};

const BUTTON_VARIANTS: &[&str] = &["Primary", "Secondary", "Outlined", "Text", "Danger"];
const SIZES: &[&str] = &["Sm", "Md", "Lg"];
const STATES: &[Variant] = &[
    Variant {
        label: "Normal",
        set: &[Set::Bool("disabled", false), Set::Bool("loading", false)],
    },
    Variant {
        label: "Deshabilitado",
        set: &[Set::Bool("disabled", true), Set::Bool("loading", false)],
    },
    Variant {
        label: "Ocupado",
        set: &[Set::Bool("disabled", false), Set::Bool("loading", true)],
    },
];

pub const BUTTON: Component = Component {
    id: "button",
    name: "Button",
    group: Group::Actions,
    summary: "Acción con texto: cinco variantes, tres tamaños y estado ocupado.",
    doc: "button.md",
    props: &[
        text("label", "Texto", "Guardar"),
        choice("variant", "Variante", BUTTON_VARIANTS, 0),
        choice("size", "Tamaño", SIZES, 1),
        toggle("disabled", "Deshabilitado", false),
        toggle("loading", "Ocupado", false),
    ],
    matrix: &[
        Axis {
            title: "Variantes",
            variants: &[
                Variant {
                    label: "Primary",
                    set: &[Set::Choice("variant", 0)],
                },
                Variant {
                    label: "Secondary",
                    set: &[Set::Choice("variant", 1)],
                },
                Variant {
                    label: "Outlined",
                    set: &[Set::Choice("variant", 2)],
                },
                Variant {
                    label: "Text",
                    set: &[Set::Choice("variant", 3)],
                },
                Variant {
                    label: "Danger",
                    set: &[Set::Choice("variant", 4)],
                },
            ],
        },
        Axis {
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
        },
        Axis {
            title: "Estados",
            variants: STATES,
        },
    ],
    preview: button_preview,
    code: button_code,
    keyboard: &[
        "Tab lo enfoca; Espacio o Return lo presionan.",
        "Deshabilitado u ocupado, no recibe foco ni clics.",
    ],
    accessibility: &[
        "Rol Button; su nombre es el texto.",
        "Deshabilitado por on_command, expone el motivo como descripción.",
        "El foco de teclado se dibuja con anillo (keyboard-focus:).",
    ],
    demo: Some(("Botones", "/buttons")),
    ..REFERENCE
};

fn button_variant(index: usize) -> ButtonVariant {
    [
        ButtonVariant::Primary,
        ButtonVariant::Secondary,
        ButtonVariant::Outlined,
        ButtonVariant::Text,
        ButtonVariant::Danger,
    ][index.min(4)]
}

fn button_size(index: usize) -> ButtonSize {
    [ButtonSize::Sm, ButtonSize::Md, ButtonSize::Lg][index.min(2)]
}

fn button_preview(props: &Props, cx: &Cx<'_>) -> Element<PreviewMsg> {
    button_sized(
        props.text("label"),
        button_variant(props.choice("variant")),
        button_size(props.choice("size")),
    )
    .disabled(props.on("disabled"))
    .busy(props.on("loading"))
    .on_click(PreviewMsg::Event("on_click"))
    .test_id(cx.id)
}

fn button_code(props: &Props) -> String {
    snippet(
        &["button_sized", "ButtonSize", "ButtonVariant"],
        chain(
            format!(
                "button_sized({}, ButtonVariant::{}, ButtonSize::{})",
                lit(props.text("label")),
                BUTTON_VARIANTS[props.choice("variant")],
                SIZES[props.choice("size")]
            ),
            [
                when(props.on("disabled"), "disabled(true)"),
                when(props.on("loading"), "busy(true)"),
                call("on_click(Msg::Guardar)"),
            ],
        ),
    )
}

const ICONS: &[&str] = &["Pencil", "Trash2", "Settings", "Copy"];
const ICON_VARIANTS: &[&str] = &[
    "Action",
    "Primary",
    "Secondary",
    "Neutral",
    "Text",
    "Outlined",
];

pub const ICON_BUTTON: Component = Component {
    id: "icon-button",
    name: "IconButton",
    group: Group::Actions,
    summary: "Acción con solo icono; el nombre accesible es obligatorio.",
    doc: "icon-button.md",
    props: &[
        choice("icon", "Icono", ICONS, 0),
        text("label", "Nombre accesible", "Editar"),
        choice("variant", "Variante", ICON_VARIANTS, 0),
        choice("size", "Tamaño", SIZES, 1),
        toggle("disabled", "Deshabilitado", false),
    ],
    matrix: &[
        Axis {
            title: "Variantes",
            variants: &[
                Variant {
                    label: "Action",
                    set: &[Set::Choice("variant", 0)],
                },
                Variant {
                    label: "Primary",
                    set: &[Set::Choice("variant", 1)],
                },
                Variant {
                    label: "Secondary",
                    set: &[Set::Choice("variant", 2)],
                },
                Variant {
                    label: "Neutral",
                    set: &[Set::Choice("variant", 3)],
                },
                Variant {
                    label: "Text",
                    set: &[Set::Choice("variant", 4)],
                },
                Variant {
                    label: "Outlined",
                    set: &[Set::Choice("variant", 5)],
                },
            ],
        },
        Axis {
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
        },
    ],
    preview: icon_button_preview,
    code: icon_button_code,
    keyboard: &["Tab lo enfoca; Espacio o Return lo presionan."],
    accessibility: &[
        "El label es obligatorio: sin texto visible, es lo único que lee VoiceOver.",
        "Rol Button; muestra el label como tooltip.",
    ],
    demo: Some(("Botones", "/buttons")),
    ..REFERENCE
};

fn icon(index: usize) -> Icon {
    [Icon::Pencil, Icon::Trash2, Icon::Settings, Icon::Copy][index.min(3)]
}

fn icon_button_preview(props: &Props, cx: &Cx<'_>) -> Element<PreviewMsg> {
    let variant = [
        IconButtonVariant::Action,
        IconButtonVariant::Primary,
        IconButtonVariant::Secondary,
        IconButtonVariant::Neutral,
        IconButtonVariant::Text,
        IconButtonVariant::Outlined,
    ][props.choice("variant").min(5)];
    let size =
        [IconButtonSize::Sm, IconButtonSize::Md, IconButtonSize::Lg][props.choice("size").min(2)];
    icon_button_sized(
        icon(props.choice("icon")),
        props.text("label"),
        variant,
        size,
    )
    .disabled(props.on("disabled"))
    .on_click(PreviewMsg::Event("on_click"))
    .test_id(cx.id)
}

fn icon_button_code(props: &Props) -> String {
    snippet(
        &["icon_button_sized", "IconButtonSize", "IconButtonVariant"],
        chain(
            format!(
                "icon_button_sized(\n    Icon::{},\n    {},\n    IconButtonVariant::{},\n    IconButtonSize::{},\n)",
                ICONS[props.choice("icon")],
                lit(props.text("label")),
                ICON_VARIANTS[props.choice("variant")],
                SIZES[props.choice("size")]
            ),
            [
                when(props.on("disabled"), "disabled(true)"),
                call("on_click(Msg::Editar)"),
            ],
        ),
    )
}

pub const TOOLBAR: Component = Component {
    id: "toolbar",
    name: "Toolbar",
    group: Group::Actions,
    summary: "Barra superior con botones, título, separadores y búsqueda.",
    doc: "toolbar.md",
    props: &[
        text("title", "Título", "Clientes"),
        text("subtitle", "Subtítulo", "128 registros"),
        toggle("separator", "Separador", true),
        toggle("search", "Búsqueda", true),
    ],
    preview: toolbar_preview,
    code: toolbar_code,
    keyboard: &[
        "Sus botones son una sola parada de Tab; ← y → se mueven entre ellos.",
        "El campo de búsqueda es su propia parada de Tab, porque las flechas editan su texto.",
    ],
    accessibility: &["Cada toolbar_item es un botón con el texto como nombre."],
    demo: Some(("Inicio (barra superior)", "/home")),
    ..REFERENCE
};

fn toolbar_preview(props: &Props, cx: &Cx<'_>) -> Element<PreviewMsg> {
    let subtitle = props.text("subtitle");
    let mut items = vec![
        toolbar_title(
            props.text("title"),
            (!subtitle.is_empty()).then_some(subtitle),
        ),
        toolbar_spacer(),
        toolbar_item(Icon::Plus, "Nuevo").on_click(PreviewMsg::Event("Nuevo")),
        toolbar_item(Icon::Trash2, "Borrar").on_click(PreviewMsg::Event("Borrar")),
    ];
    if props.on("separator") {
        items.push(toolbar_separator());
    }
    items.push(toolbar_item(Icon::Settings, "Ajustes").on_click(PreviewMsg::Event("Ajustes")));
    if props.on("search") {
        items.push(toolbar_search(
            search_field(props.text("query"), |value| {
                PreviewMsg::Set("query", Value::Text(value))
            })
            .test_id(cx.part("search"))
            .build(),
        ));
    }
    div().class("w-full").child(toolbar(items).test_id(cx.id))
}

fn toolbar_code(props: &Props) -> String {
    let mut lines = vec![format!(
        "    toolbar_title({}, {}),",
        lit(props.text("title")),
        option(props.text("subtitle"))
    )];
    lines.push("    toolbar_spacer(),".into());
    lines.push("    toolbar_item(Icon::Plus, \"Nuevo\").on_click(Msg::Nuevo),".into());
    lines.push("    toolbar_item(Icon::Trash2, \"Borrar\").on_click(Msg::Borrar),".into());
    if props.on("separator") {
        lines.push("    toolbar_separator(),".into());
    }
    lines.push("    toolbar_item(Icon::Settings, \"Ajustes\").on_click(Msg::Ajustes),".into());
    if props.on("search") {
        lines.push("    toolbar_search(search_field(&self.busqueda, Msg::Buscar).build()),".into());
    }
    let mut uses = vec!["toolbar", "toolbar_item", "toolbar_spacer", "toolbar_title"];
    if props.on("separator") {
        uses.push("toolbar_separator");
    }
    if props.on("search") {
        uses.extend(["search_field", "toolbar_search"]);
    }
    snippet(&uses, format!("toolbar([\n{}\n])", lines.join("\n")))
}

pub const COMMAND_PALETTE: Component = Component {
    id: "command-palette",
    name: "CommandPalette",
    group: Group::Actions,
    summary: "Paleta de comandos (⌘⇧P) que la sesión llena con App::commands.",
    doc: "command-palette.md",
    preview: |_, cx| {
        div().class("flex flex-col items-start gap-2").children([
            button_sized(
                "Abrir la paleta (⌘⇧P)",
                ButtonVariant::Outlined,
                ButtonSize::Md,
            )
            .run_command(kai::commands::PALETTE)
            .test_id(cx.id),
            caption("La sesión la abre y la llena con los comandos de la demo."),
        ])
    },
    code: |_| {
        "// La sesión la abre con el comando kai.palette (⌘⇧P) y la llena con\n\
         // App::commands; un botón también puede abrirla:\nuse kai::commands::PALETTE;\n\n\
         button(\"Comandos\", ButtonVariant::Outlined).run_command(PALETTE)"
            .to_owned()
    },
    keyboard: &[
        "⌘⇧P la abre; ↑/↓ mueven la selección, Return ejecuta y Escape cierra.",
        "El foco queda en el campo kai.palette.find mientras se elige.",
    ],
    accessibility: &[
        "La fila elegida es el active descendant del campo: VoiceOver la lee sin mover el foco.",
        "Filas kai.palette.<id>; las deshabilitadas muestran su motivo.",
    ],
    ..REFERENCE
};
