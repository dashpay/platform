//! Document price update operations

use crate::document::helpers::{
    convert_state_transition_creation_options, convert_token_payment_info,
};
use crate::sdk::SDKWrapper;
use crate::types::{
    DashSDKPutSettings, DashSDKResultDataType, DashSDKStateTransitionCreationOptions,
    DashSDKTokenPaymentInfo, DocumentHandle, SDKHandle, SignerHandle,
};
use crate::{DashSDKError, DashSDKErrorCode, DashSDKResult, FFIError};
use dash_sdk::dpp::document::document_methods::DocumentMethodsV0;
use dash_sdk::dpp::document::Document;
use dash_sdk::dpp::fee::Credits;
use dash_sdk::dpp::platform_value::string_encoding::Encoding;
use dash_sdk::dpp::prelude::{Identifier, UserFeeIncrease};
use dash_sdk::platform::documents::transitions::DocumentSetPriceTransitionBuilder;
use dash_sdk::platform::IdentityPublicKey;
use drive_proof_verifier::ContextProvider;
use std::ffi::CStr;
use std::os::raw::c_char;

/// Update document price (broadcast state transition)
///
/// # Safety
/// - `sdk_handle` must be a valid, non-null pointer to an initialized `SDKHandle`.
/// - `document_handle`, `data_contract_id`, `document_type_name`, `identity_public_key_handle`, and `signer_handle`
///   must be valid, non-null pointers. `data_contract_id` and `document_type_name` must point to NUL-terminated C strings.
/// - Optional pointers (`token_payment_info`, `put_settings`, `state_transition_creation_options`) may be null; when non-null they must be valid.
/// - On success, the result may contain heap-allocated data that must be freed using SDK-provided routines.
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_document_update_price_of_document(
    sdk_handle: *mut SDKHandle,
    document_handle: *const DocumentHandle,
    data_contract_id: *const c_char,
    document_type_name: *const c_char,
    price: u64,
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
    let signer = &*(signer_handle as *const crate::signer::VTableSigner);

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

    let result: Result<Vec<u8>, FFIError> = wrapper.runtime.block_on(async {
        // Parse contract ID (base58 encoded)
        let contract_id = Identifier::from_string(contract_id_str, Encoding::Base58)
            .map_err(|e| FFIError::InternalError(format!("Invalid contract ID: {}", e)))?;

        // Clone the document and bump its revision
        let mut document_to_transfer = document.clone();
        document_to_transfer.increment_revision().map_err(|e| {
            FFIError::InternalError(format!("Failed to increment document revision: {}", e))
        })?;

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

        // Use the new DocumentSetPriceTransitionBuilder
        let mut builder = DocumentSetPriceTransitionBuilder::new(
            data_contract.clone(),
            document_type_name_str.to_string(),
            document_to_transfer,
            price as Credits,
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
                FFIError::InternalError(format!("Failed to create set price transition: {}", e))
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

/// Update document price and wait for confirmation (broadcast state transition and wait for response)
///
/// # Safety
/// - Same requirements as `dash_sdk_document_update_price_of_document` regarding pointer validity and lifetimes.
/// - The function may block while waiting for confirmation; input pointers must remain valid throughout.
/// - On success, the result may contain heap-allocated data that must be freed using SDK-provided routines.
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_document_update_price_of_document_and_wait(
    sdk_handle: *mut SDKHandle,
    document_handle: *const DocumentHandle,
    data_contract_id: *const c_char,
    document_type_name: *const c_char,
    price: u64,
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
    let signer = &*(signer_handle as *const crate::signer::VTableSigner);

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

    let result: Result<Document, FFIError> = wrapper.runtime.block_on(async {
        // Parse contract ID (base58 encoded)
        let contract_id = Identifier::from_string(contract_id_str, Encoding::Base58)
            .map_err(|e| FFIError::InternalError(format!("Invalid contract ID: {}", e)))?;

        // Clone the document and bump its revision
        let mut document_to_transfer = document.clone();
        document_to_transfer.increment_revision().map_err(|e| {
            FFIError::InternalError(format!("Failed to increment document revision: {}", e))
        })?;

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

        // Use the new DocumentSetPriceTransitionBuilder with SDK method
        let mut builder = DocumentSetPriceTransitionBuilder::new(
            data_contract.clone(),
            document_type_name_str.to_string(),
            document_to_transfer,
            price as Credits,
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
            .document_set_price(builder, identity_public_key, signer)
            .await
            .map_err(|e| {
                FFIError::InternalError(format!("Failed to update document price and wait: {}", e))
            })?;

        let dash_sdk::platform::documents::transitions::DocumentSetPriceResult::Document(
            updated_document,
        ) = result;

        Ok(updated_document)
    });

    match result {
        Ok(updated_document) => {
            let handle = Box::into_raw(Box::new(updated_document)) as *mut DocumentHandle;
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

    // Helper function to create a mock document with price
    fn create_mock_document() -> Box<Document> {
        let id = Identifier::from_bytes(&[2u8; 32]).unwrap();
        let owner_id = Identifier::from_bytes(&[1u8; 32]).unwrap();

        let mut properties = BTreeMap::new();
        properties.insert(
            "name".to_string(),
            Value::Text("Priced Document".to_string()),
        );
        properties.insert("price".to_string(), Value::U64(1000));

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
            creator_id: None,
        });

        Box::new(document)
    }

    #[test]
    fn test_update_price_with_null_sdk_handle() {
        let document = create_mock_document();
        let identity_public_key = create_mock_identity_public_key();
        let signer = create_mock_signer();

        let document_handle = Box::into_raw(document) as *const DocumentHandle;
        let identity_public_key_handle =
            Box::into_raw(identity_public_key) as *const crate::types::IdentityPublicKeyHandle;
        let signer_handle = Box::into_raw(signer) as *const SignerHandle;

        let contract_id = CString::new("GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec").unwrap();
        let document_type_name = CString::new("testDoc").unwrap();
        let new_price = 2000u64;
        let put_settings = create_put_settings();

        let result = unsafe {
            dash_sdk_document_update_price_of_document(
                ptr::null_mut(), // null SDK handle
                document_handle,
                contract_id.as_ptr(),
                document_type_name.as_ptr(),
                new_price,
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
    fn test_update_price_with_null_document() {
        let sdk_handle = create_mock_sdk_handle();
        let identity_public_key = create_mock_identity_public_key();
        let signer = create_mock_signer();

        let identity_public_key_handle =
            Box::into_raw(identity_public_key) as *const crate::types::IdentityPublicKeyHandle;
        let signer_handle = Box::into_raw(signer) as *const SignerHandle;

        let contract_id = CString::new("GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec").unwrap();
        let document_type_name = CString::new("testDoc").unwrap();
        let new_price = 2000u64;
        let put_settings = create_put_settings();

        let result = unsafe {
            dash_sdk_document_update_price_of_document(
                sdk_handle,
                ptr::null(), // null document
                contract_id.as_ptr(),
                document_type_name.as_ptr(),
                new_price,
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
    fn test_update_price_with_null_data_contract() {
        let sdk_handle = create_mock_sdk_handle();
        let document = create_mock_document();
        let identity_public_key = create_mock_identity_public_key();
        let signer = create_mock_signer();

        let document_handle = Box::into_raw(document) as *const DocumentHandle;
        let identity_public_key_handle =
            Box::into_raw(identity_public_key) as *const crate::types::IdentityPublicKeyHandle;
        let signer_handle = Box::into_raw(signer) as *const SignerHandle;

        let document_type_name = CString::new("testDoc").unwrap();
        let new_price = 2000u64;
        let put_settings = create_put_settings();

        let result = unsafe {
            dash_sdk_document_update_price_of_document(
                sdk_handle,
                document_handle,
                ptr::null(), // null data contract
                document_type_name.as_ptr(),
                new_price,
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
            let _ = Box::from_raw(signer_handle as *mut crate::signer::VTableSigner);
        }
        destroy_mock_sdk_handle(sdk_handle);
    }

    #[test]
    fn test_update_price_with_null_document_type_name() {
        let sdk_handle = create_mock_sdk_handle();
        let document = create_mock_document();
        let identity_public_key = create_mock_identity_public_key();
        let signer = create_mock_signer();

        let document_handle = Box::into_raw(document) as *const DocumentHandle;
        let identity_public_key_handle =
            Box::into_raw(identity_public_key) as *const crate::types::IdentityPublicKeyHandle;
        let signer_handle = Box::into_raw(signer) as *const SignerHandle;

        let contract_id = CString::new("GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec").unwrap();
        let new_price = 2000u64;
        let put_settings = create_put_settings();

        let result = unsafe {
            dash_sdk_document_update_price_of_document(
                sdk_handle,
                document_handle,
                contract_id.as_ptr(),
                ptr::null(), // null document type name
                new_price,
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
            let _ = Box::from_raw(signer_handle as *mut crate::signer::VTableSigner);
        }
        destroy_mock_sdk_handle(sdk_handle);
    }

    #[test]
    fn test_update_price_with_zero_price() {
        let sdk_handle = create_mock_sdk_handle();
        let document = create_mock_document();
        let identity_public_key = create_mock_identity_public_key();
        let signer = create_mock_signer();

        let document_handle = Box::into_raw(document) as *const DocumentHandle;
        let identity_public_key_handle =
            Box::into_raw(identity_public_key) as *const crate::types::IdentityPublicKeyHandle;
        let signer_handle = Box::into_raw(signer) as *const SignerHandle;

        let contract_id = CString::new("GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec").unwrap();
        let document_type_name = CString::new("testDoc").unwrap();
        let new_price = 0u64; // Zero price
        let put_settings = create_put_settings();

        let result = unsafe {
            dash_sdk_document_update_price_of_document(
                sdk_handle,
                document_handle,
                contract_id.as_ptr(),
                document_type_name.as_ptr(),
                new_price,
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
            let _ = Box::from_raw(identity_public_key_handle as *mut IdentityPublicKey);
            let _ = Box::from_raw(signer_handle as *mut crate::signer::VTableSigner);
        }
        destroy_mock_sdk_handle(sdk_handle);
    }

    #[test]
    fn test_update_price_with_max_price() {
        let sdk_handle = create_mock_sdk_handle();
        let document = create_mock_document();
        let identity_public_key = create_mock_identity_public_key();
        let signer = create_mock_signer();

        let document_handle = Box::into_raw(document) as *const DocumentHandle;
        let identity_public_key_handle =
            Box::into_raw(identity_public_key) as *const crate::types::IdentityPublicKeyHandle;
        let signer_handle = Box::into_raw(signer) as *const SignerHandle;

        let contract_id = CString::new("GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec").unwrap();
        let document_type_name = CString::new("testDoc").unwrap();
        let new_price = u64::MAX; // Maximum price
        let put_settings = create_put_settings();

        let result = unsafe {
            dash_sdk_document_update_price_of_document(
                sdk_handle,
                document_handle,
                contract_id.as_ptr(),
                document_type_name.as_ptr(),
                new_price,
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
            let _ = Box::from_raw(identity_public_key_handle as *mut IdentityPublicKey);
            let _ = Box::from_raw(signer_handle as *mut crate::signer::VTableSigner);
        }
        destroy_mock_sdk_handle(sdk_handle);
    }

    #[test]
    fn test_update_price_and_wait_with_null_parameters() {
        let sdk_handle = create_mock_sdk_handle();
        let document = create_mock_document();
        let identity_public_key = create_mock_identity_public_key();
        let signer = create_mock_signer();

        let document_handle = Box::into_raw(document) as *const DocumentHandle;
        let identity_public_key_handle =
            Box::into_raw(identity_public_key) as *const crate::types::IdentityPublicKeyHandle;
        let signer_handle = Box::into_raw(signer) as *const SignerHandle;

        let contract_id = CString::new("GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec").unwrap();
        let document_type_name = CString::new("testDoc").unwrap();
        let new_price = 2000u64;
        let put_settings = create_put_settings();

        // Test with null SDK handle
        let result = unsafe {
            dash_sdk_document_update_price_of_document_and_wait(
                ptr::null_mut(),
                document_handle,
                contract_id.as_ptr(),
                document_type_name.as_ptr(),
                new_price,
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
            let _ = Box::from_raw(signer_handle as *mut crate::signer::VTableSigner);
        }
        destroy_mock_sdk_handle(sdk_handle);
    }
}
