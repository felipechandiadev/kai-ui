//! /dialogs: modal dialogs with trapped focus, toasts of each variant
//! (the error one with «Reintentar») and a button that opens the palette.

use kai::commands::{CommandRegistry, PALETTE, PageAction};
use kai::components::{
    AlertVariant, BezelStyle, Button, ButtonVariant, Dialog, DialogWidth, Toast, button, caption,
    card, card_content, card_header, page, page_body,
};
use kai::element::{Element, div};
use kai::router::{Context, HasPage, Page};
use kai::runtime::Effects;
use schemars::JsonSchema;
use serde::Deserialize;

use crate::state::State;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DemoDialog {
    Save,
    Delete,
}

/// Variant of a toast, for `toasts.show`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ToastKind {
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

#[derive(Default)]
pub struct Screen {
    dialog: Option<DemoDialog>,
}

#[derive(Clone, Debug)]
pub enum Msg {
    Open(DemoDialog),
    Confirm,
    Close,
    Toast(ToastKind),
}

impl Page for Screen {
    type Msg = Msg;
    type Params = ();
    type State = State;

    fn init(_: &(), _: &State) -> (Self, Effects<Msg>) {
        (Self::default(), Effects::none())
    }

    fn update(&mut self, msg: Msg, cx: &mut Context<'_, State>) -> Effects<Msg> {
        match msg {
            Msg::Open(dialog) => self.dialog = Some(dialog),
            Msg::Close => self.dialog = None,
            Msg::Confirm => match self.dialog.take() {
                Some(DemoDialog::Save) => {
                    let (_, title, message) = ToastKind::Success.texts();
                    cx.toast(Toast::new(0, AlertVariant::Success, title).message(message));
                }
                Some(DemoDialog::Delete) => cx.toast(
                    Toast::new(0, AlertVariant::Info, "Borrador eliminado")
                        .message("Se movió a la papelera"),
                ),
                None => {}
            },
            Msg::Toast(kind) => {
                let (_, title, message) = kind.texts();
                if kind == ToastKind::Error {
                    cx.toast_with_action(
                        Toast::new(0, kind.alert(), title)
                            .message(message)
                            .action("Reintentar", Msg::Toast(ToastKind::Success)),
                    );
                } else {
                    cx.toast(Toast::new(0, kind.alert(), title).message(message));
                }
            }
        }
        Effects::none()
    }

    fn view(&self, _: &State) -> Element<Msg> {
        let card = card()
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
                            .on_click(Msg::Open(DemoDialog::Save))
                            .test_id("dialogs.save"),
                        button("Eliminar borrador…", ButtonVariant::Danger)
                            .on_click(Msg::Open(DemoDialog::Delete))
                            .test_id("dialogs.delete"),
                    ]),
                    div()
                        .class("flex flex-wrap items-center gap-3")
                        .child(caption("Avisos:"))
                        .children(ToastKind::ALL.map(|kind| {
                            button(kind.texts().0, ButtonVariant::Secondary)
                                .on_click(Msg::Toast(kind))
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
            ]);
        page().child(page_body().child(card))
    }

    /// The open dialog, over the whole window.
    fn overlay(&self, _: &State) -> Option<Element<Msg>> {
        self.dialog.map(dialog)
    }

    fn title(&self, _: &State) -> String {
        "Diálogos y avisos".into()
    }

    fn commands<A: HasPage<Self>>(registry: &mut CommandRegistry<A>) {
        registry.page_action::<ShowToast>().expose();
    }
}

fn dialog(kind: DemoDialog) -> Element<Msg> {
    let dialog = match kind {
        DemoDialog::Save => Dialog::new("¿Guardar los cambios?")
            .message("Los cambios quedarán disponibles para todo el equipo.")
            .default_button("Guardar", Msg::Confirm),
        DemoDialog::Delete => Dialog::new("¿Eliminar el borrador?")
            .message("Esta acción no se puede deshacer.")
            .destructive(true)
            .width(DialogWidth::Sm)
            .default_button("Eliminar", Msg::Confirm),
    };
    dialog
        .cancel_button("Cancelar", Msg::Close)
        .test_id("dialog")
        .build()
}

/// `toasts.show`: shows a toast, as the buttons do.
#[derive(Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ShowToast {
    #[schemars(description = "Variante del aviso")]
    kind: ToastKind,
}

impl PageAction for ShowToast {
    type Page = Screen;
    const ID: &'static str = "toasts.show";
    const TITLE: &'static str = "Mostrar aviso";
    fn description() -> &'static str {
        "Muestra un aviso (toast) que se cierra solo a los 4 segundos"
    }
    fn into_msg(self) -> Msg {
        Msg::Toast(self.kind)
    }
}
