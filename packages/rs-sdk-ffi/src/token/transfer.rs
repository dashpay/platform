//! Token transfer operations

use super::types::DashSDKTokenTransferParams;
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
use dash_sdk::dpp::data_contract::TokenContractPosition;
use dash_sdk::dpp::platform_value::string_encoding::Encoding;
use dash_sdk::dpp::prelude::Identifier;
use dash_sdk::platform::tokens::builders::transfer::TokenTransferTransitionBuilder;
use dash_sdk::platform::tokens::transitions::TransferResult;
use dash_sdk::platform::IdentityPublicKey;
use std::ffi::CStr;
use std::sync::Arc;

/// Token transfer to another identity and wait for confirmation
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_token_transfer(
    sdk_handle: *mut SDKHandle,
    transition_owner_id: *const u8,
    params: *const DashSDKTokenTransferParams,
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
    let sender_id = match Identifier::from_bytes(transition_owner_id_slice) {
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

    // Validate recipient ID
    if params.recipient_id.is_null() {
        return DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
            "Recipient ID is required".to_string(),
        ));
    }

    let recipient_id = match parse_identifier_from_bytes(params.recipient_id) {
        Ok(id) => id,
        Err(e) => return DashSDKResult::error(e.into()),
    };

    // Parse optional notes
    let public_note = match parse_optional_note(params.public_note) {
        Ok(note) => note,
        Err(e) => return DashSDKResult::error(e.into()),
    };

    let result: Result<TransferResult, FFIError> = wrapper.runtime.block_on(async {
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

        // Create token transfer transition builder
        let mut builder = TokenTransferTransitionBuilder::new(
            Arc::new(data_contract),
            params.token_position as TokenContractPosition,
            sender_id,
            recipient_id,
            params.amount as TokenAmount,
        );

        // Add optional notes
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

        // Use SDK method to transfer and wait
        let result = wrapper
            .sdk
            .token_transfer(builder, identity_public_key, signer)
            .await
            .map_err(|e| {
                FFIError::InternalError(format!("Failed to transfer token and wait: {}", e))
            })?;

        Ok(result)
    });

    match result {
        Ok(_transfer_result) => DashSDKResult::success(std::ptr::null_mut()),
        Err(e) => DashSDKResult::error(e.into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use dash_sdk::dpp::identity::identity_public_key::v0::IdentityPublicKeyV0;
    use dash_sdk::dpp::identity::{KeyType, Purpose, SecurityLevel};
    use dash_sdk::dpp::platform_value::BinaryData;
    use dash_sdk::platform::IdentityPublicKey;
    use std::ffi::CString;
    use std::ptr;

    // Helper function to create a mock SDK handle
    fn create_mock_sdk_handle() -> *mut SDKHandle {
        let wrapper = Box::new(crate::sdk::SDKWrapper::new_mock());
        Box::into_raw(wrapper) as *mut SDKHandle
    }

    // Helper function to destroy a mock SDK handle
    fn destroy_mock_sdk_handle(handle: *mut SDKHandle) {
        unsafe {
            crate::sdk::dash_sdk_destroy(handle);
        }
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
        _signer: *const std::os::raw::c_void,
        _identity_public_key_bytes: *const u8,
        _identity_public_key_len: usize,
        _data: *const u8,
        _data_len: usize,
        result_len: *mut usize,
    ) -> *mut u8 {
        // Return a mock signature (64 bytes for ECDSA) allocated with libc::malloc
        let signature = vec![0u8; 64];
        *result_len = signature.len();
        let ptr = libc::malloc(signature.len()) as *mut u8;
        if !ptr.is_null() {
            std::ptr::copy_nonoverlapping(signature.as_ptr(), ptr, signature.len());
        }
        ptr
    }

    unsafe extern "C" fn mock_can_sign_callback(
        _signer: *const std::os::raw::c_void,
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

    fn create_valid_recipient_id() -> [u8; 32] {
        [2u8; 32]
    }

    fn create_valid_transfer_params() -> DashSDKTokenTransferParams {
        // Note: In real tests, the caller is responsible for freeing the CString memory
        DashSDKTokenTransferParams {
            token_contract_id: CString::new("GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec")
                .unwrap()
                .into_raw(),
            serialized_contract: ptr::null(),
            serialized_contract_len: 0,
            token_position: 0,
            recipient_id: Box::into_raw(Box::new(create_valid_recipient_id())) as *const u8,
            amount: 1000,
            public_note: ptr::null(),
            private_encrypted_note: ptr::null(),
            shared_encrypted_note: ptr::null(),
        }
    }

    // Helper to clean up params after use
    unsafe fn cleanup_transfer_params(params: &DashSDKTokenTransferParams) {
        if !params.token_contract_id.is_null() {
            let _ = CString::from_raw(params.token_contract_id as *mut std::os::raw::c_char);
        }
        if !params.public_note.is_null() {
            let _ = CString::from_raw(params.public_note as *mut std::os::raw::c_char);
        }
        if !params.private_encrypted_note.is_null() {
            let _ = CString::from_raw(params.private_encrypted_note as *mut std::os::raw::c_char);
        }
        if !params.shared_encrypted_note.is_null() {
            let _ = CString::from_raw(params.shared_encrypted_note as *mut std::os::raw::c_char);
        }
        if !params.recipient_id.is_null() {
            let _ = Box::from_raw(params.recipient_id as *mut [u8; 32]);
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
    fn test_transfer_with_null_sdk_handle() {
        let transition_owner_id = create_valid_transition_owner_id();
        let params = create_valid_transfer_params();
        let identity_public_key = create_mock_identity_public_key();
        let signer = create_mock_signer();
        let identity_public_key_handle =
            Box::into_raw(identity_public_key) as *const crate::types::IdentityPublicKeyHandle;
        let signer_handle = Box::into_raw(signer) as *const SignerHandle;
        let put_settings = create_put_settings();
        let state_transition_options: *const DashSDKStateTransitionCreationOptions = ptr::null();

        let result = unsafe {
            dash_sdk_token_transfer(
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
            cleanup_transfer_params(&params);
        }
    }

    #[test]
    fn test_transfer_with_null_transition_owner_id() {
        let sdk_handle = create_mock_sdk_handle();
        let params = create_valid_transfer_params();
        let identity_public_key = create_mock_identity_public_key();
        let signer = create_mock_signer();
        let identity_public_key_handle =
            Box::into_raw(identity_public_key) as *const crate::types::IdentityPublicKeyHandle;
        let signer_handle = Box::into_raw(signer) as *const SignerHandle;
        let put_settings = create_put_settings();
        let state_transition_options: *const DashSDKStateTransitionCreationOptions = ptr::null();

        let result = unsafe {
            dash_sdk_token_transfer(
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

        // Clean up params memory
        unsafe {
            cleanup_transfer_params(&params);
        }
    }

    #[test]
    fn test_transfer_with_null_params() {
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
            dash_sdk_token_transfer(
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

        // No params to clean up since we passed null
    }

    #[test]
    fn test_transfer_with_null_identity_public_key() {
        let sdk_handle = create_mock_sdk_handle();
        let transition_owner_id = create_valid_transition_owner_id();
        let params = create_valid_transfer_params();
        let signer = create_mock_signer();
        let signer_handle = Box::into_raw(signer) as *const SignerHandle;
        let put_settings = create_put_settings();
        let state_transition_options: *const DashSDKStateTransitionCreationOptions = ptr::null();

        let result = unsafe {
            dash_sdk_token_transfer(
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

        // Clean up params memory
        unsafe {
            cleanup_transfer_params(&params);
            let _ = Box::from_raw(signer_handle as *mut crate::signer::VTableSigner);
        }
        destroy_mock_sdk_handle(sdk_handle);
    }

    #[test]
    fn test_transfer_with_null_signer() {
        let sdk_handle = create_mock_sdk_handle();
        let transition_owner_id = create_valid_transition_owner_id();
        let params = create_valid_transfer_params();
        let identity_public_key = create_mock_identity_public_key();
        let identity_public_key_handle =
            Box::into_raw(identity_public_key) as *const crate::types::IdentityPublicKeyHandle;
        let put_settings = create_put_settings();
        let state_transition_options: *const DashSDKStateTransitionCreationOptions = ptr::null();

        let result = unsafe {
            dash_sdk_token_transfer(
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

        // Clean up params memory
        unsafe {
            cleanup_transfer_params(&params);
            let _ = Box::from_raw(identity_public_key_handle as *mut IdentityPublicKey);
        }
        destroy_mock_sdk_handle(sdk_handle);
    }

    #[test]
    fn test_transfer_with_null_recipient_id() {
        let sdk_handle = create_mock_sdk_handle();
        let transition_owner_id = create_valid_transition_owner_id();
        let mut params = create_valid_transfer_params();

        // Clean up the valid recipient_id first
        unsafe {
            let _ = Box::from_raw(params.recipient_id as *mut [u8; 32]);
        }
        params.recipient_id = ptr::null();

        let identity_public_key = create_mock_identity_public_key();
        let signer = create_mock_signer();
        let identity_public_key_handle =
            Box::into_raw(identity_public_key) as *const crate::types::IdentityPublicKeyHandle;
        let signer_handle = Box::into_raw(signer) as *const SignerHandle;
        let put_settings = create_put_settings();
        let state_transition_options: *const DashSDKStateTransitionCreationOptions = ptr::null();

        let result = unsafe {
            dash_sdk_token_transfer(
                sdk_handle,
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
            assert!(error_msg.contains("Recipient ID is required"));
        }

        // Clean up remaining params memory
        unsafe {
            if !params.token_contract_id.is_null() {
                let _ = CString::from_raw(params.token_contract_id as *mut std::os::raw::c_char);
            }
        }
    }

    #[test]
    fn test_transfer_with_public_note() {
        let transition_owner_id = create_valid_transition_owner_id();
        let mut params = create_valid_transfer_params();
        params.public_note = CString::new("Payment for services rendered")
            .unwrap()
            .into_raw();

        let sdk_handle = create_mock_sdk_handle();
        let identity_public_key = create_mock_identity_public_key();
        let signer = create_mock_signer();
        let identity_public_key_handle =
            Box::into_raw(identity_public_key) as *const crate::types::IdentityPublicKeyHandle;
        let signer_handle = Box::into_raw(signer) as *const SignerHandle;
        let put_settings = create_put_settings();
        let state_transition_options: *const DashSDKStateTransitionCreationOptions = ptr::null();

        // Note: This test will fail when actually executed against a real SDK
        // but it validates the parameter handling
        let _result = unsafe {
            dash_sdk_token_transfer(
                sdk_handle,
                transition_owner_id.as_ptr(),
                &params,
                identity_public_key_handle,
                signer_handle,
                &put_settings,
                state_transition_options,
            )
        };

        // Clean up params memory
        unsafe {
            cleanup_transfer_params(&params);
        }
    }

    #[test]
    fn test_transfer_with_serialized_contract() {
        let transition_owner_id = create_valid_transition_owner_id();
        let mut params = create_valid_transfer_params();
        let contract_data = vec![0u8; 100]; // Mock serialized contract
        params.serialized_contract = contract_data.as_ptr();
        params.serialized_contract_len = contract_data.len();

        let sdk_handle = create_mock_sdk_handle();
        let identity_public_key = create_mock_identity_public_key();
        let signer = create_mock_signer();
        let identity_public_key_handle =
            Box::into_raw(identity_public_key) as *const crate::types::IdentityPublicKeyHandle;
        let signer_handle = Box::into_raw(signer) as *const SignerHandle;
        let put_settings = create_put_settings();
        let state_transition_options: *const DashSDKStateTransitionCreationOptions = ptr::null();

        // Note: This test will fail when actually executed against a real SDK
        // but it validates the parameter handling
        let _result = unsafe {
            dash_sdk_token_transfer(
                sdk_handle,
                transition_owner_id.as_ptr(),
                &params,
                identity_public_key_handle,
                signer_handle,
                &put_settings,
                state_transition_options,
            )
        };

        // Clean up params memory (but not the contract data since we don't own it)
        unsafe {
            let _ = CString::from_raw(params.token_contract_id as *mut std::os::raw::c_char);
            let _ = Box::from_raw(params.recipient_id as *mut [u8; 32]);
        }
    }

    #[test]
    fn test_transfer_with_different_amounts() {
        let transition_owner_id = create_valid_transition_owner_id();
        let amounts = [1u64, 100u64, 1000u64, u64::MAX];

        for amount in amounts {
            let mut params = create_valid_transfer_params();
            params.amount = amount;

            let sdk_handle = create_mock_sdk_handle();
            let identity_public_key = create_mock_identity_public_key();
            let identity_public_key_handle =
                Box::into_raw(identity_public_key) as *const crate::types::IdentityPublicKeyHandle;
            let signer = create_mock_signer();
            let signer_handle = Box::into_raw(signer) as *const SignerHandle;
            let put_settings = create_put_settings();
            let state_transition_options: *const DashSDKStateTransitionCreationOptions =
                ptr::null();

            // Note: This test will fail when actually executed against a real SDK
            // but it validates the parameter handling
            let _result = unsafe {
                dash_sdk_token_transfer(
                    sdk_handle,
                    transition_owner_id.as_ptr(),
                    &params,
                    identity_public_key_handle,
                    signer_handle,
                    &put_settings,
                    state_transition_options,
                )
            };

            // Clean up params memory
            unsafe {
                cleanup_transfer_params(&params);
                let _ = Box::from_raw(identity_public_key_handle as *mut IdentityPublicKey);
                let _ = Box::from_raw(signer_handle as *mut crate::signer::VTableSigner);
            }
            destroy_mock_sdk_handle(sdk_handle);
        }
    }

    #[test]
    fn test_transfer_with_different_token_positions() {
        let transition_owner_id = create_valid_transition_owner_id();
        let token_positions = [0u16, 1u16, 10u16, 255u16];

        for position in token_positions {
            let mut params = create_valid_transfer_params();
            params.token_position = position;

            let sdk_handle = create_mock_sdk_handle();
            let identity_public_key = create_mock_identity_public_key();
            let identity_public_key_handle =
                Box::into_raw(identity_public_key) as *const crate::types::IdentityPublicKeyHandle;
            let signer = create_mock_signer();
            let signer_handle = Box::into_raw(signer) as *const SignerHandle;
            let put_settings = create_put_settings();
            let state_transition_options: *const DashSDKStateTransitionCreationOptions =
                ptr::null();

            // Note: This test will fail when actually executed against a real SDK
            // but it validates the parameter handling
            let _result = unsafe {
                dash_sdk_token_transfer(
                    sdk_handle,
                    transition_owner_id.as_ptr(),
                    &params,
                    identity_public_key_handle,
                    signer_handle,
                    &put_settings,
                    state_transition_options,
                )
            };

            // Clean up params memory
            unsafe {
                cleanup_transfer_params(&params);
                let _ = Box::from_raw(identity_public_key_handle as *mut IdentityPublicKey);
                let _ = Box::from_raw(signer_handle as *mut crate::signer::VTableSigner);
            }
            destroy_mock_sdk_handle(sdk_handle);
        }
    }
}
