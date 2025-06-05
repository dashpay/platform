//! Document creation operations

use dash_sdk::dpp::document::{document_factory::DocumentFactory, Document};
use dash_sdk::dpp::identity::accessors::IdentityGettersV0;
use dash_sdk::dpp::platform_value::Value;
use dash_sdk::dpp::prelude::{DataContract, Identity};
use std::collections::BTreeMap;
use std::ffi::CStr;
use std::os::raw::c_char;

use crate::sdk::SDKWrapper;
use crate::types::{DataContractHandle, DocumentHandle, IdentityHandle, SDKHandle};
use crate::{DashSDKError, DashSDKErrorCode, DashSDKResult, FFIError};

/// Document creation parameters
#[repr(C)]
pub struct DashSDKDocumentCreateParams {
    /// Data contract handle
    pub data_contract_handle: *const DataContractHandle,
    /// Document type name
    pub document_type: *const c_char,
    /// Owner identity handle
    pub owner_identity_handle: *const IdentityHandle,
    /// JSON string of document properties
    pub properties_json: *const c_char,
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
    if params.data_contract_handle.is_null()
        || params.document_type.is_null()
        || params.owner_identity_handle.is_null()
        || params.properties_json.is_null()
    {
        return DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
            "Required parameter is null".to_string(),
        ));
    }

    let wrapper = &mut *(sdk_handle as *mut SDKWrapper);
    let data_contract = &*(params.data_contract_handle as *const DataContract);
    let identity = &*(params.owner_identity_handle as *const Identity);

    let document_type = match CStr::from_ptr(params.document_type).to_str() {
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

    // Convert JSON to platform Value
    let properties = match serde_json::from_value::<BTreeMap<String, Value>>(properties_value) {
        Ok(map) => map,
        Err(e) => {
            return DashSDKResult::error(DashSDKError::new(
                DashSDKErrorCode::InvalidParameter,
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
            DashSDKResult::success(handle as *mut std::os::raw::c_void)
        }
        Err(e) => DashSDKResult::error(e.into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::test_utils;
    use crate::test_utils::test_utils::*;
    use crate::DashSDKErrorCode;
    use dash_sdk::dpp::data_contract::accessors::v1::DataContractV1Getters;
    use dash_sdk::dpp::data_contract::conversion::json::DataContractJsonConversionMethodsV0;
    use dash_sdk::dpp::data_contract::document_type::DocumentTypeRef;
    use dash_sdk::dpp::data_contract::v1::DataContractV1;
    use dash_sdk::dpp::document::document_factory::DocumentFactory;
    use dash_sdk::dpp::identity::{Identity, IdentityV0};
    use dash_sdk::dpp::platform_value::BinaryData;
    use dash_sdk::dpp::prelude::Identifier;
    use dash_sdk::dpp::version::PlatformVersion;
    use std::collections::BTreeMap;
    use std::ffi::{CStr, CString};
    use std::ptr;

    // Helper function to create a mock identity
    fn create_mock_identity() -> Box<Identity> {
        let id = Identifier::from_bytes(&[1u8; 32]).unwrap();
        let identity = Identity::V0(IdentityV0 {
            id,
            public_keys: BTreeMap::new(),
            balance: 0,
            revision: 0,
        });
        Box::new(identity)
    }

    // Helper function to create valid document create params
    fn create_valid_document_params(
        data_contract_handle: *const DataContractHandle,
        owner_identity_handle: *const IdentityHandle,
    ) -> (DashSDKDocumentCreateParams, CString, CString) {
        let document_type = CString::new("testDoc").unwrap();
        let properties_json = CString::new(r#"{"name": "John Doe", "age": 30}"#).unwrap();

        let params = DashSDKDocumentCreateParams {
            data_contract_handle,
            document_type: document_type.as_ptr(),
            owner_identity_handle,
            properties_json: properties_json.as_ptr(),
        };

        (params, document_type, properties_json)
    }

    #[test]
    fn test_document_create_with_null_sdk_handle() {
        let data_contract = test_utils::create_mock_data_contract();
        let owner_identity = create_mock_identity();
        let data_contract_handle = Box::into_raw(data_contract) as *const DataContractHandle;
        let owner_identity_handle = Box::into_raw(owner_identity) as *const IdentityHandle;

        let (params, _document_type, _properties_json) =
            create_valid_document_params(data_contract_handle, owner_identity_handle);

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

        // Clean up
        unsafe {
            let _ = Box::from_raw(data_contract_handle as *mut DataContract);
            let _ = Box::from_raw(owner_identity_handle as *mut Identity);
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
    fn test_document_create_with_null_data_contract() {
        let sdk_handle = create_mock_sdk_handle();
        let owner_identity = create_mock_identity();
        let owner_identity_handle = Box::into_raw(owner_identity) as *const IdentityHandle;

        let document_type = CString::new("testDoc").unwrap();
        let properties_json = CString::new(r#"{"name": "John Doe"}"#).unwrap();

        let params = DashSDKDocumentCreateParams {
            data_contract_handle: ptr::null(),
            document_type: document_type.as_ptr(),
            owner_identity_handle,
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

        // Clean up
        unsafe {
            let _ = Box::from_raw(owner_identity_handle as *mut Identity);
        }
        destroy_mock_sdk_handle(sdk_handle);
    }

    #[test]
    fn test_document_create_with_null_document_type() {
        let sdk_handle = create_mock_sdk_handle();
        let data_contract = test_utils::create_mock_data_contract();
        let owner_identity = create_mock_identity();
        let data_contract_handle = Box::into_raw(data_contract) as *const DataContractHandle;
        let owner_identity_handle = Box::into_raw(owner_identity) as *const IdentityHandle;

        let properties_json = CString::new(r#"{"name": "John Doe"}"#).unwrap();

        let params = DashSDKDocumentCreateParams {
            data_contract_handle,
            document_type: ptr::null(),
            owner_identity_handle,
            properties_json: properties_json.as_ptr(),
        };

        let result = unsafe { dash_sdk_document_create(sdk_handle, &params) };

        assert!(!result.error.is_null());
        unsafe {
            let error = &*result.error;
            assert_eq!(error.code, DashSDKErrorCode::InvalidParameter);
        }

        // Clean up
        unsafe {
            let _ = Box::from_raw(data_contract_handle as *mut DataContract);
            let _ = Box::from_raw(owner_identity_handle as *mut Identity);
        }
        destroy_mock_sdk_handle(sdk_handle);
    }

    #[test]
    fn test_document_create_with_null_owner_identity() {
        let sdk_handle = create_mock_sdk_handle();
        let data_contract = test_utils::create_mock_data_contract();
        let data_contract_handle = Box::into_raw(data_contract) as *const DataContractHandle;

        let document_type = CString::new("testDoc").unwrap();
        let properties_json = CString::new(r#"{"name": "John Doe"}"#).unwrap();

        let params = DashSDKDocumentCreateParams {
            data_contract_handle,
            document_type: document_type.as_ptr(),
            owner_identity_handle: ptr::null(),
            properties_json: properties_json.as_ptr(),
        };

        let result = unsafe { dash_sdk_document_create(sdk_handle, &params) };

        assert!(!result.error.is_null());
        unsafe {
            let error = &*result.error;
            assert_eq!(error.code, DashSDKErrorCode::InvalidParameter);
        }

        // Clean up
        unsafe {
            let _ = Box::from_raw(data_contract_handle as *mut DataContract);
        }
        destroy_mock_sdk_handle(sdk_handle);
    }

    #[test]
    fn test_document_create_with_null_properties_json() {
        let sdk_handle = create_mock_sdk_handle();
        let data_contract = test_utils::create_mock_data_contract();
        let owner_identity = create_mock_identity();
        let data_contract_handle = Box::into_raw(data_contract) as *const DataContractHandle;
        let owner_identity_handle = Box::into_raw(owner_identity) as *const IdentityHandle;

        let document_type = CString::new("testDoc").unwrap();

        let params = DashSDKDocumentCreateParams {
            data_contract_handle,
            document_type: document_type.as_ptr(),
            owner_identity_handle,
            properties_json: ptr::null(),
        };

        let result = unsafe { dash_sdk_document_create(sdk_handle, &params) };

        assert!(!result.error.is_null());
        unsafe {
            let error = &*result.error;
            assert_eq!(error.code, DashSDKErrorCode::InvalidParameter);
        }

        // Clean up
        unsafe {
            let _ = Box::from_raw(data_contract_handle as *mut DataContract);
            let _ = Box::from_raw(owner_identity_handle as *mut Identity);
        }
        destroy_mock_sdk_handle(sdk_handle);
    }

    #[test]
    fn test_document_create_with_invalid_json() {
        let sdk_handle = create_mock_sdk_handle();
        let data_contract = test_utils::create_mock_data_contract();
        let owner_identity = create_mock_identity();
        let data_contract_handle = Box::into_raw(data_contract) as *const DataContractHandle;
        let owner_identity_handle = Box::into_raw(owner_identity) as *const IdentityHandle;

        let document_type = CString::new("testDoc").unwrap();
        let properties_json = CString::new("{invalid json}").unwrap();

        let params = DashSDKDocumentCreateParams {
            data_contract_handle,
            document_type: document_type.as_ptr(),
            owner_identity_handle,
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

        // Clean up
        unsafe {
            let _ = Box::from_raw(data_contract_handle as *mut DataContract);
            let _ = Box::from_raw(owner_identity_handle as *mut Identity);
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
        let data_contract = test_utils::create_mock_data_contract();
        let owner_identity = create_mock_identity();
        let data_contract_handle = Box::into_raw(data_contract) as *const DataContractHandle;
        let owner_identity_handle = Box::into_raw(owner_identity) as *const IdentityHandle;

        let document_type = CString::new("unknownType").unwrap();
        let properties_json = CString::new(r#"{"name": "John Doe"}"#).unwrap();

        let params = DashSDKDocumentCreateParams {
            data_contract_handle,
            document_type: document_type.as_ptr(),
            owner_identity_handle,
            properties_json: properties_json.as_ptr(),
        };

        let result = unsafe { dash_sdk_document_create(sdk_handle, &params) };

        assert!(!result.error.is_null());
        unsafe {
            let error = &*result.error;
            assert_eq!(error.code, DashSDKErrorCode::InternalError);
            let error_msg = CStr::from_ptr(error.message).to_str().unwrap();
            assert!(error_msg.contains("Failed to create document"));
        }

        // Clean up
        unsafe {
            let _ = Box::from_raw(data_contract_handle as *mut DataContract);
            let _ = Box::from_raw(owner_identity_handle as *mut Identity);
        }
        destroy_mock_sdk_handle(sdk_handle);
    }
}
