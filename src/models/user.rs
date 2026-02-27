//! User account model.

use serde::{Deserialize, Serialize};

use super::{InstrumentId, UserId};

/// A ZenMoney user account.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct User {
    /// Unique identifier.
    pub id: UserId,
    /// Last modification timestamp (Unix seconds).
    pub changed: i64,
    /// User login (email or username).
    pub login: Option<String>,
    /// Preferred currency instrument identifier.
    pub currency: InstrumentId,
    /// Parent user identifier (for family/shared accounts).
    pub parent: Option<UserId>,
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
    fn serialize_roundtrip() {
        let user = User {
            id: UserId::new(1),
            changed: 1_700_000_000,
            login: Some("test@test.com".to_owned()),
            currency: InstrumentId::new(1),
            parent: None,
        };
        let json = serde_json::to_string(&user).unwrap();
        let deserialized: User = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, user);
    }
}
