//! Empty and loading patterns for data surfaces (lists, tables, panels).
//!
//! Inspired by Polaris `EmptyState` and Carbon empty/loading patterns, kept
//! AppKit-canonical: Lucide icon, title, optional body, optional Primary /
//! Secondary action — no web chrome.
//!
//! Distinguish **filter empty** (query returned nothing) from **resource
//! empty** (collection never had items) via [`EmptyKind`] /
//! [`EmptyState::filter`] / [`EmptyState::resource`].

use crate::element::{Element, Role, div, icon, text};
use crate::icons::Icon;

use super::progress::{DotProgress, DotSize};
use super::{ButtonVariant, button, caption, paragraph};


/// Why the surface is empty — drives recommended copy and icons.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EmptyKind {
    /// A search or filter returned no rows; suggest clearing or refining.
    Filter,
    /// The collection has never had items; suggest creating the first one.
    Resource,
}

/// Reusable empty surface: icon + title + optional description + optional action.
#[derive(Clone, Debug)]
pub struct EmptyState<Msg> {
    icon: Icon,
    title: String,
    description: Option<String>,
    action_label: Option<String>,
    action: Option<Msg>,
    action_variant: ButtonVariant,
    test_id: Option<String>,
    kind: EmptyKind,
}

impl<Msg> EmptyState<Msg> {
    pub fn new(icon: Icon, title: impl Into<String>) -> Self {
        Self {
            icon,
            title: title.into(),
            description: None,
            action_label: None,
            action: None,
            action_variant: ButtonVariant::Primary,
            test_id: None,
            kind: EmptyKind::Resource,
        }
    }

    /// Filter/search empty: search icon + title (default kind [`EmptyKind::Filter`]).
    pub fn filter(title: impl Into<String>) -> Self {
        Self {
            kind: EmptyKind::Filter,
            ..Self::new(Icon::Search, title)
        }
    }

    /// Resource empty: inbox icon + title (default kind [`EmptyKind::Resource`]).
    pub fn resource(title: impl Into<String>) -> Self {
        Self {
            kind: EmptyKind::Resource,
            ..Self::new(Icon::Inbox, title)
        }
    }

    pub fn kind(mut self, kind: EmptyKind) -> Self {
        self.kind = kind;
        self
    }

    pub fn empty_kind(&self) -> EmptyKind {
        self.kind
    }

    pub fn description(mut self, text: impl Into<String>) -> Self {
        self.description = Some(text.into());
        self
    }

    /// Optional Primary (by default) button under the body.
    pub fn action(mut self, label: impl Into<String>, msg: Msg) -> Self {
        self.action_label = Some(label.into());
        self.action = Some(msg);
        self
    }

    pub fn action_variant(mut self, variant: ButtonVariant) -> Self {
        self.action_variant = variant;
        self
    }

    pub fn test_id(mut self, test_id: impl Into<String>) -> Self {
        self.test_id = Some(test_id.into());
        self
    }

    pub fn build(self) -> Element<Msg>
    where
        Msg: Clone + 'static,
    {
        let mut children: Vec<Element<Msg>> = vec![
            icon(self.icon)
                .class(
                    "size-10 text-muted-foreground appkit:text-secondary-label shrink-0",
                )
                .part("icon"),
            text(self.title.clone())
                .class(
                    "text-base font-semibold text-foreground appkit:text-label text-center",
                )
                .part("title"),
        ];
        if let Some(description) = self.description {
            children.push(
                paragraph(description)
                    .class(
                        "text-sm text-muted-foreground appkit:text-secondary-label text-center \
                         max-w-sm",
                    )
                    .part("description"),
            );
        }
        if let (Some(label), Some(msg)) = (self.action_label, self.action) {
            children.push(
                button(label, self.action_variant)
                    .on_click(msg)
                    .part("action"),
            );
        }
        let mut root = div()
            .class(
                "flex flex-col items-center justify-center gap-3 w-full min-h-[160px] px-6 py-8",
            )
            .role(Role::Status)
            .label(self.title)
            .parts_root()
            .children(children);
        root.test_id = self.test_id;
        root
    }
}

/// Centered indeterminate loading for a list or table body.
pub fn loading_placeholder<Msg>(label: impl Into<String>) -> Element<Msg> {
    let label = label.into();
    div()
        .class(
            "flex flex-col items-center justify-center gap-3 w-full min-h-[120px] px-6 py-8 \
             text-muted-foreground appkit:text-secondary-label",
        )
        .role(Role::Status)
        .label(label.clone())
        .parts_root()
        .test_id("pattern.loading")
        .children([
            DotProgress::new()
                .size(DotSize::Md)
                .label(label.clone())
                .build()
                .part("dots"),
            caption(label).part("caption"),
        ])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assigned(mut tree: Element<()>) -> Element<()> {
        tree.assign_ids();
        tree
    }

    #[test]
    fn empty_state_exposes_title_description_and_action() {
        let tree = assigned(
            EmptyState::new(Icon::Inbox, "Sin elementos")
                .description("Todavía no hay nada aquí")
                .action("Crear", ())
                .test_id("empty")
                .build(),
        );
        assert_eq!(tree.role, Some(Role::Status));
        assert_eq!(
            tree.find_test_id("empty.title").map(|e| e.text_content()),
            Some("Sin elementos".into())
        );
        let desc = tree.find_test_id("empty.description").expect("description");
        assert!(
            desc.text_content().contains("Todavía"),
            "{}",
            desc.text_content()
        );
        assert!(tree.find_test_id("empty.action").is_some());
        assert!(tree.find_test_id("empty.icon").is_some());
    }

    #[test]
    fn loading_placeholder_is_a_named_status() {
        let tree = assigned(loading_placeholder::<()>("Cargando filas"));
        assert_eq!(tree.role, Some(Role::Status));
        assert_eq!(tree.test_id.as_deref(), Some("pattern.loading"));
        assert!(tree.find_test_id("pattern.loading.dots").is_some());
    }

    #[test]
    fn filter_and_resource_kinds() {
        assert_eq!(
            EmptyState::<()>::filter("Sin coincidencias").empty_kind(),
            EmptyKind::Filter
        );
        assert_eq!(
            EmptyState::<()>::resource("Sin elementos").empty_kind(),
            EmptyKind::Resource
        );
        let f = assigned(
            EmptyState::<()>::filter("Sin coincidencias")
                .test_id("f")
                .build(),
        );
        assert_eq!(
            f.find_test_id("f.title").map(|e| e.text_content()),
            Some("Sin coincidencias".into())
        );
    }
}
