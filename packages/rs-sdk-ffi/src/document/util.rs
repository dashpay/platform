use crate::sdk::SDKWrapper;
use crate::{DashSDKError, DashSDKErrorCode, DocumentHandle, FFIError, SDKHandle};
use dash_sdk::dpp::document::{Document, DocumentV0Getters, DocumentV0Setters};
use dash_sdk::dpp::platform_value::Value;
use std::ffi::CStr;
use std::os::raw::c_char;

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

/// Destroy a document handle
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_document_handle_destroy(handle: *mut DocumentHandle) {
    if !handle.is_null() {
        let _ = Box::from_raw(handle as *mut Document);
    }
}

/// Free a document handle (alias for destroy)
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_document_free(handle: *mut DocumentHandle) {
    dash_sdk_document_handle_destroy(handle);
}

/// Set document properties from JSON
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_document_set_properties(
    document_handle: *mut DocumentHandle,
    properties_json: *const c_char,
) -> *mut DashSDKError {
    if document_handle.is_null() || properties_json.is_null() {
        return Box::into_raw(Box::new(DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
            "Invalid parameters".to_string(),
        )));
    }

    let document = &mut *(document_handle as *mut Document);

    let properties_str = match CStr::from_ptr(properties_json).to_str() {
        Ok(s) => s,
        Err(e) => {
            return Box::into_raw(Box::new(DashSDKError::new(
                DashSDKErrorCode::InvalidParameter,
                format!("Invalid UTF-8 in properties JSON: {}", e),
            )));
        }
    };

    // Parse JSON string to Value
    let properties_value: Value = match serde_json::from_str(properties_str) {
        Ok(v) => v,
        Err(e) => {
            return Box::into_raw(Box::new(DashSDKError::new(
                DashSDKErrorCode::InvalidParameter,
                format!("Failed to parse properties JSON: {}", e),
            )));
        }
    };

    // Convert Value to BTreeMap if it's an object
    let properties_map = match properties_value {
        Value::Map(vec_map) => {
            // Convert Vec<(Value, Value)> to BTreeMap<String, Value>
            let mut btree_map = std::collections::BTreeMap::new();
            for (key, value) in vec_map {
                let key_str = match key {
                    Value::Text(s) => s,
                    _ => {
                        return Box::into_raw(Box::new(DashSDKError::new(
                            DashSDKErrorCode::InvalidParameter,
                            "Property keys must be strings".to_string(),
                        )));
                    }
                };
                btree_map.insert(key_str, value);
            }
            btree_map
        }
        _ => {
            return Box::into_raw(Box::new(DashSDKError::new(
                DashSDKErrorCode::InvalidParameter,
                "Properties must be a JSON object".to_string(),
            )));
        }
    };

    // Set the properties on the document
    document.set_properties(properties_map);

    std::ptr::null_mut()
}
