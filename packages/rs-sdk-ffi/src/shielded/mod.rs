//! Shielded pool queries and state transition FFI bindings.

mod crypto;
mod queries;
mod transitions;
pub(crate) mod types;

// Re-export all query functions
pub use queries::*;

// Re-export crypto functions (address derivation, bundle building, decryption)
pub use crypto::*;

// Re-export transition functions — build + broadcast combined
pub use transitions::dash_sdk_shielded_shield_from_chain_lock;
pub use transitions::dash_sdk_shielded_shield_from_instant_lock;
pub use transitions::dash_sdk_shielded_shield_funds;
pub use transitions::dash_sdk_shielded_transfer;
pub use transitions::dash_sdk_shielded_unshield_funds;
pub use transitions::dash_sdk_shielded_withdraw;

// Re-export builder functions — build only (returns serialized bytes)
pub use transitions::dash_sdk_shielded_build_shield;
pub use transitions::dash_sdk_shielded_build_shield_from_chain_lock;
pub use transitions::dash_sdk_shielded_build_shield_from_instant_lock;
pub use transitions::dash_sdk_shielded_build_transfer;
pub use transitions::dash_sdk_shielded_build_unshield;
pub use transitions::dash_sdk_shielded_build_withdrawal;

// Re-export generic broadcast
pub use transitions::dash_sdk_shielded_broadcast;

// Re-export FFI types
pub use transitions::shield::DashSDKShieldInput;
pub use types::{DashSDKOrchardBundleParams, DashSDKSerializedAction};
