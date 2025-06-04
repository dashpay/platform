//! Token purchase operations

use super::types::IOSSDKTokenPurchaseParams;
use super::utils::{
    convert_state_transition_creation_options, extract_user_fee_increase, validate_contract_params,
};
use crate::sdk::SDKWrapper;
use crate::types::{
    IOSSDKPutSettings, IOSSDKStateTransitionCreationOptions, IdentityHandle, SDKHandle,
    SignerHandle,
};
use crate::{FFIError, IOSSDKError, IOSSDKErrorCode, IOSSDKResult};
use dash_sdk::dpp::balances::credits::{Credits, TokenAmount};
use dash_sdk::dpp::data_contract::{DataContract, TokenContractPosition};
use dash_sdk::dpp::identity::accessors::IdentityGettersV0;
use dash_sdk::dpp::platform_value::string_encoding::Encoding;
use dash_sdk::dpp::prelude::{Identifier, Identity, UserFeeIncrease};
use dash_sdk::platform::tokens::builders::purchase::TokenPurchaseTransitionBuilder;
use dash_sdk::platform::tokens::transitions::PurchaseResult;
use dash_sdk::platform::IdentityPublicKey;
use std::ffi::CStr;
use std::os::raw::c_char;
use std::sync::Arc;

/// Purchase tokens directly and wait for confirmation
#[no_mangle]
pub unsafe extern "C" fn ios_sdk_token_purchase(
    sdk_handle: *mut SDKHandle,
    buyer_identity_handle: *const IdentityHandle,
    params: *const IOSSDKTokenPurchaseParams,
    identity_public_key_handle: *const crate::types::IdentityPublicKeyHandle,
    signer_handle: *const SignerHandle,
    put_settings: *const IOSSDKPutSettings,
    state_transition_creation_options: *const IOSSDKStateTransitionCreationOptions,
) -> IOSSDKResult {
    // Validate parameters
    if sdk_handle.is_null()
        || buyer_identity_handle.is_null()
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
    let buyer_identity = &*(buyer_identity_handle as *const Identity);
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

    // Validate amount and price
    if params.amount == 0 {
        return IOSSDKResult::error(IOSSDKError::new(
            IOSSDKErrorCode::InvalidParameter,
            "Amount must be greater than 0".to_string(),
        ));
    }

    if params.total_agreed_price == 0 {
        return IOSSDKResult::error(IOSSDKError::new(
            IOSSDKErrorCode::InvalidParameter,
            "Total agreed price must be greater than 0".to_string(),
        ));
    }

    let result: Result<PurchaseResult, FFIError> = wrapper.runtime.block_on(async {
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

        // Create token purchase transition builder
        let mut builder = TokenPurchaseTransitionBuilder::new(
            Arc::new(data_contract),
            params.token_position as TokenContractPosition,
            buyer_identity.id(),
            params.amount as TokenAmount,
            params.total_agreed_price as Credits,
        );

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

        // Use SDK method to purchase and wait
        let result = wrapper
            .sdk
            .token_purchase(builder, identity_public_key, signer)
            .await
            .map_err(|e| {
                FFIError::InternalError(format!("Failed to purchase token and wait: {}", e))
            })?;

        Ok(result)
    });

    match result {
        Ok(_purchase_result) => IOSSDKResult::success(std::ptr::null_mut()),
        Err(e) => IOSSDKResult::error(e.into()),
    }
}
