//! Recurring transaction reminder model.

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};

use super::{AccountId, InstrumentId, Interval, MerchantId, ReminderId, TagId, UserId};

/// A recurring transaction template.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Reminder {
    /// Unique identifier (UUID).
    pub id: ReminderId,
    /// Last modification timestamp.
    #[serde(with = "chrono::serde::ts_seconds")]
    pub changed: DateTime<Utc>,
    /// Owner user identifier.
    pub user: UserId,
    /// Income currency instrument.
    pub income_instrument: InstrumentId,
    /// Income destination account.
    pub income_account: AccountId,
    /// Income amount (>= 0).
    pub income: f64,
    /// Outcome currency instrument.
    pub outcome_instrument: InstrumentId,
    /// Outcome source account.
    pub outcome_account: AccountId,
    /// Outcome amount (>= 0).
    pub outcome: f64,
    /// Associated category tags.
    pub tag: Option<Vec<TagId>>,
    /// Associated merchant.
    pub merchant: Option<MerchantId>,
    /// Payee name.
    pub payee: Option<String>,
    /// User comment.
    pub comment: Option<String>,
    /// Recurrence interval unit.
    pub interval: Option<Interval>,
    /// Recurrence step count.
    pub step: Option<i32>,
    /// Specific day points within the interval.
    pub points: Option<Vec<i32>>,
    /// First occurrence date.
    pub start_date: NaiveDate,
    /// Last occurrence date.
    pub end_date: Option<NaiveDate>,
    /// Whether to send notifications.
    pub notify: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deserialize_monthly_reminder() {
        let json = r#"{
            "id": "rem-001",
            "changed": 1700000000,
            "user": 123,
            "incomeInstrument": 1,
            "incomeAccount": "acc-001",
            "income": 0,
            "outcomeInstrument": 1,
            "outcomeAccount": "acc-001",
            "outcome": 5000.0,
            "tag": ["tag-rent"],
            "merchant": null,
            "payee": "Landlord",
            "comment": "Monthly rent",
            "interval": "month",
            "step": 1,
            "points": [1],
            "startDate": "2024-01-01",
            "endDate": "2025-01-01",
            "notify": true
        }"#;
        let reminder: Reminder = serde_json::from_str(json).unwrap();
        assert_eq!(reminder.id, ReminderId::new("rem-001".to_owned()));
        assert_eq!(reminder.interval, Some(Interval::Month));
        assert_eq!(reminder.step, Some(1));
        assert_eq!(reminder.points, Some(vec![1]));
        assert!(reminder.notify);
    }

    #[test]
    fn deserialize_one_time_reminder() {
        let json = r#"{
            "id": "rem-002",
            "changed": 1700000000,
            "user": 123,
            "incomeInstrument": 1,
            "incomeAccount": "acc-001",
            "income": 0,
            "outcomeInstrument": 1,
            "outcomeAccount": "acc-001",
            "outcome": 1000.0,
            "tag": null,
            "merchant": null,
            "payee": null,
            "comment": null,
            "interval": null,
            "step": null,
            "points": null,
            "startDate": "2024-06-15",
            "endDate": null,
            "notify": false
        }"#;
        let reminder: Reminder = serde_json::from_str(json).unwrap();
        assert!(reminder.interval.is_none());
        assert!(reminder.step.is_none());
        assert!(!reminder.notify);
    }

    #[test]
    fn serialize_roundtrip() {
        let reminder = Reminder {
            id: ReminderId::new("r-1".to_owned()),
            changed: DateTime::from_timestamp(1_700_000_000, 0).unwrap(),
            user: UserId::new(1),
            income_instrument: InstrumentId::new(1),
            income_account: AccountId::new("a-1".to_owned()),
            income: 0.0,
            outcome_instrument: InstrumentId::new(1),
            outcome_account: AccountId::new("a-1".to_owned()),
            outcome: 500.0,
            tag: None,
            merchant: None,
            payee: None,
            comment: None,
            interval: Some(Interval::Week),
            step: Some(2),
            points: Some(vec![0, 1]),
            start_date: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            end_date: None,
            notify: true,
        };
        let json = serde_json::to_string(&reminder).unwrap();
        let deserialized: Reminder = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, reminder);
    }
}
