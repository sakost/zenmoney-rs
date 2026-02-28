//! Pluggable storage backends for persisting synced ZenMoney data.
//!
//! This module defines the [`Storage`] (async) and [`BlockingStorage`]
//! (blocking) traits via a shared macro, mirroring the client generation
//! pattern in [`crate::client`].

#[cfg(feature = "storage-file")]
mod file;
mod memory;

#[cfg(feature = "storage-file")]
pub use file::FileStorage;
pub use memory::InMemoryStorage;

/// Generates a storage trait (async or blocking) with all entity methods.
///
/// Uses `@methods` to define the method list once, and `@method` to render
/// each method in async (`impl Future + Send`) or blocking (`fn`) style.
macro_rules! define_storage {
    // ── Entry points ────────────────────────────────────────────────
    (
        trait_name: $trait_name:ident,
        trait_doc: $trait_doc:expr,
        mode: async_mode,
    ) => {
        #[doc = $trait_doc]
        pub trait $trait_name: core::fmt::Debug + Send + Sync {
            define_storage!(@methods async_mode);
        }
    };
    (
        trait_name: $trait_name:ident,
        trait_doc: $trait_doc:expr,
        mode: blocking,
    ) => {
        #[doc = $trait_doc]
        pub trait $trait_name: core::fmt::Debug + Send + Sync {
            define_storage!(@methods blocking);
        }
    };

    // ── Single method list (shared between both variants) ───────────
    (@methods $mode:ident) => {
        // Sync state
        define_storage!(@method $mode, server_timestamp,
            "Returns the server timestamp from the last successful sync.\n\nReturns `Ok(None)` if no sync has occurred yet.\n\n# Errors\n\nReturns an error if the storage backend fails to read the timestamp.",
            -> Result<Option<DateTime<Utc>>>);
        define_storage!(@method $mode, set_server_timestamp,
            "Stores the server timestamp after a successful sync.\n\n# Errors\n\nReturns an error if the storage backend fails to write the timestamp.",
            timestamp: DateTime<Utc>, -> Result<()>);

        // Read
        define_storage!(@method $mode, accounts,
            "Returns all stored accounts.\n\n# Errors\n\nReturns an error if the storage backend fails to read.",
            -> Result<Vec<Account>>);
        define_storage!(@method $mode, transactions,
            "Returns all stored transactions.\n\n# Errors\n\nReturns an error if the storage backend fails to read.",
            -> Result<Vec<Transaction>>);
        define_storage!(@method $mode, tags,
            "Returns all stored tags.\n\n# Errors\n\nReturns an error if the storage backend fails to read.",
            -> Result<Vec<Tag>>);
        define_storage!(@method $mode, merchants,
            "Returns all stored merchants.\n\n# Errors\n\nReturns an error if the storage backend fails to read.",
            -> Result<Vec<Merchant>>);
        define_storage!(@method $mode, instruments,
            "Returns all stored instruments.\n\n# Errors\n\nReturns an error if the storage backend fails to read.",
            -> Result<Vec<Instrument>>);
        define_storage!(@method $mode, companies,
            "Returns all stored companies.\n\n# Errors\n\nReturns an error if the storage backend fails to read.",
            -> Result<Vec<Company>>);
        define_storage!(@method $mode, countries,
            "Returns all stored countries.\n\n# Errors\n\nReturns an error if the storage backend fails to read.",
            -> Result<Vec<Country>>);
        define_storage!(@method $mode, users,
            "Returns all stored users.\n\n# Errors\n\nReturns an error if the storage backend fails to read.",
            -> Result<Vec<User>>);
        define_storage!(@method $mode, reminders,
            "Returns all stored reminders.\n\n# Errors\n\nReturns an error if the storage backend fails to read.",
            -> Result<Vec<Reminder>>);
        define_storage!(@method $mode, reminder_markers,
            "Returns all stored reminder markers.\n\n# Errors\n\nReturns an error if the storage backend fails to read.",
            -> Result<Vec<ReminderMarker>>);
        define_storage!(@method $mode, budgets,
            "Returns all stored budgets.\n\n# Errors\n\nReturns an error if the storage backend fails to read.",
            -> Result<Vec<Budget>>);

        // Upsert
        define_storage!(@method $mode, upsert_accounts,
            "Inserts or updates accounts (matched by ID).\n\n# Errors\n\nReturns an error if the storage backend fails to write.",
            items: Vec<Account>, -> Result<()>);
        define_storage!(@method $mode, upsert_transactions,
            "Inserts or updates transactions (matched by ID).\n\n# Errors\n\nReturns an error if the storage backend fails to write.",
            items: Vec<Transaction>, -> Result<()>);
        define_storage!(@method $mode, upsert_tags,
            "Inserts or updates tags (matched by ID).\n\n# Errors\n\nReturns an error if the storage backend fails to write.",
            items: Vec<Tag>, -> Result<()>);
        define_storage!(@method $mode, upsert_merchants,
            "Inserts or updates merchants (matched by ID).\n\n# Errors\n\nReturns an error if the storage backend fails to write.",
            items: Vec<Merchant>, -> Result<()>);
        define_storage!(@method $mode, upsert_instruments,
            "Inserts or updates instruments (matched by ID).\n\n# Errors\n\nReturns an error if the storage backend fails to write.",
            items: Vec<Instrument>, -> Result<()>);
        define_storage!(@method $mode, upsert_companies,
            "Inserts or updates companies (matched by ID).\n\n# Errors\n\nReturns an error if the storage backend fails to write.",
            items: Vec<Company>, -> Result<()>);
        define_storage!(@method $mode, upsert_countries,
            "Inserts or updates countries (matched by ID).\n\n# Errors\n\nReturns an error if the storage backend fails to write.",
            items: Vec<Country>, -> Result<()>);
        define_storage!(@method $mode, upsert_users,
            "Inserts or updates users (matched by ID).\n\n# Errors\n\nReturns an error if the storage backend fails to write.",
            items: Vec<User>, -> Result<()>);
        define_storage!(@method $mode, upsert_reminders,
            "Inserts or updates reminders (matched by ID).\n\n# Errors\n\nReturns an error if the storage backend fails to write.",
            items: Vec<Reminder>, -> Result<()>);
        define_storage!(@method $mode, upsert_reminder_markers,
            "Inserts or updates reminder markers (matched by ID).\n\n# Errors\n\nReturns an error if the storage backend fails to write.",
            items: Vec<ReminderMarker>, -> Result<()>);
        define_storage!(@method $mode, upsert_budgets,
            "Inserts or updates budgets (matched by composite key: user + tag + date).\n\n# Errors\n\nReturns an error if the storage backend fails to write.",
            items: Vec<Budget>, -> Result<()>);

        // Remove
        define_storage!(@method $mode, remove_accounts,
            "Removes accounts by their IDs.\n\n# Errors\n\nReturns an error if the storage backend fails to write.",
            ids: &[AccountId], -> Result<()>);
        define_storage!(@method $mode, remove_transactions,
            "Removes transactions by their IDs.\n\n# Errors\n\nReturns an error if the storage backend fails to write.",
            ids: &[TransactionId], -> Result<()>);
        define_storage!(@method $mode, remove_tags,
            "Removes tags by their IDs.\n\n# Errors\n\nReturns an error if the storage backend fails to write.",
            ids: &[TagId], -> Result<()>);
        define_storage!(@method $mode, remove_merchants,
            "Removes merchants by their IDs.\n\n# Errors\n\nReturns an error if the storage backend fails to write.",
            ids: &[MerchantId], -> Result<()>);
        define_storage!(@method $mode, remove_instruments,
            "Removes instruments by their IDs.\n\n# Errors\n\nReturns an error if the storage backend fails to write.",
            ids: &[InstrumentId], -> Result<()>);
        define_storage!(@method $mode, remove_companies,
            "Removes companies by their IDs.\n\n# Errors\n\nReturns an error if the storage backend fails to write.",
            ids: &[CompanyId], -> Result<()>);
        define_storage!(@method $mode, remove_countries,
            "Removes countries by their IDs.\n\n# Errors\n\nReturns an error if the storage backend fails to write.",
            ids: &[i32], -> Result<()>);
        define_storage!(@method $mode, remove_users,
            "Removes users by their IDs.\n\n# Errors\n\nReturns an error if the storage backend fails to write.",
            ids: &[UserId], -> Result<()>);
        define_storage!(@method $mode, remove_reminders,
            "Removes reminders by their IDs.\n\n# Errors\n\nReturns an error if the storage backend fails to write.",
            ids: &[ReminderId], -> Result<()>);
        define_storage!(@method $mode, remove_reminder_markers,
            "Removes reminder markers by their IDs.\n\n# Errors\n\nReturns an error if the storage backend fails to write.",
            ids: &[ReminderMarkerId], -> Result<()>);
        define_storage!(@method $mode, remove_budgets,
            "Removes budgets by their raw deletion IDs.\n\n# Errors\n\nReturns an error if the storage backend fails to write.",
            ids: &[String], -> Result<()>);

        // Clear
        define_storage!(@method $mode, clear,
            "Removes all stored data (used before a full re-sync).\n\n# Errors\n\nReturns an error if the storage backend fails to write.",
            -> Result<()>);
    };

    // ── Blocking method renderer ────────────────────────────────────
    (@method blocking, $name:ident, $doc:expr,
     $($param:ident: $param_ty:ty,)* -> $ret:ty) => {
        #[doc = $doc]
        fn $name(&self $(, $param: $param_ty)*) -> $ret;
    };

    // ── Async method renderer (returns impl Future + Send) ──────────
    (@method async_mode, $name:ident, $doc:expr,
     $($param:ident: $param_ty:ty,)* -> $ret:ty) => {
        #[doc = $doc]
        fn $name(&self $(, $param: $param_ty)*)
            -> impl core::future::Future<Output = $ret> + Send;
    };
}

#[cfg(feature = "async")]
mod async_storage {
    //! Async storage trait definition.

    use crate::error::Result;
    use crate::models::{
        Account, AccountId, Budget, Company, CompanyId, Country, Instrument, InstrumentId,
        Merchant, MerchantId, Reminder, ReminderId, ReminderMarker, ReminderMarkerId, Tag, TagId,
        Transaction, TransactionId, User, UserId,
    };
    use chrono::{DateTime, Utc};

    define_storage! {
        trait_name: Storage,
        trait_doc: "Async storage backend for persisting synced ZenMoney data.\n\nAll methods take `&self` — implementations should use interior mutability\n(e.g. `Mutex`) for thread-safe mutation.",
        mode: async_mode,
    }
}

#[cfg(feature = "blocking")]
mod blocking_storage {
    //! Blocking storage trait definition.

    use crate::error::Result;
    use crate::models::{
        Account, AccountId, Budget, Company, CompanyId, Country, Instrument, InstrumentId,
        Merchant, MerchantId, Reminder, ReminderId, ReminderMarker, ReminderMarkerId, Tag, TagId,
        Transaction, TransactionId, User, UserId,
    };
    use chrono::{DateTime, Utc};

    define_storage! {
        trait_name: BlockingStorage,
        trait_doc: "Blocking storage backend for persisting synced ZenMoney data.\n\nAll methods take `&self` — implementations should use interior mutability\n(e.g. `Mutex`) for thread-safe mutation.",
        mode: blocking,
    }
}

#[cfg(feature = "async")]
pub use async_storage::Storage;
#[cfg(feature = "blocking")]
pub use blocking_storage::BlockingStorage;
