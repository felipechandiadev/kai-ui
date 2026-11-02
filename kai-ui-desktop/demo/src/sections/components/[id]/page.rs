//! /components/<id>: the page of a registry component
//! ([`crate::showcase`]): a live instance with its props panel, the
//! variant matrix, the code for those props with «Copiar», and the
//! keyboard and accessibility notes (docs/dev/34).

use std::time::Duration;

use kai::components::{
    Button, ButtonVariant, EmptyState, FieldDensity, PageHeader, SwitchDensity, TextField, Toast,
    button, caption, card, card_content, card_header, page, page_body, page_grid, page_grid_item,
    segmented, select, switch_labeled,
};
use kai::element::{Element, Role, div, text};
use kai::router::{Context, Page};
use kai::runtime::Effects;

use crate::showcase::{Component, Control, Cx, Live, PreviewMsg, Props, Value, find};
use crate::state::State;

/// How long the button says «Copiado» after copying.
pub const COPIED_FOR: Duration = Duration::from_secs(2);

pub struct Screen {
    component: Option<&'static Component>,
    props: Props,
    live: Live,
    copied: bool,
}

#[derive(Clone, Debug)]
pub enum Msg {
    Prop(&'static str, Value),
    Reset,
    Preview(PreviewMsg),
    /// Matrix cells change nothing.
    Matrix,
    Copy,
    CopiedDone,
    Demo(&'static str),
    Index,
}

impl Page for Screen {
    type Msg = Msg;
    type Params = String;
    type State = State;

    fn init(id: &String, state: &State) -> (Self, Effects<Msg>) {
        let component = find(id);
        let props = component
            .map(|component| state.components.props(component))
            .unwrap_or_default();
        (
            Self {
                component,
                props,
                live: Live::default(),
                copied: false,
            },
            Effects::none(),
        )
    }

    fn update(&mut self, msg: Msg, cx: &mut Context<'_, State>) -> Effects<Msg> {
        let Some(component) = self.component else {
            if let Msg::Index = msg {
                cx.navigate("/components");
            }
            return Effects::none();
        };
        let effects = match msg {
            Msg::Prop(key, value) => {
                self.props.set(key, value);
                Effects::none()
            }
            Msg::Reset => {
                self.props = component.defaults();
                Effects::none()
            }
            Msg::Preview(PreviewMsg::Toast(variant, title, message)) => {
                let toast = Toast::new(0, variant, title);
                cx.toast(if message.is_empty() {
                    toast
                } else {
                    toast.message(message)
                });
                self.live
                    .update(PreviewMsg::Event("toast"), &mut self.props)
                    .map(Msg::Preview)
            }
            Msg::Preview(msg) => self.live.update(msg, &mut self.props).map(Msg::Preview),
            Msg::Matrix => Effects::none(),
            Msg::Copy => {
                self.copied = true;
                return Effects::batch([
                    Effects::copy_text((component.code)(&self.props)),
                    Effects::after(COPIED_FOR, Msg::CopiedDone).keyed("components.copied"),
                ]);
            }
            Msg::CopiedDone => {
                self.copied = false;
                Effects::none()
            }
            Msg::Demo(path) => {
                cx.navigate(path);
                Effects::none()
            }
            Msg::Index => {
                cx.navigate("/components");
                Effects::none()
            }
        };
        cx.state().components.remember(component, &self.props);
        effects
    }

    fn view(&self, _: &State) -> Element<Msg> {
        let Some(component) = self.component else {
            return page().child(
                page_body().child(
                    EmptyState::resource("Componente no encontrado")
                        .description("No hay una ficha con ese nombre.")
                        .action("Ver todos los componentes", Msg::Index)
                        .test_id("components.missing")
                        .build(),
                ),
            );
        };
        let header = PageHeader::new(component.name)
            .subtitle(component.summary)
            .breadcrumbs(["Fichas de componentes", component.group.label()])
            .action(
                button("Todos los componentes", ButtonVariant::Outlined)
                    .on_click(Msg::Index)
                    .test_id("components.index"),
            )
            .test_id("components.header")
            .build();
        let mut left = vec![self.preview(component), self.code(component)];
        if !component.matrix.is_empty() {
            left.push(self.matrix(component));
        }
        let right = vec![self.props_panel(component), notes(component)];
        page().child(page_body().children([
            header,
            page_grid().children([
                page_grid_item(8).child(div().class("flex flex-col gap-4").children(left)),
                page_grid_item(4).child(div().class("flex flex-col gap-4").children(right)),
            ]),
        ]))
    }

    fn overlay(&self, _: &State) -> Option<Element<Msg>> {
        let component = self.component?;
        let overlay = component.overlay?;
        self.live
            .overlay_open
            .then(|| overlay(&self.props, &self.cx("components.live", true)).map(Msg::Preview))
    }

    fn title(&self, _: &State) -> String {
        match self.component {
            Some(component) => component.name.to_owned(),
            None => "Componente no encontrado".to_owned(),
        }
    }
}

impl Screen {
    fn cx<'a>(&'a self, id: &'a str, main: bool) -> Cx<'a> {
        Cx {
            live: &self.live,
            id,
            main,
        }
    }

    fn preview(&self, component: &Component) -> Element<Msg> {
        let live = (component.preview)(&self.props, &self.cx("components.live", true));
        let event = match &self.live.last_event {
            Some(event) => format!("Último evento: {event}"),
            None => "Interactúa con la vista previa: los eventos aparecen aquí.".to_owned(),
        };
        card().test_id("components.preview").children([
            card_header("Vista previa", None),
            card_content().class("flex flex-col gap-3").children([
                div()
                    .class("flex items-center justify-center min-h-32 p-6 rounded-md border border-border")
                    .child(live.map(Msg::Preview)),
                caption(event)
                    .role(Role::Status)
                    .test_id("components.event"),
            ]),
        ])
    }

    fn code(&self, component: &Component) -> Element<Msg> {
        let code = (component.code)(&self.props);
        let lines = code.lines().map(|line| {
            text(if line.is_empty() { " " } else { line })
                .class("text-xs leading-5 whitespace-nowrap text-foreground")
        });
        let copy = Button::new(if self.copied { "Copiado" } else { "Copiar" })
            .on_click(Msg::Copy)
            .build()
            .test_id("components.copy");
        let mut content = vec![
            div()
                .class("flex flex-col p-3 rounded-md border border-border overflow-x-auto")
                .role(Role::Group)
                .label("Código")
                .test_id("components.code")
                .children(lines),
        ];
        if self.copied {
            content.push(
                caption("Código copiado al portapapeles")
                    .role(Role::Status)
                    .test_id("components.copied"),
            );
        }
        card().test_id("components.snippet").children([
            div()
                .class("flex items-center justify-between gap-2 pr-4")
                .children([
                    card_header("Código", Some("Refleja las props actuales")),
                    copy,
                ]),
            card_content()
                .class("flex flex-col gap-2")
                .children(content),
        ])
    }

    fn matrix(&self, component: &Component) -> Element<Msg> {
        let axes = component.matrix.iter().enumerate().map(|(a, axis)| {
            let cells = axis.variants.iter().enumerate().map(|(v, variant)| {
                let id = format!("components.matrix.{a}.{v}");
                let props = self.props.with(variant.set);
                div().class("flex flex-col items-start gap-2").children([
                    caption(variant.label),
                    (component.preview)(&props, &self.cx(&id, false)).map(|_| Msg::Matrix),
                ])
            });
            div().class("flex flex-col gap-2").children([
                text(axis.title).class("text-sm font-medium text-foreground"),
                div()
                    .class("flex flex-wrap items-end gap-6")
                    .children(cells),
            ])
        });
        card().test_id("components.matrix").children([
            card_header(
                "Matriz de variantes",
                Some("Las props actuales, cambiando una a la vez"),
            ),
            card_content().class("flex flex-col gap-5").children(axes),
        ])
    }

    fn props_panel(&self, component: &Component) -> Element<Msg> {
        let content: Vec<Element<Msg>> = if component.is_live() {
            component
                .props
                .iter()
                .map(|prop| self.control(prop.key, prop.label, prop.control))
                .chain([button("Restablecer", ButtonVariant::Text)
                    .on_click(Msg::Reset)
                    .test_id("components.reset")])
                .collect()
        } else {
            vec![caption(
                "Sin props que editar aquí: es una ficha de referencia.",
            )]
        };
        card().test_id("components.props").children([
            card_header("Props", Some("Cambian la vista previa y el código")),
            card_content()
                .class("flex flex-col items-stretch gap-3")
                .children(content),
        ])
    }

    fn control(&self, key: &'static str, label: &'static str, control: Control) -> Element<Msg> {
        let id = format!("components.prop.{key}");
        match control {
            Control::Toggle(_) => {
                let on = self.props.on(key);
                switch_labeled(
                    label,
                    on,
                    Msg::Prop(key, Value::Bool(!on)),
                    false,
                    SwitchDensity::Compact,
                )
                .test_id(id)
            }
            Control::Choice(options, _) => {
                let selected = self.props.choice(key);
                let pick = move |index| Msg::Prop(key, Value::Choice(index));
                let short = options.len() <= 4 && options.iter().all(|o| o.chars().count() <= 9);
                let picker = if short {
                    segmented(options, selected, pick)
                } else {
                    select(options, Some(selected), "", pick)
                };
                div()
                    .class("flex flex-col gap-1")
                    .children([caption(label), picker.label(label).test_id(id)])
            }
            Control::Text(_) => TextField::new(label, self.props.text(key))
                .density(FieldDensity::Compact)
                .on_input(move |value| Msg::Prop(key, Value::Text(value)))
                .test_id(id)
                .build(),
        }
    }
}

/// Keyboard, accessibility, docs and where the demo shows it.
fn notes(component: &Component) -> Element<Msg> {
    let list = |title: &str, items: &[&str], id: &str| {
        div().class("flex flex-col gap-1").test_id(id).children(
            std::iter::once(text(title).class("text-sm font-medium text-foreground")).chain(
                items
                    .iter()
                    .map(|item| text(format!("• {item}")).class("text-sm text-foreground")),
            ),
        )
    };
    let mut content = Vec::new();
    if !component.keyboard.is_empty() {
        content.push(list("Teclado", component.keyboard, "components.keyboard"));
    }
    if !component.accessibility.is_empty() {
        content.push(list(
            "Accesibilidad",
            component.accessibility,
            "components.a11y",
        ));
    }
    content.push(
        caption(format!("Documentación: {}", component.doc_path())).test_id("components.doc"),
    );
    if let Some((section, path)) = component.demo {
        content.push(
            button(format!("Ver en la demo: {section}"), ButtonVariant::Text)
                .on_click(Msg::Demo(path))
                .test_id("components.demo"),
        );
    }
    card().test_id("components.notes").children([
        card_header("Teclado y accesibilidad", None),
        card_content()
            .class("flex flex-col items-start gap-3")
            .children(content),
    ])
}
