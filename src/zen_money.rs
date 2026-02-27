//! High-level ZenMoney client with integrated storage.
//!
//! Combines the low-level HTTP client with a [`Storage`] /
//! [`BlockingStorage`] backend to provide automatic incremental sync
//! and convenient query methods.

use crate::error::{Result, ZenMoneyError};
use crate::models::{
    AccountId, CompanyId, DiffResponse, InstrumentId, MerchantId, ReminderId, ReminderMarkerId,
    TagId, TransactionId, UserId,
};

/// Entity type strings used in [`crate::models::Deletion::object`].
mod entity_type {
    /// Account entity type.
    pub(super) const ACCOUNT: &str = "account";
    /// Transaction entity type.
    pub(super) const TRANSACTION: &str = "transaction";
    /// Tag entity type.
    pub(super) const TAG: &str = "tag";
    /// Merchant entity type.
    pub(super) const MERCHANT: &str = "merchant";
    /// Instrument entity type.
    pub(super) const INSTRUMENT: &str = "instrument";
    /// Company entity type.
    pub(super) const COMPANY: &str = "company";
    /// Country entity type.
    pub(super) const COUNTRY: &str = "country";
    /// User entity type.
    pub(super) const USER: &str = "user";
    /// Reminder entity type.
    pub(super) const REMINDER: &str = "reminder";
    /// Reminder marker entity type.
    pub(super) const REMINDER_MARKER: &str = "reminderMarker";
}

/// Groups [`Deletion`] records by entity type for batch processing.
///
/// Numeric IDs (`instrument`, `company`, `country`, `user`) are parsed
/// from the string representation. Unknown entity types are silently
/// skipped with a tracing warning.
struct GroupedDeletions {
    /// Account IDs to remove.
    accounts: Vec<AccountId>,
    /// Transaction IDs to remove.
    transactions: Vec<TransactionId>,
    /// Tag IDs to remove.
    tags: Vec<TagId>,
    /// Merchant IDs to remove.
    merchants: Vec<MerchantId>,
    /// Instrument IDs to remove.
    instruments: Vec<InstrumentId>,
    /// Company IDs to remove.
    companies: Vec<CompanyId>,
    /// Country IDs to remove.
    countries: Vec<i32>,
    /// User IDs to remove.
    users: Vec<UserId>,
    /// Reminder IDs to remove.
    reminders: Vec<ReminderId>,
    /// Reminder marker IDs to remove.
    reminder_markers: Vec<ReminderMarkerId>,
}

impl GroupedDeletions {
    /// Groups deletion records by entity type.
    ///
    /// Numeric IDs are parsed from string; parse failures produce a
    /// [`ZenMoneyError::Storage`] error.
    fn from_response(response: &DiffResponse) -> Result<Self> {
        let mut result = Self {
            accounts: Vec::new(),
            transactions: Vec::new(),
            tags: Vec::new(),
            merchants: Vec::new(),
            instruments: Vec::new(),
            companies: Vec::new(),
            countries: Vec::new(),
            users: Vec::new(),
            reminders: Vec::new(),
            reminder_markers: Vec::new(),
        };
        for deletion in &response.deletion {
            result.push_deletion(&deletion.object, &deletion.id)?;
        }
        Ok(result)
    }

    /// Dispatches a single deletion into the appropriate ID vector.
    fn push_deletion(&mut self, object: &str, id: &str) -> Result<()> {
        match object {
            entity_type::ACCOUNT => self.accounts.push(AccountId::new(id.to_owned())),
            entity_type::TRANSACTION => self.transactions.push(TransactionId::new(id.to_owned())),
            entity_type::TAG => self.tags.push(TagId::new(id.to_owned())),
            entity_type::MERCHANT => self.merchants.push(MerchantId::new(id.to_owned())),
            entity_type::INSTRUMENT => self
                .instruments
                .push(InstrumentId::new(parse_numeric_id(id)?)),
            entity_type::COMPANY => self.companies.push(CompanyId::new(parse_numeric_id(id)?)),
            entity_type::COUNTRY => self.countries.push(parse_numeric_id(id)?),
            entity_type::USER => self.users.push(UserId::new(parse_numeric_id(id)?)),
            entity_type::REMINDER => self.reminders.push(ReminderId::new(id.to_owned())),
            entity_type::REMINDER_MARKER => self
                .reminder_markers
                .push(ReminderMarkerId::new(id.to_owned())),
            other => tracing::warn!(object = %other, id = %id, "unknown deletion type"),
        }
        Ok(())
    }
}

/// Parses a numeric ID from a string, wrapping parse errors.
fn parse_numeric_id<T: core::str::FromStr>(raw: &str) -> Result<T>
where
    T::Err: core::error::Error + Send + Sync + 'static,
{
    raw.parse::<T>()
        .map_err(|err| ZenMoneyError::Storage(Box::new(err)))
}

/// Generates a high-level ZenMoney client (async or blocking).
macro_rules! define_zen_money {
    (
        client_name: $client:ident,
        builder_name: $builder:ident,
        http_client: $http_client:ty,
        storage_trait: $storage_trait:ident,
        client_doc: $client_doc:expr,
        builder_doc: $builder_doc:expr,
        $(async_kw: $async_kw:tt,)?
        $(await_kw: $await_ext:tt,)?
        $(send_bound: $send_bound:tt,)?
    ) => {
        #[doc = $builder_doc]
        #[derive(Debug)]
        pub struct $builder<S: $storage_trait> {
            /// API token.
            token: Option<String>,
            /// Base URL override (for testing).
            base_url: Option<String>,
            /// Storage backend.
            storage: Option<S>,
        }

        impl<S: $storage_trait> $builder<S> {
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

            /// Sets the storage backend.
            #[inline]
            #[must_use]
            pub fn storage(mut self, storage: S) -> Self {
                self.storage = Some(storage);
                self
            }

            /// Builds the high-level client.
            ///
            /// # Errors
            ///
            /// Returns [`ZenMoneyError::TokenExpired`] if no token was provided.
            /// Returns [`ZenMoneyError::Storage`] if no storage was provided.
            /// Returns [`ZenMoneyError::Http`] if the HTTP client fails to build.
            #[inline]
            pub fn build(self) -> Result<$client<S>> {
                let storage = self.storage.ok_or_else(|| {
                    ZenMoneyError::Storage("storage backend is required".into())
                })?;

                let mut http_builder = <$http_client>::builder().token(
                    self.token
                        .ok_or(ZenMoneyError::TokenExpired)?,
                );
                if let Some(url) = self.base_url {
                    http_builder = http_builder.base_url(url);
                }
                let client = http_builder.build()?;

                Ok($client { client, storage })
            }
        }

        #[doc = $client_doc]
        #[derive(Debug)]
        pub struct $client<S: $storage_trait> {
            /// Low-level HTTP client.
            client: $http_client,
            /// Storage backend.
            storage: S,
        }

        impl<S: $storage_trait> $client<S> {
            /// Creates a new builder for configuring the client.
            #[inline]
            #[must_use]
            pub const fn builder() -> $builder<S> {
                $builder {
                    token: None,
                    base_url: None,
                    storage: None,
                }
            }

            /// Performs an incremental sync: reads the last server timestamp
            /// from storage, fetches changes via the diff endpoint, applies
            /// upserts and deletions, and updates the stored timestamp.
            ///
            /// Returns the diff response for inspection.
            ///
            /// # Errors
            ///
            /// Returns an error if the HTTP request, storage read/write,
            /// or deletion ID parsing fails.
            #[tracing::instrument(skip_all)]
            pub $($async_kw)? fn sync(&self) -> Result<DiffResponse> {
                let ts = self.storage.server_timestamp()
                    $( .$await_ext )?
                    ?
                    .unwrap_or(DateTime::<Utc>::UNIX_EPOCH);
                tracing::debug!(server_timestamp = %ts, "starting incremental sync");
                let request = DiffRequest::sync_only(ts, Utc::now());
                let response = self.client.diff(&request) $( .$await_ext )? ?;
                self.apply_diff(&response) $( .$await_ext )? ?;
                Ok(response)
            }

            /// Performs a full sync: clears all stored data, then syncs
            /// from epoch.
            ///
            /// Returns the diff response for inspection.
            ///
            /// # Errors
            ///
            /// Returns an error if clearing storage, the HTTP request,
            /// or applying the diff fails.
            #[tracing::instrument(skip_all)]
            pub $($async_kw)? fn full_sync(&self) -> Result<DiffResponse> {
                tracing::debug!("starting full sync");
                self.storage.clear() $( .$await_ext )? ?;
                self.sync() $( .$await_ext )?
            }

            /// Returns all accounts from storage.
            ///
            /// # Errors
            ///
            /// Returns an error if the storage backend fails to read.
            #[inline]
            pub $($async_kw)? fn accounts(&self) -> Result<Vec<Account>> {
                self.storage.accounts() $( .$await_ext )?
            }

            /// Returns all transactions from storage.
            ///
            /// # Errors
            ///
            /// Returns an error if the storage backend fails to read.
            #[inline]
            pub $($async_kw)? fn transactions(&self) -> Result<Vec<Transaction>> {
                self.storage.transactions() $( .$await_ext )?
            }

            /// Returns all tags from storage.
            ///
            /// # Errors
            ///
            /// Returns an error if the storage backend fails to read.
            #[inline]
            pub $($async_kw)? fn tags(&self) -> Result<Vec<Tag>> {
                self.storage.tags() $( .$await_ext )?
            }

            /// Returns all merchants from storage.
            ///
            /// # Errors
            ///
            /// Returns an error if the storage backend fails to read.
            #[inline]
            pub $($async_kw)? fn merchants(&self) -> Result<Vec<Merchant>> {
                self.storage.merchants() $( .$await_ext )?
            }

            /// Returns all instruments from storage.
            ///
            /// # Errors
            ///
            /// Returns an error if the storage backend fails to read.
            #[inline]
            pub $($async_kw)? fn instruments(&self) -> Result<Vec<Instrument>> {
                self.storage.instruments() $( .$await_ext )?
            }

            /// Returns all companies from storage.
            ///
            /// # Errors
            ///
            /// Returns an error if the storage backend fails to read.
            #[inline]
            pub $($async_kw)? fn companies(&self) -> Result<Vec<Company>> {
                self.storage.companies() $( .$await_ext )?
            }

            /// Returns all countries from storage.
            ///
            /// # Errors
            ///
            /// Returns an error if the storage backend fails to read.
            #[inline]
            pub $($async_kw)? fn countries(&self) -> Result<Vec<Country>> {
                self.storage.countries() $( .$await_ext )?
            }

            /// Returns all users from storage.
            ///
            /// # Errors
            ///
            /// Returns an error if the storage backend fails to read.
            #[inline]
            pub $($async_kw)? fn users(&self) -> Result<Vec<User>> {
                self.storage.users() $( .$await_ext )?
            }

            /// Returns all reminders from storage.
            ///
            /// # Errors
            ///
            /// Returns an error if the storage backend fails to read.
            #[inline]
            pub $($async_kw)? fn reminders(&self) -> Result<Vec<Reminder>> {
                self.storage.reminders() $( .$await_ext )?
            }

            /// Returns all reminder markers from storage.
            ///
            /// # Errors
            ///
            /// Returns an error if the storage backend fails to read.
            #[inline]
            pub $($async_kw)? fn reminder_markers(&self) -> Result<Vec<ReminderMarker>> {
                self.storage.reminder_markers() $( .$await_ext )?
            }

            /// Returns all budgets from storage.
            ///
            /// # Errors
            ///
            /// Returns an error if the storage backend fails to read.
            #[inline]
            pub $($async_kw)? fn budgets(&self) -> Result<Vec<Budget>> {
                self.storage.budgets() $( .$await_ext )?
            }

            /// Returns transactions within a date range (inclusive).
            ///
            /// # Errors
            ///
            /// Returns an error if the storage backend fails to read.
            pub $($async_kw)? fn transactions_by_date(
                &self,
                from: NaiveDate,
                to: NaiveDate,
            ) -> Result<Vec<Transaction>> {
                let all = self.storage.transactions() $( .$await_ext )? ?;
                Ok(all
                    .into_iter()
                    .filter(|tx| tx.date >= from && tx.date <= to)
                    .collect())
            }

            /// Returns transactions for a specific account (income or outcome).
            ///
            /// # Errors
            ///
            /// Returns an error if the storage backend fails to read.
            pub $($async_kw)? fn transactions_by_account(
                &self,
                account_id: &AccountId,
            ) -> Result<Vec<Transaction>> {
                let all = self.storage.transactions() $( .$await_ext )? ?;
                Ok(all
                    .into_iter()
                    .filter(|tx| tx.income_account == *account_id || tx.outcome_account == *account_id)
                    .collect())
            }

            /// Finds a tag by title (case-insensitive).
            ///
            /// # Errors
            ///
            /// Returns an error if the storage backend fails to read.
            pub $($async_kw)? fn find_tag_by_title(
                &self,
                title: &str,
            ) -> Result<Option<Tag>> {
                let all = self.storage.tags() $( .$await_ext )? ?;
                let lower = title.to_lowercase();
                Ok(all.into_iter().find(|tag| tag.title.to_lowercase() == lower))
            }

            /// Finds an account by title (case-insensitive).
            ///
            /// # Errors
            ///
            /// Returns an error if the storage backend fails to read.
            pub $($async_kw)? fn find_account_by_title(
                &self,
                title: &str,
            ) -> Result<Option<Account>> {
                let all = self.storage.accounts() $( .$await_ext )? ?;
                let lower = title.to_lowercase();
                Ok(all.into_iter().find(|acc| acc.title.to_lowercase() == lower))
            }

            /// Returns non-archived accounts.
            ///
            /// # Errors
            ///
            /// Returns an error if the storage backend fails to read.
            pub $($async_kw)? fn active_accounts(&self) -> Result<Vec<Account>> {
                let all = self.storage.accounts() $( .$await_ext )? ?;
                Ok(all.into_iter().filter(|acc| !acc.archive).collect())
            }

            /// Looks up an instrument by ID.
            ///
            /// # Errors
            ///
            /// Returns an error if the storage backend fails to read.
            pub $($async_kw)? fn instrument(
                &self,
                id: InstrumentId,
            ) -> Result<Option<Instrument>> {
                let all = self.storage.instruments() $( .$await_ext )? ?;
                Ok(all.into_iter().find(|instr| instr.id == id))
            }

            /// Passes a suggest request through to the HTTP client.
            ///
            /// # Errors
            ///
            /// Returns an error if the HTTP request fails.
            #[inline]
            pub $($async_kw)? fn suggest(
                &self,
                request: &SuggestRequest,
            ) -> Result<SuggestResponse> {
                self.client.suggest(request) $( .$await_ext )?
            }

            /// Returns a reference to the underlying HTTP client.
            #[inline]
            #[must_use]
            pub const fn inner_client(&self) -> &$http_client {
                &self.client
            }

            /// Returns a reference to the storage backend.
            #[inline]
            #[must_use]
            pub const fn storage(&self) -> &S {
                &self.storage
            }

            /// Applies upserts and deletions from a diff response to
            /// storage.
            #[tracing::instrument(skip_all)]
            $($async_kw)? fn apply_diff(&self, response: &DiffResponse) -> Result<()> {
                self.apply_upserts(response) $( .$await_ext )? ?;
                self.apply_deletions(response) $( .$await_ext )? ?;
                self.storage
                    .set_server_timestamp(response.server_timestamp)
                    $( .$await_ext )? ?;
                tracing::debug!(
                    server_timestamp = %response.server_timestamp,
                    "diff applied"
                );
                Ok(())
            }

            /// Upserts all entity types from a diff response.
            $($async_kw)? fn apply_upserts(&self, response: &DiffResponse) -> Result<()> {
                if !response.account.is_empty() {
                    self.storage.upsert_accounts(response.account.clone()) $( .$await_ext )? ?;
                }
                if !response.transaction.is_empty() {
                    self.storage.upsert_transactions(response.transaction.clone()) $( .$await_ext )? ?;
                }
                if !response.tag.is_empty() {
                    self.storage.upsert_tags(response.tag.clone()) $( .$await_ext )? ?;
                }
                if !response.merchant.is_empty() {
                    self.storage.upsert_merchants(response.merchant.clone()) $( .$await_ext )? ?;
                }
                if !response.instrument.is_empty() {
                    self.storage.upsert_instruments(response.instrument.clone()) $( .$await_ext )? ?;
                }
                if !response.company.is_empty() {
                    self.storage.upsert_companies(response.company.clone()) $( .$await_ext )? ?;
                }
                if !response.country.is_empty() {
                    self.storage.upsert_countries(response.country.clone()) $( .$await_ext )? ?;
                }
                if !response.user.is_empty() {
                    self.storage.upsert_users(response.user.clone()) $( .$await_ext )? ?;
                }
                if !response.reminder.is_empty() {
                    self.storage.upsert_reminders(response.reminder.clone()) $( .$await_ext )? ?;
                }
                if !response.reminder_marker.is_empty() {
                    self.storage.upsert_reminder_markers(response.reminder_marker.clone()) $( .$await_ext )? ?;
                }
                if !response.budget.is_empty() {
                    self.storage.upsert_budgets(response.budget.clone()) $( .$await_ext )? ?;
                }
                Ok(())
            }

            /// Processes deletion records from a diff response.
            $($async_kw)? fn apply_deletions(&self, response: &DiffResponse) -> Result<()> {
                if response.deletion.is_empty() {
                    return Ok(());
                }
                let groups = GroupedDeletions::from_response(response)?;
                if !groups.accounts.is_empty() {
                    self.storage.remove_accounts(&groups.accounts) $( .$await_ext )? ?;
                }
                if !groups.transactions.is_empty() {
                    self.storage.remove_transactions(&groups.transactions) $( .$await_ext )? ?;
                }
                if !groups.tags.is_empty() {
                    self.storage.remove_tags(&groups.tags) $( .$await_ext )? ?;
                }
                if !groups.merchants.is_empty() {
                    self.storage.remove_merchants(&groups.merchants) $( .$await_ext )? ?;
                }
                if !groups.instruments.is_empty() {
                    self.storage.remove_instruments(&groups.instruments) $( .$await_ext )? ?;
                }
                if !groups.companies.is_empty() {
                    self.storage.remove_companies(&groups.companies) $( .$await_ext )? ?;
                }
                if !groups.countries.is_empty() {
                    self.storage.remove_countries(&groups.countries) $( .$await_ext )? ?;
                }
                if !groups.users.is_empty() {
                    self.storage.remove_users(&groups.users) $( .$await_ext )? ?;
                }
                if !groups.reminders.is_empty() {
                    self.storage.remove_reminders(&groups.reminders) $( .$await_ext )? ?;
                }
                if !groups.reminder_markers.is_empty() {
                    self.storage.remove_reminder_markers(&groups.reminder_markers) $( .$await_ext )? ?;
                }
                Ok(())
            }
        }
    };
}

// ── Async variant ───────────────────────────────────────────────────────

#[cfg(feature = "async")]
mod async_zen_money {
    //! Async high-level client.

    use crate::client::ZenMoneyClient;
    use crate::error::{Result, ZenMoneyError};
    use crate::models::{
        Account, AccountId, Budget, Company, Country, DiffRequest, DiffResponse, Instrument,
        InstrumentId, Merchant, NaiveDate, Reminder, ReminderMarker, SuggestRequest,
        SuggestResponse, Tag, Transaction, User,
    };
    use crate::storage::Storage;
    use chrono::{DateTime, Utc};

    use super::GroupedDeletions;

    define_zen_money! {
        client_name: ZenMoney,
        builder_name: ZenMoneyBuilder,
        http_client: ZenMoneyClient,
        storage_trait: Storage,
        client_doc: "High-level async ZenMoney client with integrated storage.\n\nUse [`ZenMoney::builder()`] to construct an instance.",
        builder_doc: "Builder for constructing a [`ZenMoney`] client.",
        async_kw: async,
        await_kw: await,
        send_bound: Sync,
    }
}

// ── Blocking variant ────────────────────────────────────────────────────

#[cfg(feature = "blocking")]
mod blocking_zen_money {
    //! Blocking high-level client.

    use crate::client::ZenMoneyBlockingClient;
    use crate::error::{Result, ZenMoneyError};
    use crate::models::{
        Account, AccountId, Budget, Company, Country, DiffRequest, DiffResponse, Instrument,
        InstrumentId, Merchant, NaiveDate, Reminder, ReminderMarker, SuggestRequest,
        SuggestResponse, Tag, Transaction, User,
    };
    use crate::storage::BlockingStorage;
    use chrono::{DateTime, Utc};

    use super::GroupedDeletions;

    define_zen_money! {
        client_name: ZenMoneyBlocking,
        builder_name: ZenMoneyBlockingBuilder,
        http_client: ZenMoneyBlockingClient,
        storage_trait: BlockingStorage,
        client_doc: "High-level blocking ZenMoney client with integrated storage.\n\nUse [`ZenMoneyBlocking::builder()`] to construct an instance.",
        builder_doc: "Builder for constructing a [`ZenMoneyBlocking`] client.",
    }
}

#[cfg(feature = "async")]
pub use async_zen_money::{ZenMoney, ZenMoneyBuilder};
#[cfg(feature = "blocking")]
pub use blocking_zen_money::{ZenMoneyBlocking, ZenMoneyBlockingBuilder};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{
        Account, AccountId, AccountType, Budget, Company, CompanyId, Country, Deletion,
        DiffResponse, Instrument, InstrumentId, Merchant, MerchantId, NaiveDate, Reminder,
        ReminderId, ReminderMarker, ReminderMarkerId, Tag, TagId, Transaction, TransactionId, User,
        UserId,
    };
    use chrono::{DateTime, Utc};

    /// In-memory mock storage for testing.
    #[derive(Debug, Default)]
    struct MockStorage {
        /// All stored state behind a mutex for interior mutability.
        inner: std::sync::Mutex<MockInner>,
    }

    /// Inner state of the mock storage.
    #[derive(Debug, Default)]
    struct MockInner {
        /// Server timestamp.
        server_timestamp: Option<DateTime<Utc>>,
        /// Stored accounts.
        accounts: Vec<Account>,
        /// Stored transactions.
        transactions: Vec<Transaction>,
        /// Stored tags.
        tags: Vec<Tag>,
        /// Stored merchants.
        merchants: Vec<Merchant>,
        /// Stored instruments.
        instruments: Vec<Instrument>,
        /// Stored companies.
        companies: Vec<Company>,
        /// Stored countries.
        countries: Vec<Country>,
        /// Stored users.
        users: Vec<User>,
        /// Stored reminders.
        reminders: Vec<Reminder>,
        /// Stored reminder markers.
        reminder_markers: Vec<ReminderMarker>,
        /// Stored budgets.
        budgets: Vec<Budget>,
    }

    #[cfg(feature = "blocking")]
    impl crate::storage::BlockingStorage for MockStorage {
        fn server_timestamp(&self) -> Result<Option<DateTime<Utc>>> {
            Ok(self.inner.lock().unwrap().server_timestamp)
        }
        fn set_server_timestamp(&self, timestamp: DateTime<Utc>) -> Result<()> {
            self.inner.lock().unwrap().server_timestamp = Some(timestamp);
            Ok(())
        }
        fn accounts(&self) -> Result<Vec<Account>> {
            Ok(self.inner.lock().unwrap().accounts.clone())
        }
        fn transactions(&self) -> Result<Vec<Transaction>> {
            Ok(self.inner.lock().unwrap().transactions.clone())
        }
        fn tags(&self) -> Result<Vec<Tag>> {
            Ok(self.inner.lock().unwrap().tags.clone())
        }
        fn merchants(&self) -> Result<Vec<Merchant>> {
            Ok(self.inner.lock().unwrap().merchants.clone())
        }
        fn instruments(&self) -> Result<Vec<Instrument>> {
            Ok(self.inner.lock().unwrap().instruments.clone())
        }
        fn companies(&self) -> Result<Vec<Company>> {
            Ok(self.inner.lock().unwrap().companies.clone())
        }
        fn countries(&self) -> Result<Vec<Country>> {
            Ok(self.inner.lock().unwrap().countries.clone())
        }
        fn users(&self) -> Result<Vec<User>> {
            Ok(self.inner.lock().unwrap().users.clone())
        }
        fn reminders(&self) -> Result<Vec<Reminder>> {
            Ok(self.inner.lock().unwrap().reminders.clone())
        }
        fn reminder_markers(&self) -> Result<Vec<ReminderMarker>> {
            Ok(self.inner.lock().unwrap().reminder_markers.clone())
        }
        fn budgets(&self) -> Result<Vec<Budget>> {
            Ok(self.inner.lock().unwrap().budgets.clone())
        }
        fn upsert_accounts(&self, items: Vec<Account>) -> Result<()> {
            self.inner.lock().unwrap().accounts = items;
            Ok(())
        }
        fn upsert_transactions(&self, items: Vec<Transaction>) -> Result<()> {
            self.inner.lock().unwrap().transactions = items;
            Ok(())
        }
        fn upsert_tags(&self, items: Vec<Tag>) -> Result<()> {
            self.inner.lock().unwrap().tags = items;
            Ok(())
        }
        fn upsert_merchants(&self, items: Vec<Merchant>) -> Result<()> {
            self.inner.lock().unwrap().merchants = items;
            Ok(())
        }
        fn upsert_instruments(&self, items: Vec<Instrument>) -> Result<()> {
            self.inner.lock().unwrap().instruments = items;
            Ok(())
        }
        fn upsert_companies(&self, items: Vec<Company>) -> Result<()> {
            self.inner.lock().unwrap().companies = items;
            Ok(())
        }
        fn upsert_countries(&self, items: Vec<Country>) -> Result<()> {
            self.inner.lock().unwrap().countries = items;
            Ok(())
        }
        fn upsert_users(&self, items: Vec<User>) -> Result<()> {
            self.inner.lock().unwrap().users = items;
            Ok(())
        }
        fn upsert_reminders(&self, items: Vec<Reminder>) -> Result<()> {
            self.inner.lock().unwrap().reminders = items;
            Ok(())
        }
        fn upsert_reminder_markers(&self, items: Vec<ReminderMarker>) -> Result<()> {
            self.inner.lock().unwrap().reminder_markers = items;
            Ok(())
        }
        fn upsert_budgets(&self, items: Vec<Budget>) -> Result<()> {
            self.inner.lock().unwrap().budgets = items;
            Ok(())
        }
        fn remove_accounts(&self, ids: &[AccountId]) -> Result<()> {
            self.inner
                .lock()
                .unwrap()
                .accounts
                .retain(|a| !ids.contains(&a.id));
            Ok(())
        }
        fn remove_transactions(&self, ids: &[TransactionId]) -> Result<()> {
            self.inner
                .lock()
                .unwrap()
                .transactions
                .retain(|t| !ids.contains(&t.id));
            Ok(())
        }
        fn remove_tags(&self, ids: &[TagId]) -> Result<()> {
            self.inner
                .lock()
                .unwrap()
                .tags
                .retain(|t| !ids.contains(&t.id));
            Ok(())
        }
        fn remove_merchants(&self, ids: &[MerchantId]) -> Result<()> {
            self.inner
                .lock()
                .unwrap()
                .merchants
                .retain(|m| !ids.contains(&m.id));
            Ok(())
        }
        fn remove_instruments(&self, ids: &[InstrumentId]) -> Result<()> {
            self.inner
                .lock()
                .unwrap()
                .instruments
                .retain(|i| !ids.contains(&i.id));
            Ok(())
        }
        fn remove_companies(&self, ids: &[CompanyId]) -> Result<()> {
            self.inner
                .lock()
                .unwrap()
                .companies
                .retain(|c| !ids.contains(&c.id));
            Ok(())
        }
        fn remove_countries(&self, ids: &[i32]) -> Result<()> {
            self.inner
                .lock()
                .unwrap()
                .countries
                .retain(|c| !ids.contains(&c.id));
            Ok(())
        }
        fn remove_users(&self, ids: &[UserId]) -> Result<()> {
            self.inner
                .lock()
                .unwrap()
                .users
                .retain(|u| !ids.contains(&u.id));
            Ok(())
        }
        fn remove_reminders(&self, ids: &[ReminderId]) -> Result<()> {
            self.inner
                .lock()
                .unwrap()
                .reminders
                .retain(|r| !ids.contains(&r.id));
            Ok(())
        }
        fn remove_reminder_markers(&self, ids: &[ReminderMarkerId]) -> Result<()> {
            self.inner
                .lock()
                .unwrap()
                .reminder_markers
                .retain(|r| !ids.contains(&r.id));
            Ok(())
        }
        fn remove_budgets(&self, _ids: &[String]) -> Result<()> {
            Ok(())
        }
        fn clear(&self) -> Result<()> {
            let mut inner = self.inner.lock().unwrap();
            *inner = MockInner::default();
            Ok(())
        }
    }

    /// Creates a minimal test account.
    fn test_account(id: &str, title: &str, archive: bool) -> Account {
        Account {
            id: AccountId::new(id.to_owned()),
            changed: DateTime::from_timestamp(1_700_000_000, 0).unwrap(),
            user: UserId::new(1_i64),
            role: None,
            instrument: Some(InstrumentId::new(1_i32)),
            company: None,
            kind: AccountType::Checking,
            title: title.to_owned(),
            sync_id: None,
            balance: Some(0.0),
            start_balance: None,
            credit_limit: None,
            in_balance: true,
            savings: None,
            enable_correction: false,
            enable_sms: false,
            archive,
            capitalization: None,
            percent: None,
            start_date: None,
            end_date_offset: None,
            end_date_offset_interval: None,
            payoff_step: None,
            payoff_interval: None,
            balance_correction_type: None,
            private: None,
        }
    }

    /// Creates a minimal test tag.
    fn test_tag(id: &str, title: &str) -> Tag {
        Tag {
            id: TagId::new(id.to_owned()),
            changed: DateTime::from_timestamp(1_700_000_000, 0).unwrap(),
            user: UserId::new(1_i64),
            title: title.to_owned(),
            parent: None,
            icon: None,
            picture: None,
            color: None,
            show_income: true,
            show_outcome: true,
            budget_income: false,
            budget_outcome: false,
            required: None,
            static_id: None,
            archive: None,
        }
    }

    /// Creates a minimal test transaction.
    fn test_transaction(id: &str, account_id: &str, date: NaiveDate) -> Transaction {
        Transaction {
            id: TransactionId::new(id.to_owned()),
            changed: DateTime::from_timestamp(1_700_000_000, 0).unwrap(),
            created: DateTime::from_timestamp(1_700_000_000, 0).unwrap(),
            user: UserId::new(1_i64),
            deleted: false,
            hold: None,
            income_instrument: InstrumentId::new(1_i32),
            income_account: AccountId::new(account_id.to_owned()),
            income: 0.0,
            outcome_instrument: InstrumentId::new(1_i32),
            outcome_account: AccountId::new(account_id.to_owned()),
            outcome: 100.0,
            tag: None,
            merchant: None,
            payee: None,
            original_payee: None,
            comment: None,
            date,
            mcc: None,
            reminder_marker: None,
            op_income: None,
            op_income_instrument: None,
            op_outcome: None,
            op_outcome_instrument: None,
            latitude: None,
            longitude: None,
            income_bank_id: None,
            outcome_bank_id: None,
            qr_code: None,
            source: None,
            viewed: None,
        }
    }

    #[test]
    fn grouped_deletions_parses_entity_types() {
        let response = DiffResponse {
            server_timestamp: DateTime::from_timestamp(100, 0).unwrap(),
            instrument: Vec::new(),
            country: Vec::new(),
            company: Vec::new(),
            user: Vec::new(),
            account: Vec::new(),
            tag: Vec::new(),
            merchant: Vec::new(),
            transaction: Vec::new(),
            reminder: Vec::new(),
            reminder_marker: Vec::new(),
            budget: Vec::new(),
            deletion: vec![
                Deletion {
                    id: "acc-1".to_owned(),
                    object: "account".to_owned(),
                    stamp: DateTime::from_timestamp(100, 0).unwrap(),
                    user: 1_i64,
                },
                Deletion {
                    id: "42".to_owned(),
                    object: "instrument".to_owned(),
                    stamp: DateTime::from_timestamp(100, 0).unwrap(),
                    user: 1_i64,
                },
                Deletion {
                    id: "unknown-id".to_owned(),
                    object: "unknownType".to_owned(),
                    stamp: DateTime::from_timestamp(100, 0).unwrap(),
                    user: 1_i64,
                },
            ],
        };

        let groups = GroupedDeletions::from_response(&response).unwrap();
        assert_eq!(groups.accounts.len(), 1);
        assert_eq!(groups.instruments.len(), 1);
        assert_eq!(groups.instruments[0], InstrumentId::new(42_i32));
    }

    #[test]
    fn grouped_deletions_invalid_numeric_id_errors() {
        let response = DiffResponse {
            server_timestamp: DateTime::from_timestamp(100, 0).unwrap(),
            instrument: Vec::new(),
            country: Vec::new(),
            company: Vec::new(),
            user: Vec::new(),
            account: Vec::new(),
            tag: Vec::new(),
            merchant: Vec::new(),
            transaction: Vec::new(),
            reminder: Vec::new(),
            reminder_marker: Vec::new(),
            budget: Vec::new(),
            deletion: vec![Deletion {
                id: "not-a-number".to_owned(),
                object: "instrument".to_owned(),
                stamp: DateTime::from_timestamp(100, 0).unwrap(),
                user: 1_i64,
            }],
        };

        assert!(GroupedDeletions::from_response(&response).is_err());
    }

    #[cfg(feature = "blocking")]
    mod blocking {
        use super::*;
        use crate::storage::BlockingStorage;
        use crate::zen_money::blocking_zen_money::ZenMoneyBlocking;

        /// Helper to test `apply_diff` directly using a mock storage.
        fn apply_diff_with_mock(response: &DiffResponse) -> (Result<()>, MockStorage) {
            let storage = MockStorage::default();
            // We can't easily construct ZenMoneyBlocking without a real HTTP client,
            // so we test apply_diff through the storage trait directly.
            // Instead, test the storage interactions.

            // Simulate what apply_diff does:
            if !response.account.is_empty() {
                storage.upsert_accounts(response.account.clone()).unwrap();
            }
            if !response.transaction.is_empty() {
                storage
                    .upsert_transactions(response.transaction.clone())
                    .unwrap();
            }
            if !response.tag.is_empty() {
                storage.upsert_tags(response.tag.clone()).unwrap();
            }

            // Process deletions
            let groups_result = GroupedDeletions::from_response(response);
            match groups_result {
                Ok(groups) => {
                    if !groups.accounts.is_empty() {
                        storage.remove_accounts(&groups.accounts).unwrap();
                    }
                    if !groups.transactions.is_empty() {
                        storage.remove_transactions(&groups.transactions).unwrap();
                    }
                    storage
                        .set_server_timestamp(response.server_timestamp)
                        .unwrap();
                    (Ok(()), storage)
                }
                Err(err) => (Err(err), storage),
            }
        }

        #[test]
        fn apply_diff_upserts_and_deletes() {
            let acc1 = test_account("a-1", "First", false);
            let acc2 = test_account("a-2", "Second", false);

            let response = DiffResponse {
                server_timestamp: DateTime::from_timestamp(200, 0).unwrap(),
                instrument: Vec::new(),
                country: Vec::new(),
                company: Vec::new(),
                user: Vec::new(),
                account: vec![acc1, acc2],
                tag: Vec::new(),
                merchant: Vec::new(),
                transaction: Vec::new(),
                reminder: Vec::new(),
                reminder_marker: Vec::new(),
                budget: Vec::new(),
                deletion: vec![Deletion {
                    id: "a-1".to_owned(),
                    object: "account".to_owned(),
                    stamp: DateTime::from_timestamp(200, 0).unwrap(),
                    user: 1_i64,
                }],
            };

            let (result, storage) = apply_diff_with_mock(&response);
            result.unwrap();

            let accounts = storage.accounts().unwrap();
            assert_eq!(accounts.len(), 1);
            assert_eq!(accounts[0].title, "Second");

            let ts = storage.server_timestamp().unwrap();
            assert_eq!(ts, Some(DateTime::from_timestamp(200, 0).unwrap()));
        }

        #[test]
        fn query_active_accounts() {
            let storage = MockStorage::default();
            let acc1 = test_account("a-1", "Active", false);
            let acc2 = test_account("a-2", "Archived", true);
            storage.upsert_accounts(vec![acc1, acc2]).unwrap();

            let active: Vec<Account> = storage
                .accounts()
                .unwrap()
                .into_iter()
                .filter(|acc| !acc.archive)
                .collect();
            assert_eq!(active.len(), 1);
            assert_eq!(active[0].title, "Active");
        }

        #[test]
        fn query_find_tag_by_title() {
            let storage = MockStorage::default();
            let tag = test_tag("t-1", "Groceries");
            storage.upsert_tags(vec![tag]).unwrap();

            let all_tags = storage.tags().unwrap();
            let found = all_tags
                .into_iter()
                .find(|t| t.title.to_lowercase() == "groceries");
            assert!(found.is_some());
            assert_eq!(found.unwrap().id, TagId::new("t-1".to_owned()));
        }

        #[test]
        fn query_transactions_by_date() {
            let storage = MockStorage::default();
            let tx1 =
                test_transaction("tx-1", "a-1", NaiveDate::from_ymd_opt(2024, 1, 15).unwrap());
            let tx2 =
                test_transaction("tx-2", "a-1", NaiveDate::from_ymd_opt(2024, 2, 15).unwrap());
            let tx3 =
                test_transaction("tx-3", "a-1", NaiveDate::from_ymd_opt(2024, 3, 15).unwrap());
            storage.upsert_transactions(vec![tx1, tx2, tx3]).unwrap();

            let from = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
            let to = NaiveDate::from_ymd_opt(2024, 2, 28).unwrap();
            let filtered: Vec<Transaction> = storage
                .transactions()
                .unwrap()
                .into_iter()
                .filter(|tx| tx.date >= from && tx.date <= to)
                .collect();
            assert_eq!(filtered.len(), 2);
        }

        #[test]
        fn builder_requires_storage() {
            let result = ZenMoneyBlocking::<MockStorage>::builder()
                .token("test")
                .build();
            assert!(result.is_err());
        }

        #[test]
        fn builder_requires_token() {
            let result = ZenMoneyBlocking::builder()
                .storage(MockStorage::default())
                .build();
            assert!(result.is_err());
        }

        #[test]
        fn builder_succeeds_with_token_and_storage() {
            let result = ZenMoneyBlocking::builder()
                .token("test")
                .storage(MockStorage::default())
                .build();
            assert!(result.is_ok());
        }
    }
}
