//! Data entry: fields, pickers, switches and image.

use kai::color::Color;
use kai::components::{
    AvatarSize, FieldDensity, MediaSlot, MediaVariant, SearchVariant, SwitchDensity, TextField,
    autocomplete, checkbox_with_options, color_field, date_field, search_field,
    segmented_with_options, select_with_options, switch_labeled, text_area,
};
use kai::datetime::Date;
use kai::element::{ControlState, Element};

use super::code::{call, chain, lit, snippet, when};
use super::{
    Axis, Component, Cx, Group, PreviewMsg, Props, REFERENCE, Set, Value, Variant, choice, text,
    toggle,
};

/// Autocomplete options.
pub const COMMUNES: [&str; 12] = [
    "Antofagasta",
    "Arica",
    "Castro",
    "Concepción",
    "Coyhaique",
    "La Serena",
    "Puerto Montt",
    "Punta Arenas",
    "Santiago",
    "Talca",
    "Temuco",
    "Valparaíso",
];

/// "Today" for the pages: fixed, so the calendar and the tests do not
/// depend on the clock.
pub fn today() -> Date {
    Date::new(2026, 4, 18).expect("fecha válida")
}

fn set_text(key: &'static str) -> impl Fn(String) -> PreviewMsg {
    move |value| PreviewMsg::Set(key, Value::Text(value))
}

const FIELD_STATES: &[Variant] = &[
    Variant {
        label: "Normal",
        set: &[Set::Bool("disabled", false), Set::Bool("error", false)],
    },
    Variant {
        label: "Con error",
        set: &[Set::Bool("disabled", false), Set::Bool("error", true)],
    },
    Variant {
        label: "Deshabilitado",
        set: &[Set::Bool("disabled", true), Set::Bool("error", false)],
    },
];

pub const TEXT_FIELD: Component = Component {
    id: "text-field",
    name: "TextField",
    group: Group::Input,
    summary: "Campo de una línea, con adornos, contraseña y errores.",
    doc: "text-field.md",
    props: &[
        text("label", "Etiqueta", "Nombre"),
        text("value", "Valor", "Ana Rojas"),
        text("placeholder", "Placeholder", "Escribe tu nombre"),
        text("helper", "Ayuda", "Como aparece en tu carnet"),
        choice("density", "Densidad", &["Default", "Compact"], 0),
        toggle("required", "Obligatorio", false),
        toggle("error", "Con error", false),
        toggle("read_only", "Solo lectura", false),
        toggle("disabled", "Deshabilitado", false),
    ],
    matrix: &[
        Axis {
            title: "Estados",
            variants: FIELD_STATES,
        },
        Axis {
            title: "Densidad",
            variants: &[
                Variant {
                    label: "Default",
                    set: &[Set::Choice("density", 0)],
                },
                Variant {
                    label: "Compact",
                    set: &[Set::Choice("density", 1)],
                },
            ],
        },
    ],
    preview: text_field_preview,
    code: text_field_code,
    keyboard: &[
        "Edición de texto del sistema: ⌘A, ⌘C/⌘V/⌘X, ⌥← y ⌥→ por palabra.",
        "Return envía on_submit; Escape sigue la cadena de responders.",
    ],
    accessibility: &[
        "Rol TextInput; nombre = etiqueta; descripción = ayuda o error.",
        "required se expone; el ojo de contraseña se llama «Mostrar/Ocultar contraseña».",
    ],
    demo: Some(("Campos", "/fields")),
    ..REFERENCE
};

fn text_field_preview(props: &Props, cx: &Cx<'_>) -> Element<PreviewMsg> {
    let density = [FieldDensity::Default, FieldDensity::Compact][props.choice("density").min(1)];
    TextField::new(props.text("label"), props.text("value"))
        .placeholder(props.text("placeholder"))
        .helper_text(props.text("helper"))
        .density(density)
        .required(props.on("required"))
        .error(props.on("error"))
        .read_only(props.on("read_only"))
        .disabled(props.on("disabled"))
        .on_input(set_text("value"))
        .test_id(cx.id)
        .build()
}

fn text_field_code(props: &Props) -> String {
    snippet(
        &["TextField"],
        chain(
            format!("TextField::new({}, &self.nombre)", lit(props.text("label"))),
            [
                when(
                    !props.text("placeholder").is_empty(),
                    format!("placeholder({})", lit(props.text("placeholder"))),
                ),
                when(
                    !props.text("helper").is_empty(),
                    format!("helper_text({})", lit(props.text("helper"))),
                ),
                when(
                    props.choice("density") == 1,
                    "density(FieldDensity::Compact)",
                ),
                when(props.on("required"), "required(true)"),
                when(props.on("error"), "error(true)"),
                when(props.on("read_only"), "read_only(true)"),
                when(props.on("disabled"), "disabled(true)"),
                call("on_input(Msg::Nombre)"),
                call("build()"),
            ],
        ),
    )
}

pub const TEXT_AREA: Component = Component {
    id: "text-area",
    name: "TextArea",
    group: Group::Input,
    summary: "Campo de varias líneas con contador.",
    doc: "text-area.md",
    props: &[
        text("label", "Etiqueta", "Notas"),
        text("value", "Valor", "Llamar el lunes."),
        text("placeholder", "Placeholder", "Escribe una nota"),
        choice("rows", "Filas", &["2", "3", "4", "6"], 1),
        toggle("counter", "Contador (máx. 140)", true),
        toggle("required", "Obligatorio", false),
        toggle("disabled", "Deshabilitado", false),
    ],
    matrix: &[Axis {
        title: "Estados",
        variants: &[
            Variant {
                label: "Normal",
                set: &[Set::Bool("disabled", false)],
            },
            Variant {
                label: "Deshabilitado",
                set: &[Set::Bool("disabled", true)],
            },
        ],
    }],
    preview: text_area_preview,
    code: text_area_code,
    keyboard: &["Edición de texto del sistema, en varias líneas; Tab sale del campo."],
    accessibility: &[
        "Rol TextInput; nombre = etiqueta, como TextField.",
        "Con max_length, el contador muestra n/máx bajo el campo (parte <id>.counter).",
    ],
    demo: Some(("Campos", "/fields")),
    ..REFERENCE
};

const ROWS: [u16; 4] = [2, 3, 4, 6];

fn text_area_preview(props: &Props, cx: &Cx<'_>) -> Element<PreviewMsg> {
    let mut area = text_area(props.text("label"), props.text("value"))
        .placeholder(props.text("placeholder"))
        .rows(ROWS[props.choice("rows").min(3)])
        .required(props.on("required"))
        .disabled(props.on("disabled"))
        .on_input(set_text("value"))
        .test_id(cx.id);
    if props.on("counter") {
        area = area.max_length(140).counter(true);
    }
    area.build()
}

fn text_area_code(props: &Props) -> String {
    snippet(
        &["text_area"],
        chain(
            format!("text_area({}, &self.notas)", lit(props.text("label"))),
            [
                when(
                    !props.text("placeholder").is_empty(),
                    format!("placeholder({})", lit(props.text("placeholder"))),
                ),
                call(format!("rows({})", ROWS[props.choice("rows")])),
                when(props.on("counter"), "max_length(140)"),
                when(props.on("counter"), "counter(true)"),
                when(props.on("required"), "required(true)"),
                when(props.on("disabled"), "disabled(true)"),
                call("on_input(Msg::Notas)"),
                call("build()"),
            ],
        ),
    )
}

const SEARCH_VARIANTS: &[&str] = &["Toolbar", "Field", "Inline"];

pub const SEARCH_FIELD: Component = Component {
    id: "search-field",
    name: "SearchField",
    group: Group::Input,
    summary: "Búsqueda con lupa y botón de borrar.",
    doc: "search-field.md",
    props: &[
        text("value", "Valor", ""),
        text("placeholder", "Placeholder", "Buscar clientes"),
        choice("variant", "Variante", SEARCH_VARIANTS, 1),
        toggle("disabled", "Deshabilitado", false),
    ],
    matrix: &[Axis {
        title: "Variantes",
        variants: &[
            Variant {
                label: "Toolbar",
                set: &[Set::Choice("variant", 0)],
            },
            Variant {
                label: "Field",
                set: &[Set::Choice("variant", 1)],
            },
            Variant {
                label: "Inline",
                set: &[Set::Choice("variant", 2)],
            },
        ],
    }],
    preview: search_preview,
    code: search_code,
    keyboard: &[
        "Escape lo vacía; con el campo ya vacío, Escape sigue por la cadena de responders.",
        "Borrar también pasa por on_input, así la app conserva el valor.",
    ],
    accessibility: &[
        "Rol SearchInput; el botón de borrar se llama «Borrar búsqueda».",
        "Nómbralo con .label(..) cuando el placeholder no basta.",
    ],
    demo: Some(("Tabla", "/table")),
    ..REFERENCE
};

fn search_preview(props: &Props, cx: &Cx<'_>) -> Element<PreviewMsg> {
    let variant = [
        SearchVariant::Toolbar,
        SearchVariant::Field,
        SearchVariant::Inline,
    ][props.choice("variant").min(2)];
    search_field(props.text("value"), set_text("value"))
        .placeholder(props.text("placeholder"))
        .variant(variant)
        .disabled(props.on("disabled"))
        .test_id(cx.id)
        .build()
}

fn search_code(props: &Props) -> String {
    snippet(
        &["search_field", "SearchVariant"],
        chain(
            "search_field(&self.busqueda, Msg::Buscar)",
            [
                call(format!("placeholder({})", lit(props.text("placeholder")))),
                when(
                    props.choice("variant") != 1,
                    format!(
                        "variant(SearchVariant::{})",
                        SEARCH_VARIANTS[props.choice("variant")]
                    ),
                ),
                when(props.on("disabled"), "disabled(true)"),
                call("build()"),
            ],
        ),
    )
}

pub const AUTOCOMPLETE: Component = Component {
    id: "autocomplete",
    name: "Autocomplete",
    group: Group::Input,
    summary: "Combobox: opciones al escribir, de una lista o una búsqueda.",
    doc: "autocomplete.md",
    props: &[
        text("label", "Etiqueta", "Comuna"),
        text("placeholder", "Placeholder", "Escribe para buscar"),
        text("helper", "Ayuda", ""),
        toggle("required", "Obligatorio", false),
        toggle("disabled", "Deshabilitado", false),
    ],
    preview: autocomplete_preview,
    code: autocomplete_code,
    keyboard: &[
        "↓ abre la lista y recorre las opciones; ↑ vuelve.",
        "Return toma la opción activa; Escape cierra la lista o borra el texto.",
        "Tab cierra sin elegir y pasa al siguiente control.",
    ],
    accessibility: &[
        "El foco se queda siempre en el campo; la opción activa se anuncia con aria-activedescendant.",
        "Hover resalta y un clic toma la opción sin quitar el foco al campo.",
    ],
    ..REFERENCE
};

fn autocomplete_preview(props: &Props, cx: &Cx<'_>) -> Element<PreviewMsg> {
    autocomplete(
        props.text("label"),
        &cx.live.autocomplete,
        PreviewMsg::Autocomplete,
    )
    .placeholder(props.text("placeholder"))
    .helper_text(props.text("helper"))
    .required(props.on("required"))
    .disabled(props.on("disabled"))
    .test_id(cx.id)
    .build()
}

fn autocomplete_code(props: &Props) -> String {
    let body = chain(
        format!(
            "// En el estado: AutocompleteState::list(\"comuna\", COMUNAS)\nautocomplete({}, &self.comuna, Msg::Comuna)",
            lit(props.text("label"))
        ),
        [
            when(
                !props.text("placeholder").is_empty(),
                format!("placeholder({})", lit(props.text("placeholder"))),
            ),
            when(
                !props.text("helper").is_empty(),
                format!("helper_text({})", lit(props.text("helper"))),
            ),
            when(props.on("required"), "required(true)"),
            when(props.on("disabled"), "disabled(true)"),
            call("build()"),
        ],
    );
    snippet(&["autocomplete", "AutocompleteState"], body)
        + "\n\n// En update: Msg::Comuna(msg) => self.comuna.update(msg).map(Msg::Comuna)"
}

pub const COLOR_FIELD: Component = Component {
    id: "color-field",
    name: "ColorField",
    group: Group::Input,
    summary: "Color en hex con muestra y paleta.",
    doc: "color-field.md",
    props: &[
        text("label", "Etiqueta", "Color de la etiqueta"),
        text("value", "Valor", "#2563EB"),
        toggle("palette", "Paleta", true),
        toggle("allow_empty", "Permitir «Sin color»", false),
        toggle("disabled", "Deshabilitado", false),
    ],
    preview: color_preview,
    code: color_code,
    keyboard: &[
        "La paleta es una parada de Tab; las flechas recorren los colores y Espacio elige.",
        "En el campo se escribe el hex (#RRGGBB).",
    ],
    accessibility: &[
        "La paleta es un grupo de radio.",
        "Cada color se lee por su nombre («Azul») y su hex como descripción.",
    ],
    demo: Some(("Campos", "/fields")),
    ..REFERENCE
};

fn color_preview(props: &Props, cx: &Cx<'_>) -> Element<PreviewMsg> {
    let mut field = color_field(props.text("label"), props.text("value"), set_text("value"))
        .allow_empty(props.on("allow_empty"))
        .disabled(props.on("disabled"))
        .test_id(cx.id);
    if props.on("palette") {
        field = field.palette([
            ("Azul", Color::hex(0x2563eb)),
            ("Verde", Color::hex(0x16a34a)),
            ("Ámbar", Color::hex(0xd97706)),
            ("Rojo", Color::hex(0xdc2626)),
        ]);
    }
    field.build()
}

fn color_code(props: &Props) -> String {
    snippet(
        &["color_field"],
        chain(
            format!(
                "color_field({}, &self.color, Msg::Color)",
                lit(props.text("label"))
            ),
            [
                when(
                    props.on("palette"),
                    "palette([(\"Azul\", Color::hex(0x2563eb)), (\"Verde\", Color::hex(0x16a34a))])",
                ),
                when(props.on("allow_empty"), "allow_empty(true)"),
                when(props.on("disabled"), "disabled(true)"),
                call("build()"),
            ],
        ),
    )
}

pub const DATE_FIELD: Component = Component {
    id: "date-field",
    name: "DateField y TimeField",
    group: Group::Input,
    summary: "Fecha dd/mm/aaaa con calendario (la hora va en TimeField).",
    doc: "date-field.md",
    props: &[
        text("label", "Etiqueta", "Fecha de entrega"),
        text("value", "Valor", "18/04/2026"),
        toggle("calendar", "Con calendario", true),
    ],
    preview: date_preview,
    code: date_code,
    keyboard: &[
        "En el campo, ↑/↓ cambian el día, el mes o el año según dónde esté el cursor.",
        "Con el calendario abierto, el panel toma el foco y lo devuelve al cerrar; Escape lo cierra.",
        "Los días son un grupo de foco: ←/→ pasan de día y Return elige.",
    ],
    accessibility: &[
        "El calendario abierto es un Dialog «Calendario».",
        "Cada día es un botón con su nombre largo («sábado 18 de abril de 2026»).",
    ],
    demo: Some(("Campos", "/fields")),
    ..REFERENCE
};

fn date_preview(props: &Props, cx: &Cx<'_>) -> Element<PreviewMsg> {
    let mut field = date_field(props.text("label"), props.text("value"))
        .on_input(set_text("value"))
        .today(today())
        .test_id(cx.id);
    if props.on("calendar") {
        field = field.calendar(
            cx.main && cx.live.calendar_open,
            cx.live.month,
            PreviewMsg::ToggleCalendar,
            PreviewMsg::Month,
            PreviewMsg::Pick,
        );
    }
    field.build()
}

fn date_code(props: &Props) -> String {
    snippet(
        &["date_field"],
        chain(
            format!("date_field({}, &self.entrega)", lit(props.text("label"))),
            [
                call("on_input(Msg::Entrega)"),
                when(
                    props.on("calendar"),
                    "calendar(\n        self.calendario,\n        self.mes,\n        Msg::Calendario,\n        Msg::Mes,\n        Msg::Elegir,\n    )",
                ),
                call("build()"),
            ],
        ),
    )
}

const PLANS: &[&str] = &["Gratis", "Equipo", "Empresa"];

pub const SELECT: Component = Component {
    id: "select",
    name: "Select",
    group: Group::Input,
    summary: "Desplegable de una opción (NSPopUpButton).",
    doc: "select.md",
    props: &[
        choice(
            "selected",
            "Elegida",
            &["Ninguna", "Gratis", "Equipo", "Empresa"],
            2,
        ),
        text("placeholder", "Placeholder", "Elige un plan"),
        toggle("disabled", "Deshabilitado", false),
    ],
    matrix: &[Axis {
        title: "Estados",
        variants: &[
            Variant {
                label: "Sin elegir",
                set: &[Set::Choice("selected", 0), Set::Bool("disabled", false)],
            },
            Variant {
                label: "Elegida",
                set: &[Set::Choice("selected", 2), Set::Bool("disabled", false)],
            },
            Variant {
                label: "Deshabilitado",
                set: &[Set::Choice("selected", 2), Set::Bool("disabled", true)],
            },
        ],
    }],
    preview: select_preview,
    code: select_code,
    keyboard: &[
        "Clic, Espacio, Return o una flecha abren la lista.",
        "En la lista: flechas, Return elige, Escape o un clic fuera cierra.",
        "Búsqueda por tipeo: escribir salta a la opción que empieza así.",
    ],
    accessibility: &[
        "Nómbralo con .label(..) cuando un título aparte lo describe.",
        "La lista abierta es el panel kai.menu con filas kai.menu.<i>.",
    ],
    demo: Some(("Selección", "/selection")),
    ..REFERENCE
};

fn select_preview(props: &Props, cx: &Cx<'_>) -> Element<PreviewMsg> {
    let selected = props.choice("selected").checked_sub(1);
    select_with_options(
        PLANS,
        selected,
        props.text("placeholder"),
        |index| PreviewMsg::Set("selected", Value::Choice(index + 1)),
        props.on("disabled"),
    )
    .label("Plan")
    .test_id(cx.id)
}

fn select_code(props: &Props) -> String {
    let selected = match props.choice("selected").checked_sub(1) {
        Some(index) => format!("Some({index})"),
        None => "None".to_owned(),
    };
    let head = format!(
        "select_with_options(\n    &[\"Gratis\", \"Equipo\", \"Empresa\"],\n    {selected}, // self.plan\n    {},\n    Msg::Plan,\n    {},\n)",
        lit(props.text("placeholder")),
        props.on("disabled")
    );
    snippet(
        &["select_with_options"],
        chain(head, [call("label(\"Plan\")")]),
    )
}

const VIEWS: &[&str] = &["Día", "Semana", "Mes", "Año"];

pub const SEGMENTED: Component = Component {
    id: "segmented",
    name: "Segmented",
    group: Group::Input,
    summary: "Control segmentado: una opción de pocas, a la vista.",
    doc: "segmented.md",
    props: &[
        choice("selected", "Elegido", VIEWS, 1),
        choice("count", "Segmentos", &["2", "3", "4"], 2),
        toggle("disabled", "Deshabilitado", false),
    ],
    matrix: &[Axis {
        title: "Segmentos",
        variants: &[
            Variant {
                label: "2",
                set: &[Set::Choice("count", 0)],
            },
            Variant {
                label: "3",
                set: &[Set::Choice("count", 1)],
            },
            Variant {
                label: "4",
                set: &[Set::Choice("count", 2)],
            },
        ],
    }],
    preview: segmented_preview,
    code: segmented_code,
    keyboard: &[
        "Es una sola parada de Tab; las flechas recorren los segmentos.",
        "Espacio o un clic eligen (on_select(índice)).",
    ],
    accessibility: &[
        "Rol RadioGroup con un segmento por opción.",
        "Nombra el grupo con .label(..).",
    ],
    demo: Some(("Selección", "/selection")),
    ..REFERENCE
};

fn segment_count(props: &Props) -> usize {
    props.choice("count") + 2
}

fn segmented_preview(props: &Props, cx: &Cx<'_>) -> Element<PreviewMsg> {
    let count = segment_count(props);
    segmented_with_options(
        &VIEWS[..count],
        props.choice("selected").min(count - 1),
        |index| PreviewMsg::Set("selected", Value::Choice(index)),
        props.on("disabled"),
    )
    .label("Vista")
    .test_id(cx.id)
}

fn segmented_code(props: &Props) -> String {
    let count = segment_count(props);
    let options: Vec<String> = VIEWS[..count].iter().map(|v| lit(v)).collect();
    snippet(
        &["segmented_with_options"],
        chain(
            format!(
                "segmented_with_options(\n    &[{}],\n    {}, // self.vista\n    Msg::Vista,\n    {},\n)",
                options.join(", "),
                props.choice("selected").min(count - 1),
                props.on("disabled")
            ),
            [call("label(\"Vista\")")],
        ),
    )
}

pub const SWITCH: Component = Component {
    id: "switch",
    name: "Switch",
    group: Group::Input,
    summary: "Interruptor sí/no que aplica al momento.",
    doc: "switch.md",
    props: &[
        text("label", "Etiqueta", "Notificaciones"),
        toggle("checked", "Encendido", true),
        choice("density", "Densidad", &["Default", "Compact"], 0),
        toggle("disabled", "Deshabilitado", false),
    ],
    matrix: &[
        Axis {
            title: "Estados",
            variants: &[
                Variant {
                    label: "Apagado",
                    set: &[Set::Bool("checked", false), Set::Bool("disabled", false)],
                },
                Variant {
                    label: "Encendido",
                    set: &[Set::Bool("checked", true), Set::Bool("disabled", false)],
                },
                Variant {
                    label: "Deshabilitado",
                    set: &[Set::Bool("checked", true), Set::Bool("disabled", true)],
                },
            ],
        },
        Axis {
            title: "Densidad",
            variants: &[
                Variant {
                    label: "Default",
                    set: &[Set::Choice("density", 0)],
                },
                Variant {
                    label: "Compact",
                    set: &[Set::Choice("density", 1)],
                },
            ],
        },
    ],
    preview: switch_preview,
    code: switch_code,
    keyboard: &["Tab lo enfoca; Espacio lo alterna."],
    accessibility: &[
        "Rol Switch con estado encendido/apagado.",
        "Sin switch_labeled, nómbralo con .label(..).",
    ],
    demo: Some(("Selección", "/selection")),
    ..REFERENCE
};

fn switch_preview(props: &Props, cx: &Cx<'_>) -> Element<PreviewMsg> {
    let checked = props.on("checked");
    let density = [SwitchDensity::Default, SwitchDensity::Compact][props.choice("density").min(1)];
    switch_labeled(
        props.text("label"),
        checked,
        PreviewMsg::Set("checked", Value::Bool(!checked)),
        props.on("disabled"),
        density,
    )
    .test_id(cx.id)
}

fn switch_code(props: &Props) -> String {
    snippet(
        &["switch_labeled", "SwitchDensity"],
        format!(
            "switch_labeled(\n    {},\n    self.avisos, // {}\n    Msg::Avisos,\n    {},\n    SwitchDensity::{},\n)",
            lit(props.text("label")),
            props.on("checked"),
            props.on("disabled"),
            ["Default", "Compact"][props.choice("density")]
        ),
    )
}

const CHECK_STATES: &[&str] = &["Off", "On", "Mixed"];

pub const CHECKBOX: Component = Component {
    id: "checkbox",
    name: "Checkbox",
    group: Group::Input,
    summary: "Casilla con estado mixto; el título también la alterna.",
    doc: "checkbox.md",
    props: &[
        text("label", "Etiqueta", "Acepto los términos"),
        choice("state", "Estado", CHECK_STATES, 1),
        toggle("disabled", "Deshabilitado", false),
    ],
    matrix: &[Axis {
        title: "Estados",
        variants: &[
            Variant {
                label: "Off",
                set: &[Set::Choice("state", 0)],
            },
            Variant {
                label: "On",
                set: &[Set::Choice("state", 1)],
            },
            Variant {
                label: "Mixed",
                set: &[Set::Choice("state", 2)],
            },
            Variant {
                label: "Deshabilitado",
                set: &[Set::Bool("disabled", true)],
            },
        ],
    }],
    preview: checkbox_preview,
    code: checkbox_code,
    keyboard: &[
        "Tab la enfoca; Espacio la alterna.",
        "Qué hace un clic desde Mixed lo decide la app (on_toggle).",
    ],
    accessibility: &[
        "Rol Checkbox con estado marcado, desmarcado o mixto.",
        "La casilla y su título son un solo control: el nombre es el título.",
    ],
    demo: Some(("Selección", "/selection")),
    ..REFERENCE
};

fn checkbox_preview(props: &Props, cx: &Cx<'_>) -> Element<PreviewMsg> {
    let state =
        [ControlState::Off, ControlState::On, ControlState::Mixed][props.choice("state").min(2)];
    let next = if state == ControlState::On { 0 } else { 1 };
    checkbox_with_options(
        props.text("label"),
        state,
        PreviewMsg::Set("state", Value::Choice(next)),
        props.on("disabled"),
    )
    .test_id(cx.id)
}

fn checkbox_code(props: &Props) -> String {
    snippet(
        &["checkbox_with_options"],
        format!(
            "use kai::element::ControlState;\n\ncheckbox_with_options(\n    {},\n    ControlState::{}, // self.terminos\n    Msg::Terminos,\n    {},\n)",
            lit(props.text("label")),
            CHECK_STATES[props.choice("state")],
            props.on("disabled")
        ),
    )
}

const MEDIA: &[&str] = &["Avatar", "Banner", "Logo"];

pub const MEDIA_SLOT: Component = Component {
    id: "media-slot",
    name: "MediaSlot",
    group: Group::Input,
    summary: "Imagen de un formulario: foto, portada o logo.",
    doc: "media-slot.md",
    props: &[
        choice("variant", "Variante", MEDIA, 0),
        text("error", "Error", ""),
        toggle("disabled", "Deshabilitado", false),
    ],
    matrix: &[Axis {
        title: "Variantes",
        variants: &[
            Variant {
                label: "Avatar",
                set: &[Set::Choice("variant", 0)],
            },
            Variant {
                label: "Banner",
                set: &[Set::Choice("variant", 1)],
            },
            Variant {
                label: "Logo",
                set: &[Set::Choice("variant", 2)],
            },
        ],
    }],
    preview: media_preview,
    code: media_code,
    keyboard: &["Tab lo enfoca; Espacio o Return piden el archivo (on_pick)."],
    accessibility: &[
        "Es un botón nombrado por su acción: «Agregar imagen» o «Cambiar imagen».",
        "El error se muestra bajo la imagen con rol Alert.",
    ],
    demo: Some(("Multimedia", "/media")),
    ..REFERENCE
};

fn media_preview(props: &Props, cx: &Cx<'_>) -> Element<PreviewMsg> {
    let variant = [
        MediaVariant::Avatar(AvatarSize::Lg),
        MediaVariant::Banner,
        MediaVariant::Logo,
    ][props.choice("variant").min(2)];
    let error = props.text("error");
    kai::element::div().class("w-64").child(
        MediaSlot::new(variant)
            .on_pick(PreviewMsg::Event("on_pick"))
            .error((!error.is_empty()).then(|| error.to_owned()))
            .disabled(props.on("disabled"))
            .test_id(cx.id)
            .build(),
    )
}

fn media_code(props: &Props) -> String {
    let variant = match props.choice("variant") {
        0 => "MediaVariant::Avatar(AvatarSize::Lg)",
        1 => "MediaVariant::Banner",
        _ => "MediaVariant::Logo",
    };
    let error = props.text("error");
    snippet(
        &["MediaSlot", "MediaVariant"],
        chain(
            format!("MediaSlot::new({variant})"),
            [
                call("current(self.foto.clone())"),
                call("on_pick(Msg::ElegirFoto)"),
                when(
                    !error.is_empty(),
                    format!("error(Some({}.into()))", lit(error)),
                ),
                when(props.on("disabled"), "disabled(true)"),
                call("build()"),
            ],
        ),
    )
}
