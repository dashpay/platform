//! FFI bindings for CoreWallet operations.
//!
//! Mirrors the structure of `platform_wallet::wallet::core`.

mod addresses;
mod broadcast;
pub(crate) mod signed_payment;
mod sign_message;
mod transaction_builder;
mod wallet;

pub use addresses::*;
pub use broadcast::*;
pub use signed_payment::*;
pub use sign_message::*;
pub use transaction_builder::*;
pub use wallet::*;
