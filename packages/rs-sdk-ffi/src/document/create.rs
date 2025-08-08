//! Document creation operations

use crate::sdk::SDKWrapper;
use crate::types::{
    DashSDKResultDataType, DataContractHandle, DocumentHandle, IdentityHandle, SDKHandle,
};
use crate::{DashSDKError, DashSDKErrorCode, DashSDKResult, FFIError};
use dash_sdk::dpp::data_contract::accessors::v0::DataContractV0Getters;
use dash_sdk::dpp::data_contract::document_type::methods::DocumentTypeV0Methods;
use dash_sdk::dpp::document::{Document, DocumentV0};
use dash_sdk::dpp::identity::accessors::IdentityGettersV0;
use dash_sdk::dpp::platform_value::string_encoding::Encoding;
use dash_sdk::dpp::platform_value::Value;
use dash_sdk::dpp::prelude::{DataContract, Identifier, Identity, Revision};
use drive_proof_verifier::ContextProvider;
use std::collections::BTreeMap;
use std::ffi::CStr;
use std::os::raw::c_char;

/// Document creation result containing handle and entropy
#[repr(C)]
pub struct DashSDKDocumentCreateResult {
    /// Handle to the created document
    pub document_handle: *mut DocumentHandle,
    /// Entropy used for document ID generation (32 bytes)
    pub entropy: [u8; 32],
}

/// Document creation parameters
#[repr(C)]
pub struct DashSDKDocumentCreateParams {
    /// Data contract ID (base58 encoded)
    pub data_contract_id: *const c_char,
    /// Document type name
    pub document_type: *const c_char,
    /// Owner identity ID (base58 encoded)
    pub owner_identity_id: *const c_char,
    /// JSON string of document properties
    pub properties_json: *const c_char,
}

/// Document handle creation parameters
#[repr(C)]
pub struct DashSDKDocumentHandleParams {
    /// Document ID (base58 encoded)
    pub id: *const c_char,
    /// Data contract ID (base58 encoded)
    pub data_contract_id: *const c_char,
    /// Document type name
    pub document_type: *const c_char,
    /// Owner identity ID (base58 encoded)
    pub owner_identity_id: *const c_char,
    /// JSON string of document properties
    pub properties_json: *const c_char,
    /// Optional revision number (0 means no revision)
    pub revision: u64,
}

/// Create a new document
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_document_create(
    sdk_handle: *mut SDKHandle,
    params: *const DashSDKDocumentCreateParams,
) -> DashSDKResult {
    if sdk_handle.is_null() || params.is_null() {
        return DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
            "SDK handle or params is null".to_string(),
        ));
    }

    let params = &*params;
    if params.data_contract_id.is_null()
        || params.document_type.is_null()
        || params.owner_identity_id.is_null()
        || params.properties_json.is_null()
    {
        return DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
            "Required parameter is null".to_string(),
        ));
    }

    let wrapper = &mut *(sdk_handle as *mut SDKWrapper);

    let contract_id_str = match CStr::from_ptr(params.data_contract_id).to_str() {
        Ok(s) => s,
        Err(e) => return DashSDKResult::error(FFIError::from(e).into()),
    };

    let document_type = match CStr::from_ptr(params.document_type).to_str() {
        Ok(s) => s,
        Err(e) => return DashSDKResult::error(FFIError::from(e).into()),
    };

    let owner_id_str = match CStr::from_ptr(params.owner_identity_id).to_str() {
        Ok(s) => s,
        Err(e) => return DashSDKResult::error(FFIError::from(e).into()),
    };

    let properties_str = match CStr::from_ptr(params.properties_json).to_str() {
        Ok(s) => s,
        Err(e) => return DashSDKResult::error(FFIError::from(e).into()),
    };

    // Parse properties JSON
    let properties_value: serde_json::Value = match serde_json::from_str(properties_str) {
        Ok(v) => v,
        Err(e) => {
            return DashSDKResult::error(DashSDKError::new(
                DashSDKErrorCode::InvalidParameter,
                format!("Invalid properties JSON: {}", e),
            ))
        }
    };

    // Convert JSON to platform Value - handle hex strings for byte arrays
    let mut properties = match serde_json::from_value::<BTreeMap<String, Value>>(properties_value) {
        Ok(map) => map,
        Err(e) => {
            return DashSDKResult::error(DashSDKError::new(
                DashSDKErrorCode::InvalidParameter,
                format!("Failed to convert properties: {}", e),
            ))
        }
    };

    let result: Result<(Document, [u8; 32]), FFIError> = wrapper.runtime.block_on(async {
        // Parse contract ID (base58 encoded)
        let contract_id = Identifier::from_string(contract_id_str, Encoding::Base58)
            .map_err(|e| FFIError::InternalError(format!("Invalid contract ID: {}", e)))?;

        // Parse owner identity ID (base58 encoded)
        let owner_id = Identifier::from_string(owner_id_str, Encoding::Base58)
            .map_err(|e| FFIError::InternalError(format!("Invalid owner identity ID: {}", e)))?;

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

        // Get platform version
        let platform_version = wrapper.sdk.version();

        // Generate entropy for document ID (32 random bytes)
        let mut entropy = [0u8; 32];
        getrandom::getrandom(&mut entropy)
            .map_err(|e| FFIError::InternalError(format!("Failed to generate entropy: {}", e)))?;

        let document_type_ref = data_contract
            .document_type_borrowed_for_name(document_type)
            .map_err(|e| FFIError::InternalError(format!("Failed to get document type: {}", e)))?;

        // Sanitize document properties (convert hex/base64 to bytes, base58 to identifiers, etc.)
        use dash_sdk::dpp::data_contract::document_type::methods::DocumentTypeV0Methods;
        document_type_ref.sanitize_document_properties(&mut properties);
        eprintln!("📝 [DOCUMENT CREATE] Sanitized document properties");

        // Create document with entropy - this will generate the document ID internally
        let document = document_type_ref
            .create_document_from_data(
                properties.into(),
                owner_id,
                0, // block_height - will be set by platform
                0, // core_block_height - will be set by platform
                entropy,
                platform_version,
            )
            .map_err(|e| FFIError::InternalError(format!("Failed to create document: {}", e)))?;

        Ok((document, entropy))
    });

    match result {
        Ok((document, entropy)) => {
            let handle = Box::into_raw(Box::new(document)) as *mut DocumentHandle;
            let create_result = Box::new(DashSDKDocumentCreateResult {
                document_handle: handle,
                entropy,
            });
            DashSDKResult::success(Box::into_raw(create_result) as *mut std::os::raw::c_void)
        }
        Err(e) => DashSDKResult::error(e.into()),
    }
}

/// Free a document creation result
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_document_create_result_free(
    result: *mut DashSDKDocumentCreateResult,
) {
    if !result.is_null() {
        let _ = Box::from_raw(result);
    }
}

/// Create a document handle from parameters
/// This creates a Document object directly without broadcasting to the network
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_document_make_handle(
    params: *const DashSDKDocumentHandleParams,
) -> DashSDKResult {
    // Validate input
    if params.is_null() {
        return DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
            "Parameters are null".to_string(),
        ));
    }

    let params = &*params;

    // Validate required fields
    if params.id.is_null()
        || params.data_contract_id.is_null()
        || params.document_type.is_null()
        || params.owner_identity_id.is_null()
        || params.properties_json.is_null()
    {
        return DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
            "One or more required parameters is null".to_string(),
        ));
    }

    // Parse document ID
    let id_str = match CStr::from_ptr(params.id).to_str() {
        Ok(s) => s,
        Err(e) => return DashSDKResult::error(FFIError::from(e).into()),
    };

    let document_id = match Identifier::from_string(id_str, Encoding::Base58) {
        Ok(id) => id,
        Err(e) => {
            return DashSDKResult::error(DashSDKError::new(
                DashSDKErrorCode::InvalidParameter,
                format!("Invalid document ID: {}", e),
            ))
        }
    };

    // Parse owner identity ID
    let owner_id_str = match CStr::from_ptr(params.owner_identity_id).to_str() {
        Ok(s) => s,
        Err(e) => return DashSDKResult::error(FFIError::from(e).into()),
    };

    let owner_id = match Identifier::from_string(owner_id_str, Encoding::Base58) {
        Ok(id) => id,
        Err(e) => {
            return DashSDKResult::error(DashSDKError::new(
                DashSDKErrorCode::InvalidParameter,
                format!("Invalid owner identity ID: {}", e),
            ))
        }
    };

    // Parse properties JSON
    let properties_json_str = match CStr::from_ptr(params.properties_json).to_str() {
        Ok(s) => s,
        Err(e) => return DashSDKResult::error(FFIError::from(e).into()),
    };

    // Parse JSON into Value
    let properties_value: Value = match serde_json::from_str(properties_json_str) {
        Ok(val) => val,
        Err(e) => {
            return DashSDKResult::error(DashSDKError::new(
                DashSDKErrorCode::InvalidParameter,
                format!("Invalid JSON properties: {}", e),
            ))
        }
    };

    // Convert Value to BTreeMap<String, Value>
    let properties = match properties_value {
        Value::Map(map) => {
            let mut btree_map = BTreeMap::new();
            for (key, value) in map {
                match key {
                    Value::Text(key_str) => {
                        btree_map.insert(key_str, value);
                    }
                    _ => {
                        return DashSDKResult::error(DashSDKError::new(
                            DashSDKErrorCode::InvalidParameter,
                            "Property keys must be strings".to_string(),
                        ))
                    }
                }
            }
            btree_map
        }
        _ => {
            return DashSDKResult::error(DashSDKError::new(
                DashSDKErrorCode::InvalidParameter,
                "Properties must be a JSON object".to_string(),
            ))
        }
    };

    // Handle optional revision
    let revision = if params.revision == 0 {
        None
    } else {
        Some(params.revision)
    };

    // Create the document
    let document = Document::V0(DocumentV0 {
        id: document_id,
        owner_id,
        properties,
        revision,
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

    // Box and return as handle
    let handle = Box::into_raw(Box::new(document)) as *mut DocumentHandle;
    DashSDKResult::success_handle(
        handle as *mut std::os::raw::c_void,
        DashSDKResultDataType::ResultDocumentHandle,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::test_utils::*;
    use crate::DashSDKErrorCode;
    use std::ffi::{CStr, CString};
    use std::ptr;

    // Helper function to create valid document create params
    fn create_valid_document_params() -> (
        DashSDKDocumentCreateParams,
        CString,
        CString,
        CString,
        CString,
    ) {
        let data_contract_id =
            CString::new("4EfA9Jrvv3nnCFdSf7fad59851iiTRZ6Wcu6YVJ4iSeF").unwrap();
        let owner_identity_id =
            CString::new("BhC9M3fQHyUCyuxH4WHdhn1VGgJ4JTLmer8qmTTHkYTe").unwrap();
        let document_type = CString::new("testDoc").unwrap();
        let properties_json = CString::new(r#"{"name": "John Doe", "age": 30}"#).unwrap();

        let params = DashSDKDocumentCreateParams {
            data_contract_id: data_contract_id.as_ptr(),
            document_type: document_type.as_ptr(),
            owner_identity_id: owner_identity_id.as_ptr(),
            properties_json: properties_json.as_ptr(),
        };

        (
            params,
            data_contract_id,
            owner_identity_id,
            document_type,
            properties_json,
        )
    }

    #[test]
    fn test_document_create_with_null_sdk_handle() {
        let (params, _contract_id, _owner_id, _document_type, _properties_json) =
            create_valid_document_params();

        let result = unsafe {
            dash_sdk_document_create(
                ptr::null_mut(), // null SDK handle
                &params,
            )
        };

        assert!(!result.error.is_null());
        unsafe {
            let error = &*result.error;
            assert_eq!(error.code, DashSDKErrorCode::InvalidParameter);
            let error_msg = CStr::from_ptr(error.message).to_str().unwrap();
            assert!(error_msg.contains("null"));
        }
    }

    #[test]
    fn test_document_create_with_null_params() {
        let sdk_handle = create_mock_sdk_handle();

        let result = unsafe {
            dash_sdk_document_create(
                sdk_handle,
                ptr::null(), // null params
            )
        };

        assert!(!result.error.is_null());
        unsafe {
            let error = &*result.error;
            assert_eq!(error.code, DashSDKErrorCode::InvalidParameter);
            let error_msg = CStr::from_ptr(error.message).to_str().unwrap();
            assert!(error_msg.contains("null"));
        }

        destroy_mock_sdk_handle(sdk_handle);
    }

    #[test]
    fn test_document_create_with_null_data_contract_id() {
        let sdk_handle = create_mock_sdk_handle();
        let owner_identity_id =
            CString::new("BhC9M3fQHyUCyuxH4WHdhn1VGgJ4JTLmer8qmTTHkYTe").unwrap();
        let document_type = CString::new("testDoc").unwrap();
        let properties_json = CString::new(r#"{"name": "John Doe"}"#).unwrap();

        let params = DashSDKDocumentCreateParams {
            data_contract_id: ptr::null(),
            document_type: document_type.as_ptr(),
            owner_identity_id: owner_identity_id.as_ptr(),
            properties_json: properties_json.as_ptr(),
        };

        let result = unsafe { dash_sdk_document_create(sdk_handle, &params) };

        assert!(!result.error.is_null());
        unsafe {
            let error = &*result.error;
            assert_eq!(error.code, DashSDKErrorCode::InvalidParameter);
            let error_msg = CStr::from_ptr(error.message).to_str().unwrap();
            assert!(error_msg.contains("Required parameter is null"));
        }

        destroy_mock_sdk_handle(sdk_handle);
    }

    #[test]
    fn test_document_create_with_null_document_type() {
        let sdk_handle = create_mock_sdk_handle();
        let data_contract_id =
            CString::new("4EfA9Jrvv3nnCFdSf7fad59851iiTRZ6Wcu6YVJ4iSeF").unwrap();
        let owner_identity_id =
            CString::new("BhC9M3fQHyUCyuxH4WHdhn1VGgJ4JTLmer8qmTTHkYTe").unwrap();
        let properties_json = CString::new(r#"{"name": "John Doe"}"#).unwrap();

        let params = DashSDKDocumentCreateParams {
            data_contract_id: data_contract_id.as_ptr(),
            document_type: ptr::null(),
            owner_identity_id: owner_identity_id.as_ptr(),
            properties_json: properties_json.as_ptr(),
        };

        let result = unsafe { dash_sdk_document_create(sdk_handle, &params) };

        assert!(!result.error.is_null());
        unsafe {
            let error = &*result.error;
            assert_eq!(error.code, DashSDKErrorCode::InvalidParameter);
        }

        destroy_mock_sdk_handle(sdk_handle);
    }

    #[test]
    fn test_document_create_with_null_owner_identity_id() {
        let sdk_handle = create_mock_sdk_handle();
        let data_contract_id =
            CString::new("4EfA9Jrvv3nnCFdSf7fad59851iiTRZ6Wcu6YVJ4iSeF").unwrap();
        let document_type = CString::new("testDoc").unwrap();
        let properties_json = CString::new(r#"{"name": "John Doe"}"#).unwrap();

        let params = DashSDKDocumentCreateParams {
            data_contract_id: data_contract_id.as_ptr(),
            document_type: document_type.as_ptr(),
            owner_identity_id: ptr::null(),
            properties_json: properties_json.as_ptr(),
        };

        let result = unsafe { dash_sdk_document_create(sdk_handle, &params) };

        assert!(!result.error.is_null());
        unsafe {
            let error = &*result.error;
            assert_eq!(error.code, DashSDKErrorCode::InvalidParameter);
        }

        destroy_mock_sdk_handle(sdk_handle);
    }

    #[test]
    fn test_document_create_with_null_properties_json() {
        let sdk_handle = create_mock_sdk_handle();
        let data_contract_id =
            CString::new("4EfA9Jrvv3nnCFdSf7fad59851iiTRZ6Wcu6YVJ4iSeF").unwrap();
        let owner_identity_id =
            CString::new("BhC9M3fQHyUCyuxH4WHdhn1VGgJ4JTLmer8qmTTHkYTe").unwrap();
        let document_type = CString::new("testDoc").unwrap();

        let params = DashSDKDocumentCreateParams {
            data_contract_id: data_contract_id.as_ptr(),
            document_type: document_type.as_ptr(),
            owner_identity_id: owner_identity_id.as_ptr(),
            properties_json: ptr::null(),
        };

        let result = unsafe { dash_sdk_document_create(sdk_handle, &params) };

        assert!(!result.error.is_null());
        unsafe {
            let error = &*result.error;
            assert_eq!(error.code, DashSDKErrorCode::InvalidParameter);
        }

        destroy_mock_sdk_handle(sdk_handle);
    }

    #[test]
    fn test_document_create_with_invalid_json() {
        let sdk_handle = create_mock_sdk_handle();
        let data_contract_id =
            CString::new("4EfA9Jrvv3nnCFdSf7fad59851iiTRZ6Wcu6YVJ4iSeF").unwrap();
        let owner_identity_id =
            CString::new("BhC9M3fQHyUCyuxH4WHdhn1VGgJ4JTLmer8qmTTHkYTe").unwrap();
        let document_type = CString::new("testDoc").unwrap();
        let properties_json = CString::new("{invalid json}").unwrap();

        let params = DashSDKDocumentCreateParams {
            data_contract_id: data_contract_id.as_ptr(),
            document_type: document_type.as_ptr(),
            owner_identity_id: owner_identity_id.as_ptr(),
            properties_json: properties_json.as_ptr(),
        };

        let result = unsafe { dash_sdk_document_create(sdk_handle, &params) };

        assert!(!result.error.is_null());
        unsafe {
            let error = &*result.error;
            assert_eq!(error.code, DashSDKErrorCode::InvalidParameter);
            let error_msg = CStr::from_ptr(error.message).to_str().unwrap();
            assert!(error_msg.contains("Invalid properties JSON"));
        }

        destroy_mock_sdk_handle(sdk_handle);
    }

    // Note: Validation tests for missing required fields and additional properties
    // are removed because they test SDK behavior rather than FFI layer behavior.
    // The FFI layer tests should focus on parameter validation and proper data
    // passing, not on the underlying document validation logic.

    #[test]
    fn test_document_create_with_unknown_document_type() {
        let sdk_handle = create_mock_sdk_handle();
        let data_contract_id =
            CString::new("4EfA9Jrvv3nnCFdSf7fad59851iiTRZ6Wcu6YVJ4iSeF").unwrap();
        let owner_identity_id =
            CString::new("BhC9M3fQHyUCyuxH4WHdhn1VGgJ4JTLmer8qmTTHkYTe").unwrap();
        let document_type = CString::new("unknownType").unwrap();
        let properties_json = CString::new(r#"{"name": "John Doe"}"#).unwrap();

        let params = DashSDKDocumentCreateParams {
            data_contract_id: data_contract_id.as_ptr(),
            document_type: document_type.as_ptr(),
            owner_identity_id: owner_identity_id.as_ptr(),
            properties_json: properties_json.as_ptr(),
        };

        let result = unsafe { dash_sdk_document_create(sdk_handle, &params) };

        assert!(!result.error.is_null());
        unsafe {
            let error = &*result.error;
            assert_eq!(error.code, DashSDKErrorCode::InternalError);
            let error_msg = CStr::from_ptr(error.message).to_str().unwrap();
            assert!(error_msg.contains("Failed to") || error_msg.contains("not found"));
        }

        destroy_mock_sdk_handle(sdk_handle);
    }
}
