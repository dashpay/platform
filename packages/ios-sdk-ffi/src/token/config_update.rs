//! Token configuration update operations

use super::types::{
    IOSSDKTokenConfigUpdateParams, IOSSDKTokenConfigUpdateType,
};
use super::utils::{
    convert_state_transition_creation_options, extract_user_fee_increase, parse_optional_note,
    validate_contract_params,
};
use crate::sdk::SDKWrapper;
use crate::types::{
    IOSSDKPutSettings, IOSSDKStateTransitionCreationOptions, IdentityHandle, SDKHandle,
    SignerHandle,
};
use crate::{FFIError, IOSSDKError, IOSSDKErrorCode, IOSSDKResult};
use dash_sdk::dpp::balances::credits::TokenAmount;
use dash_sdk::dpp::data_contract::{DataContract, TokenContractPosition};
use dash_sdk::dpp::identity::accessors::IdentityGettersV0;
use dash_sdk::dpp::platform_value::string_encoding::Encoding;
use dash_sdk::dpp::prelude::{Identifier, Identity};
use dash_sdk::platform::tokens::builders::config_update::TokenConfigUpdateTransitionBuilder;
use dash_sdk::platform::tokens::transitions::ConfigUpdateResult;
use dash_sdk::platform::IdentityPublicKey;
use std::ffi::CStr;
use std::sync::Arc;

/// Update token configuration and wait for confirmation
#[no_mangle]
pub unsafe extern "C" fn ios_sdk_token_update_contract_token_configuration(
    sdk_handle: *mut SDKHandle,
    updater_identity_handle: *const IdentityHandle,
    params: *const IOSSDKTokenConfigUpdateParams,
    identity_public_key_handle: *const crate::types::IdentityPublicKeyHandle,
    signer_handle: *const SignerHandle,
    put_settings: *const IOSSDKPutSettings,
    state_transition_creation_options: *const IOSSDKStateTransitionCreationOptions,
) -> IOSSDKResult {
    // Validate parameters
    if sdk_handle.is_null()
        || updater_identity_handle.is_null()
        || params.is_null()
        || identity_public_key_handle.is_null()
        || signer_handle.is_null()
    {
        return IOSSDKResult::error(IOSSDKError::new(
            IOSSDKErrorCode::InvalidParameter,
            "One or more required parameters is null".to_string(),
        ));
    }

    let wrapper = &mut *(sdk_handle as *mut SDKWrapper);
    let updater_identity = &*(updater_identity_handle as *const Identity);
    let identity_public_key = &*(identity_public_key_handle as *const IdentityPublicKey);
    let signer = &*(signer_handle as *const crate::signer::IOSSigner);
    let params = &*params;

    // Validate contract parameters
    let (has_contract_id, has_serialized_contract) = match validate_contract_params(
        params.token_contract_id,
        params.serialized_contract,
        params.serialized_contract_len,
    ) {
        Ok(result) => result,
        Err(e) => return IOSSDKResult::error(e.into()),
    };

    // Parse optional public note
    let public_note = match parse_optional_note(params.public_note) {
        Ok(note) => note,
        Err(e) => return IOSSDKResult::error(e.into()),
    };

    // Parse optional identity ID for certain update types
    let identity_id = if params.identity_id.is_null() {
        None
    } else {
        let identity_id_str = match CStr::from_ptr(params.identity_id).to_str() {
            Ok(s) => s,
            Err(e) => return IOSSDKResult::error(FFIError::from(e).into()),
        };

        match Identifier::from_string(identity_id_str, Encoding::Base58) {
            Ok(id) => Some(id),
            Err(e) => {
                return IOSSDKResult::error(IOSSDKError::new(
                    IOSSDKErrorCode::InvalidParameter,
                    format!("Invalid identity ID: {}", e),
                ))
            }
        }
    };

    let result: Result<ConfigUpdateResult, FFIError> = wrapper.runtime.block_on(async {
        // Convert FFI types to Rust types
        let settings = crate::identity::convert_put_settings(put_settings);
        let creation_options = convert_state_transition_creation_options(state_transition_creation_options);
        let user_fee_increase = extract_user_fee_increase(put_settings);

        // Get the data contract either by fetching or deserializing
        use dash_sdk::platform::Fetch;
        use dash_sdk::dpp::prelude::DataContract;

        let data_contract = if has_contract_id {
            // Parse and fetch the contract ID
            let token_contract_id_str = match CStr::from_ptr(params.token_contract_id).to_str() {
                Ok(s) => s,
                Err(e) => return Err(FFIError::from(e)),
            };

            let token_contract_id = match Identifier::from_string(token_contract_id_str, Encoding::Base58) {
                Ok(id) => id,
                Err(e) => {
                    return Err(FFIError::InternalError(format!("Invalid token contract ID: {}", e)))
                }
            };

            // Fetch the data contract
            DataContract::fetch(&wrapper.sdk, token_contract_id)
                .await
                .map_err(FFIError::from)?
                .ok_or_else(|| FFIError::InternalError("Token contract not found".to_string()))?
        } else {
            // Deserialize the provided contract
            let contract_slice = std::slice::from_raw_parts(
                params.serialized_contract,
                params.serialized_contract_len
            );

            use dash_sdk::dpp::serialization::PlatformDeserializableWithPotentialValidationFromVersionedStructure;

            DataContract::versioned_deserialize(
                contract_slice,
                false, // skip validation since it's already validated
                wrapper.sdk.version(),
            )
            .map_err(|e| FFIError::InternalError(format!("Failed to deserialize contract: {}", e)))?
        };

        // Create token config update transition builder
        let mut builder = TokenConfigUpdateTransitionBuilder::new(
            Arc::new(data_contract),
            params.token_position as TokenContractPosition,
            updater_identity.id(),
        );

        // Configure the update based on the update type
        match params.update_type {
            IOSSDKTokenConfigUpdateType::MaxSupply => {
                builder = builder.with_max_supply_update(if params.amount == 0 {
                    None // 0 means unlimited
                } else {
                    Some(params.amount as TokenAmount)
                });
            }
            IOSSDKTokenConfigUpdateType::MintingAllowChoosingDestination => {
                builder = builder.with_minting_allow_choosing_destination(params.bool_value);
            }
            IOSSDKTokenConfigUpdateType::NewTokensDestinationIdentity => {
                if let Some(id) = identity_id {
                    builder = builder.with_new_tokens_destination_identity(id);
                } else {
                    return Err(FFIError::InternalError(
                        "Identity ID required for NewTokensDestinationIdentity update".to_string()
                    ));
                }
            }
            IOSSDKTokenConfigUpdateType::ManualMinting => {
                // Convert FFI action takers to Rust type
                // Note: This would need proper implementation based on the actual SDK types
                // For now, return an error indicating this needs implementation
                return Err(FFIError::InternalError(
                    "ManualMinting config update not yet implemented".to_string()
                ));
            }
            IOSSDKTokenConfigUpdateType::ManualBurning => {
                return Err(FFIError::InternalError(
                    "ManualBurning config update not yet implemented".to_string()
                ));
            }
            IOSSDKTokenConfigUpdateType::Freeze => {
                return Err(FFIError::InternalError(
                    "Freeze config update not yet implemented".to_string()
                ));
            }
            IOSSDKTokenConfigUpdateType::Unfreeze => {
                return Err(FFIError::InternalError(
                    "Unfreeze config update not yet implemented".to_string()
                ));
            }
            IOSSDKTokenConfigUpdateType::MainControlGroup => {
                builder = builder.with_main_control_group(params.group_position);
            }
            IOSSDKTokenConfigUpdateType::NoChange => {
                // No change - builder remains as is
            }
        }

        // Add optional public note
        if let Some(note) = public_note {
            builder = builder.with_public_note(note);
        }

        // Add settings
        if let Some(settings) = settings {
            builder = builder.with_settings(settings);
        }

        // Add user fee increase
        if user_fee_increase > 0 {
            builder = builder.with_user_fee_increase(user_fee_increase);
        }

        // Add state transition creation options
        if let Some(options) = creation_options {
            builder = builder.with_state_transition_creation_options(options);
        }

        // Use SDK method to update config and wait
        let result = wrapper
            .sdk
            .token_update_contract_token_configuration(builder, identity_public_key, signer)
            .await
            .map_err(|e| {
                FFIError::InternalError(format!("Failed to update token config and wait: {}", e))
            })?;

        Ok(result)
    });

    match result {
        Ok(_config_update_result) => IOSSDKResult::success(std::ptr::null_mut()),
        Err(e) => IOSSDKResult::error(e.into()),
    }
}
