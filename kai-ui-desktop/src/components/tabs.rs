//! Tab view (`NSTabView`): a row of tabs over the panel of the selected
//! one.

use super::{KAI_CONTRAST_BORDER, themed};
use crate::element::{Element, Role, div, text};

/// Tabs over the panel of the selected one, as `NSTabView` (ARIA tabs with
/// automatic activation). The tab row is one Tab stop that enters at the
/// selected tab; ← and → move the selection, sending `on_select(index)` as
/// a click does ([`Element::tabs_focus_group`]); Tab goes on into the
/// panel. Only the selected panel is in the tree.
///
/// AccessKit sees a [`Role::TabList`] of [`Role::Tab`]s, the selected one
/// `selected`, and a [`Role::TabPanel`] named after it. Kai draws the tabs
/// on a muted track with the selected one raised; AppKit centers them
/// over a bordered box, as the system does.
///
/// Parts: `tab.<index>` for each tab and `panel`.
pub struct Tabs<Msg> {
    selected: usize,
    on_select: Box<dyn Fn(usize) -> Msg>,
    tabs: Vec<(String, Element<Msg>)>,
}

impl<Msg: 'static> Tabs<Msg> {
    pub fn new(selected: usize, on_select: impl Fn(usize) -> Msg + 'static) -> Self {
        Self {
            selected,
            on_select: Box::new(on_select),
            tabs: Vec::new(),
        }
    }

    /// A tab titled `label` that shows `content` while selected.
    pub fn tab(mut self, label: impl Into<String>, content: Element<Msg>) -> Self {
        self.tabs.push((label.into(), content));
        self
    }

    pub fn build(self) -> Element<Msg> {
        let selected = self.selected.min(self.tabs.len().saturating_sub(1));
        let tab_class = themed(
            "flex items-center justify-center px-3 rounded-[4px] text-sm leading-5 font-medium \
             whitespace-nowrap text-muted-foreground cursor-pointer hover:text-foreground \
             selected:bg-background selected:text-foreground selected:shadow-sm \
             focus-visible:ring-2 focus-visible:ring-primary/40 \
             contrast-more:focus-visible:ring-primary",
            "flex items-center justify-center px-control border border-transparent \
             rounded-[4px] text-control whitespace-nowrap text-control-text cursor-default \
             hover:bg-fill-3 selected:border-choice selected:bg-choice/20 \
             keyboard-focus:ring-focus window-inactive:ring-0",
        );
        let mut tabs = Vec::new();
        let mut panel = None;
        for (index, (label, content)) in self.tabs.into_iter().enumerate() {
            tabs.push(
                div()
                    .class(&tab_class)
                    .role(Role::Tab)
                    .key(index.to_string())
                    .part(format!("tab.{index}"))
                    .pressed(index == selected)
                    .on_click((self.on_select)(index))
                    .child(text(label.as_str())),
            );
            if index == selected {
                panel = Some(
                    div()
                        .class(&themed(
                            "flex flex-col pt-3",
                            "flex flex-col pt-6 px-4 pb-4 rounded-[6px] border-[0.5px] \
                             border-separator bg-fill-4",
                        ))
                        .role(Role::TabPanel)
                        .label(label)
                        .part("panel")
                        .child(content),
                );
            }
        }
        let list = div()
            .class(&themed(
                &format!(
                    "inline-flex items-stretch gap-1 h-control p-0.5 rounded-md bg-muted \
                     {KAI_CONTRAST_BORDER}"
                ),
                "inline-flex items-stretch gap-0.5 h-control p-0.5 rounded-control bg-window \
                 border-[0.5px] border-separator contrast-more:border-secondary-label",
            ))
            .role(Role::TabList)
            .tabs_focus_group()
            .children(tabs);
        div()
            .class("flex flex-col")
            .parts_root()
            .child(
                div()
                    .class(&themed("flex", "relative z-10 flex justify-center -mb-3"))
                    .child(list),
            )
            .children(panel)
    }
}

#[cfg(test)]
mod tests {
    use super::Tabs;
    use crate::element::{ControlState, Element, Role, div, text};
    use crate::runtime::{App, Effects};
    use crate::testing::{By, TestApp};
    use crate::ui_engine::{Key, Modifiers};

    #[derive(Default)]
    struct Ajustes {
        tab: usize,
    }

    impl App for Ajustes {
        type Msg = usize;

        fn title(&self) -> String {
            "Ajustes".into()
        }

        fn update(&mut self, tab: usize) -> Effects<usize> {
            self.tab = tab;
            Effects::none()
        }

        fn view(&self) -> Element<usize> {
            let panel = |name: &str| {
                div().child(
                    crate::components::button(name, crate::components::ButtonVariant::Secondary)
                        .on_click(self.tab)
                        .test_id(format!("boton.{name}")),
                )
            };
            div().class("p-4").children([
                text("Ajustes").test_id("titulo"),
                Tabs::new(self.tab, |tab| tab)
                    .tab("General", panel("general"))
                    .tab("Cuenta", panel("cuenta"))
                    .tab("Avanzado", panel("avanzado"))
                    .build()
                    .test_id("ajustes"),
            ])
        }
    }

    fn tab(index: usize) -> By {
        By::test_id(format!("ajustes.tab.{index}"))
    }

    #[test]
    fn roles_names_and_the_selected_tab_reach_assistive_tech() {
        let app = TestApp::new(Ajustes { tab: 1 });
        let screen = app.screen();
        let list = screen.get(By::role(Role::TabList));
        let tabs = screen.within(list).all(By::role(Role::Tab));
        let names: Vec<_> = tabs.iter().map(|tab| tab.name()).collect();
        assert_eq!(
            names,
            [
                Some("General".to_owned()),
                Some("Cuenta".to_owned()),
                Some("Avanzado".to_owned())
            ]
        );
        let states: Vec<_> = tabs.iter().map(|tab| tab.node().control_state).collect();
        assert_eq!(
            states,
            [
                Some(ControlState::Off),
                Some(ControlState::On),
                Some(ControlState::Off)
            ]
        );
        let panels = screen.all(By::role(Role::TabPanel));
        assert_eq!(panels.len(), 1, "only the selected panel");
        assert_eq!(panels[0].name().as_deref(), Some("Cuenta"));
        assert_eq!(panels[0].test_id(), Some("ajustes.panel"));
        screen.within(panels[0]).get(By::test_id("boton.cuenta"));

        let update = app.session().a11y_tree(1.0);
        let selected = update
            .nodes
            .iter()
            .find(|(id, _)| *id == crate::a11y::node_id(tabs[1].id()))
            .map(|(_, node)| (node.role(), node.is_selected()));
        assert_eq!(selected, Some((accesskit::Role::Tab, Some(true))));
    }

    #[test]
    fn arrows_select_the_next_tab_and_tab_goes_into_the_panel() {
        let mut app = TestApp::new(Ajustes::default());
        app.click(tab(0));
        assert!(app.is_focused(tab(0)));

        app.press(Key::ArrowRight, Modifiers::NONE);
        assert_eq!(app.app().tab, 1);
        assert!(app.is_focused(tab(1)));
        app.press(Key::ArrowRight, Modifiers::NONE);
        app.press(Key::ArrowRight, Modifiers::NONE);
        assert_eq!(app.app().tab, 2, "stays at the last tab");
        app.press(Key::ArrowDown, Modifiers::NONE);
        assert_eq!(app.app().tab, 2, "↑ and ↓ do not move tabs");
        app.press(Key::ArrowLeft, Modifiers::NONE);
        assert_eq!(app.app().tab, 1);

        app.press(Key::Tab, Modifiers::NONE);
        assert!(app.is_focused(By::test_id("boton.cuenta")));
        let back = Modifiers {
            shift: true,
            ..Modifiers::NONE
        };
        app.press(Key::Tab, back);
        assert!(app.is_focused(tab(1)), "back at the selected tab");
    }
}
