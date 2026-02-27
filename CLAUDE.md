# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Rust client/library for the ZenMoney API. Uses Rust edition 2024.

## Build Commands

- **Build:** `cargo build`
- **Run:** `cargo run`
- **Test:** `cargo test`
- **Single test:** `cargo test <test_name>`
- **Clippy lint:** `cargo clippy`
- **Format:** `cargo fmt`
- **Check (fast compile check):** `cargo check`

## Strict Code Rules

### Errors and Safety
- Never use `.unwrap()` or `.expect()` in library code. Use proper error handling with `Result` and the `?` operator.
- `.unwrap()` is only acceptable in tests and examples.
- Never use `unsafe` unless absolutely unavoidable, and always document why it's needed.
- Never silently ignore errors. No `let _ = fallible_call();` — handle or propagate every `Result` and `Option`.

### Code Quality
- All code must pass `cargo clippy -- -D warnings` with zero warnings before committing.
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

### Dependencies
- Avoid adding dependencies for trivial functionality that can be written in a few lines.
- Prefer well-maintained, widely-used crates from the Rust ecosystem.
