//! Pure cryptographic helpers used across the identity domain.
//!
//! No state, no network — just deterministic functions over keys,
//! paths, and bytes.

pub mod auto_accept;
pub mod dip14;
pub mod validation;

pub use auto_accept::derive_auto_accept_private_key;
pub use dip14::{
    calculate_account_reference, unmask_account_reference, derive_contact_payment_address, derive_contact_payment_addresses,
    derive_contact_xpub, ContactXpubData, DEFAULT_CONTACT_GAP_LIMIT,
};
