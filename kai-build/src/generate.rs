//! Writes the app root from an [`AppTree`]: modules, `Route`, `AppRoot`,
//! `RootMsg` and `run` (docs/dev/26 §5.2). The output is plain, formatted
//! Rust that reads like the root of `kai-ui-desktop/src/session/pages_tests.rs`.

use std::fmt::Write;
use std::path::Path;

use crate::scan::{AppTree, LayoutNode, PageNode, Section, Segment};

/// `sales_report` → `SalesReport`.
fn camel(name: &str) -> String {
    name.split('_')
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                Some(first) => first.to_ascii_uppercase().to_string() + chars.as_str(),
                None => String::new(),
            }
        })
        .collect()
}

/// Variant of `Route` for a page: `customers/[id]` → `CustomersId`.
fn variant(page: &PageNode) -> String {
    page.segments.iter().map(|s| camel(s.name())).collect()
}

/// Field of `AppRoot` that holds a live page: `page_customers_id`.
fn slot(page: &PageNode) -> String {
    let names: Vec<&str> = page.segments.iter().map(Segment::name).collect();
    format!("page_{}", names.join("_"))
}

/// Module name of a folder: `[id]` → `param_id`.
fn module(segment: &Segment) -> String {
    match segment {
        Segment::Static(name) => name.clone(),
        Segment::Param(name) => format!("param_{name}"),
    }
}

/// Path of the page's `Screen` type.
fn screen(page: &PageNode) -> String {
    let modules: Vec<String> = page.segments.iter().map(module).collect();
    format!("crate::sections::{}::page::Screen", modules.join("::"))
}

/// URL segment: `sales_report` → `sales-report`.
fn kebab(name: &str) -> String {
    name.replace('_', "-")
}

fn rust_str(path: &Path) -> String {
    format!("{:?}", path.to_string_lossy())
}

/// All pages, section by section.
fn pages(tree: &AppTree) -> Vec<(usize, &PageNode)> {
    tree.sections
        .iter()
        .enumerate()
        .flat_map(|(index, section)| section.pages.iter().map(move |page| (index, page)))
        .collect()
}

/// The page whose route is `segments` without its last folder(s): the
/// nearest ancestor with a `page.rs`.
fn parent<'a>(section: &'a Section, page: &PageNode) -> Option<&'a PageNode> {
    (1..page.segments.len()).rev().find_map(|len| {
        section
            .pages
            .iter()
            .find(|other| other.segments[..] == page.segments[..len])
    })
}

/// `Route::X { id }` or `Route::X`, binding the parameter as `value`.
fn pattern(page: &PageNode, binding: &str) -> String {
    match page.param() {
        Some(param) => format!("Route::{} {{ {param}: {binding} }}", variant(page)),
        None => format!("Route::{}", variant(page)),
    }
}

/// The page's parameters as an expression, from a binding of the route's
/// parameter named `value`.
fn params(page: &PageNode, binding: &str) -> String {
    match page.param() {
        Some(_) => format!("{binding}.clone()"),
        None => "()".into(),
    }
}

/// A layout of the app and the routes it frames (docs/dev/28).
struct LayoutPlan {
    /// `root`, `customers`, `customers_new`.
    name: String,
    /// `Root`, `Customers`, `CustomersNew`.
    variant: String,
    /// Path of its `Frame` type.
    frame: String,
    /// Routes it frames, as a pattern; `None` for the root layout, which
    /// frames every route.
    scope: Option<String>,
    /// Folders down from `src/sections`; the root layout is 0.
    depth: usize,
}

impl LayoutPlan {
    /// Field of `AppRoot` that holds the live layout.
    fn slot(&self) -> String {
        format!("layout_{}", self.name)
    }

    /// `matches!(route, ..) && `, or nothing when it frames every route.
    fn condition(&self) -> String {
        match &self.scope {
            Some(pattern) => format!("matches!(self.router.current(), {pattern}) && "),
            None => String::new(),
        }
    }
}

/// The root layout first, then each section's, outermost first.
fn layouts(tree: &AppTree) -> Vec<LayoutPlan> {
    let mut plans = Vec::new();
    if tree.root_layout.is_some() {
        plans.push(LayoutPlan {
            name: "root".into(),
            variant: "Root".into(),
            frame: "crate::layout::Frame".into(),
            scope: None,
            depth: 0,
        });
    }
    for section in &tree.sections {
        for layout in &section.layouts {
            let names: Vec<&str> = layout.segments.iter().map(Segment::name).collect();
            let modules: Vec<String> = layout.segments.iter().map(module).collect();
            let scope: Vec<String> = section
                .pages
                .iter()
                .filter(|page| page.segments.starts_with(&layout.segments))
                .map(|page| pattern(page, "_"))
                .collect();
            plans.push(LayoutPlan {
                name: names.join("_"),
                variant: layout.segments.iter().map(|s| camel(s.name())).collect(),
                frame: format!("crate::sections::{}::layout::Frame", modules.join("::")),
                scope: Some(scope.join(" | ")),
                depth: layout.segments.len(),
            });
        }
    }
    plans
}

pub fn generate(tree: &AppTree) -> String {
    let mut out = String::new();
    let all = pages(tree);
    let plans = layouts(tree);
    let w = &mut out;

    w.push_str("// @generated by kai-build from src/sections: do not edit.\n\n");

    // --- modules ---------------------------------------------------------
    writeln!(
        w,
        "#[path = {}]\npub mod state;\n",
        rust_str(&tree.state_file)
    )
    .unwrap();
    if let Some(file) = &tree.root_layout {
        writeln!(w, "#[path = {}]\npub mod layout;\n", rust_str(file)).unwrap();
    }
    w.push_str("pub mod sections {\n");
    for section in &tree.sections {
        writeln!(w, "    pub mod {} {{", section.name).unwrap();
        writeln!(w, "        #[path = {}]", rust_str(&section.section_file)).unwrap();
        w.push_str("        pub mod section;\n");
        write_page_modules(w, &section.pages[..], &section.layouts[..], 1, 2);
        w.push_str("    }\n");
    }
    w.push_str("}\n\n");

    // The parameter's type is the page's: it may or may not be `Copy`.
    w.push_str("#[allow(clippy::clone_on_copy, clippy::match_single_binding)]\npub mod app {\n");
    w.push_str("    use kai::element::{Element, div};\n");
    w.push_str("    use kai::router::{Context, NavState, Navigable, Navigate, Page, Router, SectionMeta, SectionSidebar};\n");
    if !plans.is_empty() {
        w.push_str("    use kai::router::{Layout, RouteInfo};\n");
    }
    w.push_str("    use kai::runtime::{App, Effects};\n\n");

    // --- Route -----------------------------------------------------------
    w.push_str("    /// Every page of the app, by its folder in src/sections.\n");
    w.push_str("    #[derive(Clone, Debug, PartialEq, Eq)]\n    pub enum Route {\n");
    for (_, page) in &all {
        match page.param() {
            Some(param) => writeln!(
                w,
                "        {} {{ {param}: <{} as Page>::Params }},",
                variant(page),
                screen(page)
            )
            .unwrap(),
            None => writeln!(w, "        {},", variant(page)).unwrap(),
        }
    }
    w.push_str("    }\n\n");

    w.push_str("    impl kai::router::Route for Route {\n");
    w.push_str("        fn parse(path: &str) -> Option<Self> {\n");
    w.push_str(
        "            let segments: Vec<&str> = path.trim_matches('/').split('/').collect();\n",
    );
    w.push_str("            match segments.as_slice() {\n");
    // Fixed paths before parameters, so `customers/new` wins over `[id]`.
    let mut ordered = all.clone();
    ordered.sort_by_key(|(_, page)| page.param().is_some());
    for (_, page) in &ordered {
        let parts: Vec<String> = page
            .segments
            .iter()
            .map(|segment| match segment {
                Segment::Static(name) => format!("{:?}", kebab(name)),
                Segment::Param(name) => name.clone(),
            })
            .collect();
        let slice = format!("[{}]", parts.join(", "));
        match page.param() {
            Some(param) => writeln!(
                w,
                "                {slice} => {param}.parse().ok().map(|{param}| Route::{} {{ {param} }}),",
                variant(page)
            )
            .unwrap(),
            None => writeln!(w, "                {slice} => Some(Route::{}),", variant(page)).unwrap(),
        }
    }
    w.push_str("                _ => None,\n            }\n        }\n\n");

    w.push_str("        fn path(&self) -> String {\n            match self {\n");
    for (_, page) in &all {
        let text: Vec<String> = page
            .segments
            .iter()
            .map(|segment| match segment {
                Segment::Static(name) => kebab(name),
                Segment::Param(_) => "{value}".into(),
            })
            .collect();
        let text = format!("/{}", text.join("/"));
        match page.param() {
            Some(_) => writeln!(
                w,
                "                {} => format!({text:?}),",
                pattern(page, "value")
            )
            .unwrap(),
            None => writeln!(
                w,
                "                {} => {text:?}.into(),",
                pattern(page, "value")
            )
            .unwrap(),
        }
    }
    w.push_str("            }\n        }\n\n");

    w.push_str("        fn parent(&self) -> Option<Self> {\n            match self {\n");
    for (index, page) in &all {
        let section = &tree.sections[*index];
        let Some(up) = parent(section, page) else {
            continue;
        };
        let target = match up.param() {
            Some(param) => format!("Route::{} {{ {param}: value.clone() }}", variant(up)),
            None => format!("Route::{}", variant(up)),
        };
        let binding = if up.param().is_some() { "value" } else { "_" };
        writeln!(
            w,
            "                {} => Some({target}),",
            pattern(page, binding)
        )
        .unwrap();
    }
    w.push_str("                #[allow(unreachable_patterns)]\n                _ => None,\n            }\n        }\n    }\n\n");

    w.push_str("    impl Route {\n");
    w.push_str("        /// Index into [`SECTIONS`] of the section this route is in.\n");
    w.push_str("        pub fn section(&self) -> usize {\n            match self {\n");
    for (index, page) in &all {
        writeln!(w, "                {} => {index},", pattern(page, "_")).unwrap();
    }
    w.push_str("            }\n        }\n    }\n\n");

    // --- sections ----------------------------------------------------------
    let count = tree.sections.len();
    let metas: Vec<String> = tree
        .sections
        .iter()
        .map(|s| format!("crate::sections::{}::section::SECTION", s.name))
        .collect();
    writeln!(
        w,
        "    /// How each section shows in the sidebar, by folder name.\n    pub const SECTIONS: [SectionMeta; {count}] = [{}];\n",
        metas.join(", ")
    )
    .unwrap();
    let paths: Vec<String> = tree
        .sections
        .iter()
        .map(|s| format!("{:?}", format!("/{}", kebab(&s.name))))
        .collect();
    writeln!(
        w,
        "    /// Path of each section's own page.\n    pub const SECTION_PATHS: [&str; {count}] = [{}];\n",
        paths.join(", ")
    )
    .unwrap();

    // --- AppRoot -----------------------------------------------------------
    w.push_str("    /// The app: router, shared state and the live pages.\n");
    w.push_str("    pub struct AppRoot {\n");
    w.push_str("        pub router: Router<Route>,\n");
    w.push_str("        pub state: crate::state::State,\n");
    w.push_str("        sidebar: SectionSidebar,\n");
    w.push_str("        /// Where the last section is kept between runs; none in tests.\n");
    w.push_str("        store: Option<kai::store::Store>,\n");
    w.push_str("        sidebar_collapsed: bool,\n");
    w.push_str("        /// The toasts pages asked for, oldest first.\n");
    w.push_str("        toasts: Vec<kai::components::Toast<RootMsg>>,\n        next_toast: u64,\n");
    w.push_str("        /// The first window size has set `sidebar_collapsed`.\n");
    w.push_str("        sized: bool,\n");
    for (_, page) in &all {
        if page.is_section_root() {
            writeln!(w, "        {}: Option<{}>,", slot(page), screen(page)).unwrap();
        } else {
            writeln!(
                w,
                "        {}: Option<(<{s} as Page>::Params, {s})>,",
                slot(page),
                s = screen(page)
            )
            .unwrap();
        }
    }
    for plan in &plans {
        writeln!(w, "        {}: Option<{}>,", plan.slot(), plan.frame).unwrap();
    }
    w.push_str("    }\n\n");

    w.push_str("    #[derive(Clone, Debug)]\n    pub enum RootMsg {\n");
    w.push_str("        Navigate(Navigate),\n        Sidebar(usize),\n");
    w.push_str("        LastSectionSaved(Result<(), kai::store::StoreError>),\n");
    w.push_str("        ToggleSidebar,\n        InitialWidth { narrow: bool },\n        CloseToast(u64),\n");
    for (_, page) in &all {
        writeln!(
            w,
            "        {}(<{} as Page>::Msg),",
            variant(page),
            screen(page)
        )
        .unwrap();
    }
    for plan in &plans {
        writeln!(
            w,
            "        Layout{}(<{} as Layout>::Msg),",
            plan.variant, plan.frame
        )
        .unwrap();
    }
    w.push_str("    }\n\n");

    w.push_str("    impl AppRoot {\n");
    w.push_str("        /// Starts at the first section of the sidebar.\n");
    w.push_str("        pub fn new(state: crate::state::State) -> Self {\n");
    w.push_str("            let sidebar = SectionSidebar::new(&SECTIONS);\n");
    w.push_str("            let first = sidebar.section_at(0).unwrap_or(0);\n");
    w.push_str(
        "            let start = <Route as kai::router::Route>::parse(SECTION_PATHS[first])\n",
    );
    w.push_str("                .expect(\"the path of a section parses\");\n");
    w.push_str("            Self {\n                router: Router::new(start),\n                state,\n                sidebar,\n                store: None,\n                sidebar_collapsed: false,\n                toasts: Vec::new(),\n                next_toast: 1,\n                sized: false,\n");
    for (_, page) in &all {
        writeln!(w, "                {}: None,", slot(page)).unwrap();
    }
    for plan in &plans {
        writeln!(w, "                {}: None,", plan.slot()).unwrap();
    }
    w.push_str("            }\n        }\n\n");

    w.push_str("        /// Keep the section on screen in `store` and start at the one\n        /// saved there, if it still exists.\n");
    w.push_str("        pub fn with_store(mut self, store: kai::store::Store) -> Self {\n");
    w.push_str("            match store.load::<kai::router::LastSection>() {\n");
    w.push_str("                Ok(Some(last)) => {\n                    if let Some(route) = <Route as kai::router::Route>::parse(&last.path) {\n                        self.router = Router::new(route);\n                    }\n                }\n");
    w.push_str("                Ok(None) => {}\n");
    w.push_str("                Err(error) => eprintln!(\"Última sección ignorada ({}): {error}\", store.dir().display()),\n");
    w.push_str(
        "            }\n            self.store = Some(store);\n            self\n        }\n\n",
    );

    w.push_str("        /// Add a page's toast with the next id; its timer closes it.\n");
    w.push_str("        fn show_toast(&mut self, mut shown: kai::components::Toast<RootMsg>) -> Effects<RootMsg> {\n");
    w.push_str("            let id = self.next_toast;\n            self.next_toast += 1;\n");
    w.push_str("            shown.id = id;\n");
    w.push_str(
        "            let timer = kai::components::toast_timer(&shown, RootMsg::CloseToast(id));\n",
    );
    w.push_str("            self.toasts.push(shown);\n            timer\n        }\n\n");
    let open_page = if plans.is_empty() {
        "open_current"
    } else {
        "open_page"
    };
    if !plans.is_empty() {
        w.push_str("        /// Make sure the layouts around the current route are alive.\n");
        w.push_str("        fn open_layouts(&mut self) -> Effects<RootMsg> {\n            let mut opened = Vec::new();\n");
        for plan in &plans {
            writeln!(
                w,
                "            if {cond}self.{slot}.is_none() {{\n                let (layout, effects) = <{frame} as Layout>::init(&self.state);\n                self.{slot} = Some(layout);\n                opened.push(effects.map(RootMsg::Layout{v}));\n            }}",
                cond = plan.condition(),
                slot = plan.slot(),
                frame = plan.frame,
                v = plan.variant
            )
            .unwrap();
        }
        w.push_str("            Effects::batch(opened)\n        }\n\n");
        w.push_str(
            "        /// Make sure the layouts and the page of the current route are alive.\n",
        );
        w.push_str("        fn open_current(&mut self) -> Effects<RootMsg> {\n            Effects::batch([self.open_layouts(), self.open_page()])\n        }\n\n");
    }
    w.push_str("        /// Make sure the page of the current route is alive.\n");
    writeln!(
        w,
        "        fn {open_page}(&mut self) -> Effects<RootMsg> {{"
    )
    .unwrap();
    w.push_str("            match self.router.current().clone() {\n");
    for (_, page) in &all {
        let s = screen(page);
        if page.is_section_root() {
            writeln!(
                w,
                "                Route::{v} if self.{f}.is_none() => {{\n                    let (page, effects) = <{s} as Page>::init(&(), &self.state);\n                    self.{f} = Some(page);\n                    effects.map(RootMsg::{v})\n                }}",
                v = variant(page),
                f = slot(page)
            )
            .unwrap();
        } else {
            writeln!(
                w,
                "                {pat} => {{\n                    let params = {params};\n                    if self.{f}.as_ref().map(|(at, _)| at) == Some(&params) {{\n                        return Effects::none();\n                    }}\n                    let (page, effects) = <{s} as Page>::init(&params, &self.state);\n                    self.{f} = Some((params, page));\n                    effects.map(RootMsg::{v})\n                }}",
                pat = pattern(page, "value"),
                params = params(page, "value"),
                v = variant(page),
                f = slot(page)
            )
            .unwrap();
        }
    }
    w.push_str("                #[allow(unreachable_patterns)]\n                _ => Effects::none(),\n            }\n        }\n\n");

    w.push_str("        /// Carry out navigation, drop pages with parameters no longer\n        /// shown, and open the new page.\n");
    w.push_str("        fn navigate(&mut self, requests: Vec<Navigate>) -> Effects<RootMsg> {\n");
    w.push_str("            let section = self.router.current().section();\n");
    w.push_str("            for request in requests {\n                self.router.apply(request);\n            }\n");
    w.push_str("            let mut saved = Effects::none();\n");
    w.push_str("            if let Some(store) = &self.store\n                && self.router.current().section() != section\n            {\n");
    w.push_str("                let last = kai::router::LastSection {\n                    path: SECTION_PATHS[self.router.current().section()].into(),\n                };\n");
    w.push_str("                saved = store.save_effect(&last, RootMsg::LastSectionSaved);\n            }\n");
    for (_, page) in &all {
        if !page.is_section_root() {
            writeln!(
                w,
                "            if !matches!(self.router.current(), {}) {{\n                self.{} = None;\n            }}",
                pattern(page, "_"),
                slot(page)
            )
            .unwrap();
        }
    }
    for plan in &plans {
        if let Some(scope) = &plan.scope {
            writeln!(
                w,
                "            if !matches!(self.router.current(), {scope}) {{\n                self.{} = None;\n            }}",
                plan.slot()
            )
            .unwrap();
        }
    }
    w.push_str("            Effects::batch([self.open_current(), saved])\n        }\n    }\n\n");

    // --- App -----------------------------------------------------------------
    w.push_str("    impl App for AppRoot {\n        type Msg = RootMsg;\n\n");
    w.push_str("        fn commands(registry: &mut kai::commands::CommandRegistry<Self>) {\n            registry.navigation();\n            registry.sidebar();\n");
    for (_, page) in &all {
        writeln!(
            w,
            "            <{} as Page>::commands(registry);",
            screen(page)
        )
        .unwrap();
    }
    w.push_str("        }\n\n");
    w.push_str("        fn init(&mut self) -> Effects<RootMsg> {\n            self.open_current()\n        }\n\n");
    w.push_str("        fn route(&self) -> Option<String> {\n            Some(<Route as kai::router::Route>::path(self.router.current()))\n        }\n\n");
    w.push_str("        /// `kai dev` reopens the route that was on screen.\n");
    w.push_str("        fn dev_state(&self) -> Option<kai::serde_json::Value> {\n            Some(kai::serde_json::json!({\n                \"path\": <Route as kai::router::Route>::path(self.router.current()),\n                \"sidebar_collapsed\": self.sidebar_collapsed,\n            }))\n        }\n\n");
    w.push_str("        /// A path an older build saved and this one lacks is ignored.\n");
    w.push_str("        fn restore_dev_state(&mut self, state: kai::serde_json::Value) {\n            if let Some(route) = state\n                .get(\"path\")\n                .and_then(|path| path.as_str())\n                .and_then(<Route as kai::router::Route>::parse)\n            {\n                self.router = Router::new(route);\n            }\n            if let Some(collapsed) = state.get(\"sidebar_collapsed\").and_then(|c| c.as_bool()) {\n                self.sidebar_collapsed = collapsed;\n                // The first window size must not undo it.\n                self.sized = true;\n            }\n        }\n\n");

    w.push_str("        /// The background effect of the page on screen.\n");
    w.push_str("        fn background(&self) -> kai::background::BackgroundEffect {\n            match self.router.current() {\n");
    for (_, page) in &all {
        let get = if page.is_section_root() {
            "page"
        } else {
            "(_, page)"
        };
        writeln!(
            w,
            "                {} => self.{}.as_ref().map(|{get}| page.background(&self.state)),",
            pattern(page, "_"),
            slot(page)
        )
        .unwrap();
    }
    w.push_str("            }\n            .unwrap_or(kai::background::BackgroundEffect::NONE)\n        }\n\n");

    w.push_str("        /// The theme the page on screen asks for, if any.\n");
    w.push_str("        fn theme(&self) -> Option<kai::theme::Theme> {\n            match self.router.current() {\n");
    for (_, page) in &all {
        let get = if page.is_section_root() {
            "page"
        } else {
            "(_, page)"
        };
        writeln!(
            w,
            "                {} => self.{}.as_ref().and_then(|{get}| page.theme(&self.state)),",
            pattern(page, "_"),
            slot(page)
        )
        .unwrap();
    }
    w.push_str("            }\n        }\n\n");

    w.push_str("        /// A narrow window starts with the sidebar as a rail.\n");
    w.push_str("        fn resized(&self, size: kai::geometry::Size) -> Option<RootMsg> {\n            (!self.sized).then_some(RootMsg::InitialWidth { narrow: size.width < 640.0 })\n        }\n\n");

    w.push_str("        fn title(&self) -> String {\n            match self.router.current() {\n");
    for (_, page) in &all {
        let get = if page.is_section_root() {
            "page"
        } else {
            "(_, page)"
        };
        writeln!(
            w,
            "                {} => self.{}.as_ref().map(|{get}| page.title(&self.state)),",
            pattern(page, "_"),
            slot(page)
        )
        .unwrap();
    }
    w.push_str("            }\n            .unwrap_or_default()\n        }\n\n");

    w.push_str("        fn update(&mut self, msg: RootMsg) -> Effects<RootMsg> {\n");
    w.push_str(
        "            let mut action_toasts: Vec<kai::components::Toast<RootMsg>> = Vec::new();\n",
    );
    w.push_str("            let mut cx = Context::new(&mut self.state);\n");
    w.push_str("            let effects = match msg {\n");
    w.push_str("                RootMsg::Navigate(request) => {\n                    drop(cx);\n                    return self.navigate(vec![request]);\n                }\n");
    w.push_str("                RootMsg::LastSectionSaved(result) => {\n                    if let Err(error) = result {\n                        eprintln!(\"No se pudo guardar la última sección: {error}\");\n                    }\n                    Effects::none()\n                }\n                RootMsg::CloseToast(id) => {\n                    self.toasts.retain(|toast| toast.id != id);\n                    Effects::cancel(kai::components::toast_key(id))\n                }\n                RootMsg::ToggleSidebar => {\n                    self.sidebar_collapsed = !self.sidebar_collapsed;\n                    Effects::none()\n                }\n                RootMsg::InitialWidth { narrow } => {\n                    self.sidebar_collapsed = narrow;\n                    self.sized = true;\n                    Effects::none()\n                }\n                RootMsg::Sidebar(row) => {\n                    drop(cx);\n                    let section = self.sidebar.section_at(row).unwrap_or(0);\n                    return self.navigate(vec![Navigate::To(SECTION_PATHS[section].into())]);\n                }\n");
    for (_, page) in &all {
        let get = if page.is_section_root() {
            "page"
        } else {
            "(_, page)"
        };
        writeln!(
            w,
            "                RootMsg::{v}(msg) => match &mut self.{f} {{\n                    Some({get}) => {{\n                        let effects = page.update(msg, &mut cx).map(RootMsg::{v});\n                        let taken = cx.take_action_toasts::<<{s} as Page>::Msg>();\n                        action_toasts.extend(taken.into_iter().map(|toast| toast.map(RootMsg::{v})));\n                        effects\n                    }}\n                    None => Effects::none(),\n                }},",
            v = variant(page),
            f = slot(page),
            s = screen(page)
        )
        .unwrap();
    }
    for plan in &plans {
        writeln!(
            w,
            "                RootMsg::Layout{v}(msg) => match &mut self.{f} {{\n                    Some(layout) => {{\n                        let effects = layout.update(msg, &mut cx).map(RootMsg::Layout{v});\n                        let taken = cx.take_action_toasts::<<{frame} as Layout>::Msg>();\n                        action_toasts.extend(taken.into_iter().map(|toast| toast.map(RootMsg::Layout{v})));\n                        effects\n                    }}\n                    None => Effects::none(),\n                }},",
            v = plan.variant,
            f = plan.slot(),
            frame = plan.frame
        )
        .unwrap();
    }
    w.push_str("            };\n            let (requests, toasts) = cx.into_parts();\n");
    w.push_str("            let plain = toasts.into_iter().map(|toast| toast.map(|never| match never {}));\n");
    w.push_str("            let shown: Vec<Effects<RootMsg>> = plain.chain(action_toasts).map(|toast| self.show_toast(toast)).collect();\n");
    w.push_str("            Effects::batch([effects, self.navigate(requests), Effects::batch(shown)])\n        }\n\n");

    writeln!(
        w,
        "        fn view(&self) -> Element<RootMsg> {{\n            let {}page = match self.router.current() {{",
        if plans.is_empty() { "" } else { "mut " }
    )
    .unwrap();
    for (_, page) in &all {
        let get = if page.is_section_root() {
            "page"
        } else {
            "(_, page)"
        };
        writeln!(
            w,
            "                {} => self.{}.as_ref().map(|{get}| page.view(&self.state).map(RootMsg::{})),",
            pattern(page, "_"),
            slot(page),
            variant(page)
        )
        .unwrap();
    }
    w.push_str("            }\n            .unwrap_or_else(div);\n");
    w.push_str("            let overlay = match self.router.current() {\n");
    for (_, page) in &all {
        let get = if page.is_section_root() {
            "page"
        } else {
            "(_, page)"
        };
        writeln!(
            w,
            "                {} => self.{}.as_ref().and_then(|{get}| page.overlay(&self.state)).map(|layer| layer.map(RootMsg::{})),",
            pattern(page, "_"),
            slot(page),
            variant(page)
        )
        .unwrap();
    }
    w.push_str("            };\n");
    if !plans.is_empty() {
        w.push_str("            let path = <Route as kai::router::Route>::path(self.router.current());\n            let title = self.title();\n            let info = RouteInfo {\n                path: &path,\n                title: &title,\n                can_back: self.router.can_go_back(),\n                can_forward: self.router.can_go_forward(),\n                sidebar_collapsed: self.sidebar_collapsed,\n            };\n");
        w.push_str("            // Innermost layout first: each one wraps what is inside it.\n");
        let mut inner_first: Vec<&LayoutPlan> = plans.iter().collect();
        inner_first.sort_by_key(|plan| std::cmp::Reverse(plan.depth));
        for plan in inner_first {
            writeln!(
                w,
                "            if {cond}let Some(layout) = &self.{slot} {{\n                page = layout\n                    .view(&self.state, &info)\n                    .map(RootMsg::Layout{v})\n                    .fill_outlet(page);\n            }}",
                cond = plan.condition(),
                slot = plan.slot(),
                v = plan.variant
            )
            .unwrap();
        }
    }
    w.push_str("            let selected = self.sidebar.row_of(self.router.current().section()).unwrap_or(0);\n");
    w.push_str("            div().class(\"relative flex flex-row w-full h-full\").children([\n");
    w.push_str("                kai::components::Sidebar::new(&self.sidebar.groups, selected, RootMsg::Sidebar)\n");
    w.push_str("                    .collapsed(self.sidebar_collapsed)\n                    .on_toggle(RootMsg::ToggleSidebar)\n                    .label(\"Secciones\")\n                    .test_id(\"sections\")\n                    .build()\n                    .class(if self.sidebar_collapsed { \"\" } else { \"w-56 shrink-0\" }),\n");
    w.push_str(
        "                div().class(\"flex-1 min-w-0 min-h-0 flex flex-col\").child(page),\n",
    );
    w.push_str("                kai::router::announcer(&self.title()),\n");
    w.push_str(
        "                kai::components::toast_stack(&self.toasts, RootMsg::CloseToast),\n",
    );
    w.push_str("            ])\n            .children(overlay)\n        }\n    }\n\n");

    for (_, page) in &all {
        let get = if page.is_section_root() {
            ".as_ref()"
        } else {
            ".as_ref().map(|(_, page)| page)"
        };
        writeln!(
            w,
            "    impl kai::router::HasPage<{s}> for AppRoot {{\n        fn page(&self) -> Option<&{s}> {{\n            self.{f}{get}\n        }}\n\n        fn page_state(&self) -> &crate::state::State {{\n            &self.state\n        }}\n\n        fn page_msg(msg: <{s} as Page>::Msg) -> RootMsg {{\n            RootMsg::{v}(msg)\n        }}\n    }}\n",
            s = screen(page),
            f = slot(page),
            v = variant(page)
        )
        .unwrap();
    }
    w.push_str("    impl kai::router::CollapsibleSidebar for AppRoot {\n        fn toggle_sidebar() -> RootMsg {\n            RootMsg::ToggleSidebar\n        }\n    }\n\n");
    w.push_str("    impl Navigable for AppRoot {\n");
    w.push_str(
        "        fn nav_state(&self) -> NavState {\n            self.router.state()\n        }\n\n",
    );
    w.push_str("        fn has_route(path: &str) -> bool {\n            <Route as kai::router::Route>::parse(path).is_some()\n        }\n\n");
    w.push_str("        fn navigate(request: Navigate) -> RootMsg {\n            RootMsg::Navigate(request)\n        }\n    }\n\n");

    w.push_str("    /// The app on `state`, for tests (`TestApp::new(app::app(state))`).\n");
    w.push_str("    pub fn app(state: crate::state::State) -> AppRoot {\n        AppRoot::new(state)\n    }\n\n");
    w.push_str("    /// Open the window and run the app until it is closed.\n");
    w.push_str("    pub fn run(\n        state: crate::state::State,\n        theme: kai::theme::Theme,\n    ) -> Result<(), Box<dyn std::error::Error>> {\n");
    w.push_str("        // The package name identifies the app's data folder.\n");
    w.push_str("        let root = match kai::store::Store::for_app(env!(\"CARGO_PKG_NAME\")) {\n            Ok(store) => AppRoot::new(state).with_store(store),\n            Err(error) => {\n                eprintln!(\"Sin carpeta de datos: {error}\");\n                AppRoot::new(state)\n            }\n        };\n");
    w.push_str("        kai::runtime::run(root, theme)\n    }\n}\n");

    out
}

/// `pub mod page;` for the section's page and nested folders for the rest.
fn write_page_modules(
    w: &mut String,
    pages: &[PageNode],
    layouts: &[LayoutNode],
    depth: usize,
    indent: usize,
) {
    let pad = "    ".repeat(indent);
    if let Some(page) = pages.iter().find(|p| p.segments.len() == depth) {
        writeln!(w, "{pad}#[path = {}]", rust_str(&page.file)).unwrap();
        writeln!(w, "{pad}pub mod page;").unwrap();
    }
    if let Some(layout) = layouts.iter().find(|l| l.segments.len() == depth) {
        writeln!(w, "{pad}#[path = {}]", rust_str(&layout.file)).unwrap();
        writeln!(w, "{pad}pub mod layout;").unwrap();
    }
    let mut children: Vec<&Segment> = pages
        .iter()
        .filter(|p| p.segments.len() > depth)
        .map(|p| &p.segments[depth])
        .collect();
    let mut seen = Vec::new();
    children.retain(|child| {
        let new = !seen.contains(child);
        seen.push(*child);
        new
    });
    for child in children {
        let below: Vec<PageNode> = pages
            .iter()
            .filter(|p| p.segments.len() > depth && &p.segments[depth] == child)
            .cloned()
            .collect();
        let below_layouts: Vec<LayoutNode> = layouts
            .iter()
            .filter(|l| l.segments.len() > depth && &l.segments[depth] == child)
            .cloned()
            .collect();
        writeln!(w, "{pad}pub mod {} {{", module(child)).unwrap();
        write_page_modules(w, &below, &below_layouts, depth + 1, indent + 1);
        writeln!(w, "{pad}}}").unwrap();
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::generate;
    use crate::scan::{AppTree, LayoutNode, PageNode, Section, Segment};

    fn tree(with_layouts: bool) -> AppTree {
        let customers = Segment::Static("customers".into());
        let page = |segments: Vec<Segment>| PageNode {
            segments,
            file: PathBuf::from("page.rs"),
        };
        AppTree {
            state_file: PathBuf::from("state.rs"),
            root_layout: with_layouts.then(|| PathBuf::from("layout.rs")),
            sections: vec![Section {
                name: "customers".into(),
                section_file: PathBuf::from("section.rs"),
                pages: vec![
                    page(vec![customers.clone()]),
                    page(vec![customers.clone(), Segment::Param("id".into())]),
                ],
                layouts: if with_layouts {
                    vec![LayoutNode {
                        segments: vec![customers],
                        file: PathBuf::from("customers_layout.rs"),
                    }]
                } else {
                    Vec::new()
                },
            }],
        }
    }

    #[test]
    fn an_app_without_layouts_generates_none_of_their_code() {
        let code = generate(&tree(false));
        assert!(!code.contains("Layout"), "{code}");
        assert!(!code.contains("fill_outlet"));
        assert!(code.contains("fn open_current(&mut self)"));
    }

    #[test]
    fn layouts_get_a_slot_a_message_a_lifecycle_and_a_place_in_the_view() {
        let code = generate(&tree(true));
        assert!(code.contains("#[path = \"layout.rs\"]\npub mod layout;"));
        assert!(code.contains("#[path = \"customers_layout.rs\"]"));
        assert!(code.contains("layout_root: Option<crate::layout::Frame>"));
        assert!(
            code.contains("layout_customers: Option<crate::sections::customers::layout::Frame>")
        );
        assert!(code.contains("LayoutRoot(<crate::layout::Frame as Layout>::Msg)"));
        assert!(code.contains("fn open_layouts(&mut self)"));
        assert!(code.contains("fn open_page(&mut self)"));
        // The section's layout is dropped when the route leaves its pages.
        assert!(code.contains(
            "if !matches!(self.router.current(), Route::Customers | Route::CustomersId { id: _ }) {\n                self.layout_customers = None;"
        ));
        // Innermost wraps first, so the root layout ends up outside.
        let customers = code
            .find("RootMsg::LayoutCustomers)\n                    .fill_outlet")
            .unwrap();
        let root = code
            .find("RootMsg::LayoutRoot)\n                    .fill_outlet")
            .unwrap();
        assert!(customers < root);
    }
}
