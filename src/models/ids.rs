//! Newtype wrappers for entity identifiers.
//!
//! These prevent accidentally mixing up IDs of different entity types
//! at compile time.

use serde::{Deserialize, Serialize};

/// Macro to define a newtype ID wrapping a `Copy` inner type.
macro_rules! define_copy_id {
    (
        $(#[$meta:meta])*
        $name:ident($inner:ty)
    ) => {
        $(#[$meta])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $name($inner);

        impl $name {
            /// Creates a new identifier from the given value.
            #[inline]
            #[must_use]
            pub const fn new(value: $inner) -> Self {
                Self(value)
            }

            /// Returns a reference to the inner value.
            #[inline]
            #[must_use]
            pub const fn as_inner(&self) -> &$inner {
                &self.0
            }

            /// Consumes the wrapper and returns the inner value.
            #[inline]
            #[must_use]
            pub const fn into_inner(self) -> $inner {
                self.0
            }
        }

        impl core::fmt::Display for $name {
            #[inline]
            fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                core::fmt::Display::fmt(&self.0, f)
            }
        }

        impl From<$inner> for $name {
            #[inline]
            fn from(value: $inner) -> Self {
                Self(value)
            }
        }
    };
}

/// Macro to define a newtype ID wrapping a `String` inner type.
macro_rules! define_string_id {
    (
        $(#[$meta:meta])*
        $name:ident
    ) => {
        $(#[$meta])*
        #[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $name(String);

        impl $name {
            /// Creates a new identifier from the given string.
            #[inline]
            #[must_use]
            pub const fn new(value: String) -> Self {
                Self(value)
            }

            /// Returns a reference to the inner string.
            #[inline]
            #[must_use]
            pub fn as_inner(&self) -> &str {
                &self.0
            }

            /// Consumes the wrapper and returns the inner string.
            #[inline]
            #[must_use]
            pub fn into_inner(self) -> String {
                self.0
            }
        }

        impl core::fmt::Display for $name {
            #[inline]
            fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                core::fmt::Display::fmt(&self.0, f)
            }
        }

        impl From<String> for $name {
            #[inline]
            fn from(value: String) -> Self {
                Self(value)
            }
        }
    };
}

define_copy_id! {
    /// Unique identifier for a user.
    UserId(i64)
}

define_copy_id! {
    /// Unique identifier for a currency/financial instrument.
    InstrumentId(i32)
}

define_copy_id! {
    /// Unique identifier for a financial company/institution.
    CompanyId(i32)
}

define_string_id! {
    /// Unique identifier for a user account (UUID string).
    AccountId
}

define_string_id! {
    /// Unique identifier for a transaction category tag (UUID string).
    TagId
}

define_string_id! {
    /// Unique identifier for a merchant/payee (UUID string).
    MerchantId
}

define_string_id! {
    /// Unique identifier for a reminder (UUID string).
    ReminderId
}

define_string_id! {
    /// Unique identifier for a reminder marker instance (UUID string).
    ReminderMarkerId
}

define_string_id! {
    /// Unique identifier for a transaction (UUID string).
    TransactionId
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn user_id_serde_roundtrip() {
        let id = UserId::new(42);
        let json = serde_json::to_string(&id).unwrap();
        assert_eq!(json, "42");
        let deserialized: UserId = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, id);
    }

    #[test]
    fn instrument_id_serde_roundtrip() {
        let id = InstrumentId::new(1);
        let json = serde_json::to_string(&id).unwrap();
        assert_eq!(json, "1");
        let deserialized: InstrumentId = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, id);
    }

    #[test]
    fn company_id_serde_roundtrip() {
        let id = CompanyId::new(100);
        let json = serde_json::to_string(&id).unwrap();
        assert_eq!(json, "100");
        let deserialized: CompanyId = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, id);
    }

    #[test]
    fn account_id_serde_roundtrip() {
        let id = AccountId::new("550e8400-e29b-41d4-a716-446655440000".to_owned());
        let json = serde_json::to_string(&id).unwrap();
        assert_eq!(json, r#""550e8400-e29b-41d4-a716-446655440000""#);
        let deserialized: AccountId = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, id);
    }

    #[test]
    fn tag_id_serde_roundtrip() {
        let id = TagId::new("a1b2c3d4-0000-0000-0000-000000000000".to_owned());
        let json = serde_json::to_string(&id).unwrap();
        let deserialized: TagId = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, id);
    }

    #[test]
    fn string_id_display() {
        let id = AccountId::new("abc-123".to_owned());
        assert_eq!(id.to_string(), "abc-123");
    }

    #[test]
    fn numeric_id_display() {
        let id = UserId::new(99);
        assert_eq!(id.to_string(), "99");
    }

    #[test]
    fn id_from_inner() {
        let id: UserId = 42_i64.into();
        assert_eq!(*id.as_inner(), 42);

        let id: AccountId = "abc".to_owned().into();
        assert_eq!(id.as_inner(), "abc");
    }

    #[test]
    fn id_into_inner() {
        let id = UserId::new(7);
        assert_eq!(id.into_inner(), 7);

        let id = MerchantId::new("m-1".to_owned());
        assert_eq!(id.into_inner(), "m-1");
    }

    #[test]
    fn copy_id_is_copy() {
        let id = UserId::new(1);
        let id2 = id;
        // Both still usable — Copy semantics
        assert_eq!(id, id2);
    }

    #[test]
    fn different_id_types_are_distinct() {
        let _user = UserId::new(1);
        let _instrument = InstrumentId::new(1);
        let _company = CompanyId::new(1);
    }
}
