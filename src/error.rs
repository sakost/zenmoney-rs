//! Error types for the ZenMoney client library.

/// All errors that can occur when using the ZenMoney client.
#[derive(Debug, thiserror::Error)]
pub enum ZenMoneyError {
    /// HTTP request failed.
    #[cfg(any(feature = "async", feature = "blocking"))]
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    /// API returned a non-success status code.
    #[cfg(any(feature = "async", feature = "blocking"))]
    #[error("API error (status {status}): {message}")]
    Api {
        /// HTTP status code.
        status: u16,
        /// Error message from the API response body.
        message: String,
    },

    /// JSON serialization or deserialization failed.
    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// Token storage backend failed.
    #[error("token storage error: {0}")]
    TokenStorage(Box<dyn core::error::Error + Send + Sync>),

    /// Storage backend operation failed.
    #[error("storage error: {0}")]
    Storage(Box<dyn core::error::Error + Send + Sync>),

    /// Access token has expired and cannot be refreshed.
    #[error("access token expired and no refresh mechanism is available")]
    TokenExpired,

    /// OAuth flow error.
    #[cfg(feature = "oauth")]
    #[error("OAuth error: {0}")]
    OAuth(String),
}

/// Convenience type alias for results using [`ZenMoneyError`].
pub type Result<T> = core::result::Result<T, ZenMoneyError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_from_serde_json() {
        let serde_err = serde_json::from_str::<String>("not json").unwrap_err();
        let err = ZenMoneyError::from(serde_err);
        assert!(matches!(err, ZenMoneyError::Serialization(_)));
        let msg = err.to_string();
        assert!(msg.contains("serialization error"));
    }

    #[test]
    fn error_token_storage_display() {
        let inner = std::io::Error::new(std::io::ErrorKind::NotFound, "file missing");
        let err = ZenMoneyError::TokenStorage(Box::new(inner));
        let msg = err.to_string();
        assert!(msg.contains("token storage error"));
        assert!(msg.contains("file missing"));
    }

    #[test]
    fn error_storage_display() {
        let inner = std::io::Error::new(std::io::ErrorKind::Other, "disk full");
        let err = ZenMoneyError::Storage(Box::new(inner));
        let msg = err.to_string();
        assert!(msg.contains("storage error"));
        assert!(msg.contains("disk full"));
    }

    #[test]
    fn error_token_expired_display() {
        let err = ZenMoneyError::TokenExpired;
        assert!(err.to_string().contains("expired"));
    }

    #[cfg(any(feature = "async", feature = "blocking"))]
    #[test]
    fn error_api_display() {
        let err = ZenMoneyError::Api {
            status: 401,
            message: "Unauthorized".to_owned(),
        };
        let msg = err.to_string();
        assert!(msg.contains("401"));
        assert!(msg.contains("Unauthorized"));
    }

    #[test]
    fn error_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<ZenMoneyError>();
    }
}
