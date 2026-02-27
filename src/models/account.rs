//! Financial account model.

use serde::{Deserialize, Serialize};

use super::{AccountId, AccountType, CompanyId, InstrumentId, PayoffInterval, UserId};

/// A user's financial account (bank account, credit card, cash, etc.).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(
    clippy::struct_excessive_bools,
    reason = "matches ZenMoney API schema which has multiple boolean flags"
)]
pub struct Account {
    /// Unique identifier (UUID).
    pub id: AccountId,
    /// Last modification timestamp (Unix seconds).
    pub changed: i64,
    /// Owner user identifier.
    pub user: UserId,
    /// Role user identifier (for shared accounts).
    pub role: Option<UserId>,
    /// Currency instrument identifier.
    pub instrument: Option<InstrumentId>,
    /// Associated financial company.
    pub company: Option<CompanyId>,
    /// Type of account.
    #[serde(rename = "type")]
    pub kind: AccountType,
    /// Display name.
    pub title: String,
    /// Bank account identifiers for SMS recognition.
    #[serde(rename = "syncID")]
    pub sync_id: Option<Vec<String>>,
    /// Current balance.
    pub balance: Option<f64>,
    /// Initial balance when the account was created.
    pub start_balance: Option<f64>,
    /// Credit limit (>= 0).
    pub credit_limit: Option<f64>,
    /// Whether to include in total balance calculation.
    pub in_balance: bool,
    /// Whether this is a savings account.
    pub savings: Option<bool>,
    /// Enable automatic balance correction from SMS.
    pub enable_correction: bool,
    /// Enable SMS transaction recognition.
    #[serde(rename = "enableSMS")]
    pub enable_sms: bool,
    /// Whether the account is archived.
    pub archive: bool,
    /// Whether interest is capitalized (deposits/loans).
    pub capitalization: Option<bool>,
    /// Interest rate percentage (>= 0, < 100).
    pub percent: Option<f64>,
    /// Start date of the deposit/loan (yyyy-MM-dd).
    pub start_date: Option<String>,
    /// End date offset from start.
    pub end_date_offset: Option<i32>,
    /// Unit for end date offset.
    pub end_date_offset_interval: Option<PayoffInterval>,
    /// Repayment step count.
    pub payoff_step: Option<i32>,
    /// Repayment interval unit.
    pub payoff_interval: Option<PayoffInterval>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deserialize_checking_account() {
        let json = r#"{
            "id": "a1b2c3d4-0000-0000-0000-000000000001",
            "changed": 1700000000,
            "user": 123,
            "role": null,
            "instrument": 1,
            "company": 4,
            "type": "checking",
            "title": "Main Account",
            "syncID": ["1234", "5678"],
            "balance": 50000.0,
            "startBalance": 0.0,
            "creditLimit": null,
            "inBalance": true,
            "savings": false,
            "enableCorrection": false,
            "enableSMS": true,
            "archive": false,
            "capitalization": null,
            "percent": null,
            "startDate": null,
            "endDateOffset": null,
            "endDateOffsetInterval": null,
            "payoffStep": null,
            "payoffInterval": null
        }"#;
        let account: Account = serde_json::from_str(json).unwrap();
        assert_eq!(
            account.id,
            AccountId::new("a1b2c3d4-0000-0000-0000-000000000001".to_owned())
        );
        assert_eq!(account.kind, AccountType::Checking);
        assert_eq!(account.title, "Main Account");
        assert!(account.enable_sms);
        assert!(!account.archive);
    }

    #[test]
    fn deserialize_credit_card() {
        let json = r#"{
            "id": "a1b2c3d4-0000-0000-0000-000000000002",
            "changed": 1700000000,
            "user": 123,
            "role": null,
            "instrument": 1,
            "company": 4,
            "type": "ccard",
            "title": "Credit Card",
            "syncID": null,
            "balance": -15000.0,
            "startBalance": null,
            "creditLimit": 100000.0,
            "inBalance": true,
            "savings": null,
            "enableCorrection": false,
            "enableSMS": false,
            "archive": false,
            "capitalization": null,
            "percent": null,
            "startDate": null,
            "endDateOffset": null,
            "endDateOffsetInterval": null,
            "payoffStep": null,
            "payoffInterval": null
        }"#;
        let account: Account = serde_json::from_str(json).unwrap();
        assert_eq!(account.kind, AccountType::CreditCard);
        assert!((account.credit_limit.unwrap() - 100_000.0).abs() < f64::EPSILON);
    }

    #[test]
    fn deserialize_deposit_account() {
        let json = r#"{
            "id": "a1b2c3d4-0000-0000-0000-000000000003",
            "changed": 1700000000,
            "user": 123,
            "role": null,
            "instrument": 1,
            "company": 4,
            "type": "deposit",
            "title": "Savings Deposit",
            "syncID": null,
            "balance": 200000.0,
            "startBalance": 100000.0,
            "creditLimit": null,
            "inBalance": true,
            "savings": true,
            "enableCorrection": false,
            "enableSMS": false,
            "archive": false,
            "capitalization": true,
            "percent": 7.5,
            "startDate": "2024-01-01",
            "endDateOffset": 12,
            "endDateOffsetInterval": "month",
            "payoffStep": 1,
            "payoffInterval": "month"
        }"#;
        let account: Account = serde_json::from_str(json).unwrap();
        assert_eq!(account.kind, AccountType::Deposit);
        assert_eq!(account.capitalization, Some(true));
        assert!((account.percent.unwrap() - 7.5).abs() < f64::EPSILON);
        assert_eq!(account.start_date.as_deref(), Some("2024-01-01"));
        assert_eq!(account.end_date_offset, Some(12));
        assert_eq!(
            account.end_date_offset_interval,
            Some(PayoffInterval::Month)
        );
    }

    #[test]
    fn serialize_roundtrip() {
        let account = Account {
            id: AccountId::new("test-id".to_owned()),
            changed: 1_700_000_000,
            user: UserId::new(1),
            role: None,
            instrument: Some(InstrumentId::new(1)),
            company: None,
            kind: AccountType::Cash,
            title: "Cash".to_owned(),
            sync_id: None,
            balance: Some(1000.0),
            start_balance: None,
            credit_limit: None,
            in_balance: true,
            savings: None,
            enable_correction: false,
            enable_sms: false,
            archive: false,
            capitalization: None,
            percent: None,
            start_date: None,
            end_date_offset: None,
            end_date_offset_interval: None,
            payoff_step: None,
            payoff_interval: None,
        };
        let json = serde_json::to_string(&account).unwrap();
        let deserialized: Account = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, account);
    }
}
