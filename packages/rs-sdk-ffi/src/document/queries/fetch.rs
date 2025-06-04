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
