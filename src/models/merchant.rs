//! Merchant/payee model.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::{MerchantId, UserId};

/// A merchant or payee associated with transactions.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Merchant {
    /// Unique identifier (UUID).
    pub id: MerchantId,
    /// Last modification timestamp.
    #[serde(with = "chrono::serde::ts_seconds")]
    pub changed: DateTime<Utc>,
    /// Owner user identifier. System merchants may not have an owner.
    pub user: Option<UserId>,
    /// Merchant display name.
    pub title: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deserialize_merchant() {
        let json = r#"{
            "id": "merchant-001",
            "changed": 1700000000,
            "user": 123,
            "title": "McDonald's"
        }"#;
        let merchant: Merchant = serde_json::from_str(json).unwrap();
        assert_eq!(merchant.id, MerchantId::new("merchant-001".to_owned()));
        assert_eq!(merchant.user, Some(UserId::new(123)));
        assert_eq!(merchant.title, "McDonald's");
    }

    #[test]
    fn deserialize_system_merchant_without_user() {
        let json = r#"{
            "id": "428875db-3f0c-4b37-a132-774b0ccad703",
            "changed": 1403602415,
            "user": null,
            "title": "Перекрёсток"
        }"#;
        let merchant: Merchant = serde_json::from_str(json).unwrap();
        assert_eq!(merchant.user, None);
        assert_eq!(merchant.title, "Перекрёсток");
    }

    #[test]
    fn serialize_roundtrip() {
        let merchant = Merchant {
            id: MerchantId::new("m-1".to_owned()),
            changed: DateTime::from_timestamp(1_700_000_000, 0).unwrap(),
            user: Some(UserId::new(1)),
            title: "Test Merchant".to_owned(),
        };
        let json = serde_json::to_string(&merchant).unwrap();
        let deserialized: Merchant = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, merchant);
    }
}
