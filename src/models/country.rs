//! Country model.

use serde::{Deserialize, Serialize};

use super::InstrumentId;

/// A country with its associated currency.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Country {
    /// Unique identifier.
    pub id: i32,
    /// Country name.
    pub title: String,
    /// Default currency instrument identifier.
    pub currency: InstrumentId,
    /// Domain suffix (e.g. "ru", "us").
    #[serde(default)]
    pub domain: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deserialize_country() {
        let json = r#"{
            "id": 1,
            "title": "Russia",
            "currency": 2,
            "domain": "ru"
        }"#;
        let country: Country = serde_json::from_str(json).unwrap();
        assert_eq!(country.id, 1);
        assert_eq!(country.title, "Russia");
        assert_eq!(country.currency, InstrumentId::new(2));
        assert_eq!(country.domain.as_deref(), Some("ru"));
    }

    #[test]
    fn deserialize_country_without_domain() {
        let json = r#"{
            "id": 2,
            "title": "United States",
            "currency": 1
        }"#;
        let country: Country = serde_json::from_str(json).unwrap();
        assert_eq!(country.id, 2);
        assert!(country.domain.is_none());
    }

    #[test]
    fn serialize_roundtrip() {
        let country = Country {
            id: 1,
            title: "Russia".to_owned(),
            currency: InstrumentId::new(2),
            domain: Some("ru".to_owned()),
        };
        let json = serde_json::to_string(&country).unwrap();
        let deserialized: Country = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, country);
    }
}
