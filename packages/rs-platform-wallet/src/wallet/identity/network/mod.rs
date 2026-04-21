//! Network-facing handles — thin wrappers around the shared
//! `WalletManager` + the Platform SDK. Two façades live here:
//!
//! - `IdentityWallet`: identity-only operations (register, top-up,
//!   transfer, withdraw, update, discovery, DPNS, loading).
//! - `DashPayWallet<B>`: DashPay-contract operations (contact
//!   requests, contacts, profile, payments, account labels).
//!
//! Both structs share the underlying `PlatformWalletManager`
//! wallet state; they're two views on the same wallet.

// Identity handle + its operations.
mod discovery;
mod dpns;
mod identity_handle;
mod loading;
mod register_from_addresses;
mod registration;
mod top_up;
mod top_up_from_addresses;
mod transfer;
mod transfer_to_addresses;
mod update;
mod withdrawal;

// DashPay handle + its operations.
mod account_labels;
mod contact_requests;
mod contacts;
mod dashpay_handle;
mod payments;
mod profile;

pub use dashpay_handle::DashPayWallet;
pub use identity_handle::IdentityWallet;

// Helpers declared on `identity_handle.rs` that siblings reach
// through `use super::*;`. Both are `pub(super)` so they stay
// module-private — re-exporting here just avoids each sibling
// having to spell out `identity_handle::` on every call site.
pub(super) use identity_handle::{derive_identity_auth_key_hash, IDENTITY_GAP_LIMIT};
