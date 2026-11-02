//! /components: the index of component pages, one per page in
//! `docs/componentes`, by group. Each row opens `/components/<id>`
//! (docs/dev/34).

use kai::components::{
    BadgeVariant, PageHeader, badge, caption, card, card_content, card_header, page, page_body,
    page_grid, page_grid_item, search_field,
};
use kai::element::{Element, Role, div, text};
use kai::router::{Context, Page};
use kai::runtime::Effects;

use crate::showcase::{COMPONENTS, Component, Group};
use crate::state::State;

pub struct Screen {
    query: String,
}

#[derive(Clone, Debug)]
pub enum Msg {
    Query(String),
    Open(&'static str),
}

impl Page for Screen {
    type Msg = Msg;
    type Params = ();
    type State = State;

    fn init(_: &(), _: &State) -> (Self, Effects<Msg>) {
        (
            Self {
                query: String::new(),
            },
            Effects::none(),
        )
    }

    fn update(&mut self, msg: Msg, cx: &mut Context<'_, State>) -> Effects<Msg> {
        match msg {
            Msg::Query(query) => self.query = query,
            Msg::Open(id) => cx.navigate(format!("/components/{id}")),
        }
        Effects::none()
    }

    fn view(&self, _: &State) -> Element<Msg> {
        let query = self.query.trim().to_lowercase();
        let matches = |component: &Component| {
            query.is_empty()
                || component.name.to_lowercase().contains(&query)
                || component.summary.to_lowercase().contains(&query)
        };
        let groups: Vec<Element<Msg>> = Group::ALL
            .into_iter()
            .filter_map(|group| {
                let rows: Vec<Element<Msg>> = COMPONENTS
                    .iter()
                    .filter(|component| component.group == group && matches(component))
                    .map(row)
                    .collect();
                (!rows.is_empty()).then(|| {
                    page_grid_item(6).child(
                        card()
                            .test_id(format!("components.group.{group:?}").to_lowercase())
                            .children([
                                card_header(group.label(), None),
                                card_content().class("flex flex-col gap-1").children(rows),
                            ]),
                    )
                })
            })
            .collect();
        let live = COMPONENTS.iter().filter(|c| c.is_live()).count();
        let body = if groups.is_empty() {
            vec![
                caption(format!("Ningún componente coincide con «{}».", self.query))
                    .role(Role::Status)
                    .test_id("components.none"),
            ]
        } else {
            vec![page_grid().children(groups)]
        };
        page().child(
            page_body()
                .children([PageHeader::new("Fichas de componentes")
                    .subtitle(format!(
                        "{} componentes de docs/componentes; {live} con props en vivo",
                        COMPONENTS.len()
                    ))
                    .action(
                        search_field(self.query.clone(), Msg::Query)
                            .placeholder("Filtrar componentes")
                            .test_id("components.filter")
                            .build(),
                    )
                    .build()])
                .children(body),
        )
    }

    fn title(&self, _: &State) -> String {
        "Fichas de componentes".into()
    }
}

/// An index row: name, summary and whether it has live props.
fn row(component: &'static Component) -> Element<Msg> {
    let (tag, variant) = if component.is_live() {
        ("En vivo", BadgeVariant::SuccessOutlined)
    } else {
        ("Referencia", BadgeVariant::SecondaryOutlined)
    };
    div()
        .class(
            "flex items-center gap-3 px-2 py-1.5 rounded-md cursor-pointer \
             hover:bg-primary/10 focus-visible:ring-2 focus-visible:ring-primary/40",
        )
        .role(Role::Button)
        .label(component.name)
        .on_click(Msg::Open(component.id))
        .test_id(format!("components.item.{}", component.id))
        .children([
            div().class("flex flex-col flex-1 min-w-0").children([
                text(component.name).class("text-sm font-medium text-foreground"),
                caption(component.summary),
            ]),
            badge(tag, variant),
        ])
}
