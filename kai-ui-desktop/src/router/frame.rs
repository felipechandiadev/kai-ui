//! Layouts: the frame around a page (docs/dev/28).
//!
//! A [`Layout`] draws the bars and panels around the content of its folder
//! and leaves an [`outlet`] where the content goes. The root maps the
//! layout's view to its own message and fills the outlet with what is
//! inside, from the page outwards ([`Element::fill_outlet`]).

use crate::element::{Element, div};
use crate::runtime::Effects;

use super::Context;

/// Key of the element [`outlet`] returns; the root replaces it before the
/// tree reaches a session.
const OUTLET_KEY: &str = "kai.outlet";

/// The place in a layout's view where its content goes: exactly one per
/// layout. The root replaces it with the page, or with the next layout
/// inside, through [`Element::fill_outlet`].
pub fn outlet<Msg>() -> Element<Msg> {
    div()
        .key(OUTLET_KEY)
        .class("flex-1 min-w-0 min-h-0 flex flex-col")
}

impl<Msg> Element<Msg> {
    /// Replace the first [`outlet`] of this tree with `content`. Without
    /// one, the tree is returned as it was and debug builds panic: a
    /// layout that forgot its outlet would hide its page.
    pub fn fill_outlet(mut self, content: Element<Msg>) -> Element<Msg> {
        let mut content = Some(content);
        let filled = fill(&mut self, &mut content);
        debug_assert!(filled, "a layout's view has no outlet()");
        self
    }

    /// How many [`outlet`]s this tree has; a layout's view has one.
    pub fn outlet_count(&self) -> usize {
        usize::from(self.key.as_deref() == Some(OUTLET_KEY))
            + self
                .children
                .iter()
                .map(Element::outlet_count)
                .sum::<usize>()
    }
}

fn fill<Msg>(element: &mut Element<Msg>, content: &mut Option<Element<Msg>>) -> bool {
    if element.key.as_deref() == Some(OUTLET_KEY) {
        return match content.take() {
            Some(content) => {
                *element = content;
                true
            }
            None => false,
        };
    }
    element
        .children
        .iter_mut()
        .any(|child| fill(child, content))
}

/// What a layout may read of where the app is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RouteInfo<'a> {
    /// The current route as a path: `/customers/42`.
    pub path: &'a str,
    /// The page's title, user-visible.
    pub title: &'a str,
    pub can_back: bool,
    pub can_forward: bool,
    pub sidebar_collapsed: bool,
}

/// The frame around the pages of a folder: it lives while the route stays
/// inside the folder, so its state (a search box, a filter) survives going
/// from `/customers` to `/customers/42`. Same shape as [`Page`](super::Page).
pub trait Layout: Sized + 'static {
    /// `Infallible` when the layout's buttons only send commands.
    type Msg: Clone + std::fmt::Debug + 'static;
    /// State the whole app shares (session, loaded data).
    type State: 'static;

    fn init(state: &Self::State) -> (Self, Effects<Self::Msg>);

    fn update(
        &mut self,
        _msg: Self::Msg,
        _cx: &mut Context<'_, Self::State>,
    ) -> Effects<Self::Msg> {
        Effects::none()
    }

    /// Must contain exactly one [`outlet`].
    fn view(&self, state: &Self::State, route: &RouteInfo<'_>) -> Element<Self::Msg>;
}

#[cfg(test)]
mod tests {
    use std::convert::Infallible;

    use super::*;
    use crate::components::{ButtonVariant, button};
    use crate::element::{Role, text};
    use crate::router::Context;
    use crate::runtime::App;
    use crate::testing::{By, TestApp};

    #[test]
    fn an_outlet_is_replaced_by_the_content() {
        let view: Element<()> =
            div().children([text("encabezado"), div().child(outlet()), text("pie")]);
        assert_eq!(view.outlet_count(), 1);
        let filled = view.fill_outlet(text("contenido"));
        assert_eq!(filled.outlet_count(), 0);
        assert_eq!(filled.text_content(), "encabezado contenido pie");
    }

    #[test]
    fn only_the_first_outlet_is_filled() {
        let view: Element<()> = div().children([outlet(), outlet()]);
        assert_eq!(view.outlet_count(), 2);
        let filled = view.fill_outlet(text("x"));
        assert_eq!(filled.outlet_count(), 1);
    }

    #[test]
    #[should_panic(expected = "no outlet()")]
    fn a_layout_without_an_outlet_is_a_bug() {
        let _ = div::<()>()
            .child(text("sin hueco"))
            .fill_outlet(text("página"));
    }

    // --- A root written by hand: outer layout > inner layout > page --------

    #[derive(Default)]
    struct Shared {
        log: Vec<&'static str>,
    }

    #[derive(Clone, Debug, PartialEq)]
    enum OuterMsg {
        Bump,
    }

    struct Outer {
        bumps: u32,
    }

    impl Layout for Outer {
        type Msg = OuterMsg;
        type State = Shared;

        fn init(_: &Shared) -> (Self, Effects<OuterMsg>) {
            (Self { bumps: 0 }, Effects::none())
        }

        fn update(&mut self, msg: OuterMsg, cx: &mut Context<'_, Shared>) -> Effects<OuterMsg> {
            match msg {
                OuterMsg::Bump => {
                    self.bumps += 1;
                    cx.state().log.push("outer");
                }
            }
            Effects::none()
        }

        fn view(&self, _: &Shared, route: &RouteInfo<'_>) -> Element<OuterMsg> {
            div().children([
                text(format!("barra {} {}", route.title, self.bumps)),
                button("Barra", ButtonVariant::Secondary).on_click(OuterMsg::Bump),
                outlet(),
            ])
        }
    }

    /// No messages: the layout only draws.
    struct Inner;

    impl Layout for Inner {
        type Msg = Infallible;
        type State = Shared;

        fn init(_: &Shared) -> (Self, Effects<Infallible>) {
            (Self, Effects::none())
        }

        fn view(&self, _: &Shared, _: &RouteInfo<'_>) -> Element<Infallible> {
            div().children([text("panel"), outlet()])
        }
    }

    #[derive(Clone, Debug, PartialEq)]
    enum RootMsg {
        Outer(OuterMsg),
        Page,
    }

    struct Root {
        shared: Shared,
        outer: Outer,
        inner: Inner,
        page_clicks: u32,
    }

    impl App for Root {
        type Msg = RootMsg;

        fn title(&self) -> String {
            "Página".into()
        }

        fn update(&mut self, msg: RootMsg) -> Effects<RootMsg> {
            match msg {
                RootMsg::Outer(msg) => {
                    let mut cx = Context::new(&mut self.shared);
                    self.outer.update(msg, &mut cx).map(RootMsg::Outer)
                }
                RootMsg::Page => {
                    self.page_clicks += 1;
                    Effects::none()
                }
            }
        }

        fn view(&self) -> Element<RootMsg> {
            let route = RouteInfo {
                path: "/x",
                title: "Página",
                can_back: false,
                can_forward: false,
                sidebar_collapsed: false,
            };
            let page: Element<RootMsg> = div().role(Role::Main).children([
                text(format!("página {}", self.page_clicks)),
                button("Página", ButtonVariant::Secondary).on_click(RootMsg::Page),
            ]);
            // Innermost first: the page, then each layout around it.
            let content = {
                let inner = self.inner.view(&self.shared, &route);
                let inner: Element<RootMsg> = inner.map(|never| match never {});
                inner.fill_outlet(page)
            };
            self.outer
                .view(&self.shared, &route)
                .map(RootMsg::Outer)
                .fill_outlet(content)
        }
    }

    fn root() -> TestApp<Root> {
        TestApp::new(Root {
            shared: Shared::default(),
            outer: Outer::init(&Shared::default()).0,
            inner: Inner::init(&Shared::default()).0,
            page_clicks: 0,
        })
    }

    #[test]
    fn nested_layouts_wrap_the_page_in_order() {
        let app = root();
        let tree = app.app().view();
        assert_eq!(tree.outlet_count(), 0);
        assert_eq!(
            tree.text_content(),
            "barra Página 0 Barra panel página 0 Página"
        );
    }

    #[test]
    fn layout_and_page_messages_reach_their_own_update() {
        let mut app = root();
        app.click(By::text("Barra"));
        app.click(By::text("Barra"));
        app.click(By::text("Página"));
        assert_eq!(app.app().outer.bumps, 2);
        assert_eq!(app.app().shared.log, ["outer", "outer"]);
        assert_eq!(app.app().page_clicks, 1);
        // Both texts are on screen (`get_by_text` panics when one is not).
        app.screen().get_by_text("barra Página 2");
        app.screen().get_by_text("página 1");
    }
}
