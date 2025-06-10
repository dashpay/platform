//! Document replacement operations

use dash_sdk::dpp::document::Document;
use dash_sdk::dpp::prelude::{DataContract, UserFeeIncrease};
use dash_sdk::platform::documents::transitions::DocumentReplaceTransitionBuilder;
use dash_sdk::platform::IdentityPublicKey;
use std::ffi::CStr;
use std::os::raw::c_char;
use std::sync::Arc;

use crate::document::helpers::{
    convert_state_transition_creation_options, convert_token_payment_info,
};
use crate::sdk::SDKWrapper;
use crate::types::{
    DashSDKPutSettings, DashSDKResultDataType, DashSDKStateTransitionCreationOptions,
    DashSDKTokenPaymentInfo, DataContractHandle, DocumentHandle, SDKHandle, SignerHandle,
};
use crate::{DashSDKError, DashSDKErrorCode, DashSDKResult, FFIError};

/// Replace document on platform (broadcast state transition)
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_document_replace_on_platform(
    sdk_handle: *mut SDKHandle,
    document_handle: *const DocumentHandle,
    data_contract_handle: *const DataContractHandle,
    document_type_name: *const c_char,
    identity_public_key_handle: *const crate::types::IdentityPublicKeyHandle,
    signer_handle: *const SignerHandle,
    token_payment_info: *const DashSDKTokenPaymentInfo,
    put_settings: *const DashSDKPutSettings,
    state_transition_creation_options: *const DashSDKStateTransitionCreationOptions,
) -> DashSDKResult {
    // Validate required parameters
    if sdk_handle.is_null()
        || document_handle.is_null()
        || data_contract_handle.is_null()
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
    let document = &*(document_handle as *const Document);
    let data_contract = &*(data_contract_handle as *const DataContract);
    let identity_public_key = &*(identity_public_key_handle as *const IdentityPublicKey);
    let signer = &*(signer_handle as *const crate::signer::IOSSigner);

    let document_type_name_str = match CStr::from_ptr(document_type_name).to_str() {
        Ok(s) => s,
        Err(e) => return DashSDKResult::error(FFIError::from(e).into()),
    };

    let result: Result<Vec<u8>, FFIError> = wrapper.runtime.block_on(async {
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

        // Use the new DocumentReplaceTransitionBuilder
        let mut builder = DocumentReplaceTransitionBuilder::new(
            Arc::new(data_contract.clone()),
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

        let state_transition = builder
            .sign(
                &wrapper.sdk,
                identity_public_key,
                signer,
                wrapper.sdk.version(),
            )
            .await
            .map_err(|e| {
                FFIError::InternalError(format!("Failed to create replace transition: {}", e))
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

/// Replace document on platform and wait for confirmation (broadcast state transition and wait for response)
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_document_replace_on_platform_and_wait(
    sdk_handle: *mut SDKHandle,
    document_handle: *const DocumentHandle,
    data_contract_handle: *const DataContractHandle,
    document_type_name: *const c_char,
    identity_public_key_handle: *const crate::types::IdentityPublicKeyHandle,
    signer_handle: *const SignerHandle,
    token_payment_info: *const DashSDKTokenPaymentInfo,
    put_settings: *const DashSDKPutSettings,
    state_transition_creation_options: *const DashSDKStateTransitionCreationOptions,
) -> DashSDKResult {
    // Validate required parameters
    if sdk_handle.is_null()
        || document_handle.is_null()
        || data_contract_handle.is_null()
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
    let document = &*(document_handle as *const Document);
    let data_contract = &*(data_contract_handle as *const DataContract);
    let identity_public_key = &*(identity_public_key_handle as *const IdentityPublicKey);
    let signer = &*(signer_handle as *const crate::signer::IOSSigner);

    let document_type_name_str = match CStr::from_ptr(document_type_name).to_str() {
        Ok(s) => s,
        Err(e) => return DashSDKResult::error(FFIError::from(e).into()),
    };

    let result: Result<Document, FFIError> = wrapper.runtime.block_on(async {
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

        // Use the new DocumentReplaceTransitionBuilder with SDK method
        let mut builder = DocumentReplaceTransitionBuilder::new(
            Arc::new(data_contract.clone()),
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
            .document_replace(builder, identity_public_key, signer)
            .await
            .map_err(|e| {
                FFIError::InternalError(format!("Failed to replace document and wait: {}", e))
            })?;

        let replaced_document = match result {
            dash_sdk::platform::documents::transitions::DocumentReplaceResult::Document(doc) => doc,
        };

        Ok(replaced_document)
    });

    match result {
        Ok(replaced_document) => {
            let handle = Box::into_raw(Box::new(replaced_document)) as *mut DocumentHandle;
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
    use dash_sdk::dpp::prelude::Identifier;

    use std::collections::BTreeMap;
    use std::ffi::{CStr, CString};
    use std::ptr;

    // Helper function to create a mock document for replacement (revision > 1)
    fn create_mock_document_for_replace() -> Box<Document> {
        let id = Identifier::from_bytes(&[2u8; 32]).unwrap();
        let owner_id = Identifier::from_bytes(&[1u8; 32]).unwrap();

        let mut properties = BTreeMap::new();
        properties.insert(
            "name".to_string(),
            Value::Text("Updated Document".to_string()),
        );
        properties.insert("age".to_string(), Value::U64(25));

        let document = Document::V0(DocumentV0 {
            id,
            owner_id,
            properties: properties,
            revision: Some(2), // Revision > 1 for replace
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
    fn test_replace_with_null_sdk_handle() {
        let document = create_mock_document_for_replace();
        let data_contract = create_mock_data_contract();
        let identity_public_key = create_mock_identity_public_key();
        let signer = create_mock_signer();

        let document_handle = Box::into_raw(document) as *const DocumentHandle;
        let data_contract_handle = Box::into_raw(data_contract) as *const DataContractHandle;
        let identity_public_key_handle =
            Box::into_raw(identity_public_key) as *const crate::types::IdentityPublicKeyHandle;
        let signer_handle = Box::into_raw(signer) as *const SignerHandle;

        let document_type_name = CString::new("testDoc").unwrap();
        let put_settings = create_put_settings();

        let result = unsafe {
            dash_sdk_document_replace_on_platform(
                ptr::null_mut(), // null SDK handle
                document_handle,
                data_contract_handle,
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
            let _ = Box::from_raw(document_handle as *mut Document);
            let _ = Box::from_raw(data_contract_handle as *mut DataContract);
            let _ = Box::from_raw(identity_public_key_handle as *mut IdentityPublicKey);
            let _ = Box::from_raw(signer_handle as *mut crate::signer::IOSSigner);
        }
    }

    #[test]
    fn test_replace_with_null_document() {
        let sdk_handle = create_mock_sdk_handle();
        let data_contract = create_mock_data_contract();
        let identity_public_key = create_mock_identity_public_key();
        let signer = create_mock_signer();

        let data_contract_handle = Box::into_raw(data_contract) as *const DataContractHandle;
        let identity_public_key_handle =
            Box::into_raw(identity_public_key) as *const crate::types::IdentityPublicKeyHandle;
        let signer_handle = Box::into_raw(signer) as *const SignerHandle;

        let document_type_name = CString::new("testDoc").unwrap();
        let put_settings = create_put_settings();

        let result = unsafe {
            dash_sdk_document_replace_on_platform(
                sdk_handle,
                ptr::null(), // null document
                data_contract_handle,
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
            let _ = Box::from_raw(data_contract_handle as *mut DataContract);
            let _ = Box::from_raw(identity_public_key_handle as *mut IdentityPublicKey);
            let _ = Box::from_raw(signer_handle as *mut crate::signer::IOSSigner);
        }
        destroy_mock_sdk_handle(sdk_handle);
    }

    #[test]
    fn test_replace_with_null_data_contract() {
        let sdk_handle = create_mock_sdk_handle();
        let document = create_mock_document_for_replace();
        let identity_public_key = create_mock_identity_public_key();
        let signer = create_mock_signer();

        let document_handle = Box::into_raw(document) as *const DocumentHandle;
        let identity_public_key_handle =
            Box::into_raw(identity_public_key) as *const crate::types::IdentityPublicKeyHandle;
        let signer_handle = Box::into_raw(signer) as *const SignerHandle;

        let document_type_name = CString::new("testDoc").unwrap();
        let put_settings = create_put_settings();

        let result = unsafe {
            dash_sdk_document_replace_on_platform(
                sdk_handle,
                document_handle,
                ptr::null(), // null data contract
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
            let _ = Box::from_raw(document_handle as *mut Document);
            let _ = Box::from_raw(identity_public_key_handle as *mut IdentityPublicKey);
            let _ = Box::from_raw(signer_handle as *mut crate::signer::IOSSigner);
        }
        destroy_mock_sdk_handle(sdk_handle);
    }

    #[test]
    fn test_replace_with_null_document_type_name() {
        let sdk_handle = create_mock_sdk_handle();
        let document = create_mock_document_for_replace();
        let data_contract = create_mock_data_contract();
        let identity_public_key = create_mock_identity_public_key();
        let signer = create_mock_signer();

        let document_handle = Box::into_raw(document) as *const DocumentHandle;
        let data_contract_handle = Box::into_raw(data_contract) as *const DataContractHandle;
        let identity_public_key_handle =
            Box::into_raw(identity_public_key) as *const crate::types::IdentityPublicKeyHandle;
        let signer_handle = Box::into_raw(signer) as *const SignerHandle;

        let put_settings = create_put_settings();

        let result = unsafe {
            dash_sdk_document_replace_on_platform(
                sdk_handle,
                document_handle,
                data_contract_handle,
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
            let _ = Box::from_raw(document_handle as *mut Document);
            let _ = Box::from_raw(data_contract_handle as *mut DataContract);
            let _ = Box::from_raw(identity_public_key_handle as *mut IdentityPublicKey);
            let _ = Box::from_raw(signer_handle as *mut crate::signer::IOSSigner);
        }
        destroy_mock_sdk_handle(sdk_handle);
    }

    #[test]
    fn test_replace_with_null_identity_public_key() {
        let sdk_handle = create_mock_sdk_handle();
        let document = create_mock_document_for_replace();
        let data_contract = create_mock_data_contract();
        let signer = create_mock_signer();

        let document_handle = Box::into_raw(document) as *const DocumentHandle;
        let data_contract_handle = Box::into_raw(data_contract) as *const DataContractHandle;
        let signer_handle = Box::into_raw(signer) as *const SignerHandle;

        let document_type_name = CString::new("testDoc").unwrap();
        let put_settings = create_put_settings();

        let result = unsafe {
            dash_sdk_document_replace_on_platform(
                sdk_handle,
                document_handle,
                data_contract_handle,
                document_type_name.as_ptr(),
                ptr::null(), // null identity public key
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
            let _ = Box::from_raw(data_contract_handle as *mut DataContract);
            let _ = Box::from_raw(signer_handle as *mut crate::signer::IOSSigner);
        }
        destroy_mock_sdk_handle(sdk_handle);
    }

    #[test]
    fn test_replace_with_null_signer() {
        let sdk_handle = create_mock_sdk_handle();
        let document = create_mock_document_for_replace();
        let data_contract = create_mock_data_contract();
        let identity_public_key = create_mock_identity_public_key();

        let document_handle = Box::into_raw(document) as *const DocumentHandle;
        let data_contract_handle = Box::into_raw(data_contract) as *const DataContractHandle;
        let identity_public_key_handle =
            Box::into_raw(identity_public_key) as *const crate::types::IdentityPublicKeyHandle;

        let document_type_name = CString::new("testDoc").unwrap();
        let put_settings = create_put_settings();

        let result = unsafe {
            dash_sdk_document_replace_on_platform(
                sdk_handle,
                document_handle,
                data_contract_handle,
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
            let _ = Box::from_raw(document_handle as *mut Document);
            let _ = Box::from_raw(data_contract_handle as *mut DataContract);
            let _ = Box::from_raw(identity_public_key_handle as *mut IdentityPublicKey);
        }
        destroy_mock_sdk_handle(sdk_handle);
    }

    #[test]
    fn test_replace_success() {
        let sdk_handle = create_mock_sdk_handle();
        let document = create_mock_document_for_replace();
        let data_contract = create_mock_data_contract();
        let identity_public_key = create_mock_identity_public_key();
        let signer = create_mock_signer();

        let document_handle = Box::into_raw(document) as *const DocumentHandle;
        let data_contract_handle = Box::into_raw(data_contract) as *const DataContractHandle;
        let identity_public_key_handle =
            Box::into_raw(identity_public_key) as *const crate::types::IdentityPublicKeyHandle;
        let signer_handle = Box::into_raw(signer) as *const SignerHandle;

        let document_type_name = CString::new("testDoc").unwrap();
        let put_settings = create_put_settings();

        let result = unsafe {
            dash_sdk_document_replace_on_platform(
                sdk_handle,
                document_handle,
                data_contract_handle,
                document_type_name.as_ptr(),
                identity_public_key_handle,
                signer_handle,
                ptr::null(),
                &put_settings,
                ptr::null(),
            )
        };

        // Should succeed with serialized data
        assert!(result.error.is_null());
        assert!(!result.data.is_null());

        // Clean up
        unsafe {
            let _ = Box::from_raw(document_handle as *mut Document);
            let _ = Box::from_raw(data_contract_handle as *mut DataContract);
            let _ = Box::from_raw(identity_public_key_handle as *mut IdentityPublicKey);
            let _ = Box::from_raw(signer_handle as *mut crate::signer::IOSSigner);
        }
        destroy_mock_sdk_handle(sdk_handle);
    }

    #[test]
    fn test_replace_and_wait_with_null_parameters() {
        let sdk_handle = create_mock_sdk_handle();
        let document = create_mock_document_for_replace();
        let data_contract = create_mock_data_contract();
        let identity_public_key = create_mock_identity_public_key();
        let signer = create_mock_signer();

        let document_handle = Box::into_raw(document) as *const DocumentHandle;
        let data_contract_handle = Box::into_raw(data_contract) as *const DataContractHandle;
        let identity_public_key_handle =
            Box::into_raw(identity_public_key) as *const crate::types::IdentityPublicKeyHandle;
        let signer_handle = Box::into_raw(signer) as *const SignerHandle;

        let document_type_name = CString::new("testDoc").unwrap();
        let put_settings = create_put_settings();

        // Test with null SDK handle
        let result = unsafe {
            dash_sdk_document_replace_on_platform_and_wait(
                ptr::null_mut(),
                document_handle,
                data_contract_handle,
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
            let _ = Box::from_raw(document_handle as *mut Document);
            let _ = Box::from_raw(data_contract_handle as *mut DataContract);
            let _ = Box::from_raw(identity_public_key_handle as *mut IdentityPublicKey);
            let _ = Box::from_raw(signer_handle as *mut crate::signer::IOSSigner);
        }
        destroy_mock_sdk_handle(sdk_handle);
    }
}
