# zenmoney-rs

[![Crates.io](https://img.shields.io/crates/v/zenmoney-rs)](https://crates.io/crates/zenmoney-rs)
[![docs.rs](https://img.shields.io/docsrs/zenmoney-rs)](https://docs.rs/zenmoney-rs)
[![CI](https://github.com/sakost/zenmoney-rs/actions/workflows/ci.yml/badge.svg)](https://github.com/sakost/zenmoney-rs/actions/workflows/ci.yml)
[![License](https://img.shields.io/crates/l/zenmoney-rs)](LICENSE-MIT)

Rust client library for the [ZenMoney](https://zenmoney.ru/) API.

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
zenmoney-rs = "0.1"
```

## Usage

```rust
// TODO: add usage example once the API client is implemented
```

## Development

This project uses [just](https://github.com/casey/just) as a task runner. Available recipes:

```sh
just          # list all recipes
just check    # fast compile check
just build    # build the project
just test     # run tests
just lint     # run clippy lints
just fmt      # format code (requires nightly)
just deny     # audit dependencies
just machete  # check for unused dependencies
just coverage # run code coverage
just check-all # full pre-commit suite
```

### Requirements

- Rust 1.93+ (pinned via `rust-toolchain.toml`)
- [just](https://github.com/casey/just) (task runner)
- [cargo-deny](https://github.com/EmbarkStudios/cargo-deny) (dependency audit)
- [cargo-machete](https://github.com/bnjbvr/cargo-machete) (unused dependency detection)
- [cargo-llvm-cov](https://github.com/taiki-e/cargo-llvm-cov) (code coverage)

## Contributing

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/my-feature`)
3. Ensure all checks pass: `just check-all`
4. Commit your changes using [conventional commits](https://www.conventionalcommits.org/)
5. Open a pull request

## License

Licensed under either of

- [Apache License, Version 2.0](LICENSE-APACHE)
- [MIT License](LICENSE-MIT)

at your option.
