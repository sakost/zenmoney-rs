//! In-memory storage backend for testing.
//!
//! Provides [`InMemoryStorage`], a thread-safe in-memory implementation of
//! the storage traits. Ideal for unit and integration tests where file I/O
//! is undesirable.

use core::hash::Hash;
use std::collections::HashMap;
use std::sync::Mutex;

use chrono::{DateTime, Utc};

#[cfg(feature = "async")]
use core::future::{self, Future};

use crate::error::{Result, ZenMoneyError};
use crate::models::{
    Account, AccountId, Budget, Company, CompanyId, Country, Instrument, InstrumentId, Merchant,
    MerchantId, NaiveDate, Reminder, ReminderId, ReminderMarker, ReminderMarkerId, Tag, TagId,
    Transaction, TransactionId, User, UserId,
};

/// Constant timestamp for test helpers.
#[cfg(test)]
const TEST_TIMESTAMP_SECS: i64 = 1_700_000_000;

/// Thread-safe in-memory storage for testing.
///
/// This type implements both [`super::Storage`] (async) and
/// [`super::BlockingStorage`] (blocking) traits, providing a zero-setup
/// storage backend for tests.
///
/// # Upsert semantics
///
/// Like [`super::FileStorage`], upserts merge by key: existing items with
/// matching IDs are replaced, new items are appended.
///
/// # Example
///
/// ```rust
/// use zenmoney_rs::storage::InMemoryStorage;
///
/// let storage = InMemoryStorage::new();
/// // Use with ZenMoney or ZenMoneyBlocking builders:
/// // ZenMoneyBlocking::builder().storage(storage).token("...").build()
/// ```
#[derive(Debug, Default)]
pub struct InMemoryStorage {
    /// All state behind a single mutex for thread-safe interior mutability.
    inner: Mutex<Inner>,
}

/// Inner mutable state.
#[derive(Debug, Default)]
struct Inner {
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

impl InMemoryStorage {
    /// Creates a new empty in-memory storage.
    #[inline]
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Acquires the inner lock and applies a closure.
    fn with_lock<R>(&self, f: impl FnOnce(&mut Inner) -> R) -> Result<R> {
        let mut inner = self.inner.lock().map_err(|err| lock_error(&err))?;
        Ok(f(&mut inner))
    }
}

/// Merges `new_items` into `existing` by key, replacing duplicates.
fn upsert_by_key<T, K>(existing: &mut Vec<T>, new_items: Vec<T>, key_fn: fn(&T) -> K)
where
    K: Hash + Eq,
{
    let mut map: HashMap<K, T> = HashMap::with_capacity(existing.len() + new_items.len());
    for item in existing.drain(..) {
        let key = key_fn(&item);
        let _old = map.insert(key, item);
    }
    for item in new_items {
        let key = key_fn(&item);
        let _old = map.insert(key, item);
    }
    *existing = map.into_values().collect();
}

/// Removes items whose key is in `ids`.
fn remove_by_key<T, K>(existing: &mut Vec<T>, ids: &[K], key_fn: fn(&T) -> K)
where
    K: Hash + Eq,
{
    let id_set: std::collections::HashSet<&K> = ids.iter().collect();
    existing.retain(|item| !id_set.contains(&key_fn(item)));
}

/// Wraps a mutex poison error.
fn lock_error<T>(err: &std::sync::PoisonError<T>) -> ZenMoneyError {
    ZenMoneyError::Storage(err.to_string().into())
}

/// Extracts the budget composite key.
fn budget_key(budget: &Budget) -> (UserId, Option<TagId>, NaiveDate) {
    (budget.user, budget.tag.clone(), budget.date)
}

// ── BlockingStorage implementation ──────────────────────────────────────

#[cfg(feature = "blocking")]
impl super::BlockingStorage for InMemoryStorage {
    #[inline]
    fn server_timestamp(&self) -> Result<Option<DateTime<Utc>>> {
        self.with_lock(|inner| inner.server_timestamp)
    }

    #[inline]
    fn set_server_timestamp(&self, timestamp: DateTime<Utc>) -> Result<()> {
        self.with_lock(|inner| inner.server_timestamp = Some(timestamp))
    }

    #[inline]
    fn accounts(&self) -> Result<Vec<Account>> {
        self.with_lock(|inner| inner.accounts.clone())
    }

    #[inline]
    fn transactions(&self) -> Result<Vec<Transaction>> {
        self.with_lock(|inner| inner.transactions.clone())
    }

    #[inline]
    fn tags(&self) -> Result<Vec<Tag>> {
        self.with_lock(|inner| inner.tags.clone())
    }

    #[inline]
    fn merchants(&self) -> Result<Vec<Merchant>> {
        self.with_lock(|inner| inner.merchants.clone())
    }

    #[inline]
    fn instruments(&self) -> Result<Vec<Instrument>> {
        self.with_lock(|inner| inner.instruments.clone())
    }

    #[inline]
    fn companies(&self) -> Result<Vec<Company>> {
        self.with_lock(|inner| inner.companies.clone())
    }

    #[inline]
    fn countries(&self) -> Result<Vec<Country>> {
        self.with_lock(|inner| inner.countries.clone())
    }

    #[inline]
    fn users(&self) -> Result<Vec<User>> {
        self.with_lock(|inner| inner.users.clone())
    }

    #[inline]
    fn reminders(&self) -> Result<Vec<Reminder>> {
        self.with_lock(|inner| inner.reminders.clone())
    }

    #[inline]
    fn reminder_markers(&self) -> Result<Vec<ReminderMarker>> {
        self.with_lock(|inner| inner.reminder_markers.clone())
    }

    #[inline]
    fn budgets(&self) -> Result<Vec<Budget>> {
        self.with_lock(|inner| inner.budgets.clone())
    }

    #[inline]
    fn upsert_accounts(&self, items: Vec<Account>) -> Result<()> {
        self.with_lock(|inner| upsert_by_key(&mut inner.accounts, items, |a| a.id.clone()))
    }

    #[inline]
    fn upsert_transactions(&self, items: Vec<Transaction>) -> Result<()> {
        self.with_lock(|inner| upsert_by_key(&mut inner.transactions, items, |t| t.id.clone()))
    }

    #[inline]
    fn upsert_tags(&self, items: Vec<Tag>) -> Result<()> {
        self.with_lock(|inner| upsert_by_key(&mut inner.tags, items, |t| t.id.clone()))
    }

    #[inline]
    fn upsert_merchants(&self, items: Vec<Merchant>) -> Result<()> {
        self.with_lock(|inner| upsert_by_key(&mut inner.merchants, items, |m| m.id.clone()))
    }

    #[inline]
    fn upsert_instruments(&self, items: Vec<Instrument>) -> Result<()> {
        self.with_lock(|inner| upsert_by_key(&mut inner.instruments, items, |i| i.id))
    }

    #[inline]
    fn upsert_companies(&self, items: Vec<Company>) -> Result<()> {
        self.with_lock(|inner| upsert_by_key(&mut inner.companies, items, |c| c.id))
    }

    #[inline]
    fn upsert_countries(&self, items: Vec<Country>) -> Result<()> {
        self.with_lock(|inner| upsert_by_key(&mut inner.countries, items, |c| c.id))
    }

    #[inline]
    fn upsert_users(&self, items: Vec<User>) -> Result<()> {
        self.with_lock(|inner| upsert_by_key(&mut inner.users, items, |u| u.id))
    }

    #[inline]
    fn upsert_reminders(&self, items: Vec<Reminder>) -> Result<()> {
        self.with_lock(|inner| upsert_by_key(&mut inner.reminders, items, |r| r.id.clone()))
    }

    #[inline]
    fn upsert_reminder_markers(&self, items: Vec<ReminderMarker>) -> Result<()> {
        self.with_lock(|inner| upsert_by_key(&mut inner.reminder_markers, items, |r| r.id.clone()))
    }

    #[inline]
    fn upsert_budgets(&self, items: Vec<Budget>) -> Result<()> {
        self.with_lock(|inner| upsert_by_key(&mut inner.budgets, items, budget_key))
    }

    #[inline]
    fn remove_accounts(&self, ids: &[AccountId]) -> Result<()> {
        self.with_lock(|inner| remove_by_key(&mut inner.accounts, ids, |a| a.id.clone()))
    }

    #[inline]
    fn remove_transactions(&self, ids: &[TransactionId]) -> Result<()> {
        self.with_lock(|inner| remove_by_key(&mut inner.transactions, ids, |t| t.id.clone()))
    }

    #[inline]
    fn remove_tags(&self, ids: &[TagId]) -> Result<()> {
        self.with_lock(|inner| remove_by_key(&mut inner.tags, ids, |t| t.id.clone()))
    }

    #[inline]
    fn remove_merchants(&self, ids: &[MerchantId]) -> Result<()> {
        self.with_lock(|inner| remove_by_key(&mut inner.merchants, ids, |m| m.id.clone()))
    }

    #[inline]
    fn remove_instruments(&self, ids: &[InstrumentId]) -> Result<()> {
        self.with_lock(|inner| remove_by_key(&mut inner.instruments, ids, |i| i.id))
    }

    #[inline]
    fn remove_companies(&self, ids: &[CompanyId]) -> Result<()> {
        self.with_lock(|inner| remove_by_key(&mut inner.companies, ids, |c| c.id))
    }

    #[inline]
    fn remove_countries(&self, ids: &[i32]) -> Result<()> {
        self.with_lock(|inner| remove_by_key(&mut inner.countries, ids, |c| c.id))
    }

    #[inline]
    fn remove_users(&self, ids: &[UserId]) -> Result<()> {
        self.with_lock(|inner| remove_by_key(&mut inner.users, ids, |u| u.id))
    }

    #[inline]
    fn remove_reminders(&self, ids: &[ReminderId]) -> Result<()> {
        self.with_lock(|inner| remove_by_key(&mut inner.reminders, ids, |r| r.id.clone()))
    }

    #[inline]
    fn remove_reminder_markers(&self, ids: &[ReminderMarkerId]) -> Result<()> {
        self.with_lock(|inner| remove_by_key(&mut inner.reminder_markers, ids, |r| r.id.clone()))
    }

    #[inline]
    fn remove_budgets(&self, _ids: &[String]) -> Result<()> {
        // Budget deletions use composite keys; raw ID string matching
        // is not straightforward. Left as no-op, matching FileStorage.
        Ok(())
    }

    #[inline]
    fn clear(&self) -> Result<()> {
        self.with_lock(|inner| *inner = Inner::default())
    }
}

// ── Storage (async) implementation ──────────────────────────────────────

#[cfg(feature = "async")]
impl super::Storage for InMemoryStorage {
    #[inline]
    fn server_timestamp(&self) -> impl Future<Output = Result<Option<DateTime<Utc>>>> + Send {
        future::ready(self.with_lock(|inner| inner.server_timestamp))
    }

    #[inline]
    fn set_server_timestamp(
        &self,
        timestamp: DateTime<Utc>,
    ) -> impl Future<Output = Result<()>> + Send {
        future::ready(self.with_lock(|inner| inner.server_timestamp = Some(timestamp)))
    }

    #[inline]
    fn accounts(&self) -> impl Future<Output = Result<Vec<Account>>> + Send {
        future::ready(self.with_lock(|inner| inner.accounts.clone()))
    }

    #[inline]
    fn transactions(&self) -> impl Future<Output = Result<Vec<Transaction>>> + Send {
        future::ready(self.with_lock(|inner| inner.transactions.clone()))
    }

    #[inline]
    fn tags(&self) -> impl Future<Output = Result<Vec<Tag>>> + Send {
        future::ready(self.with_lock(|inner| inner.tags.clone()))
    }

    #[inline]
    fn merchants(&self) -> impl Future<Output = Result<Vec<Merchant>>> + Send {
        future::ready(self.with_lock(|inner| inner.merchants.clone()))
    }

    #[inline]
    fn instruments(&self) -> impl Future<Output = Result<Vec<Instrument>>> + Send {
        future::ready(self.with_lock(|inner| inner.instruments.clone()))
    }

    #[inline]
    fn companies(&self) -> impl Future<Output = Result<Vec<Company>>> + Send {
        future::ready(self.with_lock(|inner| inner.companies.clone()))
    }

    #[inline]
    fn countries(&self) -> impl Future<Output = Result<Vec<Country>>> + Send {
        future::ready(self.with_lock(|inner| inner.countries.clone()))
    }

    #[inline]
    fn users(&self) -> impl Future<Output = Result<Vec<User>>> + Send {
        future::ready(self.with_lock(|inner| inner.users.clone()))
    }

    #[inline]
    fn reminders(&self) -> impl Future<Output = Result<Vec<Reminder>>> + Send {
        future::ready(self.with_lock(|inner| inner.reminders.clone()))
    }

    #[inline]
    fn reminder_markers(&self) -> impl Future<Output = Result<Vec<ReminderMarker>>> + Send {
        future::ready(self.with_lock(|inner| inner.reminder_markers.clone()))
    }

    #[inline]
    fn budgets(&self) -> impl Future<Output = Result<Vec<Budget>>> + Send {
        future::ready(self.with_lock(|inner| inner.budgets.clone()))
    }

    #[inline]
    fn upsert_accounts(&self, items: Vec<Account>) -> impl Future<Output = Result<()>> + Send {
        future::ready(
            self.with_lock(|inner| upsert_by_key(&mut inner.accounts, items, |a| a.id.clone())),
        )
    }

    #[inline]
    fn upsert_transactions(
        &self,
        items: Vec<Transaction>,
    ) -> impl Future<Output = Result<()>> + Send {
        future::ready(
            self.with_lock(|inner| upsert_by_key(&mut inner.transactions, items, |t| t.id.clone())),
        )
    }

    #[inline]
    fn upsert_tags(&self, items: Vec<Tag>) -> impl Future<Output = Result<()>> + Send {
        future::ready(
            self.with_lock(|inner| upsert_by_key(&mut inner.tags, items, |t| t.id.clone())),
        )
    }

    #[inline]
    fn upsert_merchants(&self, items: Vec<Merchant>) -> impl Future<Output = Result<()>> + Send {
        future::ready(
            self.with_lock(|inner| upsert_by_key(&mut inner.merchants, items, |m| m.id.clone())),
        )
    }

    #[inline]
    fn upsert_instruments(
        &self,
        items: Vec<Instrument>,
    ) -> impl Future<Output = Result<()>> + Send {
        future::ready(
            self.with_lock(|inner| upsert_by_key(&mut inner.instruments, items, |i| i.id)),
        )
    }

    #[inline]
    fn upsert_companies(&self, items: Vec<Company>) -> impl Future<Output = Result<()>> + Send {
        future::ready(self.with_lock(|inner| upsert_by_key(&mut inner.companies, items, |c| c.id)))
    }

    #[inline]
    fn upsert_countries(&self, items: Vec<Country>) -> impl Future<Output = Result<()>> + Send {
        future::ready(self.with_lock(|inner| upsert_by_key(&mut inner.countries, items, |c| c.id)))
    }

    #[inline]
    fn upsert_users(&self, items: Vec<User>) -> impl Future<Output = Result<()>> + Send {
        future::ready(self.with_lock(|inner| upsert_by_key(&mut inner.users, items, |u| u.id)))
    }

    #[inline]
    fn upsert_reminders(&self, items: Vec<Reminder>) -> impl Future<Output = Result<()>> + Send {
        future::ready(
            self.with_lock(|inner| upsert_by_key(&mut inner.reminders, items, |r| r.id.clone())),
        )
    }

    #[inline]
    fn upsert_reminder_markers(
        &self,
        items: Vec<ReminderMarker>,
    ) -> impl Future<Output = Result<()>> + Send {
        future::ready(
            self.with_lock(|inner| {
                upsert_by_key(&mut inner.reminder_markers, items, |r| r.id.clone());
            }),
        )
    }

    #[inline]
    fn upsert_budgets(&self, items: Vec<Budget>) -> impl Future<Output = Result<()>> + Send {
        future::ready(self.with_lock(|inner| upsert_by_key(&mut inner.budgets, items, budget_key)))
    }

    #[inline]
    fn remove_accounts(&self, ids: &[AccountId]) -> impl Future<Output = Result<()>> + Send {
        future::ready(
            self.with_lock(|inner| remove_by_key(&mut inner.accounts, ids, |a| a.id.clone())),
        )
    }

    #[inline]
    fn remove_transactions(
        &self,
        ids: &[TransactionId],
    ) -> impl Future<Output = Result<()>> + Send {
        future::ready(
            self.with_lock(|inner| remove_by_key(&mut inner.transactions, ids, |t| t.id.clone())),
        )
    }

    #[inline]
    fn remove_tags(&self, ids: &[TagId]) -> impl Future<Output = Result<()>> + Send {
        future::ready(self.with_lock(|inner| remove_by_key(&mut inner.tags, ids, |t| t.id.clone())))
    }

    #[inline]
    fn remove_merchants(&self, ids: &[MerchantId]) -> impl Future<Output = Result<()>> + Send {
        future::ready(
            self.with_lock(|inner| remove_by_key(&mut inner.merchants, ids, |m| m.id.clone())),
        )
    }

    #[inline]
    fn remove_instruments(&self, ids: &[InstrumentId]) -> impl Future<Output = Result<()>> + Send {
        future::ready(self.with_lock(|inner| remove_by_key(&mut inner.instruments, ids, |i| i.id)))
    }

    #[inline]
    fn remove_companies(&self, ids: &[CompanyId]) -> impl Future<Output = Result<()>> + Send {
        future::ready(self.with_lock(|inner| remove_by_key(&mut inner.companies, ids, |c| c.id)))
    }

    #[inline]
    fn remove_countries(&self, ids: &[i32]) -> impl Future<Output = Result<()>> + Send {
        future::ready(self.with_lock(|inner| remove_by_key(&mut inner.countries, ids, |c| c.id)))
    }

    #[inline]
    fn remove_users(&self, ids: &[UserId]) -> impl Future<Output = Result<()>> + Send {
        future::ready(self.with_lock(|inner| remove_by_key(&mut inner.users, ids, |u| u.id)))
    }

    #[inline]
    fn remove_reminders(&self, ids: &[ReminderId]) -> impl Future<Output = Result<()>> + Send {
        future::ready(
            self.with_lock(|inner| remove_by_key(&mut inner.reminders, ids, |r| r.id.clone())),
        )
    }

    #[inline]
    fn remove_reminder_markers(
        &self,
        ids: &[ReminderMarkerId],
    ) -> impl Future<Output = Result<()>> + Send {
        future::ready(
            self.with_lock(|inner| {
                remove_by_key(&mut inner.reminder_markers, ids, |r| r.id.clone());
            }),
        )
    }

    #[inline]
    fn remove_budgets(&self, _ids: &[String]) -> impl Future<Output = Result<()>> + Send {
        future::ready(Ok(()))
    }

    #[inline]
    fn clear(&self) -> impl Future<Output = Result<()>> + Send {
        future::ready(self.with_lock(|inner| *inner = Inner::default()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{AccountType, Interval, ReminderMarkerState};

    // ── Test helpers ───────────────────────────────────────────────────

    fn ts() -> DateTime<Utc> {
        DateTime::from_timestamp(TEST_TIMESTAMP_SECS, 0).unwrap()
    }

    fn test_account(id: &str) -> Account {
        Account {
            id: AccountId::new(id.to_owned()),
            changed: ts(),
            user: UserId::new(1_i64),
            role: None,
            instrument: Some(InstrumentId::new(1_i32)),
            company: None,
            kind: AccountType::Checking,
            title: format!("Account {id}"),
            sync_id: None,
            balance: Some(0.0),
            start_balance: None,
            credit_limit: None,
            in_balance: true,
            savings: None,
            enable_correction: false,
            enable_sms: false,
            archive: false,
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

    fn test_transaction(id: &str) -> Transaction {
        Transaction {
            id: TransactionId::new(id.to_owned()),
            changed: ts(),
            created: ts(),
            user: UserId::new(1_i64),
            deleted: false,
            hold: None,
            income_instrument: InstrumentId::new(1_i32),
            income_account: AccountId::new("a-1".to_owned()),
            income: 0.0,
            outcome_instrument: InstrumentId::new(1_i32),
            outcome_account: AccountId::new("a-1".to_owned()),
            outcome: 100.0,
            tag: None,
            merchant: None,
            payee: None,
            original_payee: None,
            comment: None,
            date: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
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

    fn test_tag(id: &str) -> Tag {
        Tag {
            id: TagId::new(id.to_owned()),
            changed: ts(),
            user: UserId::new(1_i64),
            title: format!("Tag {id}"),
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

    fn test_merchant(id: &str) -> Merchant {
        Merchant {
            id: MerchantId::new(id.to_owned()),
            changed: ts(),
            user: UserId::new(1_i64),
            title: format!("Merchant {id}"),
        }
    }

    fn test_instrument(id: i32) -> Instrument {
        Instrument {
            id: InstrumentId::new(id),
            title: "Currency".to_owned(),
            short_title: "CUR".to_owned(),
            symbol: "C".to_owned(),
            rate: 1.0,
            changed: ts(),
        }
    }

    fn test_company(id: i32) -> Company {
        Company {
            id: CompanyId::new(id),
            changed: ts(),
            title: "Bank".to_owned(),
            full_title: None,
            www: None,
            country: None,
            country_code: None,
            deleted: None,
        }
    }

    fn test_country(id: i32) -> Country {
        Country {
            id,
            title: "Country".to_owned(),
            currency: InstrumentId::new(1_i32),
            domain: None,
        }
    }

    fn test_user(id: i64) -> User {
        User {
            id: UserId::new(id),
            changed: ts(),
            login: Some("test@test.com".to_owned()),
            currency: InstrumentId::new(1_i32),
            parent: None,
            country: None,
            country_code: None,
            email: None,
            is_forecast_enabled: None,
            month_start_day: None,
            paid_till: None,
            plan_balance_mode: None,
            plan_settings: None,
            subscription: None,
            subscription_renewal_date: None,
        }
    }

    fn test_reminder(id: &str) -> Reminder {
        Reminder {
            id: ReminderId::new(id.to_owned()),
            changed: ts(),
            user: UserId::new(1_i64),
            income_instrument: InstrumentId::new(1_i32),
            income_account: AccountId::new("a-1".to_owned()),
            income: 0.0,
            outcome_instrument: InstrumentId::new(1_i32),
            outcome_account: AccountId::new("a-1".to_owned()),
            outcome: 100.0,
            tag: None,
            merchant: None,
            payee: None,
            comment: None,
            interval: Some(Interval::Month),
            step: Some(1_i32),
            points: Some(vec![1_i32]),
            start_date: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            end_date: None,
            notify: false,
        }
    }

    fn test_reminder_marker(id: &str) -> ReminderMarker {
        ReminderMarker {
            id: ReminderMarkerId::new(id.to_owned()),
            changed: ts(),
            user: UserId::new(1_i64),
            income_instrument: InstrumentId::new(1_i32),
            income_account: AccountId::new("a-1".to_owned()),
            income: 0.0,
            outcome_instrument: InstrumentId::new(1_i32),
            outcome_account: AccountId::new("a-1".to_owned()),
            outcome: 100.0,
            tag: None,
            merchant: None,
            payee: None,
            comment: None,
            date: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            reminder: ReminderId::new("r-1".to_owned()),
            state: ReminderMarkerState::Planned,
            notify: false,
            is_forecast: None,
        }
    }

    fn test_budget() -> Budget {
        Budget {
            changed: ts(),
            user: UserId::new(1_i64),
            tag: None,
            date: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            income: 1000.0,
            income_lock: false,
            outcome: 500.0,
            outcome_lock: false,
            is_income_forecast: None,
            is_outcome_forecast: None,
        }
    }

    // ── Blocking tests ─────────────────────────────────────────────────

    #[cfg(feature = "blocking")]
    mod blocking {
        use super::*;
        use crate::storage::BlockingStorage;

        #[test]
        fn server_timestamp_lifecycle() {
            let s = InMemoryStorage::new();
            assert!(s.server_timestamp().unwrap().is_none());
            s.set_server_timestamp(ts()).unwrap();
            assert_eq!(s.server_timestamp().unwrap(), Some(ts()));
        }

        #[test]
        fn upsert_and_read_accounts() {
            let s = InMemoryStorage::new();
            s.upsert_accounts(vec![test_account("a-1"), test_account("a-2")])
                .unwrap();
            assert_eq!(s.accounts().unwrap().len(), 2);
            // Upsert replaces existing by key.
            s.upsert_accounts(vec![test_account("a-1")]).unwrap();
            assert_eq!(s.accounts().unwrap().len(), 2);
        }

        #[test]
        fn remove_accounts() {
            let s = InMemoryStorage::new();
            s.upsert_accounts(vec![test_account("a-1")]).unwrap();
            s.remove_accounts(&[AccountId::new("a-1".to_owned())])
                .unwrap();
            assert!(s.accounts().unwrap().is_empty());
        }

        #[test]
        fn upsert_and_remove_transactions() {
            let s = InMemoryStorage::new();
            s.upsert_transactions(vec![test_transaction("t-1")])
                .unwrap();
            assert_eq!(s.transactions().unwrap().len(), 1);
            s.remove_transactions(&[TransactionId::new("t-1".to_owned())])
                .unwrap();
            assert!(s.transactions().unwrap().is_empty());
        }

        #[test]
        fn upsert_and_remove_tags() {
            let s = InMemoryStorage::new();
            s.upsert_tags(vec![test_tag("t-1")]).unwrap();
            assert_eq!(s.tags().unwrap().len(), 1);
            s.remove_tags(&[TagId::new("t-1".to_owned())]).unwrap();
            assert!(s.tags().unwrap().is_empty());
        }

        #[test]
        fn upsert_and_remove_merchants() {
            let s = InMemoryStorage::new();
            s.upsert_merchants(vec![test_merchant("m-1")]).unwrap();
            assert_eq!(s.merchants().unwrap().len(), 1);
            s.remove_merchants(&[MerchantId::new("m-1".to_owned())])
                .unwrap();
            assert!(s.merchants().unwrap().is_empty());
        }

        #[test]
        fn upsert_and_remove_instruments() {
            let s = InMemoryStorage::new();
            s.upsert_instruments(vec![test_instrument(1)]).unwrap();
            assert_eq!(s.instruments().unwrap().len(), 1);
            s.remove_instruments(&[InstrumentId::new(1)]).unwrap();
            assert!(s.instruments().unwrap().is_empty());
        }

        #[test]
        fn upsert_and_remove_companies() {
            let s = InMemoryStorage::new();
            s.upsert_companies(vec![test_company(1)]).unwrap();
            assert_eq!(s.companies().unwrap().len(), 1);
            s.remove_companies(&[CompanyId::new(1)]).unwrap();
            assert!(s.companies().unwrap().is_empty());
        }

        #[test]
        fn upsert_and_remove_countries() {
            let s = InMemoryStorage::new();
            s.upsert_countries(vec![test_country(1)]).unwrap();
            assert_eq!(s.countries().unwrap().len(), 1);
            s.remove_countries(&[1]).unwrap();
            assert!(s.countries().unwrap().is_empty());
        }

        #[test]
        fn upsert_and_remove_users() {
            let s = InMemoryStorage::new();
            s.upsert_users(vec![test_user(1)]).unwrap();
            assert_eq!(s.users().unwrap().len(), 1);
            s.remove_users(&[UserId::new(1)]).unwrap();
            assert!(s.users().unwrap().is_empty());
        }

        #[test]
        fn upsert_and_remove_reminders() {
            let s = InMemoryStorage::new();
            s.upsert_reminders(vec![test_reminder("r-1")]).unwrap();
            assert_eq!(s.reminders().unwrap().len(), 1);
            s.remove_reminders(&[ReminderId::new("r-1".to_owned())])
                .unwrap();
            assert!(s.reminders().unwrap().is_empty());
        }

        #[test]
        fn upsert_and_remove_reminder_markers() {
            let s = InMemoryStorage::new();
            s.upsert_reminder_markers(vec![test_reminder_marker("rm-1")])
                .unwrap();
            assert_eq!(s.reminder_markers().unwrap().len(), 1);
            s.remove_reminder_markers(&[ReminderMarkerId::new("rm-1".to_owned())])
                .unwrap();
            assert!(s.reminder_markers().unwrap().is_empty());
        }

        #[test]
        fn upsert_budgets_and_remove_is_noop() {
            let s = InMemoryStorage::new();
            s.upsert_budgets(vec![test_budget()]).unwrap();
            assert_eq!(s.budgets().unwrap().len(), 1);
            // remove_budgets is a no-op.
            s.remove_budgets(&["key".to_owned()]).unwrap();
            assert_eq!(s.budgets().unwrap().len(), 1);
        }

        #[test]
        fn clear_resets_everything() {
            let s = InMemoryStorage::new();
            s.set_server_timestamp(ts()).unwrap();
            s.upsert_accounts(vec![test_account("a-1")]).unwrap();
            s.upsert_transactions(vec![test_transaction("t-1")])
                .unwrap();
            s.upsert_companies(vec![test_company(1)]).unwrap();
            s.clear().unwrap();
            assert!(s.server_timestamp().unwrap().is_none());
            assert!(s.accounts().unwrap().is_empty());
            assert!(s.transactions().unwrap().is_empty());
            assert!(s.companies().unwrap().is_empty());
        }
    }

    // ── Async tests ────────────────────────────────────────────────────

    #[cfg(feature = "async")]
    mod async_tests {
        use super::*;
        use crate::storage::Storage;

        #[tokio::test]
        async fn server_timestamp_lifecycle() {
            let s = InMemoryStorage::new();
            assert!(s.server_timestamp().await.unwrap().is_none());
            s.set_server_timestamp(ts()).await.unwrap();
            assert_eq!(s.server_timestamp().await.unwrap(), Some(ts()));
        }

        #[tokio::test]
        async fn upsert_and_read_accounts() {
            let s = InMemoryStorage::new();
            s.upsert_accounts(vec![test_account("a-1"), test_account("a-2")])
                .await
                .unwrap();
            assert_eq!(s.accounts().await.unwrap().len(), 2);
        }

        #[tokio::test]
        async fn remove_accounts() {
            let s = InMemoryStorage::new();
            s.upsert_accounts(vec![test_account("a-1")]).await.unwrap();
            s.remove_accounts(&[AccountId::new("a-1".to_owned())])
                .await
                .unwrap();
            assert!(s.accounts().await.unwrap().is_empty());
        }

        #[tokio::test]
        async fn upsert_and_remove_transactions() {
            let s = InMemoryStorage::new();
            s.upsert_transactions(vec![test_transaction("t-1")])
                .await
                .unwrap();
            assert_eq!(s.transactions().await.unwrap().len(), 1);
            s.remove_transactions(&[TransactionId::new("t-1".to_owned())])
                .await
                .unwrap();
            assert!(s.transactions().await.unwrap().is_empty());
        }

        #[tokio::test]
        async fn upsert_and_remove_tags() {
            let s = InMemoryStorage::new();
            s.upsert_tags(vec![test_tag("t-1")]).await.unwrap();
            assert_eq!(s.tags().await.unwrap().len(), 1);
            s.remove_tags(&[TagId::new("t-1".to_owned())])
                .await
                .unwrap();
            assert!(s.tags().await.unwrap().is_empty());
        }

        #[tokio::test]
        async fn upsert_and_remove_merchants() {
            let s = InMemoryStorage::new();
            s.upsert_merchants(vec![test_merchant("m-1")])
                .await
                .unwrap();
            assert_eq!(s.merchants().await.unwrap().len(), 1);
            s.remove_merchants(&[MerchantId::new("m-1".to_owned())])
                .await
                .unwrap();
            assert!(s.merchants().await.unwrap().is_empty());
        }

        #[tokio::test]
        async fn upsert_and_remove_instruments() {
            let s = InMemoryStorage::new();
            s.upsert_instruments(vec![test_instrument(1)])
                .await
                .unwrap();
            assert_eq!(s.instruments().await.unwrap().len(), 1);
            s.remove_instruments(&[InstrumentId::new(1)]).await.unwrap();
            assert!(s.instruments().await.unwrap().is_empty());
        }

        #[tokio::test]
        async fn upsert_and_remove_companies() {
            let s = InMemoryStorage::new();
            s.upsert_companies(vec![test_company(1)]).await.unwrap();
            assert_eq!(s.companies().await.unwrap().len(), 1);
            s.remove_companies(&[CompanyId::new(1)]).await.unwrap();
            assert!(s.companies().await.unwrap().is_empty());
        }

        #[tokio::test]
        async fn upsert_and_remove_countries() {
            let s = InMemoryStorage::new();
            s.upsert_countries(vec![test_country(1)]).await.unwrap();
            assert_eq!(s.countries().await.unwrap().len(), 1);
            s.remove_countries(&[1]).await.unwrap();
            assert!(s.countries().await.unwrap().is_empty());
        }

        #[tokio::test]
        async fn upsert_and_remove_users() {
            let s = InMemoryStorage::new();
            s.upsert_users(vec![test_user(1)]).await.unwrap();
            assert_eq!(s.users().await.unwrap().len(), 1);
            s.remove_users(&[UserId::new(1)]).await.unwrap();
            assert!(s.users().await.unwrap().is_empty());
        }

        #[tokio::test]
        async fn upsert_and_remove_reminders() {
            let s = InMemoryStorage::new();
            s.upsert_reminders(vec![test_reminder("r-1")])
                .await
                .unwrap();
            assert_eq!(s.reminders().await.unwrap().len(), 1);
            s.remove_reminders(&[ReminderId::new("r-1".to_owned())])
                .await
                .unwrap();
            assert!(s.reminders().await.unwrap().is_empty());
        }

        #[tokio::test]
        async fn upsert_and_remove_reminder_markers() {
            let s = InMemoryStorage::new();
            s.upsert_reminder_markers(vec![test_reminder_marker("rm-1")])
                .await
                .unwrap();
            assert_eq!(s.reminder_markers().await.unwrap().len(), 1);
            s.remove_reminder_markers(&[ReminderMarkerId::new("rm-1".to_owned())])
                .await
                .unwrap();
            assert!(s.reminder_markers().await.unwrap().is_empty());
        }

        #[tokio::test]
        async fn upsert_budgets_and_remove_is_noop() {
            let s = InMemoryStorage::new();
            s.upsert_budgets(vec![test_budget()]).await.unwrap();
            assert_eq!(s.budgets().await.unwrap().len(), 1);
            s.remove_budgets(&["key".to_owned()]).await.unwrap();
            assert_eq!(s.budgets().await.unwrap().len(), 1);
        }

        #[tokio::test]
        async fn clear_resets_everything() {
            let s = InMemoryStorage::new();
            s.set_server_timestamp(ts()).await.unwrap();
            s.upsert_accounts(vec![test_account("a-1")]).await.unwrap();
            s.upsert_companies(vec![test_company(1)]).await.unwrap();
            s.upsert_users(vec![test_user(1)]).await.unwrap();
            s.clear().await.unwrap();
            assert!(s.server_timestamp().await.unwrap().is_none());
            assert!(s.accounts().await.unwrap().is_empty());
            assert!(s.companies().await.unwrap().is_empty());
            assert!(s.users().await.unwrap().is_empty());
        }
    }
}
