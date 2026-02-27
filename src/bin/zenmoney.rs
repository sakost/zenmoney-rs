//! CLI binary for smoke-testing the ZenMoney API.
#![allow(
    clippy::exit,
    reason = "CLI binary uses process::exit for fatal errors"
)]

use std::io::{self, Write as _};
use std::process::ExitCode;

use comfy_table::presets::UTF8_FULL;
use comfy_table::{Cell, Color, Table};
use indicatif::{ProgressBar, ProgressStyle};
use owo_colors::OwoColorize;
use zenmoney_rs::client::ZenMoneyBlockingClient;
use zenmoney_rs::models::{DiffRequest, DiffResponse, SuggestRequest, SuggestResponse, TagId};

/// Environment variable name for the API token.
const TOKEN_ENV: &str = "ZENMONEY_TOKEN";

/// Runs the CLI, returning an appropriate exit code.
fn run() -> io::Result<ExitCode> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let _dotenv = dotenvy::dotenv();

    let token = match std::env::var(TOKEN_ENV) {
        Ok(val) if !val.is_empty() => val,
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
            return Ok(ExitCode::FAILURE);
        }
    };

    let client = match ZenMoneyBlockingClient::builder().token(token).build() {
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

    let args: Vec<String> = std::env::args().collect();
    let command = args.get(1).map(String::as_str);

    match command {
        Some("diff") => cmd_diff(&client),
        Some("suggest") => cmd_suggest(&client, &args),
        _ => {
            print_usage()?;
            Ok(ExitCode::FAILURE)
        }
    }
}

/// Executes the `diff` subcommand: full sync and display results.
fn cmd_diff(client: &ZenMoneyBlockingClient) -> io::Result<ExitCode> {
    let spinner = make_spinner("Syncing with ZenMoney API...");

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|dur| dur.as_secs())
        .unwrap_or(0);

    #[allow(
        clippy::cast_possible_wrap,
        reason = "Unix timestamp fits i64 until year 292 billion"
    )]
    let timestamp = now as i64;
    let request = DiffRequest::sync_only(0, timestamp);

    match client.diff(&request) {
        Ok(response) => {
            spinner.finish_and_clear();
            print_diff_summary(&response)?;
            Ok(ExitCode::SUCCESS)
        }
        Err(err) => {
            spinner.finish_and_clear();
            writeln!(
                io::stderr().lock(),
                "{} diff failed: {err}",
                "error:".red().bold()
            )?;
            Ok(ExitCode::FAILURE)
        }
    }
}

/// Parses `--payee` and `--comment` arguments from the argument list.
fn parse_suggest_args(args: &[String]) -> io::Result<Option<SuggestRequest>> {
    let mut payee = None;
    let mut comment = None;
    let mut idx = 2_usize;
    while idx < args.len() {
        match args.get(idx).map(String::as_str) {
            Some("--payee") => {
                idx += 1;
                payee = args.get(idx).cloned();
            }
            Some("--comment") => {
                idx += 1;
                comment = args.get(idx).cloned();
            }
            Some(other) => {
                writeln!(
                    io::stderr().lock(),
                    "{} unknown argument: {other}",
                    "error:".red().bold()
                )?;
                return Ok(None);
            }
            None => break,
        }
        idx += 1;
    }

    if payee.is_none() && comment.is_none() {
        writeln!(
            io::stderr().lock(),
            "{} suggest requires at least --payee or --comment",
            "error:".red().bold()
        )?;
        return Ok(None);
    }

    Ok(Some(SuggestRequest { payee, comment }))
}

/// Executes the `suggest` subcommand: query suggestions for payee/comment.
fn cmd_suggest(client: &ZenMoneyBlockingClient, args: &[String]) -> io::Result<ExitCode> {
    let Some(request) = parse_suggest_args(args)? else {
        return Ok(ExitCode::FAILURE);
    };

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

/// Prints usage information.
fn print_usage() -> io::Result<()> {
    let mut out = io::stdout().lock();
    writeln!(out, "{}", "zenmoney - ZenMoney API CLI".bold())?;
    writeln!(out)?;
    writeln!(out, "{}", "Usage:".yellow().bold())?;
    writeln!(
        out,
        "  zenmoney diff                       Full sync from server"
    )?;
    writeln!(
        out,
        "  zenmoney suggest --payee <name>      Get category suggestions"
    )?;
    writeln!(
        out,
        "  zenmoney suggest --comment <text>    Get suggestions by comment"
    )?;
    writeln!(out)?;
    writeln!(out, "{}", "Environment:".yellow().bold())?;
    writeln!(out, "  {TOKEN_ENV}    API access token (or set in .env)")?;
    writeln!(
        out,
        "  RUST_LOG          Tracing filter (e.g. debug, trace)"
    )?;
    Ok(())
}

/// Entry point.
fn main() -> ExitCode {
    match run() {
        Ok(code) => code,
        Err(err) => {
            // Last-resort error output — if stderr itself failed, nothing we can do.
            let _ignored = writeln!(io::stderr(), "fatal I/O error: {err}");
            ExitCode::FAILURE
        }
    }
}
