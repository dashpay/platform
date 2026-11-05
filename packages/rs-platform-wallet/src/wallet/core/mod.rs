pub mod balance;
pub mod balance_handler;
mod broadcast;
pub mod generation;
// Inherent `CoreWallet::sign_message` only — no types to re-export.
mod sign_message;
pub mod spend_authorization;
pub mod spend_observer;
pub mod stored_coin;
pub(crate) use sign_message::is_signable_funding_account;
mod transaction;
mod transaction_decode;
pub mod wallet;

pub use balance::WalletBalance;
pub use balance_handler::BalanceUpdateHandler;
pub use generation::WalletGeneration;
pub(crate) use generation::{InBroadcastFences, InBroadcastPin};
pub use spend_observer::SpendObservationHandler;
pub(crate) use transaction::resolve_source_accounts;
#[cfg(feature = "shielded")]
pub(crate) use transaction::resolved_funding_accounts;
pub use transaction::{SignedCoreTransaction, ASSET_LOCK_FUNDING_SOURCES, SEND_FUNDING_SOURCES};
pub use transaction_decode::{
    decode_transaction, DecodedTransaction, DecodedTxInput, DecodedTxOutput,
};
pub use wallet::CoreWallet;
