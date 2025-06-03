//! Document operations

use crate::sdk::SDKWrapper;
use crate::types::{
    DataContractHandle, DocumentHandle, IOSSDKDocumentInfo, IOSSDKGasFeesPaidBy, IOSSDKPutSettings,
    IOSSDKResultDataType, IOSSDKTokenPaymentInfo, IdentityHandle, SDKHandle, SignerHandle,
};
use crate::{FFIError, IOSSDKError, IOSSDKErrorCode, IOSSDKResult};
use dash_sdk::dpp::data_contract::accessors::v0::DataContractV0Getters;
use dash_sdk::dpp::document::{document_factory::DocumentFactory, Document, DocumentV0Getters};
use dash_sdk::dpp::fee::Credits;
use dash_sdk::dpp::identity::accessors::IdentityGettersV0;
use dash_sdk::dpp::platform_value::{string_encoding::Encoding, Value};
use dash_sdk::dpp::prelude::{DataContract, Identifier, Identity};
use dash_sdk::dpp::tokens::gas_fees_paid_by::GasFeesPaidBy;
use dash_sdk::dpp::tokens::token_payment_info::v0::TokenPaymentInfoV0;
use dash_sdk::dpp::tokens::token_payment_info::TokenPaymentInfo;
use dash_sdk::platform::transition::update_price_of_document::UpdatePriceOfDocument;
use dash_sdk::platform::{DocumentQuery, Fetch, IdentityPublicKey};
use std::collections::BTreeMap;
use std::ffi::{CStr, CString};
use std::os::raw::c_char;

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

/// Update an existing document
#[no_mangle]
pub unsafe extern "C" fn ios_sdk_document_update(
    sdk_handle: *mut SDKHandle,
    document_handle: *mut DocumentHandle,
    properties_json: *const c_char,
) -> *mut IOSSDKError {
    if sdk_handle.is_null() || document_handle.is_null() || properties_json.is_null() {
        return Box::into_raw(Box::new(IOSSDKError::new(
            IOSSDKErrorCode::InvalidParameter,
            "Invalid parameters".to_string(),
        )));
    }

    // TODO: Implement document update
    Box::into_raw(Box::new(IOSSDKError::new(
        IOSSDKErrorCode::NotImplemented,
        "Document update not yet implemented".to_string(),
    )))
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

    // TODO: Implement document deletion via state transition
    Box::into_raw(Box::new(IOSSDKError::new(
        IOSSDKErrorCode::NotImplemented,
        "Document deletion not yet implemented".to_string(),
    )))
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

        // Get document type from data contract
        let document_type = data_contract
            .document_type_for_name(document_type_name_str)
            .map_err(|e| FFIError::InternalError(format!("Failed to get document type: {}", e)))?;

        let document_type_owned = document_type.to_owned_document_type();

        // Put document to platform using the PutDocument trait
        use dash_sdk::platform::transition::put_document::PutDocument;

        let state_transition = document
            .put_to_platform(
                &wrapper.sdk,
                document_type_owned,
                entropy_bytes,
                identity_public_key.clone(),
                token_payment_info_converted,
                signer,
                settings,
            )
            .await
            .map_err(|e| {
                FFIError::InternalError(format!("Failed to put document to platform: {}", e))
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

        // Get document type from data contract
        let document_type = data_contract
            .document_type_for_name(document_type_name_str)
            .map_err(|e| FFIError::InternalError(format!("Failed to get document type: {}", e)))?;

        let document_type_owned = document_type.to_owned_document_type();

        // Put document to platform and wait for response
        use dash_sdk::platform::transition::put_document::PutDocument;

        let confirmed_document = document
            .put_to_platform_and_wait_for_response(
                &wrapper.sdk,
                document_type_owned,
                entropy_bytes,
                identity_public_key.clone(),
                token_payment_info_converted,
                signer,
                settings,
            )
            .await
            .map_err(|e| {
                FFIError::InternalError(format!(
                    "Failed to put document to platform and wait: {}",
                    e
                ))
            })?;

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

/// Purchase document (broadcast state transition)
#[no_mangle]
pub unsafe extern "C" fn ios_sdk_document_purchase_to_platform(
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

        // Get document type from data contract
        let document_type = data_contract
            .document_type_for_name(document_type_name_str)
            .map_err(|e| FFIError::InternalError(format!("Failed to get document type: {}", e)))?;

        let document_type_owned = document_type.to_owned_document_type();

        // Purchase document using the PurchaseDocument trait
        use dash_sdk::platform::transition::purchase_document::PurchaseDocument;

        let state_transition = document
            .purchase_document(
                price,
                &wrapper.sdk,
                document_type_owned,
                purchaser_id,
                identity_public_key.clone(),
                token_payment_info_converted,
                signer,
                settings,
            )
            .await
            .map_err(|e| FFIError::InternalError(format!("Failed to purchase document: {}", e)))?;

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
pub unsafe extern "C" fn ios_sdk_document_purchase_to_platform_and_wait(
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

        // Get document type from data contract
        let document_type = data_contract
            .document_type_for_name(document_type_name_str)
            .map_err(|e| FFIError::InternalError(format!("Failed to get document type: {}", e)))?;

        let document_type_owned = document_type.to_owned_document_type();

        // Purchase document and wait for response
        use dash_sdk::platform::transition::purchase_document::PurchaseDocument;

        let purchased_document = document
            .purchase_document_and_wait_for_response(
                price,
                &wrapper.sdk,
                document_type_owned,
                purchaser_id,
                identity_public_key.clone(),
                token_payment_info_converted,
                signer,
                settings,
            )
            .await
            .map_err(|e| {
                FFIError::InternalError(format!("Failed to purchase document and wait: {}", e))
            })?;

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
    put_settings: *const crate::types::IOSSDKPutSettings,
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

        // Get document type from the contract
        let document_type = data_contract
            .document_types()
            .get(document_type_name_str)
            .ok_or_else(|| {
                FFIError::InternalError(format!(
                    "Document type '{}' not found",
                    document_type_name_str
                ))
            })?
            .clone();

        // Convert settings
        let settings = crate::identity::convert_put_settings(put_settings);

        // Use TransferDocument trait to transfer document
        use dash_sdk::platform::transition::transfer_document::TransferDocument;

        let state_transition = document
            .transfer_document_to_identity(
                recipient_identifier,
                &wrapper.sdk,
                document_type,
                identity_public_key.clone(),
                token_payment_info_converted,
                signer,
                settings,
            )
            .await
            .map_err(|e| FFIError::InternalError(format!("Failed to transfer document: {}", e)))?;

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
    put_settings: *const crate::types::IOSSDKPutSettings,
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

        // Get document type from the contract
        let document_type = data_contract
            .document_types()
            .get(document_type_name_str)
            .ok_or_else(|| {
                FFIError::InternalError(format!(
                    "Document type '{}' not found",
                    document_type_name_str
                ))
            })?
            .clone();

        // Convert settings
        let settings = crate::identity::convert_put_settings(put_settings);

        // Use TransferDocument trait to transfer document and wait
        use dash_sdk::platform::transition::transfer_document::TransferDocument;

        let transferred_document = document
            .transfer_document_to_identity_and_wait_for_response(
                recipient_identifier,
                &wrapper.sdk,
                document_type,
                identity_public_key.clone(),
                token_payment_info_converted,
                signer,
                settings,
            )
            .await
            .map_err(|e| {
                FFIError::InternalError(format!("Failed to transfer document and wait: {}", e))
            })?;

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

        // Get document type from data contract
        let document_type = data_contract
            .document_type_for_name(document_type_name_str)
            .map_err(|e| FFIError::InternalError(format!("Failed to get document type: {}", e)))?;

        let document_type_owned = document_type.to_owned_document_type();

        // Update document price using the UpdatePriceOfDocument trait
        let state_transition = document
            .update_price_of_document(
                price as Credits,
                &wrapper.sdk,
                document_type_owned,
                identity_public_key.clone(),
                token_payment_info_converted,
                signer,
                settings,
            )
            .await
            .map_err(|e| {
                FFIError::InternalError(format!("Failed to update document price: {}", e))
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

        // Get document type from data contract
        let document_type = data_contract
            .document_type_for_name(document_type_name_str)
            .map_err(|e| FFIError::InternalError(format!("Failed to get document type: {}", e)))?;

        let document_type_owned = document_type.to_owned_document_type();

        // Update document price and wait for response
        let updated_document = document
            .update_price_of_document_and_wait_for_response(
                price as Credits,
                &wrapper.sdk,
                document_type_owned,
                identity_public_key.clone(),
                token_payment_info_converted,
                signer,
                settings,
            )
            .await
            .map_err(|e| {
                FFIError::InternalError(format!("Failed to update document price and wait: {}", e))
            })?;

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
