//! HTTP client for the ZenMoney API.
//!
//! Provides both async and blocking client variants behind feature flags.

#[cfg(feature = "async")]
mod async_client;

#[cfg(feature = "async")]
pub use async_client::{ZenMoneyClient, ZenMoneyClientBuilder};
