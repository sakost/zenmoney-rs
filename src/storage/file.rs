//! JSON-file-based storage backend.
//!
//! Stores each entity type in a separate JSON file under a configurable
//! directory (default: `$XDG_DATA_HOME/zenmoney-rs/`).

use core::hash::Hash;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::error::{Result, ZenMoneyError};
use crate::models::{
    Account, AccountId, Budget, Company, CompanyId, Country, Instrument, InstrumentId, Merchant,
    MerchantId, NaiveDate, Reminder, ReminderId, ReminderMarker, ReminderMarkerId, Tag, TagId,
    Transaction, TransactionId, User, UserId,
};

/// Application name used for the XDG data directory.
const APP_NAME: &str = "zenmoney-rs";

/// File names for each entity type.
const META_FILE: &str = "meta.json";
/// File name for accounts.
const ACCOUNTS_FILE: &str = "accounts.json";
/// File name for transactions.
const TRANSACTIONS_FILE: &str = "transactions.json";
/// File name for tags.
const TAGS_FILE: &str = "tags.json";
/// File name for merchants.
const MERCHANTS_FILE: &str = "merchants.json";
/// File name for instruments.
const INSTRUMENTS_FILE: &str = "instruments.json";
/// File name for companies.
const COMPANIES_FILE: &str = "companies.json";
/// File name for countries.
const COUNTRIES_FILE: &str = "countries.json";
/// File name for users.
const USERS_FILE: &str = "users.json";
/// File name for reminders.
const REMINDERS_FILE: &str = "reminders.json";
/// File name for reminder markers.
const REMINDER_MARKERS_FILE: &str = "reminder_markers.json";
/// File name for budgets.
const BUDGETS_FILE: &str = "budgets.json";
/// Sentinel file used for cross-process file locking.
const LOCK_FILE: &str = "storage.lock";

/// Metadata stored alongside entity files.
#[derive(Debug, Serialize, Deserialize, Default)]
struct Meta {
    /// Server timestamp in seconds since epoch, or absent if never synced.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    server_timestamp: Option<i64>,
}

/// File-backed storage that persists synced data as JSON files.
///
/// Each entity type is stored in a separate `.json` file. A `meta.json`
/// file tracks the last server timestamp for incremental sync.
///
/// # Concurrency
///
/// Thread safety within a single process is provided by an in-process
/// [`Mutex`]. Cross-process safety is achieved via an advisory file lock
/// on `storage.lock` (using [`std::fs::File::lock`] /
/// [`std::fs::File::lock_shared`]).
///
/// Read operations acquire a shared lock (allowing concurrent readers),
/// while write operations acquire an exclusive lock.
///
/// # File layout
///
/// ```text
/// <dir>/
///   storage.lock          (cross-process lock sentinel)
///   meta.json
///   accounts.json
///   transactions.json
///   tags.json
///   merchants.json
///   instruments.json
///   companies.json
///   countries.json
///   users.json
///   reminders.json
///   reminder_markers.json
///   budgets.json
/// ```
#[derive(Debug)]
pub struct FileStorage {
    /// Root directory containing all JSON files.
    dir: PathBuf,
    /// Mutex serializing concurrent in-process access.
    lock: Mutex<()>,
    /// Sentinel file for cross-process advisory locking.
    lock_file: fs::File,
}

impl FileStorage {
    /// Creates a new file storage rooted at the given directory.
    ///
    /// Creates the directory (and parents) if it does not exist. Also
    /// opens (or creates) the `storage.lock` sentinel file used for
    /// cross-process advisory locking.
    ///
    /// # Errors
    ///
    /// Returns an error if the directory cannot be created or the lock
    /// file cannot be opened.
    #[inline]
    pub fn new(dir: PathBuf) -> Result<Self> {
        fs::create_dir_all(&dir).map_err(storage_io_error)?;
        let lock_file = fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(dir.join(LOCK_FILE))
            .map_err(storage_io_error)?;
        Ok(Self {
            dir,
            lock: Mutex::new(()),
            lock_file,
        })
    }

    /// Returns the default XDG-compliant data directory for this application.
    ///
    /// On Linux: `$XDG_DATA_HOME/zenmoney-rs/` (typically
    /// `~/.local/share/zenmoney-rs/`).
    ///
    /// # Errors
    ///
    /// Returns an error if the platform data directory cannot be determined.
    #[inline]
    pub fn default_dir() -> Result<PathBuf> {
        dirs::data_dir()
            .map(|data_path| data_path.join(APP_NAME))
            .ok_or_else(|| {
                ZenMoneyError::Storage("could not determine platform data directory".into())
            })
    }

    // ── Private helpers ─────────────────────────────────────────────

    /// Returns the full path for a given file name.
    fn path(&self, name: &str) -> PathBuf {
        self.dir.join(name)
    }

    /// Acquires an in-process mutex guard and a shared (read) file lock,
    /// executes `op`, then releases the file lock.
    fn with_shared_lock<R, F: FnOnce() -> Result<R>>(&self, op: F) -> Result<R> {
        let _guard: MutexGuard<'_, ()> = self.lock.lock().map_err(|err| lock_poison_error(&err))?;
        self.lock_file.lock_shared().map_err(storage_io_error)?;
        let result = op();
        // Only surface the unlock error when the operation succeeded;
        // otherwise the original error is more useful.
        if let Err(err) = self.lock_file.unlock()
            && result.is_ok()
        {
            return Err(storage_io_error(err));
        }
        result
    }

    /// Acquires an in-process mutex guard and an exclusive (write) file
    /// lock, executes `op`, then releases the file lock.
    fn with_exclusive_lock<R, F: FnOnce() -> Result<R>>(&self, op: F) -> Result<R> {
        let _guard: MutexGuard<'_, ()> = self.lock.lock().map_err(|err| lock_poison_error(&err))?;
        self.lock_file.lock().map_err(storage_io_error)?;
        let result = op();
        if let Err(err) = self.lock_file.unlock()
            && result.is_ok()
        {
            return Err(storage_io_error(err));
        }
        result
    }

    /// Reads and deserializes a JSON file. Returns an empty `Vec` if the
    /// file does not exist.
    fn read_entities<T: serde::de::DeserializeOwned>(&self, name: &str) -> Result<Vec<T>> {
        let path = self.path(name);
        match fs::read_to_string(&path) {
            Ok(contents) => serde_json::from_str(&contents).map_err(ZenMoneyError::from),
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(Vec::new()),
            Err(err) => Err(storage_io_error(err)),
        }
    }

    /// Atomically writes a serialized JSON file (write-to-tmp then rename).
    fn write_entities<T: Serialize>(&self, name: &str, items: &[T]) -> Result<()> {
        let path = self.path(name);
        let tmp_path = self.path(&format!("{name}.tmp"));
        let json = serde_json::to_string_pretty(items).map_err(ZenMoneyError::from)?;
        fs::write(&tmp_path, json).map_err(storage_io_error)?;
        fs::rename(&tmp_path, &path).map_err(storage_io_error)?;
        Ok(())
    }

    /// Reads the metadata file.
    fn read_meta(&self) -> Result<Meta> {
        let path = self.path(META_FILE);
        match fs::read_to_string(&path) {
            Ok(contents) => serde_json::from_str(&contents).map_err(ZenMoneyError::from),
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(Meta::default()),
            Err(err) => Err(storage_io_error(err)),
        }
    }

    /// Atomically writes the metadata file.
    fn write_meta(&self, meta: &Meta) -> Result<()> {
        let path = self.path(META_FILE);
        let tmp_path = self.path(&format!("{META_FILE}.tmp"));
        let json = serde_json::to_string_pretty(meta).map_err(ZenMoneyError::from)?;
        fs::write(&tmp_path, json).map_err(storage_io_error)?;
        fs::rename(&tmp_path, &path).map_err(storage_io_error)?;
        Ok(())
    }

    /// Merges new items into an entity file by key (insert-or-replace).
    fn upsert_file<T, K>(&self, name: &str, new_items: Vec<T>, key_fn: fn(&T) -> K) -> Result<()>
    where
        T: Serialize + serde::de::DeserializeOwned,
        K: Hash + Eq,
    {
        if new_items.is_empty() {
            return Ok(());
        }
        self.with_exclusive_lock(|| {
            let existing: Vec<T> = self.read_entities(name)?;
            let merged = upsert_by_key(existing, new_items, key_fn);
            self.write_entities(name, &merged)
        })
    }

    /// Removes items from an entity file by key.
    fn remove_file<T, K>(&self, name: &str, ids: &[K], key_fn: fn(&T) -> K) -> Result<()>
    where
        T: Serialize + serde::de::DeserializeOwned,
        K: Hash + Eq,
    {
        if ids.is_empty() {
            return Ok(());
        }
        self.with_exclusive_lock(|| {
            let existing: Vec<T> = self.read_entities(name)?;
            let filtered = remove_by_key(existing, ids, key_fn);
            self.write_entities(name, &filtered)
        })
    }

    /// Reads `server_timestamp` from meta (with lock).
    fn read_server_timestamp(&self) -> Result<Option<DateTime<Utc>>> {
        self.with_shared_lock(|| {
            let meta = self.read_meta()?;
            Ok(meta
                .server_timestamp
                .and_then(|ts| DateTime::from_timestamp(ts, 0_u32)))
        })
    }

    /// Writes `server_timestamp` to meta (with lock).
    fn write_server_timestamp(&self, timestamp: DateTime<Utc>) -> Result<()> {
        self.with_exclusive_lock(|| {
            let mut meta = self.read_meta()?;
            meta.server_timestamp = Some(timestamp.timestamp());
            self.write_meta(&meta)
        })
    }

    /// Deletes all entity files and metadata.
    ///
    /// The `storage.lock` sentinel is intentionally preserved — it is
    /// infrastructure, not data.
    fn clear_all(&self) -> Result<()> {
        self.with_exclusive_lock(|| {
            let files = [
                META_FILE,
                ACCOUNTS_FILE,
                TRANSACTIONS_FILE,
                TAGS_FILE,
                MERCHANTS_FILE,
                INSTRUMENTS_FILE,
                COMPANIES_FILE,
                COUNTRIES_FILE,
                USERS_FILE,
                REMINDERS_FILE,
                REMINDER_MARKERS_FILE,
                BUDGETS_FILE,
            ];
            for name in files {
                let path = self.path(name);
                match fs::remove_file(&path) {
                    Ok(()) => {}
                    Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
                    Err(err) => return Err(storage_io_error(err)),
                }
            }
            Ok(())
        })
    }
}

// ── Free-standing helpers ───────────────────────────────────────────────

/// Wraps an I/O error into a [`ZenMoneyError::Storage`].
fn storage_io_error(err: std::io::Error) -> ZenMoneyError {
    ZenMoneyError::Storage(Box::new(err))
}

/// Wraps a mutex poison error into a [`ZenMoneyError::Storage`].
fn lock_poison_error<T>(err: &std::sync::PoisonError<T>) -> ZenMoneyError {
    ZenMoneyError::Storage(err.to_string().into())
}

/// Merges `new_items` into `existing` by key, replacing duplicates.
fn upsert_by_key<T, K>(existing: Vec<T>, new_items: Vec<T>, key_fn: fn(&T) -> K) -> Vec<T>
where
    K: Hash + Eq,
{
    let mut map: HashMap<K, T> = HashMap::with_capacity(existing.len() + new_items.len());
    for item in existing {
        let key = key_fn(&item);
        let _old = map.insert(key, item);
    }
    for item in new_items {
        let key = key_fn(&item);
        let _old = map.insert(key, item);
    }
    map.into_values().collect()
}

/// Removes items whose key is in `ids`.
fn remove_by_key<T, K>(existing: Vec<T>, ids: &[K], key_fn: fn(&T) -> K) -> Vec<T>
where
    K: Hash + Eq,
{
    let id_set: std::collections::HashSet<&K> = ids.iter().collect();
    existing
        .into_iter()
        .filter(|item| !id_set.contains(&key_fn(item)))
        .collect()
}

/// Extracts the budget composite key.
fn budget_key(budget: &Budget) -> (UserId, Option<TagId>, NaiveDate) {
    (budget.user, budget.tag.clone(), budget.date)
}

// ── Key extraction functions ────────────────────────────────────────────

/// Extracts the account ID.
fn account_key(item: &Account) -> AccountId {
    item.id.clone()
}

/// Extracts the transaction ID.
fn transaction_key(item: &Transaction) -> TransactionId {
    item.id.clone()
}

/// Extracts the tag ID.
fn tag_key(item: &Tag) -> TagId {
    item.id.clone()
}

/// Extracts the merchant ID.
fn merchant_key(item: &Merchant) -> MerchantId {
    item.id.clone()
}

/// Extracts the instrument ID.
const fn instrument_key(item: &Instrument) -> InstrumentId {
    item.id
}

/// Extracts the company ID.
const fn company_key(item: &Company) -> CompanyId {
    item.id
}

/// Extracts the country ID.
const fn country_key(item: &Country) -> i32 {
    item.id
}

/// Extracts the user ID.
const fn user_key(item: &User) -> UserId {
    item.id
}

/// Extracts the reminder ID.
fn reminder_key(item: &Reminder) -> ReminderId {
    item.id.clone()
}

/// Extracts the reminder marker ID.
fn reminder_marker_key(item: &ReminderMarker) -> ReminderMarkerId {
    item.id.clone()
}

// ── BlockingStorage implementation ──────────────────────────────────────

#[cfg(feature = "blocking")]
impl super::BlockingStorage for FileStorage {
    #[inline]
    fn server_timestamp(&self) -> Result<Option<DateTime<Utc>>> {
        self.read_server_timestamp()
    }

    #[inline]
    fn set_server_timestamp(&self, timestamp: DateTime<Utc>) -> Result<()> {
        self.write_server_timestamp(timestamp)
    }

    #[inline]
    fn accounts(&self) -> Result<Vec<Account>> {
        self.with_shared_lock(|| self.read_entities(ACCOUNTS_FILE))
    }

    #[inline]
    fn transactions(&self) -> Result<Vec<Transaction>> {
        self.with_shared_lock(|| self.read_entities(TRANSACTIONS_FILE))
    }

    #[inline]
    fn tags(&self) -> Result<Vec<Tag>> {
        self.with_shared_lock(|| self.read_entities(TAGS_FILE))
    }

    #[inline]
    fn merchants(&self) -> Result<Vec<Merchant>> {
        self.with_shared_lock(|| self.read_entities(MERCHANTS_FILE))
    }

    #[inline]
    fn instruments(&self) -> Result<Vec<Instrument>> {
        self.with_shared_lock(|| self.read_entities(INSTRUMENTS_FILE))
    }

    #[inline]
    fn companies(&self) -> Result<Vec<Company>> {
        self.with_shared_lock(|| self.read_entities(COMPANIES_FILE))
    }

    #[inline]
    fn countries(&self) -> Result<Vec<Country>> {
        self.with_shared_lock(|| self.read_entities(COUNTRIES_FILE))
    }

    #[inline]
    fn users(&self) -> Result<Vec<User>> {
        self.with_shared_lock(|| self.read_entities(USERS_FILE))
    }

    #[inline]
    fn reminders(&self) -> Result<Vec<Reminder>> {
        self.with_shared_lock(|| self.read_entities(REMINDERS_FILE))
    }

    #[inline]
    fn reminder_markers(&self) -> Result<Vec<ReminderMarker>> {
        self.with_shared_lock(|| self.read_entities(REMINDER_MARKERS_FILE))
    }

    #[inline]
    fn budgets(&self) -> Result<Vec<Budget>> {
        self.with_shared_lock(|| self.read_entities(BUDGETS_FILE))
    }

    #[inline]
    fn upsert_accounts(&self, items: Vec<Account>) -> Result<()> {
        self.upsert_file(ACCOUNTS_FILE, items, account_key)
    }

    #[inline]
    fn upsert_transactions(&self, items: Vec<Transaction>) -> Result<()> {
        self.upsert_file(TRANSACTIONS_FILE, items, transaction_key)
    }

    #[inline]
    fn upsert_tags(&self, items: Vec<Tag>) -> Result<()> {
        self.upsert_file(TAGS_FILE, items, tag_key)
    }

    #[inline]
    fn upsert_merchants(&self, items: Vec<Merchant>) -> Result<()> {
        self.upsert_file(MERCHANTS_FILE, items, merchant_key)
    }

    #[inline]
    fn upsert_instruments(&self, items: Vec<Instrument>) -> Result<()> {
        self.upsert_file(INSTRUMENTS_FILE, items, instrument_key)
    }

    #[inline]
    fn upsert_companies(&self, items: Vec<Company>) -> Result<()> {
        self.upsert_file(COMPANIES_FILE, items, company_key)
    }

    #[inline]
    fn upsert_countries(&self, items: Vec<Country>) -> Result<()> {
        self.upsert_file(COUNTRIES_FILE, items, country_key)
    }

    #[inline]
    fn upsert_users(&self, items: Vec<User>) -> Result<()> {
        self.upsert_file(USERS_FILE, items, user_key)
    }

    #[inline]
    fn upsert_reminders(&self, items: Vec<Reminder>) -> Result<()> {
        self.upsert_file(REMINDERS_FILE, items, reminder_key)
    }

    #[inline]
    fn upsert_reminder_markers(&self, items: Vec<ReminderMarker>) -> Result<()> {
        self.upsert_file(REMINDER_MARKERS_FILE, items, reminder_marker_key)
    }

    #[inline]
    fn upsert_budgets(&self, items: Vec<Budget>) -> Result<()> {
        self.upsert_file(BUDGETS_FILE, items, budget_key)
    }

    #[inline]
    fn remove_accounts(&self, ids: &[AccountId]) -> Result<()> {
        self.remove_file(ACCOUNTS_FILE, ids, account_key)
    }

    #[inline]
    fn remove_transactions(&self, ids: &[TransactionId]) -> Result<()> {
        self.remove_file(TRANSACTIONS_FILE, ids, transaction_key)
    }

    #[inline]
    fn remove_tags(&self, ids: &[TagId]) -> Result<()> {
        self.remove_file(TAGS_FILE, ids, tag_key)
    }

    #[inline]
    fn remove_merchants(&self, ids: &[MerchantId]) -> Result<()> {
        self.remove_file(MERCHANTS_FILE, ids, merchant_key)
    }

    #[inline]
    fn remove_instruments(&self, ids: &[InstrumentId]) -> Result<()> {
        self.remove_file(INSTRUMENTS_FILE, ids, instrument_key)
    }

    #[inline]
    fn remove_companies(&self, ids: &[CompanyId]) -> Result<()> {
        self.remove_file(COMPANIES_FILE, ids, company_key)
    }

    #[inline]
    fn remove_countries(&self, ids: &[i32]) -> Result<()> {
        self.remove_file(COUNTRIES_FILE, ids, country_key)
    }

    #[inline]
    fn remove_users(&self, ids: &[UserId]) -> Result<()> {
        self.remove_file(USERS_FILE, ids, user_key)
    }

    #[inline]
    fn remove_reminders(&self, ids: &[ReminderId]) -> Result<()> {
        self.remove_file(REMINDERS_FILE, ids, reminder_key)
    }

    #[inline]
    fn remove_reminder_markers(&self, ids: &[ReminderMarkerId]) -> Result<()> {
        self.remove_file(REMINDER_MARKERS_FILE, ids, reminder_marker_key)
    }

    #[inline]
    fn remove_budgets(&self, _ids: &[String]) -> Result<()> {
        // Budget deletions are not expected from the API; composite key
        // matching would require parsing the raw ID string. Left as no-op.
        Ok(())
    }

    #[inline]
    fn clear(&self) -> Result<()> {
        self.clear_all()
    }
}

// ── Storage (async) implementation ──────────────────────────────────────

#[cfg(feature = "async")]
impl super::Storage for FileStorage {
    #[inline]
    fn server_timestamp(&self) -> impl Future<Output = Result<Option<DateTime<Utc>>>> + Send {
        core::future::ready(self.read_server_timestamp())
    }

    #[inline]
    fn set_server_timestamp(
        &self,
        timestamp: DateTime<Utc>,
    ) -> impl Future<Output = Result<()>> + Send {
        core::future::ready(self.write_server_timestamp(timestamp))
    }

    #[inline]
    fn accounts(&self) -> impl Future<Output = Result<Vec<Account>>> + Send {
        core::future::ready(self.with_shared_lock(|| self.read_entities(ACCOUNTS_FILE)))
    }

    #[inline]
    fn transactions(&self) -> impl Future<Output = Result<Vec<Transaction>>> + Send {
        core::future::ready(self.with_shared_lock(|| self.read_entities(TRANSACTIONS_FILE)))
    }

    #[inline]
    fn tags(&self) -> impl Future<Output = Result<Vec<Tag>>> + Send {
        core::future::ready(self.with_shared_lock(|| self.read_entities(TAGS_FILE)))
    }

    #[inline]
    fn merchants(&self) -> impl Future<Output = Result<Vec<Merchant>>> + Send {
        core::future::ready(self.with_shared_lock(|| self.read_entities(MERCHANTS_FILE)))
    }

    #[inline]
    fn instruments(&self) -> impl Future<Output = Result<Vec<Instrument>>> + Send {
        core::future::ready(self.with_shared_lock(|| self.read_entities(INSTRUMENTS_FILE)))
    }

    #[inline]
    fn companies(&self) -> impl Future<Output = Result<Vec<Company>>> + Send {
        core::future::ready(self.with_shared_lock(|| self.read_entities(COMPANIES_FILE)))
    }

    #[inline]
    fn countries(&self) -> impl Future<Output = Result<Vec<Country>>> + Send {
        core::future::ready(self.with_shared_lock(|| self.read_entities(COUNTRIES_FILE)))
    }

    #[inline]
    fn users(&self) -> impl Future<Output = Result<Vec<User>>> + Send {
        core::future::ready(self.with_shared_lock(|| self.read_entities(USERS_FILE)))
    }

    #[inline]
    fn reminders(&self) -> impl Future<Output = Result<Vec<Reminder>>> + Send {
        core::future::ready(self.with_shared_lock(|| self.read_entities(REMINDERS_FILE)))
    }

    #[inline]
    fn reminder_markers(&self) -> impl Future<Output = Result<Vec<ReminderMarker>>> + Send {
        core::future::ready(self.with_shared_lock(|| self.read_entities(REMINDER_MARKERS_FILE)))
    }

    #[inline]
    fn budgets(&self) -> impl Future<Output = Result<Vec<Budget>>> + Send {
        core::future::ready(self.with_shared_lock(|| self.read_entities(BUDGETS_FILE)))
    }

    #[inline]
    fn upsert_accounts(&self, items: Vec<Account>) -> impl Future<Output = Result<()>> + Send {
        core::future::ready(self.upsert_file(ACCOUNTS_FILE, items, account_key))
    }

    #[inline]
    fn upsert_transactions(
        &self,
        items: Vec<Transaction>,
    ) -> impl Future<Output = Result<()>> + Send {
        core::future::ready(self.upsert_file(TRANSACTIONS_FILE, items, transaction_key))
    }

    #[inline]
    fn upsert_tags(&self, items: Vec<Tag>) -> impl Future<Output = Result<()>> + Send {
        core::future::ready(self.upsert_file(TAGS_FILE, items, tag_key))
    }

    #[inline]
    fn upsert_merchants(&self, items: Vec<Merchant>) -> impl Future<Output = Result<()>> + Send {
        core::future::ready(self.upsert_file(MERCHANTS_FILE, items, merchant_key))
    }

    #[inline]
    fn upsert_instruments(
        &self,
        items: Vec<Instrument>,
    ) -> impl Future<Output = Result<()>> + Send {
        core::future::ready(self.upsert_file(INSTRUMENTS_FILE, items, instrument_key))
    }

    #[inline]
    fn upsert_companies(&self, items: Vec<Company>) -> impl Future<Output = Result<()>> + Send {
        core::future::ready(self.upsert_file(COMPANIES_FILE, items, company_key))
    }

    #[inline]
    fn upsert_countries(&self, items: Vec<Country>) -> impl Future<Output = Result<()>> + Send {
        core::future::ready(self.upsert_file(COUNTRIES_FILE, items, country_key))
    }

    #[inline]
    fn upsert_users(&self, items: Vec<User>) -> impl Future<Output = Result<()>> + Send {
        core::future::ready(self.upsert_file(USERS_FILE, items, user_key))
    }

    #[inline]
    fn upsert_reminders(&self, items: Vec<Reminder>) -> impl Future<Output = Result<()>> + Send {
        core::future::ready(self.upsert_file(REMINDERS_FILE, items, reminder_key))
    }

    #[inline]
    fn upsert_reminder_markers(
        &self,
        items: Vec<ReminderMarker>,
    ) -> impl Future<Output = Result<()>> + Send {
        core::future::ready(self.upsert_file(REMINDER_MARKERS_FILE, items, reminder_marker_key))
    }

    #[inline]
    fn upsert_budgets(&self, items: Vec<Budget>) -> impl Future<Output = Result<()>> + Send {
        core::future::ready(self.upsert_file(BUDGETS_FILE, items, budget_key))
    }

    #[inline]
    fn remove_accounts(&self, ids: &[AccountId]) -> impl Future<Output = Result<()>> + Send {
        core::future::ready(self.remove_file(ACCOUNTS_FILE, ids, account_key))
    }

    #[inline]
    fn remove_transactions(
        &self,
        ids: &[TransactionId],
    ) -> impl Future<Output = Result<()>> + Send {
        core::future::ready(self.remove_file(TRANSACTIONS_FILE, ids, transaction_key))
    }

    #[inline]
    fn remove_tags(&self, ids: &[TagId]) -> impl Future<Output = Result<()>> + Send {
        core::future::ready(self.remove_file(TAGS_FILE, ids, tag_key))
    }

    #[inline]
    fn remove_merchants(&self, ids: &[MerchantId]) -> impl Future<Output = Result<()>> + Send {
        core::future::ready(self.remove_file(MERCHANTS_FILE, ids, merchant_key))
    }

    #[inline]
    fn remove_instruments(&self, ids: &[InstrumentId]) -> impl Future<Output = Result<()>> + Send {
        core::future::ready(self.remove_file(INSTRUMENTS_FILE, ids, instrument_key))
    }

    #[inline]
    fn remove_companies(&self, ids: &[CompanyId]) -> impl Future<Output = Result<()>> + Send {
        core::future::ready(self.remove_file(COMPANIES_FILE, ids, company_key))
    }

    #[inline]
    fn remove_countries(&self, ids: &[i32]) -> impl Future<Output = Result<()>> + Send {
        core::future::ready(self.remove_file(COUNTRIES_FILE, ids, country_key))
    }

    #[inline]
    fn remove_users(&self, ids: &[UserId]) -> impl Future<Output = Result<()>> + Send {
        core::future::ready(self.remove_file(USERS_FILE, ids, user_key))
    }

    #[inline]
    fn remove_reminders(&self, ids: &[ReminderId]) -> impl Future<Output = Result<()>> + Send {
        core::future::ready(self.remove_file(REMINDERS_FILE, ids, reminder_key))
    }

    #[inline]
    fn remove_reminder_markers(
        &self,
        ids: &[ReminderMarkerId],
    ) -> impl Future<Output = Result<()>> + Send {
        core::future::ready(self.remove_file(REMINDER_MARKERS_FILE, ids, reminder_marker_key))
    }

    #[inline]
    fn remove_budgets(&self, _ids: &[String]) -> impl Future<Output = Result<()>> + Send {
        core::future::ready(Ok(()))
    }

    #[inline]
    fn clear(&self) -> impl Future<Output = Result<()>> + Send {
        core::future::ready(self.clear_all())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{AccountType, InstrumentId, UserId};

    /// Helper to create a [`FileStorage`] in a temporary directory.
    fn temp_storage() -> (FileStorage, tempfile::TempDir) {
        let dir = tempfile::tempdir().unwrap();
        let storage = FileStorage::new(dir.path().to_path_buf()).unwrap();
        (storage, dir)
    }

    /// Creates a minimal test account.
    fn test_account(id: &str, title: &str) -> Account {
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

    #[cfg(feature = "blocking")]
    mod blocking {
        use super::*;
        use crate::storage::BlockingStorage;

        #[test]
        fn server_timestamp_initially_none() {
            let (storage, _dir) = temp_storage();
            assert!(storage.server_timestamp().unwrap().is_none());
        }

        #[test]
        fn set_and_get_server_timestamp() {
            let (storage, _dir) = temp_storage();
            let ts = DateTime::from_timestamp(1_700_000_100, 0).unwrap();
            storage.set_server_timestamp(ts).unwrap();
            assert_eq!(storage.server_timestamp().unwrap(), Some(ts));
        }

        #[test]
        fn empty_storage_returns_empty_vecs() {
            let (storage, _dir) = temp_storage();
            assert!(storage.accounts().unwrap().is_empty());
            assert!(storage.transactions().unwrap().is_empty());
            assert!(storage.tags().unwrap().is_empty());
            assert!(storage.instruments().unwrap().is_empty());
        }

        #[test]
        fn upsert_and_read_accounts() {
            let (storage, _dir) = temp_storage();
            let acc1 = test_account("a-1", "Checking");
            let acc2 = test_account("a-2", "Savings");
            storage.upsert_accounts(vec![acc1, acc2]).unwrap();

            let accounts = storage.accounts().unwrap();
            assert_eq!(accounts.len(), 2);
        }

        #[test]
        fn upsert_replaces_existing() {
            let (storage, _dir) = temp_storage();
            let acc = test_account("a-1", "Old Title");
            storage.upsert_accounts(vec![acc]).unwrap();

            let updated = test_account("a-1", "New Title");
            storage.upsert_accounts(vec![updated]).unwrap();

            let accounts = storage.accounts().unwrap();
            assert_eq!(accounts.len(), 1);
            assert_eq!(accounts[0].title, "New Title");
        }

        #[test]
        fn remove_accounts() {
            let (storage, _dir) = temp_storage();
            let acc1 = test_account("a-1", "First");
            let acc2 = test_account("a-2", "Second");
            storage.upsert_accounts(vec![acc1, acc2]).unwrap();

            storage
                .remove_accounts(&[AccountId::new("a-1".to_owned())])
                .unwrap();

            let accounts = storage.accounts().unwrap();
            assert_eq!(accounts.len(), 1);
            assert_eq!(accounts[0].title, "Second");
        }

        #[test]
        fn clear_removes_everything() {
            let (storage, _dir) = temp_storage();
            let acc = test_account("a-1", "Test");
            storage.upsert_accounts(vec![acc]).unwrap();
            let ts = DateTime::from_timestamp(100, 0).unwrap();
            storage.set_server_timestamp(ts).unwrap();

            storage.clear().unwrap();

            assert!(storage.accounts().unwrap().is_empty());
            assert!(storage.server_timestamp().unwrap().is_none());
        }

        #[test]
        fn default_dir_returns_path() {
            // Just verify it doesn't error on supported platforms.
            let dir = FileStorage::default_dir();
            assert!(dir.is_ok());
        }

        #[test]
        fn upsert_empty_vec_is_noop() {
            let (storage, _dir) = temp_storage();
            storage.upsert_accounts(Vec::new()).unwrap();
            // Should not create any file.
            assert!(!storage.path(ACCOUNTS_FILE).exists());
        }

        #[test]
        fn remove_from_empty_is_ok() {
            let (storage, _dir) = temp_storage();
            storage
                .remove_accounts(&[AccountId::new("nonexistent".to_owned())])
                .unwrap();
        }
    }

    #[test]
    fn lockfile_created_on_construction() {
        let (storage, _dir) = temp_storage();
        assert!(storage.path(LOCK_FILE).exists());
    }

    #[test]
    fn clear_preserves_lockfile() {
        let (storage, _dir) = temp_storage();
        storage.clear_all().unwrap();
        assert!(storage.path(LOCK_FILE).exists());
    }

    #[cfg(feature = "blocking")]
    #[test]
    fn concurrent_upserts_are_safe() {
        use std::sync::Arc;
        use std::thread;

        let (storage, _dir) = temp_storage();
        let storage = Arc::new(storage);
        let num_threads: usize = 8;
        let items_per_thread: usize = 50;

        let handles: Vec<_> = (0..num_threads)
            .map(|thread_idx| {
                let storage = Arc::clone(&storage);
                thread::spawn(move || {
                    use crate::storage::BlockingStorage;
                    for item_idx in 0..items_per_thread {
                        let id = format!("t{thread_idx}-{item_idx}");
                        let acc = test_account(&id, &format!("Account {id}"));
                        storage.upsert_accounts(vec![acc]).unwrap();
                    }
                })
            })
            .collect();

        for handle in handles {
            handle.join().unwrap();
        }

        use crate::storage::BlockingStorage;
        let accounts = storage.accounts().unwrap();
        assert_eq!(accounts.len(), num_threads * items_per_thread);
    }

    #[cfg(feature = "async")]
    mod async_tests {
        use super::*;
        use crate::storage::Storage;

        #[tokio::test]
        async fn server_timestamp_initially_none() {
            let (storage, _dir) = temp_storage();
            assert!(storage.server_timestamp().await.unwrap().is_none());
        }

        #[tokio::test]
        async fn upsert_and_read_accounts() {
            let (storage, _dir) = temp_storage();
            let acc = test_account("a-1", "Test");
            storage.upsert_accounts(vec![acc]).await.unwrap();

            let accounts = storage.accounts().await.unwrap();
            assert_eq!(accounts.len(), 1);
            assert_eq!(accounts[0].title, "Test");
        }
    }
}
