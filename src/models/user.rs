//! User account model.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::{InstrumentId, UserId};

/// A ZenMoney user account.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct User {
    /// Unique identifier.
    pub id: UserId,
    /// Last modification timestamp.
    #[serde(with = "chrono::serde::ts_seconds")]
    pub changed: DateTime<Utc>,
    /// User login (email or username).
    pub login: Option<String>,
    /// Preferred currency instrument identifier.
    pub currency: InstrumentId,
    /// Parent user identifier (for family/shared accounts).
    pub parent: Option<UserId>,
    /// Country identifier.
    #[serde(default)]
    pub country: Option<i32>,
    /// Two-letter country code.
    #[serde(default)]
    pub country_code: Option<String>,
    /// User email address.
    #[serde(default)]
    pub email: Option<String>,
    /// Whether forecasting is enabled.
    #[serde(default)]
    pub is_forecast_enabled: Option<bool>,
    /// Day of the month when budgets start.
    #[serde(default)]
    pub month_start_day: Option<i32>,
    /// Subscription paid-until timestamp.
    #[serde(default, with = "chrono::serde::ts_seconds_option")]
    pub paid_till: Option<DateTime<Utc>>,
    /// Balance planning mode.
    #[serde(default)]
    pub plan_balance_mode: Option<String>,
    /// Plan settings (opaque string, e.g. JSON array).
    #[serde(default)]
    pub plan_settings: Option<String>,
    /// Subscription plan name.
    #[serde(default)]
    pub subscription: Option<String>,
    /// Subscription renewal timestamp.
    #[serde(default, with = "chrono::serde::ts_seconds_option")]
    pub subscription_renewal_date: Option<DateTime<Utc>>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deserialize_user() {
        let json = r#"{
            "id": 123,
            "changed": 1700000000,
            "login": "user@example.com",
            "currency": 2,
            "parent": null
        }"#;
        let user: User = serde_json::from_str(json).unwrap();
        assert_eq!(user.id, UserId::new(123));
        assert_eq!(user.login.as_deref(), Some("user@example.com"));
        assert_eq!(user.currency, InstrumentId::new(2));
        assert!(user.parent.is_none());
    }

    #[test]
    fn deserialize_user_with_parent() {
        let json = r#"{
            "id": 456,
            "changed": 1700000000,
            "login": null,
            "currency": 1,
            "parent": 123
        }"#;
        let user: User = serde_json::from_str(json).unwrap();
        assert_eq!(user.parent, Some(UserId::new(123)));
        assert!(user.login.is_none());
    }

    #[test]
    fn deserialize_user_with_extra_fields() {
        let json = r#"{
            "id": 789,
            "changed": 1700000000,
            "login": "user@example.com",
            "currency": 2,
            "parent": null,
            "country": 1,
            "countryCode": "RU",
            "email": "user@example.com",
            "isForecastEnabled": true,
            "monthStartDay": 1,
            "paidTill": 1772380966,
            "planBalanceMode": "balance",
            "planSettings": "[]",
            "subscription": "1MonthRenewableSubscription",
            "subscriptionRenewalDate": 1772380938
        }"#;
        let user: User = serde_json::from_str(json).unwrap();
        assert_eq!(user.country, Some(1));
        assert_eq!(user.country_code.as_deref(), Some("RU"));
        assert_eq!(user.email.as_deref(), Some("user@example.com"));
        assert_eq!(user.is_forecast_enabled, Some(true));
        assert_eq!(user.month_start_day, Some(1));
        assert_eq!(
            user.paid_till,
            Some(DateTime::from_timestamp(1_772_380_966, 0).unwrap())
        );
        assert_eq!(user.plan_balance_mode.as_deref(), Some("balance"));
        assert_eq!(user.plan_settings.as_deref(), Some("[]"));
        assert_eq!(
            user.subscription.as_deref(),
            Some("1MonthRenewableSubscription")
        );
        assert_eq!(
            user.subscription_renewal_date,
            Some(DateTime::from_timestamp(1_772_380_938, 0).unwrap())
        );
    }

    #[test]
    fn serialize_roundtrip() {
        let user = User {
            id: UserId::new(1),
            changed: DateTime::from_timestamp(1_700_000_000, 0).unwrap(),
            login: Some("test@test.com".to_owned()),
            currency: InstrumentId::new(1),
            parent: None,
            country: None,
            country_code: None,
            email: None,
            is_forecast_enabled: None,
            month_start_day: None,
            paid_till: None,
            plan_balance_mode: None,
            plan_settings: None,
            subscription: None,
            subscription_renewal_date: None,
        };
        let json = serde_json::to_string(&user).unwrap();
        let deserialized: User = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, user);
    }
}
