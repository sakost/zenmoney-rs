//! Transaction model.

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};

use super::{AccountId, InstrumentId, MerchantId, ReminderMarkerId, TagId, TransactionId, UserId};

/// A financial transaction between accounts.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Transaction {
    /// Unique identifier (UUID).
    pub id: TransactionId,
    /// Last modification timestamp.
    #[serde(with = "chrono::serde::ts_seconds")]
    pub changed: DateTime<Utc>,
    /// Creation timestamp.
    #[serde(with = "chrono::serde::ts_seconds")]
    pub created: DateTime<Utc>,
    /// Owner user identifier.
    pub user: UserId,
    /// Whether the transaction has been deleted.
    pub deleted: bool,
    /// Whether the transaction is on hold (pending).
    pub hold: Option<bool>,
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
    /// Original payee name (before normalization).
    pub original_payee: Option<String>,
    /// User comment.
    pub comment: Option<String>,
    /// Transaction date.
    pub date: NaiveDate,
    /// Merchant Category Code.
    pub mcc: Option<i32>,
    /// Associated reminder marker.
    pub reminder_marker: Option<ReminderMarkerId>,
    /// Operational income amount (in transaction currency).
    pub op_income: Option<f64>,
    /// Operational income instrument.
    pub op_income_instrument: Option<InstrumentId>,
    /// Operational outcome amount (in transaction currency).
    pub op_outcome: Option<f64>,
    /// Operational outcome instrument.
    pub op_outcome_instrument: Option<InstrumentId>,
    /// Latitude coordinate.
    pub latitude: Option<f64>,
    /// Longitude coordinate.
    pub longitude: Option<f64>,
    /// Income bank transaction identifier.
    #[serde(default, rename = "incomeBankID")]
    pub income_bank_id: Option<String>,
    /// Outcome bank transaction identifier.
    #[serde(default, rename = "outcomeBankID")]
    pub outcome_bank_id: Option<String>,
    /// QR code data.
    #[serde(default)]
    pub qr_code: Option<String>,
    /// Transaction source (e.g. "import", "user").
    #[serde(default)]
    pub source: Option<String>,
    /// Whether the transaction has been viewed.
    #[serde(default)]
    pub viewed: Option<bool>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deserialize_simple_transaction() {
        let json = r#"{
            "id": "tx-001",
            "changed": 1700000000,
            "created": 1700000000,
            "user": 123,
            "deleted": false,
            "hold": null,
            "incomeInstrument": 1,
            "incomeAccount": "acc-001",
            "income": 0,
            "outcomeInstrument": 1,
            "outcomeAccount": "acc-001",
            "outcome": 500.0,
            "tag": ["tag-001"],
            "merchant": "merchant-001",
            "payee": "Coffee Shop",
            "originalPayee": "COFFEE SHOP LLC",
            "comment": "Morning coffee",
            "date": "2024-01-15",
            "mcc": 5812,
            "reminderMarker": null,
            "opIncome": null,
            "opIncomeInstrument": null,
            "opOutcome": null,
            "opOutcomeInstrument": null,
            "latitude": 55.7558,
            "longitude": 37.6173
        }"#;
        let tx: Transaction = serde_json::from_str(json).unwrap();
        assert_eq!(tx.id, TransactionId::new("tx-001".to_owned()));
        assert!(!tx.deleted);
        assert!((tx.outcome - 500.0).abs() < f64::EPSILON);
        assert_eq!(tx.date, NaiveDate::from_ymd_opt(2024, 1, 15).unwrap());
        assert_eq!(tx.mcc, Some(5812));
        assert!((tx.latitude.unwrap() - 55.7558).abs() < f64::EPSILON);
    }

    #[test]
    fn deserialize_transfer_with_currency_conversion() {
        let json = r#"{
            "id": "tx-002",
            "changed": 1700000000,
            "created": 1700000000,
            "user": 123,
            "deleted": false,
            "hold": false,
            "incomeInstrument": 2,
            "incomeAccount": "acc-usd",
            "income": 100.0,
            "outcomeInstrument": 1,
            "outcomeAccount": "acc-rub",
            "outcome": 9250.0,
            "tag": null,
            "merchant": null,
            "payee": null,
            "originalPayee": null,
            "comment": "Currency exchange",
            "date": "2024-01-15",
            "mcc": null,
            "reminderMarker": null,
            "opIncome": 100.0,
            "opIncomeInstrument": 2,
            "opOutcome": 9250.0,
            "opOutcomeInstrument": 1,
            "latitude": null,
            "longitude": null
        }"#;
        let tx: Transaction = serde_json::from_str(json).unwrap();
        assert_eq!(tx.income_instrument, InstrumentId::new(2));
        assert_eq!(tx.outcome_instrument, InstrumentId::new(1));
        assert!(tx.op_income.is_some());
        assert_eq!(tx.hold, Some(false));
    }

    #[test]
    fn serialize_roundtrip() {
        let tx = Transaction {
            id: TransactionId::new("t-1".to_owned()),
            changed: DateTime::from_timestamp(1_700_000_000, 0).unwrap(),
            created: DateTime::from_timestamp(1_700_000_000, 0).unwrap(),
            user: UserId::new(1),
            deleted: false,
            hold: None,
            income_instrument: InstrumentId::new(1),
            income_account: AccountId::new("a-1".to_owned()),
            income: 0.0,
            outcome_instrument: InstrumentId::new(1),
            outcome_account: AccountId::new("a-1".to_owned()),
            outcome: 100.0,
            tag: None,
            merchant: None,
            payee: None,
            original_payee: None,
            comment: None,
            date: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            mcc: None,
            reminder_marker: None,
            op_income: None,
            op_income_instrument: None,
            op_outcome: None,
            op_outcome_instrument: None,
            latitude: None,
            longitude: None,
            income_bank_id: None,
            outcome_bank_id: None,
            qr_code: None,
            source: None,
            viewed: None,
        };
        let json = serde_json::to_string(&tx).unwrap();
        let deserialized: Transaction = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, tx);
    }
}
