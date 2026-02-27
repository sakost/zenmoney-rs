//! Diff synchronization request/response models.

use serde::{Deserialize, Serialize};

use super::{
    Account, Budget, Company, Instrument, Merchant, Reminder, ReminderMarker, Tag, Transaction,
    User,
};

/// A deletion record identifying a removed entity.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Deletion {
    /// Entity identifier.
    pub id: String,
    /// Entity type name (e.g. "transaction", "account").
    pub object: String,
    /// Timestamp of deletion (Unix seconds).
    pub stamp: i64,
    /// User who deleted the entity.
    pub user: i64,
}

/// Request body for the `/v8/diff/` synchronization endpoint.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiffRequest {
    /// Client's current Unix timestamp (for server time correction).
    pub current_client_timestamp: i64,
    /// Last known server timestamp (0 for initial sync).
    pub server_timestamp: i64,
    /// Entity types to force-fetch completely.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub force_fetch: Vec<String>,
    /// Accounts to create or update.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub account: Vec<Account>,
    /// Tags to create or update.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tag: Vec<Tag>,
    /// Merchants to create or update.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub merchant: Vec<Merchant>,
    /// Transactions to create or update.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub transaction: Vec<Transaction>,
    /// Reminders to create or update.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub reminder: Vec<Reminder>,
    /// Reminder markers to create or update.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub reminder_marker: Vec<ReminderMarker>,
    /// Budgets to create or update.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub budget: Vec<Budget>,
    /// Entities to delete.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub deletion: Vec<Deletion>,
}

impl DiffRequest {
    /// Creates a minimal diff request for syncing (read-only, no changes).
    #[inline]
    #[must_use]
    pub const fn sync_only(server_timestamp: i64, current_client_timestamp: i64) -> Self {
        Self {
            current_client_timestamp,
            server_timestamp,
            force_fetch: Vec::new(),
            account: Vec::new(),
            tag: Vec::new(),
            merchant: Vec::new(),
            transaction: Vec::new(),
            reminder: Vec::new(),
            reminder_marker: Vec::new(),
            budget: Vec::new(),
            deletion: Vec::new(),
        }
    }
}

/// Response body from the `/v8/diff/` synchronization endpoint.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiffResponse {
    /// New server timestamp to use for the next sync.
    pub server_timestamp: i64,
    /// Updated instruments.
    #[serde(default)]
    pub instrument: Vec<Instrument>,
    /// Updated companies.
    #[serde(default)]
    pub company: Vec<Company>,
    /// Updated users.
    #[serde(default)]
    pub user: Vec<User>,
    /// Updated accounts.
    #[serde(default)]
    pub account: Vec<Account>,
    /// Updated tags.
    #[serde(default)]
    pub tag: Vec<Tag>,
    /// Updated merchants.
    #[serde(default)]
    pub merchant: Vec<Merchant>,
    /// Updated transactions.
    #[serde(default)]
    pub transaction: Vec<Transaction>,
    /// Updated reminders.
    #[serde(default)]
    pub reminder: Vec<Reminder>,
    /// Updated reminder markers.
    #[serde(default)]
    pub reminder_marker: Vec<ReminderMarker>,
    /// Updated budgets.
    #[serde(default)]
    pub budget: Vec<Budget>,
    /// Deleted entities.
    #[serde(default)]
    pub deletion: Vec<Deletion>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serialize_sync_only_request() {
        let req = DiffRequest::sync_only(0, 1_700_000_000);
        let json = serde_json::to_value(&req).unwrap();
        assert_eq!(json["currentClientTimestamp"], 1_700_000_000);
        assert_eq!(json["serverTimestamp"], 0);
        // Empty arrays should be omitted
        assert!(json.get("account").is_none());
        assert!(json.get("transaction").is_none());
        assert!(json.get("deletion").is_none());
    }

    #[test]
    fn deserialize_diff_response_minimal() {
        let json = r#"{
            "serverTimestamp": 1700000100
        }"#;
        let resp: DiffResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.server_timestamp, 1_700_000_100);
        assert!(resp.instrument.is_empty());
        assert!(resp.transaction.is_empty());
    }

    #[test]
    fn deserialize_diff_response_with_entities() {
        let json = r#"{
            "serverTimestamp": 1700000100,
            "instrument": [
                {
                    "id": 1,
                    "changed": 1700000000,
                    "title": "Russian Ruble",
                    "shortTitle": "RUB",
                    "symbol": "\u20bd",
                    "rate": 1.0
                }
            ],
            "deletion": [
                {
                    "id": "tx-old",
                    "object": "transaction",
                    "stamp": 1700000050,
                    "user": 123
                }
            ]
        }"#;
        let resp: DiffResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.instrument.len(), 1);
        assert_eq!(resp.deletion.len(), 1);
        assert_eq!(resp.deletion[0].object, "transaction");
    }

    #[test]
    fn deletion_serde_roundtrip() {
        let deletion = Deletion {
            id: "some-id".to_owned(),
            object: "account".to_owned(),
            stamp: 1_700_000_000,
            user: 123,
        };
        let json = serde_json::to_string(&deletion).unwrap();
        let deserialized: Deletion = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, deletion);
    }

    #[test]
    fn diff_request_roundtrip() {
        let req = DiffRequest::sync_only(100, 200);
        let json = serde_json::to_string(&req).unwrap();
        let deserialized: DiffRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.server_timestamp, 100);
        assert_eq!(deserialized.current_client_timestamp, 200);
    }
}
