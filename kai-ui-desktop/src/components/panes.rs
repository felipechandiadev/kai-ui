//! Fixed panes side by side over the window background, and a floating
//! sidebar for them (the Glass layout of an app: docs/dev/28 §3.3).
//! Panes are solid (content); the sidebar is glass (navigation), as the
//! HIG asks. Not resizable: `PaneGroup` is for later.

use super::{section_tab_index, themed};
use crate::element::{Element, Role, div, icon, text};
use crate::icons::Icon;

/// Row of [`pane`]s with a gap between them, filling what is left. Give
/// each pane its width with `.class("w-[300px] shrink-0")` or
/// `.class("flex-1 min-w-0")`.
pub fn panes<Msg>(children: impl IntoIterator<Item = Element<Msg>>) -> Element<Msg> {
    div()
        .class("flex flex-row items-stretch gap-3 flex-1 min-w-0 min-h-0 p-3")
        .children(children)
}

/// A solid, rounded panel of content: opaque window color, hairline
/// border, a soft shadow; its children stack.
pub fn pane<Msg>() -> Element<Msg> {
    div()
        .class(&themed(
            "flex flex-col min-h-0 bg-background border border-border/70 rounded-xl shadow-sm \
             overflow-hidden",
            "flex flex-col min-h-0 bg-window border-[0.5px] border-separator rounded-[8px] \
             glass:rounded-[14px] shadow-sm overflow-hidden",
        ))
        .role(Role::Group)
}

/// Thin row at the bottom of a [`pane`]: a count, small actions. Its
/// height (40 pt AppKit, 44 Kai) holds a small icon button (28 pt,
/// `IconButtonSize::Sm`) with 6 pt above and below, so the hover fill
/// stays inside the row.
pub fn pane_footer<Msg>(children: impl IntoIterator<Item = Element<Msg>>) -> Element<Msg> {
    div()
        .class(&themed(
            "flex items-center gap-1 h-11 shrink-0 px-3 border-t border-border/70 text-xs \
             text-muted-foreground",
            "flex items-center gap-1 h-[40px] shrink-0 px-2.5 border-t-[0.5px] border-separator \
             text-caption1 text-secondary-label",
        ))
        .children(children)
}

/// Floating glass sidebar of an app: rows `(icon, label, path)`, the one
/// of `path` selected by route ([`section_tab_index`]); a row with an
/// empty path is shown disabled (not there yet). A click sends
/// `on_select(index)`. Name it with `.label(..)`; rows are parts by index.
pub fn app_sidebar<Msg>(
    rows: &[(Icon, &str, &str)],
    path: &str,
    on_select: impl Fn(usize) -> Msg,
) -> Element<Msg> {
    let routes: Vec<(&str, &str)> = rows
        .iter()
        .filter(|(_, _, route)| !route.is_empty())
        .map(|(_, label, route)| (*label, *route))
        .collect();
    let selected = section_tab_index(&routes, path).map(|index| routes[index].0);
    let row_class = themed(
        "flex items-center gap-2.5 h-9 px-3 rounded-md text-sm text-foreground cursor-pointer \
         hover:bg-muted/40 selected:bg-choice/15 disabled:opacity-50",
        "flex items-center gap-2.5 h-[34px] px-3 rounded-[8px] glass:rounded-[10px] text-control \
         text-label cursor-default hover:bg-fill-3 selected:bg-choice/20 \
         disabled:text-disabled-control-text",
    );
    div()
        .class(&themed(
            "flex flex-col gap-1 w-48 shrink-0 my-3 ml-3 p-2 rounded-xl bg-background/80 border \
             border-border/70 shadow-md",
            "flex flex-col gap-0.5 w-[172px] shrink-0 my-3 ml-3 p-2 rounded-[8px] \
             glass:rounded-[16px] bg-window/60 border-[0.5px] border-separator shadow-md",
        ))
        .role(Role::Group)
        .focus_group()
        .parts_root()
        .children(
            rows.iter()
                .enumerate()
                .map(|(index, (glyph, label, route))| {
                    div()
                        .class(&row_class)
                        .role(Role::Button)
                        .key(index.to_string())
                        .part(index.to_string())
                        .label(*label)
                        .pressed(Some(*label) == selected && !route.is_empty())
                        .disabled(route.is_empty())
                        .on_click(on_select(index))
                        .children([icon(*glyph).class("size-4"), text(*label)])
                }),
        )
}

#[cfg(test)]
mod tests {
    use super::{app_sidebar, pane, panes};
    use crate::element::{ControlState, Element};
    use crate::icons::Icon;
    use crate::runtime::SimpleApp;
    use crate::testing::{By, TestApp};

    const ROWS: [(Icon, &str, &str); 3] = [
        (Icon::LayoutDashboard, "Resumen", "/app"),
        (Icon::Users, "Clientes", "/app/clients"),
        (Icon::Layers, "Laboratorio", ""),
    ];

    struct Window {
        path: String,
    }

    #[derive(Clone, Debug)]
    enum Msg {
        Go(usize),
    }

    impl SimpleApp for Window {
        type Msg = Msg;
        fn title(&self) -> String {
            "Ventana".into()
        }
        fn update(&mut self, msg: Msg) {
            let Msg::Go(index) = msg;
            self.path = ROWS[index].2.to_owned();
        }
        fn view(&self) -> Element<Msg> {
            panes([
                app_sidebar(&ROWS, &self.path, Msg::Go)
                    .label("App")
                    .test_id("nav"),
                pane().class("flex-1").test_id("content"),
            ])
        }
    }

    fn on<A: crate::runtime::App>(app: &mut TestApp<A>, id: &str) -> bool {
        app.screen().get(By::test_id(id)).node().control_state == Some(ControlState::On)
    }

    #[test]
    fn the_route_selects_a_row_and_rows_without_one_are_disabled() {
        let mut app = TestApp::new(Window {
            path: "/app/clients/3".into(),
        });
        assert!(on(&mut app, "nav.1"));
        assert!(!on(&mut app, "nav.0"));
        assert!(!app.is_enabled(By::test_id("nav.2")));
        app.click(By::test_id("nav.0"));
        assert_eq!(app.app().path, "/app");
        assert!(on(&mut app, "nav.0"));
        app.screen().get(By::test_id("content"));
        assert_eq!(app.unknown_classes(), Vec::<&str>::new());
    }
}
