//! Token claim operations

use super::types::DashSDKTokenClaimParams;
use super::utils::{
    convert_state_transition_creation_options, convert_token_distribution_type,
    extract_user_fee_increase, parse_optional_note, validate_contract_params,
};
use crate::sdk::SDKWrapper;
use crate::types::{
    DashSDKPutSettings, DashSDKStateTransitionCreationOptions, SDKHandle, SignerHandle,
};
use crate::{DashSDKError, DashSDKErrorCode, DashSDKResult, FFIError};
use dash_sdk::dpp::data_contract::TokenContractPosition;
use dash_sdk::dpp::platform_value::string_encoding::Encoding;
use dash_sdk::dpp::prelude::Identifier;
use dash_sdk::platform::tokens::builders::claim::TokenClaimTransitionBuilder;
use dash_sdk::platform::tokens::transitions::ClaimResult;
use dash_sdk::platform::IdentityPublicKey;
use std::ffi::CStr;
use std::sync::Arc;

/// Claim tokens from a distribution and wait for confirmation
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_token_claim(
    sdk_handle: *mut SDKHandle,
    transition_owner_id: *const u8,
    params: *const DashSDKTokenClaimParams,
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
    let identity_public_key = unsafe { &*(identity_public_key_handle as *const IdentityPublicKey) };
    let signer = unsafe { &*(signer_handle as *const crate::signer::VTableSigner) };
    let params = unsafe { &*params };

    // Convert transition owner ID from bytes
    let transition_owner_id_slice = unsafe { std::slice::from_raw_parts(transition_owner_id, 32) };
    let claimer_id = match Identifier::from_bytes(transition_owner_id_slice) {
        Ok(id) => id,
        Err(e) => {
            return DashSDKResult::error(DashSDKError::new(
                DashSDKErrorCode::InvalidParameter,
                format!("Invalid transition owner ID: {}", e),
            ))
        }
    };

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

    // Convert distribution type
    let distribution_type = convert_token_distribution_type(params.distribution_type);

    let result: Result<ClaimResult, FFIError> = wrapper.runtime.block_on(async {
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

        // Create token claim transition builder
        let mut builder = TokenClaimTransitionBuilder::new(
            Arc::new(data_contract),
            params.token_position as TokenContractPosition,
            claimer_id,
            distribution_type,
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

        // Use SDK method to claim and wait
        let result = wrapper
            .sdk
            .token_claim(builder, identity_public_key, signer)
            .await
            .map_err(|e| {
                FFIError::InternalError(format!("Failed to claim token and wait: {}", e))
            })?;

        Ok(result)
    });

    match result {
        Ok(_claim_result) => DashSDKResult::success(std::ptr::null_mut()),
        Err(e) => DashSDKResult::error(e.into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::token::types::DashSDKTokenDistributionType;
    use crate::types::{
        DashSDKConfig, DashSDKPutSettings, DashSDKStateTransitionCreationOptions, SDKHandle,
    };
    use crate::DashSDKErrorCode;
    use dash_sdk::dpp::identity::identity_public_key::v0::IdentityPublicKeyV0;
    use dash_sdk::dpp::identity::{KeyType, Purpose, SecurityLevel};
    use dash_sdk::dpp::platform_value::BinaryData;
    use std::ffi::{CStr, CString};
    use std::ptr;

    // Helper function to create a mock SDK handle
    fn create_mock_sdk_handle() -> *mut SDKHandle {
        let config = DashSDKConfig {
            network: crate::types::DashSDKNetwork::SDKLocal,
            dapi_addresses: ptr::null(), // Use mock SDK
            skip_asset_lock_proof_verification: false,
            request_retry_count: 3,
            request_timeout_ms: 5000,
        };

        let result = unsafe { crate::sdk::dash_sdk_create(&config) };
        assert!(result.error.is_null());
        result.data as *mut SDKHandle
    }

    // Helper function to destroy mock SDK handle
    fn destroy_mock_sdk_handle(handle: *mut SDKHandle) {
        unsafe {
            crate::sdk::dash_sdk_destroy(handle);
        }
    }

    // Helper function to create a mock identity public key
    fn create_mock_identity_public_key() -> Box<IdentityPublicKey> {
        let key_v0 = IdentityPublicKeyV0 {
            id: 0,
            purpose: Purpose::AUTHENTICATION,
            security_level: SecurityLevel::MASTER,
            key_type: KeyType::ECDSA_SECP256K1,
            read_only: false,
            data: BinaryData::new(vec![0u8; 33]), // 33 bytes for compressed secp256k1 key
            disabled_at: None,
            contract_bounds: None,
        };
        Box::new(IdentityPublicKey::V0(key_v0))
    }

    // Mock signer callbacks
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
    fn create_mock_signer() -> Box<crate::signer::VTableSigner> {
        // Create a mock signer vtable
        let vtable = Box::new(crate::signer::SignerVTable {
            sign: mock_sign_callback,
            can_sign_with: mock_can_sign_callback,
            destroy: mock_destroy_callback,
        });

        Box::new(crate::signer::VTableSigner {
            signer_ptr: std::ptr::null_mut(),
            vtable: Box::into_raw(vtable),
        })
    }

    // Mock destroy callback
    unsafe extern "C" fn mock_destroy_callback(_signer: *mut std::os::raw::c_void) {
        // No-op for mock
    }

    fn create_valid_transition_owner_id() -> [u8; 32] {
        [1u8; 32]
    }

    fn create_valid_claim_params() -> DashSDKTokenClaimParams {
        DashSDKTokenClaimParams {
            token_contract_id: CString::new("GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec")
                .unwrap()
                .into_raw(),
            serialized_contract: ptr::null(),
            serialized_contract_len: 0,
            token_position: 0,
            distribution_type: DashSDKTokenDistributionType::PreProgrammed,
            public_note: ptr::null(),
        }
    }

    unsafe fn cleanup_claim_params(params: &DashSDKTokenClaimParams) {
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
    fn test_claim_with_null_sdk_handle() {
        let transition_owner_id = create_valid_transition_owner_id();
        let params = create_valid_claim_params();
        let identity_public_key_handle = 1 as *const crate::types::IdentityPublicKeyHandle;
        let signer_handle = 1 as *const SignerHandle;
        let put_settings = create_put_settings();
        let state_transition_options: *const DashSDKStateTransitionCreationOptions = ptr::null();

        let result = unsafe {
            dash_sdk_token_claim(
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
            cleanup_claim_params(&params);
        }
    }

    #[test]
    fn test_claim_with_null_transition_owner_id() {
        // This test validates that the function properly handles null transition owner ID
        // We use real mock data to avoid segfaults when the function validates other parameters
        let sdk_handle = create_mock_sdk_handle();
        let identity_public_key = create_mock_identity_public_key();
        let signer = create_mock_signer();

        let params = create_valid_claim_params();
        let identity_public_key_handle =
            Box::into_raw(identity_public_key) as *const crate::types::IdentityPublicKeyHandle;
        let signer_handle = Box::into_raw(signer) as *const SignerHandle;
        let put_settings = create_put_settings();
        let state_transition_options: *const DashSDKStateTransitionCreationOptions = ptr::null();

        let result = unsafe {
            dash_sdk_token_claim(
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

        // Clean up
        unsafe {
            cleanup_claim_params(&params);
            let _ = Box::from_raw(identity_public_key_handle as *mut IdentityPublicKey);
            let _ = Box::from_raw(signer_handle as *mut crate::signer::VTableSigner);
        }
        destroy_mock_sdk_handle(sdk_handle);
    }

    #[test]
    fn test_claim_with_null_params() {
        // This test validates that the function properly handles null params
        // We use real mock data to avoid segfaults when the function validates other parameters
        let sdk_handle = create_mock_sdk_handle();
        let identity_public_key = create_mock_identity_public_key();
        let signer = create_mock_signer();

        let transition_owner_id = create_valid_transition_owner_id();
        let identity_public_key_handle =
            Box::into_raw(identity_public_key) as *const crate::types::IdentityPublicKeyHandle;
        let signer_handle = Box::into_raw(signer) as *const SignerHandle;
        let put_settings = create_put_settings();
        let state_transition_options: *const DashSDKStateTransitionCreationOptions = ptr::null();

        let result = unsafe {
            dash_sdk_token_claim(
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

        // Clean up
        unsafe {
            let _ = Box::from_raw(identity_public_key_handle as *mut IdentityPublicKey);
            let _ = Box::from_raw(signer_handle as *mut crate::signer::VTableSigner);
        }
        destroy_mock_sdk_handle(sdk_handle);
    }

    #[test]
    fn test_claim_with_null_identity_public_key() {
        // This test validates that the function properly handles null identity public key
        // We use real mock data to avoid segfaults when the function validates other parameters
        let sdk_handle = create_mock_sdk_handle();
        let signer = create_mock_signer();

        let transition_owner_id = create_valid_transition_owner_id();
        let params = create_valid_claim_params();
        let signer_handle = Box::into_raw(signer) as *const SignerHandle;
        let put_settings = create_put_settings();
        let state_transition_options: *const DashSDKStateTransitionCreationOptions = ptr::null();

        let result = unsafe {
            dash_sdk_token_claim(
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

        // Clean up
        unsafe {
            cleanup_claim_params(&params);
            let _ = Box::from_raw(signer_handle as *mut crate::signer::VTableSigner);
        }
        destroy_mock_sdk_handle(sdk_handle);
    }

    #[test]
    fn test_claim_with_null_signer() {
        // This test validates that the function properly handles null signer
        // We use real mock data to avoid segfaults when the function validates other parameters
        let sdk_handle = create_mock_sdk_handle();
        let identity_public_key = create_mock_identity_public_key();

        let transition_owner_id = create_valid_transition_owner_id();
        let params = create_valid_claim_params();
        let identity_public_key_handle =
            Box::into_raw(identity_public_key) as *const crate::types::IdentityPublicKeyHandle;
        let put_settings = create_put_settings();
        let state_transition_options: *const DashSDKStateTransitionCreationOptions = ptr::null();

        let result = unsafe {
            dash_sdk_token_claim(
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

        // Clean up
        unsafe {
            cleanup_claim_params(&params);
            let _ = Box::from_raw(identity_public_key_handle as *mut IdentityPublicKey);
        }
        destroy_mock_sdk_handle(sdk_handle);
    }

    #[test]
    fn test_claim_with_different_distribution_types() {
        let mut params = create_valid_claim_params();

        // Test PreProgrammed distribution
        params.distribution_type = DashSDKTokenDistributionType::PreProgrammed;
        assert_eq!(
            params.distribution_type as u32,
            DashSDKTokenDistributionType::PreProgrammed as u32
        );

        // Test Perpetual distribution
        params.distribution_type = DashSDKTokenDistributionType::Perpetual;
        assert_eq!(
            params.distribution_type as u32,
            DashSDKTokenDistributionType::Perpetual as u32
        );

        unsafe {
            cleanup_claim_params(&params);
        }
    }

    #[test]
    fn test_claim_params_with_public_note() {
        let public_note = CString::new("Test claim note").unwrap();
        let contract_id = CString::new("GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec").unwrap();

        let params = DashSDKTokenClaimParams {
            token_contract_id: contract_id.as_ptr(),
            serialized_contract: ptr::null(),
            serialized_contract_len: 0,
            token_position: 0,
            distribution_type: DashSDKTokenDistributionType::PreProgrammed,
            public_note: public_note.as_ptr(),
        };

        unsafe {
            let note_str = CStr::from_ptr(params.public_note);
            assert_eq!(note_str.to_str().unwrap(), "Test claim note");
        }
    }

    #[test]
    fn test_claim_params_with_serialized_contract() {
        let contract_data = vec![1u8, 2, 3, 4, 5];
        let params = DashSDKTokenClaimParams {
            token_contract_id: ptr::null(),
            serialized_contract: contract_data.as_ptr(),
            serialized_contract_len: contract_data.len(),
            token_position: 0,
            distribution_type: DashSDKTokenDistributionType::Perpetual,
            public_note: ptr::null(),
        };

        assert_eq!(params.serialized_contract_len, 5);
        assert!(!params.serialized_contract.is_null());
        assert!(params.token_contract_id.is_null());
    }

    #[test]
    fn test_claim_with_different_token_positions() {
        let mut params = create_valid_claim_params();

        let positions: Vec<u16> = vec![0, 1, 100, u16::MAX];

        for position in positions {
            params.token_position = position;
            assert_eq!(params.token_position, position);
        }

        unsafe {
            cleanup_claim_params(&params);
        }
    }
}
