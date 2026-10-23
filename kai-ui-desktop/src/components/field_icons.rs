//! Documented map from field role → Lucide icon for form adornments.
//!
//! Keeps demo and apps from pairing the wrong glyph (e.g. a search glass on
//! an email field). See `docs/dev/19-iconografia-campos.md`.

use crate::icons::Icon;

/// Semantic kind of a text field, for icon resolution.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FieldKind {
    /// Person / display name.
    Name,
    /// Email address.
    Email,
    /// Secret value (password, PIN, token).
    Password,
    /// Free-text search / filter.
    Search,
    /// Phone number.
    Phone,
    /// Website or URL (no dedicated globe in catalog → use Mail's peer `Hash`
    /// is wrong; prefer no icon or a custom SVG — documented as `None` helper).
    Url,
    /// Generic user account / handle.
    User,
    /// Calendar / date entry (no date field yet; map reserved).
    Date,
    /// Currency / amount (use a symbol adornment, not an icon, by default).
    Amount,
}

/// Canonical Lucide icon for `kind`, when the catalog has one.
///
/// Returns `None` for kinds that should use a text symbol (`$`, `%`, `https://`)
/// or a custom SVG rather than a misleading catalog icon.
pub fn field_icon_for(kind: FieldKind) -> Option<Icon> {
    Some(match kind {
        FieldKind::Name | FieldKind::User => Icon::User,
        FieldKind::Email => Icon::Mail,
        FieldKind::Password => Icon::Lock,
        FieldKind::Search => Icon::Search,
        FieldKind::Phone => Icon::Phone,
        FieldKind::Date => Icon::CalendarDays,
        FieldKind::Url | FieldKind::Amount => return None,
    })
}

/// Stable table for docs and tests: `(kind, lucide name or "—")`.
pub const FIELD_ICON_MAP: &[(FieldKind, &str)] = &[
    (FieldKind::Name, "user"),
    (FieldKind::User, "user"),
    (FieldKind::Email, "mail"),
    (FieldKind::Password, "lock"),
    (FieldKind::Search, "search"),
    (FieldKind::Phone, "phone"),
    (FieldKind::Date, "calendar-days"),
    (FieldKind::Url, "—"),
    (FieldKind::Amount, "—"),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn map_matches_field_icon_for() {
        for &(kind, name) in FIELD_ICON_MAP {
            match field_icon_for(kind) {
                Some(icon) => assert_eq!(icon.name(), name, "{kind:?}"),
                None => assert_eq!(name, "—", "{kind:?}"),
            }
        }
    }

    #[test]
    fn email_is_not_a_search_glass() {
        assert_eq!(field_icon_for(FieldKind::Email), Some(Icon::Mail));
        assert_ne!(field_icon_for(FieldKind::Email), Some(Icon::Search));
    }
}
