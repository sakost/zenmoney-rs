//! Error types for the ZenMoney client library.

/// All errors that can occur when using the ZenMoney client.
#[derive(Debug, thiserror::Error)]
pub enum ZenMoneyError {
    /// JSON serialization or deserialization failed.
    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// Token storage backend failed.
    #[error("token storage error: {0}")]
    TokenStorage(Box<dyn core::error::Error + Send + Sync>),

    /// Access token has expired and cannot be refreshed.
    #[error("access token expired and no refresh mechanism is available")]
    TokenExpired,
}

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
    fn error_token_expired_display() {
        let err = ZenMoneyError::TokenExpired;
        assert!(err.to_string().contains("expired"));
    }

    #[test]
    fn error_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<ZenMoneyError>();
    }
}
