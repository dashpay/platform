//! Document deletion operations

use dash_sdk::dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use dash_sdk::dpp::platform_value::string_encoding::Encoding;
use dash_sdk::dpp::prelude::{Identifier, UserFeeIncrease};
use dash_sdk::platform::documents::transitions::DocumentDeleteTransitionBuilder;
use dash_sdk::platform::IdentityPublicKey;
use drive_proof_verifier::ContextProvider;
use std::ffi::CStr;
use std::os::raw::c_char;
use tracing::{debug, error, info};

use crate::document::helpers::{
    convert_state_transition_creation_options, convert_token_payment_info,
};
use crate::sdk::SDKWrapper;
use crate::types::{
    DashSDKPutSettings, DashSDKStateTransitionCreationOptions, DashSDKTokenPaymentInfo, SDKHandle,
    SignerHandle,
};
use crate::{DashSDKError, DashSDKErrorCode, DashSDKResult, FFIError};

/// Delete a document from the platform
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_document_delete(
    sdk_handle: *mut SDKHandle,
    document_id: *const c_char,
    owner_id: *const c_char,
    data_contract_id: *const c_char,
    document_type_name: *const c_char,
    identity_public_key_handle: *const crate::types::IdentityPublicKeyHandle,
    signer_handle: *const SignerHandle,
    token_payment_info: *const DashSDKTokenPaymentInfo,
    put_settings: *const DashSDKPutSettings,
    state_transition_creation_options: *const DashSDKStateTransitionCreationOptions,
) -> DashSDKResult {
    // Validate required parameters
    if sdk_handle.is_null()
        || document_id.is_null()
        || owner_id.is_null()
        || data_contract_id.is_null()
        || document_type_name.is_null()
        || identity_public_key_handle.is_null()
        || signer_handle.is_null()
    {
        return DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
            "One or more required parameters is null".to_string(),
        ));
    }

    let wrapper = &mut *(sdk_handle as *mut SDKWrapper);

    // Parse document ID
    let document_id_str = match CStr::from_ptr(document_id).to_str() {
        Ok(s) => s,
        Err(e) => return DashSDKResult::error(FFIError::from(e).into()),
    };

    // Parse owner ID
    let owner_id_str = match CStr::from_ptr(owner_id).to_str() {
        Ok(s) => s,
        Err(e) => return DashSDKResult::error(FFIError::from(e).into()),
    };

    // Parse data contract ID
    let contract_id_str = match CStr::from_ptr(data_contract_id).to_str() {
        Ok(s) => s,
        Err(e) => return DashSDKResult::error(FFIError::from(e).into()),
    };

    let document_type_name_str = match CStr::from_ptr(document_type_name).to_str() {
        Ok(s) => s,
        Err(e) => return DashSDKResult::error(FFIError::from(e).into()),
    };

    let identity_public_key = &*(identity_public_key_handle as *const IdentityPublicKey);
    let signer = &*(signer_handle as *const crate::signer::VTableSigner);

    let result: Result<Vec<u8>, FFIError> = wrapper.runtime.block_on(async {
        // Parse identifiers (base58 encoded)
        let doc_id = Identifier::from_string(document_id_str, Encoding::Base58)
            .map_err(|e| FFIError::InternalError(format!("Invalid document ID: {}", e)))?;

        let owner_identifier = Identifier::from_string(owner_id_str, Encoding::Base58)
            .map_err(|e| FFIError::InternalError(format!("Invalid owner ID: {}", e)))?;

        let contract_id = Identifier::from_string(contract_id_str, Encoding::Base58)
            .map_err(|e| FFIError::InternalError(format!("Invalid contract ID: {}", e)))?;

        // Get contract from trusted context provider
        let data_contract = if let Some(ref provider) = wrapper.trusted_provider {
            let platform_version = wrapper.sdk.version();
            provider
                .get_data_contract(&contract_id, platform_version)
                .map_err(|e| {
                    FFIError::InternalError(format!("Failed to get contract from context: {}", e))
                })?
                .ok_or_else(|| {
                    FFIError::InternalError(format!(
                        "Contract {} not found in trusted context",
                        contract_id_str
                    ))
                })?
        } else {
            return Err(FFIError::InternalError(
                "No trusted context provider configured".to_string(),
            ));
        };

        // Convert FFI types to Rust types
        let token_payment_info_converted = convert_token_payment_info(token_payment_info)?;
        let settings = crate::identity::convert_put_settings(put_settings);
        let creation_options =
            convert_state_transition_creation_options(state_transition_creation_options);

        // Extract user fee increase from put_settings or use default
        let user_fee_increase: UserFeeIncrease = if put_settings.is_null() {
            0
        } else {
            (*put_settings).user_fee_increase
        };

        // Use DocumentDeleteTransitionBuilder::new with just IDs
        let mut builder = DocumentDeleteTransitionBuilder::new(
            data_contract.clone(),
            document_type_name_str.to_string(),
            doc_id,
            owner_identifier,
        );

        if let Some(token_info) = token_payment_info_converted {
            builder = builder.with_token_payment_info(token_info);
        }

        if let Some(settings) = settings {
            builder = builder.with_settings(settings);
        }

        if user_fee_increase > 0 {
            builder = builder.with_user_fee_increase(user_fee_increase);
        }

        if let Some(options) = creation_options {
            builder = builder.with_state_transition_creation_options(options);
        }

        let state_transition = builder
            .sign(
                &wrapper.sdk,
                identity_public_key,
                signer,
                wrapper.sdk.version(),
            )
            .await
            .map_err(|e| {
                FFIError::InternalError(format!("Failed to create delete transition: {}", e))
            })?;

        // Serialize the state transition with bincode
        let config = bincode::config::standard();
        let serialized = bincode::encode_to_vec(&state_transition, config).map_err(|e| {
            FFIError::InternalError(format!("Failed to serialize state transition: {}", e))
        })?;
        debug!(
            size = serialized.len(),
            "[DOCUMENT DELETE] serialized transition size (bytes)"
        );
        debug!(hex = %hex::encode(&serialized), "[DOCUMENT DELETE] state transition hex");
        Ok(serialized)
    });

    match result {
        Ok(serialized_data) => DashSDKResult::success_binary(serialized_data),
        Err(e) => DashSDKResult::error(e.into()),
    }
}

/// Delete a document from the platform and wait for confirmation
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_document_delete_and_wait(
    sdk_handle: *mut SDKHandle,
    document_id: *const c_char,
    owner_id: *const c_char,
    data_contract_id: *const c_char,
    document_type_name: *const c_char,
    identity_public_key_handle: *const crate::types::IdentityPublicKeyHandle,
    signer_handle: *const SignerHandle,
    token_payment_info: *const DashSDKTokenPaymentInfo,
    put_settings: *const DashSDKPutSettings,
    state_transition_creation_options: *const DashSDKStateTransitionCreationOptions,
) -> DashSDKResult {
    // Validate required parameters
    if sdk_handle.is_null()
        || document_id.is_null()
        || owner_id.is_null()
        || data_contract_id.is_null()
        || document_type_name.is_null()
        || identity_public_key_handle.is_null()
        || signer_handle.is_null()
    {
        return DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
            "One or more required parameters is null".to_string(),
        ));
    }

    info!("[DOCUMENT DELETE] starting document delete operation");

    let wrapper = &mut *(sdk_handle as *mut SDKWrapper);
    let signer = &*(signer_handle as *const crate::signer::VTableSigner);

    // Parse document ID
    let document_id_str = match CStr::from_ptr(document_id).to_str() {
        Ok(s) => s,
        Err(e) => {
            error!(error = %e, "[DOCUMENT DELETE] failed to parse document ID");
            return DashSDKResult::error(FFIError::from(e).into());
        }
    };

    // Parse owner ID
    let owner_id_str = match CStr::from_ptr(owner_id).to_str() {
        Ok(s) => s,
        Err(e) => {
            error!(error = %e, "[DOCUMENT DELETE] failed to parse owner ID");
            return DashSDKResult::error(FFIError::from(e).into());
        }
    };

    // Parse data contract ID
    let contract_id_str = match CStr::from_ptr(data_contract_id).to_str() {
        Ok(s) => s,
        Err(e) => {
            error!(error = %e, "[DOCUMENT DELETE] failed to parse contract ID");
            return DashSDKResult::error(FFIError::from(e).into());
        }
    };

    let document_type_name_str = match CStr::from_ptr(document_type_name).to_str() {
        Ok(s) => s,
        Err(e) => {
            error!(error = %e, "[DOCUMENT DELETE] failed to parse document type name");
            return DashSDKResult::error(FFIError::from(e).into());
        }
    };

    let identity_public_key = &*(identity_public_key_handle as *const IdentityPublicKey);

    debug!(
        document_type = document_type_name_str,
        "[DOCUMENT DELETE] document type"
    );
    debug!(
        document_id = document_id_str,
        "[DOCUMENT DELETE] document id"
    );
    debug!(owner_id = owner_id_str, "[DOCUMENT DELETE] owner id");

    let result: Result<Identifier, FFIError> = wrapper.runtime.block_on(async {
        // Parse identifiers (base58 encoded)
        let doc_id = Identifier::from_string(document_id_str, Encoding::Base58)
            .map_err(|e| FFIError::InternalError(format!("Invalid document ID: {}", e)))?;

        let owner_identifier = Identifier::from_string(owner_id_str, Encoding::Base58)
            .map_err(|e| FFIError::InternalError(format!("Invalid owner ID: {}", e)))?;

        let contract_id = Identifier::from_string(contract_id_str, Encoding::Base58)
            .map_err(|e| FFIError::InternalError(format!("Invalid contract ID: {}", e)))?;

        // Get contract from trusted context provider
        let data_contract = if let Some(ref provider) = wrapper.trusted_provider {
            let platform_version = wrapper.sdk.version();
            provider
                .get_data_contract(&contract_id, platform_version)
                .map_err(|e| {
                    FFIError::InternalError(format!("Failed to get contract from context: {}", e))
                })?
                .ok_or_else(|| {
                    FFIError::InternalError(format!(
                        "Contract {} not found in trusted context",
                        contract_id_str
                    ))
                })?
        } else {
            return Err(FFIError::InternalError(
                "No trusted context provider configured".to_string(),
            ));
        };

        // Convert FFI types to Rust types
        let token_payment_info_converted = convert_token_payment_info(token_payment_info)?;
        let settings = crate::identity::convert_put_settings(put_settings);
        let creation_options =
            convert_state_transition_creation_options(state_transition_creation_options);

        // Extract user fee increase from put_settings or use default
        let user_fee_increase: UserFeeIncrease = if put_settings.is_null() {
            0
        } else {
            (*put_settings).user_fee_increase
        };

        debug!("[DOCUMENT DELETE] building document delete transition");

        // Use DocumentDeleteTransitionBuilder::new with just IDs
        let mut builder = DocumentDeleteTransitionBuilder::new(
            data_contract.clone(),
            document_type_name_str.to_string(),
            doc_id,
            owner_identifier,
        );

        if let Some(token_info) = token_payment_info_converted {
            debug!("[DOCUMENT DELETE] adding token payment info");
            builder = builder.with_token_payment_info(token_info);
        }

        if let Some(settings) = settings {
            debug!("[DOCUMENT DELETE] adding put settings");
            builder = builder.with_settings(settings);
        }

        if user_fee_increase > 0 {
            debug!(user_fee_increase, "[DOCUMENT DELETE] setting user fee increase");
            builder = builder.with_user_fee_increase(user_fee_increase);
        }

        if let Some(options) = creation_options {
            debug!("[DOCUMENT DELETE] adding state transition creation options");
            builder = builder.with_state_transition_creation_options(options);
        }

        debug!("[DOCUMENT DELETE] calling SDK document_delete");
        debug!(key_id = identity_public_key.id(), purpose = ?identity_public_key.purpose(), security_level = ?identity_public_key.security_level(), key_type = ?identity_public_key.key_type(), "[DOCUMENT DELETE] identity public key info");

        let result = wrapper
            .sdk
            .document_delete(builder, identity_public_key, signer)
            .await
            .map_err(|e| {
                error!(error = %e, key_id = identity_public_key.id(), "[DOCUMENT DELETE] SDK call failed");
                FFIError::InternalError(format!("Failed to delete document and wait: {}", e))
            })?;

        info!("[DOCUMENT DELETE] SDK call completed successfully");

        let dash_sdk::platform::documents::transitions::DocumentDeleteResult::Deleted(deleted_id) = result;

        Ok(deleted_id)
    });

    match result {
        Ok(_deleted_id) => {
            info!("[DOCUMENT DELETE] document delete completed successfully");
            DashSDKResult::success(std::ptr::null_mut())
        }
        Err(e) => {
            error!(error = ?e, "[DOCUMENT DELETE] document delete failed");
            DashSDKResult::error(e.into())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::test_utils::*;
    use crate::DashSDKErrorCode;

    use dash_sdk::dpp::document::{Document, DocumentV0};
    use dash_sdk::dpp::platform_value::Value;
    use dash_sdk::dpp::prelude::Identifier;
    use dash_sdk::platform::IdentityPublicKey;

    use std::collections::BTreeMap;
    use std::ffi::{CStr, CString};
    use std::ptr;

    // Helper function to create a mock document
    fn create_mock_document() -> Box<Document> {
        let id = Identifier::from_bytes(&[2u8; 32]).unwrap();
        let owner_id = Identifier::from_bytes(&[1u8; 32]).unwrap();

        let mut properties = BTreeMap::new();
        properties.insert("name".to_string(), Value::Text("Test Document".to_string()));

        let document = Document::V0(DocumentV0 {
            id,
            owner_id,
            properties,
            revision: Some(1),
            created_at: None,
            updated_at: None,
            transferred_at: None,
            created_at_block_height: None,
            updated_at_block_height: None,
            transferred_at_block_height: None,
            created_at_core_block_height: None,
            updated_at_core_block_height: None,
            transferred_at_core_block_height: None,
        });

        Box::new(document)
    }

    #[test]
    fn test_delete_with_null_sdk_handle() {
        let identity_public_key = create_mock_identity_public_key();
        let signer = create_mock_signer();

        // Create string IDs instead of using document handle
        let document_id = CString::new("GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec").unwrap();
        let owner_id = CString::new("4EfA9Jrvv3nnCFdSf7fad59851iiTRZ6Wcu6YVJ4iSeF").unwrap();
        let contract_id = CString::new("GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec").unwrap();
        let document_type_name = CString::new("testDoc").unwrap();

        // Use IdentityPublicKeyHandle instead of raw bytes
        let identity_public_key_handle =
            Box::into_raw(identity_public_key) as *const crate::types::IdentityPublicKeyHandle;
        let signer_handle = Box::into_raw(signer) as *const SignerHandle;

        let put_settings = create_put_settings();

        let result = unsafe {
            dash_sdk_document_delete(
                ptr::null_mut(), // null SDK handle
                document_id.as_ptr(),
                owner_id.as_ptr(),
                contract_id.as_ptr(),
                document_type_name.as_ptr(),
                identity_public_key_handle,
                signer_handle,
                ptr::null(),
                &put_settings,
                ptr::null(),
            )
        };

        assert!(!result.error.is_null());
        unsafe {
            let error = &*result.error;
            assert_eq!(error.code, DashSDKErrorCode::InvalidParameter);
            let error_msg = CStr::from_ptr(error.message).to_str().unwrap();
            assert!(error_msg.contains("null"));
        }

        // Clean up
        unsafe {
            let _ = Box::from_raw(identity_public_key_handle as *mut IdentityPublicKey);
            let _ = Box::from_raw(signer_handle as *mut crate::signer::VTableSigner);
        }
    }

    #[test]
    fn test_delete_with_null_document() {
        let sdk_handle = create_mock_sdk_handle();
        let identity_public_key = create_mock_identity_public_key();
        let signer = create_mock_signer();

        let owner_id = CString::new("4EfA9Jrvv3nnCFdSf7fad59851iiTRZ6Wcu6YVJ4iSeF").unwrap();
        let contract_id = CString::new("GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec").unwrap();
        let document_type_name = CString::new("testDoc").unwrap();

        let identity_public_key_handle =
            Box::into_raw(identity_public_key) as *const crate::types::IdentityPublicKeyHandle;
        let signer_handle = Box::into_raw(signer) as *const SignerHandle;

        let put_settings = create_put_settings();

        let result = unsafe {
            dash_sdk_document_delete(
                sdk_handle,
                ptr::null(), // null document_id
                owner_id.as_ptr(),
                contract_id.as_ptr(),
                document_type_name.as_ptr(),
                identity_public_key_handle,
                signer_handle,
                ptr::null(),
                &put_settings,
                ptr::null(),
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
    fn test_delete_with_null_data_contract() {
        let sdk_handle = create_mock_sdk_handle();
        let identity_public_key = create_mock_identity_public_key();
        let signer = create_mock_signer();

        let document_id = CString::new("GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec").unwrap();
        let owner_id = CString::new("4EfA9Jrvv3nnCFdSf7fad59851iiTRZ6Wcu6YVJ4iSeF").unwrap();
        let document_type_name = CString::new("testDoc").unwrap();

        let identity_public_key_handle =
            Box::into_raw(identity_public_key) as *const crate::types::IdentityPublicKeyHandle;
        let signer_handle = Box::into_raw(signer) as *const SignerHandle;

        let put_settings = create_put_settings();

        let result = unsafe {
            dash_sdk_document_delete(
                sdk_handle,
                document_id.as_ptr(),
                owner_id.as_ptr(),
                ptr::null(), // null data contract ID
                document_type_name.as_ptr(),
                identity_public_key_handle,
                signer_handle,
                ptr::null(),
                &put_settings,
                ptr::null(),
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
    fn test_delete_with_null_document_type_name() {
        let sdk_handle = create_mock_sdk_handle();
        let identity_public_key = create_mock_identity_public_key();
        let signer = create_mock_signer();

        let document_id = CString::new("GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec").unwrap();
        let owner_id = CString::new("4EfA9Jrvv3nnCFdSf7fad59851iiTRZ6Wcu6YVJ4iSeF").unwrap();
        let contract_id = CString::new("GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec").unwrap();

        let identity_public_key_handle =
            Box::into_raw(identity_public_key) as *const crate::types::IdentityPublicKeyHandle;
        let signer_handle = Box::into_raw(signer) as *const SignerHandle;

        let put_settings = create_put_settings();

        let result = unsafe {
            dash_sdk_document_delete(
                sdk_handle,
                document_id.as_ptr(),
                owner_id.as_ptr(),
                contract_id.as_ptr(),
                ptr::null(), // null document type name
                identity_public_key_handle,
                signer_handle,
                ptr::null(),
                &put_settings,
                ptr::null(),
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
    fn test_delete_with_null_identity_public_key() {
        let sdk_handle = create_mock_sdk_handle();
        let signer = create_mock_signer();

        let document_id = CString::new("GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec").unwrap();
        let owner_id = CString::new("4EfA9Jrvv3nnCFdSf7fad59851iiTRZ6Wcu6YVJ4iSeF").unwrap();
        let contract_id = CString::new("GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec").unwrap();
        let document_type_name = CString::new("testDoc").unwrap();

        let signer_handle = Box::into_raw(signer) as *const SignerHandle;

        let put_settings = create_put_settings();

        let result = unsafe {
            dash_sdk_document_delete(
                sdk_handle,
                document_id.as_ptr(),
                owner_id.as_ptr(),
                contract_id.as_ptr(),
                document_type_name.as_ptr(),
                ptr::null(), // null identity public key handle
                signer_handle,
                ptr::null(),
                &put_settings,
                ptr::null(),
            )
        };

        assert!(!result.error.is_null());
        unsafe {
            let error = &*result.error;
            assert_eq!(error.code, DashSDKErrorCode::InvalidParameter);
        }

        // Clean up
        unsafe {
            let _ = Box::from_raw(signer_handle as *mut crate::signer::VTableSigner);
        }
        destroy_mock_sdk_handle(sdk_handle);
    }

    #[test]
    fn test_delete_with_null_signer() {
        let sdk_handle = create_mock_sdk_handle();
        let identity_public_key = create_mock_identity_public_key();

        let document_id = CString::new("GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec").unwrap();
        let owner_id = CString::new("4EfA9Jrvv3nnCFdSf7fad59851iiTRZ6Wcu6YVJ4iSeF").unwrap();
        let contract_id = CString::new("GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec").unwrap();
        let document_type_name = CString::new("testDoc").unwrap();

        let identity_public_key_handle =
            Box::into_raw(identity_public_key) as *const crate::types::IdentityPublicKeyHandle;

        let put_settings = create_put_settings();

        let result = unsafe {
            dash_sdk_document_delete(
                sdk_handle,
                document_id.as_ptr(),
                owner_id.as_ptr(),
                contract_id.as_ptr(),
                document_type_name.as_ptr(),
                identity_public_key_handle,
                ptr::null(), // null signer
                ptr::null(),
                &put_settings,
                ptr::null(),
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
        }
        destroy_mock_sdk_handle(sdk_handle);
    }

    #[test]
    fn test_delete_and_wait_with_null_parameters() {
        // Similar tests for dash_sdk_document_delete_and_wait
        let sdk_handle = create_mock_sdk_handle();
        let identity_public_key = create_mock_identity_public_key();
        let signer = create_mock_signer();

        let document_id = CString::new("GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec").unwrap();
        let owner_id = CString::new("4EfA9Jrvv3nnCFdSf7fad59851iiTRZ6Wcu6YVJ4iSeF").unwrap();
        let contract_id = CString::new("GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec").unwrap();
        let document_type_name = CString::new("testDoc").unwrap();

        let identity_public_key_handle =
            Box::into_raw(identity_public_key) as *const crate::types::IdentityPublicKeyHandle;
        let signer_handle = Box::into_raw(signer) as *const SignerHandle;

        let put_settings = create_put_settings();

        // Test with null SDK handle
        let result = unsafe {
            dash_sdk_document_delete_and_wait(
                ptr::null_mut(),
                document_id.as_ptr(),
                owner_id.as_ptr(),
                contract_id.as_ptr(),
                document_type_name.as_ptr(),
                identity_public_key_handle,
                signer_handle,
                ptr::null(),
                &put_settings,
                ptr::null(),
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
}
