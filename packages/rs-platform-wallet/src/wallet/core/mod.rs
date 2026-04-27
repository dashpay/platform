//! Core (UTXO) wallet: balances, addresses, broadcast helpers, and the
//! lock-free [`WalletBalance`] used by UI layers.

pub mod balance;
pub mod balance_handler;
mod broadcast;
pub mod wallet;

pub use balance::WalletBalance;
pub use balance_handler::BalanceUpdateHandler;
pub use wallet::CoreWallet;
