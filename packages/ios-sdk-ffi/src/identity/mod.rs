//! Identity operations

pub mod create;
pub mod helpers;
pub mod info;
pub mod names;
pub mod put;
pub mod queries;
pub mod topup;
pub mod transfer;
pub mod withdraw;

// Re-export all public functions for convenient access
pub use create::ios_sdk_identity_create;
pub use info::{ios_sdk_identity_destroy, ios_sdk_identity_get_info};
pub use names::ios_sdk_identity_register_name;
pub use put::{
    ios_sdk_identity_put_to_platform_with_chain_lock,
    ios_sdk_identity_put_to_platform_with_chain_lock_and_wait,
    ios_sdk_identity_put_to_platform_with_instant_lock,
    ios_sdk_identity_put_to_platform_with_instant_lock_and_wait,
};
pub use topup::{
    ios_sdk_identity_topup_with_instant_lock, ios_sdk_identity_topup_with_instant_lock_and_wait,
};
pub use transfer::{
    ios_sdk_identity_transfer_credits, ios_sdk_transfer_credits_result_free,
    IOSSDKTransferCreditsResult,
};
pub use withdraw::ios_sdk_identity_withdraw;

// Re-export query functions
pub use queries::{
    ios_sdk_identity_fetch, ios_sdk_identity_fetch_balance, ios_sdk_identity_fetch_public_keys,
    ios_sdk_identity_resolve_name,
};

// Re-export helper functions for use by submodules
pub use helpers::{
    convert_put_settings, create_chain_asset_lock_proof, create_instant_asset_lock_proof,
    parse_private_key,
};
