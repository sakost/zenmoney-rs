//! CLI binary for smoke-testing the ZenMoney API.
#![allow(
    clippy::exit,
    reason = "CLI binary uses process::exit for fatal errors"
)]

use std::io::{self, Write as _};
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

    let storage = match create_storage() {
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

/// Creates the storage backend in the default XDG data directory.
fn create_storage() -> zenmoney_rs::error::Result<FileStorage> {
    let dir = FileStorage::default_dir()?;
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
