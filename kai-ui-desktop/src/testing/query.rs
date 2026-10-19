//! Queries over a computed [`Layout`]. Roles and accessible names come from
//! the same functions as the AccessKit tree, so a test finds an element
//! exactly as VoiceOver names it.

use std::fmt;

use crate::a11y::{accessible_name, effective_role};
use crate::element::{Role, WidgetId};
use crate::geometry::Rect;
use crate::layout::{Content, Layout, LayoutNode};

/// Candidates listed when a query fails.
const MAX_CANDIDATES: usize = 10;

#[derive(Clone, Debug)]
enum Name {
    Exact(String),
    Contains(String),
}

/// Filter for [`Screen`] queries; every condition given must hold.
/// Hidden (`display: none`) elements never match unless
/// [`By::include_hidden`].
#[derive(Clone, Debug, Default)]
pub struct By {
    role: Option<Role>,
    name: Option<Name>,
    label: Option<String>,
    text: Option<String>,
    test_id: Option<String>,
    include_hidden: bool,
}

impl By {
    /// Elements with this role, explicit or inferred (clickable elements
    /// are buttons, inputs text inputs, text labels).
    pub fn role(role: Role) -> Self {
        Self {
            role: Some(role),
            ..Self::default()
        }
    }

    /// Inputs whose accessible name (label, or placeholder) is `label`.
    pub fn label(label: impl Into<String>) -> Self {
        Self {
            label: Some(label.into()),
            ..Self::default()
        }
    }

    /// Text elements whose content is exactly `text`.
    pub fn text(text: impl Into<String>) -> Self {
        Self {
            text: Some(text.into()),
            ..Self::default()
        }
    }

    /// The element with this `test_id`.
    pub fn test_id(test_id: impl Into<String>) -> Self {
        Self {
            test_id: Some(test_id.into()),
            ..Self::default()
        }
    }

    /// Accessible name exactly `name`.
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(Name::Exact(name.into()));
        self
    }

    /// Accessible name containing `part`.
    pub fn name_contains(mut self, part: impl Into<String>) -> Self {
        self.name = Some(Name::Contains(part.into()));
        self
    }

    /// Also match hidden elements.
    pub fn include_hidden(mut self) -> Self {
        self.include_hidden = true;
        self
    }

    fn matches(&self, layout: &Layout, index: usize) -> bool {
        let node = layout.get(index);
        if node.hidden && !self.include_hidden {
            return false;
        }
        if self.role.is_some_and(|role| effective_role(node) != role) {
            return false;
        }
        if self
            .test_id
            .as_ref()
            .is_some_and(|test_id| node.test_id.as_ref() != Some(test_id))
        {
            return false;
        }
        if self
            .text
            .as_ref()
            .is_some_and(|text| !matches!(&node.content, Content::Text(t) if t == text))
        {
            return false;
        }
        let name = accessible_name(layout, index);
        if let Some(label) = &self.label
            && !(matches!(node.content, Content::Input { .. }) && name.as_ref() == Some(label))
        {
            return false;
        }
        match &self.name {
            Some(Name::Exact(expected)) => name.as_ref() == Some(expected),
            Some(Name::Contains(part)) => name.is_some_and(|name| name.contains(part.as_str())),
            None => true,
        }
    }

    /// Lowercase strings a near miss could contain or be contained in.
    fn needles(&self) -> Vec<String> {
        let name = self.name.as_ref().map(|name| match name {
            Name::Exact(name) | Name::Contains(name) => name,
        });
        [
            name,
            self.label.as_ref(),
            self.text.as_ref(),
            self.test_id.as_ref(),
        ]
        .into_iter()
        .flatten()
        .map(|needle| needle.to_lowercase())
        .collect()
    }

    /// Near miss worth listing. Text folded into a control's name is left
    /// out: the control is listed instead.
    fn is_similar(&self, layout: &Layout, index: usize) -> bool {
        let node = layout.get(index);
        if node.hidden && !self.include_hidden {
            return false;
        }
        let mut ancestor = node.parent;
        while let Some(parent) = ancestor {
            if layout.get(parent).control {
                return false;
            }
            ancestor = layout.get(parent).parent;
        }
        if self.role.is_some_and(|role| effective_role(node) == role) {
            return true;
        }
        let name = accessible_name(layout, index).map(|name| name.to_lowercase());
        let test_id = node.test_id.as_ref().map(|id| id.to_lowercase());
        self.needles().iter().any(|needle| {
            [&name, &test_id].into_iter().flatten().any(|value| {
                !value.is_empty() && (value.contains(needle.as_str()) || needle.contains(value))
            })
        })
    }
}

impl fmt::Display for By {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut parts = Vec::new();
        if let Some(role) = self.role {
            parts.push(format!("role={role:?}"));
        }
        match &self.name {
            Some(Name::Exact(name)) => parts.push(format!("name={name:?}")),
            Some(Name::Contains(part)) => parts.push(format!("name~={part:?}")),
            None => {}
        }
        if let Some(label) = &self.label {
            parts.push(format!("label={label:?}"));
        }
        if let Some(text) = &self.text {
            parts.push(format!("text={text:?}"));
        }
        if let Some(test_id) = &self.test_id {
            parts.push(format!("test_id={test_id:?}"));
        }
        if self.include_hidden {
            parts.push("include_hidden".to_owned());
        }
        f.write_str(&parts.join(", "))
    }
}

/// A laid-out tree to query, whole or scoped to a subtree
/// ([`Screen::within`]).
#[derive(Clone, Copy)]
pub struct Screen<'a> {
    layout: &'a Layout,
    /// Descendants of this node only; `None` = the whole tree.
    scope: Option<usize>,
}

impl<'a> Screen<'a> {
    pub fn new(layout: &'a Layout) -> Self {
        Self {
            layout,
            scope: None,
        }
    }

    /// Queries limited to the descendants of `container`.
    pub fn within(&self, container: Found<'a>) -> Screen<'a> {
        Screen {
            layout: self.layout,
            scope: Some(container.index),
        }
    }

    /// Every match, in document order.
    pub fn all(&self, by: By) -> Vec<Found<'a>> {
        self.indices()
            .into_iter()
            .filter(|index| by.matches(self.layout, *index))
            .map(|index| self.found(index))
            .collect()
    }

    /// The single match, if any. Panics when several elements match.
    #[track_caller]
    pub fn query(&self, by: By) -> Option<Found<'a>> {
        let matches = self.all(by.clone());
        if matches.len() > 1 {
            panic!("{}", self.too_many(&by, &matches));
        }
        matches.into_iter().next()
    }

    /// The single match. Panics, listing near misses, when none or several
    /// elements match.
    #[track_caller]
    pub fn get(&self, by: By) -> Found<'a> {
        self.try_get(by)
            .unwrap_or_else(|message| panic!("{message}"))
    }

    /// [`Screen::get`] returning the failure message instead of panicking.
    pub fn try_get(&self, by: By) -> Result<Found<'a>, String> {
        let matches = self.all(by.clone());
        match matches.len() {
            1 => Ok(matches[0]),
            0 => Err(self.none(&by)),
            _ => Err(self.too_many(&by, &matches)),
        }
    }

    #[track_caller]
    pub fn get_by_test_id(&self, test_id: &str) -> Found<'a> {
        self.get(By::test_id(test_id))
    }

    #[track_caller]
    pub fn get_by_label(&self, label: &str) -> Found<'a> {
        self.get(By::label(label))
    }

    #[track_caller]
    pub fn get_by_text(&self, text: &str) -> Found<'a> {
        self.get(By::text(text))
    }

    pub(crate) fn by_id(&self, id: WidgetId) -> Option<Found<'a>> {
        let index = self.layout.nodes().iter().position(|node| node.id == id)?;
        Some(self.found(index))
    }

    fn found(&self, index: usize) -> Found<'a> {
        Found {
            layout: self.layout,
            index,
        }
    }

    /// Nodes in scope, in document order.
    fn indices(&self) -> Vec<usize> {
        fn collect(layout: &Layout, index: usize, out: &mut Vec<usize>) {
            out.push(index);
            for child in &layout.get(index).children {
                collect(layout, *child, out);
            }
        }
        let mut out = Vec::new();
        match self.scope {
            Some(scope) => {
                for child in &self.layout.get(scope).children {
                    collect(self.layout, *child, &mut out);
                }
            }
            None if self.layout.root().is_some() => collect(self.layout, 0, &mut out),
            None => {}
        }
        out
    }

    fn none(&self, by: &By) -> String {
        let candidates: Vec<Found<'a>> = self
            .indices()
            .into_iter()
            .filter(|index| by.is_similar(self.layout, *index))
            .map(|index| self.found(index))
            .collect();
        let mut message = format!("ningún elemento con {by}");
        if candidates.is_empty() {
            message.push_str("; sin candidatos parecidos");
        } else {
            message.push_str("; candidatos parecidos:");
            list(&mut message, &candidates);
        }
        message
    }

    fn too_many(&self, by: &By, matches: &[Found<'a>]) -> String {
        let mut message = format!(
            "{} elementos con {by}; agregar condiciones o usar within/all:",
            matches.len()
        );
        list(&mut message, matches);
        message
    }
}

fn list(message: &mut String, found: &[Found<'_>]) {
    for item in found.iter().take(MAX_CANDIDATES) {
        message.push_str("\n  ");
        message.push_str(&item.describe());
    }
    if found.len() > MAX_CANDIDATES {
        message.push_str(&format!("\n  … y {} más", found.len() - MAX_CANDIDATES));
    }
}

/// An element found by a query.
#[derive(Clone, Copy)]
pub struct Found<'a> {
    layout: &'a Layout,
    index: usize,
}

impl<'a> Found<'a> {
    pub fn id(&self) -> WidgetId {
        self.node().id
    }

    /// Position in [`Layout::nodes`] (document order).
    pub fn index(&self) -> usize {
        self.index
    }

    pub fn node(&self) -> &'a LayoutNode {
        self.layout.get(self.index)
    }

    pub fn role(&self) -> Role {
        effective_role(self.node())
    }

    /// Accessible name, as VoiceOver reads it.
    pub fn name(&self) -> Option<String> {
        accessible_name(self.layout, self.index)
    }

    pub fn test_id(&self) -> Option<&'a str> {
        self.node().test_id.as_deref()
    }

    pub fn bounds(&self) -> Rect {
        self.node().bounds
    }

    pub fn parent(&self) -> Option<Found<'a>> {
        self.node().parent.map(|index| Found {
            layout: self.layout,
            index,
        })
    }

    /// Child positions from the root, e.g. `root > 2 > 0`.
    pub fn path(&self) -> String {
        let mut positions = Vec::new();
        let mut index = self.index;
        while let Some(parent) = self.layout.get(index).parent {
            let position = self
                .layout
                .get(parent)
                .children
                .iter()
                .position(|child| *child == index)
                .unwrap_or_default();
            positions.push(position.to_string());
            index = parent;
        }
        positions.push("root".to_owned());
        positions.reverse();
        positions.join(" > ")
    }

    pub(crate) fn describe(&self) -> String {
        let mut text = format!("{:?}", self.role());
        if let Some(name) = self.name() {
            text.push_str(&format!(" {name:?}"));
        }
        if let Some(test_id) = self.test_id() {
            text.push_str(&format!(" test_id={test_id:?}"));
        }
        if self.node().hidden {
            text.push_str(" (oculto)");
        }
        text.push_str(&format!(" en {}", self.path()));
        text
    }
}

impl fmt::Debug for Found<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.describe())
    }
}

#[cfg(test)]
mod tests {
    use super::{By, Screen};
    use crate::element::sample::{self, Msg};
    use crate::element::{Element, Role, div, text};
    use crate::testing::layout_of;

    #[test]
    fn role_and_name_find_buttons_like_voiceover() {
        let layout = layout_of(&sample::tree());
        let screen = Screen::new(&layout);
        let button = screen.get(By::role(Role::Button).name("Probar interacción"));
        assert_eq!(button.id(), sample::button());
        assert_eq!(button.role(), Role::Button);
        assert_eq!(
            screen
                .get(By::role(Role::Button).name_contains("disponible"))
                .id(),
            sample::disabled_button()
        );
        let buttons: Vec<_> = screen
            .all(By::role(Role::Button))
            .iter()
            .map(|found| found.id())
            .collect();
        assert_eq!(buttons, [sample::button(), sample::disabled_button()]);
        assert_eq!(
            screen.get(By::role(Role::Heading)).name().as_deref(),
            Some("Kai UI Desktop")
        );
    }

    #[test]
    fn label_text_and_test_id() {
        let mut tree = sample::tree();
        tree.children[4].test_id = Some("estado".to_owned());
        let layout = layout_of(&tree);
        let screen = Screen::new(&layout);
        assert_eq!(screen.get_by_label("Escribe aquí").id(), sample::field());
        assert!(
            screen.query(By::label("Listo")).is_none(),
            "labels only name inputs"
        );
        assert_eq!(screen.get_by_text("Listo").id(), sample::status());
        assert_eq!(screen.get_by_test_id("estado").id(), sample::status());
        assert_eq!(screen.get_by_test_id("estado").test_id(), Some("estado"));
        assert!(screen.query(By::test_id("otro")).is_none());
    }

    #[test]
    fn within_limits_the_search_to_descendants() {
        let layout = layout_of(&sample::tree());
        let screen = Screen::new(&layout);
        let button = screen.get(By::role(Role::Button).name("Probar interacción"));
        let inside = screen.within(button);
        assert_eq!(
            inside.get_by_text("Probar interacción").id(),
            sample::button_label()
        );
        assert!(inside.query(By::text("Listo")).is_none());
        assert!(
            inside.query(By::role(Role::Button)).is_none(),
            "the container itself is not a descendant"
        );
        assert_eq!(
            inside
                .get_by_text("Probar interacción")
                .parent()
                .unwrap()
                .id(),
            sample::button()
        );
    }

    #[test]
    fn hidden_elements_are_skipped_unless_asked() {
        let mut tree: Element<Msg> =
            div().children([text("Visible"), div().class("hidden").child(text("Oculto"))]);
        tree.assign_ids();
        let layout = layout_of(&tree);
        let screen = Screen::new(&layout);
        assert!(screen.query(By::text("Oculto")).is_none());
        let hidden = screen.get(By::text("Oculto").include_hidden());
        assert!(hidden.node().hidden);
        assert_eq!(hidden.path(), "root > 1 > 0");
    }

    #[test]
    fn get_without_matches_lists_near_misses() {
        let mut tree = sample::tree();
        tree.children[2].test_id = Some("demo.probar".to_owned());
        let layout = layout_of(&tree);
        let screen = Screen::new(&layout);
        assert_eq!(
            screen
                .try_get(By::role(Role::Button).name("Probar"))
                .unwrap_err(),
            "ningún elemento con role=Button, name=\"Probar\"; candidatos parecidos:\n  \
             Button \"Probar interacción\" test_id=\"demo.probar\" en root > 2\n  \
             Button \"No disponible\" en root > 3"
        );
        assert_eq!(
            screen.try_get(By::test_id("demo.probar.x")).unwrap_err(),
            "ningún elemento con test_id=\"demo.probar.x\"; candidatos parecidos:\n  \
             Button \"Probar interacción\" test_id=\"demo.probar\" en root > 2"
        );
        assert_eq!(
            screen.try_get(By::text("Inexistente")).unwrap_err(),
            "ningún elemento con text=\"Inexistente\"; sin candidatos parecidos"
        );
    }

    #[test]
    fn get_with_several_matches_lists_them() {
        let layout = layout_of(&sample::tree());
        let screen = Screen::new(&layout);
        assert_eq!(
            screen.try_get(By::role(Role::Button)).unwrap_err(),
            "2 elementos con role=Button; agregar condiciones o usar within/all:\n  \
             Button \"Probar interacción\" en root > 2\n  \
             Button \"No disponible\" en root > 3"
        );
    }

    #[test]
    #[should_panic(expected = "2 elementos con role=Button")]
    fn query_panics_on_several_matches() {
        let layout = layout_of(&sample::tree());
        Screen::new(&layout).query(By::role(Role::Button));
    }
}
