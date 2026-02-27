//! Async HTTP client for the ZenMoney API.

use reqwest::header::{AUTHORIZATION, CONTENT_TYPE};

use crate::error::{Result, ZenMoneyError};
use crate::models::{DiffRequest, DiffResponse, SuggestRequest, SuggestResponse};

/// Base URL for the ZenMoney API.
const DEFAULT_BASE_URL: &str = "https://api.zenmoney.ru";

/// Diff endpoint path.
const DIFF_PATH: &str = "/v8/diff/";

/// Suggest endpoint path.
const SUGGEST_PATH: &str = "/v8/suggest/";

/// Builder for constructing a [`ZenMoneyClient`].
#[derive(Debug)]
pub struct ZenMoneyClientBuilder {
    /// Access token for API authentication.
    token: Option<String>,
    /// Base URL override (for testing).
    base_url: Option<String>,
}

impl ZenMoneyClientBuilder {
    /// Sets the access token for API authentication.
    #[inline]
    #[must_use]
    pub fn token<T: Into<String>>(mut self, token: T) -> Self {
        self.token = Some(token.into());
        self
    }

    /// Overrides the base URL (useful for testing with a mock server).
    #[inline]
    #[must_use]
    pub fn base_url<T: Into<String>>(mut self, url: T) -> Self {
        self.base_url = Some(url.into());
        self
    }

    /// Builds the client.
    ///
    /// # Errors
    ///
    /// Returns [`ZenMoneyError::TokenExpired`] if no token was provided.
    /// Returns [`ZenMoneyError::Http`] if the HTTP client fails to build.
    #[inline]
    pub fn build(self) -> Result<ZenMoneyClient> {
        let token = self.token.ok_or(ZenMoneyError::TokenExpired)?;
        let base_url = self.base_url.unwrap_or_else(|| DEFAULT_BASE_URL.to_owned());
        let http = reqwest::Client::builder().build()?;

        Ok(ZenMoneyClient {
            http,
            token,
            base_url,
        })
    }
}

/// Async client for the ZenMoney API.
///
/// Use [`ZenMoneyClient::builder()`] to construct an instance.
#[derive(Debug)]
pub struct ZenMoneyClient {
    /// Underlying HTTP client.
    http: reqwest::Client,
    /// Bearer access token.
    token: String,
    /// API base URL.
    base_url: String,
}

impl ZenMoneyClient {
    /// Creates a new builder for configuring the client.
    #[inline]
    #[must_use]
    pub const fn builder() -> ZenMoneyClientBuilder {
        ZenMoneyClientBuilder {
            token: None,
            base_url: None,
        }
    }

    /// Synchronizes data with the server via the `/v8/diff/` endpoint.
    ///
    /// Sends local changes and receives server changes since the last sync.
    ///
    /// # Errors
    ///
    /// Returns an error if the HTTP request fails, the server returns a
    /// non-success status, or the response cannot be deserialized.
    #[inline]
    pub async fn diff(&self, request: &DiffRequest) -> Result<DiffResponse> {
        self.post_json(DIFF_PATH, request).await
    }

    /// Gets category and payee suggestions via the `/v8/suggest/` endpoint.
    ///
    /// # Errors
    ///
    /// Returns an error if the HTTP request fails, the server returns a
    /// non-success status, or the response cannot be deserialized.
    #[inline]
    pub async fn suggest(&self, request: &SuggestRequest) -> Result<SuggestResponse> {
        self.post_json(SUGGEST_PATH, request).await
    }

    /// Sends an authenticated JSON POST request and deserializes the response.
    async fn post_json<Req: serde::Serialize + Sync, Resp: serde::de::DeserializeOwned>(
        &self,
        path: &str,
        body: &Req,
    ) -> Result<Resp> {
        let url = format!("{}{path}", self.base_url);
        let response = self
            .http
            .post(&url)
            .header(AUTHORIZATION, format!("Bearer {}", self.token))
            .header(CONTENT_TYPE, "application/json")
            .json(body)
            .send()
            .await?;

        Self::handle_response(response).await
    }

    /// Handles an HTTP response, checking status and deserializing the body.
    async fn handle_response<T: serde::de::DeserializeOwned>(
        response: reqwest::Response,
    ) -> Result<T> {
        let status = response.status();
        if status.is_success() {
            let body = response.text().await?;
            serde_json::from_str(&body).map_err(ZenMoneyError::from)
        } else {
            let message = response
                .text()
                .await
                .unwrap_or_else(|_| "unknown error".to_owned());
            Err(ZenMoneyError::Api {
                status: status.as_u16(),
                message,
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builder_requires_token() {
        let result = ZenMoneyClient::builder().build();
        assert!(result.is_err());
    }

    #[test]
    fn builder_with_token_succeeds() {
        let client = ZenMoneyClient::builder()
            .token("test-token")
            .build()
            .unwrap();
        assert_eq!(client.base_url, DEFAULT_BASE_URL);
    }

    #[test]
    fn builder_custom_base_url() {
        let client = ZenMoneyClient::builder()
            .token("test-token")
            .base_url("http://localhost:8080")
            .build()
            .unwrap();
        assert_eq!(client.base_url, "http://localhost:8080");
    }
}
