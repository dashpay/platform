//! Funding source for identity registration and top-up.
//!
//! Re-exports [`AssetLockFunding`](crate::wallet::asset_lock::AssetLockFunding)
//! under the original `IdentityFunding` name so existing callers and
//! the FFI surface keep compiling. The type's body moved to the
//! asset-lock module when platform-address funding adopted the same
//! shape — funding source is funding-target-agnostic; the resolver's
//! `funding_type` parameter picks the BIP44 derivation family.

pub use crate::wallet::asset_lock::AssetLockFunding as IdentityFunding;
