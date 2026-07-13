//! FFI bindings for CoreWallet operations.
//!
//! Mirrors the structure of `platform_wallet::wallet::core`.

mod addresses;
mod broadcast;
mod send;
pub(crate) mod signed_payment;
mod transaction_builder;
mod wallet;

pub use addresses::*;
pub use broadcast::*;
pub use send::*;
pub use signed_payment::*;
pub use transaction_builder::*;
pub use wallet::*;
