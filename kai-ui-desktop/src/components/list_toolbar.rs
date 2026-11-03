//! Toolbar of a list (the footer of a list pane): read-only info on the
//! left, actions on the right and, at the right end, a search that
//! unfolds over the actions (docs/componentes/list-toolbar.md).

use std::time::Duration;

use super::{
    IconButtonSize, IconButtonVariant, SearchVariant, icon_button_sized, pane_footer, search_field,
};
use crate::element::{ControlSize, Element, div, text};
use crate::icons::Icon;
use crate::responder::Action;
use crate::runtime::Effects;

/// One action of a [`ListToolbar`]: a compact icon button (28 pt) with
/// its name as tooltip, a menu when it has options, and a dot when its
/// setting is on (an active filter or order).
pub struct ListAction<Msg> {
    icon: Icon,
    label: String,
    on_click: Option<Msg>,
    menu: Option<(
        Vec<String>,
        Option<usize>,
        std::rc::Rc<dyn Fn(usize) -> Msg>,
    )>,
    active: bool,
    test_id: Option<String>,
}

impl<Msg: Clone + 'static> ListAction<Msg> {
    /// A button that sends `msg`.
    pub fn button(icon: Icon, label: impl Into<String>, msg: Msg) -> Self {
        Self {
            icon,
            label: label.into(),
            on_click: Some(msg),
            menu: None,
            active: false,
            test_id: None,
        }
    }

    /// A button that opens a menu of `items` with `selected` checked;
    /// picking sends `on_select(index)`.
    pub fn menu(
        icon: Icon,
        label: impl Into<String>,
        items: &[&str],
        selected: Option<usize>,
        on_select: impl Fn(usize) -> Msg + 'static,
    ) -> Self {
        Self {
            icon,
            label: label.into(),
            on_click: None,
            menu: Some((
                items.iter().map(|item| (*item).to_owned()).collect(),
                selected,
                std::rc::Rc::new(on_select),
            )),
            active: false,
            test_id: None,
        }
    }

    /// Its setting is on: a dot over the icon (and «activo» in its name).
    pub fn active(mut self, active: bool) -> Self {
        self.active = active;
        self
    }

    pub fn test_id(mut self, test_id: impl Into<String>) -> Self {
        self.test_id = Some(test_id.into());
        self
    }

    fn build(self) -> Element<Msg> {
        let label = if self.active {
            format!("{} (activo)", self.label)
        } else {
            self.label.clone()
        };
        let mut button = icon_button_sized(
            self.icon,
            label,
            IconButtonVariant::Neutral,
            IconButtonSize::Sm,
        )
        .tooltip(self.label.as_str());
        if let Some(msg) = self.on_click {
            button = button.on_click(msg);
        }
        if let Some((items, selected, on_select)) = self.menu {
            button = button
                .options(items, selected)
                .on_select(move |index| on_select(index));
        }
        button.test_id = self.test_id;
        let mut wrap = div().class("relative inline-flex shrink-0").child(button);
        if self.active {
            wrap =
                wrap.child(div().class("absolute top-1 right-1 size-1.5 rounded-full bg-choice"));
        }
        wrap
    }
}

/// How far the search of a [`ListToolbar`] is open, animated from the
/// app's `update` with timers ([`Effects::after`]): `open`/`close` start
/// it and every `tick` moves it one frame until it rests.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct SearchReveal {
    /// 0 closed (only the icon), 1 open (the whole field).
    pub progress: f32,
    opening: bool,
}

impl SearchReveal {
    /// Time to open or close.
    pub const DURATION: Duration = Duration::from_millis(180);
    /// One frame of the animation.
    pub const FRAME: Duration = Duration::from_millis(16);
    const KEY: &'static str = "kai.list_toolbar.reveal";

    /// Whether the search shows (opening, open or closing).
    pub fn is_open(&self) -> bool {
        self.opening || self.progress > 0.0
    }

    /// Whether it is (or is going) open.
    pub fn is_opening(&self) -> bool {
        self.opening
    }

    pub fn open<Msg: Send + 'static>(&mut self, tick: Msg) -> Effects<Msg> {
        self.opening = true;
        self.step(tick)
    }

    pub fn close<Msg: Send + 'static>(&mut self, tick: Msg) -> Effects<Msg> {
        self.opening = false;
        self.step(tick)
    }

    /// One frame; schedules the next until it rests.
    pub fn tick<Msg: Send + 'static>(&mut self, tick: Msg) -> Effects<Msg> {
        let delta = Self::FRAME.as_secs_f32() / Self::DURATION.as_secs_f32();
        self.progress = if self.opening {
            (self.progress + delta).min(1.0)
        } else {
            (self.progress - delta).max(0.0)
        };
        self.step(tick)
    }

    fn step<Msg: Send + 'static>(&self, tick: Msg) -> Effects<Msg> {
        let done = if self.opening {
            self.progress >= 1.0
        } else {
            self.progress <= 0.0
        };
        if done {
            Effects::none()
        } else {
            Effects::after(Self::FRAME, tick).keyed(Self::KEY)
        }
    }

    /// Eased progress (ease-out cubic).
    pub fn eased(&self) -> f32 {
        let t = self.progress.clamp(0.0, 1.0);
        1.0 - (1.0 - t).powi(3)
    }
}

/// Toolbar of a list: `info` on the left (read only: «8 clientes», «3 de
/// 8 clientes»), the actions on the right and a search button at the
/// right end. While the search is open ([`SearchReveal`]) the field grows
/// from that button to the left over the actions, which hide; the info
/// stays. It takes the focus on opening. A single X (the field's clear)
/// empties it; Escape first clears the text and then closes, and leaving
/// it empty closes it too.
///
/// Bar of 44 pt with 28 pt buttons and a 28 pt field, so hover and the
/// focus ring keep 8 pt of margin inside (rules in
/// `docs/componentes/list-toolbar.md`). Parts: `<id>.info`,
/// `<id>.search` (the button) and `<id>.field`.
pub struct ListToolbar<Msg> {
    info: String,
    actions: Vec<ListAction<Msg>>,
    search: Option<Search<Msg>>,
    test_id: Option<String>,
}

struct Search<Msg> {
    value: String,
    placeholder: String,
    reveal: SearchReveal,
    on_input: std::rc::Rc<dyn Fn(String) -> Msg>,
    on_open: Msg,
    on_close: Msg,
}

/// Widest the search grows to, in points.
pub const LIST_SEARCH_WIDTH: f32 = 190.0;
const BUTTON: f32 = 28.0;

pub fn list_toolbar<Msg>(info: impl Into<String>) -> ListToolbar<Msg> {
    ListToolbar {
        info: info.into(),
        actions: Vec::new(),
        search: None,
        test_id: None,
    }
}

impl<Msg: Clone + 'static> ListToolbar<Msg> {
    pub fn action(mut self, action: ListAction<Msg>) -> Self {
        self.actions.push(action);
        self
    }

    /// The expandable search: its text, how open it is, each edit, and
    /// the messages that open and close it.
    pub fn search(
        mut self,
        value: impl Into<String>,
        reveal: SearchReveal,
        on_input: impl Fn(String) -> Msg + 'static,
        on_open: Msg,
        on_close: Msg,
    ) -> Self {
        self.search = Some(Search {
            value: value.into(),
            placeholder: "Buscar".into(),
            reveal,
            on_input: std::rc::Rc::new(on_input),
            on_open,
            on_close,
        });
        self
    }

    pub fn placeholder(mut self, placeholder: impl Into<String>) -> Self {
        if let Some(search) = &mut self.search {
            search.placeholder = placeholder.into();
        }
        self
    }

    pub fn test_id(mut self, test_id: impl Into<String>) -> Self {
        self.test_id = Some(test_id.into());
        self
    }

    pub fn build(self) -> Element<Msg> {
        let id = self.test_id.clone().unwrap_or_default();
        let part = |name: &str| format!("{id}.{name}");
        let open = self.search.as_ref().is_some_and(|s| s.reveal.is_open());
        let mut items = vec![
            text(self.info.clone())
                .class("flex-1 min-w-0 truncate whitespace-nowrap")
                .test_id(part("info")),
        ];
        if !open {
            items.extend(self.actions.into_iter().map(ListAction::build));
        }
        if let Some(search) = self.search {
            if open {
                // 3 pt of padding (cancelled by the margin) so the clip that
                // animates the width does not cut the 2 pt focus ring.
                let width = 6.0 + BUTTON + (LIST_SEARCH_WIDTH - BUTTON) * search.reveal.eased();
                let close = search.on_close.clone();
                let empty = search.value.is_empty();
                let input = search.on_input.clone();
                let mut field = search_field(search.value.clone(), move |value| input(value))
                    .placeholder(search.placeholder.clone())
                    .variant(SearchVariant::Inline)
                    .control_size(ControlSize::Small)
                    .autofocus(search.reveal.is_opening())
                    .test_id(part("field"));
                if empty {
                    field = field.on_blur(close.clone());
                }
                items.push(
                    div()
                        .class(&format!(
                            "flex items-center shrink-0 overflow-hidden p-[3px] -m-[3px] w-[{width:.0}px]"
                        ))
                        .claims_escape(true)
                        .on_action(move |action| {
                            (action == Action::CancelOperation).then(|| close.clone())
                        })
                        // The field's own X clears; Escape on an empty field
                        // or leaving it empty closes. No second button.
                        .child(
                            div()
                                .class("flex-1 min-w-0")
                                .child(field.build().class("h-[28px]")),
                        ),
                );
            } else {
                items.push(
                    icon_button_sized(
                        Icon::Search,
                        "Buscar",
                        IconButtonVariant::Neutral,
                        IconButtonSize::Sm,
                    )
                    .tooltip("Buscar")
                    .on_click(search.on_open.clone())
                    .test_id(part("search")),
                );
            }
        }
        let mut bar = pane_footer(items);
        bar.test_id = self.test_id;
        bar
    }
}

#[cfg(test)]
mod tests {
    use super::{ListAction, SearchReveal, list_toolbar};
    use crate::element::Element;
    use crate::icons::Icon;
    use crate::runtime::SimpleApp;
    use crate::testing::{By, Key, Modifiers, TestApp};

    #[test]
    fn the_reveal_opens_and_closes_in_frames_and_rests() {
        let mut reveal = SearchReveal::default();
        assert!(!reveal.is_open());
        assert!(!reveal.open(()).is_none(), "schedules a frame");
        assert!(reveal.is_open());
        let mut frames = 0;
        while !reveal.tick(()).is_none() {
            frames += 1;
            assert!(frames < 100);
        }
        assert_eq!(reveal.progress, 1.0);
        assert!(frames >= 5, "it moves over several frames");
        assert_eq!(reveal.eased(), 1.0);
        reveal.close(());
        while !reveal.tick(()).is_none() {}
        assert!(!reveal.is_open());
    }

    #[derive(Default)]
    struct Window {
        query: String,
        reveal: SearchReveal,
        added: u32,
    }

    #[derive(Clone, Debug)]
    enum Msg {
        Query(String),
        Open,
        Close,
        Add,
    }

    impl SimpleApp for Window {
        type Msg = Msg;
        fn title(&self) -> String {
            "Ventana".into()
        }
        fn update(&mut self, msg: Msg) {
            // Without timers here: open and close at once.
            match msg {
                Msg::Query(query) => self.query = query,
                Msg::Open => self.reveal.progress = 1.0,
                Msg::Close => {
                    self.query.clear();
                    self.reveal = SearchReveal::default();
                }
                Msg::Add => self.added += 1,
            }
        }
        fn view(&self) -> Element<Msg> {
            let mut reveal = self.reveal;
            if reveal.progress > 0.0 {
                let _ = reveal.open(());
            }
            list_toolbar(format!("{} filas", self.query.len()))
                .action(
                    ListAction::button(Icon::Plus, "Añadir", Msg::Add)
                        .active(true)
                        .test_id("add"),
                )
                .search(
                    self.query.clone(),
                    reveal,
                    Msg::Query,
                    Msg::Open,
                    Msg::Close,
                )
                .test_id("bar")
                .build()
        }
    }

    #[test]
    fn the_search_takes_the_focus_hides_the_actions_and_escape_closes_it() {
        let mut app = TestApp::new(Window::default());
        assert_eq!(
            app.screen().get(By::test_id("add")).name().as_deref(),
            Some("Añadir (activo)")
        );
        app.click(By::test_id("add"));
        assert_eq!(app.app().added, 1);
        app.click(By::test_id("bar.search"));
        assert!(app.is_focused(By::test_id("bar.field.input")), "autofocus");
        assert!(app.screen().query(By::test_id("add")).is_none());
        app.screen().get(By::test_id("bar.info"));
        // A single X: the field's clear; no close button.
        assert!(app.screen().query(By::test_id("bar.close")).is_none());
        app.type_text(By::test_id("bar.field.input"), "ana");
        app.screen().get(By::test_id("bar.field.clear"));
        // The first Escape clears, the second closes.
        app.press(Key::Escape, Modifiers::NONE);
        assert_eq!(app.value(By::test_id("bar.field.input")), "");
        assert!(app.screen().query(By::test_id("bar.search")).is_none());
        app.press(Key::Escape, Modifiers::NONE);
        app.screen().get(By::test_id("bar.search"));
        app.screen().get(By::test_id("add"));
        assert_eq!(app.unknown_classes(), Vec::<&str>::new());
    }
}
