//! Identity domain — every aspect of how this wallet owns
//! identities on Platform, including the DashPay contract state
//! layered on top of each identity.
//!
//! Layered by architectural role — a reader can skim the top of
//! any file and immediately know what role it plays:
//!
//! - [`types`] — pure data types (no logic, no network).
//! - [`crypto`] — deterministic cryptographic helpers.
//! - [`state`] — in-memory data + per-identity mutations +
//!   the per-wallet collection manager.
//! - [`network`] — the single `IdentityWallet<B>` SDK handle
//!   covering both identity-lifecycle and DashPay operations.
//!
//! Every layer depends only on layers above it (reading down).

pub mod crypto;
pub mod network;
pub mod state;
pub mod types;

// --- Flat re-exports for external crates + backward compatibility.
// Consumers used `crate::wallet::identity::X` / `crate::wallet::dashpay::X`;
// the merged re-export surface preserves the former and absorbs the
// latter so `lib.rs`-level re-exports keep resolving.

pub use crypto::{
    calculate_account_reference, derive_auto_accept_private_key, derive_contact_payment_address,
    derive_contact_payment_addresses, derive_contact_xpub, pubkey_binds_expected_key_data,
    unmask_account_reference, ContactXpubData, DEFAULT_CONTACT_GAP_LIMIT,
};
pub use network::{DashPayView, IdentityWallet};
pub use state::{
    BlockTime, DashPayState, IdentityLocation, IdentityManager, ManagedIdentity, RegistrationIndex,
};
pub use types::dashpay::profile::{calculate_avatar_hash, calculate_dhash_fingerprint};
pub use types::{
    ContactProfileEntry, ContactRequest, DashPayProfile, DashpayAddressMatch, DpnsNameInfo,
    EstablishedContact, IdentityStatus, KeyStorage, PaymentDirection, PaymentEntry, PaymentStatus,
    PrivateKeyData, ProfileUpdate,
};
