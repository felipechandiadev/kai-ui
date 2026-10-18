//! AccessKit tree derived from the computed layout, built as pure data.
//!
//! Bounds are the visible part of each node in logical points; the window
//! node carries a scale transform because AccessKit's macOS adapter expects
//! physical pixels. Node ids are `WidgetId + 1`; id 0 is the window.
//!
//! Roles come from [`Element::role`](crate::element::Element::role) or are
//! inferred: inputs are text inputs, clickable elements are buttons named
//! after their text content, text elements are labels. Buttons and inputs
//! are leaves; hidden (`display: none`) subtrees are omitted.

use accesskit::{
    Action, ActionData, ActionRequest, Affine, Node, NodeId, Rect as AccessRect, Role, TreeId,
    TreeInfo, TreeUpdate,
};

use crate::element::{FocusKind, FocusOrder, Role as KaiRole, WidgetId};
use crate::geometry::Rect;
use crate::layout::{Content, Layout, LayoutNode};
use crate::ui_engine::{Intent, TextIntent};

pub const WINDOW_ID: NodeId = NodeId(0);

pub fn node_id(widget: WidgetId) -> NodeId {
    NodeId(widget.0 + 1)
}

pub fn widget_id(node: NodeId) -> Option<WidgetId> {
    node.0.checked_sub(1).map(WidgetId)
}

pub struct A11ySnapshot<'a> {
    pub layout: &'a Layout,
    pub scale_factor: f64,
    pub focused: Option<WidgetId>,
    pub window_title: &'a str,
    /// Current value of each input.
    pub field_values: &'a [(WidgetId, &'a str)],
}

pub fn a11y_tree(snapshot: &A11ySnapshot<'_>) -> TreeUpdate {
    let layout = snapshot.layout;
    let mut window = Node::new(Role::Window);
    window.set_label(snapshot.window_title);
    window.set_transform(Affine::scale(snapshot.scale_factor));
    window.set_bounds(access_rect(Rect {
        x: 0.0,
        y: 0.0,
        width: layout.viewport.width,
        height: layout.viewport.height,
    }));
    let mut nodes = Vec::new();
    if let Some(root) = layout.root() {
        window.set_children(vec![node_id(root.id)]);
        emit(snapshot, 0, &mut nodes);
    }
    nodes.insert(0, (WINDOW_ID, window));
    TreeUpdate {
        nodes,
        tree: Some(TreeInfo::new(WINDOW_ID)),
        tree_id: TreeId::ROOT,
        focus: snapshot.focused.map_or(WINDOW_ID, node_id),
    }
}

fn role_of(node: &LayoutNode) -> Role {
    if matches!(node.content, Content::Input { masked: true, .. }) {
        return Role::PasswordInput;
    }
    match node.role {
        Some(KaiRole::Generic) => Role::GenericContainer,
        Some(KaiRole::Main) => Role::Main,
        Some(KaiRole::Header) => Role::Banner,
        Some(KaiRole::Group) => Role::Group,
        Some(KaiRole::Heading) => Role::Heading,
        Some(KaiRole::Paragraph) => Role::Paragraph,
        Some(KaiRole::Label) => Role::Label,
        Some(KaiRole::Status) => Role::Status,
        Some(KaiRole::Button) => Role::Button,
        Some(KaiRole::TextInput) => Role::TextInput,
        Some(KaiRole::Image) => Role::Image,
        None => match node.content {
            Content::Input { .. } => Role::TextInput,
            Content::Icon(_) => Role::Image,
            _ if node.control => Role::Button,
            Content::Text(_) => Role::Label,
            Content::Box => Role::GenericContainer,
        },
    }
}

/// Hidden nodes and unlabeled (decorative) icons are left out of the tree.
fn exposed(node: &LayoutNode) -> bool {
    let decorative = matches!(node.content, Content::Icon(_)) && node.label.is_none();
    !node.hidden && !decorative
}

fn text_content(layout: &Layout, index: usize, parts: &mut Vec<String>) {
    let node = layout.get(index);
    if node.hidden {
        return;
    }
    if let Content::Text(text) = &node.content {
        parts.push(text.clone());
    }
    for child in &node.children {
        text_content(layout, *child, parts);
    }
}

fn emit(snapshot: &A11ySnapshot<'_>, index: usize, nodes: &mut Vec<(NodeId, Node)>) {
    let layout = snapshot.layout;
    let source = layout.get(index);
    let role = role_of(source);
    let mut node = Node::new(role);
    let leaf = matches!(role, Role::Button | Role::TextInput | Role::PasswordInput);
    match &source.content {
        Content::Text(text) => node.set_value(text.as_str()),
        Content::Input {
            placeholder,
            masked,
            read_only,
        } => {
            node.set_label(source.label.as_deref().unwrap_or(placeholder));
            node.set_placeholder(placeholder.as_str());
            if !masked {
                let value = snapshot
                    .field_values
                    .iter()
                    .find(|(id, _)| *id == source.id)
                    .map_or("", |(_, value)| *value);
                node.set_value(value);
            }
            if *read_only {
                node.set_read_only();
            }
        }
        Content::Icon(_) => {
            if let Some(label) = &source.label {
                node.set_label(label.as_str());
            }
        }
        Content::Box => {
            if let Some(label) = &source.label {
                node.set_label(label.as_str());
            } else if role == Role::Button {
                let mut parts = Vec::new();
                text_content(layout, index, &mut parts);
                node.set_label(parts.join(" "));
            }
        }
    }
    if source.required {
        node.set_required();
    }
    if let Some(description) = &source.description {
        node.set_description(description.as_str());
    }
    if source.control {
        if source.disabled {
            node.set_disabled();
        } else {
            node.add_action(Action::Focus);
            node.add_action(Action::Click);
            if matches!(source.content, Content::Input { .. }) {
                node.add_action(Action::ReplaceSelectedText);
            }
        }
    }
    if let Some(bounds) = source.visible_bounds() {
        node.set_bounds(access_rect(bounds));
    }
    let children: Vec<usize> = if leaf {
        Vec::new()
    } else {
        source
            .children
            .iter()
            .copied()
            .filter(|child| exposed(layout.get(*child)))
            .collect()
    };
    if !children.is_empty() {
        node.set_children(
            children
                .iter()
                .map(|child| node_id(layout.get(*child).id))
                .collect::<Vec<_>>(),
        );
    }
    nodes.push((node_id(source.id), node));
    for child in children {
        emit(snapshot, child, nodes);
    }
}

/// Translate an assistive-technology request into Kai intents. Only enabled,
/// focusable elements accept actions.
pub fn intents_for_action(request: &ActionRequest, order: &FocusOrder) -> Vec<Intent> {
    let Some((id, kind)) =
        widget_id(request.target_node).and_then(|id| order.kind(id).map(|kind| (id, kind)))
    else {
        return Vec::new();
    };
    match (request.action, kind, &request.data) {
        (Action::Focus, _, _) => vec![Intent::SetFocus(Some(id))],
        (Action::Click, _, _) => vec![Intent::Activate(id)],
        (Action::ReplaceSelectedText, FocusKind::TextField, Some(ActionData::Value(text))) => {
            vec![
                Intent::SetFocus(Some(id)),
                Intent::Text(TextIntent::InsertText(text.to_string())),
            ]
        }
        _ => Vec::new(),
    }
}

fn access_rect(rect: Rect) -> AccessRect {
    AccessRect {
        x0: f64::from(rect.x),
        y0: f64::from(rect.y),
        x1: f64::from(rect.x + rect.width),
        y1: f64::from(rect.y + rect.height),
    }
}

#[cfg(test)]
mod tests {
    use accesskit::{Action, ActionData, ActionRequest, Affine, NodeId, Role, TreeId};

    use super::{A11ySnapshot, WINDOW_ID, a11y_tree, intents_for_action, node_id, widget_id};
    use crate::element::sample::{self, Msg};
    use crate::element::{Element, FocusOrder, WidgetId, div, icon, input};
    use crate::geometry::Size;
    use crate::icons::Icon;
    use crate::layout::testing::FixedMeasure;
    use crate::layout::{Interaction, Layout, LayoutInput, ScrollOffsets, StyleCache};
    use crate::theme::{Density, Theme};
    use crate::ui_engine::{Intent, TextIntent};

    fn layout(tree: &Element<Msg>, viewport: Size, scroll: &ScrollOffsets) -> Layout {
        Layout::compute(
            tree,
            &LayoutInput {
                viewport,
                theme: &Theme::kai(Density::Comfortable),
                interaction: &Interaction::default(),
                scroll,
            },
            &mut StyleCache::default(),
            &mut FixedMeasure,
        )
    }

    fn snapshot<'a>(layout: &'a Layout, focused: Option<WidgetId>) -> A11ySnapshot<'a> {
        A11ySnapshot {
            layout,
            scale_factor: 2.0,
            focused,
            window_title: "Kai",
            field_values: &[],
        }
    }

    fn node(update: &accesskit::TreeUpdate, id: NodeId) -> &accesskit::Node {
        &update
            .nodes
            .iter()
            .find(|(node_id, _)| *node_id == id)
            .unwrap()
            .1
    }

    fn request(action: Action, target: NodeId, data: Option<ActionData>) -> ActionRequest {
        ActionRequest {
            action,
            target_tree: TreeId::ROOT,
            target_node: target,
            data,
        }
    }

    const WINDOW: Size = Size {
        width: 1024.0,
        height: 720.0,
    };

    #[test]
    fn tree_mirrors_elements_with_roles_names_values_and_scaled_root() {
        let tree = sample::tree();
        let layout = layout(&tree, WINDOW, &ScrollOffsets::new());
        let values = [(sample::field(), "hola")];
        let update = a11y_tree(&A11ySnapshot {
            field_values: &values,
            ..snapshot(&layout, None)
        });

        assert_eq!(update.tree.as_ref().unwrap().root, WINDOW_ID);
        assert_eq!(update.focus, WINDOW_ID);
        assert_eq!(
            update.nodes.len(),
            8,
            "window + root + 6 children; button labels fold"
        );
        let window = node(&update, WINDOW_ID);
        assert_eq!(window.role(), Role::Window);
        assert_eq!(window.transform(), Some(&Affine::scale(2.0)));
        assert_eq!(window.children(), &[node_id(sample::root())]);
        let root = node(&update, node_id(sample::root()));
        assert_eq!(root.role(), Role::Main);
        assert_eq!(root.children().len(), 6);

        assert_eq!(
            node(&update, node_id(sample::heading())).role(),
            Role::Heading
        );
        let description = node(&update, node_id(sample::description()));
        assert_eq!(description.role(), Role::Label);
        let status = node(&update, node_id(sample::status()));
        assert_eq!(status.role(), Role::Status);
        assert_eq!(status.value(), Some("Listo"));

        let button = node(&update, node_id(sample::button()));
        assert_eq!(button.role(), Role::Button);
        assert_eq!(button.label(), Some("Probar interacción"));
        assert!(button.supports_action(Action::Click));
        assert!(button.children().is_empty());

        let disabled = node(&update, node_id(sample::disabled_button()));
        assert!(disabled.is_disabled());
        assert!(!disabled.supports_action(Action::Click));

        let field = node(&update, node_id(sample::field()));
        assert_eq!(field.role(), Role::TextInput);
        assert_eq!(field.value(), Some("hola"));
        assert_eq!(field.placeholder(), Some("Escribe aquí"));
        assert!(field.supports_action(Action::ReplaceSelectedText));
    }

    #[test]
    fn icons_and_field_flags_map_to_accessibility_properties() {
        let mut tree: Element<Msg> = div().children([
            icon(Icon::Search),
            icon(Icon::CircleAlert).label("Error"),
            input("secreto", "Contraseña")
                .masked(true)
                .required(true)
                .description("Mínimo 8 caracteres"),
            input("fijo", "Código").read_only(true),
        ]);
        tree.assign_ids();
        let ids: Vec<WidgetId> = tree.children.iter().map(|child| child.id).collect();
        let layout = layout(&tree, WINDOW, &ScrollOffsets::new());
        let values = [(ids[2], "secreto"), (ids[3], "fijo")];
        let update = a11y_tree(&A11ySnapshot {
            field_values: &values,
            ..snapshot(&layout, None)
        });

        let root = node(&update, node_id(tree.id));
        assert_eq!(
            root.children(),
            &[node_id(ids[1]), node_id(ids[2]), node_id(ids[3])],
            "decorative icons are omitted"
        );
        let alert = node(&update, node_id(ids[1]));
        assert_eq!(alert.role(), Role::Image);
        assert_eq!(alert.label(), Some("Error"));

        let password = node(&update, node_id(ids[2]));
        assert_eq!(password.role(), Role::PasswordInput);
        assert_eq!(password.value(), None, "masked values never leak");
        assert!(password.is_required());
        assert_eq!(password.description(), Some("Mínimo 8 caracteres"));

        let code = node(&update, node_id(ids[3]));
        assert_eq!(code.role(), Role::TextInput);
        assert!(code.is_read_only());
        assert_eq!(code.value(), Some("fijo"));
    }

    #[test]
    fn focus_and_bounds_follow_layout_scroll_and_clip() {
        let tree = sample::tree();
        let small = Size {
            width: 420.0,
            height: 200.0,
        };
        let unscrolled = layout(&tree, small, &ScrollOffsets::new());
        let hidden = a11y_tree(&snapshot(&unscrolled, None));
        assert!(
            node(&hidden, node_id(sample::field())).bounds().is_none(),
            "clipped out"
        );

        let mut offsets = ScrollOffsets::new();
        offsets.insert(sample::root(), 10_000.0);
        let scrolled = layout(&tree, small, &offsets);
        let update = a11y_tree(&snapshot(&scrolled, Some(sample::field())));
        assert_eq!(update.focus, node_id(sample::field()));
        let bounds = node(&update, node_id(sample::field())).bounds().unwrap();
        let field = scrolled.bounds(sample::field()).unwrap();
        assert_eq!(bounds.y0, f64::from(field.y));
        assert!(bounds.y1 <= 200.0);
    }

    #[test]
    fn actions_translate_to_intents_for_enabled_elements_only() {
        let order = FocusOrder::from_tree(&sample::tree());
        let (button, field) = (sample::button(), sample::field());
        assert_eq!(widget_id(node_id(field)), Some(field));
        assert_eq!(widget_id(WINDOW_ID), None);
        assert_eq!(
            intents_for_action(&request(Action::Click, node_id(button), None), &order),
            vec![Intent::Activate(button)]
        );
        assert_eq!(
            intents_for_action(&request(Action::Focus, node_id(field), None), &order),
            vec![Intent::SetFocus(Some(field))]
        );
        assert_eq!(
            intents_for_action(
                &request(
                    Action::ReplaceSelectedText,
                    node_id(field),
                    Some(ActionData::Value("á".into()))
                ),
                &order
            ),
            vec![
                Intent::SetFocus(Some(field)),
                Intent::Text(TextIntent::InsertText("á".to_owned())),
            ]
        );
        assert!(intents_for_action(&request(Action::Click, WINDOW_ID, None), &order).is_empty());
        assert!(
            intents_for_action(
                &request(Action::Click, node_id(sample::disabled_button()), None),
                &order
            )
            .is_empty()
        );
        assert!(
            intents_for_action(
                &request(Action::ReplaceSelectedText, node_id(button), None),
                &order
            )
            .is_empty()
        );
    }
}
