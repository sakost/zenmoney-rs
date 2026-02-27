//! Reminder marker (instance) model.

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};

use super::{
    AccountId, InstrumentId, MerchantId, ReminderId, ReminderMarkerId, ReminderMarkerState, TagId,
    UserId,
};

/// A generated instance of a recurring reminder.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReminderMarker {
    /// Unique identifier (UUID).
    pub id: ReminderMarkerId,
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
    /// Scheduled date.
    pub date: NaiveDate,
    /// Parent reminder identifier.
    pub reminder: ReminderId,
    /// Current state of this marker.
    pub state: ReminderMarkerState,
    /// Whether to send a notification.
    pub notify: bool,
    /// Whether this marker is a forecast entry.
    #[serde(default)]
    pub is_forecast: Option<bool>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deserialize_planned_marker() {
        let json = r#"{
            "id": "rm-001",
            "changed": 1700000000,
            "user": 123,
            "incomeInstrument": 1,
            "incomeAccount": "acc-001",
            "income": 0,
            "outcomeInstrument": 1,
            "outcomeAccount": "acc-001",
            "outcome": 5000.0,
            "tag": null,
            "merchant": null,
            "payee": null,
            "comment": null,
            "date": "2024-02-01",
            "reminder": "rem-001",
            "state": "planned",
            "notify": true
        }"#;
        let marker: ReminderMarker = serde_json::from_str(json).unwrap();
        assert_eq!(marker.state, ReminderMarkerState::Planned);
        assert_eq!(marker.reminder, ReminderId::new("rem-001".to_owned()));
        assert_eq!(marker.date, NaiveDate::from_ymd_opt(2024, 2, 1).unwrap());
    }

    #[test]
    fn deserialize_processed_marker() {
        let json = r#"{
            "id": "rm-002",
            "changed": 1700000000,
            "user": 123,
            "incomeInstrument": 1,
            "incomeAccount": "acc-001",
            "income": 0,
            "outcomeInstrument": 1,
            "outcomeAccount": "acc-001",
            "outcome": 5000.0,
            "tag": ["tag-001"],
            "merchant": "m-001",
            "payee": "Landlord",
            "comment": "Rent paid",
            "date": "2024-01-01",
            "reminder": "rem-001",
            "state": "processed",
            "notify": false
        }"#;
        let marker: ReminderMarker = serde_json::from_str(json).unwrap();
        assert_eq!(marker.state, ReminderMarkerState::Processed);
        assert!(!marker.notify);
    }

    #[test]
    fn serialize_roundtrip() {
        let marker = ReminderMarker {
            id: ReminderMarkerId::new("rm-1".to_owned()),
            changed: DateTime::from_timestamp(1_700_000_000, 0).unwrap(),
            user: UserId::new(1),
            income_instrument: InstrumentId::new(1),
            income_account: AccountId::new("a-1".to_owned()),
            income: 0.0,
            outcome_instrument: InstrumentId::new(1),
            outcome_account: AccountId::new("a-1".to_owned()),
            outcome: 100.0,
            tag: None,
            merchant: None,
            payee: None,
            comment: None,
            date: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            reminder: ReminderId::new("r-1".to_owned()),
            state: ReminderMarkerState::Deleted,
            notify: false,
            is_forecast: None,
        };
        let json = serde_json::to_string(&marker).unwrap();
        let deserialized: ReminderMarker = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, marker);
    }
}
