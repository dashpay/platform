//! Document operations

use crate::sdk::SDKWrapper;
use crate::signer::IOSSigner;
use crate::types::{
    DataContractHandle, DocumentHandle, IOSSDKDocumentInfo, IdentityHandle, SDKHandle, SignerHandle,
};
use crate::{FFIError, IOSSDKError, IOSSDKErrorCode, IOSSDKResult};
use bincode::Options;
use dash_sdk::platform::{DocumentQuery, Fetch, FetchMany};
use dpp::data_contract::document_type::{accessors::DocumentTypeV0Getters, DocumentType};
use dpp::document::{document_factory::DocumentFactory, Document, DocumentV0Getters};
use dpp::identity::accessors::IdentityGettersV0;
use dpp::prelude::{DataContract, Identifier, Identity, IdentityPublicKey};
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

    let result = wrapper.runtime.block_on(async {
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
    sdk_handle: *const SDKHandle,
    params: *const IOSSDKDocumentSearchParams,
) -> IOSSDKResult {
    if sdk_handle.is_null() || params.is_null() {
        return IOSSDKResult::error(IOSSDKError::new(
            IOSSDKErrorCode::InvalidParameter,
            "Invalid parameters".to_string(),
        ));
    }

    let params = &*params;
    if params.data_contract_handle.is_null() || params.document_type.is_null() {
        return IOSSDKResult::error(IOSSDKError::new(
            IOSSDKErrorCode::InvalidParameter,
            "Required parameter is null".to_string(),
        ));
    }

    let wrapper = &*(sdk_handle as *const SDKWrapper);
    let data_contract = &*(params.data_contract_handle as *const DataContract);

    let document_type_str = match CStr::from_ptr(params.document_type).to_str() {
        Ok(s) => s,
        Err(e) => return IOSSDKResult::error(FFIError::from(e).into()),
    };

    let result = wrapper.runtime.block_on(async {
        let mut query = DocumentQuery::new(data_contract.clone(), document_type_str)
            .map_err(|e| FFIError::InternalError(format!("Failed to create query: {}", e)))?;

        // Apply where clauses if provided
        if !params.where_json.is_null() {
            let where_str = CStr::from_ptr(params.where_json)
                .to_str()
                .map_err(|e| FFIError::from(e))?;

            if !where_str.is_empty() {
                // TODO: Parse and apply where clauses
                // This would require implementing JSON parsing for WhereClause structures
            }
        }

        // Apply order by if provided
        if !params.order_by_json.is_null() {
            let order_str = CStr::from_ptr(params.order_by_json)
                .to_str()
                .map_err(|e| FFIError::from(e))?;

            if !order_str.is_empty() {
                // TODO: Parse and apply order by clauses
                // This would require implementing JSON parsing for OrderClause structures
            }
        }

        // Apply limit
        if params.limit > 0 {
            query = query.with_limit(params.limit);
        }

        // Apply start at for pagination
        if params.start_at > 0 {
            // TODO: Implement start_at pagination
        }

        let documents = Document::fetch_many(&wrapper.sdk, query)
            .await
            .map_err(FFIError::from)?;

        Ok(documents)
    });

    match result {
        Ok(documents) => {
            // Convert Vec<Document> to a handle
            // For now, we'll just return the first document if any
            // In a real implementation, you'd want to return an array handle
            if let Some(document) = documents.into_iter().next() {
                let handle = Box::into_raw(Box::new(document)) as *mut DocumentHandle;
                IOSSDKResult::success(handle as *mut std::os::raw::c_void)
            } else {
                IOSSDKResult::error(IOSSDKError::new(
                    IOSSDKErrorCode::NotFound,
                    "No documents found".to_string(),
                ))
            }
        }
        Err(e) => IOSSDKResult::error(e.into()),
    }
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

/// Put document to platform (broadcast state transition) - TODO: Implement
#[no_mangle]
pub unsafe extern "C" fn ios_sdk_document_put_to_platform(
    _sdk_handle: *mut SDKHandle,
    _document_handle: *const DocumentHandle,
    _document_type_name: *const c_char,
    _entropy: *const [u8; 32],
    _identity_public_key_bytes: *const u8,
    _identity_public_key_len: usize,
    _signer_handle: *const SignerHandle,
) -> IOSSDKResult {
    IOSSDKResult::error(IOSSDKError::new(
        IOSSDKErrorCode::NotImplemented,
        "put_to_platform not yet implemented".to_string(),
    ))
}

// Helper function for freeing strings
use crate::types::ios_sdk_string_free;
