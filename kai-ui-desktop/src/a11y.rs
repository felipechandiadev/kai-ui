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

use crate::element::{ControlState, FocusKind, FocusOrder, Role as KaiRole, WidgetId};
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

/// Kai role of a node: its own, or inferred (inputs are text inputs,
/// icons images, clickable elements buttons, text labels). Shared by the
/// AccessKit tree and the `testing` queries.
pub fn effective_role(node: &LayoutNode) -> KaiRole {
    node.role.unwrap_or(match node.content {
        Content::Input { lines: Some(_), .. } => KaiRole::TextArea,
        Content::Input { .. } => KaiRole::TextInput,
        Content::Icon(_) | Content::Image(_) => KaiRole::Image,
        _ if node.control => KaiRole::Button,
        Content::Text(_) => KaiRole::Label,
        Content::Box => KaiRole::Generic,
    })
}

/// Name assistive technologies read: the explicit label; for inputs the
/// placeholder; for buttons (default ones too) their text content; for
/// text, the text itself.
pub fn accessible_name(layout: &Layout, index: usize) -> Option<String> {
    let node = layout.get(index);
    match &node.content {
        Content::Input { placeholder, .. } => {
            Some(node.label.clone().unwrap_or_else(|| placeholder.clone()))
        }
        Content::Text(text) => Some(node.label.clone().unwrap_or_else(|| text.clone())),
        Content::Icon(_) | Content::Image(_) => node.label.clone(),
        Content::Box => node.label.clone().or_else(|| {
            matches!(
                effective_role(node),
                KaiRole::Button
                    | KaiRole::Switch
                    | KaiRole::Checkbox
                    | KaiRole::Radio
                    | KaiRole::PopUpButton
                    | KaiRole::MenuItem
                    | KaiRole::SidebarItem
                    | KaiRole::Row
                    | KaiRole::Cell
                    | KaiRole::ColumnHeader
                    | KaiRole::ListItem
                    | KaiRole::Tooltip
                    | KaiRole::Alert
            )
            .then(|| {
                let mut parts = Vec::new();
                text_content(layout, index, &mut parts);
                parts.join(" ")
            })
        }),
    }
}

fn role_of(node: &LayoutNode) -> Role {
    if matches!(node.content, Content::Input { masked: true, .. }) {
        return Role::PasswordInput;
    }
    match effective_role(node) {
        KaiRole::Generic => Role::GenericContainer,
        KaiRole::Main => Role::Main,
        KaiRole::Header => Role::Banner,
        KaiRole::Group => Role::Group,
        KaiRole::Heading => Role::Heading,
        KaiRole::Paragraph => Role::Paragraph,
        KaiRole::Label => Role::Label,
        KaiRole::Status => Role::Status,
        KaiRole::Button if node.is_default => Role::DefaultButton,
        KaiRole::Button => Role::Button,
        KaiRole::Switch => Role::Switch,
        KaiRole::Checkbox => Role::CheckBox,
        KaiRole::RadioGroup => Role::RadioGroup,
        KaiRole::Radio => Role::RadioButton,
        // `accesskit_macos` publishes `ComboBox` as `AXPopUpButton`.
        KaiRole::PopUpButton => Role::ComboBox,
        KaiRole::Menu => Role::MenuListPopup,
        KaiRole::MenuItem => Role::MenuListOption,
        // `accesskit_macos`: `Tree` is `AXOutline` and `TreeItem` an
        // `AXRow` (`AXOutlineRow`), as Finder's sidebar.
        KaiRole::Sidebar => Role::Tree,
        KaiRole::SidebarItem => Role::TreeItem,
        KaiRole::TextInput => Role::TextInput,
        KaiRole::SearchInput => Role::SearchInput,
        KaiRole::TextArea => Role::MultilineTextInput,
        KaiRole::Toolbar => Role::Toolbar,
        KaiRole::Alert => Role::Alert,
        KaiRole::Table => Role::Table,
        KaiRole::Row => Role::Row,
        KaiRole::Cell => Role::Cell,
        KaiRole::ColumnHeader => Role::ColumnHeader,
        KaiRole::List => Role::List,
        KaiRole::ListItem => Role::ListItem,
        KaiRole::Tooltip => Role::Tooltip,
        KaiRole::Image => Role::Image,
        KaiRole::Dialog => Role::Dialog,
    }
}

/// Hidden nodes and unlabeled (decorative) icons and images are left out
/// of the tree.
fn exposed(node: &LayoutNode) -> bool {
    let decorative =
        matches!(node.content, Content::Icon(_) | Content::Image(_)) && node.label.is_none();
    !node.hidden && !decorative
}

/// Text of the subtree; an image (an avatar) has its own name and adds
/// nothing.
fn text_content(layout: &Layout, index: usize, parts: &mut Vec<String>) {
    let node = layout.get(index);
    if node.hidden || node.role == Some(KaiRole::Image) {
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
    let leaf = matches!(
        role,
        Role::Button
            | Role::DefaultButton
            | Role::Switch
            | Role::CheckBox
            | Role::RadioButton
            | Role::ComboBox
            | Role::MenuListOption
            | Role::TreeItem
            | Role::TextInput
            | Role::SearchInput
            | Role::MultilineTextInput
            | Role::PasswordInput
            | Role::Image
    );
    if !matches!(source.content, Content::Text(_))
        && let Some(name) = accessible_name(layout, index)
    {
        node.set_label(name);
    }
    match &source.content {
        Content::Text(text) => node.set_value(text.as_str()),
        Content::Input {
            placeholder,
            masked,
            read_only,
            ..
        } => {
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
        Content::Icon(_) | Content::Image(_) | Content::Box => {}
    }
    if let Some(test_id) = &source.test_id {
        node.set_author_id(test_id.as_str());
    }
    if source.required {
        node.set_required();
    }
    match role {
        Role::Dialog => node.set_modal(),
        Role::Status => node.set_live(accesskit::Live::Polite),
        Role::Alert => node.set_live(accesskit::Live::Assertive),
        _ => {}
    }
    if let Some(expanded) = source.expanded {
        node.set_expanded(expanded);
        let mut parts = Vec::new();
        text_content(layout, index, &mut parts);
        node.set_value(parts.join(" "));
    }
    if matches!(role, Role::MenuListOption | Role::TreeItem | Role::Row) {
        node.set_selected(source.control_state == Some(ControlState::On));
    }
    if let Some(count) = source.row_count {
        node.set_row_count(count);
    }
    if let Some(index) = source.row_index {
        node.set_row_index(index);
    }
    if role == Role::ColumnHeader {
        match source.control_state {
            Some(ControlState::On) => node.set_sort_direction(accesskit::SortDirection::Ascending),
            Some(ControlState::Mixed) => {
                node.set_sort_direction(accesskit::SortDirection::Descending);
            }
            _ => {}
        }
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
            if let Some(state) = source.control_state
                && matches!(
                    role,
                    Role::Button
                        | Role::DefaultButton
                        | Role::Switch
                        | Role::CheckBox
                        | Role::RadioButton
                )
            {
                node.set_toggled(match state {
                    ControlState::Off => accesskit::Toggled::False,
                    ControlState::On => accesskit::Toggled::True,
                    ControlState::Mixed => accesskit::Toggled::Mixed,
                });
            }
            if let Some(shortcut) = source.key_equivalent {
                node.set_keyboard_shortcut(shortcut.label());
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
    fn test_id_is_exposed_as_author_id() {
        let mut tree = sample::tree();
        tree.children[2].test_id = Some("demo.probar".to_owned());
        tree.children[5].test_id = Some("demo.campo".to_owned());
        let layout = layout(&tree, WINDOW, &ScrollOffsets::new());
        let update = a11y_tree(&snapshot(&layout, None));
        let author_id = |id| node(&update, node_id(id)).author_id();
        assert_eq!(author_id(sample::button()), Some("demo.probar"));
        assert_eq!(author_id(sample::field()), Some("demo.campo"));
        assert_eq!(author_id(sample::heading()), None);
    }

    #[test]
    fn labeled_switch_is_a_switch_named_after_its_label() {
        let mut tree: Element<Msg> = div().child(
            crate::components::switch_labeled(
                "Notificaciones",
                true,
                Msg::Clicked,
                false,
                crate::components::SwitchDensity::Default,
            )
            .test_id("avisos"),
        );
        tree.assign_ids();
        let track = tree.find_test_id("avisos.track").unwrap().id;
        let layout = layout(&tree, WINDOW, &ScrollOffsets::new());
        let update = a11y_tree(&snapshot(&layout, None));
        let switch = node(&update, node_id(track));
        assert_eq!(switch.role(), Role::Switch);
        assert_eq!(switch.label(), Some("Notificaciones"));
        assert_eq!(switch.toggled(), Some(accesskit::Toggled::True));
        assert_eq!(switch.author_id(), Some("avisos.track"));
        assert!(switch.children().is_empty());
    }

    #[test]
    fn checkbox_is_a_leaf_named_after_its_title_with_three_states() {
        use crate::components::checkbox;
        use crate::element::ControlState;

        for (state, toggled) in [
            (ControlState::Off, accesskit::Toggled::False),
            (ControlState::On, accesskit::Toggled::True),
            (ControlState::Mixed, accesskit::Toggled::Mixed),
        ] {
            let mut tree: Element<Msg> =
                div().child(checkbox("Acepto", state, Msg::Clicked).test_id("ok"));
            tree.assign_ids();
            let id = tree.children[0].id;
            let layout = layout(&tree, WINDOW, &ScrollOffsets::new());
            let update = a11y_tree(&snapshot(&layout, None));
            let check = node(&update, node_id(id));
            assert_eq!(check.role(), Role::CheckBox);
            assert_eq!(check.label(), Some("Acepto"));
            assert_eq!(check.toggled(), Some(toggled));
            assert!(check.children().is_empty());
        }
    }

    #[test]
    fn segmented_is_a_radio_group_of_toggled_radio_buttons() {
        let mut tree: Element<Msg> = div().child(
            crate::components::segmented(&["Día", "Mes"], 1, |_| Msg::Clicked)
                .label("Periodo")
                .test_id("p"),
        );
        tree.assign_ids();
        let group = tree.children[0].id;
        let layout = layout(&tree, WINDOW, &ScrollOffsets::new());
        let update = a11y_tree(&snapshot(&layout, None));
        let radio_group = node(&update, node_id(group));
        assert_eq!(radio_group.role(), Role::RadioGroup);
        assert_eq!(radio_group.label(), Some("Periodo"));
        assert_eq!(radio_group.children().len(), 2);
        let month = node(&update, radio_group.children()[1]);
        assert_eq!(month.role(), Role::RadioButton);
        assert_eq!(month.label(), Some("Mes"));
        assert_eq!(month.author_id(), Some("p.1"));
        assert_eq!(month.toggled(), Some(accesskit::Toggled::True));
        let day = node(&update, radio_group.children()[0]);
        assert_eq!(day.toggled(), Some(accesskit::Toggled::False));
    }

    #[test]
    fn sidebar_is_an_outline_of_rows_with_one_selected() {
        use crate::components::{SidebarGroup, SidebarItem, sidebar};

        let groups = [SidebarGroup::titled(
            "Demo",
            [
                SidebarItem::new("Inicio").badge("3"),
                SidebarItem::new("Campos"),
            ],
        )];
        let mut tree: Element<Msg> =
            div().child(sidebar(&groups, 1, |_| Msg::Clicked).test_id("nav"));
        tree.assign_ids();
        let root = tree.children[0].id;
        let rows = ["nav.0", "nav.1"].map(|id| tree.find_test_id(id).unwrap().id);
        let layout = layout(&tree, WINDOW, &ScrollOffsets::new());
        let update = a11y_tree(&snapshot(&layout, None));
        assert_eq!(node(&update, node_id(root)).role(), Role::Tree);
        let first = node(&update, node_id(rows[0]));
        assert_eq!(first.role(), Role::TreeItem);
        assert_eq!(
            first.label(),
            Some("Inicio"),
            "the badge is not in the name"
        );
        assert_eq!(first.is_selected(), Some(false));
        assert!(first.children().is_empty());
        let second = node(&update, node_id(rows[1]));
        assert_eq!(second.is_selected(), Some(true));
        assert_eq!(second.author_id(), Some("nav.1"));
        let mut headings = Vec::new();
        for (_, n) in &update.nodes {
            if n.role() == Role::Heading {
                headings.push(n.value().map(str::to_owned));
            }
        }
        assert_eq!(headings, [Some("Demo".to_owned())]);
    }

    #[test]
    fn dialogs_are_modal_and_named_after_their_title() {
        let mut tree: Element<Msg> = div().child(
            crate::components::dialog("Confirmar", crate::element::text("¿Seguro?"), Vec::new())
                .test_id("d"),
        );
        tree.assign_ids();
        let panel = tree.find_test_id("d.panel").unwrap().id;
        let layout = layout(&tree, WINDOW, &ScrollOffsets::new());
        let update = a11y_tree(&snapshot(&layout, None));
        let dialog = node(&update, node_id(panel));
        assert_eq!(dialog.role(), Role::Dialog);
        assert_eq!(dialog.label(), Some("Confirmar"));
        assert!(dialog.is_modal());
    }

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
            div()
                .role(crate::element::Role::Button)
                .label("Ver")
                .pressed(true)
                .on_click(Msg::Clicked),
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
            &[
                node_id(ids[1]),
                node_id(ids[2]),
                node_id(ids[3]),
                node_id(ids[4])
            ],
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

        let toggle = node(&update, node_id(ids[4]));
        assert_eq!(toggle.role(), Role::Button);
        assert_eq!(toggle.toggled(), Some(accesskit::Toggled::True));
    }

    #[test]
    fn default_button_is_named_after_its_text() {
        let mut tree: Element<Msg> = div().children([
            div()
                .default_button(true)
                .on_click(Msg::Clicked)
                .child(crate::element::text("Enviar")),
            div()
                .role(crate::element::Role::Button)
                .default_button(true)
                .on_click(Msg::Clicked)
                .child(crate::element::text("Guardar")),
        ]);
        tree.assign_ids();
        let layout = layout(&tree, WINDOW, &ScrollOffsets::new());
        let update = a11y_tree(&snapshot(&layout, None));
        for (child, name) in tree.children.iter().zip(["Enviar", "Guardar"]) {
            let button = node(&update, node_id(child.id));
            assert_eq!(button.role(), Role::DefaultButton);
            assert_eq!(button.label(), Some(name));
            assert!(
                button.children().is_empty(),
                "{name}: text folds into the name"
            );
        }
    }

    #[test]
    fn control_state_default_button_and_shortcuts_are_exposed() {
        use crate::element::{ControlState, KeyEquivalent};
        use crate::ui_engine::Key;

        let mut tree: Element<Msg> = div().children([
            div()
                .label("Mixto")
                .state(ControlState::Mixed)
                .on_click(Msg::Clicked),
            div().label("Apagado").pressed(false).on_click(Msg::Clicked),
            div()
                .label("Guardar")
                .default_button(true)
                .on_click(Msg::Clicked),
            div()
                .label("Exportar")
                .key_equivalent(KeyEquivalent::cmd(Key::Char('e')))
                .on_click(Msg::Clicked),
        ]);
        tree.assign_ids();
        let ids: Vec<WidgetId> = tree.children.iter().map(|child| child.id).collect();
        let layout = layout(&tree, WINDOW, &ScrollOffsets::new());
        let update = a11y_tree(&snapshot(&layout, None));
        let mixed = node(&update, node_id(ids[0]));
        assert_eq!(mixed.toggled(), Some(accesskit::Toggled::Mixed));
        let off = node(&update, node_id(ids[1]));
        assert_eq!(off.toggled(), Some(accesskit::Toggled::False));
        let default = node(&update, node_id(ids[2]));
        assert_eq!(default.role(), Role::DefaultButton);
        assert_eq!(default.toggled(), None);
        assert_eq!(default.keyboard_shortcut(), Some("↩"));
        let export = node(&update, node_id(ids[3]));
        assert_eq!(export.role(), Role::Button);
        assert_eq!(export.keyboard_shortcut(), Some("⌘E"));
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
