//! Identity operations

mod create;
mod create_from_components;
mod get_public_key;
mod helpers;
mod info;
mod keys;
mod names;
mod parse;
mod put;
mod queries;
mod test_transfer;
mod topup;
mod transfer;
mod withdraw;

// Re-export all public functions for convenient access
pub use create::dash_sdk_identity_create;
pub use create_from_components::{dash_sdk_identity_create_from_components, DashSDKPublicKeyData};
pub use get_public_key::dash_sdk_identity_get_public_key_by_id;
pub use info::{dash_sdk_identity_destroy, dash_sdk_identity_get_info};
pub use keys::{
    dash_sdk_identity_get_signing_key_for_transition, dash_sdk_identity_get_transfer_private_key,
    dash_sdk_identity_public_key_destroy, dash_sdk_identity_public_key_get_id, StateTransitionType,
};
pub use names::dash_sdk_identity_register_name;
pub use parse::dash_sdk_identity_parse_json;
pub use put::{
    dash_sdk_identity_put_to_platform_with_chain_lock,
    dash_sdk_identity_put_to_platform_with_chain_lock_and_wait,
    dash_sdk_identity_put_to_platform_with_instant_lock,
    dash_sdk_identity_put_to_platform_with_instant_lock_and_wait,
};
pub use test_transfer::dash_sdk_test_identity_transfer_crash;
pub use topup::{
    dash_sdk_identity_topup_with_instant_lock, dash_sdk_identity_topup_with_instant_lock_and_wait,
};
pub use transfer::{
    dash_sdk_identity_transfer_credits, dash_sdk_transfer_credits_result_free,
    DashSDKTransferCreditsResult,
};
pub use withdraw::dash_sdk_identity_withdraw;

// Re-export query functions
pub use queries::{
    dash_sdk_identities_fetch_balances, dash_sdk_identity_fetch, dash_sdk_identity_fetch_balance,
    dash_sdk_identity_fetch_balance_and_revision,
    dash_sdk_identity_fetch_by_non_unique_public_key_hash,
    dash_sdk_identity_fetch_by_public_key_hash, dash_sdk_identity_fetch_handle,
    dash_sdk_identity_fetch_public_keys, dash_sdk_identity_resolve_name,
};

// Re-export helper functions for use by submodules
pub use helpers::{
    convert_put_settings, create_chain_asset_lock_proof, create_instant_asset_lock_proof,
    parse_private_key,
};
