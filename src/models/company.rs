//! Financial company/institution model.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::CompanyId;

/// A financial institution (bank, payment provider, etc.).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Company {
    /// Unique identifier.
    pub id: CompanyId,
    /// Last modification timestamp.
    #[serde(with = "chrono::serde::ts_seconds")]
    pub changed: DateTime<Utc>,
    /// Short company name.
    pub title: String,
    /// Full legal name.
    pub full_title: Option<String>,
    /// Company website URL.
    pub www: Option<String>,
    /// Country identifier.
    #[serde(default)]
    pub country: Option<i32>,
    /// Two-letter country code.
    #[serde(default)]
    pub country_code: Option<String>,
    /// Whether the company has been deleted.
    #[serde(default)]
    pub deleted: Option<bool>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deserialize_company_full() {
        let json = r#"{
            "id": 4,
            "changed": 1700000000,
            "title": "Sberbank",
            "fullTitle": "Sberbank of Russia",
            "www": "https://www.sberbank.ru",
            "country": 1,
            "countryCode": "RU"
        }"#;
        let company: Company = serde_json::from_str(json).unwrap();
        assert_eq!(company.id, CompanyId::new(4));
        assert_eq!(company.title, "Sberbank");
        assert_eq!(company.full_title.as_deref(), Some("Sberbank of Russia"));
        assert_eq!(company.www.as_deref(), Some("https://www.sberbank.ru"));
        assert_eq!(company.country, Some(1));
        assert_eq!(company.country_code.as_deref(), Some("RU"));
    }

    #[test]
    fn deserialize_company_nullable_fields() {
        let json = r#"{
            "id": 5,
            "changed": 1700000000,
            "title": "Unknown Bank",
            "fullTitle": null,
            "www": null,
            "country": null
        }"#;
        let company: Company = serde_json::from_str(json).unwrap();
        assert!(company.full_title.is_none());
        assert!(company.www.is_none());
        assert!(company.country.is_none());
    }

    #[test]
    fn deserialize_company_with_deleted() {
        let json = r#"{
            "id": 6,
            "changed": 1700000000,
            "title": "Old Bank",
            "fullTitle": null,
            "www": null,
            "country": 2,
            "deleted": true
        }"#;
        let company: Company = serde_json::from_str(json).unwrap();
        assert_eq!(company.deleted, Some(true));
    }

    #[test]
    fn serialize_roundtrip() {
        let company = Company {
            id: CompanyId::new(1),
            changed: DateTime::from_timestamp(1_700_000_000, 0).unwrap(),
            title: "Test Bank".to_owned(),
            full_title: None,
            www: Some("https://example.com".to_owned()),
            country: Some(1),
            country_code: Some("US".to_owned()),
            deleted: None,
        };
        let json = serde_json::to_string(&company).unwrap();
        let deserialized: Company = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, company);
    }
}
