//! Transaction category tag model.

use serde::{Deserialize, Serialize};

use super::{TagId, UserId};

/// A transaction category tag with optional hierarchy.
///
/// Tags can be nested one level deep via the `parent` field.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(
    clippy::struct_excessive_bools,
    reason = "matches ZenMoney API schema which has multiple boolean flags"
)]
pub struct Tag {
    /// Unique identifier (UUID).
    pub id: TagId,
    /// Last modification timestamp (Unix seconds).
    pub changed: i64,
    /// Owner user identifier.
    pub user: UserId,
    /// Display name.
    pub title: String,
    /// Parent tag identifier (max 1 level nesting).
    pub parent: Option<TagId>,
    /// Icon identifier.
    pub icon: Option<String>,
    /// Picture URL.
    pub picture: Option<String>,
    /// Color in ARGB format.
    pub color: Option<i64>,
    /// Whether to show in income reports.
    pub show_income: bool,
    /// Whether to show in outcome reports.
    pub show_outcome: bool,
    /// Whether to include in income budgets.
    pub budget_income: bool,
    /// Whether to include in outcome budgets.
    pub budget_outcome: bool,
    /// Whether the tag is required for transactions (defaults to true if
    /// null).
    pub required: Option<bool>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deserialize_tag() {
        let json = r#"{
            "id": "tag-001",
            "changed": 1700000000,
            "user": 123,
            "title": "Groceries",
            "parent": null,
            "icon": "food",
            "picture": null,
            "color": -16711936,
            "showIncome": false,
            "showOutcome": true,
            "budgetIncome": false,
            "budgetOutcome": true,
            "required": null
        }"#;
        let tag: Tag = serde_json::from_str(json).unwrap();
        assert_eq!(tag.id, TagId::new("tag-001".to_owned()));
        assert_eq!(tag.title, "Groceries");
        assert!(tag.parent.is_none());
        assert!(tag.show_outcome);
        assert!(tag.required.is_none());
    }

    #[test]
    fn deserialize_tag_with_parent() {
        let json = r#"{
            "id": "tag-002",
            "changed": 1700000000,
            "user": 123,
            "title": "Fast Food",
            "parent": "tag-001",
            "icon": null,
            "picture": null,
            "color": null,
            "showIncome": false,
            "showOutcome": true,
            "budgetIncome": false,
            "budgetOutcome": true,
            "required": true
        }"#;
        let tag: Tag = serde_json::from_str(json).unwrap();
        assert_eq!(tag.parent, Some(TagId::new("tag-001".to_owned())));
        assert_eq!(tag.required, Some(true));
    }

    #[test]
    fn serialize_roundtrip() {
        let tag = Tag {
            id: TagId::new("t-1".to_owned()),
            changed: 1_700_000_000,
            user: UserId::new(1),
            title: "Test".to_owned(),
            parent: None,
            icon: None,
            picture: None,
            color: None,
            show_income: true,
            show_outcome: true,
            budget_income: false,
            budget_outcome: false,
            required: Some(false),
        };
        let json = serde_json::to_string(&tag).unwrap();
        let deserialized: Tag = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, tag);
    }
}
