//! Data models for ZenMoney API entities.
//!
//! This module contains strongly-typed representations of all ZenMoney
//! entities, newtype ID wrappers, and enumeration types for constrained
//! values.

mod account;
mod budget;
mod company;
mod diff;
mod enums;
mod ids;
mod instrument;
mod merchant;
mod reminder;
mod reminder_marker;
mod suggest;
mod tag;
mod transaction;
mod user;

pub use account::Account;
pub use budget::Budget;
pub use company::Company;
pub use diff::{Deletion, DiffRequest, DiffResponse};
pub use enums::{AccountType, Interval, PayoffInterval, ReminderMarkerState};
pub use ids::{
    AccountId, CompanyId, InstrumentId, MerchantId, ReminderId, ReminderMarkerId, TagId,
    TransactionId, UserId,
};
pub use instrument::Instrument;
pub use merchant::Merchant;
pub use reminder::Reminder;
pub use reminder_marker::ReminderMarker;
pub use suggest::{SuggestRequest, SuggestResponse};
pub use tag::Tag;
pub use transaction::Transaction;
pub use user::User;
