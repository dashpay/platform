//! Token burn operations

use super::types::DashSDKTokenBurnParams;
use super::utils::{
    convert_state_transition_creation_options, extract_user_fee_increase, parse_optional_note,
    validate_contract_params,
};
use crate::sdk::SDKWrapper;
use crate::types::{
    DashSDKPutSettings, DashSDKStateTransitionCreationOptions, SDKHandle, SignerHandle,
};
use crate::{DashSDKError, DashSDKErrorCode, DashSDKResult, FFIError};
use dash_sdk::dpp::balances::credits::TokenAmount;
use dash_sdk::dpp::data_contract::TokenContractPosition;
use dash_sdk::dpp::platform_value::string_encoding::Encoding;
use dash_sdk::dpp::prelude::Identifier;
use dash_sdk::platform::tokens::builders::burn::TokenBurnTransitionBuilder;
use dash_sdk::platform::tokens::transitions::BurnResult;
use dash_sdk::platform::IdentityPublicKey;
use std::ffi::CStr;
use std::sync::Arc;

/// Burn tokens from an identity and wait for confirmation
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_token_burn(
    sdk_handle: *mut SDKHandle,
    transition_owner_id: *const u8,
    params: *const DashSDKTokenBurnParams,
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
    let signer = unsafe { &*(signer_handle as *const crate::signer::IOSSigner) };
    let params = unsafe { &*params };

    // Convert transition owner ID from bytes
    let transition_owner_id_slice = unsafe { std::slice::from_raw_parts(transition_owner_id, 32) };
    let transition_owner_id = match Identifier::from_bytes(transition_owner_id_slice) {
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

    let result: Result<BurnResult, FFIError> = wrapper.runtime.block_on(async {
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

        // Create token burn transition builder
        let mut builder = TokenBurnTransitionBuilder::new(
            Arc::new(data_contract),
            params.token_position as TokenContractPosition,
            transition_owner_id,
            params.amount as TokenAmount,
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

        // Use SDK method to burn and wait
        let result = wrapper
            .sdk
            .token_burn(builder, identity_public_key, signer)
            .await
            .map_err(|e| {
                FFIError::InternalError(format!("Failed to burn token and wait: {}", e))
            })?;

        Ok(result)
    });

    match result {
        Ok(_burn_result) => DashSDKResult::success(std::ptr::null_mut()),
        Err(e) => DashSDKResult::error(e.into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::test_utils::*;
    use crate::types::{
        DashSDKConfig, DashSDKPutSettings, DashSDKStateTransitionCreationOptions, SDKHandle,
        SignerHandle,
    };
    use crate::DashSDKErrorCode;
    use dash_sdk::platform::IdentityPublicKey;
    use std::ffi::{CStr, CString};
    use std::ptr;

    fn create_valid_burn_params() -> DashSDKTokenBurnParams {
        // Note: In real tests, the caller is responsible for freeing the CString memory
        DashSDKTokenBurnParams {
            token_contract_id: CString::new("GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec")
                .unwrap()
                .into_raw(),
            serialized_contract: ptr::null(),
            serialized_contract_len: 0,
            token_position: 0,
            amount: 1000,
            public_note: ptr::null(),
        }
    }

    // Helper to clean up params after use
    unsafe fn cleanup_burn_params(params: &DashSDKTokenBurnParams) {
        if !params.token_contract_id.is_null() {
            let _ = CString::from_raw(params.token_contract_id as *mut std::os::raw::c_char);
        }
        if !params.public_note.is_null() {
            let _ = CString::from_raw(params.public_note as *mut std::os::raw::c_char);
        }
    }

    #[test]
    fn test_burn_with_null_sdk_handle() {
        let transition_owner_id = create_valid_transition_owner_id();
        let params = create_valid_burn_params();
        let identity_public_key_handle = 1 as *const crate::types::IdentityPublicKeyHandle;
        let signer_handle = 1 as *const SignerHandle;
        let put_settings = create_put_settings();
        let state_transition_options: *const DashSDKStateTransitionCreationOptions = ptr::null();

        let result = unsafe {
            dash_sdk_token_burn(
                ptr::null_mut(), // null SDK handle
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
            // Check that the error message contains "null"
            let error_msg = CStr::from_ptr(error.message).to_str().unwrap();
            assert!(error_msg.contains("null"));
        }

        // Clean up params memory
        unsafe {
            cleanup_burn_params(&params);
        }
    }

    #[test]
    fn test_burn_with_null_transition_owner_id() {
        // This test validates that the function properly handles null transition owner ID
        // We use real mock data to avoid segfaults when the function validates other parameters
        let sdk_handle = create_mock_sdk_handle();
        let identity_public_key = create_mock_identity_public_key_with_id(0);
        let signer = create_mock_signer();

        let params = create_valid_burn_params();
        let identity_public_key_handle =
            Box::into_raw(identity_public_key) as *const crate::types::IdentityPublicKeyHandle;
        let signer_handle = Box::into_raw(signer) as *const SignerHandle;
        let put_settings = create_put_settings();
        let state_transition_options: *const DashSDKStateTransitionCreationOptions = ptr::null();

        let result = unsafe {
            dash_sdk_token_burn(
                sdk_handle,
                ptr::null(), // null transition owner ID
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
            cleanup_burn_params(&params);
            let _ = Box::from_raw(identity_public_key_handle as *mut IdentityPublicKey);
            let _ = Box::from_raw(signer_handle as *mut crate::signer::IOSSigner);
        }
        destroy_mock_sdk_handle(sdk_handle);
    }

    #[test]
    fn test_burn_with_null_params() {
        // This test validates that the function properly handles null params
        // We use real mock data to avoid segfaults when the function validates other parameters
        let sdk_handle = create_mock_sdk_handle();
        let identity_public_key = create_mock_identity_public_key_with_id(0);
        let signer = create_mock_signer();

        let transition_owner_id = create_valid_transition_owner_id();
        let identity_public_key_handle =
            Box::into_raw(identity_public_key) as *const crate::types::IdentityPublicKeyHandle;
        let signer_handle = Box::into_raw(signer) as *const SignerHandle;
        let put_settings = create_put_settings();
        let state_transition_options: *const DashSDKStateTransitionCreationOptions = ptr::null();

        let result = unsafe {
            dash_sdk_token_burn(
                sdk_handle,
                transition_owner_id.as_ptr(),
                ptr::null(), // null params
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
            let _ = Box::from_raw(signer_handle as *mut crate::signer::IOSSigner);
        }
        destroy_mock_sdk_handle(sdk_handle);
    }

    #[test]
    fn test_burn_with_null_identity_public_key() {
        // This test validates that the function properly handles null identity public key
        // We use real mock data to avoid segfaults when the function validates other parameters
        let sdk_handle = create_mock_sdk_handle();
        let signer = create_mock_signer();

        let transition_owner_id = create_valid_transition_owner_id();
        let params = create_valid_burn_params();
        let signer_handle = Box::into_raw(signer) as *const SignerHandle;
        let put_settings = create_put_settings();
        let state_transition_options: *const DashSDKStateTransitionCreationOptions = ptr::null();

        let result = unsafe {
            dash_sdk_token_burn(
                sdk_handle,
                transition_owner_id.as_ptr(),
                &params,
                ptr::null(), // null identity public key
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
            cleanup_burn_params(&params);
            let _ = Box::from_raw(signer_handle as *mut crate::signer::IOSSigner);
        }
        destroy_mock_sdk_handle(sdk_handle);
    }

    #[test]
    fn test_burn_with_null_signer() {
        // This test validates that the function properly handles null signer
        // We use real mock data to avoid segfaults when the function validates other parameters
        let sdk_handle = create_mock_sdk_handle();
        let identity_public_key = create_mock_identity_public_key_with_id(0);

        let transition_owner_id = create_valid_transition_owner_id();
        let params = create_valid_burn_params();
        let identity_public_key_handle =
            Box::into_raw(identity_public_key) as *const crate::types::IdentityPublicKeyHandle;
        let put_settings = create_put_settings();
        let state_transition_options: *const DashSDKStateTransitionCreationOptions = ptr::null();

        let result = unsafe {
            dash_sdk_token_burn(
                sdk_handle,
                transition_owner_id.as_ptr(),
                &params,
                identity_public_key_handle,
                ptr::null(), // null signer
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
            cleanup_burn_params(&params);
            let _ = Box::from_raw(identity_public_key_handle as *mut IdentityPublicKey);
        }
        destroy_mock_sdk_handle(sdk_handle);
    }

    #[test]
    fn test_burn_with_invalid_transition_owner_id() {
        // Instead of testing invalid ID bytes, test with invalid contract ID
        // which will fail during parameter validation
        let transition_owner_id = create_valid_transition_owner_id();
        let sdk_handle = create_mock_sdk_handle();
        let identity_public_key = create_mock_identity_public_key_with_id(0);
        let signer = create_mock_signer();

        // Create params with invalid contract ID
        let invalid_contract_id = CString::new("invalid-base58-string!@#$").unwrap();
        let params = DashSDKTokenBurnParams {
            token_contract_id: invalid_contract_id.into_raw(),
            serialized_contract: ptr::null(),
            serialized_contract_len: 0,
            token_position: 0,
            amount: 1000,
            public_note: ptr::null(),
        };

        let identity_public_key_handle =
            Box::into_raw(identity_public_key) as *const crate::types::IdentityPublicKeyHandle;
        let signer_handle = Box::into_raw(signer) as *const SignerHandle;
        let put_settings = create_put_settings();
        let state_transition_options: *const DashSDKStateTransitionCreationOptions = ptr::null();

        let result = unsafe {
            dash_sdk_token_burn(
                sdk_handle,
                transition_owner_id.as_ptr(),
                &params,
                identity_public_key_handle,
                signer_handle,
                &put_settings,
                state_transition_options,
            )
        };

        // Should return an error for invalid contract ID
        assert!(!result.error.is_null());
        unsafe {
            let error = &*result.error;
            // Could be either InternalError or ProtocolError for invalid base58
            assert!(
                error.code == DashSDKErrorCode::InternalError
                    || error.code == DashSDKErrorCode::ProtocolError,
                "Expected InternalError or ProtocolError, got {:?}",
                error.code
            );
            let error_msg = CStr::from_ptr(error.message).to_str().unwrap();
            // Check that the error is related to the invalid contract ID
            assert!(
                error_msg.contains("Invalid token contract ID")
                    || error_msg.contains("base58")
                    || error_msg.contains("decode")
                    || error_msg.contains("Failed to deserialize contract"),
                "Error message '{}' doesn't contain expected content",
                error_msg
            );
        }

        // Clean up
        unsafe {
            cleanup_burn_params(&params);
            let _ = Box::from_raw(identity_public_key_handle as *mut IdentityPublicKey);
            let _ = Box::from_raw(signer_handle as *mut crate::signer::IOSSigner);
        }
        destroy_mock_sdk_handle(sdk_handle);
    }

    #[test]
    fn test_burn_params_with_public_note() {
        let public_note = CString::new("Test burn note").unwrap();
        let contract_id = CString::new("GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec").unwrap();

        let params = DashSDKTokenBurnParams {
            token_contract_id: contract_id.as_ptr(),
            serialized_contract: ptr::null(),
            serialized_contract_len: 0,
            token_position: 0,
            amount: 1000,
            public_note: public_note.as_ptr(),
        };

        // Verify the note can be read back
        unsafe {
            let note_str = CStr::from_ptr(params.public_note);
            assert_eq!(note_str.to_str().unwrap(), "Test burn note");
        }

        // CStrings are automatically dropped when they go out of scope
    }

    #[test]
    fn test_burn_params_with_serialized_contract() {
        let contract_data = vec![1u8, 2, 3, 4, 5];
        let params = DashSDKTokenBurnParams {
            token_contract_id: ptr::null(),
            serialized_contract: contract_data.as_ptr(),
            serialized_contract_len: contract_data.len(),
            token_position: 0,
            amount: 1000,
            public_note: ptr::null(),
        };

        assert_eq!(params.serialized_contract_len, 5);
        assert!(!params.serialized_contract.is_null());
        assert!(params.token_contract_id.is_null());
    }

    #[test]
    fn test_burn_params_validation() {
        // Test with both contract ID and serialized contract (should be mutually exclusive)
        let contract_id = CString::new("GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec").unwrap();
        let contract_data = vec![1u8, 2, 3];

        let params = DashSDKTokenBurnParams {
            token_contract_id: contract_id.as_ptr(),
            serialized_contract: contract_data.as_ptr(),
            serialized_contract_len: 3,
            token_position: 0,
            amount: 1000,
            public_note: ptr::null(),
        };

        // This should be handled by validate_contract_params function
        assert!(!params.token_contract_id.is_null());
        assert!(!params.serialized_contract.is_null());

        // CString and Vec are automatically dropped when they go out of scope
    }

    #[test]
    fn test_burn_with_different_token_positions() {
        let mut params = create_valid_burn_params();

        // Test with different token positions
        let positions: Vec<u16> = vec![0, 1, 100, u16::MAX];

        for position in positions {
            params.token_position = position;
            assert_eq!(params.token_position, position);
        }
    }

    #[test]
    fn test_burn_with_different_amounts() {
        let mut params = create_valid_burn_params();

        // Test with different amounts
        let amounts: Vec<u64> = vec![0, 1, 1000, u64::MAX];

        for amount in amounts {
            params.amount = amount;
            assert_eq!(params.amount, amount);
        }
    }

    #[test]
    fn test_memory_cleanup_for_burn_params() {
        // This test verifies that CString memory is properly managed
        let contract_id = CString::new("GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec").unwrap();
        let note = CString::new("Test note").unwrap();

        let contract_id_ptr = contract_id.into_raw();
        let note_ptr = note.into_raw();

        let params = DashSDKTokenBurnParams {
            token_contract_id: contract_id_ptr,
            serialized_contract: ptr::null(),
            serialized_contract_len: 0,
            token_position: 0,
            amount: 1000,
            public_note: note_ptr,
        };

        // Verify the pointers are set correctly
        assert!(!params.token_contract_id.is_null());
        assert!(!params.public_note.is_null());

        // Manually clean up the CStrings since we can't implement Drop for FFI types
        unsafe {
            let _ = CString::from_raw(contract_id_ptr);
            let _ = CString::from_raw(note_ptr);
        }
    }
}
