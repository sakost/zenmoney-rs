# zenmoney-rs

[![Crates.io](https://img.shields.io/crates/v/zenmoney-rs)](https://crates.io/crates/zenmoney-rs)
[![docs.rs](https://img.shields.io/docsrs/zenmoney-rs)](https://docs.rs/zenmoney-rs)
[![CI](https://github.com/sakost/zenmoney-rs/actions/workflows/ci.yml/badge.svg)](https://github.com/sakost/zenmoney-rs/actions/workflows/ci.yml)
[![License](https://img.shields.io/crates/l/zenmoney-rs)](LICENSE-MIT)

Rust client library for the [ZenMoney](https://zenmoney.ru/) personal finance API.

## Features

- Async and blocking HTTP clients (feature-gated)
- Incremental and full sync via the diff endpoint
- CRUD operations: push (create/update) and delete for all entity types
- Composable `TransactionFilter` with builder pattern (date range, account, tag, payee, merchant, amount)
- Category suggestion endpoint
- Pluggable storage backends (`FileStorage` included, `InMemoryStorage` for testing, custom backends via `Storage`/`BlockingStorage` traits)
- Strongly-typed models with newtype IDs (`AccountId`, `TagId`, `TransactionId`, etc.)
- Optional CLI binary for browsing synced data

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
zenmoney-rs = "0.3"
```

### Feature flags

| Flag | Default | Description |
|------|---------|-------------|
| `async` | Yes | Async HTTP client (requires tokio runtime) |
| `blocking` | No | Blocking HTTP client |
| `storage-file` | Yes | JSON file-based storage backend |
| `oauth` | No | OAuth authorization URL builder |
| `cli` | Yes | CLI binary (`zenmoney`) |
| `full` | No | Enables all features |

To use only the blocking client without the CLI:

```toml
[dependencies]
zenmoney-rs = { version = "0.3", default-features = false, features = ["blocking", "storage-file"] }
```

## Usage

### Blocking client

```rust,no_run
use zenmoney_rs::storage::FileStorage;
use zenmoney_rs::zen_money::{TransactionFilter, ZenMoneyBlocking};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let storage = FileStorage::new(FileStorage::default_dir()?)?;
    let client = ZenMoneyBlocking::builder()
        .token("your-api-token")
        .storage(storage)
        .build()?;

    // Incremental sync
    let response = client.sync()?;
    println!("Synced {} transactions", response.transaction.len());

    // Query with filters
    let filter = TransactionFilter::new()
        .payee("grocery");
    let txs = client.filter_transactions(&filter)?;
    println!("Found {} matching transactions", txs.len());

    // Push changes
    let accounts = client.active_accounts()?;
    println!("{} active accounts", accounts.len());

    Ok(())
}
```

### Async client

```rust,no_run
use zenmoney_rs::storage::FileStorage;
use zenmoney_rs::zen_money::ZenMoney;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let storage = FileStorage::new(FileStorage::default_dir()?)?;
    let client = ZenMoney::builder()
        .token("your-api-token")
        .storage(storage)
        .build()?;

    let response = client.sync().await?;
    println!("Synced {} transactions", response.transaction.len());

    Ok(())
}
```

### Testing

Use `InMemoryStorage` for unit and integration tests — no file I/O needed:

```rust
use zenmoney_rs::storage::InMemoryStorage;
use zenmoney_rs::zen_money::ZenMoneyBlocking;

let storage = InMemoryStorage::new();
let client = ZenMoneyBlocking::builder()
    .token("test-token")
    .storage(storage)
    .build()
    .unwrap();
```

## CLI

The `zenmoney` binary provides a command-line interface for syncing and browsing data. Set the `ZENMONEY_TOKEN` environment variable (or use a `.env` file):

```sh
export ZENMONEY_TOKEN=your-api-token

zenmoney diff                              # Incremental sync
zenmoney full-sync                         # Clear and re-sync everything
zenmoney accounts                          # List active accounts
zenmoney transactions                      # List all transactions
zenmoney transactions --from 2024-01-01 --to 2024-12-31  # Date range
zenmoney transactions --account "Cash" --tag "Food"       # Filter by account/tag
zenmoney transactions --payee "grocery" --min-amount 50   # Filter by payee/amount
zenmoney tags                              # List all tags
zenmoney suggest --payee "Starbucks"       # Get category suggestions
```

## Development

This project uses [just](https://github.com/casey/just) as a task runner:

```sh
just              # List all recipes
just check        # Fast compile check
just build        # Build the project
just test         # Run tests
just lint         # Run clippy lints
just fmt          # Format code (requires nightly)
just fmt-check    # Check formatting
just deny         # Audit dependencies
just machete      # Check for unused dependencies
just coverage     # Run code coverage (95% line minimum)
just check-all    # Full pre-commit suite
```

### Requirements

- Rust 1.93+ (pinned via `rust-toolchain.toml`)
- [just](https://github.com/casey/just) (task runner)
- [cargo-deny](https://github.com/EmbarkStudios/cargo-deny) (dependency audit)
- [cargo-machete](https://github.com/bnjbvr/cargo-machete) (unused dependency detection)
- [cargo-llvm-cov](https://github.com/taiki-e/cargo-llvm-cov) (code coverage)

See [CONTRIBUTING.md](CONTRIBUTING.md) for detailed contribution guidelines.

## License

Licensed under either of

- [Apache License, Version 2.0](LICENSE-APACHE)
- [MIT License](LICENSE-MIT)

at your option.
