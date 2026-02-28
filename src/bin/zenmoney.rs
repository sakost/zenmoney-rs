//! CLI binary for smoke-testing the ZenMoney API.
#![allow(
    clippy::exit,
    reason = "CLI binary uses process::exit for fatal errors"
)]

use std::io::{self, Write as _};
use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Args, Parser, Subcommand};
use comfy_table::presets::UTF8_FULL;
use comfy_table::{Cell, Color, Table};
use indicatif::{ProgressBar, ProgressStyle};
use owo_colors::OwoColorize;
use zenmoney_rs::models::{
    Account, DiffResponse, NaiveDate, SuggestRequest, SuggestResponse, Tag, TagId, Transaction,
};
use zenmoney_rs::storage::{BlockingStorage, FileStorage};
use zenmoney_rs::zen_money::{TransactionFilter, ZenMoneyBlocking};

/// Environment variable name for the API token.
const TOKEN_ENV: &str = "ZENMONEY_TOKEN";

/// ZenMoney API CLI — sync and browse personal finance data.
#[derive(Debug, Parser)]
#[command(name = "zenmoney", version, about)]
struct Cli {
    /// Override the storage directory (default: XDG data dir).
    #[arg(long, global = true, value_name = "DIR")]
    data_dir: Option<PathBuf>,
    /// Subcommand to execute.
    #[command(subcommand)]
    command: Command,
}

/// Available subcommands.
#[derive(Debug, Subcommand)]
enum Command {
    /// Incremental sync from the ZenMoney server.
    Diff,
    /// Clear local storage and re-sync everything from scratch.
    FullSync,
    /// List active (non-archived) accounts.
    Accounts,
    /// List transactions, optionally filtered by date range, account,
    /// tag, payee, or amount.
    Transactions(TransactionArgs),
    /// List all tags.
    Tags,
    /// Get category suggestions for a payee or comment.
    Suggest {
        /// Payee name to get suggestions for.
        #[arg(long)]
        payee: Option<String>,
        /// Comment text to get suggestions for.
        #[arg(long)]
        comment: Option<String>,
    },
}

/// Arguments for the `transactions` subcommand.
#[derive(Debug, Args)]
struct TransactionArgs {
    /// Start date (inclusive, YYYY-MM-DD). Requires --to.
    #[arg(long, requires = "to", value_parser = parse_date)]
    from: Option<NaiveDate>,
    /// End date (inclusive, YYYY-MM-DD). Requires --from.
    #[arg(long, requires = "from", value_parser = parse_date)]
    to: Option<NaiveDate>,
    /// Filter by account title (case-insensitive).
    #[arg(long)]
    account: Option<String>,
    /// Filter by tag title (case-insensitive).
    #[arg(long)]
    tag: Option<String>,
    /// Filter by payee name (case-insensitive substring match).
    #[arg(long)]
    payee: Option<String>,
    /// Minimum transaction amount (income or outcome).
    #[arg(long)]
    min_amount: Option<f64>,
    /// Maximum transaction amount (income and outcome).
    #[arg(long)]
    max_amount: Option<f64>,
}

/// Parses a date string in `YYYY-MM-DD` format for clap.
fn parse_date(s: &str) -> Result<NaiveDate, String> {
    NaiveDate::parse_from_str(s, "%Y-%m-%d").map_err(|err| format!("{err}"))
}

/// Reads the API token from the environment.
fn read_token() -> io::Result<Option<String>> {
    match std::env::var(TOKEN_ENV) {
        Ok(val) if !val.is_empty() => Ok(Some(val)),
        _ => {
            let mut err = io::stderr().lock();
            writeln!(
                err,
                "{} {} environment variable is not set",
                "error:".red().bold(),
                TOKEN_ENV.bold()
            )?;
            writeln!(
                err,
                "  {} create a .env file with {}=<your_token>",
                "hint:".cyan(),
                TOKEN_ENV
            )?;
            Ok(None)
        }
    }
}

/// Runs the CLI, returning an appropriate exit code.
fn run() -> io::Result<ExitCode> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let _dotenv = dotenvy::dotenv();

    let cli = Cli::parse();

    let Some(token) = read_token()? else {
        return Ok(ExitCode::FAILURE);
    };

    let storage = match create_storage(cli.data_dir) {
        Ok(storage) => storage,
        Err(err) => {
            writeln!(
                io::stderr().lock(),
                "{} failed to initialize storage: {err}",
                "error:".red().bold()
            )?;
            return Ok(ExitCode::FAILURE);
        }
    };

    let client = match ZenMoneyBlocking::builder()
        .token(token)
        .storage(storage)
        .build()
    {
        Ok(client) => client,
        Err(err) => {
            writeln!(
                io::stderr().lock(),
                "{} failed to build client: {err}",
                "error:".red().bold()
            )?;
            return Ok(ExitCode::FAILURE);
        }
    };

    dispatch(&client, cli.command)
}

/// Creates the storage backend, using `data_dir` if provided or the
/// default XDG data directory otherwise.
fn create_storage(data_dir: Option<PathBuf>) -> zenmoney_rs::error::Result<FileStorage> {
    let dir = match data_dir {
        Some(dir) => dir,
        None => FileStorage::default_dir()?,
    };
    FileStorage::new(dir)
}

/// Dispatches to the appropriate subcommand handler.
fn dispatch<S: BlockingStorage>(
    client: &ZenMoneyBlocking<S>,
    command: Command,
) -> io::Result<ExitCode> {
    match command {
        Command::Diff => cmd_diff(client),
        Command::FullSync => cmd_full_sync(client),
        Command::Accounts => cmd_accounts(client),
        Command::Transactions(args) => cmd_transactions(client, &args),
        Command::Tags => cmd_tags(client),
        Command::Suggest { payee, comment } => cmd_suggest(client, payee, comment),
    }
}

/// Executes the `diff` subcommand: incremental sync and display results.
fn cmd_diff<S: BlockingStorage>(client: &ZenMoneyBlocking<S>) -> io::Result<ExitCode> {
    let spinner = make_spinner("Syncing with ZenMoney API...");

    match client.sync() {
        Ok(response) => {
            spinner.finish_and_clear();
            print_diff_summary(&response)?;
            Ok(ExitCode::SUCCESS)
        }
        Err(err) => {
            spinner.finish_and_clear();
            writeln!(
                io::stderr().lock(),
                "{} sync failed: {err}",
                "error:".red().bold()
            )?;
            Ok(ExitCode::FAILURE)
        }
    }
}

/// Executes the `full-sync` subcommand: clears storage and re-syncs
/// from scratch.
fn cmd_full_sync<S: BlockingStorage>(client: &ZenMoneyBlocking<S>) -> io::Result<ExitCode> {
    let spinner = make_spinner("Full sync from ZenMoney API...");

    match client.full_sync() {
        Ok(response) => {
            spinner.finish_and_clear();
            print_diff_summary(&response)?;
            Ok(ExitCode::SUCCESS)
        }
        Err(err) => {
            spinner.finish_and_clear();
            writeln!(
                io::stderr().lock(),
                "{} full sync failed: {err}",
                "error:".red().bold()
            )?;
            Ok(ExitCode::FAILURE)
        }
    }
}

/// Executes the `accounts` subcommand: lists all active accounts.
fn cmd_accounts<S: BlockingStorage>(client: &ZenMoneyBlocking<S>) -> io::Result<ExitCode> {
    match client.active_accounts() {
        Ok(accounts) => {
            print_accounts_table(&accounts)?;
            Ok(ExitCode::SUCCESS)
        }
        Err(err) => {
            writeln!(
                io::stderr().lock(),
                "{} failed to read accounts: {err}",
                "error:".red().bold()
            )?;
            Ok(ExitCode::FAILURE)
        }
    }
}

/// Resolves a named entity to its ID, printing an error on failure.
///
/// Returns `Ok(Some(id))` on success, `Ok(None)` if the entity was not
/// found (error already printed), or `Err` on I/O failure.
fn resolve_name<T, F>(label: &str, name: &str, lookup: F) -> io::Result<Option<T>>
where
    F: FnOnce(&str) -> zenmoney_rs::error::Result<Option<T>>,
{
    match lookup(name) {
        Ok(Some(value)) => Ok(Some(value)),
        Ok(None) => {
            writeln!(
                io::stderr().lock(),
                "{} {label} not found: {name}",
                "error:".red().bold()
            )?;
            Ok(None)
        }
        Err(err) => {
            writeln!(
                io::stderr().lock(),
                "{} failed to look up {label}: {err}",
                "error:".red().bold()
            )?;
            Ok(None)
        }
    }
}

/// Builds a [`TransactionFilter`] from CLI arguments, resolving names
/// to IDs via the client.
fn build_transaction_filter<S: BlockingStorage>(
    client: &ZenMoneyBlocking<S>,
    args: &TransactionArgs,
) -> io::Result<Option<TransactionFilter>> {
    let mut filter = TransactionFilter::new();

    if let Some((from_date, to_date)) = args.from.zip(args.to) {
        filter = filter.date_range(from_date, to_date);
    }
    if let Some(name) = args.account.as_deref() {
        let Some(acc) = resolve_name("account", name, |n| client.find_account_by_title(n))? else {
            return Ok(None);
        };
        filter = filter.account(acc.id);
    }
    if let Some(name) = args.tag.as_deref() {
        let Some(t) = resolve_name("tag", name, |n| client.find_tag_by_title(n))? else {
            return Ok(None);
        };
        filter = filter.tag(t.id);
    }
    if let Some(payee_str) = args.payee.as_deref() {
        filter = filter.payee(payee_str);
    }
    match (args.min_amount, args.max_amount) {
        (Some(min), Some(max)) => filter = filter.amount_range(min, max),
        (Some(min), None) => filter.min_amount = Some(min),
        (None, Some(max)) => filter.max_amount = Some(max),
        (None, None) => {}
    }
    Ok(Some(filter))
}

/// Executes the `transactions` subcommand: lists transactions with
/// optional filters.
fn cmd_transactions<S: BlockingStorage>(
    client: &ZenMoneyBlocking<S>,
    args: &TransactionArgs,
) -> io::Result<ExitCode> {
    let Some(filter) = build_transaction_filter(client, args)? else {
        return Ok(ExitCode::FAILURE);
    };

    match client.filter_transactions(&filter) {
        Ok(txs) => {
            print_transactions_table(&txs)?;
            Ok(ExitCode::SUCCESS)
        }
        Err(err) => {
            writeln!(
                io::stderr().lock(),
                "{} failed to read transactions: {err}",
                "error:".red().bold()
            )?;
            Ok(ExitCode::FAILURE)
        }
    }
}

/// Executes the `tags` subcommand: lists all tags.
fn cmd_tags<S: BlockingStorage>(client: &ZenMoneyBlocking<S>) -> io::Result<ExitCode> {
    match client.tags() {
        Ok(tags) => {
            print_tags_table(&tags)?;
            Ok(ExitCode::SUCCESS)
        }
        Err(err) => {
            writeln!(
                io::stderr().lock(),
                "{} failed to read tags: {err}",
                "error:".red().bold()
            )?;
            Ok(ExitCode::FAILURE)
        }
    }
}

/// Executes the `suggest` subcommand: query suggestions for
/// payee/comment.
fn cmd_suggest<S: BlockingStorage>(
    client: &ZenMoneyBlocking<S>,
    payee: Option<String>,
    comment: Option<String>,
) -> io::Result<ExitCode> {
    if payee.is_none() && comment.is_none() {
        writeln!(
            io::stderr().lock(),
            "{} suggest requires at least --payee or --comment",
            "error:".red().bold()
        )?;
        return Ok(ExitCode::FAILURE);
    }

    let request = SuggestRequest { payee, comment };
    let spinner = make_spinner("Querying suggestions...");

    match client.suggest(&request) {
        Ok(response) => {
            spinner.finish_and_clear();
            print_suggest_result(&response)?;
            Ok(ExitCode::SUCCESS)
        }
        Err(err) => {
            spinner.finish_and_clear();
            writeln!(
                io::stderr().lock(),
                "{} suggest failed: {err}",
                "error:".red().bold()
            )?;
            Ok(ExitCode::FAILURE)
        }
    }
}

// ── Output formatting ────────────────────────────────────────────────

/// Prints the suggest response in a human-readable format.
fn print_suggest_result(response: &SuggestResponse) -> io::Result<()> {
    let mut out = io::stdout().lock();
    writeln!(out, "{}", "Suggestions".green().bold())?;
    writeln!(out)?;
    if let Some(payee_val) = response.payee.as_ref() {
        writeln!(out, "  {} {payee_val}", "Payee:".bold())?;
    }
    if let Some(merchant) = response.merchant.as_ref() {
        writeln!(out, "  {} {merchant}", "Merchant:".bold())?;
    }
    if let Some(tags) = response.tag.as_ref() {
        let tag_list: Vec<&str> = tags.iter().map(TagId::as_inner).collect();
        writeln!(out, "  {} {}", "Tags:".bold(), tag_list.join(", "))?;
    }
    Ok(())
}

/// Prints accounts in a table.
fn print_accounts_table(accounts: &[Account]) -> io::Result<()> {
    let mut out = io::stdout().lock();
    if accounts.is_empty() {
        writeln!(out, "{}", "No accounts found.".dimmed())?;
        return Ok(());
    }

    let mut table = Table::new();
    _ = table.load_preset(UTF8_FULL);
    _ = table.set_header(vec![
        Cell::new("Title").fg(Color::Cyan),
        Cell::new("Type").fg(Color::Cyan),
        Cell::new("Balance").fg(Color::Cyan),
    ]);

    for acc in accounts {
        let balance_str = acc
            .balance
            .map_or_else(|| "\u{2014}".to_owned(), |bal| format!("{bal:.2}"));
        let type_str = format!("{:?}", acc.kind);
        _ = table.add_row(vec![
            Cell::new(&acc.title),
            Cell::new(type_str),
            Cell::new(balance_str),
        ]);
    }

    writeln!(
        out,
        "{} {}",
        "Active Accounts".green().bold(),
        format_args!("({})", accounts.len()).dimmed()
    )?;
    writeln!(out)?;
    writeln!(out, "{table}")?;
    Ok(())
}

/// Prints transactions in a table.
fn print_transactions_table(txs: &[Transaction]) -> io::Result<()> {
    let mut out = io::stdout().lock();
    if txs.is_empty() {
        writeln!(out, "{}", "No transactions found.".dimmed())?;
        return Ok(());
    }

    let mut table = Table::new();
    _ = table.load_preset(UTF8_FULL);
    _ = table.set_header(vec![
        Cell::new("Date").fg(Color::Cyan),
        Cell::new("Payee").fg(Color::Cyan),
        Cell::new("Outcome").fg(Color::Cyan),
        Cell::new("Income").fg(Color::Cyan),
        Cell::new("Comment").fg(Color::Cyan),
    ]);

    for tx in txs {
        let payee = tx.payee.as_deref().unwrap_or("\u{2014}");
        let comment = tx.comment.as_deref().unwrap_or("");

        let outcome_cell = if tx.outcome > 0.0_f64 {
            Cell::new(format!("{:.2}", tx.outcome)).fg(Color::Red)
        } else {
            Cell::new("\u{2014}").fg(Color::DarkGrey)
        };

        let income_cell = if tx.income > 0.0_f64 {
            Cell::new(format!("{:.2}", tx.income)).fg(Color::Green)
        } else {
            Cell::new("\u{2014}").fg(Color::DarkGrey)
        };

        _ = table.add_row(vec![
            Cell::new(tx.date),
            Cell::new(payee),
            outcome_cell,
            income_cell,
            Cell::new(comment),
        ]);
    }

    writeln!(
        out,
        "{} {}",
        "Transactions".green().bold(),
        format_args!("({})", txs.len()).dimmed()
    )?;
    writeln!(out)?;
    writeln!(out, "{table}")?;
    Ok(())
}

/// Prints tags in a table.
fn print_tags_table(tags: &[Tag]) -> io::Result<()> {
    let mut out = io::stdout().lock();
    if tags.is_empty() {
        writeln!(out, "{}", "No tags found.".dimmed())?;
        return Ok(());
    }

    let mut table = Table::new();
    _ = table.load_preset(UTF8_FULL);
    _ = table.set_header(vec![
        Cell::new("Title").fg(Color::Cyan),
        Cell::new("Parent").fg(Color::Cyan),
    ]);

    for tag in tags {
        let parent = tag
            .parent
            .as_ref()
            .map_or_else(|| "\u{2014}".to_owned(), ToString::to_string);
        _ = table.add_row(vec![Cell::new(&tag.title), Cell::new(parent)]);
    }

    writeln!(
        out,
        "{} {}",
        "Tags".green().bold(),
        format_args!("({})", tags.len()).dimmed()
    )?;
    writeln!(out)?;
    writeln!(out, "{table}")?;
    Ok(())
}

/// Creates a spinner with the given message.
fn make_spinner(message: &str) -> ProgressBar {
    let spinner = ProgressBar::new_spinner();
    spinner.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.cyan} {msg}")
            .unwrap_or_else(|_| ProgressStyle::default_spinner()),
    );
    spinner.set_message(message.to_owned());
    spinner.enable_steady_tick(core::time::Duration::from_millis(80));
    spinner
}

/// Prints a summary table of a diff response.
fn print_diff_summary(response: &DiffResponse) -> io::Result<()> {
    let mut out = io::stdout().lock();
    writeln!(
        out,
        "{} {}",
        "Sync complete!".green().bold(),
        format_args!("(server timestamp: {})", response.server_timestamp).dimmed()
    )?;
    writeln!(out)?;

    let mut table = Table::new();
    _ = table.load_preset(UTF8_FULL);
    _ = table.set_header(vec![
        Cell::new("Entity").fg(Color::Cyan),
        Cell::new("Count").fg(Color::Cyan),
    ]);

    let rows: &[(&str, usize)] = &[
        ("Instruments", response.instrument.len()),
        ("Companies", response.company.len()),
        ("Users", response.user.len()),
        ("Accounts", response.account.len()),
        ("Tags", response.tag.len()),
        ("Merchants", response.merchant.len()),
        ("Transactions", response.transaction.len()),
        ("Reminders", response.reminder.len()),
        ("Reminder Markers", response.reminder_marker.len()),
        ("Budgets", response.budget.len()),
        ("Deletions", response.deletion.len()),
    ];

    for &(name, count) in rows {
        let count_cell = if count > 0 {
            Cell::new(count).fg(Color::Green)
        } else {
            Cell::new(count).fg(Color::DarkGrey)
        };
        _ = table.add_row(vec![Cell::new(name), count_cell]);
    }

    writeln!(out, "{table}")?;
    Ok(())
}

/// Entry point.
fn main() -> ExitCode {
    match run() {
        Ok(code) => code,
        Err(err) => {
            // Last-resort error output — if stderr itself failed, nothing
            // we can do.
            let _ignored = writeln!(io::stderr(), "fatal I/O error: {err}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use chrono::DateTime;
    use zenmoney_rs::models::{
        AccountId, AccountType, DiffResponse, InstrumentId, MerchantId, SuggestResponse, TagId,
        TransactionId, UserId,
    };
    use zenmoney_rs::storage::InMemoryStorage;

    /// Creates a test account.
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
            balance: Some(1000.0),
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

    /// Creates a test transaction.
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
            outcome: 50.0,
            tag: None,
            merchant: None,
            payee: Some("Test Payee".to_owned()),
            original_payee: None,
            comment: Some("Test comment".to_owned()),
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

    /// Creates a test tag.
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

    /// Creates a mock `ZenMoneyBlocking` with a pre-populated storage.
    fn mock_client() -> ZenMoneyBlocking<InMemoryStorage> {
        ZenMoneyBlocking::builder()
            .token("test-token")
            .storage(InMemoryStorage::new())
            .build()
            .unwrap()
    }

    // ── parse_date tests ──────────────────────────────────────────────

    #[test]
    fn parse_date_valid() {
        let date = parse_date("2024-01-15").unwrap();
        assert_eq!(date, NaiveDate::from_ymd_opt(2024, 1, 15).unwrap());
    }

    #[test]
    fn parse_date_invalid() {
        assert!(parse_date("not-a-date").is_err());
        assert!(parse_date("01-15-2024").is_err());
    }

    // ── create_storage tests ──────────────────────────────────────────

    #[test]
    fn create_storage_with_custom_dir() {
        let dir = tempfile::tempdir().unwrap();
        let storage = create_storage(Some(dir.path().to_path_buf()));
        assert!(storage.is_ok());
    }

    #[test]
    fn create_storage_with_default_dir() {
        let storage = create_storage(None);
        assert!(storage.is_ok());
    }

    // ── resolve_name tests ────────────────────────────────────────────

    #[test]
    fn resolve_name_found() {
        let result = resolve_name("account", "Test", |_| Ok(Some(42_i32))).unwrap();
        assert_eq!(result, Some(42_i32));
    }

    #[test]
    fn resolve_name_not_found() {
        let result = resolve_name::<i32, _>("account", "Missing", |_| Ok(None)).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn resolve_name_lookup_error() {
        let result = resolve_name::<i32, _>("account", "Bad", |_| {
            Err(zenmoney_rs::error::ZenMoneyError::Storage(Box::from(
                "lookup failed",
            )))
        })
        .unwrap();
        assert!(result.is_none());
    }

    // ── build_transaction_filter tests ────────────────────────────────

    #[test]
    fn build_filter_no_args() {
        let client = mock_client();
        let args = TransactionArgs {
            from: None,
            to: None,
            account: None,
            tag: None,
            payee: None,
            min_amount: None,
            max_amount: None,
        };
        let filter = build_transaction_filter(&client, &args).unwrap();
        assert!(filter.is_some());
    }

    #[test]
    fn build_filter_with_date_range() {
        let client = mock_client();
        let args = TransactionArgs {
            from: Some(NaiveDate::from_ymd_opt(2024, 1, 1).unwrap()),
            to: Some(NaiveDate::from_ymd_opt(2024, 12, 31).unwrap()),
            account: None,
            tag: None,
            payee: None,
            min_amount: None,
            max_amount: None,
        };
        let filter = build_transaction_filter(&client, &args).unwrap().unwrap();
        assert!(filter.date_from.is_some());
        assert!(filter.date_to.is_some());
    }

    #[test]
    fn build_filter_account_not_found_returns_none() {
        let client = mock_client();
        let args = TransactionArgs {
            from: None,
            to: None,
            account: Some("Nonexistent".to_owned()),
            tag: None,
            payee: None,
            min_amount: None,
            max_amount: None,
        };
        let filter = build_transaction_filter(&client, &args).unwrap();
        assert!(filter.is_none());
    }

    #[test]
    fn build_filter_tag_not_found_returns_none() {
        let client = mock_client();
        let args = TransactionArgs {
            from: None,
            to: None,
            account: None,
            tag: Some("Nonexistent".to_owned()),
            payee: None,
            min_amount: None,
            max_amount: None,
        };
        let filter = build_transaction_filter(&client, &args).unwrap();
        assert!(filter.is_none());
    }

    #[test]
    fn build_filter_with_account_found() {
        let storage = InMemoryStorage::new();
        storage
            .upsert_accounts(vec![test_account("a-1", "Checking", false)])
            .unwrap();
        let client = ZenMoneyBlocking::builder()
            .token("test")
            .storage(storage)
            .build()
            .unwrap();
        let args = TransactionArgs {
            from: None,
            to: None,
            account: Some("Checking".to_owned()),
            tag: None,
            payee: None,
            min_amount: None,
            max_amount: None,
        };
        let filter = build_transaction_filter(&client, &args).unwrap().unwrap();
        assert!(filter.account.is_some());
    }

    #[test]
    fn build_filter_with_tag_found() {
        let storage = InMemoryStorage::new();
        storage.upsert_tags(vec![test_tag("t-1", "Food")]).unwrap();
        let client = ZenMoneyBlocking::builder()
            .token("test")
            .storage(storage)
            .build()
            .unwrap();
        let args = TransactionArgs {
            from: None,
            to: None,
            account: None,
            tag: Some("Food".to_owned()),
            payee: None,
            min_amount: None,
            max_amount: None,
        };
        let filter = build_transaction_filter(&client, &args).unwrap().unwrap();
        assert!(filter.tag.is_some());
    }

    #[test]
    fn build_filter_with_payee() {
        let client = mock_client();
        let args = TransactionArgs {
            from: None,
            to: None,
            account: None,
            tag: None,
            payee: Some("Coffee".to_owned()),
            min_amount: None,
            max_amount: None,
        };
        let filter = build_transaction_filter(&client, &args).unwrap().unwrap();
        assert!(filter.payee.is_some());
    }

    #[test]
    fn build_filter_with_amount_range() {
        let client = mock_client();
        let args = TransactionArgs {
            from: None,
            to: None,
            account: None,
            tag: None,
            payee: None,
            min_amount: Some(10.0),
            max_amount: Some(100.0),
        };
        let filter = build_transaction_filter(&client, &args).unwrap().unwrap();
        assert!(filter.min_amount.is_some());
        assert!(filter.max_amount.is_some());
    }

    #[test]
    fn build_filter_with_min_only() {
        let client = mock_client();
        let args = TransactionArgs {
            from: None,
            to: None,
            account: None,
            tag: None,
            payee: None,
            min_amount: Some(10.0),
            max_amount: None,
        };
        let filter = build_transaction_filter(&client, &args).unwrap().unwrap();
        assert!(filter.min_amount.is_some());
        assert!(filter.max_amount.is_none());
    }

    #[test]
    fn build_filter_with_max_only() {
        let client = mock_client();
        let args = TransactionArgs {
            from: None,
            to: None,
            account: None,
            tag: None,
            payee: None,
            min_amount: None,
            max_amount: Some(100.0),
        };
        let filter = build_transaction_filter(&client, &args).unwrap().unwrap();
        assert!(filter.min_amount.is_none());
        assert!(filter.max_amount.is_some());
    }

    // ── print function tests ─────────────────────────────────────────

    #[test]
    fn print_accounts_table_empty() {
        assert!(print_accounts_table(&[]).is_ok());
    }

    #[test]
    fn print_accounts_table_with_data() {
        let accounts = vec![
            test_account("a-1", "Checking", false),
            test_account("a-2", "Savings", false),
        ];
        assert!(print_accounts_table(&accounts).is_ok());
    }

    #[test]
    fn print_transactions_table_empty() {
        assert!(print_transactions_table(&[]).is_ok());
    }

    #[test]
    fn print_transactions_table_with_data() {
        let txs = vec![
            test_transaction("tx-1", "a-1", NaiveDate::from_ymd_opt(2024, 1, 1).unwrap()),
            {
                let mut tx =
                    test_transaction("tx-2", "a-1", NaiveDate::from_ymd_opt(2024, 1, 2).unwrap());
                tx.income = 200.0;
                tx.outcome = 0.0;
                tx.payee = None;
                tx.comment = None;
                tx
            },
        ];
        assert!(print_transactions_table(&txs).is_ok());
    }

    #[test]
    fn print_tags_table_empty() {
        assert!(print_tags_table(&[]).is_ok());
    }

    #[test]
    fn print_tags_table_with_data() {
        let tags = vec![test_tag("t-1", "Food"), {
            let mut t = test_tag("t-2", "Fast Food");
            t.parent = Some(TagId::new("t-1".to_owned()));
            t
        }];
        assert!(print_tags_table(&tags).is_ok());
    }

    #[test]
    fn print_diff_summary_works() {
        let response = DiffResponse {
            server_timestamp: DateTime::from_timestamp(1_700_000_100, 0).unwrap(),
            instrument: Vec::new(),
            country: Vec::new(),
            company: Vec::new(),
            user: Vec::new(),
            account: vec![test_account("a-1", "Test", false)],
            tag: Vec::new(),
            merchant: Vec::new(),
            transaction: Vec::new(),
            reminder: Vec::new(),
            reminder_marker: Vec::new(),
            budget: Vec::new(),
            deletion: Vec::new(),
        };
        assert!(print_diff_summary(&response).is_ok());
    }

    #[test]
    fn print_suggest_result_works() {
        let response = SuggestResponse {
            payee: Some("Starbucks".to_owned()),
            merchant: Some(MerchantId::new("m-1".to_owned())),
            tag: Some(vec![TagId::new("t-1".to_owned())]),
        };
        assert!(print_suggest_result(&response).is_ok());
    }

    #[test]
    fn print_suggest_result_empty() {
        let response = SuggestResponse {
            payee: None,
            merchant: None,
            tag: None,
        };
        assert!(print_suggest_result(&response).is_ok());
    }

    // ── make_spinner test ────────────────────────────────────────────

    #[test]
    fn make_spinner_creates_spinner() {
        let spinner = make_spinner("Testing...");
        spinner.finish_and_clear();
    }

    // ── cmd_* tests ──────────────────────────────────────────────────

    #[test]
    fn cmd_accounts_empty() {
        let client = mock_client();
        let code = cmd_accounts(&client).unwrap();
        assert_eq!(code, ExitCode::SUCCESS);
    }

    #[test]
    fn cmd_accounts_with_data() {
        let storage = InMemoryStorage::new();
        storage
            .upsert_accounts(vec![test_account("a-1", "Checking", false)])
            .unwrap();
        let client = ZenMoneyBlocking::builder()
            .token("test")
            .storage(storage)
            .build()
            .unwrap();
        let code = cmd_accounts(&client).unwrap();
        assert_eq!(code, ExitCode::SUCCESS);
    }

    #[test]
    fn cmd_tags_empty() {
        let client = mock_client();
        let code = cmd_tags(&client).unwrap();
        assert_eq!(code, ExitCode::SUCCESS);
    }

    #[test]
    fn cmd_tags_with_data() {
        let storage = InMemoryStorage::new();
        storage.upsert_tags(vec![test_tag("t-1", "Food")]).unwrap();
        let client = ZenMoneyBlocking::builder()
            .token("test")
            .storage(storage)
            .build()
            .unwrap();
        let code = cmd_tags(&client).unwrap();
        assert_eq!(code, ExitCode::SUCCESS);
    }

    #[test]
    fn cmd_transactions_empty() {
        let client = mock_client();
        let args = TransactionArgs {
            from: None,
            to: None,
            account: None,
            tag: None,
            payee: None,
            min_amount: None,
            max_amount: None,
        };
        let code = cmd_transactions(&client, &args).unwrap();
        assert_eq!(code, ExitCode::SUCCESS);
    }

    #[test]
    fn cmd_transactions_with_data() {
        let storage = InMemoryStorage::new();
        storage
            .upsert_transactions(vec![test_transaction(
                "tx-1",
                "a-1",
                NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            )])
            .unwrap();
        let client = ZenMoneyBlocking::builder()
            .token("test")
            .storage(storage)
            .build()
            .unwrap();
        let args = TransactionArgs {
            from: None,
            to: None,
            account: None,
            tag: None,
            payee: None,
            min_amount: None,
            max_amount: None,
        };
        let code = cmd_transactions(&client, &args).unwrap();
        assert_eq!(code, ExitCode::SUCCESS);
    }

    #[test]
    fn cmd_transactions_filter_not_found() {
        let client = mock_client();
        let args = TransactionArgs {
            from: None,
            to: None,
            account: Some("Nonexistent".to_owned()),
            tag: None,
            payee: None,
            min_amount: None,
            max_amount: None,
        };
        let code = cmd_transactions(&client, &args).unwrap();
        assert_eq!(code, ExitCode::FAILURE);
    }

    #[test]
    fn cmd_suggest_no_args() {
        let client = mock_client();
        let code = cmd_suggest(&client, None, None).unwrap();
        assert_eq!(code, ExitCode::FAILURE);
    }

    // ── dispatch tests ───────────────────────────────────────────────

    #[test]
    fn dispatch_accounts() {
        let client = mock_client();
        let code = dispatch(&client, Command::Accounts).unwrap();
        assert_eq!(code, ExitCode::SUCCESS);
    }

    #[test]
    fn dispatch_tags() {
        let client = mock_client();
        let code = dispatch(&client, Command::Tags).unwrap();
        assert_eq!(code, ExitCode::SUCCESS);
    }

    #[test]
    fn dispatch_transactions() {
        let client = mock_client();
        let code = dispatch(
            &client,
            Command::Transactions(TransactionArgs {
                from: None,
                to: None,
                account: None,
                tag: None,
                payee: None,
                min_amount: None,
                max_amount: None,
            }),
        )
        .unwrap();
        assert_eq!(code, ExitCode::SUCCESS);
    }
}
