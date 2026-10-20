//! Window toolbar drawn by Kai (`NSToolbar`, 12 Paso 20.8).

use super::themed;
use crate::element::{Element, Role, div, icon, text};
use crate::icons::Icon;

/// Bar across the top of the window: 52 pt in Kai, 38 pt in AppKit (a
/// unified title bar's toolbar). Its buttons are one Tab stop and ←/→ move
/// between them ([`Element::focus_group`]); a field inside
/// ([`toolbar_search`]) is a stop of its own, since arrows edit its text.
/// Put [`toolbar_item`]s, [`toolbar_title`], [`toolbar_separator`] and
/// [`toolbar_spacer`] in it, in order.
pub fn toolbar<Msg>(items: impl IntoIterator<Item = Element<Msg>>) -> Element<Msg> {
    div()
        .class(&themed(
            "flex items-center gap-1 h-[52px] shrink-0 px-3 bg-background \
             border-b border-border/70 shadow-md z-10",
            "flex items-center gap-1 h-[38px] shrink-0 px-2 bg-window \
             border-b border-separator z-10",
        ))
        .role(Role::Toolbar)
        .focus_group()
        .children(items)
}

/// Toolbar button: an icon without a border that shows one under the
/// pointer, named `label` for accessibility and showing it as its tooltip.
/// Chain `.on_click(..)` or `.on_command(..)`, and `.pressed(..)` for a
/// toggle.
pub fn toolbar_item<Msg>(glyph: Icon, label: impl Into<String>) -> Element<Msg> {
    let label = label.into();
    div()
        .class(&themed(
            "inline-flex items-center justify-center shrink-0 size-9 rounded-md border \
             border-transparent text-secondary cursor-pointer hover:border-border \
             hover:bg-primary/10 active:bg-primary/20 selected:border-choice \
             selected:bg-choice/20 selected:text-foreground focus-visible:ring-2 focus-visible:ring-primary/40 \
             disabled:opacity-50 disabled:cursor-not-allowed",
            "inline-flex items-center justify-center shrink-0 h-[28px] w-[32px] rounded-md \
             border-[0.5px] border-transparent text-secondary-label cursor-default \
             hover:border-separator hover:bg-fill-3 active:bg-fill-2 selected:border-choice \
             selected:bg-choice/20 selected:text-label keyboard-focus:ring-focus window-inactive:ring-0 \
             window-inactive:text-tertiary-label disabled:text-disabled-control-text",
        ))
        .role(Role::Button)
        .tooltip(label.as_str())
        .label(label)
        .child(icon(glyph).class(&themed("size-5", "size-4")))
}

/// Title (and optional subtitle) of the window in the toolbar.
pub fn toolbar_title<Msg>(title: impl Into<String>, subtitle: Option<&str>) -> Element<Msg> {
    let mut children = vec![text(title.into()).class(&themed(
        "text-lg font-semibold text-primary whitespace-nowrap",
        "text-control font-semibold text-label whitespace-nowrap",
    ))];
    if let Some(subtitle) = subtitle {
        children.push(text(subtitle).class(&themed(
            "text-xs text-muted-foreground whitespace-nowrap",
            "text-footnote text-secondary-label whitespace-nowrap",
        )));
    }
    div()
        .class(&themed(
            "flex items-baseline gap-2 min-w-0 px-2",
            "flex items-baseline gap-1.5 min-w-0 px-1.5",
        ))
        .role(Role::Heading)
        .children(children)
}

/// Thin vertical line between groups of items.
pub fn toolbar_separator<Msg>() -> Element<Msg> {
    div().class(&themed(
        "w-px h-6 mx-1 shrink-0 bg-border",
        "w-px h-[18px] mx-1 shrink-0 bg-separator",
    ))
}

/// Flexible space: pushes what follows to the trailing end.
pub fn toolbar_spacer<Msg>() -> Element<Msg> {
    div().class("flex-1 min-w-2")
}

/// Slot for a search field (or any field): a fixed width at the end of
/// the toolbar, its own Tab stop.
pub fn toolbar_search<Msg>(field: Element<Msg>) -> Element<Msg> {
    div()
        .class(&themed("w-56 shrink min-w-0", "w-48 shrink min-w-0"))
        .focus_group()
        .child(field)
}

#[cfg(test)]
mod tests {
    use super::{toolbar, toolbar_item, toolbar_search, toolbar_separator, toolbar_spacer};
    use crate::components::{search_field, toolbar_title};
    use crate::element::{Element, Role};
    use crate::icons::Icon;
    use crate::runtime::SimpleApp;
    use crate::testing::{By, TestApp};
    use crate::ui_engine::{Key, Modifiers};

    #[derive(Default)]
    struct Window {
        query: String,
        sidebar: bool,
        added: u32,
    }

    #[derive(Clone, Debug, PartialEq)]
    enum Msg {
        Query(String),
        Sidebar,
        Add,
        Share,
    }

    impl SimpleApp for Window {
        type Msg = Msg;
        fn title(&self) -> String {
            "Ventana".into()
        }
        fn update(&mut self, msg: Msg) {
            match msg {
                Msg::Query(query) => self.query = query,
                Msg::Sidebar => self.sidebar = !self.sidebar,
                Msg::Add => self.added += 1,
                Msg::Share => {}
            }
        }
        fn view(&self) -> Element<Msg> {
            toolbar([
                toolbar_item(Icon::PanelLeft, "Barra lateral")
                    .pressed(self.sidebar)
                    .on_click(Msg::Sidebar)
                    .test_id("barra.sidebar"),
                toolbar_title("Ventana", Some("v1")),
                toolbar_spacer(),
                toolbar_item(Icon::Plus, "Agregar")
                    .on_click(Msg::Add)
                    .test_id("barra.agregar"),
                toolbar_separator(),
                toolbar_item(Icon::Settings, "Compartir")
                    .on_click(Msg::Share)
                    .test_id("barra.compartir"),
                toolbar_search(
                    search_field(&self.query, Msg::Query)
                        .test_id("barra.buscar")
                        .build(),
                ),
            ])
        }
    }

    #[test]
    fn items_are_one_tab_stop_walked_with_arrows_and_the_search_is_another() {
        let mut app = TestApp::new(Window::default());
        app.screen().get(By::role(Role::Toolbar));
        app.press(Key::Tab, Modifiers::NONE);
        assert!(app.is_focused(By::test_id("barra.sidebar")));
        app.press(Key::ArrowRight, Modifiers::NONE);
        assert!(app.is_focused(By::test_id("barra.agregar")));
        app.press(Key::Space, Modifiers::NONE);
        assert_eq!(app.app().added, 1);
        app.press(Key::ArrowRight, Modifiers::NONE);
        app.press(Key::ArrowRight, Modifiers::NONE);
        assert!(
            app.is_focused(By::test_id("barra.compartir")),
            "stays at the end"
        );

        app.press(Key::Tab, Modifiers::NONE);
        assert!(app.is_focused(By::test_id("barra.buscar")));
        app.type_text(By::test_id("barra.buscar"), "ab");
        app.press(Key::ArrowLeft, Modifiers::NONE);
        assert!(app.is_focused(By::test_id("barra.buscar")), "arrows edit");
        app.press(Key::Tab, Modifiers::NONE);
        assert!(
            app.is_focused(By::test_id("barra.buscar.clear")),
            "the clear button is a stop of its own"
        );
        app.press(Key::Tab, Modifiers::NONE);
        assert!(
            app.is_focused(By::test_id("barra.sidebar")),
            "Tab enters the items at the first one"
        );
    }

    #[test]
    fn a_toggle_item_reports_its_state() {
        let mut app = TestApp::new(Window::default());
        app.click(By::test_id("barra.sidebar"));
        assert!(app.app().sidebar);
        let item = app.screen().get(By::test_id("barra.sidebar"));
        assert_eq!(item.role(), Role::Button);
        assert_eq!(item.name().as_deref(), Some("Barra lateral"));
        assert_eq!(
            item.node().control_state,
            Some(crate::element::ControlState::On)
        );
    }

    #[test]
    fn items_show_their_name_as_a_tooltip() {
        let mut app = TestApp::new(Window::default());
        app.hover(By::test_id("barra.agregar"));
        app.advance(crate::element::TOOLTIP_DELAY);
        let tooltip = app.screen().get_by_test_id(crate::session::TOOLTIP);
        let item = app.screen().get_by_test_id("barra.agregar").bounds();
        assert_eq!(
            tooltip.name(),
            app.screen().get_by_test_id("barra.agregar").name()
        );
        assert!(tooltip.bounds().y >= item.y + item.height, "below the item");
    }
}
