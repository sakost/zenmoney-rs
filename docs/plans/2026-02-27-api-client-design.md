# ZenMoney API Client — Design Document

**Date:** 2026-02-27
**Status:** Approved

## Overview

Rust client library for the ZenMoney API (`https://api.zenmoney.ru`). Provides three layers: strongly-typed data models, a low-level HTTP client, and an ergonomic high-level API with CRUD operations, query helpers, and sync state management.

## API Surface

ZenMoney exposes two main endpoints:

- `POST /v8/diff/` — symmetric sync: send local changes, receive server changes. Uses `serverTimestamp` for incremental sync.
- `POST /v8/suggest/` — category/payee suggestions from partial transaction data.

Authentication is OAuth 2.0 with 24-hour access tokens and refresh tokens.

## Architecture

### Three Layers

1. **Models** (`models/`) — Serde-serializable structs for all 10 entity types plus diff/suggest request/response types. Newtype IDs, enums for constrained values, `Option<T>` for nullable fields.

2. **Client** (`client/`) — Low-level HTTP client. Async (reqwest) and blocking (reqwest::blocking) variants behind feature flags. Accepts a plain access token by default. With the `oauth` feature, gains authorization URL generation, code-to-token exchange, and automatic token refresh via `TokenStorage` trait.

3. **High-level API** (`api/`) — `SyncManager` tracks sync state and provides incremental/full sync. Per-entity CRUD methods (`create`, `update`, `delete`) that build `DiffRequest` internally. Query/filter helpers (`by_type`, `between`, `by_tag`, `active`, `total_balance`) over synced data.

### Feature Flags

```toml
[features]
default = ["async", "storage-file"]
async = ["dep:reqwest"]             # Async client
blocking = ["dep:reqwest"]          # Sync client (reqwest::blocking)
oauth = ["dep:url"]                 # Full OAuth 2.0 flow
storage-file = []                   # JSON file token/state storage
storage-sqlx = ["dep:sqlx"]         # SQLx-based storage (SQLite/Postgres/MySQL)
full = ["async", "blocking", "oauth", "storage-file", "storage-sqlx"]
```

Without `oauth`, the user supplies a pre-obtained access token. Storage features provide trait implementations; users can always implement the traits themselves.

## Data Models

### Newtype IDs

```
UserId(i64), InstrumentId(i32), CompanyId(i32),
AccountId(String), TagId(String), MerchantId(String),
ReminderId(String), ReminderMarkerId(String), TransactionId(String)
```

### Enums

- `AccountType` — Cash, CreditCard, Checking, Loan, Deposit, EMoney, Debt
- `Interval` — Day, Week, Month, Year
- `ReminderMarkerState` — Planned, Processed, Deleted

### Entities

All use `#[serde(rename_all = "camelCase")]`, `Option<T>` for nullable fields, `NaiveDate` for date strings, `i64` for Unix timestamps, `f64` for monetary amounts.

Entities: Instrument, Company, User, Account, Tag, Merchant, Transaction, Reminder, ReminderMarker, Budget.

Diff types: DiffRequest, DiffResponse, Deletion.
Suggest types: SuggestRequest, SuggestResponse.

## Error Handling

Single `Error` enum via `thiserror`:

- `Http(reqwest::Error)` — network/HTTP failures
- `Api { status, message }` — non-2xx API responses
- `Serialization(serde_json::Error)` — JSON errors
- `TokenStorage(Box<dyn Error + Send + Sync>)` — storage backend errors
- `TokenExpired` — token needs refresh but no refresh mechanism available
- `OAuth(String)` — OAuth flow errors (behind `oauth` feature)

## Client

Builder pattern:

```rust
ZenMoneyClient::builder()
    .token("access_token")
    .base_url("https://custom/")  // optional
    .build()
```

Low-level methods:
- `diff(&self, request: &DiffRequest) -> Result<DiffResponse>`
- `suggest(&self, request: &SuggestRequest) -> Result<SuggestResponse>`

With `oauth` feature, builder gains `.oauth(key, secret, storage)` for automatic token management.

## Storage Traits

```rust
// Async (always available)
trait TokenStorage: Send + Sync {
    async fn load(&self) -> Result<Option<TokenPair>>;
    async fn save(&self, token: &TokenPair) -> Result<()>;
}

trait SyncStateStorage: Send + Sync {
    async fn load_timestamp(&self) -> Result<i64>;
    async fn save_timestamp(&self, timestamp: i64) -> Result<()>;
}

// Blocking (behind `blocking` feature)
trait TokenStorageBlocking: Send + Sync { ... }
trait SyncStateStorageBlocking: Send + Sync { ... }
```

Implementations:
- `storage-file`: `FileTokenStorage`, `FileSyncStateStorage` (JSON files)
- `storage-sqlx`: `SqlxTokenStorage`, `SqlxSyncStateStorage` (with migrations)

## High-Level API

### SyncManager

```rust
struct SyncManager<S: SyncStateStorage> {
    client: ZenMoneyClient,
    state: S,
}
```

- `sync()` — incremental sync using stored timestamp
- `full_sync()` — complete sync (timestamp=0)
- `push(changes)` — send local mutations

### CRUD

```rust
client.accounts().create(&account).await?;
client.transactions().update(&tx).await?;
client.tags().delete(&tag_id).await?;
```

### Query Helpers

Returned list wrappers provide zero-cost local filters:

- `by_type(AccountType)`, `active()`, `archived()`
- `between(date, date)`, `by_tag(&TagId)`, `by_merchant(&MerchantId)`
- `total_balance()`

## Module Structure

```
src/
├── lib.rs
├── error.rs
├── models/
│   ├── mod.rs, ids.rs
│   ├── instrument.rs, company.rs, user.rs, account.rs
│   ├── tag.rs, merchant.rs, transaction.rs
│   ├── reminder.rs, reminder_marker.rs, budget.rs
│   └── diff.rs, suggest.rs
├── client/
│   ├── mod.rs, async_client.rs, blocking_client.rs
├── oauth/            (behind `oauth`)
│   ├── mod.rs, token.rs, flow.rs
├── storage/
│   ├── mod.rs, file.rs, sqlx.rs
└── api/
    ├── mod.rs, crud.rs, query.rs
```

## Dependencies

- `serde`, `serde_json` — serialization
- `thiserror` — error derive
- `chrono` (no default features, with serde) — date handling
- `secrecy` (with serde) — token wrapping
- `reqwest` (optional) — HTTP client
- `url` (optional) — OAuth URL building
- `sqlx` (optional) — database storage

All added via `cargo add` at implementation time to pin latest versions.

## Implementation Order

1. Models + error types
2. Async client (diff/suggest + plain token)
3. Blocking client
4. Storage traits + file backend
5. OAuth flow
6. High-level API (SyncManager, CRUD, query)
7. SQLx storage
8. Integration tests (mock server)

## Testing — TDD Approach

Tests are written **before** implementation at every step:

1. **Models**: Write deserialization tests from real API JSON samples first, then implement structs until tests pass. Write serialization roundtrip tests. Write ID newtype conversion tests.
2. **Client**: Write tests against a mock server (wiremock/httpmock) defining expected request/response pairs for diff and suggest. Then implement the client.
3. **Blocking client**: Same mock-based tests, synchronous variant.
4. **Storage**: Write tests for save/load roundtrips with temp files and in-memory SQLite. Then implement backends.
5. **OAuth**: Write tests for URL generation, token exchange, refresh flow against mock endpoints. Then implement.
6. **High-level API**: Write tests for CRUD operations, sync flows, and query filters with fixture data. Then implement SyncManager and helpers.
7. **Integration tests**: Full end-to-end flows against mock server in `tests/` directory.
8. **Doc tests**: Compilable usage examples on all public API items.
