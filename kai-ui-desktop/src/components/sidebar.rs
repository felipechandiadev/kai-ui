//! Sidebar of sections (a source list), expanded or collapsed to a rail
//! of icons (12 Paso 20.7).

use super::themed;
use crate::element::{Element, Role, div, icon, text};
use crate::icons::Icon;

/// Row of a [`sidebar`]: a title, an optional icon before it and an
/// optional badge (a count) after it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SidebarItem {
    pub label: String,
    pub icon: Option<Icon>,
    pub badge: Option<String>,
}

impl SidebarItem {
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            icon: None,
            badge: None,
        }
    }

    pub fn icon(mut self, icon: Icon) -> Self {
        self.icon = Some(icon);
        self
    }

    pub fn badge(mut self, badge: impl Into<String>) -> Self {
        self.badge = Some(badge.into());
        self
    }
}

/// Rows of a [`sidebar`], under a title or not.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SidebarGroup {
    pub title: Option<String>,
    pub items: Vec<SidebarItem>,
}

impl SidebarGroup {
    pub fn titled(title: impl Into<String>, items: impl IntoIterator<Item = SidebarItem>) -> Self {
        Self {
            title: Some(title.into()),
            items: items.into_iter().collect(),
        }
    }

    pub fn untitled(items: impl IntoIterator<Item = SidebarItem>) -> Self {
        Self {
            title: None,
            items: items.into_iter().collect(),
        }
    }
}

/// Sidebar of sections (a source list): one row per item, `selected`
/// counted across groups in order. It is one Tab stop that enters at the
/// selected row, and ↑ and ↓ move the selection, sending
/// `on_select(index)` as a click does ([`Element::selecting_focus_group`]).
/// The selected row is tinted glass like the info
/// [`alert`](super::alert) (a translucent `info` fill and border), and
/// hovering a row previews a fainter version of it.
///
/// [`Sidebar::collapsed`] turns it into a rail of centered icons 48 pt
/// wide (`w-12`): group titles become separators, a badge becomes a dot
/// on the icon and each row shows its title in a tooltip; names, the
/// selection and the hover stay. With [`Sidebar::on_toggle`] a button at
/// the bottom (`<id>.alternar`, outside the Tab stop of the rows) sends
/// that message.
///
/// Expanded, give it a width with `.class(..)` on the built element.
/// Parts: one per row, by index (`<id>.0`…).
pub struct Sidebar<Msg> {
    groups: Vec<SidebarGroup>,
    selected: usize,
    /// `on_select(index)` of every row, in order.
    messages: Vec<Msg>,
    collapsed: bool,
    on_toggle: Option<Msg>,
    label: Option<String>,
    test_id: Option<String>,
}

impl<Msg> Sidebar<Msg> {
    pub fn new(groups: &[SidebarGroup], selected: usize, on_select: impl Fn(usize) -> Msg) -> Self {
        let rows = groups.iter().map(|group| group.items.len()).sum();
        Self {
            groups: groups.to_vec(),
            selected,
            messages: (0..rows).map(on_select).collect(),
            collapsed: false,
            on_toggle: None,
            label: None,
            test_id: None,
        }
    }

    pub fn collapsed(mut self, collapsed: bool) -> Self {
        self.collapsed = collapsed;
        self
    }

    /// Message of the collapse button at the bottom.
    pub fn on_toggle(mut self, msg: Msg) -> Self {
        self.on_toggle = Some(msg);
        self
    }

    /// Accessible name of the list of sections.
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    pub fn test_id(mut self, test_id: impl Into<String>) -> Self {
        self.test_id = Some(test_id.into());
        self
    }

    pub fn build(self) -> Element<Msg> {
        let collapsed = self.collapsed;
        let mut messages = self.messages.into_iter();
        let mut index = 0;
        let mut sections = Vec::new();
        for (position, group) in self.groups.iter().enumerate() {
            let mut children = Vec::new();
            match (&group.title, collapsed) {
                (Some(_), true) if position > 0 => children.push(
                    div()
                        .class(&themed(
                            "w-6 h-px mb-1 self-center bg-border",
                            "w-5 h-px mb-1 self-center bg-separator",
                        ))
                        .role(Role::Generic),
                ),
                (Some(title), false) => children.push(
                    text(title.as_str())
                        .class(&themed(
                            "px-3 pb-1 text-xs font-medium text-muted-foreground",
                            "px-2 pb-0.5 text-footnote font-semibold text-secondary-label",
                        ))
                        .role(Role::Heading),
                ),
                _ => {}
            }
            for item in &group.items {
                let Some(msg) = messages.next() else {
                    break;
                };
                let selected = index == self.selected;
                children.push(if collapsed {
                    rail_row(item, index, selected, msg)
                } else {
                    sidebar_row(item, index, selected, msg)
                });
                index += 1;
            }
            sections.push(
                div()
                    .class(if collapsed {
                        "flex flex-col items-center gap-1"
                    } else {
                        "flex flex-col gap-px"
                    })
                    .children(children),
            );
        }
        let layout = match collapsed {
            true => themed(
                "group flex flex-col items-center gap-2 w-12 shrink-0 py-3 overflow-y-auto",
                "group flex flex-col items-center gap-1.5 w-12 shrink-0 py-2 overflow-y-auto",
            ),
            false => themed(
                "group flex flex-col gap-4 p-3 overflow-y-auto",
                "group flex flex-col gap-3 px-2.5 py-2 overflow-y-auto",
            ),
        };
        let mut list = div()
            .class(&layout)
            .role(Role::Sidebar)
            .selecting_focus_group()
            .parts_root()
            .children(sections);
        if let Some(label) = self.label {
            list = list.label(label);
        }
        let Some(toggle) = self.on_toggle else {
            list.test_id = self.test_id;
            return list;
        };
        let title = if collapsed {
            "Mostrar barra lateral"
        } else {
            "Ocultar barra lateral"
        };
        let mut button = div()
            .class(&themed(
                "inline-flex size-8 shrink-0 items-center justify-center rounded-md \
                 text-muted-foreground cursor-pointer hover:bg-info/10 hover:text-foreground \
                 focus-visible:ring-2 focus-visible:ring-primary/40",
                "inline-flex size-7 shrink-0 items-center justify-center rounded-[5px] \
                 text-secondary-label cursor-default hover:bg-fill",
            ))
            .role(Role::Button)
            .label(title)
            .tooltip(title)
            .on_click(toggle)
            .child(icon(Icon::PanelLeft).class("size-4"));
        if let Some(id) = &self.test_id {
            button = button.test_id(format!("{id}.alternar"));
        }
        list.test_id = self.test_id;
        div()
            .class(if collapsed {
                "flex flex-col items-center min-h-0"
            } else {
                "flex flex-col min-h-0"
            })
            .children([
                list.class("flex-1 min-h-0"),
                div()
                    .class(&themed("flex px-3 py-2", "flex px-2.5 py-1.5"))
                    .class(if collapsed { "justify-center" } else { "" })
                    .child(button),
            ])
    }
}

/// [`Sidebar`] expanded, without a collapse button.
pub fn sidebar<Msg>(
    groups: &[SidebarGroup],
    selected: usize,
    on_select: impl Fn(usize) -> Msg,
) -> Element<Msg> {
    Sidebar::new(groups, selected, on_select).build()
}

/// Classes of a row: its look (selected or not), the icon color and the
/// badge color.
fn row_look(selected: bool) -> (String, String, String) {
    // Every row has a border so the selected one does not shift its content.
    if selected {
        (
            themed(
                "border-info bg-info/20 hover:bg-info/25 text-foreground font-medium",
                "border-info bg-info/20 hover:bg-info/25 text-label glass:font-semibold",
            ),
            "text-info".to_owned(),
            "text-info".to_owned(),
        )
    } else {
        (
            themed(
                "border-transparent text-foreground hover:bg-info/10 hover:border-info/40",
                "border-transparent text-label hover:bg-info/10 hover:border-info/40",
            ),
            themed("text-muted-foreground", "text-control-accent"),
            themed("text-muted-foreground", "text-secondary-label"),
        )
    }
}

fn sidebar_row<Msg>(item: &SidebarItem, index: usize, selected: bool, msg: Msg) -> Element<Msg> {
    let (row, glyph, badge) = row_look(selected);
    let mut children = Vec::new();
    if let Some(glyph_icon) = item.icon {
        children.push(icon(glyph_icon).class(&format!("size-4 shrink-0 {glyph}")));
    }
    children.push(text(item.label.as_str()).class("flex-1 truncate"));
    if let Some(count) = &item.badge {
        children.push(
            text(count.as_str()).class(&format!("{badge} {}", themed("text-xs", "text-footnote"))),
        );
    }
    div()
        .class(&format!(
            "{} {row}",
            themed(
                "flex items-center gap-2 h-9 px-3 border rounded-md text-sm cursor-pointer \
                 focus-visible:ring-2 focus-visible:ring-primary/40",
                "flex items-center gap-1.5 h-7 px-2 border rounded-[5px] text-control cursor-default",
            )
        ))
        .role(Role::SidebarItem)
        .key(index.to_string())
        .part(index.to_string())
        .label(item.label.as_str())
        .pressed(selected)
        .on_click(msg)
        .children(children)
}

/// A row of the rail: the icon centered (or the title's first letter),
/// the badge as a dot, the title in a tooltip.
fn rail_row<Msg>(item: &SidebarItem, index: usize, selected: bool, msg: Msg) -> Element<Msg> {
    let (row, glyph, _) = row_look(selected);
    let mut children = vec![match item.icon {
        Some(glyph_icon) => icon(glyph_icon).class(&format!("size-4 shrink-0 {glyph}")),
        None => text(item.label.chars().take(1).collect::<String>())
            .class(&format!("text-xs font-semibold {glyph}")),
    }];
    if item.badge.is_some() {
        children.push(
            div()
                .class("absolute top-1 right-1 size-2 rounded-full bg-info")
                .part(format!("{index}.punto")),
        );
    }
    let mut row = div()
        .class(&format!(
            "{} {row}",
            themed(
                "relative flex items-center justify-center size-9 border rounded-md \
                 cursor-pointer focus-visible:ring-2 focus-visible:ring-primary/40",
                "relative flex items-center justify-center size-8 border rounded-[6px] \
                 cursor-default",
            )
        ))
        .role(Role::SidebarItem)
        .key(index.to_string())
        .part(index.to_string())
        .label(item.label.as_str())
        .tooltip(item.label.as_str())
        .pressed(selected)
        .on_click(msg)
        .children(children);
    if let Some(badge) = &item.badge {
        row = row.description(badge.as_str());
    }
    row
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;
    use crate::element::TOOLTIP_DELAY;
    use crate::runtime::SimpleApp;
    use crate::session::TOOLTIP;
    use crate::testing::{By, TestApp};
    use crate::ui_engine::{Key, Modifiers};

    #[derive(Default)]
    struct Nav {
        selected: usize,
        collapsed: bool,
    }

    #[derive(Clone, Debug, PartialEq)]
    enum Msg {
        Select(usize),
        Toggle,
    }

    impl SimpleApp for Nav {
        type Msg = Msg;

        fn title(&self) -> String {
            "Nav".to_owned()
        }

        fn update(&mut self, msg: Msg) {
            match msg {
                Msg::Select(index) => self.selected = index,
                Msg::Toggle => self.collapsed = !self.collapsed,
            }
        }

        fn view(&self) -> Element<Msg> {
            let groups = [
                SidebarGroup::titled(
                    "Demo",
                    [SidebarItem::new("Inicio").icon(Icon::House).badge("3")],
                ),
                SidebarGroup::titled(
                    "Componentes",
                    [
                        SidebarItem::new("Campos").icon(Icon::TextCursorInput),
                        SidebarItem::new("Botones"),
                    ],
                ),
            ];
            div()
                .class("flex w-full h-full")
                .test_id("ventana")
                .children([
                    Sidebar::new(&groups, self.selected, Msg::Select)
                        .collapsed(self.collapsed)
                        .on_toggle(Msg::Toggle)
                        .label("Secciones")
                        .test_id("nav")
                        .build(),
                    div().class("flex-1 h-full").test_id("contenido"),
                ])
        }
    }

    fn collapsed() -> TestApp<Nav> {
        TestApp::new(Nav {
            selected: 1,
            collapsed: true,
        })
    }

    fn row(name: &str) -> By {
        By::role(Role::SidebarItem).name(name)
    }

    fn tooltip(app: &TestApp<Nav>) -> Option<String> {
        app.screen()
            .query(By::test_id(TOOLTIP))
            .and_then(|found| found.name())
    }

    #[test]
    fn the_rail_keeps_names_and_selection_and_turns_badges_into_dots() {
        let app = collapsed();
        let screen = app.screen();
        let nav = screen.get_by_test_id("nav");
        assert_eq!(nav.role(), Role::Sidebar);
        assert_eq!(nav.name().as_deref(), Some("Secciones"));
        assert_eq!(nav.bounds().width, 48.0, "w-12");
        let rows = screen.within(nav).all(By::role(Role::SidebarItem));
        let names: Vec<_> = rows.iter().filter_map(|row| row.name()).collect();
        assert_eq!(names, ["Inicio", "Campos", "Botones"]);
        let inicio = screen.get(row("Inicio"));
        assert_eq!(inicio.node().description.as_deref(), Some("3"));
        screen.get_by_test_id("nav.0.punto");
        assert!(
            screen.within(nav).query(By::role(Role::Heading)).is_none(),
            "group titles become separators"
        );
        assert!(screen.within(nav).query(By::text("Inicio")).is_none());
        assert_eq!(
            app.screen().get(row("Campos")).node().control_state,
            Some(crate::element::ControlState::On)
        );
    }

    #[test]
    fn the_toggle_switches_between_rail_and_list_outside_the_rows_stop() {
        let mut app = TestApp::new(Nav::default());
        let toggle = By::test_id("nav.alternar");
        assert_eq!(
            app.screen().get(toggle.clone()).name().as_deref(),
            Some("Ocultar barra lateral")
        );
        app.screen()
            .get(By::role(Role::Heading).name("Componentes"));
        app.click(toggle.clone());
        assert_eq!(app.messages(), [Msg::Toggle]);
        assert!(app.app().collapsed);
        assert_eq!(
            app.screen().get(toggle.clone()).name().as_deref(),
            Some("Mostrar barra lateral")
        );
        assert_eq!(app.screen().get_by_test_id("nav").bounds().width, 48.0);

        app.press(Key::Tab, Modifiers::NONE);
        assert!(app.is_focused(row("Inicio")));
        app.press(Key::ArrowDown, Modifiers::NONE);
        assert_eq!(app.app().selected, 1, "arrows still select in the rail");
        app.press(Key::Tab, Modifiers::NONE);
        assert!(app.is_focused(toggle));
    }

    #[test]
    fn the_tooltip_shows_after_the_delay_beside_the_rail() {
        let mut app = collapsed();
        app.hover(row("Campos"));
        assert_eq!(tooltip(&app), None);
        app.advance(TOOLTIP_DELAY - Duration::from_millis(1));
        assert_eq!(tooltip(&app), None, "not before the delay");
        app.advance(Duration::from_millis(1));
        assert_eq!(tooltip(&app).as_deref(), Some("Campos"));
        let anchor = app.screen().get(row("Campos")).bounds();
        let bubble = app.screen().get_by_test_id(TOOLTIP).bounds();
        assert!(
            bubble.x >= anchor.x + anchor.width,
            "to the right of the rail"
        );
        assert_eq!(app.screen().get_by_test_id(TOOLTIP).role(), Role::Tooltip);

        app.hover(row("Botones"));
        assert_eq!(
            tooltip(&app).as_deref(),
            Some("Botones"),
            "at once while another one shows"
        );
        app.hover(By::test_id("contenido"));
        assert_eq!(tooltip(&app), None, "hidden on leave");
        app.advance(TOOLTIP_DELAY * 2);
        assert_eq!(tooltip(&app), None);
    }

    #[test]
    fn a_press_hides_the_tooltip_until_the_pointer_leaves() {
        let mut app = collapsed();
        app.hover(row("Inicio"));
        app.advance(TOOLTIP_DELAY);
        assert_eq!(tooltip(&app).as_deref(), Some("Inicio"));
        app.click(row("Inicio"));
        assert_eq!(app.messages(), [Msg::Select(0)]);
        assert_eq!(tooltip(&app), None);
        app.advance(TOOLTIP_DELAY * 2);
        assert_eq!(tooltip(&app), None, "not again while it stays");

        app.hover(By::test_id("contenido"));
        app.hover(row("Inicio"));
        app.advance(TOOLTIP_DELAY);
        assert_eq!(tooltip(&app).as_deref(), Some("Inicio"));
    }

    /// `cargo test sidebar_capture -- --ignored` writes
    /// `target/snapshots/sidebar-<theme>-<expandida|riel>.png` (needs a
    /// GPU); the rail one shows a tooltip.
    #[test]
    #[ignore]
    fn sidebar_capture() {
        use crate::theme::{AppKitGeneration, Appearance, Density, Theme};
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("target/snapshots");
        std::fs::create_dir_all(&dir).unwrap();
        let themes = [
            ("kai", Theme::kai(Density::Comfortable)),
            (
                "appkit",
                Theme::appkit_static(AppKitGeneration::Glass27, Appearance::Light),
            ),
        ];
        for (name, theme) in themes {
            for collapsed in [false, true] {
                let mut app = TestApp::builder(Nav {
                    selected: 1,
                    collapsed,
                })
                .theme(theme)
                .viewport(360.0, 240.0)
                .build();
                if collapsed {
                    app.hover(row("Campos"));
                    app.advance(TOOLTIP_DELAY);
                }
                let image = app.capture(By::test_id("ventana")).unwrap();
                let shape = if collapsed { "riel" } else { "expandida" };
                let path = dir.join(format!("sidebar-{name}-{shape}.png"));
                crate::snapshot::write_png(&path, &image).unwrap();
            }
        }
    }

    #[test]
    fn the_tooltip_takes_no_pointer_and_no_focus() {
        let mut app = collapsed();
        app.hover(row("Campos"));
        app.advance(TOOLTIP_DELAY);
        let bubble = app.screen().get_by_test_id(TOOLTIP).bounds();
        let center = crate::geometry::Point {
            x: bubble.x + bubble.width / 2.0,
            y: bubble.y + bubble.height / 2.0,
        };
        app.click_at(center);
        assert_eq!(app.messages(), [], "the click goes through to the content");
        app.press(Key::Tab, Modifiers::NONE);
        assert!(app.is_focused(row("Campos")));
    }
}
