//! Route-driven navigation between the parts of a section, for the center
//! of a top bar (docs/dev/28 §3.3): a pill segmented control in Glass
//! (grey capsule track, the active option tinted with the choice color and
//! a thin outline of it), the usual segmented control elsewhere.

use super::themed;
use crate::element::{Element, Role, div, text};

/// The tab whose path `path` is in: the longest `prefix` that equals
/// `path` or is followed by `/` in it. `None` when no tab matches.
pub fn section_tab_index(tabs: &[(&str, &str)], path: &str) -> Option<usize> {
    tabs.iter()
        .enumerate()
        .filter(|(_, (_, prefix))| {
            path == *prefix
                || path
                    .strip_prefix(prefix)
                    .is_some_and(|rest| rest.starts_with('/') || prefix.ends_with('/'))
        })
        .max_by_key(|(_, (_, prefix))| prefix.len())
        .map(|(index, _)| index)
}

/// Tabs `(label, path)` with the one of `path` selected
/// ([`section_tab_index`]); a click or Space sends `on_select(index)`. One
/// Tab stop whose tabs the arrows walk. Name it with `.label(..)`; parts
/// by index (`<id>.0`, `<id>.1`…).
pub fn section_tabs<Msg>(
    tabs: &[(&str, &str)],
    path: &str,
    on_select: impl Fn(usize) -> Msg,
) -> Element<Msg> {
    let selected = section_tab_index(tabs, path);
    let tab_class = themed(
        "flex items-center justify-center px-3 border border-transparent rounded-[4px] text-sm \
         leading-5 font-medium whitespace-nowrap text-muted-foreground cursor-pointer \
         hover:text-foreground selected:border-choice selected:bg-choice/20 \
         selected:text-foreground focus-visible:ring-2 focus-visible:ring-primary/40",
        "flex items-center justify-center px-3 border border-transparent rounded-[4px] \
         glass:rounded-full glass:px-3.5 text-control whitespace-nowrap text-control-text \
         cursor-default hover:bg-fill-3 selected:border-choice/70 selected:bg-choice/15 \
         keyboard-focus:ring-focus window-inactive:ring-0",
    );
    div()
        .class(&themed(
            "inline-flex items-stretch gap-0.5 h-control p-0.5 rounded-md bg-neutral border \
             border-border",
            "inline-flex items-stretch gap-0.5 h-[26px] p-0.5 rounded-control glass:rounded-full \
             bg-fill-2",
        ))
        .role(Role::RadioGroup)
        .focus_group()
        .parts_root()
        .children(tabs.iter().enumerate().map(|(index, (label, _))| {
            div()
                .class(&tab_class)
                .role(Role::Radio)
                .key(index.to_string())
                .part(index.to_string())
                .pressed(Some(index) == selected)
                .on_click(on_select(index))
                .child(text(*label))
        }))
}

#[cfg(test)]
mod tests {
    use super::{section_tab_index, section_tabs};
    use crate::element::{Element, Role};
    use crate::runtime::SimpleApp;
    use crate::testing::{By, TestApp};

    const TABS: [(&str, &str); 3] = [
        ("Lista", "/real-app"),
        ("Clientes", "/real-app/clients"),
        ("Mapa", "/real-app/map"),
    ];

    #[test]
    fn the_route_picks_the_longest_matching_tab() {
        assert_eq!(section_tab_index(&TABS, "/real-app"), Some(0));
        assert_eq!(section_tab_index(&TABS, "/real-app/clients"), Some(1));
        assert_eq!(section_tab_index(&TABS, "/real-app/clients/42"), Some(1));
        assert_eq!(section_tab_index(&TABS, "/real-app/map"), Some(2));
        assert_eq!(section_tab_index(&TABS, "/real-app/other"), Some(0));
        assert_eq!(section_tab_index(&TABS, "/real-applet"), None);
        assert_eq!(section_tab_index(&TABS, "/home"), None);
    }

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
            self.path = TABS[index].1.to_owned();
        }
        fn view(&self) -> Element<Msg> {
            section_tabs(&TABS, &self.path, Msg::Go)
                .label("Partes")
                .test_id("tabs")
        }
    }

    fn on<A: crate::runtime::App>(app: &mut TestApp<A>, id: &str) -> bool {
        app.screen().get(By::test_id(id)).node().control_state
            == Some(crate::element::ControlState::On)
    }

    #[test]
    fn clicking_a_tab_selects_it_through_the_route() {
        let mut app = TestApp::new(Window {
            path: "/real-app/map/7".into(),
        });
        assert!(on(&mut app, "tabs.2"));
        assert!(!on(&mut app, "tabs.0"));
        app.click(By::role(Role::Radio).name("Clientes"));
        assert_eq!(app.app().path, "/real-app/clients");
        assert!(on(&mut app, "tabs.1"));
    }
}
