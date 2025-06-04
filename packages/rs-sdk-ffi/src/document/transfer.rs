//! Document transfer operations

use dash_sdk::dpp::data_contract::accessors::v0::DataContractV0Getters;
use dash_sdk::dpp::document::Document;
use dash_sdk::dpp::platform_value::string_encoding::Encoding;
use dash_sdk::dpp::prelude::{DataContract, Identifier, UserFeeIncrease};
use dash_sdk::platform::documents::transitions::DocumentTransferTransitionBuilder;
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

/// Transfer document to another identity
///
/// # Parameters
/// - `document_handle`: Handle to the document to transfer
/// - `recipient_id`: Base58-encoded ID of the recipient identity
/// - `data_contract_handle`: Handle to the data contract
/// - `document_type_name`: Name of the document type
/// - `identity_public_key_handle`: Public key for signing
/// - `signer_handle`: Cryptographic signer
/// - `token_payment_info`: Optional token payment information (can be null for defaults)
/// - `put_settings`: Optional settings for the operation (can be null for defaults)
///
/// # Returns
/// Serialized state transition on success
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_document_transfer_to_identity(
    sdk_handle: *mut SDKHandle,
    document_handle: *const DocumentHandle,
    recipient_id: *const c_char,
    data_contract_handle: *const DataContractHandle,
    document_type_name: *const c_char,
    identity_public_key_handle: *const crate::types::IdentityPublicKeyHandle,
    signer_handle: *const SignerHandle,
    token_payment_info: *const DashSDKTokenPaymentInfo,
    put_settings: *const DashSDKPutSettings,
    state_transition_creation_options: *const DashSDKStateTransitionCreationOptions,
) -> DashSDKResult {
    // Validate parameters
    if sdk_handle.is_null()
        || document_handle.is_null()
        || recipient_id.is_null()
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

    let recipient_id_str = match CStr::from_ptr(recipient_id).to_str() {
        Ok(s) => s,
        Err(e) => return DashSDKResult::error(FFIError::from(e).into()),
    };

    let document_type_name_str = match CStr::from_ptr(document_type_name).to_str() {
        Ok(s) => s,
        Err(e) => return DashSDKResult::error(FFIError::from(e).into()),
    };

    let recipient_identifier = match Identifier::from_string(recipient_id_str, Encoding::Base58) {
        Ok(id) => id,
        Err(e) => {
            return DashSDKResult::error(DashSDKError::new(
                DashSDKErrorCode::InvalidParameter,
                format!("Invalid recipient ID: {}", e),
            ))
        }
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

        // Get document type from data contract
        let _document_type = data_contract
            .document_type_for_name(document_type_name_str)
            .map_err(|e| FFIError::InternalError(format!("Failed to get document type: {}", e)))?;

        let _document_type_owned = _document_type.to_owned_document_type();

        // Use the new DocumentTransferTransitionBuilder
        let mut builder = DocumentTransferTransitionBuilder::new(
            Arc::new(data_contract.clone()),
            document_type_name_str.to_string(),
            document.clone(),
            recipient_identifier,
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
                FFIError::InternalError(format!("Failed to create transfer transition: {}", e))
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

/// Transfer document to another identity and wait for confirmation
///
/// # Parameters
/// - `document_handle`: Handle to the document to transfer
/// - `recipient_id`: Base58-encoded ID of the recipient identity
/// - `data_contract_handle`: Handle to the data contract
/// - `document_type_name`: Name of the document type
/// - `identity_public_key_handle`: Public key for signing
/// - `signer_handle`: Cryptographic signer
/// - `token_payment_info`: Optional token payment information (can be null for defaults)
/// - `put_settings`: Optional settings for the operation (can be null for defaults)
///
/// # Returns
/// Handle to the transferred document on success
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_document_transfer_to_identity_and_wait(
    sdk_handle: *mut SDKHandle,
    document_handle: *const DocumentHandle,
    recipient_id: *const c_char,
    data_contract_handle: *const DataContractHandle,
    document_type_name: *const c_char,
    identity_public_key_handle: *const crate::types::IdentityPublicKeyHandle,
    signer_handle: *const SignerHandle,
    token_payment_info: *const DashSDKTokenPaymentInfo,
    put_settings: *const DashSDKPutSettings,
    state_transition_creation_options: *const DashSDKStateTransitionCreationOptions,
) -> DashSDKResult {
    // Validate parameters
    if sdk_handle.is_null()
        || document_handle.is_null()
        || recipient_id.is_null()
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

    let recipient_id_str = match CStr::from_ptr(recipient_id).to_str() {
        Ok(s) => s,
        Err(e) => return DashSDKResult::error(FFIError::from(e).into()),
    };

    let document_type_name_str = match CStr::from_ptr(document_type_name).to_str() {
        Ok(s) => s,
        Err(e) => return DashSDKResult::error(FFIError::from(e).into()),
    };

    let recipient_identifier = match Identifier::from_string(recipient_id_str, Encoding::Base58) {
        Ok(id) => id,
        Err(e) => {
            return DashSDKResult::error(DashSDKError::new(
                DashSDKErrorCode::InvalidParameter,
                format!("Invalid recipient ID: {}", e),
            ))
        }
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

        // Get document type from data contract
        let _document_type = data_contract
            .document_type_for_name(document_type_name_str)
            .map_err(|e| FFIError::InternalError(format!("Failed to get document type: {}", e)))?;

        let _document_type_owned = _document_type.to_owned_document_type();

        // Use the new DocumentTransferTransitionBuilder with SDK method
        let mut builder = DocumentTransferTransitionBuilder::new(
            Arc::new(data_contract.clone()),
            document_type_name_str.to_string(),
            document.clone(),
            recipient_identifier,
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
            .document_transfer(builder, identity_public_key, signer)
            .await
            .map_err(|e| {
                FFIError::InternalError(format!("Failed to transfer document and wait: {}", e))
            })?;

        let transferred_document = match result {
            dash_sdk::platform::documents::transitions::DocumentTransferResult::Document(doc) => {
                doc
            }
        };

        Ok(transferred_document)
    });

    match result {
        Ok(transferred_document) => {
            let handle = Box::into_raw(Box::new(transferred_document)) as *mut DocumentHandle;
            DashSDKResult::success_handle(
                handle as *mut std::os::raw::c_void,
                DashSDKResultDataType::DocumentHandle,
            )
        }
        Err(e) => DashSDKResult::error(e.into()),
    }
}
