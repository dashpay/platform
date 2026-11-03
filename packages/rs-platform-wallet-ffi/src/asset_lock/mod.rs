//! FFI bindings for AssetLockManager operations.
//!
//! Mirrors the structure of `platform_wallet::wallet::asset_lock`.

mod build;
mod built_callback;
mod manager;
mod sync;

pub use build::*;
pub use built_callback::AssetLockBuiltFn;
pub(crate) use built_callback::{observer_ref, AssetLockBuiltCallback};
pub use manager::*;
pub use sync::*;
