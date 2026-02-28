# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Rust client/library for the ZenMoney API. Uses Rust edition 2024.

## Build Commands

- **Build:** `cargo build` or `just build`
- **Test:** `cargo test` or `just test`
- **Single test:** `cargo test <test_name>`
- **Clippy lint:** `cargo clippy` or `just lint`
- **Format:** `cargo fmt` (stable) or `cargo +nightly fmt` / `just fmt` (nightly, full feature set)
- **Format check:** `cargo +nightly fmt --check` or `just fmt-check`
- **Check (fast compile check):** `cargo check` or `just check`
- **Dependency audit:** `cargo deny check` or `just deny`
- **Unused dependency check:** `cargo machete` or `just machete`
- **Code coverage:** `cargo llvm-cov --all-features --fail-under-lines 95` or `just coverage`
- **Coverage HTML report:** `just coverage-html`
- **Copy-paste detection:** `jscpd src/` or `just jscpd`
- **Code metrics:** `rust-code-analysis-cli -m -p ./src/ --pr -O json` or `just metrics`
- **Full pre-commit suite:** `just check-all`
- **Release:** `release-plz update` (update versions/changelog), `release-plz release` (publish)

## Strict Code Rules

### Errors and Safety
- Never use `.unwrap()` or `.expect()` in library code. Use proper error handling with `Result` and the `?` operator.
- `.unwrap()` is only acceptable in tests and examples.
- Never use `unsafe` unless absolutely unavoidable, and always document why it's needed.
- Never silently ignore errors. No `let _ = fallible_call();` — handle or propagate every `Result` and `Option`.

### Code Quality
- All code must pass `cargo clippy` with zero warnings/errors before committing. Lints are configured in `Cargo.toml` at deny/forbid level — clippy pedantic, nursery, and many restriction lints are enforced.
- All code must be formatted with `cargo fmt` before committing.
- No `#[allow(...)]` attributes unless explicitly approved by the user and accompanied by a comment explaining why the lint is intentionally suppressed.
- No `todo!()`, `unimplemented!()`, or `dbg!()` in committed code.
- No dead code: unused functions, imports, variables, or modules must be removed.
- Avoid `clone()` unless necessary — prefer borrowing and references.
- Prefer iterators and combinators over manual loops where they improve clarity.

### Design
- No god structs or god functions. Keep functions short and focused on a single responsibility.
- No magic numbers or string literals — use named constants or enums.
- Prefer strong typing: use newtypes and enums over raw primitives (e.g., `UserId(i64)` over bare `i64`).
- Make invalid states unrepresentable through the type system.
- Public API types must derive or implement `Debug`. Prefer also deriving `Clone`, `PartialEq`, `Eq` where appropriate.
- Keep `pub` visibility to the minimum necessary. Default to private; only make items public when needed by external consumers.

### Commit Messages
- Use [Conventional Commits](https://www.conventionalcommits.org/) format: `<type>(<optional scope>): <Description>`
- Start the description with a **capital letter**, **do not end with a period**
- Allowed types: `feat`, `fix`, `docs`, `perf`, `refactor`, `build`, `ci`, `test`, `chore`
- Append `!` after type/scope for breaking changes: `feat!: Rename sync to incremental_sync`
- See `CONTRIBUTING.md` for full details and examples

### Dependencies
- Avoid adding dependencies for trivial functionality that can be written in a few lines.
- Prefer well-maintained, widely-used crates from the Rust ecosystem.
