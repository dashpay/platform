//! Document put-to-platform operations

use dash_sdk::dpp::document::{Document, DocumentV0Getters};
use dash_sdk::dpp::prelude::{DataContract, UserFeeIncrease};
use dash_sdk::platform::documents::transitions::{
    DocumentCreateTransitionBuilder, DocumentReplaceTransitionBuilder,
};
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

/// Put document to platform (broadcast state transition)
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_document_put_to_platform(
    sdk_handle: *mut SDKHandle,
    document_handle: *const DocumentHandle,
    data_contract_handle: *const DataContractHandle,
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
        || data_contract_handle.is_null()
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
    let data_contract = &*(data_contract_handle as *const DataContract);
    let identity_public_key = &*(identity_public_key_handle as *const IdentityPublicKey);
    let signer = &*(signer_handle as *const crate::signer::IOSSigner);
    let entropy_bytes = *entropy;

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

        // Use the new DocumentCreateTransitionBuilder or DocumentReplaceTransitionBuilder
        let state_transition = if document.revision().unwrap_or(0) == 1 {
            // Create transition for new documents
            let mut builder = DocumentCreateTransitionBuilder::new(
                Arc::new(data_contract.clone()),
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
                    identity_public_key,
                    signer,
                    wrapper.sdk.version(),
                )
                .await
        } else {
            // Replace transition for existing documents
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

            builder
                .sign(
                    &wrapper.sdk,
                    identity_public_key,
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
    data_contract_handle: *const DataContractHandle,
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
        || data_contract_handle.is_null()
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
    let data_contract = &*(data_contract_handle as *const DataContract);
    let identity_public_key = &*(identity_public_key_handle as *const IdentityPublicKey);
    let signer = &*(signer_handle as *const crate::signer::IOSSigner);
    let entropy_bytes = *entropy;

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

        // Use the new builder pattern and SDK methods
        let confirmed_document = if document.revision().unwrap_or(0) == 1 {
            // Create transition for new documents
            let mut builder = DocumentCreateTransitionBuilder::new(
                Arc::new(data_contract.clone()),
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
                .document_create(builder, identity_public_key, signer)
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
                DashSDKResultDataType::DocumentHandle,
            )
        }
        Err(e) => DashSDKResult::error(e.into()),
    }
}
