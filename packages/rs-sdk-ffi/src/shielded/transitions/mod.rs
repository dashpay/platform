//! Shielded state transition FFI bindings.

pub mod broadcast;
pub mod builders;
pub mod shield;
pub mod shield_from_asset_lock;
pub mod shielded_transfer;
pub mod shielded_withdrawal;
pub mod unshield;

// Build + broadcast combined
pub use shield::dash_sdk_shielded_shield_funds;
pub use shield_from_asset_lock::{
    dash_sdk_shielded_shield_from_chain_lock, dash_sdk_shielded_shield_from_instant_lock,
};
pub use shielded_transfer::dash_sdk_shielded_transfer;
pub use shielded_withdrawal::dash_sdk_shielded_withdraw;
pub use unshield::dash_sdk_shielded_unshield_funds;

// Build-only (returns serialized bytes)
pub use builders::{
    dash_sdk_shielded_build_shield, dash_sdk_shielded_build_shield_from_chain_lock,
    dash_sdk_shielded_build_shield_from_instant_lock, dash_sdk_shielded_build_transfer,
    dash_sdk_shielded_build_unshield, dash_sdk_shielded_build_withdrawal,
};

// Generic broadcast for pre-built transitions
pub use broadcast::dash_sdk_shielded_broadcast;
