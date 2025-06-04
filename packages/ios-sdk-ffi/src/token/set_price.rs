//! Token price setting operations

use super::types::{IOSSDKTokenPriceEntry, IOSSDKTokenPricingType, IOSSDKTokenSetPriceParams};
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
use dash_sdk::dpp::balances::credits::{Credits, TokenAmount};
use dash_sdk::dpp::data_contract::{DataContract, TokenContractPosition};
use dash_sdk::dpp::identity::accessors::IdentityGettersV0;
use dash_sdk::dpp::platform_value::string_encoding::Encoding;
use dash_sdk::dpp::prelude::{Identifier, Identity, UserFeeIncrease};
use dash_sdk::platform::tokens::builders::set_price::TokenSetPriceTransitionBuilder;
use dash_sdk::platform::tokens::transitions::SetPriceResult;
use dash_sdk::platform::IdentityPublicKey;
use std::ffi::CStr;
use std::os::raw::c_char;
use std::sync::Arc;

/// Set token price for direct purchase and wait for confirmation
#[no_mangle]
pub unsafe extern "C" fn ios_sdk_token_set_price(
    sdk_handle: *mut SDKHandle,
    setter_identity_handle: *const IdentityHandle,
    params: *const IOSSDKTokenSetPriceParams,
    identity_public_key_handle: *const crate::types::IdentityPublicKeyHandle,
    signer_handle: *const SignerHandle,
    put_settings: *const IOSSDKPutSettings,
    state_transition_creation_options: *const IOSSDKStateTransitionCreationOptions,
) -> IOSSDKResult {
    // Validate parameters
    if sdk_handle.is_null()
        || setter_identity_handle.is_null()
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
    let setter_identity = &*(setter_identity_handle as *const Identity);
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

    // Validate pricing parameters based on pricing type
    match params.pricing_type {
        IOSSDKTokenPricingType::SinglePrice => {
            if params.single_price == 0 {
                return IOSSDKResult::error(IOSSDKError::new(
                    IOSSDKErrorCode::InvalidParameter,
                    "Single price must be greater than 0".to_string(),
                ));
            }
        }
        IOSSDKTokenPricingType::SetPrices => {
            if params.price_entries.is_null() || params.price_entries_count == 0 {
                return IOSSDKResult::error(IOSSDKError::new(
                    IOSSDKErrorCode::InvalidParameter,
                    "Price entries must be provided for SetPrices pricing type".to_string(),
                ));
            }
        }
    }

    // Parse optional public note
    let public_note = match parse_optional_note(params.public_note) {
        Ok(note) => note,
        Err(e) => return IOSSDKResult::error(e.into()),
    };

    let result: Result<SetPriceResult, FFIError> = wrapper.runtime.block_on(async {
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

        // Create token set price transition builder
        let mut builder = TokenSetPriceTransitionBuilder::new(
            Arc::new(data_contract),
            params.token_position as TokenContractPosition,
            setter_identity.id(),
        );

        // Configure pricing based on the pricing type
        match params.pricing_type {
            IOSSDKTokenPricingType::SinglePrice => {
                builder = builder.with_single_price(params.single_price as Credits);
            }
            IOSSDKTokenPricingType::SetPrices => {
                // Convert FFI price entries to Rust Vec
                let price_entries_slice = std::slice::from_raw_parts(
                    params.price_entries,
                    params.price_entries_count as usize
                );

                let mut price_entries = Vec::new();
                for entry in price_entries_slice {
                    if entry.amount == 0 || entry.price == 0 {
                        return Err(FFIError::InternalError(
                            "Price entry amount and price must be greater than 0".to_string()
                        ));
                    }
                    // Note: This assumes there's a PriceEntry type in the SDK
                    // The actual implementation would need to match the SDK's price entry structure
                    price_entries.push((entry.amount as TokenAmount, entry.price as Credits));
                }

                builder = builder.with_price_entries(price_entries);
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

        // Use SDK method to set price and wait
        let result = wrapper
            .sdk
            .token_set_price(builder, identity_public_key, signer)
            .await
            .map_err(|e| {
                FFIError::InternalError(format!("Failed to set token price and wait: {}", e))
            })?;

        Ok(result)
    });

    match result {
        Ok(_set_price_result) => IOSSDKResult::success(std::ptr::null_mut()),
        Err(e) => IOSSDKResult::error(e.into()),
    }
}
