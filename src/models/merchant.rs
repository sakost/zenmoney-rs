//! Merchant/payee model.

use serde::{Deserialize, Serialize};

use super::{MerchantId, UserId};

/// A merchant or payee associated with transactions.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Merchant {
    /// Unique identifier (UUID).
    pub id: MerchantId,
    /// Last modification timestamp (Unix seconds).
    pub changed: i64,
    /// Owner user identifier.
    pub user: UserId,
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
        assert_eq!(merchant.title, "McDonald's");
    }

    #[test]
    fn serialize_roundtrip() {
        let merchant = Merchant {
            id: MerchantId::new("m-1".to_owned()),
            changed: 1_700_000_000,
            user: UserId::new(1),
            title: "Test Merchant".to_owned(),
        };
        let json = serde_json::to_string(&merchant).unwrap();
        let deserialized: Merchant = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, merchant);
    }
}
