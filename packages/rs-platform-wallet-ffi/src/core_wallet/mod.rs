//! FFI bindings for CoreWallet operations.
//!
//! Mirrors the structure of `platform_wallet::wallet::core`.

mod addresses;
mod broadcast;
mod transaction_builder;
mod wallet;

pub use addresses::*;
pub use broadcast::*;
pub use transaction_builder::*;
pub use wallet::*;
