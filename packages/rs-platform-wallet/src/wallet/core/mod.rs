pub mod balance;
pub mod balance_handler;
mod broadcast;
pub(crate) mod broadcast_resolver;
pub mod generation;
// Inherent `CoreWallet::sign_message` only — no types to re-export.
mod sign_message;
pub mod spend_observer;
pub(crate) use sign_message::is_signable_funding_account;
mod transaction;
pub mod wallet;

pub use balance::WalletBalance;
pub use balance_handler::BalanceUpdateHandler;
pub use generation::WalletGeneration;
pub(crate) use generation::{InBroadcastFences, InBroadcastPin};
pub use spend_observer::SpendObservationHandler;
pub(crate) use transaction::{
    build_error_awaiting_network, is_shortfall, resolve_source_accounts, trial_with_waiting_coins,
};
pub use transaction::{
    check_fee_rate, is_final, BuilderFactory, FinalizeOptions, SignedCoreTransaction,
    ASSET_LOCK_FUNDING_SOURCES, SEND_FUNDING_SOURCES,
};
pub use wallet::CoreWallet;

use key_wallet::managed_account::transaction_record::TransactionRecord;

/// Whether `record` spends coins the wallet owns: it recorded input details
/// (entries keyed to inputs that spent the account's outpoints) and the
/// account is not a contact's watch-only chain
/// ([`AccountType::is_contact_owned`]), whose input details record the
/// *contact* spending. The persisted projection's input rule and the
/// broadcast resolver's "own send".
///
/// [`AccountType::is_contact_owned`]: key_wallet::account::AccountType::is_contact_owned
pub(crate) fn record_spends_own_coins(record: &TransactionRecord) -> bool {
    !record.input_details.is_empty() && !record.account_type.is_contact_owned()
}
