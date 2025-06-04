//! Document operations

use crate::sdk::SDKWrapper;
use crate::types::{
    DataContractHandle, DocumentHandle, IOSSDKDocumentInfo, IOSSDKGasFeesPaidBy, IOSSDKPutSettings,
    IOSSDKResultDataType, IOSSDKStateTransitionCreationOptions, IOSSDKTokenPaymentInfo,
    IdentityHandle, SDKHandle, SignerHandle,
};
use crate::{FFIError, IOSSDKError, IOSSDKErrorCode, IOSSDKResult};
use dash_sdk::dpp::data_contract::accessors::v0::DataContractV0Getters;
use dash_sdk::dpp::document::{document_factory::DocumentFactory, Document, DocumentV0Getters};
use dash_sdk::dpp::fee::Credits;
use dash_sdk::dpp::identity::accessors::IdentityGettersV0;
use dash_sdk::dpp::platform_value::{string_encoding::Encoding, Value};
use dash_sdk::dpp::prelude::{DataContract, Identifier, Identity, UserFeeIncrease};
use dash_sdk::dpp::state_transition::batch_transition::methods::StateTransitionCreationOptions;
use dash_sdk::dpp::state_transition::StateTransitionSigningOptions;
use dash_sdk::dpp::tokens::gas_fees_paid_by::GasFeesPaidBy;
use dash_sdk::dpp::tokens::token_payment_info::v0::TokenPaymentInfoV0;
use dash_sdk::dpp::tokens::token_payment_info::TokenPaymentInfo;
use dash_sdk::platform::{DocumentQuery, Fetch, IdentityPublicKey};
// FeatureVersion type import will be resolved by the compiler
use dash_sdk::platform::documents::transitions::{
    DocumentCreateTransitionBuilder, DocumentDeleteTransitionBuilder,
    DocumentPurchaseTransitionBuilder, DocumentReplaceTransitionBuilder,
    DocumentSetPriceTransitionBuilder, DocumentTransferTransitionBuilder,
};
use std::collections::BTreeMap;
use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::sync::Arc;

/// Convert FFI GasFeesPaidBy to Rust enum
unsafe fn convert_gas_fees_paid_by(ffi_value: IOSSDKGasFeesPaidBy) -> GasFeesPaidBy {
    match ffi_value {
        IOSSDKGasFeesPaidBy::DocumentOwner => GasFeesPaidBy::DocumentOwner,
        IOSSDKGasFeesPaidBy::ContractOwner => GasFeesPaidBy::ContractOwner,
        IOSSDKGasFeesPaidBy::PreferContractOwner => GasFeesPaidBy::PreferContractOwner,
    }
}

/// Convert FFI TokenPaymentInfo to Rust TokenPaymentInfo
unsafe fn convert_token_payment_info(
    ffi_token_payment_info: *const IOSSDKTokenPaymentInfo,
) -> Result<Option<TokenPaymentInfo>, FFIError> {
    if ffi_token_payment_info.is_null() {
        return Ok(None);
    }

    let token_info = &*ffi_token_payment_info;

    let payment_token_contract_id = if token_info.payment_token_contract_id.is_null() {
        None
    } else {
        let id_bytes = &*token_info.payment_token_contract_id;
        Some(Identifier::from_bytes(id_bytes).map_err(|e| {
            FFIError::InternalError(format!("Invalid payment token contract ID: {}", e))
        })?)
    };

    let token_payment_info_v0 = TokenPaymentInfoV0 {
        payment_token_contract_id,
        token_contract_position: token_info.token_contract_position,
        minimum_token_cost: if token_info.minimum_token_cost == 0 {
            None
        } else {
            Some(token_info.minimum_token_cost)
        },
        maximum_token_cost: if token_info.maximum_token_cost == 0 {
            None
        } else {
            Some(token_info.maximum_token_cost)
        },
        gas_fees_paid_by: convert_gas_fees_paid_by(token_info.gas_fees_paid_by),
    };

    Ok(Some(TokenPaymentInfo::V0(token_payment_info_v0)))
}

/// Convert FFI StateTransitionCreationOptions to Rust StateTransitionCreationOptions
unsafe fn convert_state_transition_creation_options(
    ffi_options: *const IOSSDKStateTransitionCreationOptions,
) -> Option<StateTransitionCreationOptions> {
    if ffi_options.is_null() {
        return None;
    }

    let options = &*ffi_options;

    let signing_options = StateTransitionSigningOptions {
        allow_signing_with_any_security_level: options.allow_signing_with_any_security_level,
        allow_signing_with_any_purpose: options.allow_signing_with_any_purpose,
    };

    Some(StateTransitionCreationOptions {
        signing_options,
        batch_feature_version: if options.batch_feature_version == 0 {
            None
        } else {
            Some(options.batch_feature_version)
        },
        method_feature_version: if options.method_feature_version == 0 {
            None
        } else {
            Some(options.method_feature_version)
        },
        base_feature_version: if options.base_feature_version == 0 {
            None
        } else {
            Some(options.base_feature_version)
        },
    })
}

/// Document creation parameters
#[repr(C)]
pub struct IOSSDKDocumentCreateParams {
    /// Data contract handle
    pub data_contract_handle: *const DataContractHandle,
    /// Document type name
    pub document_type: *const c_char,
    /// Owner identity handle
    pub owner_identity_handle: *const IdentityHandle,
    /// JSON string of document properties
    pub properties_json: *const c_char,
}

/// Document search parameters
#[repr(C)]
pub struct IOSSDKDocumentSearchParams {
    /// Data contract handle
    pub data_contract_handle: *const DataContractHandle,
    /// Document type name
    pub document_type: *const c_char,
    /// JSON string of where clauses (optional)
    pub where_json: *const c_char,
    /// JSON string of order by clauses (optional)
    pub order_by_json: *const c_char,
    /// Limit number of results (0 = default)
    pub limit: u32,
    /// Start from index (for pagination)
    pub start_at: u32,
}

/// Create a new document
#[no_mangle]
pub unsafe extern "C" fn ios_sdk_document_create(
    sdk_handle: *mut SDKHandle,
    params: *const IOSSDKDocumentCreateParams,
) -> IOSSDKResult {
    if sdk_handle.is_null() || params.is_null() {
        return IOSSDKResult::error(IOSSDKError::new(
            IOSSDKErrorCode::InvalidParameter,
            "SDK handle or params is null".to_string(),
        ));
    }

    let params = &*params;
    if params.data_contract_handle.is_null()
        || params.document_type.is_null()
        || params.owner_identity_handle.is_null()
        || params.properties_json.is_null()
    {
        return IOSSDKResult::error(IOSSDKError::new(
            IOSSDKErrorCode::InvalidParameter,
            "Required parameter is null".to_string(),
        ));
    }

    let wrapper = &mut *(sdk_handle as *mut SDKWrapper);
    let data_contract = &*(params.data_contract_handle as *const DataContract);
    let identity = &*(params.owner_identity_handle as *const Identity);

    let document_type = match CStr::from_ptr(params.document_type).to_str() {
        Ok(s) => s,
        Err(e) => return IOSSDKResult::error(FFIError::from(e).into()),
    };

    let properties_str = match CStr::from_ptr(params.properties_json).to_str() {
        Ok(s) => s,
        Err(e) => return IOSSDKResult::error(FFIError::from(e).into()),
    };

    // Parse properties JSON
    let properties_value: serde_json::Value = match serde_json::from_str(properties_str) {
        Ok(v) => v,
        Err(e) => {
            return IOSSDKResult::error(IOSSDKError::new(
                IOSSDKErrorCode::InvalidParameter,
                format!("Invalid properties JSON: {}", e),
            ))
        }
    };

    // Convert JSON to platform Value
    let properties = match serde_json::from_value::<BTreeMap<String, Value>>(properties_value) {
        Ok(map) => map,
        Err(e) => {
            return IOSSDKResult::error(IOSSDKError::new(
                IOSSDKErrorCode::InvalidParameter,
                format!("Failed to convert properties: {}", e),
            ))
        }
    };

    let result: Result<Document, FFIError> = wrapper.runtime.block_on(async {
        // Get platform version
        let platform_version = wrapper.sdk.version();

        // Convert properties to platform Value
        let data = Value::Map(
            properties
                .into_iter()
                .map(|(k, v)| (Value::Text(k), v))
                .collect(),
        );

        // Create document factory
        let factory = DocumentFactory::new(platform_version.protocol_version)
            .map_err(|e| FFIError::InternalError(format!("Failed to create factory: {}", e)))?;

        // Create document
        let document = factory
            .create_document(
                data_contract,
                identity.id(),
                document_type.to_string(),
                data,
            )
            .map_err(|e| FFIError::InternalError(format!("Failed to create document: {}", e)))?;

        Ok(document)
    });

    match result {
        Ok(document) => {
            let handle = Box::into_raw(Box::new(document)) as *mut DocumentHandle;
            IOSSDKResult::success(handle as *mut std::os::raw::c_void)
        }
        Err(e) => IOSSDKResult::error(e.into()),
    }
}

/// Fetch a document by ID
#[no_mangle]
pub unsafe extern "C" fn ios_sdk_document_fetch(
    sdk_handle: *const SDKHandle,
    data_contract_handle: *const DataContractHandle,
    document_type: *const c_char,
    document_id: *const c_char,
) -> IOSSDKResult {
    if sdk_handle.is_null()
        || data_contract_handle.is_null()
        || document_type.is_null()
        || document_id.is_null()
    {
        return IOSSDKResult::error(IOSSDKError::new(
            IOSSDKErrorCode::InvalidParameter,
            "Invalid parameters".to_string(),
        ));
    }

    let wrapper = &*(sdk_handle as *const SDKWrapper);
    let data_contract = &*(data_contract_handle as *const DataContract);

    let document_type_str = match CStr::from_ptr(document_type).to_str() {
        Ok(s) => s,
        Err(e) => return IOSSDKResult::error(FFIError::from(e).into()),
    };

    let document_id_str = match CStr::from_ptr(document_id).to_str() {
        Ok(s) => s,
        Err(e) => return IOSSDKResult::error(FFIError::from(e).into()),
    };

    let document_id = match Identifier::from_string(document_id_str, Encoding::Base58) {
        Ok(id) => id,
        Err(e) => {
            return IOSSDKResult::error(IOSSDKError::new(
                IOSSDKErrorCode::InvalidParameter,
                format!("Invalid document ID: {}", e),
            ))
        }
    };

    let result = wrapper.runtime.block_on(async {
        let query = DocumentQuery::new(data_contract.clone(), document_type_str)
            .map_err(|e| FFIError::InternalError(format!("Failed to create query: {}", e)))?
            .with_document_id(&document_id);

        Document::fetch(&wrapper.sdk, query)
            .await
            .map_err(FFIError::from)
    });

    match result {
        Ok(Some(document)) => {
            let handle = Box::into_raw(Box::new(document)) as *mut DocumentHandle;
            IOSSDKResult::success(handle as *mut std::os::raw::c_void)
        }
        Ok(None) => IOSSDKResult::error(IOSSDKError::new(
            IOSSDKErrorCode::NotFound,
            "Document not found".to_string(),
        )),
        Err(e) => IOSSDKResult::error(e.into()),
    }
}

/// Search for documents
#[no_mangle]
pub unsafe extern "C" fn ios_sdk_document_search(
    _sdk_handle: *const SDKHandle,
    _params: *const IOSSDKDocumentSearchParams,
) -> IOSSDKResult {
    // TODO: Implement document search
    // This requires handling DocumentQuery with proper trait bounds for Options
    IOSSDKResult::error(IOSSDKError::new(
        IOSSDKErrorCode::NotImplemented,
        "Document search not yet implemented. \
         DocumentQuery trait bounds need to be resolved."
            .to_string(),
    ))
}

/// Destroy a document
#[no_mangle]
pub unsafe extern "C" fn ios_sdk_document_destroy(
    sdk_handle: *mut SDKHandle,
    document_handle: *mut DocumentHandle,
) -> *mut IOSSDKError {
    if sdk_handle.is_null() || document_handle.is_null() {
        return Box::into_raw(Box::new(IOSSDKError::new(
            IOSSDKErrorCode::InvalidParameter,
            "Invalid parameters".to_string(),
        )));
    }

    let wrapper = &mut *(sdk_handle as *mut SDKWrapper);
    let _document = &*(document_handle as *const Document);

    let result: Result<(), FFIError> = wrapper.runtime.block_on(async {
        // Use DocumentDeleteTransitionBuilder to delete the document
        // We need to get the data contract and document type information
        // This is a simplified implementation - in practice you might need more context

        // For now, return not implemented as we need more context about the data contract
        Err(FFIError::InternalError(
            "Document deletion requires data contract context - use specific delete function"
                .to_string(),
        ))
    });

    match result {
        Ok(_) => std::ptr::null_mut(),
        Err(e) => Box::into_raw(Box::new(e.into())),
    }
}

/// Get document information
#[no_mangle]
pub unsafe extern "C" fn ios_sdk_document_get_info(
    document_handle: *const DocumentHandle,
) -> *mut IOSSDKDocumentInfo {
    if document_handle.is_null() {
        return std::ptr::null_mut();
    }

    let document = &*(document_handle as *const Document);

    let id_str = match CString::new(document.id().to_string(Encoding::Base58)) {
        Ok(s) => s.into_raw(),
        Err(_) => return std::ptr::null_mut(),
    };

    let owner_id_str = match CString::new(document.owner_id().to_string(Encoding::Base58)) {
        Ok(s) => s.into_raw(),
        Err(_) => {
            ios_sdk_string_free(id_str);
            return std::ptr::null_mut();
        }
    };

    // Document doesn't have data_contract_id, use placeholder
    let data_contract_id_str = match CString::new("unknown") {
        Ok(s) => s.into_raw(),
        Err(_) => {
            ios_sdk_string_free(id_str);
            ios_sdk_string_free(owner_id_str);
            return std::ptr::null_mut();
        }
    };

    // Document doesn't have document_type_name, use placeholder
    let document_type_str = match CString::new("unknown") {
        Ok(s) => s.into_raw(),
        Err(_) => {
            ios_sdk_string_free(id_str);
            ios_sdk_string_free(owner_id_str);
            ios_sdk_string_free(data_contract_id_str);
            return std::ptr::null_mut();
        }
    };

    let info = IOSSDKDocumentInfo {
        id: id_str,
        owner_id: owner_id_str,
        data_contract_id: data_contract_id_str,
        document_type: document_type_str,
        revision: document.revision().map(|r| r as u64).unwrap_or(0),
        created_at: document.created_at().map(|t| t as i64).unwrap_or(0),
        updated_at: document.updated_at().map(|t| t as i64).unwrap_or(0),
    };

    Box::into_raw(Box::new(info))
}

/// Delete a document from the platform
#[no_mangle]
pub unsafe extern "C" fn ios_sdk_document_delete(
    sdk_handle: *mut SDKHandle,
    document_handle: *const DocumentHandle,
    data_contract_handle: *const DataContractHandle,
    document_type_name: *const c_char,
    identity_public_key_handle: *const crate::types::IdentityPublicKeyHandle,
    signer_handle: *const SignerHandle,
    token_payment_info: *const IOSSDKTokenPaymentInfo,
    put_settings: *const IOSSDKPutSettings,
    state_transition_creation_options: *const IOSSDKStateTransitionCreationOptions,
) -> IOSSDKResult {
    // Validate required parameters
    if sdk_handle.is_null()
        || document_handle.is_null()
        || data_contract_handle.is_null()
        || document_type_name.is_null()
        || identity_public_key_handle.is_null()
        || signer_handle.is_null()
    {
        return IOSSDKResult::error(IOSSDKError::new(
            IOSSDKErrorCode::InvalidParameter,
            "One or more required parameters is null".to_string(),
        ));
    }

    let wrapper = &mut *(sdk_handle as *mut SDKWrapper);
    let document = &*(document_handle as *const Document);
    let data_contract = &*(data_contract_handle as *const DataContract);
    let identity_public_key = &*(identity_public_key_handle as *const IdentityPublicKey);
    let signer = &*(signer_handle as *const super::signer::IOSSigner);

    let document_type_name_str = match CStr::from_ptr(document_type_name).to_str() {
        Ok(s) => s,
        Err(e) => return IOSSDKResult::error(FFIError::from(e).into()),
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

        // Use the new DocumentDeleteTransitionBuilder
        let mut builder = DocumentDeleteTransitionBuilder::from_document(
            Arc::new(data_contract.clone()),
            document_type_name_str.to_string(),
            document,
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
        bincode::encode_to_vec(&state_transition, config).map_err(|e| {
            FFIError::InternalError(format!("Failed to serialize state transition: {}", e))
        })
    });

    match result {
        Ok(serialized_data) => IOSSDKResult::success_binary(serialized_data),
        Err(e) => IOSSDKResult::error(e.into()),
    }
}

/// Delete a document from the platform and wait for confirmation
#[no_mangle]
pub unsafe extern "C" fn ios_sdk_document_delete_and_wait(
    sdk_handle: *mut SDKHandle,
    document_handle: *const DocumentHandle,
    data_contract_handle: *const DataContractHandle,
    document_type_name: *const c_char,
    identity_public_key_handle: *const crate::types::IdentityPublicKeyHandle,
    signer_handle: *const SignerHandle,
    token_payment_info: *const IOSSDKTokenPaymentInfo,
    put_settings: *const IOSSDKPutSettings,
    state_transition_creation_options: *const IOSSDKStateTransitionCreationOptions,
) -> IOSSDKResult {
    // Validate required parameters
    if sdk_handle.is_null()
        || document_handle.is_null()
        || data_contract_handle.is_null()
        || document_type_name.is_null()
        || identity_public_key_handle.is_null()
        || signer_handle.is_null()
    {
        return IOSSDKResult::error(IOSSDKError::new(
            IOSSDKErrorCode::InvalidParameter,
            "One or more required parameters is null".to_string(),
        ));
    }

    let wrapper = &mut *(sdk_handle as *mut SDKWrapper);
    let document = &*(document_handle as *const Document);
    let data_contract = &*(data_contract_handle as *const DataContract);
    let identity_public_key = &*(identity_public_key_handle as *const IdentityPublicKey);
    let signer = &*(signer_handle as *const super::signer::IOSSigner);

    let document_type_name_str = match CStr::from_ptr(document_type_name).to_str() {
        Ok(s) => s,
        Err(e) => return IOSSDKResult::error(FFIError::from(e).into()),
    };

    let result: Result<Identifier, FFIError> = wrapper.runtime.block_on(async {
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

        // Use the new DocumentDeleteTransitionBuilder with SDK method
        let mut builder = DocumentDeleteTransitionBuilder::from_document(
            Arc::new(data_contract.clone()),
            document_type_name_str.to_string(),
            document,
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
            .document_delete(builder, identity_public_key, signer)
            .await
            .map_err(|e| {
                FFIError::InternalError(format!("Failed to delete document and wait: {}", e))
            })?;

        let deleted_id = match result {
            dash_sdk::platform::documents::transitions::DocumentDeleteResult::Deleted(id) => id,
        };

        Ok(deleted_id)
    });

    match result {
        Ok(_deleted_id) => IOSSDKResult::success(std::ptr::null_mut()),
        Err(e) => IOSSDKResult::error(e.into()),
    }
}

/// Destroy a document handle
#[no_mangle]
pub unsafe extern "C" fn ios_sdk_document_handle_destroy(handle: *mut DocumentHandle) {
    if !handle.is_null() {
        let _ = Box::from_raw(handle as *mut Document);
    }
}

/// Put document to platform (broadcast state transition)
#[no_mangle]
pub unsafe extern "C" fn ios_sdk_document_put_to_platform(
    sdk_handle: *mut SDKHandle,
    document_handle: *const DocumentHandle,
    data_contract_handle: *const DataContractHandle,
    document_type_name: *const c_char,
    entropy: *const [u8; 32],
    identity_public_key_handle: *const crate::types::IdentityPublicKeyHandle,
    signer_handle: *const SignerHandle,
    token_payment_info: *const IOSSDKTokenPaymentInfo,
    put_settings: *const IOSSDKPutSettings,
    state_transition_creation_options: *const IOSSDKStateTransitionCreationOptions,
) -> IOSSDKResult {
    // Validate required parameters
    if sdk_handle.is_null()
        || document_handle.is_null()
        || data_contract_handle.is_null()
        || document_type_name.is_null()
        || entropy.is_null()
        || identity_public_key_handle.is_null()
        || signer_handle.is_null()
    {
        return IOSSDKResult::error(IOSSDKError::new(
            IOSSDKErrorCode::InvalidParameter,
            "One or more required parameters is null".to_string(),
        ));
    }

    let wrapper = &mut *(sdk_handle as *mut SDKWrapper);
    let document = &*(document_handle as *const Document);
    let data_contract = &*(data_contract_handle as *const DataContract);
    let identity_public_key = &*(identity_public_key_handle as *const IdentityPublicKey);
    let signer = &*(signer_handle as *const super::signer::IOSSigner);
    let entropy_bytes = *entropy;

    let document_type_name_str = match CStr::from_ptr(document_type_name).to_str() {
        Ok(s) => s,
        Err(e) => return IOSSDKResult::error(FFIError::from(e).into()),
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
        Ok(serialized_data) => IOSSDKResult::success_binary(serialized_data),
        Err(e) => IOSSDKResult::error(e.into()),
    }
}

/// Put document to platform and wait for confirmation (broadcast state transition and wait for response)
#[no_mangle]
pub unsafe extern "C" fn ios_sdk_document_put_to_platform_and_wait(
    sdk_handle: *mut SDKHandle,
    document_handle: *const DocumentHandle,
    data_contract_handle: *const DataContractHandle,
    document_type_name: *const c_char,
    entropy: *const [u8; 32],
    identity_public_key_handle: *const crate::types::IdentityPublicKeyHandle,
    signer_handle: *const SignerHandle,
    token_payment_info: *const IOSSDKTokenPaymentInfo,
    put_settings: *const IOSSDKPutSettings,
    state_transition_creation_options: *const IOSSDKStateTransitionCreationOptions,
) -> IOSSDKResult {
    // Validate required parameters
    if sdk_handle.is_null()
        || document_handle.is_null()
        || data_contract_handle.is_null()
        || document_type_name.is_null()
        || entropy.is_null()
        || identity_public_key_handle.is_null()
        || signer_handle.is_null()
    {
        return IOSSDKResult::error(IOSSDKError::new(
            IOSSDKErrorCode::InvalidParameter,
            "One or more required parameters is null".to_string(),
        ));
    }

    let wrapper = &mut *(sdk_handle as *mut SDKWrapper);
    let document = &*(document_handle as *const Document);
    let data_contract = &*(data_contract_handle as *const DataContract);
    let identity_public_key = &*(identity_public_key_handle as *const IdentityPublicKey);
    let signer = &*(signer_handle as *const super::signer::IOSSigner);
    let entropy_bytes = *entropy;

    let document_type_name_str = match CStr::from_ptr(document_type_name).to_str() {
        Ok(s) => s,
        Err(e) => return IOSSDKResult::error(FFIError::from(e).into()),
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
            IOSSDKResult::success_handle(
                handle as *mut std::os::raw::c_void,
                IOSSDKResultDataType::DocumentHandle,
            )
        }
        Err(e) => IOSSDKResult::error(e.into()),
    }
}

/// Replace document on platform (broadcast state transition)
#[no_mangle]
pub unsafe extern "C" fn ios_sdk_document_replace_on_platform(
    sdk_handle: *mut SDKHandle,
    document_handle: *const DocumentHandle,
    data_contract_handle: *const DataContractHandle,
    document_type_name: *const c_char,
    identity_public_key_handle: *const crate::types::IdentityPublicKeyHandle,
    signer_handle: *const SignerHandle,
    token_payment_info: *const IOSSDKTokenPaymentInfo,
    put_settings: *const IOSSDKPutSettings,
    state_transition_creation_options: *const IOSSDKStateTransitionCreationOptions,
) -> IOSSDKResult {
    // Validate required parameters
    if sdk_handle.is_null()
        || document_handle.is_null()
        || data_contract_handle.is_null()
        || document_type_name.is_null()
        || identity_public_key_handle.is_null()
        || signer_handle.is_null()
    {
        return IOSSDKResult::error(IOSSDKError::new(
            IOSSDKErrorCode::InvalidParameter,
            "One or more required parameters is null".to_string(),
        ));
    }

    let wrapper = &mut *(sdk_handle as *mut SDKWrapper);
    let document = &*(document_handle as *const Document);
    let data_contract = &*(data_contract_handle as *const DataContract);
    let identity_public_key = &*(identity_public_key_handle as *const IdentityPublicKey);
    let signer = &*(signer_handle as *const super::signer::IOSSigner);

    let document_type_name_str = match CStr::from_ptr(document_type_name).to_str() {
        Ok(s) => s,
        Err(e) => return IOSSDKResult::error(FFIError::from(e).into()),
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
        Ok(serialized_data) => IOSSDKResult::success_binary(serialized_data),
        Err(e) => IOSSDKResult::error(e.into()),
    }
}

/// Replace document on platform and wait for confirmation (broadcast state transition and wait for response)
#[no_mangle]
pub unsafe extern "C" fn ios_sdk_document_replace_on_platform_and_wait(
    sdk_handle: *mut SDKHandle,
    document_handle: *const DocumentHandle,
    data_contract_handle: *const DataContractHandle,
    document_type_name: *const c_char,
    identity_public_key_handle: *const crate::types::IdentityPublicKeyHandle,
    signer_handle: *const SignerHandle,
    token_payment_info: *const IOSSDKTokenPaymentInfo,
    put_settings: *const IOSSDKPutSettings,
    state_transition_creation_options: *const IOSSDKStateTransitionCreationOptions,
) -> IOSSDKResult {
    // Validate required parameters
    if sdk_handle.is_null()
        || document_handle.is_null()
        || data_contract_handle.is_null()
        || document_type_name.is_null()
        || identity_public_key_handle.is_null()
        || signer_handle.is_null()
    {
        return IOSSDKResult::error(IOSSDKError::new(
            IOSSDKErrorCode::InvalidParameter,
            "One or more required parameters is null".to_string(),
        ));
    }

    let wrapper = &mut *(sdk_handle as *mut SDKWrapper);
    let document = &*(document_handle as *const Document);
    let data_contract = &*(data_contract_handle as *const DataContract);
    let identity_public_key = &*(identity_public_key_handle as *const IdentityPublicKey);
    let signer = &*(signer_handle as *const super::signer::IOSSigner);

    let document_type_name_str = match CStr::from_ptr(document_type_name).to_str() {
        Ok(s) => s,
        Err(e) => return IOSSDKResult::error(FFIError::from(e).into()),
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
            IOSSDKResult::success_handle(
                handle as *mut std::os::raw::c_void,
                IOSSDKResultDataType::DocumentHandle,
            )
        }
        Err(e) => IOSSDKResult::error(e.into()),
    }
}

/// Purchase document (broadcast state transition)
#[no_mangle]
pub unsafe extern "C" fn ios_sdk_document_purchase(
    sdk_handle: *mut SDKHandle,
    document_handle: *const DocumentHandle,
    data_contract_handle: *const DataContractHandle,
    document_type_name: *const c_char,
    price: u64,
    purchaser_id: *const c_char,
    identity_public_key_handle: *const crate::types::IdentityPublicKeyHandle,
    signer_handle: *const SignerHandle,
    token_payment_info: *const IOSSDKTokenPaymentInfo,
    put_settings: *const IOSSDKPutSettings,
    state_transition_creation_options: *const IOSSDKStateTransitionCreationOptions,
) -> IOSSDKResult {
    // Validate parameters
    if sdk_handle.is_null()
        || document_handle.is_null()
        || data_contract_handle.is_null()
        || document_type_name.is_null()
        || purchaser_id.is_null()
        || identity_public_key_handle.is_null()
        || signer_handle.is_null()
    {
        return IOSSDKResult::error(IOSSDKError::new(
            IOSSDKErrorCode::InvalidParameter,
            "One or more required parameters is null".to_string(),
        ));
    }

    let wrapper = &mut *(sdk_handle as *mut SDKWrapper);
    let document = &*(document_handle as *const Document);
    let data_contract = &*(data_contract_handle as *const DataContract);
    let identity_public_key = &*(identity_public_key_handle as *const IdentityPublicKey);
    let signer = &*(signer_handle as *const super::signer::IOSSigner);

    let document_type_name_str = match CStr::from_ptr(document_type_name).to_str() {
        Ok(s) => s,
        Err(e) => return IOSSDKResult::error(FFIError::from(e).into()),
    };

    let purchaser_id_str = match CStr::from_ptr(purchaser_id).to_str() {
        Ok(s) => s,
        Err(e) => return IOSSDKResult::error(FFIError::from(e).into()),
    };

    let purchaser_id = match Identifier::from_string(purchaser_id_str, Encoding::Base58) {
        Ok(id) => id,
        Err(e) => {
            return IOSSDKResult::error(IOSSDKError::new(
                IOSSDKErrorCode::InvalidParameter,
                format!("Invalid purchaser ID: {}", e),
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

        // Use the new DocumentPurchaseTransitionBuilder
        let mut builder = DocumentPurchaseTransitionBuilder::new(
            Arc::new(data_contract.clone()),
            document_type_name_str.to_string(),
            document.clone(),
            purchaser_id,
            price,
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
                FFIError::InternalError(format!("Failed to create purchase transition: {}", e))
            })?;

        // Serialize the state transition with bincode
        let config = bincode::config::standard();
        bincode::encode_to_vec(&state_transition, config).map_err(|e| {
            FFIError::InternalError(format!("Failed to serialize state transition: {}", e))
        })
    });

    match result {
        Ok(serialized_data) => IOSSDKResult::success_binary(serialized_data),
        Err(e) => IOSSDKResult::error(e.into()),
    }
}

/// Purchase document and wait for confirmation (broadcast state transition and wait for response)
#[no_mangle]
pub unsafe extern "C" fn ios_sdk_document_purchase_and_wait(
    sdk_handle: *mut SDKHandle,
    document_handle: *const DocumentHandle,
    data_contract_handle: *const DataContractHandle,
    document_type_name: *const c_char,
    price: u64,
    purchaser_id: *const c_char,
    identity_public_key_handle: *const crate::types::IdentityPublicKeyHandle,
    signer_handle: *const SignerHandle,
    token_payment_info: *const IOSSDKTokenPaymentInfo,
    put_settings: *const IOSSDKPutSettings,
    state_transition_creation_options: *const IOSSDKStateTransitionCreationOptions,
) -> IOSSDKResult {
    // Validate parameters
    if sdk_handle.is_null()
        || document_handle.is_null()
        || data_contract_handle.is_null()
        || document_type_name.is_null()
        || purchaser_id.is_null()
        || identity_public_key_handle.is_null()
        || signer_handle.is_null()
    {
        return IOSSDKResult::error(IOSSDKError::new(
            IOSSDKErrorCode::InvalidParameter,
            "One or more required parameters is null".to_string(),
        ));
    }

    let wrapper = &mut *(sdk_handle as *mut SDKWrapper);
    let document = &*(document_handle as *const Document);
    let data_contract = &*(data_contract_handle as *const DataContract);
    let identity_public_key = &*(identity_public_key_handle as *const IdentityPublicKey);
    let signer = &*(signer_handle as *const super::signer::IOSSigner);

    let document_type_name_str = match CStr::from_ptr(document_type_name).to_str() {
        Ok(s) => s,
        Err(e) => return IOSSDKResult::error(FFIError::from(e).into()),
    };

    let purchaser_id_str = match CStr::from_ptr(purchaser_id).to_str() {
        Ok(s) => s,
        Err(e) => return IOSSDKResult::error(FFIError::from(e).into()),
    };

    let purchaser_id = match Identifier::from_string(purchaser_id_str, Encoding::Base58) {
        Ok(id) => id,
        Err(e) => {
            return IOSSDKResult::error(IOSSDKError::new(
                IOSSDKErrorCode::InvalidParameter,
                format!("Invalid purchaser ID: {}", e),
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

        // Use the new DocumentPurchaseTransitionBuilder with SDK method
        let mut builder = DocumentPurchaseTransitionBuilder::new(
            Arc::new(data_contract.clone()),
            document_type_name_str.to_string(),
            document.clone(),
            purchaser_id,
            price,
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
            .document_purchase(builder, identity_public_key, signer)
            .await
            .map_err(|e| {
                FFIError::InternalError(format!("Failed to purchase document and wait: {}", e))
            })?;

        let purchased_document = match result {
            dash_sdk::platform::documents::transitions::DocumentPurchaseResult::Document(doc) => {
                doc
            }
        };

        Ok(purchased_document)
    });

    match result {
        Ok(purchased_document) => {
            let handle = Box::into_raw(Box::new(purchased_document)) as *mut DocumentHandle;
            IOSSDKResult::success_handle(
                handle as *mut std::os::raw::c_void,
                IOSSDKResultDataType::DocumentHandle,
            )
        }
        Err(e) => IOSSDKResult::error(e.into()),
    }
}

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
pub unsafe extern "C" fn ios_sdk_document_transfer_to_identity(
    sdk_handle: *mut SDKHandle,
    document_handle: *const DocumentHandle,
    recipient_id: *const c_char,
    data_contract_handle: *const DataContractHandle,
    document_type_name: *const c_char,
    identity_public_key_handle: *const crate::types::IdentityPublicKeyHandle,
    signer_handle: *const SignerHandle,
    token_payment_info: *const IOSSDKTokenPaymentInfo,
    put_settings: *const IOSSDKPutSettings,
    state_transition_creation_options: *const IOSSDKStateTransitionCreationOptions,
) -> IOSSDKResult {
    // Validate parameters
    if sdk_handle.is_null()
        || document_handle.is_null()
        || recipient_id.is_null()
        || data_contract_handle.is_null()
        || document_type_name.is_null()
        || identity_public_key_handle.is_null()
        || signer_handle.is_null()
    {
        return IOSSDKResult::error(IOSSDKError::new(
            IOSSDKErrorCode::InvalidParameter,
            "One or more required parameters is null".to_string(),
        ));
    }

    let wrapper = &mut *(sdk_handle as *mut SDKWrapper);
    let document = &*(document_handle as *const Document);
    let data_contract = &*(data_contract_handle as *const DataContract);
    let identity_public_key = &*(identity_public_key_handle as *const IdentityPublicKey);
    let signer = &*(signer_handle as *const super::signer::IOSSigner);

    let recipient_id_str = match CStr::from_ptr(recipient_id).to_str() {
        Ok(s) => s,
        Err(e) => return IOSSDKResult::error(FFIError::from(e).into()),
    };

    let document_type_name_str = match CStr::from_ptr(document_type_name).to_str() {
        Ok(s) => s,
        Err(e) => return IOSSDKResult::error(FFIError::from(e).into()),
    };

    let recipient_identifier = match Identifier::from_string(recipient_id_str, Encoding::Base58) {
        Ok(id) => id,
        Err(e) => {
            return IOSSDKResult::error(IOSSDKError::new(
                IOSSDKErrorCode::InvalidParameter,
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
        Ok(serialized_data) => IOSSDKResult::success_binary(serialized_data),
        Err(e) => IOSSDKResult::error(e.into()),
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
pub unsafe extern "C" fn ios_sdk_document_transfer_to_identity_and_wait(
    sdk_handle: *mut SDKHandle,
    document_handle: *const DocumentHandle,
    recipient_id: *const c_char,
    data_contract_handle: *const DataContractHandle,
    document_type_name: *const c_char,
    identity_public_key_handle: *const crate::types::IdentityPublicKeyHandle,
    signer_handle: *const SignerHandle,
    token_payment_info: *const IOSSDKTokenPaymentInfo,
    put_settings: *const IOSSDKPutSettings,
    state_transition_creation_options: *const IOSSDKStateTransitionCreationOptions,
) -> IOSSDKResult {
    // Validate parameters
    if sdk_handle.is_null()
        || document_handle.is_null()
        || recipient_id.is_null()
        || data_contract_handle.is_null()
        || document_type_name.is_null()
        || identity_public_key_handle.is_null()
        || signer_handle.is_null()
    {
        return IOSSDKResult::error(IOSSDKError::new(
            IOSSDKErrorCode::InvalidParameter,
            "One or more required parameters is null".to_string(),
        ));
    }

    let wrapper = &mut *(sdk_handle as *mut SDKWrapper);
    let document = &*(document_handle as *const Document);
    let data_contract = &*(data_contract_handle as *const DataContract);
    let identity_public_key = &*(identity_public_key_handle as *const IdentityPublicKey);
    let signer = &*(signer_handle as *const super::signer::IOSSigner);

    let recipient_id_str = match CStr::from_ptr(recipient_id).to_str() {
        Ok(s) => s,
        Err(e) => return IOSSDKResult::error(FFIError::from(e).into()),
    };

    let document_type_name_str = match CStr::from_ptr(document_type_name).to_str() {
        Ok(s) => s,
        Err(e) => return IOSSDKResult::error(FFIError::from(e).into()),
    };

    let recipient_identifier = match Identifier::from_string(recipient_id_str, Encoding::Base58) {
        Ok(id) => id,
        Err(e) => {
            return IOSSDKResult::error(IOSSDKError::new(
                IOSSDKErrorCode::InvalidParameter,
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
            IOSSDKResult::success_handle(
                handle as *mut std::os::raw::c_void,
                IOSSDKResultDataType::DocumentHandle,
            )
        }
        Err(e) => IOSSDKResult::error(e.into()),
    }
}

/// Update document price (broadcast state transition)
#[no_mangle]
pub unsafe extern "C" fn ios_sdk_document_update_price_of_document(
    sdk_handle: *mut SDKHandle,
    document_handle: *const DocumentHandle,
    data_contract_handle: *const DataContractHandle,
    document_type_name: *const c_char,
    price: u64,
    identity_public_key_handle: *const crate::types::IdentityPublicKeyHandle,
    signer_handle: *const SignerHandle,
    token_payment_info: *const IOSSDKTokenPaymentInfo,
    put_settings: *const IOSSDKPutSettings,
    state_transition_creation_options: *const IOSSDKStateTransitionCreationOptions,
) -> IOSSDKResult {
    // Validate required parameters
    if sdk_handle.is_null()
        || document_handle.is_null()
        || data_contract_handle.is_null()
        || document_type_name.is_null()
        || identity_public_key_handle.is_null()
        || signer_handle.is_null()
    {
        return IOSSDKResult::error(IOSSDKError::new(
            IOSSDKErrorCode::InvalidParameter,
            "One or more required parameters is null".to_string(),
        ));
    }

    let wrapper = &mut *(sdk_handle as *mut SDKWrapper);
    let document = &*(document_handle as *const Document);
    let data_contract = &*(data_contract_handle as *const DataContract);
    let identity_public_key = &*(identity_public_key_handle as *const IdentityPublicKey);
    let signer = &*(signer_handle as *const super::signer::IOSSigner);

    let document_type_name_str = match CStr::from_ptr(document_type_name).to_str() {
        Ok(s) => s,
        Err(e) => return IOSSDKResult::error(FFIError::from(e).into()),
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

        // Use the new DocumentSetPriceTransitionBuilder
        let mut builder = DocumentSetPriceTransitionBuilder::new(
            Arc::new(data_contract.clone()),
            document_type_name_str.to_string(),
            document.clone(),
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
        Ok(serialized_data) => IOSSDKResult::success_binary(serialized_data),
        Err(e) => IOSSDKResult::error(e.into()),
    }
}

/// Update document price and wait for confirmation (broadcast state transition and wait for response)
#[no_mangle]
pub unsafe extern "C" fn ios_sdk_document_update_price_of_document_and_wait(
    sdk_handle: *mut SDKHandle,
    document_handle: *const DocumentHandle,
    data_contract_handle: *const DataContractHandle,
    document_type_name: *const c_char,
    price: u64,
    identity_public_key_handle: *const crate::types::IdentityPublicKeyHandle,
    signer_handle: *const SignerHandle,
    token_payment_info: *const IOSSDKTokenPaymentInfo,
    put_settings: *const IOSSDKPutSettings,
    state_transition_creation_options: *const IOSSDKStateTransitionCreationOptions,
) -> IOSSDKResult {
    // Validate required parameters
    if sdk_handle.is_null()
        || document_handle.is_null()
        || data_contract_handle.is_null()
        || document_type_name.is_null()
        || identity_public_key_handle.is_null()
        || signer_handle.is_null()
    {
        return IOSSDKResult::error(IOSSDKError::new(
            IOSSDKErrorCode::InvalidParameter,
            "One or more required parameters is null".to_string(),
        ));
    }

    let wrapper = &mut *(sdk_handle as *mut SDKWrapper);
    let document = &*(document_handle as *const Document);
    let data_contract = &*(data_contract_handle as *const DataContract);
    let identity_public_key = &*(identity_public_key_handle as *const IdentityPublicKey);
    let signer = &*(signer_handle as *const super::signer::IOSSigner);

    let document_type_name_str = match CStr::from_ptr(document_type_name).to_str() {
        Ok(s) => s,
        Err(e) => return IOSSDKResult::error(FFIError::from(e).into()),
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

        // Use the new DocumentSetPriceTransitionBuilder with SDK method
        let mut builder = DocumentSetPriceTransitionBuilder::new(
            Arc::new(data_contract.clone()),
            document_type_name_str.to_string(),
            document.clone(),
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

        let updated_document = match result {
            dash_sdk::platform::documents::transitions::DocumentSetPriceResult::Document(doc) => {
                doc
            }
        };

        Ok(updated_document)
    });

    match result {
        Ok(updated_document) => {
            let handle = Box::into_raw(Box::new(updated_document)) as *mut DocumentHandle;
            IOSSDKResult::success_handle(
                handle as *mut std::os::raw::c_void,
                IOSSDKResultDataType::DocumentHandle,
            )
        }
        Err(e) => IOSSDKResult::error(e.into()),
    }
}

// Helper function for freeing strings
use crate::types::ios_sdk_string_free;
