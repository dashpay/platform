//! Document information and lifecycle operations

use dash_sdk::dpp::document::{Document, DocumentV0Getters};
use dash_sdk::dpp::platform_value::string_encoding::Encoding;
use std::ffi::CString;

use crate::sdk::SDKWrapper;
use crate::types::{DashSDKDocumentInfo, DocumentHandle, SDKHandle};
use crate::{DashSDKError, DashSDKErrorCode, FFIError};

/// Destroy a document
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_document_destroy(
    sdk_handle: *mut SDKHandle,
    document_handle: *mut DocumentHandle,
) -> *mut DashSDKError {
    if sdk_handle.is_null() || document_handle.is_null() {
        return Box::into_raw(Box::new(DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
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
pub unsafe extern "C" fn dash_sdk_document_get_info(
    document_handle: *const DocumentHandle,
) -> *mut DashSDKDocumentInfo {
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
            crate::types::dash_sdk_string_free(id_str);
            return std::ptr::null_mut();
        }
    };

    // Document doesn't have data_contract_id, use placeholder
    let data_contract_id_str = match CString::new("unknown") {
        Ok(s) => s.into_raw(),
        Err(_) => {
            crate::types::dash_sdk_string_free(id_str);
            crate::types::dash_sdk_string_free(owner_id_str);
            return std::ptr::null_mut();
        }
    };

    // Document doesn't have document_type_name, use placeholder
    let document_type_str = match CString::new("unknown") {
        Ok(s) => s.into_raw(),
        Err(_) => {
            crate::types::dash_sdk_string_free(id_str);
            crate::types::dash_sdk_string_free(owner_id_str);
            crate::types::dash_sdk_string_free(data_contract_id_str);
            return std::ptr::null_mut();
        }
    };

    let info = DashSDKDocumentInfo {
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
pub unsafe extern "C" fn dash_sdk_document_handle_destroy(handle: *mut DocumentHandle) {
    if !handle.is_null() {
        let _ = Box::from_raw(handle as *mut Document);
    }
}
