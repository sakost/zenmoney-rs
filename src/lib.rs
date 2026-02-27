//! Rust client library for the `ZenMoney` API.
//!
//! This crate provides a typed client for interacting with the
//! [ZenMoney](https://zenmoney.ru/) personal finance API.

#[cfg(any(feature = "async", feature = "blocking"))]
pub mod client;
pub mod error;
pub mod models;
