//! Document put-to-platform operations

use dash_sdk::dpp::document::{Document, DocumentV0Getters};
use dash_sdk::dpp::platform_value::string_encoding::Encoding;
use dash_sdk::dpp::prelude::{DataContract, Identifier, UserFeeIncrease};
use dash_sdk::platform::documents::transitions::{
    DocumentCreateTransitionBuilder, DocumentReplaceTransitionBuilder,
};
use dash_sdk::platform::IdentityPublicKey;
use drive_proof_verifier::ContextProvider;
use std::ffi::CStr;
use std::os::raw::c_char;
use std::sync::Arc;

use crate::document::helpers::{
    convert_state_transition_creation_options, convert_token_payment_info,
};
use crate::sdk::SDKWrapper;
use crate::types::{
    DashSDKPutSettings, DashSDKResultDataType, DashSDKStateTransitionCreationOptions,
    DashSDKTokenPaymentInfo, DocumentHandle, SDKHandle, SignerHandle,
};
use crate::{DashSDKError, DashSDKErrorCode, DashSDKResult, FFIError};

/// Put document to platform (broadcast state transition)
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_document_put_to_platform(
    sdk_handle: *mut SDKHandle,
    document_handle: *const DocumentHandle,
    data_contract_id: *const c_char,
    document_type_name: *const c_char,
    entropy: *const [u8; 32],
    identity_public_key_handle: *const crate::types::IdentityPublicKeyHandle,
    signer_handle: *const SignerHandle,
    token_payment_info: *const DashSDKTokenPaymentInfo,
    put_settings: *const DashSDKPutSettings,
    state_transition_creation_options: *const DashSDKStateTransitionCreationOptions,
) -> DashSDKResult {
    // Validate required parameters
    if sdk_handle.is_null()
        || document_handle.is_null()
        || data_contract_id.is_null()
        || document_type_name.is_null()
        || entropy.is_null()
        || identity_public_key_handle.is_null()
        || signer_handle.is_null()
    {
        return DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
            "One or more required parameters is null".to_string(),
        ));
    }

    let wrapper = &mut *(sdk_handle as *mut SDKWrapper);
    let document = &*(document_handle as *const Document);
    let identity_public_key = &*(identity_public_key_handle as *const IdentityPublicKey);
    let signer = &*(signer_handle as *const crate::signer::VTableSigner);
    let entropy_bytes = *entropy;

    let contract_id_str = match CStr::from_ptr(data_contract_id).to_str() {
        Ok(s) => s,
        Err(e) => return DashSDKResult::error(FFIError::from(e).into()),
    };

    let document_type_name_str = match CStr::from_ptr(document_type_name).to_str() {
        Ok(s) => s,
        Err(e) => return DashSDKResult::error(FFIError::from(e).into()),
    };

    let result: Result<Vec<u8>, FFIError> = wrapper.runtime.block_on(async {
        // Parse contract ID (base58 encoded)
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

        // Use the new DocumentCreateTransitionBuilder or DocumentReplaceTransitionBuilder
        let state_transition = if document.revision().unwrap_or(0) == 1 {
            // Create transition for new documents
            let mut builder = DocumentCreateTransitionBuilder::new(
                data_contract.clone(),
                document_type_name_str.to_string(),
                document.clone(),
                entropy_bytes,
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

            builder
                .sign(
                    &wrapper.sdk,
                    &identity_public_key,
                    signer,
                    wrapper.sdk.version(),
                )
                .await
        } else {
            // Replace transition for existing documents
            let mut builder = DocumentReplaceTransitionBuilder::new(
                data_contract.clone(),
                document_type_name_str.to_string(),
                document.clone(),
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

            builder
                .sign(
                    &wrapper.sdk,
                    &identity_public_key,
                    signer,
                    wrapper.sdk.version(),
                )
                .await
        }
        .map_err(|e| {
            FFIError::InternalError(format!("Failed to create document transition: {}", e))
        })?;

        // Serialize the state transition with bincode
        let config = bincode::config::standard();
        bincode::encode_to_vec(&state_transition, config).map_err(|e| {
            FFIError::InternalError(format!("Failed to serialize state transition: {}", e))
        })
    });

    match result {
        Ok(serialized_data) => DashSDKResult::success_binary(serialized_data),
        Err(e) => DashSDKResult::error(e.into()),
    }
}

/// Put document to platform and wait for confirmation (broadcast state transition and wait for response)
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_document_put_to_platform_and_wait(
    sdk_handle: *mut SDKHandle,
    document_handle: *const DocumentHandle,
    data_contract_id: *const c_char,
    document_type_name: *const c_char,
    entropy: *const [u8; 32],
    identity_public_key_handle: *const crate::types::IdentityPublicKeyHandle,
    signer_handle: *const SignerHandle,
    token_payment_info: *const DashSDKTokenPaymentInfo,
    put_settings: *const DashSDKPutSettings,
    state_transition_creation_options: *const DashSDKStateTransitionCreationOptions,
) -> DashSDKResult {
    // Validate required parameters
    if sdk_handle.is_null()
        || document_handle.is_null()
        || data_contract_id.is_null()
        || document_type_name.is_null()
        || entropy.is_null()
        || identity_public_key_handle.is_null()
        || signer_handle.is_null()
    {
        return DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
            "One or more required parameters is null".to_string(),
        ));
    }

    let wrapper = &mut *(sdk_handle as *mut SDKWrapper);
    let document = &*(document_handle as *const Document);
    let identity_public_key = &*(identity_public_key_handle as *const IdentityPublicKey);
    let signer = &*(signer_handle as *const crate::signer::VTableSigner);
    let entropy_bytes = *entropy;

    let contract_id_str = match CStr::from_ptr(data_contract_id).to_str() {
        Ok(s) => s,
        Err(e) => return DashSDKResult::error(FFIError::from(e).into()),
    };

    let document_type_name_str = match CStr::from_ptr(document_type_name).to_str() {
        Ok(s) => s,
        Err(e) => return DashSDKResult::error(FFIError::from(e).into()),
    };

    let result: Result<Document, FFIError> = wrapper.runtime.block_on(async {
        // Parse contract ID (base58 encoded)
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

        // Use the new builder pattern and SDK methods
        let confirmed_document = if document.revision().unwrap_or(1) == 1 {
            // Create transition for new documents
            let mut builder = DocumentCreateTransitionBuilder::new(
                data_contract.clone(),
                document_type_name_str.to_string(),
                document.clone(),
                entropy_bytes,
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

            let result = wrapper
                .sdk
                .document_create(builder, &identity_public_key, signer)
                .await
                .map_err(|e| {
                    FFIError::InternalError(format!("Failed to create document and wait: {}", e))
                })?;

            match result {
                dash_sdk::platform::documents::transitions::DocumentCreateResult::Document(doc) => {
                    doc
                }
            }
        } else {
            // Replace transition for existing documents
            let mut builder = DocumentReplaceTransitionBuilder::new(
                data_contract.clone(),
                document_type_name_str.to_string(),
                document.clone(),
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

            let result = wrapper
                .sdk
                .document_replace(builder, &identity_public_key, signer)
                .await
                .map_err(|e| {
                    FFIError::InternalError(format!("Failed to replace document and wait: {}", e))
                })?;

            match result {
                dash_sdk::platform::documents::transitions::DocumentReplaceResult::Document(
                    doc,
                ) => doc,
            }
        };

        Ok(confirmed_document)
    });

    match result {
        Ok(confirmed_document) => {
            let handle = Box::into_raw(Box::new(confirmed_document)) as *mut DocumentHandle;
            DashSDKResult::success_handle(
                handle as *mut std::os::raw::c_void,
                DashSDKResultDataType::ResultDocumentHandle,
            )
        }
        Err(e) => DashSDKResult::error(e.into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::test_utils::*;
    use crate::DashSDKErrorCode;

    use dash_sdk::dpp::document::{Document, DocumentV0};
    use dash_sdk::dpp::platform_value::Value;
    use dash_sdk::dpp::prelude::{Identifier, Revision};

    use std::collections::BTreeMap;
    use std::ffi::{CStr, CString};
    use std::ptr;

    // Helper function to create a mock document with specific revision
    fn create_mock_document_with_revision(revision: Revision) -> Box<Document> {
        let id = Identifier::from_bytes(&[2u8; 32]).unwrap();
        let owner_id = Identifier::from_bytes(&[1u8; 32]).unwrap();

        let mut properties = BTreeMap::new();
        properties.insert("name".to_string(), Value::Text("Test Document".to_string()));

        let document = Document::V0(DocumentV0 {
            id,
            owner_id,
            properties: properties,
            revision: Some(revision),
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

    // Helper function to create valid entropy
    fn create_valid_entropy() -> [u8; 32] {
        [42u8; 32]
    }

    #[test]
    fn test_put_with_null_sdk_handle() {
        let document = create_mock_document_with_revision(1);
        let identity_public_key = create_mock_identity_public_key();
        let signer = create_mock_signer();

        let document_handle = Box::into_raw(document) as *const DocumentHandle;
        let signer_handle = Box::into_raw(signer) as *const SignerHandle;

        let document_type_name = CString::new("testDoc").unwrap();
        let entropy = create_valid_entropy();
        let put_settings = create_put_settings();
        let contract_id = CString::new("4EfA9Jrvv3nnCFdSf7fad59851iiTRZ6Wcu6YVJ4iSeF").unwrap();

        let identity_public_key_handle =
            Box::into_raw(identity_public_key) as *const crate::types::IdentityPublicKeyHandle;

        let result = unsafe {
            dash_sdk_document_put_to_platform(
                ptr::null_mut(), // null SDK handle
                document_handle,
                contract_id.as_ptr(),
                document_type_name.as_ptr(),
                &entropy,
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
            let _ = Box::from_raw(document_handle as *mut Document);
            let _ = Box::from_raw(identity_public_key_handle as *mut IdentityPublicKey);
            let _ = Box::from_raw(signer_handle as *mut crate::signer::VTableSigner);
        }
    }

    #[test]
    fn test_put_with_null_document() {
        let sdk_handle = create_mock_sdk_handle();
        let data_contract = create_mock_data_contract();
        let identity_public_key = create_mock_identity_public_key();
        let signer = create_mock_signer();

        // No longer need data contract handle
        let identity_public_key_handle =
            Box::into_raw(identity_public_key) as *const crate::types::IdentityPublicKeyHandle;
        let signer_handle = Box::into_raw(signer) as *const SignerHandle;

        let document_type_name = CString::new("testDoc").unwrap();
        let entropy = create_valid_entropy();
        let put_settings = create_put_settings();
        let contract_id = CString::new("4EfA9Jrvv3nnCFdSf7fad59851iiTRZ6Wcu6YVJ4iSeF").unwrap();

        let result = unsafe {
            dash_sdk_document_put_to_platform(
                sdk_handle,
                ptr::null(), // null document
                contract_id.as_ptr(),
                document_type_name.as_ptr(),
                &entropy,
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
            // No longer need to clean up data contract handle
            let _ = Box::from_raw(identity_public_key_handle as *mut IdentityPublicKey);
            let _ = Box::from_raw(signer_handle as *mut crate::signer::VTableSigner);
        }
        destroy_mock_sdk_handle(sdk_handle);
    }

    #[test]
    fn test_put_with_null_entropy() {
        let sdk_handle = create_mock_sdk_handle();
        let document = create_mock_document_with_revision(1);
        let data_contract = create_mock_data_contract();
        let identity_public_key = create_mock_identity_public_key();
        let signer = create_mock_signer();

        let document_handle = Box::into_raw(document) as *const DocumentHandle;
        // No longer need data contract handle
        let identity_public_key_handle =
            Box::into_raw(identity_public_key) as *const crate::types::IdentityPublicKeyHandle;
        let signer_handle = Box::into_raw(signer) as *const SignerHandle;

        let document_type_name = CString::new("testDoc").unwrap();
        let put_settings = create_put_settings();
        let contract_id = CString::new("4EfA9Jrvv3nnCFdSf7fad59851iiTRZ6Wcu6YVJ4iSeF").unwrap();

        let result = unsafe {
            dash_sdk_document_put_to_platform(
                sdk_handle,
                document_handle,
                contract_id.as_ptr(),
                document_type_name.as_ptr(),
                ptr::null(), // null entropy
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
            let _ = Box::from_raw(document_handle as *mut Document);
            // No longer need to clean up data contract handle
            let _ = Box::from_raw(identity_public_key_handle as *mut IdentityPublicKey);
            let _ = Box::from_raw(signer_handle as *mut crate::signer::VTableSigner);
        }
        destroy_mock_sdk_handle(sdk_handle);
    }

    #[test]
    fn test_put_new_document_revision_1() {
        // Test that revision 1 documents use DocumentCreateTransitionBuilder
        let sdk_handle = create_mock_sdk_handle();
        let document = create_mock_document_with_revision(1);
        let data_contract = create_mock_data_contract();
        let identity_public_key = create_mock_identity_public_key();
        let signer = create_mock_signer();

        let document_handle = Box::into_raw(document) as *const DocumentHandle;
        // No longer need data contract handle
        let identity_public_key_handle =
            Box::into_raw(identity_public_key) as *const crate::types::IdentityPublicKeyHandle;
        let signer_handle = Box::into_raw(signer) as *const SignerHandle;

        let document_type_name = CString::new("testDoc").unwrap();
        let entropy = create_valid_entropy();
        let put_settings = create_put_settings();
        let contract_id = CString::new("4EfA9Jrvv3nnCFdSf7fad59851iiTRZ6Wcu6YVJ4iSeF").unwrap();

        let result = unsafe {
            dash_sdk_document_put_to_platform(
                sdk_handle,
                document_handle,
                contract_id.as_ptr(),
                document_type_name.as_ptr(),
                &entropy,
                identity_public_key_handle,
                signer_handle,
                ptr::null(),
                &put_settings,
                ptr::null(),
            )
        };

        // Mock SDK doesn't have trusted provider, so it will fail
        assert!(!result.error.is_null());
        unsafe {
            let error = &*result.error;
            assert_eq!(error.code, DashSDKErrorCode::InternalError);
            let error_msg = CStr::from_ptr(error.message).to_str().unwrap();
            assert!(
                error_msg.contains("trusted context provider"),
                "Expected trusted provider error, got: '{}'",
                error_msg
            );
        }

        // Clean up
        unsafe {
            let _ = Box::from_raw(document_handle as *mut Document);
            // No longer need to clean up data contract handle
            let _ = Box::from_raw(identity_public_key_handle as *mut IdentityPublicKey);
            let _ = Box::from_raw(signer_handle as *mut crate::signer::VTableSigner);
        }
        destroy_mock_sdk_handle(sdk_handle);
    }

    #[test]
    fn test_put_existing_document_revision_2() {
        // Test that revision > 1 documents use DocumentReplaceTransitionBuilder
        let sdk_handle = create_mock_sdk_handle();
        let document = create_mock_document_with_revision(2);
        let data_contract = create_mock_data_contract();
        let identity_public_key = create_mock_identity_public_key();
        let signer = create_mock_signer();

        let document_handle = Box::into_raw(document) as *const DocumentHandle;
        // No longer need data contract handle
        let identity_public_key_handle =
            Box::into_raw(identity_public_key) as *const crate::types::IdentityPublicKeyHandle;
        let signer_handle = Box::into_raw(signer) as *const SignerHandle;

        let document_type_name = CString::new("testDoc").unwrap();
        let entropy = create_valid_entropy();
        let put_settings = create_put_settings();
        let contract_id = CString::new("4EfA9Jrvv3nnCFdSf7fad59851iiTRZ6Wcu6YVJ4iSeF").unwrap();

        let result = unsafe {
            dash_sdk_document_put_to_platform(
                sdk_handle,
                document_handle,
                contract_id.as_ptr(),
                document_type_name.as_ptr(),
                &entropy,
                identity_public_key_handle,
                signer_handle,
                ptr::null(),
                &put_settings,
                ptr::null(),
            )
        };

        // Mock SDK doesn't have trusted provider, so it will fail
        assert!(!result.error.is_null());
        unsafe {
            let error = &*result.error;
            assert_eq!(error.code, DashSDKErrorCode::InternalError);
            let error_msg = CStr::from_ptr(error.message).to_str().unwrap();
            assert!(
                error_msg.contains("trusted context provider"),
                "Expected trusted provider error, got: '{}'",
                error_msg
            );
        }

        // Clean up
        unsafe {
            let _ = Box::from_raw(document_handle as *mut Document);
            // No longer need to clean up data contract handle
            let _ = Box::from_raw(identity_public_key_handle as *mut IdentityPublicKey);
            let _ = Box::from_raw(signer_handle as *mut crate::signer::VTableSigner);
        }
        destroy_mock_sdk_handle(sdk_handle);
    }

    #[test]
    fn test_put_and_wait_with_null_parameters() {
        let sdk_handle = create_mock_sdk_handle();
        let document = create_mock_document_with_revision(1);
        let data_contract = create_mock_data_contract();
        let identity_public_key = create_mock_identity_public_key();
        let signer = create_mock_signer();

        let document_handle = Box::into_raw(document) as *const DocumentHandle;
        // No longer need data contract handle
        let identity_public_key_handle =
            Box::into_raw(identity_public_key) as *const crate::types::IdentityPublicKeyHandle;
        let signer_handle = Box::into_raw(signer) as *const SignerHandle;

        let document_type_name = CString::new("testDoc").unwrap();
        let entropy = create_valid_entropy();
        let put_settings = create_put_settings();
        let contract_id = CString::new("4EfA9Jrvv3nnCFdSf7fad59851iiTRZ6Wcu6YVJ4iSeF").unwrap();

        // Test with null SDK handle
        let result = unsafe {
            dash_sdk_document_put_to_platform_and_wait(
                ptr::null_mut(),
                document_handle,
                contract_id.as_ptr(),
                document_type_name.as_ptr(),
                &entropy,
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
            let _ = Box::from_raw(document_handle as *mut Document);
            // No longer need to clean up data contract handle
            let _ = Box::from_raw(identity_public_key_handle as *mut IdentityPublicKey);
            let _ = Box::from_raw(signer_handle as *mut crate::signer::VTableSigner);
        }
        destroy_mock_sdk_handle(sdk_handle);
    }
}
