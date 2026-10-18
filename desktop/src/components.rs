//! Kai components as element builders, styled with the same classes and
//! tokens as the web package (`web/src/components`).
//!
//! Components are plain functions returning [`Element`]s, so they compose
//! like JSX: pass children in, chain `.on_click(..)`, `.disabled(..)` or
//! extra `.class(..)` on the result. State lives in the app model; the only
//! framework-owned state is caret/selection inside inputs.

use crate::element::{Element, Kind, Role, div, input, text};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ButtonVariant {
    /// `.fs-button--contained-primary`.
    Primary,
    Secondary,
    /// `.fs-button--outlined`.
    Outlined,
    /// `.fs-button--text`.
    Text,
    /// `.fs-button--contained-danger`.
    Danger,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ButtonSize {
    Sm,
    Md,
    Lg,
}

/// `.fs-button`: medium weight, `text-sm`, `leading-5`, `rounded-md`, 1px
/// transparent border. Focus shows a ring (the web relies on the browser
/// outline); disabled fades (the web only changes the cursor).
const BUTTON_BASE: &str = "inline-flex items-center justify-center gap-2 rounded-md border \
     border-transparent font-medium whitespace-nowrap cursor-pointer focus:ring-2 \
     focus:ring-primary/40 focus:ring-offset-2 disabled:opacity-50 disabled:cursor-not-allowed";

pub fn button_classes(variant: ButtonVariant, size: ButtonSize) -> String {
    let size = match size {
        ButtonSize::Sm => "px-3 py-1.5 text-sm leading-5",
        ButtonSize::Md => "px-4 h-control text-sm leading-5",
        ButtonSize::Lg => "px-6 py-3 text-lg",
    };
    let variant = match variant {
        ButtonVariant::Primary => "bg-primary text-white hover:bg-accent active:bg-active/90",
        ButtonVariant::Secondary => {
            "bg-secondary text-foreground hover:bg-secondary/80 active:bg-secondary/70"
        }
        ButtonVariant::Outlined => {
            "bg-transparent border-foreground text-foreground hover:bg-accent \
             hover:border-accent hover:text-white active:bg-active"
        }
        ButtonVariant::Text => {
            "bg-transparent text-primary hover:bg-primary/10 active:bg-primary/20"
        }
        ButtonVariant::Danger => "bg-error text-white hover:bg-error/80 active:bg-error/70",
    };
    format!("{BUTTON_BASE} {size} {variant}")
}

/// Medium button; attach behavior with `.on_click(msg)`.
pub fn button<Msg>(label: impl Into<String>, variant: ButtonVariant) -> Element<Msg> {
    button_sized(label, variant, ButtonSize::Md)
}

pub fn button_sized<Msg>(
    label: impl Into<String>,
    variant: ButtonVariant,
    size: ButtonSize,
) -> Element<Msg> {
    div()
        .class(&button_classes(variant, size))
        .role(Role::Button)
        .child(text(label))
}

/// `textfield.css` input: `rounded-md`, 1px `border-border`, background,
/// `text-sm` / `leading-5`, `px-3`, control height; primary border on focus
/// and no shadow. Controlled: wire `.on_input(..)` back into the model.
pub const TEXT_INPUT_CLASSES: &str = "w-full h-control rounded-md border border-border \
     bg-background px-3 text-sm leading-5 text-foreground cursor-text \
     placeholder-muted-foreground hover:border-muted-foreground focus:border-primary \
     disabled:bg-neutral disabled:cursor-not-allowed disabled:opacity-70";

pub fn text_input<Msg>(value: impl Into<String>, placeholder: impl Into<String>) -> Element<Msg> {
    input(value, placeholder)
        .class(TEXT_INPUT_CLASSES)
        .role(Role::TextInput)
}

/// Text field with the web's floating label: the label sits on the top
/// border while the field is focused or has a value; otherwise the
/// placeholder (defaulting to the label) is shown.
pub fn text_field<Msg>(label: &str, mut field: Element<Msg>) -> Element<Msg> {
    let has_value = match &mut field.kind {
        Kind::Input { value, placeholder } => {
            if placeholder.is_empty() {
                label.clone_into(placeholder);
            }
            !value.is_empty()
        }
        _ => false,
    };
    if field.label.is_none() {
        field.label = Some(label.to_owned());
    }
    let visibility = if has_value {
        "block"
    } else {
        "hidden group-focus-within:block"
    };
    div()
        .class("group relative flex flex-col w-full")
        .children([
            field.class("focus:placeholder-transparent"),
            div()
                .class(&format!(
                    "absolute left-2 -top-2 z-20 px-1 rounded-md bg-background pointer-events-none \
                 {visibility}"
                ))
                .child(text(label).class("text-xs leading-4 font-medium text-foreground")),
        ])
}

/// `.fs-card`.
pub fn card<Msg>() -> Element<Msg> {
    div()
        .class(
            "flex flex-col bg-background border border-border/70 rounded-lg shadow-sm \
             overflow-hidden",
        )
        .role(Role::Group)
}

/// `.fs-card__header` with `.fs-card__title` and `.fs-card__subtitle`.
pub fn card_header<Msg>(title: &str, subtitle: Option<&str>) -> Element<Msg> {
    let mut header = div().class("flex flex-col gap-0.5 px-4 pt-4 pb-3").child(
        text(title)
            .class("text-lg leading-[1.4] font-semibold text-foreground")
            .role(Role::Heading),
    );
    if let Some(subtitle) = subtitle {
        header = header.child(text(subtitle).class("text-sm leading-5 text-muted-foreground"));
    }
    header
}

/// `.fs-card__content`.
pub fn card_content<Msg>() -> Element<Msg> {
    div().class("flex flex-col gap-3 p-4")
}

/// `.fs-card__actions`.
pub fn card_actions<Msg>() -> Element<Msg> {
    div().class(
        "flex flex-wrap items-center justify-end gap-2 border-t border-border/70 px-4 pt-3 pb-4",
    )
}

/// Compact statistics tile (`StatisticsCard`).
pub fn stat<Msg>(label: &str, value: impl Into<String>, hint: Option<&str>) -> Element<Msg> {
    let mut tile = card().class("p-4 gap-1").children([
        text(label).class("text-sm leading-5 font-medium text-muted-foreground"),
        text(value).class("text-2xl font-semibold text-foreground"),
    ]);
    if let Some(hint) = hint {
        tile = tile.child(text(hint).class("text-xs text-muted-foreground"));
    }
    tile
}

pub fn heading<Msg>(content: impl Into<String>) -> Element<Msg> {
    text(content)
        .class("text-xl leading-tight font-semibold text-foreground")
        .role(Role::Heading)
}

pub fn paragraph<Msg>(content: impl Into<String>) -> Element<Msg> {
    text(content)
        .class("text-base text-foreground")
        .role(Role::Paragraph)
}

pub fn caption<Msg>(content: impl Into<String>) -> Element<Msg> {
    text(content).class("text-sm text-muted-foreground")
}

#[cfg(test)]
mod tests {
    use super::{
        ButtonSize, ButtonVariant, TEXT_INPUT_CLASSES, button, button_classes, card, card_actions,
        card_content, card_header, stat, text_field, text_input,
    };
    use crate::element::{Element, Kind};
    use crate::theme::{Density, Theme};
    use crate::tw::{ClassList, State};

    #[derive(Clone, Debug)]
    enum Msg {}

    fn assert_known(classes: &str) {
        let list = ClassList::parse(classes, &Theme::kai(Density::Comfortable));
        assert!(list.unknown.is_empty(), "{classes}: {:?}", list.unknown);
    }

    fn all_classes(element: &Element<Msg>, out: &mut Vec<String>) {
        element.walk(&mut |element| out.push(element.class.clone()));
    }

    #[test]
    fn every_component_class_is_supported() {
        for variant in [
            ButtonVariant::Primary,
            ButtonVariant::Secondary,
            ButtonVariant::Outlined,
            ButtonVariant::Text,
            ButtonVariant::Danger,
        ] {
            for size in [ButtonSize::Sm, ButtonSize::Md, ButtonSize::Lg] {
                assert_known(&button_classes(variant, size));
            }
        }
        assert_known(TEXT_INPUT_CLASSES);
        let mut classes = Vec::new();
        let tree: Element<Msg> = card().children([
            card_header("Título", Some("Subtítulo")),
            card_content().child(text_field("Nombre", text_input("", ""))),
            card_actions().child(button("Ok", ButtonVariant::Primary)),
            stat("Clics", "3", Some("hoy")),
        ]);
        all_classes(&tree, &mut classes);
        for class in classes {
            assert_known(&class);
        }
    }

    #[test]
    fn primary_button_matches_web_tokens_in_each_state() {
        let theme = Theme::kai(Density::Comfortable);
        let list = ClassList::parse(
            &button_classes(ButtonVariant::Primary, ButtonSize::Md),
            &theme,
        );
        let idle = list.resolve(State::default());
        assert_eq!(idle.background, Some(theme.colors.primary));
        assert_eq!(idle.radius, 6.0);
        assert_eq!(idle.text.font_size, Some(theme.typography.font_size_sm));
        assert_eq!(idle.border_width.top, 1.0);
        let hover = list.resolve(State {
            hover: true,
            ..State::default()
        });
        assert_eq!(hover.background, Some(theme.colors.accent));
        let focus = list.resolve(State {
            focus: true,
            ..State::default()
        });
        assert_eq!((focus.ring.width, focus.ring.offset), (2.0, 2.0));
    }

    #[test]
    fn text_field_floats_its_label_only_with_value_or_focus() {
        let theme = Theme::kai(Density::Comfortable);
        let empty: Element<Msg> = text_field("Nombre", text_input("", ""));
        let label = &empty.children[1];
        let list = ClassList::parse(&label.class, &theme);
        assert_eq!(
            list.resolve(State::default()).display,
            crate::style::Display::None
        );
        assert_eq!(
            list.resolve(State {
                group_focus_within: true,
                ..State::default()
            })
            .display,
            crate::style::Display::Block
        );
        assert!(matches!(
            &empty.children[0].kind,
            Kind::Input { placeholder, .. } if placeholder == "Nombre"
        ));
        assert_eq!(empty.children[0].label.as_deref(), Some("Nombre"));

        let filled: Element<Msg> = text_field("Nombre", text_input("Ana", "Tu nombre"));
        let list = ClassList::parse(&filled.children[1].class, &theme);
        assert_eq!(
            list.resolve(State::default()).display,
            crate::style::Display::Block
        );
    }
}
