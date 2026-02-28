//! Integration test: create a dummy transaction, verify it, then delete it.
//!
//! Requires `ZENMONEY_TOKEN` environment variable.
//!
//! Run: `cargo run --example create_transaction --features cli`

use std::process::ExitCode;

use chrono::Utc;
use uuid::Uuid;
use zenmoney_rs::models::{InstrumentId, Transaction, TransactionId, UserId};
use zenmoney_rs::storage::FileStorage;
use zenmoney_rs::zen_money::ZenMoneyBlocking;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("error: {err}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let _dotenv = dotenvy::dotenv();

    let token = std::env::var("ZENMONEY_TOKEN")
        .map_err(|_| "ZENMONEY_TOKEN environment variable not set")?;

    let storage = FileStorage::new(FileStorage::default_dir()?)?;
    let client = ZenMoneyBlocking::builder()
        .token(token)
        .storage(storage)
        .build()?;

    // Sync first to have up-to-date data
    println!("Syncing...");
    let _sync = client.sync()?;

    // Use the first active account for the dummy transaction
    let accounts = client.active_accounts()?;
    let account = accounts.first().ok_or("no active accounts found")?;
    println!(
        "Using account: {} ({})",
        account.title,
        account.id.as_inner()
    );

    let instrument = account.instrument.unwrap_or(InstrumentId::new(1));

    let users = client.users()?;
    let user_id = users.first().map(|u| u.id).unwrap_or(UserId::new(0));

    let now = Utc::now();
    let tx_id = TransactionId::new(Uuid::new_v4().to_string());

    let dummy_tx = Transaction {
        id: tx_id.clone(),
        changed: now,
        created: now,
        user: user_id,
        deleted: false,
        hold: Some(false),
        income_instrument: instrument,
        income_account: account.id.clone(),
        income: 0.0,
        outcome_instrument: instrument,
        outcome_account: account.id.clone(),
        outcome: 1.0,
        tag: None,
        merchant: None,
        payee: Some("DUMMY TEST TRANSACTION".to_owned()),
        original_payee: None,
        comment: Some("Created by create_transaction example — safe to delete".to_owned()),
        date: now.date_naive(),
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
    };

    // Push the dummy transaction
    println!("Pushing dummy transaction (id: {})...", tx_id.as_inner());
    let response = client.push_transactions(vec![dummy_tx])?;
    println!(
        "Push response: {} transactions returned by server",
        response.transaction.len()
    );

    // Verify it exists in local storage
    let filter = zenmoney_rs::zen_money::TransactionFilter::new().payee("DUMMY TEST TRANSACTION");
    let found = client.filter_transactions(&filter)?;
    println!("Found {} matching transactions in storage", found.len());

    if found.is_empty() {
        eprintln!("WARNING: transaction not found in storage after push");
    } else {
        for tx in &found {
            println!(
                "  - {} | {} | outcome={:.2}",
                tx.date,
                tx.payee.as_deref().unwrap_or("—"),
                tx.outcome
            );
        }
    }

    // Delete the dummy transaction
    println!("Deleting dummy transaction...");
    let del_response = client.delete_transactions(&[tx_id.clone()])?;
    println!(
        "Delete response: {} deletions returned by server",
        del_response.deletion.len()
    );

    // Verify deletion
    let after_delete = client.filter_transactions(&filter)?;
    println!(
        "After deletion: {} matching transactions remain",
        after_delete.len()
    );

    if after_delete.is_empty() {
        println!("SUCCESS: dummy transaction created and deleted");
    } else {
        eprintln!("WARNING: transaction still exists after deletion");
    }

    Ok(())
}
