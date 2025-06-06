//! Document fetch operations

use dash_sdk::dpp::document::Document;
use dash_sdk::dpp::platform_value::string_encoding::Encoding;
use dash_sdk::dpp::prelude::{DataContract, Identifier};
use dash_sdk::platform::{DocumentQuery, Fetch};
use std::ffi::CStr;
use std::os::raw::c_char;

use crate::sdk::SDKWrapper;
use crate::types::{DataContractHandle, DocumentHandle, SDKHandle};
use crate::{DashSDKError, DashSDKErrorCode, DashSDKResult, FFIError};

/// Fetch a document by ID
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_document_fetch(
    sdk_handle: *const SDKHandle,
    data_contract_handle: *const DataContractHandle,
    document_type: *const c_char,
    document_id: *const c_char,
) -> DashSDKResult {
    if sdk_handle.is_null()
        || data_contract_handle.is_null()
        || document_type.is_null()
        || document_id.is_null()
    {
        return DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
            "Invalid parameters".to_string(),
        ));
    }

    let wrapper = &*(sdk_handle as *const SDKWrapper);
    let data_contract = &*(data_contract_handle as *const DataContract);

    let document_type_str = match CStr::from_ptr(document_type).to_str() {
        Ok(s) => s,
        Err(e) => return DashSDKResult::error(FFIError::from(e).into()),
    };

    let document_id_str = match CStr::from_ptr(document_id).to_str() {
        Ok(s) => s,
        Err(e) => return DashSDKResult::error(FFIError::from(e).into()),
    };

    let document_id = match Identifier::from_string(document_id_str, Encoding::Base58) {
        Ok(id) => id,
        Err(e) => {
            return DashSDKResult::error(DashSDKError::new(
                DashSDKErrorCode::InvalidParameter,
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
            DashSDKResult::success(handle as *mut std::os::raw::c_void)
        }
        Ok(None) => DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::NotFound,
            "Document not found".to_string(),
        )),
        Err(e) => DashSDKResult::error(e.into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::test_utils::*;
    use crate::DashSDKErrorCode;

    use std::ffi::{CStr, CString};
    use std::ptr;

    #[test]
    fn test_fetch_with_null_sdk_handle() {
        let data_contract = create_mock_data_contract();
        let data_contract_handle = Box::into_raw(data_contract) as *const DataContractHandle;
        let document_type = CString::new("testDoc").unwrap();
        let document_id = CString::new("4EfA9Jrvv3nnCFdSf7fad59851iiTRZ6Wcu6YVJ4iSeF").unwrap();

        let result = unsafe {
            dash_sdk_document_fetch(
                ptr::null(), // null SDK handle
                data_contract_handle,
                document_type.as_ptr(),
                document_id.as_ptr(),
            )
        };

        assert!(!result.error.is_null());
        unsafe {
            let error = &*result.error;
            assert_eq!(error.code, DashSDKErrorCode::InvalidParameter);
            let error_msg = CStr::from_ptr(error.message).to_str().unwrap();
            assert!(error_msg.contains("Invalid parameters"));
        }

        // Clean up
        unsafe {
            let _ = Box::from_raw(data_contract_handle as *mut DataContract);
        }
    }

    #[test]
    fn test_fetch_with_null_data_contract() {
        let sdk_handle = create_mock_sdk_handle();
        let document_type = CString::new("testDoc").unwrap();
        let document_id = CString::new("4EfA9Jrvv3nnCFdSf7fad59851iiTRZ6Wcu6YVJ4iSeF").unwrap();

        let result = unsafe {
            dash_sdk_document_fetch(
                sdk_handle,
                ptr::null(), // null data contract
                document_type.as_ptr(),
                document_id.as_ptr(),
            )
        };

        assert!(!result.error.is_null());
        unsafe {
            let error = &*result.error;
            assert_eq!(error.code, DashSDKErrorCode::InvalidParameter);
        }

        destroy_mock_sdk_handle(sdk_handle);
    }

    #[test]
    fn test_fetch_with_null_document_type() {
        let sdk_handle = create_mock_sdk_handle();
        let data_contract = create_mock_data_contract();
        let data_contract_handle = Box::into_raw(data_contract) as *const DataContractHandle;
        let document_id = CString::new("4EfA9Jrvv3nnCFdSf7fad59851iiTRZ6Wcu6YVJ4iSeF").unwrap();

        let result = unsafe {
            dash_sdk_document_fetch(
                sdk_handle,
                data_contract_handle,
                ptr::null(), // null document type
                document_id.as_ptr(),
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
        }
        destroy_mock_sdk_handle(sdk_handle);
    }

    #[test]
    fn test_fetch_with_null_document_id() {
        let sdk_handle = create_mock_sdk_handle();
        let data_contract = create_mock_data_contract();
        let data_contract_handle = Box::into_raw(data_contract) as *const DataContractHandle;
        let document_type = CString::new("testDoc").unwrap();

        let result = unsafe {
            dash_sdk_document_fetch(
                sdk_handle,
                data_contract_handle,
                document_type.as_ptr(),
                ptr::null(), // null document ID
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
        }
        destroy_mock_sdk_handle(sdk_handle);
    }

    #[test]
    fn test_fetch_with_invalid_document_id() {
        let sdk_handle = create_mock_sdk_handle();
        let data_contract = create_mock_data_contract();
        let data_contract_handle = Box::into_raw(data_contract) as *const DataContractHandle;
        let document_type = CString::new("testDoc").unwrap();
        let document_id = CString::new("invalid-base58-id!@#$").unwrap();

        let result = unsafe {
            dash_sdk_document_fetch(
                sdk_handle,
                data_contract_handle,
                document_type.as_ptr(),
                document_id.as_ptr(),
            )
        };

        assert!(!result.error.is_null());
        unsafe {
            let error = &*result.error;
            assert_eq!(error.code, DashSDKErrorCode::InvalidParameter);
            let error_msg = CStr::from_ptr(error.message).to_str().unwrap();
            assert!(error_msg.contains("Invalid document ID"));
        }

        // Clean up
        unsafe {
            let _ = Box::from_raw(data_contract_handle as *mut DataContract);
        }
        destroy_mock_sdk_handle(sdk_handle);
    }

    #[test]
    fn test_fetch_with_unknown_document_type() {
        let sdk_handle = create_mock_sdk_handle();
        let data_contract = create_mock_data_contract();
        let data_contract_handle = Box::into_raw(data_contract) as *const DataContractHandle;
        let document_type = CString::new("unknownType").unwrap();
        let document_id = CString::new("4EfA9Jrvv3nnCFdSf7fad59851iiTRZ6Wcu6YVJ4iSeF").unwrap();

        let result = unsafe {
            dash_sdk_document_fetch(
                sdk_handle,
                data_contract_handle,
                document_type.as_ptr(),
                document_id.as_ptr(),
            )
        };

        // This should fail when creating the query
        assert!(!result.error.is_null());
        unsafe {
            let error = &*result.error;
            assert_eq!(error.code, DashSDKErrorCode::InternalError);
            let error_msg = CStr::from_ptr(error.message).to_str().unwrap();
            assert!(error_msg.contains("Failed to create query"));
        }

        // Clean up
        unsafe {
            let _ = Box::from_raw(data_contract_handle as *mut DataContract);
        }
        destroy_mock_sdk_handle(sdk_handle);
    }

    #[test]
    fn test_fetch_memory_cleanup() {
        // Test that CString memory is properly managed
        let sdk_handle = create_mock_sdk_handle();
        let data_contract = create_mock_data_contract();
        let data_contract_handle = Box::into_raw(data_contract) as *const DataContractHandle;

        let document_type = CString::new("testDoc").unwrap();
        let document_id = CString::new("4EfA9Jrvv3nnCFdSf7fad59851iiTRZ6Wcu6YVJ4iSeF").unwrap();

        // Get raw pointers
        let document_type_ptr = document_type.as_ptr();
        let document_id_ptr = document_id.as_ptr();

        // CStrings will be dropped at the end of scope, which is proper cleanup
        let _result = unsafe {
            dash_sdk_document_fetch(
                sdk_handle,
                data_contract_handle,
                document_type_ptr,
                document_id_ptr,
            )
        };

        // Clean up
        unsafe {
            let _ = Box::from_raw(data_contract_handle as *mut DataContract);
        }
        destroy_mock_sdk_handle(sdk_handle);
    }
}
