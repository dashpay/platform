//! Common utilities for token operations

use super::types::DashSDKTokenDistributionType;
use crate::types::{DashSDKPutSettings, DashSDKStateTransitionCreationOptions};
use crate::{sdk::SDKWrapper, FFIError};
use dash_sdk::dpp::data_contract::associated_token::token_distribution_key::TokenDistributionType;
use dash_sdk::dpp::data_contract::{DataContract, TokenContractPosition};
use dash_sdk::dpp::platform_value::string_encoding::Encoding;
use dash_sdk::dpp::prelude::{Identifier, UserFeeIncrease};
use dash_sdk::dpp::state_transition::batch_transition::methods::StateTransitionCreationOptions;
use dash_sdk::dpp::state_transition::StateTransitionSigningOptions;
use std::ffi::CStr;
use std::os::raw::c_char;

/// Convert FFI StateTransitionCreationOptions to Rust StateTransitionCreationOptions
pub unsafe fn convert_state_transition_creation_options(
    ffi_options: *const DashSDKStateTransitionCreationOptions,
) -> Option<StateTransitionCreationOptions> {
    if ffi_options.is_null() {
        return None;
    }

    let options = &*ffi_options;

    let signing_options = StateTransitionSigningOptions {
        allow_signing_with_any_security_level: options.allow_signing_with_any_security_level,
        allow_signing_with_any_purpose: options.allow_signing_with_any_purpose,
    };

    Some(StateTransitionCreationOptions {
        signing_options,
        batch_feature_version: if options.batch_feature_version == 0 {
            None
        } else {
            Some(options.batch_feature_version)
        },
        method_feature_version: if options.method_feature_version == 0 {
            None
        } else {
            Some(options.method_feature_version)
        },
        base_feature_version: if options.base_feature_version == 0 {
            None
        } else {
            Some(options.base_feature_version)
        },
    })
}

/// Convert FFI TokenDistributionType to Rust TokenDistributionType
pub fn convert_token_distribution_type(
    ffi_type: DashSDKTokenDistributionType,
) -> TokenDistributionType {
    match ffi_type {
        DashSDKTokenDistributionType::PreProgrammed => TokenDistributionType::PreProgrammed,
        DashSDKTokenDistributionType::Perpetual => TokenDistributionType::Perpetual,
    }
}

/// Helper function to get data contract from either ID or serialized data
pub unsafe fn get_data_contract(
    token_contract_id: *const c_char,
    serialized_contract: *const u8,
    serialized_contract_len: usize,
    sdk: &dash_sdk::Sdk,
) -> Result<DataContract, FFIError> {
    if !token_contract_id.is_null() {
        // Use contract ID to fetch from platform
        let contract_id_str = CStr::from_ptr(token_contract_id)
            .to_str()
            .map_err(FFIError::from)?;
        let contract_id = Identifier::from_string(contract_id_str, Encoding::Base58)
            .map_err(|e| FFIError::InternalError(format!("Invalid contract ID: {}", e)))?;

        // TODO: Implement contract fetching from platform
        // For now, return an error as this requires async implementation
        Err(FFIError::InternalError(
            "Contract fetching from platform not implemented in FFI layer".to_string(),
        ))
    } else if !serialized_contract.is_null() && serialized_contract_len > 0 {
        // Deserialize contract from provided data
        let contract_data =
            std::slice::from_raw_parts(serialized_contract, serialized_contract_len);

        use dash_sdk::dpp::serialization::PlatformDeserializableWithPotentialValidationFromVersionedStructure;

        DataContract::versioned_deserialize(
            contract_data,
            false, // skip validation since it's already validated
            sdk.version(),
        )
        .map_err(|e| FFIError::InternalError(format!("Failed to deserialize contract: {}", e)))
    } else {
        Err(FFIError::InternalError(
            "Either token_contract_id or serialized_contract must be provided".to_string(),
        ))
    }
}

/// Extract user fee increase from put_settings or use default
pub unsafe fn extract_user_fee_increase(
    put_settings: *const DashSDKPutSettings,
) -> UserFeeIncrease {
    if put_settings.is_null() {
        0
    } else {
        (*put_settings).user_fee_increase
    }
}

/// Validate that either contract ID or serialized contract is provided (but not both)
pub unsafe fn validate_contract_params(
    token_contract_id: *const c_char,
    serialized_contract: *const u8,
    serialized_contract_len: usize,
) -> Result<bool, FFIError> {
    let has_contract_id = !token_contract_id.is_null();
    let has_serialized_contract = !serialized_contract.is_null() && serialized_contract_len > 0;

    if !has_contract_id && !has_serialized_contract {
        return Err(FFIError::InternalError(
            "Either token contract ID or serialized contract must be provided".to_string(),
        ));
    }

    if has_contract_id && has_serialized_contract {
        return Err(FFIError::InternalError(
            "Cannot provide both token contract ID and serialized contract".to_string(),
        ));
    }

    Ok(has_serialized_contract)
}

/// Parse optional public note from C string
pub unsafe fn parse_optional_note(note_ptr: *const c_char) -> Result<Option<String>, FFIError> {
    if note_ptr.is_null() {
        Ok(None)
    } else {
        match CStr::from_ptr(note_ptr).to_str() {
            Ok(s) => Ok(Some(s.to_string())),
            Err(e) => Err(FFIError::from(e)),
        }
    }
}

/// Parse recipient ID from C string
pub unsafe fn parse_recipient_id(recipient_id_ptr: *const c_char) -> Result<Identifier, FFIError> {
    let recipient_id_str = CStr::from_ptr(recipient_id_ptr)
        .to_str()
        .map_err(FFIError::from)?;

    Identifier::from_string(recipient_id_str, Encoding::Base58)
        .map_err(|e| FFIError::InternalError(format!("Invalid recipient ID: {}", e)))
}

/// Parse identifier from raw bytes (32 bytes)
pub unsafe fn parse_identifier_from_bytes(id_bytes: *const u8) -> Result<Identifier, FFIError> {
    if id_bytes.is_null() {
        return Err(FFIError::InternalError(
            "Identifier bytes cannot be null".to_string(),
        ));
    }

    let id_slice = std::slice::from_raw_parts(id_bytes, 32);
    Identifier::from_bytes(id_slice)
        .map_err(|e| FFIError::InternalError(format!("Invalid identifier: {}", e)))
}
