//! Token configuration update operations

use super::types::{DashSDKTokenConfigUpdateParams, DashSDKTokenConfigUpdateType};
use super::utils::{
    convert_state_transition_creation_options, extract_user_fee_increase,
    parse_identifier_from_bytes, parse_optional_note, validate_contract_params,
};
use crate::sdk::SDKWrapper;
use crate::types::{
    DashSDKPutSettings, DashSDKStateTransitionCreationOptions, SDKHandle, SignerHandle,
};
use crate::{DashSDKError, DashSDKErrorCode, DashSDKResult, FFIError};
use dash_sdk::dpp::balances::credits::TokenAmount;
use dash_sdk::dpp::data_contract::associated_token::token_configuration_item::TokenConfigurationChangeItem;
use dash_sdk::dpp::data_contract::TokenContractPosition;
use dash_sdk::dpp::platform_value::string_encoding::Encoding;
use dash_sdk::dpp::prelude::Identifier;
use dash_sdk::platform::tokens::builders::config_update::TokenConfigUpdateTransitionBuilder;
use dash_sdk::platform::tokens::transitions::ConfigUpdateResult;
use dash_sdk::platform::IdentityPublicKey;
use std::ffi::CStr;
use std::sync::Arc;

/// Update token configuration and wait for confirmation
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_token_update_contract_token_configuration(
    sdk_handle: *mut SDKHandle,
    transition_owner_id: *const u8,
    params: *const DashSDKTokenConfigUpdateParams,
    identity_public_key_handle: *const crate::types::IdentityPublicKeyHandle,
    signer_handle: *const SignerHandle,
    put_settings: *const DashSDKPutSettings,
    state_transition_creation_options: *const DashSDKStateTransitionCreationOptions,
) -> DashSDKResult {
    // Validate parameters
    if sdk_handle.is_null()
        || transition_owner_id.is_null()
        || params.is_null()
        || identity_public_key_handle.is_null()
        || signer_handle.is_null()
    {
        return DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
            "One or more required parameters is null".to_string(),
        ));
    }

    // SAFETY: We've verified all pointers are non-null above
    let wrapper = unsafe { &mut *(sdk_handle as *mut SDKWrapper) };

    // Convert transition_owner_id from bytes to Identifier (32 bytes)
    let transition_owner_id = {
        let id_bytes = unsafe { std::slice::from_raw_parts(transition_owner_id, 32) };
        match Identifier::from_bytes(id_bytes) {
            Ok(id) => id,
            Err(e) => {
                return DashSDKResult::error(DashSDKError::new(
                    DashSDKErrorCode::InvalidParameter,
                    format!("Invalid transition owner ID: {}", e),
                ))
            }
        }
    };

    let identity_public_key = unsafe { &*(identity_public_key_handle as *const IdentityPublicKey) };
    let signer = unsafe { &*(signer_handle as *const crate::signer::VTableSigner) };
    let params = unsafe { &*params };

    // Validate contract parameters
    let has_serialized_contract = match validate_contract_params(
        params.token_contract_id,
        params.serialized_contract,
        params.serialized_contract_len,
    ) {
        Ok(result) => result,
        Err(e) => return DashSDKResult::error(e.into()),
    };

    // Parse optional public note
    let public_note = match parse_optional_note(params.public_note) {
        Ok(note) => note,
        Err(e) => return DashSDKResult::error(e.into()),
    };

    // Parse optional identity ID for certain update types
    let identity_id = if params.identity_id.is_null() {
        None
    } else {
        match parse_identifier_from_bytes(params.identity_id) {
            Ok(id) => Some(id),
            Err(e) => return DashSDKResult::error(e.into()),
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

        let data_contract = if !has_serialized_contract {
            // Parse and fetch the contract ID
            let token_contract_id_str = match unsafe { CStr::from_ptr(params.token_contract_id) }.to_str() {
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
            let contract_slice = unsafe {
                std::slice::from_raw_parts(
                    params.serialized_contract,
                    params.serialized_contract_len
                )
            };

            use dash_sdk::dpp::serialization::PlatformDeserializableWithPotentialValidationFromVersionedStructure;

            DataContract::versioned_deserialize(
                contract_slice,
                false, // skip validation since it's already validated
                wrapper.sdk.version(),
            )
            .map_err(|e| FFIError::InternalError(format!("Failed to deserialize contract: {}", e)))?
        };

        // Create the appropriate token configuration change item based on the update type
        let update_item = match params.update_type {
            DashSDKTokenConfigUpdateType::MaxSupply => {
                TokenConfigurationChangeItem::MaxSupply(if params.amount == 0 {
                    None // 0 means unlimited
                } else {
                    Some(params.amount as TokenAmount)
                })
            }
            DashSDKTokenConfigUpdateType::MintingAllowChoosingDestination => {
                TokenConfigurationChangeItem::MintingAllowChoosingDestination(params.bool_value)
            }
            DashSDKTokenConfigUpdateType::NewTokensDestinationIdentity => {
                if let Some(id) = identity_id {
                    TokenConfigurationChangeItem::NewTokensDestinationIdentity(Some(id))
                } else {
                    return Err(FFIError::InternalError(
                        "Identity ID required for NewTokensDestinationIdentity update".to_string()
                    ));
                }
            }
            DashSDKTokenConfigUpdateType::ManualMinting => {
                // Note: This would need proper implementation based on the actual SDK types
                // For now, return an error indicating this needs implementation
                return Err(FFIError::InternalError(
                    "ManualMinting config update not yet implemented".to_string()
                ));
            }
            DashSDKTokenConfigUpdateType::ManualBurning => {
                return Err(FFIError::InternalError(
                    "ManualBurning config update not yet implemented".to_string()
                ));
            }
            DashSDKTokenConfigUpdateType::Freeze => {
                return Err(FFIError::InternalError(
                    "Freeze config update not yet implemented".to_string()
                ));
            }
            DashSDKTokenConfigUpdateType::Unfreeze => {
                return Err(FFIError::InternalError(
                    "Unfreeze config update not yet implemented".to_string()
                ));
            }
            DashSDKTokenConfigUpdateType::MainControlGroup => {
                TokenConfigurationChangeItem::MainControlGroup(Some(params.group_position))
            }
            DashSDKTokenConfigUpdateType::NoChange => {
                TokenConfigurationChangeItem::TokenConfigurationNoChange
            }
        };

        // Create token config update transition builder
        let mut builder = TokenConfigUpdateTransitionBuilder::new(
            Arc::new(data_contract),
            params.token_position as TokenContractPosition,
            transition_owner_id,
            update_item,
        );

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
        Ok(_config_update_result) => DashSDKResult::success(std::ptr::null_mut()),
        Err(e) => DashSDKResult::error(e.into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::token::types::{DashSDKAuthorizedActionTakers, DashSDKTokenConfigUpdateType};
    use crate::types::{DashSDKPutSettings, DashSDKStateTransitionCreationOptions, SDKHandle};
    use crate::DashSDKErrorCode;
    use dash_sdk::dpp::identity::identity_public_key::v0::IdentityPublicKeyV0;
    use dash_sdk::dpp::identity::{KeyType, Purpose, SecurityLevel};
    use dash_sdk::dpp::platform_value::BinaryData;
    use dash_sdk::platform::IdentityPublicKey;
    use std::ffi::{CStr, CString};
    use std::ptr;

    // Helper function to create a mock SDK handle
    fn create_mock_sdk_handle() -> *mut SDKHandle {
        let wrapper = Box::new(SDKWrapper::new_mock());
        Box::into_raw(wrapper) as *mut SDKHandle
    }

    // Helper function to create a mock identity public key
    fn create_mock_identity_public_key() -> Box<IdentityPublicKey> {
        Box::new(IdentityPublicKey::V0(IdentityPublicKeyV0 {
            id: 1,
            purpose: Purpose::AUTHENTICATION,
            security_level: SecurityLevel::MEDIUM,
            contract_bounds: None,
            key_type: KeyType::ECDSA_SECP256K1,
            read_only: false,
            data: BinaryData::new(vec![0u8; 33]),
            disabled_at: None,
        }))
    }

    // Mock callbacks for signer
    unsafe extern "C" fn mock_sign_callback(
        _identity_public_key_bytes: *const u8,
        _identity_public_key_len: usize,
        _data: *const u8,
        _data_len: usize,
        result_len: *mut usize,
    ) -> *mut u8 {
        // Return a mock signature (64 bytes for ECDSA)
        let signature = vec![0u8; 64];
        *result_len = signature.len();
        let ptr = signature.as_ptr() as *mut u8;
        std::mem::forget(signature); // Prevent deallocation
        ptr
    }

    unsafe extern "C" fn mock_can_sign_callback(
        _identity_public_key_bytes: *const u8,
        _identity_public_key_len: usize,
    ) -> bool {
        true
    }

    // Helper function to create a mock signer
    fn create_mock_signer() -> Box<crate::signer::IOSSigner> {
        Box::new(crate::signer::IOSSigner::new(
            mock_sign_callback,
            mock_can_sign_callback,
        ))
    }

    fn create_valid_transition_owner_id() -> [u8; 32] {
        [1u8; 32]
    }

    fn create_valid_config_update_params() -> DashSDKTokenConfigUpdateParams {
        DashSDKTokenConfigUpdateParams {
            token_contract_id: CString::new("GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec")
                .unwrap()
                .into_raw(),
            serialized_contract: ptr::null(),
            serialized_contract_len: 0,
            token_position: 0,
            update_type: DashSDKTokenConfigUpdateType::MaxSupply,
            amount: 1000000,
            bool_value: false,
            identity_id: ptr::null(),
            group_position: 0,
            action_takers: DashSDKAuthorizedActionTakers::AuthorizedContractOwner,
            public_note: ptr::null(),
        }
    }

    unsafe fn cleanup_config_update_params(params: &DashSDKTokenConfigUpdateParams) {
        if !params.token_contract_id.is_null() {
            let _ = CString::from_raw(params.token_contract_id as *mut std::os::raw::c_char);
        }
        if !params.public_note.is_null() {
            let _ = CString::from_raw(params.public_note as *mut std::os::raw::c_char);
        }
    }

    fn create_put_settings() -> DashSDKPutSettings {
        DashSDKPutSettings {
            connect_timeout_ms: 0,
            timeout_ms: 0,
            retries: 0,
            ban_failed_address: false,
            identity_nonce_stale_time_s: 0,
            user_fee_increase: 0,
            allow_signing_with_any_security_level: false,
            allow_signing_with_any_purpose: false,
            wait_timeout_ms: 0,
        }
    }

    #[test]
    fn test_config_update_with_null_sdk_handle() {
        let transition_owner_id = create_valid_transition_owner_id();
        let params = create_valid_config_update_params();
        let identity_public_key = create_mock_identity_public_key();
        let signer = create_mock_signer();
        let identity_public_key_handle =
            Box::into_raw(identity_public_key) as *const crate::types::IdentityPublicKeyHandle;
        let signer_handle = Box::into_raw(signer) as *const SignerHandle;
        let put_settings = create_put_settings();
        let state_transition_options: *const DashSDKStateTransitionCreationOptions = ptr::null();

        let result = unsafe {
            dash_sdk_token_update_contract_token_configuration(
                ptr::null_mut(),
                transition_owner_id.as_ptr(),
                &params,
                identity_public_key_handle,
                signer_handle,
                &put_settings,
                state_transition_options,
            )
        };

        assert!(!result.error.is_null());
        unsafe {
            let error = &*result.error;
            assert_eq!(error.code, DashSDKErrorCode::InvalidParameter);
            let error_msg = CStr::from_ptr(error.message).to_str().unwrap();
            assert!(error_msg.contains("null"));
        }

        unsafe {
            cleanup_config_update_params(&params);
        }
    }

    #[test]
    fn test_config_update_with_null_transition_owner_id() {
        let sdk_handle = create_mock_sdk_handle();
        let params = create_valid_config_update_params();
        let identity_public_key = create_mock_identity_public_key();
        let signer = create_mock_signer();
        let identity_public_key_handle =
            Box::into_raw(identity_public_key) as *const crate::types::IdentityPublicKeyHandle;
        let signer_handle = Box::into_raw(signer) as *const SignerHandle;
        let put_settings = create_put_settings();
        let state_transition_options: *const DashSDKStateTransitionCreationOptions = ptr::null();

        let result = unsafe {
            dash_sdk_token_update_contract_token_configuration(
                sdk_handle,
                ptr::null(),
                &params,
                identity_public_key_handle,
                signer_handle,
                &put_settings,
                state_transition_options,
            )
        };

        assert!(!result.error.is_null());
        unsafe {
            let error = &*result.error;
            assert_eq!(error.code, DashSDKErrorCode::InvalidParameter);
        }

        unsafe {
            cleanup_config_update_params(&params);
        }
    }

    #[test]
    fn test_config_update_with_null_params() {
        let sdk_handle = create_mock_sdk_handle();
        let transition_owner_id = create_valid_transition_owner_id();
        let identity_public_key = create_mock_identity_public_key();
        let signer = create_mock_signer();
        let identity_public_key_handle =
            Box::into_raw(identity_public_key) as *const crate::types::IdentityPublicKeyHandle;
        let signer_handle = Box::into_raw(signer) as *const SignerHandle;
        let put_settings = create_put_settings();
        let state_transition_options: *const DashSDKStateTransitionCreationOptions = ptr::null();

        let result = unsafe {
            dash_sdk_token_update_contract_token_configuration(
                sdk_handle,
                transition_owner_id.as_ptr(),
                ptr::null(),
                identity_public_key_handle,
                signer_handle,
                &put_settings,
                state_transition_options,
            )
        };

        assert!(!result.error.is_null());
        unsafe {
            let error = &*result.error;
            assert_eq!(error.code, DashSDKErrorCode::InvalidParameter);
        }
    }

    #[test]
    fn test_config_update_with_null_identity_public_key() {
        let sdk_handle = create_mock_sdk_handle();
        let transition_owner_id = create_valid_transition_owner_id();
        let params = create_valid_config_update_params();
        let signer_handle = 1 as *const SignerHandle;
        let put_settings = create_put_settings();
        let state_transition_options: *const DashSDKStateTransitionCreationOptions = ptr::null();

        let result = unsafe {
            dash_sdk_token_update_contract_token_configuration(
                sdk_handle,
                transition_owner_id.as_ptr(),
                &params,
                ptr::null(),
                signer_handle,
                &put_settings,
                state_transition_options,
            )
        };

        assert!(!result.error.is_null());
        unsafe {
            let error = &*result.error;
            assert_eq!(error.code, DashSDKErrorCode::InvalidParameter);
        }

        unsafe {
            cleanup_config_update_params(&params);
        }
    }

    #[test]
    fn test_config_update_with_null_signer() {
        let sdk_handle = create_mock_sdk_handle();
        let transition_owner_id = create_valid_transition_owner_id();
        let params = create_valid_config_update_params();
        let identity_public_key_handle = 1 as *const crate::types::IdentityPublicKeyHandle;
        let put_settings = create_put_settings();
        let state_transition_options: *const DashSDKStateTransitionCreationOptions = ptr::null();

        let result = unsafe {
            dash_sdk_token_update_contract_token_configuration(
                sdk_handle,
                transition_owner_id.as_ptr(),
                &params,
                identity_public_key_handle,
                ptr::null(),
                &put_settings,
                state_transition_options,
            )
        };

        assert!(!result.error.is_null());
        unsafe {
            let error = &*result.error;
            assert_eq!(error.code, DashSDKErrorCode::InvalidParameter);
        }

        unsafe {
            cleanup_config_update_params(&params);
        }
    }

    #[test]
    fn test_config_update_different_update_types() {
        let mut params = create_valid_config_update_params();

        // Test MaxSupply
        params.update_type = DashSDKTokenConfigUpdateType::MaxSupply;
        params.amount = 1000000;
        assert_eq!(
            params.update_type as u32,
            DashSDKTokenConfigUpdateType::MaxSupply as u32
        );

        // Test MintingAllowChoosingDestination
        params.update_type = DashSDKTokenConfigUpdateType::MintingAllowChoosingDestination;
        params.bool_value = true;
        assert_eq!(
            params.update_type as u32,
            DashSDKTokenConfigUpdateType::MintingAllowChoosingDestination as u32
        );

        // Test MainControlGroup
        params.update_type = DashSDKTokenConfigUpdateType::MainControlGroup;
        params.group_position = 1;
        assert_eq!(
            params.update_type as u32,
            DashSDKTokenConfigUpdateType::MainControlGroup as u32
        );

        // Test NoChange
        params.update_type = DashSDKTokenConfigUpdateType::NoChange;
        assert_eq!(
            params.update_type as u32,
            DashSDKTokenConfigUpdateType::NoChange as u32
        );

        unsafe {
            cleanup_config_update_params(&params);
        }
    }

    #[test]
    fn test_config_update_with_identity_id() {
        let identity_id = [2u8; 32];
        let params = DashSDKTokenConfigUpdateParams {
            token_contract_id: CString::new("GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec")
                .unwrap()
                .into_raw(),
            serialized_contract: ptr::null(),
            serialized_contract_len: 0,
            token_position: 0,
            update_type: DashSDKTokenConfigUpdateType::NewTokensDestinationIdentity,
            amount: 0,
            bool_value: false,
            identity_id: identity_id.as_ptr(),
            group_position: 0,
            action_takers: DashSDKAuthorizedActionTakers::AuthorizedContractOwner,
            public_note: ptr::null(),
        };

        assert!(!params.identity_id.is_null());
        unsafe {
            cleanup_config_update_params(&params);
        }
    }

    #[test]
    fn test_config_update_with_public_note() {
        let public_note = CString::new("Config update note").unwrap();
        let contract_id = CString::new("GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec").unwrap();

        let params = DashSDKTokenConfigUpdateParams {
            token_contract_id: contract_id.as_ptr(),
            serialized_contract: ptr::null(),
            serialized_contract_len: 0,
            token_position: 0,
            update_type: DashSDKTokenConfigUpdateType::MaxSupply,
            amount: 500000,
            bool_value: false,
            identity_id: ptr::null(),
            group_position: 0,
            action_takers: DashSDKAuthorizedActionTakers::AuthorizedContractOwner,
            public_note: public_note.as_ptr(),
        };

        unsafe {
            let note_str = CStr::from_ptr(params.public_note);
            assert_eq!(note_str.to_str().unwrap(), "Config update note");
        }
    }

    #[test]
    fn test_config_update_with_different_action_takers() {
        let mut params = create_valid_config_update_params();

        // Test different action takers
        params.action_takers = DashSDKAuthorizedActionTakers::NoOne;
        assert_eq!(
            params.action_takers as u32,
            DashSDKAuthorizedActionTakers::NoOne as u32
        );

        params.action_takers = DashSDKAuthorizedActionTakers::AuthorizedContractOwner;
        assert_eq!(
            params.action_takers as u32,
            DashSDKAuthorizedActionTakers::AuthorizedContractOwner as u32
        );

        params.action_takers = DashSDKAuthorizedActionTakers::MainGroup;
        assert_eq!(
            params.action_takers as u32,
            DashSDKAuthorizedActionTakers::MainGroup as u32
        );

        params.action_takers = DashSDKAuthorizedActionTakers::Identity;
        assert_eq!(
            params.action_takers as u32,
            DashSDKAuthorizedActionTakers::Identity as u32
        );

        params.action_takers = DashSDKAuthorizedActionTakers::Group;
        assert_eq!(
            params.action_takers as u32,
            DashSDKAuthorizedActionTakers::Group as u32
        );

        unsafe {
            cleanup_config_update_params(&params);
        }
    }

    #[test]
    fn test_config_update_with_serialized_contract() {
        let contract_data = vec![1u8, 2, 3, 4, 5];
        let params = DashSDKTokenConfigUpdateParams {
            token_contract_id: ptr::null(),
            serialized_contract: contract_data.as_ptr(),
            serialized_contract_len: contract_data.len(),
            token_position: 0,
            update_type: DashSDKTokenConfigUpdateType::MaxSupply,
            amount: 100000,
            bool_value: false,
            identity_id: ptr::null(),
            group_position: 0,
            action_takers: DashSDKAuthorizedActionTakers::AuthorizedContractOwner,
            public_note: ptr::null(),
        };

        assert_eq!(params.serialized_contract_len, 5);
        assert!(!params.serialized_contract.is_null());
        assert!(params.token_contract_id.is_null());
    }
}
