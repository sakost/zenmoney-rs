//! Monthly budget model.

use serde::{Deserialize, Serialize};

use super::{TagId, UserId};

/// A monthly income/outcome budget target for a category.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Budget {
    /// Last modification timestamp (Unix seconds).
    pub changed: i64,
    /// Owner user identifier.
    pub user: UserId,
    /// Category tag (null or special UUID
    /// "00000000-0000-0000-0000-000000000000" for aggregate).
    pub tag: Option<TagId>,
    /// Budget month start date (yyyy-MM-dd).
    pub date: String,
    /// Income target amount.
    pub income: f64,
    /// Whether the income value is manually locked.
    pub income_lock: bool,
    /// Outcome target amount.
    pub outcome: f64,
    /// Whether the outcome value is manually locked.
    pub outcome_lock: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deserialize_budget() {
        let json = r#"{
            "changed": 1700000000,
            "user": 123,
            "tag": "tag-groceries",
            "date": "2024-01-01",
            "income": 0,
            "incomeLock": false,
            "outcome": 30000.0,
            "outcomeLock": true
        }"#;
        let budget: Budget = serde_json::from_str(json).unwrap();
        assert_eq!(budget.tag, Some(TagId::new("tag-groceries".to_owned())));
        assert!((budget.outcome - 30_000.0).abs() < f64::EPSILON);
        assert!(budget.outcome_lock);
        assert!(!budget.income_lock);
    }

    #[test]
    fn deserialize_aggregate_budget() {
        let json = r#"{
            "changed": 1700000000,
            "user": 123,
            "tag": null,
            "date": "2024-01-01",
            "income": 100000.0,
            "incomeLock": true,
            "outcome": 80000.0,
            "outcomeLock": true
        }"#;
        let budget: Budget = serde_json::from_str(json).unwrap();
        assert!(budget.tag.is_none());
    }

    #[test]
    fn serialize_roundtrip() {
        let budget = Budget {
            changed: 1_700_000_000,
            user: UserId::new(1),
            tag: Some(TagId::new("t-1".to_owned())),
            date: "2024-01-01".to_owned(),
            income: 0.0,
            income_lock: false,
            outcome: 5000.0,
            outcome_lock: false,
        };
        let json = serde_json::to_string(&budget).unwrap();
        let deserialized: Budget = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, budget);
    }
}
