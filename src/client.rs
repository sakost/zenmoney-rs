//! HTTP client for the ZenMoney API.
//!
//! Provides both async and blocking client variants behind feature flags.

/// Base URL for the ZenMoney API.
const DEFAULT_BASE_URL: &str = "https://api.zenmoney.ru";

/// Diff endpoint path.
const DIFF_PATH: &str = "/v8/diff/";

/// Suggest endpoint path.
const SUGGEST_PATH: &str = "/v8/suggest/";

/// Generates a ZenMoney client (async or blocking) with builder, methods, and tests.
macro_rules! define_client {
    (
        client_name: $client:ident,
        builder_name: $builder:ident,
        http_type: $http_type:ty,
        response_type: $resp_type:ty,
        client_doc: $client_doc:expr,
        builder_doc: $builder_doc:expr,
        $(async_kw: $async_kw:tt,)?
        $(await_kw: $await_ext:tt,)?
        $(send_bound: $send_bound:tt,)?
    ) => {
        #[doc = $builder_doc]
        #[derive(Debug)]
        pub struct $builder {
            /// Access token for API authentication.
            token: Option<String>,
            /// Base URL override (for testing).
            base_url: Option<String>,
        }

        impl $builder {
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
            #[tracing::instrument(skip_all)]
            pub fn build(self) -> Result<$client> {
                let token = self.token.ok_or(ZenMoneyError::TokenExpired)?;
                let base_url = self
                    .base_url
                    .unwrap_or_else(|| DEFAULT_BASE_URL.to_owned());
                tracing::debug!(base_url = %base_url, "building client");
                let http = <$http_type>::builder().build()?;

                Ok($client {
                    http,
                    token,
                    base_url,
                })
            }
        }

        #[doc = $client_doc]
        #[derive(Debug)]
        pub struct $client {
            /// Underlying HTTP client.
            http: $http_type,
            /// Bearer access token.
            token: String,
            /// API base URL.
            base_url: String,
        }

        impl $client {
            /// Creates a new builder for configuring the client.
            #[inline]
            #[must_use]
            pub const fn builder() -> $builder {
                $builder {
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
            #[tracing::instrument(skip_all)]
            pub $($async_kw)? fn diff(
                &self,
                request: &DiffRequest,
            ) -> Result<DiffResponse> {
                tracing::debug!("calling diff endpoint");
                self.post_json(DIFF_PATH, request) $( .$await_ext )?
            }

            /// Gets category and payee suggestions via the `/v8/suggest/`
            /// endpoint.
            ///
            /// # Errors
            ///
            /// Returns an error if the HTTP request fails, the server returns a
            /// non-success status, or the response cannot be deserialized.
            #[inline]
            #[tracing::instrument(skip_all)]
            pub $($async_kw)? fn suggest(
                &self,
                request: &SuggestRequest,
            ) -> Result<SuggestResponse> {
                tracing::debug!("calling suggest endpoint");
                self.post_json(SUGGEST_PATH, request) $( .$await_ext )?
            }

            /// Sends an authenticated JSON POST request and deserializes the
            /// response.
            #[tracing::instrument(skip_all, fields(path = %path))]
            $($async_kw)? fn post_json<
                Req: serde::Serialize $(+ $send_bound)?,
                Resp: serde::de::DeserializeOwned,
            >(
                &self,
                path: &str,
                request: &Req,
            ) -> Result<Resp> {
                let url = format!("{}{path}", self.base_url);
                tracing::trace!(url = %url, "sending POST request");
                let response: $resp_type = self
                    .http
                    .post(&url)
                    .header(AUTHORIZATION, format!("Bearer {}", self.token))
                    .header(CONTENT_TYPE, "application/json")
                    .json(request)
                    .send()
                    $( .$await_ext )?
                    ?;

                let status = response.status();
                tracing::debug!(status = %status, "received response");
                if status.is_success() {
                    let body = response.text() $( .$await_ext )? ?;
                    tracing::trace!(body_len = body.len(), "parsing response body");
                    serde_json::from_str(&body).map_err(ZenMoneyError::from)
                } else {
                    let message = response
                        .text()
                        $( .$await_ext )?
                        .unwrap_or_else(|_| "unknown error".to_owned());
                    tracing::debug!(status = status.as_u16(), message = %message, "API error");
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
                let result = $client::builder().build();
                assert!(result.is_err());
            }

            #[test]
            fn builder_with_token_succeeds() {
                let client = $client::builder()
                    .token("test-token")
                    .build()
                    .unwrap();
                assert_eq!(client.base_url, DEFAULT_BASE_URL);
            }

            #[test]
            fn builder_custom_base_url() {
                let client = $client::builder()
                    .token("test-token")
                    .base_url("http://localhost:8080")
                    .build()
                    .unwrap();
                assert_eq!(client.base_url, "http://localhost:8080");
            }
        }
    };
}

#[cfg(feature = "async")]
mod async_client {
    //! Async HTTP client for the ZenMoney API.

    use reqwest::header::{AUTHORIZATION, CONTENT_TYPE};

    use super::{DEFAULT_BASE_URL, DIFF_PATH, SUGGEST_PATH};
    use crate::error::{Result, ZenMoneyError};
    use crate::models::{DiffRequest, DiffResponse, SuggestRequest, SuggestResponse};

    define_client! {
        client_name: ZenMoneyClient,
        builder_name: ZenMoneyClientBuilder,
        http_type: reqwest::Client,
        response_type: reqwest::Response,
        client_doc: "Async client for the ZenMoney API.\n\nUse [`ZenMoneyClient::builder()`] to construct an instance.",
        builder_doc: "Builder for constructing a [`ZenMoneyClient`].",
        async_kw: async,
        await_kw: await,
        send_bound: Sync,
    }
}

#[cfg(feature = "blocking")]
mod blocking_client {
    //! Blocking (synchronous) HTTP client for the ZenMoney API.

    use reqwest::header::{AUTHORIZATION, CONTENT_TYPE};

    use super::{DEFAULT_BASE_URL, DIFF_PATH, SUGGEST_PATH};
    use crate::error::{Result, ZenMoneyError};
    use crate::models::{DiffRequest, DiffResponse, SuggestRequest, SuggestResponse};

    define_client! {
        client_name: ZenMoneyBlockingClient,
        builder_name: ZenMoneyBlockingClientBuilder,
        http_type: reqwest::blocking::Client,
        response_type: reqwest::blocking::Response,
        client_doc: "Blocking (synchronous) client for the ZenMoney API.\n\nUse [`ZenMoneyBlockingClient::builder()`] to construct an instance.",
        builder_doc: "Builder for constructing a [`ZenMoneyBlockingClient`].",
    }
}

#[cfg(feature = "async")]
pub use async_client::{ZenMoneyClient, ZenMoneyClientBuilder};
#[cfg(feature = "blocking")]
pub use blocking_client::{ZenMoneyBlockingClient, ZenMoneyBlockingClientBuilder};
