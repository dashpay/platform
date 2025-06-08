//! Identity query operations

pub mod balance;
pub mod balance_and_revision;
pub mod by_non_unique_public_key_hash;
pub mod by_public_key_hash;
pub mod contract_nonce;
pub mod fetch;
pub mod identities_balances;
pub mod identities_contract_keys;
pub mod nonce;
pub mod public_keys;
pub mod resolve;

#[cfg(test)]
mod resolve_test;

// Re-export main functions for convenient access
pub use balance::dash_sdk_identity_fetch_balance;
pub use balance_and_revision::dash_sdk_identity_fetch_balance_and_revision;
pub use by_non_unique_public_key_hash::dash_sdk_identity_fetch_by_non_unique_public_key_hash;
pub use by_public_key_hash::dash_sdk_identity_fetch_by_public_key_hash;
pub use fetch::dash_sdk_identity_fetch;
pub use identities_balances::dash_sdk_identities_fetch_balances;
pub use public_keys::dash_sdk_identity_fetch_public_keys;
pub use resolve::dash_sdk_identity_resolve_name;
