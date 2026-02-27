//! Suggest endpoint request/response models.

use serde::{Deserialize, Serialize};

use super::{MerchantId, TagId};

/// A partial transaction used as input for the suggest endpoint.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SuggestRequest {
    /// Payee name to get suggestions for.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub payee: Option<String>,
    /// Comment to get suggestions for.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub comment: Option<String>,
}

/// Suggestion result from the `/v8/suggest/` endpoint.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SuggestResponse {
    /// Normalized payee name.
    #[serde(default)]
    pub payee: Option<String>,
    /// Suggested merchant identifier.
    #[serde(default)]
    pub merchant: Option<MerchantId>,
    /// Suggested category tags.
    #[serde(default)]
    pub tag: Option<Vec<TagId>>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serialize_suggest_request_payee_only() {
        let req = SuggestRequest {
            payee: Some("McDonalds".to_owned()),
            comment: None,
        };
        let json = serde_json::to_value(&req).unwrap();
        assert_eq!(json["payee"], "McDonalds");
        assert!(json.get("comment").is_none());
    }

    #[test]
    fn deserialize_suggest_response() {
        let json = r#"{
            "payee": "McDonald's",
            "merchant": "merchant-mcdonalds",
            "tag": ["tag-food"]
        }"#;
        let resp: SuggestResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.payee.as_deref(), Some("McDonald's"));
        assert_eq!(
            resp.merchant,
            Some(MerchantId::new("merchant-mcdonalds".to_owned()))
        );
        assert_eq!(resp.tag.as_ref().map(Vec::len), Some(1));
    }

    #[test]
    fn deserialize_suggest_response_minimal() {
        let json = r#"{}"#;
        let resp: SuggestResponse = serde_json::from_str(json).unwrap();
        assert!(resp.payee.is_none());
        assert!(resp.merchant.is_none());
        assert!(resp.tag.is_none());
    }

    #[test]
    fn suggest_request_default_is_empty() {
        let req = SuggestRequest::default();
        let json = serde_json::to_value(&req).unwrap();
        let obj = json.as_object().unwrap();
        assert!(obj.is_empty());
    }
}
