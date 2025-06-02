//! Document operations

use crate::sdk::SDKWrapper;
use crate::types::{
    DataContractHandle, DocumentHandle, IOSSDKDocumentInfo, IdentityHandle, SDKHandle, SignerHandle,
};
use crate::{FFIError, IOSSDKError, IOSSDKErrorCode, IOSSDKResult};
use dash_sdk::platform::{DocumentQuery, Fetch};
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::document::{document_factory::DocumentFactory, Document, DocumentV0Getters};
use dpp::identity::accessors::IdentityGettersV0;
use dpp::prelude::{DataContract, Identifier, Identity};
use platform_value::{string_encoding::Encoding, Value};
use std::collections::BTreeMap;
use std::ffi::{CStr, CString};
use std::os::raw::c_char;

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

    let result: Result<dpp::document::Document, FFIError> = wrapper.runtime.block_on(async {
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
) -> IOSSDKResult {
    // Validate parameters
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
    let identity_public_key =
        &*(identity_public_key_handle as *const dpp::identity::IdentityPublicKey);
    let signer = &*(signer_handle as *const super::signer::IOSSigner);
    let entropy_bytes = *entropy;

    let document_type_name_str = match CStr::from_ptr(document_type_name).to_str() {
        Ok(s) => s,
        Err(e) => return IOSSDKResult::error(FFIError::from(e).into()),
    };

    let result: Result<String, FFIError> = wrapper.runtime.block_on(async {
        // Get document type from data contract
        let document_type = data_contract
            .document_type_for_name(document_type_name_str)
            .map_err(|e| FFIError::InternalError(format!("Failed to get document type: {}", e)))?;

        let document_type_owned = document_type.to_owned_document_type();

        // Put document to platform using the PutDocument trait
        use dash_sdk::platform::transition::put_document::PutDocument;

        let _state_transition = document
            .put_to_platform(
                &wrapper.sdk,
                document_type_owned,
                entropy_bytes,
                identity_public_key.clone(),
                None, // token_payment_info
                signer,
                None, // settings (use defaults)
            )
            .await
            .map_err(|e| {
                FFIError::InternalError(format!("Failed to put document to platform: {}", e))
            })?;

        // For now, just return success. In a full implementation, you would return the state transition ID
        Ok("success".to_string())
    });

    match result {
        Ok(id_string) => match CString::new(id_string) {
            Ok(c_string) => {
                let ptr = c_string.into_raw();
                IOSSDKResult::success(ptr as *mut std::os::raw::c_void)
            }
            Err(e) => IOSSDKResult::error(IOSSDKError::new(
                IOSSDKErrorCode::InternalError,
                format!("Failed to create C string: {}", e),
            )),
        },
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
) -> IOSSDKResult {
    // Validate parameters
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
    let identity_public_key =
        &*(identity_public_key_handle as *const dpp::identity::IdentityPublicKey);
    let signer = &*(signer_handle as *const super::signer::IOSSigner);
    let entropy_bytes = *entropy;

    let document_type_name_str = match CStr::from_ptr(document_type_name).to_str() {
        Ok(s) => s,
        Err(e) => return IOSSDKResult::error(FFIError::from(e).into()),
    };

    let result: Result<Document, FFIError> = wrapper.runtime.block_on(async {
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
                None, // token_payment_info
                signer,
                None, // settings (use defaults)
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
            IOSSDKResult::success(handle as *mut std::os::raw::c_void)
        }
        Err(e) => IOSSDKResult::error(e.into()),
    }
}

// Helper function for freeing strings
use crate::types::ios_sdk_string_free;
