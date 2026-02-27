//! Monthly budget model.

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};

use super::{TagId, UserId};

/// A monthly income/outcome budget target for a category.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Budget {
    /// Last modification timestamp.
    #[serde(with = "chrono::serde::ts_seconds")]
    pub changed: DateTime<Utc>,
    /// Owner user identifier.
    pub user: UserId,
    /// Category tag (null or special UUID
    /// "00000000-0000-0000-0000-000000000000" for aggregate).
    pub tag: Option<TagId>,
    /// Budget month start date.
    pub date: NaiveDate,
    /// Income target amount.
    pub income: f64,
    /// Whether the income value is manually locked.
    pub income_lock: bool,
    /// Outcome target amount.
    pub outcome: f64,
    /// Whether the outcome value is manually locked.
    pub outcome_lock: bool,
    /// Whether the income value is a forecast.
    #[serde(default)]
    pub is_income_forecast: Option<bool>,
    /// Whether the outcome value is a forecast.
    #[serde(default)]
    pub is_outcome_forecast: Option<bool>,
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
            changed: DateTime::from_timestamp(1_700_000_000, 0).unwrap(),
            user: UserId::new(1),
            tag: Some(TagId::new("t-1".to_owned())),
            date: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            income: 0.0,
            income_lock: false,
            outcome: 5000.0,
            outcome_lock: false,
            is_income_forecast: None,
            is_outcome_forecast: None,
        };
        let json = serde_json::to_string(&budget).unwrap();
        let deserialized: Budget = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, budget);
    }
}
