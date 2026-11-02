//! Overlays: Dialog and Toast. Both open from the preview; the page draws
//! the dialog over the whole window ([`Component::overlay`]) and the demo
//! root draws the toast.

use kai::components::{ButtonVariant, Dialog, DialogWidth, button, caption};
use kai::element::{Element, div};

use super::code::{call, chain, lit, snippet, when};
use super::content::alert_variant;
use super::{Component, Cx, Group, PreviewMsg, Props, REFERENCE, choice, text, toggle};

const WIDTHS: &[&str] = &["Sm", "Md", "Lg"];

pub const DIALOG: Component = Component {
    id: "dialog",
    name: "Dialog",
    group: Group::Overlays,
    summary: "Diálogo modal con botones por defecto y cancelar.",
    doc: "dialog.md",
    props: &[
        text("title", "Título", "¿Borrar el cliente?"),
        text("message", "Mensaje", "Esta acción no se puede deshacer."),
        text("confirm", "Botón por defecto", "Borrar"),
        toggle("cancel", "Botón cancelar", true),
        toggle("destructive", "Destructivo", true),
        choice("width", "Ancho", WIDTHS, 0),
    ],
    preview: dialog_preview,
    overlay: Some(dialog_overlay),
    code: dialog_code,
    keyboard: &[
        "Modal: el foco queda dentro, los clics fuera se ignoran y vuelve al cerrar.",
        "Return presiona el botón por defecto; Escape y ⌘. el de cancelar (o on_dismiss).",
    ],
    accessibility: &["Rol Dialog con el título como nombre."],
    demo: Some(("Diálogos y avisos", "/dialogs")),
    ..REFERENCE
};

fn dialog_preview(_: &Props, cx: &Cx<'_>) -> Element<PreviewMsg> {
    div().class("flex flex-col items-start gap-2").children([
        button("Abrir diálogo", ButtonVariant::Outlined)
            .on_click(PreviewMsg::OpenOverlay)
            .test_id(cx.id),
        caption("Se abre sobre toda la ventana, con las props de la izquierda."),
    ])
}

fn dialog_overlay(props: &Props, _: &Cx<'_>) -> Element<PreviewMsg> {
    let width = [DialogWidth::Sm, DialogWidth::Md, DialogWidth::Lg][props.choice("width").min(2)];
    let mut dialog = Dialog::new(props.text("title"))
        .message(props.text("message"))
        .default_button(
            props.text("confirm"),
            PreviewMsg::CloseOverlay("default_button"),
        )
        .destructive(props.on("destructive"))
        .width(width)
        .test_id("components.dialog");
    dialog = if props.on("cancel") {
        dialog.cancel_button("Cancelar", PreviewMsg::CloseOverlay("cancel_button"))
    } else {
        dialog.on_dismiss(PreviewMsg::CloseOverlay("on_dismiss"))
    };
    dialog.build()
}

fn dialog_code(props: &Props) -> String {
    let body = chain(
        format!("Dialog::new({})", lit(props.text("title"))),
        [
            call(format!("message({})", lit(props.text("message")))),
            call(format!(
                "default_button({}, Msg::Confirmar)",
                lit(props.text("confirm"))
            )),
            if props.on("cancel") {
                call("cancel_button(\"Cancelar\", Msg::Cerrar)")
            } else {
                call("on_dismiss(Msg::Cerrar)")
            },
            when(props.on("destructive"), "destructive(true)"),
            when(
                props.choice("width") != 0,
                format!("width(DialogWidth::{})", WIDTHS[props.choice("width")]),
            ),
            call("build()"),
        ],
    );
    snippet(
        &["Dialog", "DialogWidth"],
        format!("// En Page::overlay, mientras está abierto:\n{body}"),
    )
}

const TOASTS: &[&str] = &["Success", "Info", "Warning", "Error"];

pub const TOAST: Component = Component {
    id: "toast",
    name: "Toast",
    group: Group::Overlays,
    summary: "Avisos en la esquina que se cierran solos.",
    doc: "toast.md",
    props: &[
        choice("variant", "Variante", TOASTS, 0),
        text("title", "Título", "Cliente guardado"),
        text("message", "Mensaje", "Los cambios ya están en el servidor."),
    ],
    preview: |props, cx| {
        div().class("flex flex-col items-start gap-2").children([
            button("Mostrar aviso", ButtonVariant::Outlined)
                .on_click(PreviewMsg::Toast(
                    alert_variant(props.choice("variant")),
                    props.text("title").to_owned(),
                    props.text("message").to_owned(),
                ))
                .test_id(cx.id),
            caption("Aparece abajo a la derecha; los errores no se cierran solos."),
        ])
    },
    code: |props| {
        let body = chain(
            format!(
                "Toast::new(0, AlertVariant::{}, {})",
                TOASTS[props.choice("variant")],
                lit(props.text("title"))
            ),
            [when(
                !props.text("message").is_empty(),
                format!("message({})", lit(props.text("message"))),
            )],
        );
        snippet(
            &["AlertVariant", "Toast"],
            format!(
                "// En Page::update:\ncx.toast(\n    {}\n);",
                body.replace('\n', "\n    ")
            ),
        )
    },
    accessibility: &[
        "Errores: rol Alert (se leen al momento). El resto: rol Status.",
        "Cada aviso tiene botón de cerrar.",
    ],
    demo: Some(("Diálogos y avisos", "/dialogs")),
    ..REFERENCE
};
