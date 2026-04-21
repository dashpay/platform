//! Network-facing handle — a single façade over the shared
//! `WalletManager` + the Platform SDK.
//!
//! `IdentityWallet<B>` covers both the identity lifecycle (register,
//! top-up, transfer, withdraw, update, discovery, DPNS, loading) and
//! the DashPay-contract operations that live on the same identity
//! (contact requests, contacts, profile, payments, account labels).
//!
//! Historically DashPay lived on a separate `DashPayWallet<B>` facade,
//! but both views operated on the same underlying state (a single
//! `ManagedIdentity` carries both identity fields and DashPay fields).
//! The two facades were merged to cut handle-juggling at the FFI
//! boundary and so DashPay ops can reuse the same signer / asset-lock
//! plumbing the identity lifecycle already owns. `B` picks the
//! transaction broadcaster used by DashPay `send_payment`; it defaults
//! to `SpvBroadcaster` so most call sites don't need to name it.

// Core handle + identity-lifecycle operations.
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

// DashPay-contract operations (same `IdentityWallet` impl blocks).
mod account_labels;
mod contact_requests;
mod contacts;
mod dashpay_sync;
mod payments;
mod profile;

pub use dpns::{ContestContender, ContestVoteState, ContestWinner};
pub use identity_handle::IdentityWallet;

// Helpers declared on `identity_handle.rs` that siblings reach
// through `use super::*;`. All are `pub(super)` so they stay
// module-private — re-exporting here just avoids each sibling
// having to spell out `identity_handle::` on every call site.
pub(super) use identity_handle::{derive_identity_auth_key_hash, IDENTITY_GAP_LIMIT};
