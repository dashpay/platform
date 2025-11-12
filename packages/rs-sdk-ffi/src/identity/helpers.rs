//! Helper functions for identity operations

use dash_sdk::dpp::dashcore::{self, Network, PrivateKey};
use dash_sdk::dpp::prelude::{AssetLockProof, UserFeeIncrease};
use dash_sdk::dpp::state_transition::batch_transition::methods::StateTransitionCreationOptions;
use dash_sdk::dpp::state_transition::StateTransitionSigningOptions;
use dash_sdk::platform::transition::put_settings::PutSettings;
use dash_sdk::RequestSettings;
use std::time::Duration;

use crate::types::DashSDKPutSettings;
use crate::FFIError;

/// Helper function to convert DashSDKPutSettings to PutSettings
///
/// # Safety
/// - `put_settings` may be null; when non-null it must be a valid pointer to a `DashSDKPutSettings` structure
///   that remains valid for the duration of the call.
pub unsafe fn convert_put_settings(put_settings: *const DashSDKPutSettings) -> Option<PutSettings> {
    if put_settings.is_null() {
        None
    } else {
        let ios_settings = &*put_settings;

        // Convert request settings
        let mut request_settings = RequestSettings::default();
        if ios_settings.connect_timeout_ms > 0 {
            request_settings.connect_timeout =
                Some(Duration::from_millis(ios_settings.connect_timeout_ms));
        }
        if ios_settings.timeout_ms > 0 {
            request_settings.timeout = Some(Duration::from_millis(ios_settings.timeout_ms));
        }
        if ios_settings.retries > 0 {
            request_settings.retries = Some(ios_settings.retries as usize);
        }
        request_settings.ban_failed_address = Some(ios_settings.ban_failed_address);

        // Convert other settings
        let identity_nonce_stale_time_s = if ios_settings.identity_nonce_stale_time_s > 0 {
            Some(ios_settings.identity_nonce_stale_time_s)
        } else {
            None
        };

        let user_fee_increase = if ios_settings.user_fee_increase > 0 {
            Some(ios_settings.user_fee_increase as UserFeeIncrease)
        } else {
            None
        };

        let signing_options = StateTransitionSigningOptions {
            allow_signing_with_any_security_level: ios_settings
                .allow_signing_with_any_security_level,
            allow_signing_with_any_purpose: ios_settings.allow_signing_with_any_purpose,
        };

        let state_transition_creation_options = Some(StateTransitionCreationOptions {
            signing_options,
            batch_feature_version: None,
            method_feature_version: None,
            base_feature_version: None,
        });

        let wait_timeout = if ios_settings.wait_timeout_ms > 0 {
            Some(Duration::from_millis(ios_settings.wait_timeout_ms))
        } else {
            None
        };

        Some(PutSettings {
            request_settings,
            identity_nonce_stale_time_s,
            user_fee_increase,
            state_transition_creation_options,
            wait_timeout,
        })
    }
}

/// Helper function to parse private key
///
/// # Safety
/// - `private_key_bytes` must be a valid, non-null pointer to 32 readable bytes for the duration of the call.
#[allow(clippy::result_large_err)]
pub unsafe fn parse_private_key(
    private_key_bytes: *const [u8; 32],
) -> Result<PrivateKey, FFIError> {
    let key_bytes = *private_key_bytes;
    let secret_key = dashcore::secp256k1::SecretKey::from_byte_array(&key_bytes)
        .map_err(|e| FFIError::InternalError(format!("Invalid private key: {}", e)))?;
    Ok(PrivateKey::new(secret_key, Network::Dash))
}

/// Helper function to create instant asset lock proof from components
///
/// # Safety
/// - `instant_lock_bytes` must be a valid, non-null pointer to `instant_lock_len` readable bytes.
/// - `transaction_bytes` must be a valid, non-null pointer to `transaction_len` readable bytes.
/// - The pointers must remain valid for the duration of the call.
#[allow(clippy::result_large_err)]
pub unsafe fn create_instant_asset_lock_proof(
    instant_lock_bytes: *const u8,
    instant_lock_len: usize,
    transaction_bytes: *const u8,
    transaction_len: usize,
    output_index: u32,
) -> Result<AssetLockProof, FFIError> {
    use dash_sdk::dpp::dashcore::consensus::deserialize;
    use dash_sdk::dpp::identity::state_transition::asset_lock_proof::instant::InstantAssetLockProof;

    // Deserialize instant lock
    let instant_lock_data = std::slice::from_raw_parts(instant_lock_bytes, instant_lock_len);
    let instant_lock = deserialize(instant_lock_data).map_err(|e| {
        FFIError::InternalError(format!("Failed to deserialize instant lock: {}", e))
    })?;

    // Deserialize transaction
    let transaction_data = std::slice::from_raw_parts(transaction_bytes, transaction_len);
    let transaction = deserialize(transaction_data).map_err(|e| {
        FFIError::InternalError(format!("Failed to deserialize transaction: {}", e))
    })?;

    // Create instant asset lock proof
    let instant_proof = InstantAssetLockProof::new(instant_lock, transaction, output_index);

    Ok(AssetLockProof::Instant(instant_proof))
}

/// Helper function to create chain asset lock proof from components
///
/// # Safety
/// - `out_point_bytes` must be a valid, non-null pointer to 36 readable bytes.
/// - The pointer must remain valid for the duration of the call.
#[allow(clippy::result_large_err)]
pub unsafe fn create_chain_asset_lock_proof(
    core_chain_locked_height: u32,
    out_point_bytes: *const [u8; 36],
) -> Result<AssetLockProof, FFIError> {
    use dash_sdk::dpp::identity::state_transition::asset_lock_proof::chain::ChainAssetLockProof;

    let out_point = *out_point_bytes;

    // Create chain asset lock proof
    let chain_proof = ChainAssetLockProof::new(core_chain_locked_height, out_point);

    Ok(AssetLockProof::Chain(chain_proof))
}
