//! Currency/financial instrument model.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::InstrumentId;

/// A currency or financial instrument with its exchange rate.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Instrument {
    /// Unique identifier.
    pub id: InstrumentId,
    /// Last modification timestamp.
    #[serde(with = "chrono::serde::ts_seconds")]
    pub changed: DateTime<Utc>,
    /// Full name of the instrument (e.g. "US Dollar").
    pub title: String,
    /// Three-letter currency code (e.g. "USD").
    pub short_title: String,
    /// Currency symbol (e.g. "$").
    pub symbol: String,
    /// Exchange rate relative to Russian ruble.
    pub rate: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deserialize_instrument() {
        let json = r#"{
            "id": 2,
            "changed": 1700000000,
            "title": "US Dollar",
            "shortTitle": "USD",
            "symbol": "$",
            "rate": 92.5
        }"#;
        let instrument: Instrument = serde_json::from_str(json).unwrap();
        assert_eq!(instrument.id, InstrumentId::new(2));
        assert_eq!(
            instrument.changed,
            DateTime::from_timestamp(1_700_000_000, 0).unwrap()
        );
        assert_eq!(instrument.title, "US Dollar");
        assert_eq!(instrument.short_title, "USD");
        assert_eq!(instrument.symbol, "$");
        assert!((instrument.rate - 92.5).abs() < f64::EPSILON);
    }

    #[test]
    fn serialize_roundtrip() {
        let instrument = Instrument {
            id: InstrumentId::new(1),
            changed: DateTime::from_timestamp(1_700_000_000, 0).unwrap(),
            title: "Russian Ruble".to_owned(),
            short_title: "RUB".to_owned(),
            symbol: "\u{20bd}".to_owned(),
            rate: 1.0,
        };
        let json = serde_json::to_string(&instrument).unwrap();
        let deserialized: Instrument = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, instrument);
    }
}
