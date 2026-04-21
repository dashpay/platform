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
//! - [`network`] — SDK wrappers (`IdentityWallet`,
//!   `DashPayWallet`) that talk to Platform.
//!
//! Every layer depends only on layers above it (reading down).

pub mod crypto;
pub mod network;
pub mod state;
pub mod types;

// --- Flat re-exports for external crates + backward compatibility.
// Consumers used `crate::wallet::identity::X` / `crate::wallet::identity::X`;
// the merged re-export surface preserves the former and absorbs the
// latter so `lib.rs`-level re-exports keep resolving.

pub use crypto::{
    calculate_account_reference, derive_auto_accept_private_key, derive_contact_payment_address,
    derive_contact_payment_addresses, derive_contact_xpub, ContactXpubData,
    DEFAULT_CONTACT_GAP_LIMIT,
};
pub use network::{DashPayWallet, IdentityWallet};
pub use state::{BlockTime, IdentityManager, ManagedIdentity, WatchedIdentity};
pub use types::dashpay::profile::{calculate_avatar_hash, calculate_dhash_fingerprint};
pub use types::{
    ContactRequest, DashPayProfile, DashpayAddressMatch, DpnsNameInfo, EstablishedContact,
    IdentityFunding, IdentityFundingMethod, IdentityStatus, KeyStorage, PaymentDirection,
    PaymentEntry, PaymentStatus, PrivateKeyData, ProfileUpdate, TopUpFundingMethod,
};
