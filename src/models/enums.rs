//! Enumeration types for constrained API values.

use serde::{Deserialize, Serialize};

/// Type of a financial account.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AccountType {
    /// Physical cash.
    Cash,
    /// Credit card.
    #[serde(rename = "ccard")]
    CreditCard,
    /// Checking/current account.
    Checking,
    /// Loan account.
    Loan,
    /// Deposit/savings account.
    Deposit,
    /// Electronic money (e-wallet).
    #[serde(rename = "emoney")]
    EMoney,
    /// Debt tracking account.
    Debt,
}

/// Time interval unit used for reminders and account offsets.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Interval {
    /// Daily interval.
    Day,
    /// Weekly interval.
    Week,
    /// Monthly interval.
    Month,
    /// Yearly interval.
    Year,
}

/// Payoff interval for loan/deposit accounts.
///
/// A subset of [`Interval`] — only month and year are valid for payoff
/// schedules.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PayoffInterval {
    /// Monthly payoff.
    Month,
    /// Yearly payoff.
    Year,
}

/// State of a reminder marker instance.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ReminderMarkerState {
    /// Scheduled but not yet executed.
    Planned,
    /// Has been processed/applied.
    Processed,
    /// Marked as deleted/skipped.
    Deleted,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn account_type_serde_cash() {
        let json = serde_json::to_string(&AccountType::Cash).unwrap();
        assert_eq!(json, r#""cash""#);
        let deserialized: AccountType = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, AccountType::Cash);
    }

    #[test]
    fn account_type_serde_credit_card() {
        let json = serde_json::to_string(&AccountType::CreditCard).unwrap();
        assert_eq!(json, r#""ccard""#);
        let deserialized: AccountType = serde_json::from_str(r#""ccard""#).unwrap();
        assert_eq!(deserialized, AccountType::CreditCard);
    }

    #[test]
    fn account_type_serde_checking() {
        let deserialized: AccountType = serde_json::from_str(r#""checking""#).unwrap();
        assert_eq!(deserialized, AccountType::Checking);
    }

    #[test]
    fn account_type_serde_emoney() {
        let json = serde_json::to_string(&AccountType::EMoney).unwrap();
        assert_eq!(json, r#""emoney""#);
        let deserialized: AccountType = serde_json::from_str(r#""emoney""#).unwrap();
        assert_eq!(deserialized, AccountType::EMoney);
    }

    #[test]
    fn account_type_all_variants_roundtrip() {
        let variants = [
            AccountType::Cash,
            AccountType::CreditCard,
            AccountType::Checking,
            AccountType::Loan,
            AccountType::Deposit,
            AccountType::EMoney,
            AccountType::Debt,
        ];
        for variant in variants {
            let json = serde_json::to_string(&variant).unwrap();
            let deserialized: AccountType = serde_json::from_str(&json).unwrap();
            assert_eq!(deserialized, variant);
        }
    }

    #[test]
    fn interval_serde_roundtrip() {
        let variants = [
            (Interval::Day, r#""day""#),
            (Interval::Week, r#""week""#),
            (Interval::Month, r#""month""#),
            (Interval::Year, r#""year""#),
        ];
        for (variant, expected_json) in variants {
            let json = serde_json::to_string(&variant).unwrap();
            assert_eq!(json, expected_json);
            let deserialized: Interval = serde_json::from_str(&json).unwrap();
            assert_eq!(deserialized, variant);
        }
    }

    #[test]
    fn payoff_interval_serde_roundtrip() {
        let variants = [
            (PayoffInterval::Month, r#""month""#),
            (PayoffInterval::Year, r#""year""#),
        ];
        for (variant, expected_json) in variants {
            let json = serde_json::to_string(&variant).unwrap();
            assert_eq!(json, expected_json);
            let deserialized: PayoffInterval = serde_json::from_str(&json).unwrap();
            assert_eq!(deserialized, variant);
        }
    }

    #[test]
    fn reminder_marker_state_serde_roundtrip() {
        let variants = [
            (ReminderMarkerState::Planned, r#""planned""#),
            (ReminderMarkerState::Processed, r#""processed""#),
            (ReminderMarkerState::Deleted, r#""deleted""#),
        ];
        for (variant, expected_json) in variants {
            let json = serde_json::to_string(&variant).unwrap();
            assert_eq!(json, expected_json);
            let deserialized: ReminderMarkerState = serde_json::from_str(&json).unwrap();
            assert_eq!(deserialized, variant);
        }
    }

    #[test]
    fn invalid_account_type_fails() {
        let result = serde_json::from_str::<AccountType>(r#""invalid""#);
        assert!(result.is_err());
    }

    #[test]
    fn invalid_interval_fails() {
        let result = serde_json::from_str::<Interval>(r#""hourly""#);
        assert!(result.is_err());
    }
}
