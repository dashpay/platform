//! FFI bindings for AssetLockManager operations.
//!
//! Mirrors the structure of `platform_wallet::wallet::asset_lock`.

mod build;
mod manager;
mod sync;

pub use build::*;
pub use manager::*;
pub use sync::*;
