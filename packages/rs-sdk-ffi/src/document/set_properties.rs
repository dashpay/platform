//! Document property update operations

use dash_sdk::dpp::document::Document;
use dash_sdk::dpp::document::DocumentV0Setters;
use dash_sdk::dpp::platform_value::Value;
use std::collections::BTreeMap;
use std::ffi::CStr;
use std::os::raw::c_char;

use crate::types::DocumentHandle;
use crate::{DashSDKError, DashSDKErrorCode, FFIError};

/// Set properties on a document handle
///
/// This function updates the properties of a document. The properties are provided
/// as a JSON string which will be parsed and set on the document.
///
/// # Safety
/// - `document_handle` must be a valid pointer to a DocumentHandle
/// - `properties_json` must be a valid null-terminated C string containing valid JSON
///
/// # Parameters
/// - `document_handle`: Handle to the document to update
/// - `properties_json`: JSON string containing the new properties
///
/// # Returns
/// - NULL on success
/// - DashSDKError pointer on failure (caller must free with dash_sdk_error_free)
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_document_set_properties(
    document_handle: *mut DocumentHandle,
    properties_json: *const c_char,
) -> *mut DashSDKError {
    if document_handle.is_null() || properties_json.is_null() {
        return Box::into_raw(Box::new(DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
            "Document handle or properties JSON is null".to_string(),
        )));
    }

    let document = &mut *(document_handle as *mut Document);

    let properties_str = match CStr::from_ptr(properties_json).to_str() {
        Ok(s) => s,
        Err(e) => {
            return Box::into_raw(Box::new(FFIError::from(e).into()));
        }
    };

    // Parse properties JSON
    let properties_value: serde_json::Value = match serde_json::from_str(properties_str) {
        Ok(v) => v,
        Err(e) => {
            return Box::into_raw(Box::new(DashSDKError::new(
                DashSDKErrorCode::InvalidParameter,
                format!("Invalid properties JSON: {}", e),
            )));
        }
    };

    // Convert JSON to platform Value BTreeMap
    let properties = match serde_json::from_value::<BTreeMap<String, Value>>(properties_value) {
        Ok(map) => map,
        Err(e) => {
            return Box::into_raw(Box::new(DashSDKError::new(
                DashSDKErrorCode::InvalidParameter,
                format!("Failed to convert properties to platform values: {}", e),
            )));
        }
    };

    // Update the document properties
    match document {
        Document::V0(ref mut doc_v0) => {
            doc_v0.set_properties(properties);
        }
    }

    // Return NULL to indicate success
    std::ptr::null_mut()
}

/// Set a single property on a document handle
///
/// This function updates a single property of a document using a path.
/// The path supports lodash-style notation (e.g., "user.address.city").
///
/// # Safety
/// - `document_handle` must be a valid pointer to a DocumentHandle
/// - `path` must be a valid null-terminated C string
/// - `value_json` must be a valid null-terminated C string containing valid JSON
///
/// # Parameters
/// - `document_handle`: Handle to the document to update
/// - `path`: Path to the property (lodash-style)
/// - `value_json`: JSON string containing the new value
///
/// # Returns
/// - NULL on success
/// - DashSDKError pointer on failure (caller must free with dash_sdk_error_free)
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_document_set(
    document_handle: *mut DocumentHandle,
    path: *const c_char,
    value_json: *const c_char,
) -> *mut DashSDKError {
    if document_handle.is_null() || path.is_null() || value_json.is_null() {
        return Box::into_raw(Box::new(DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
            "Document handle, path, or value JSON is null".to_string(),
        )));
    }

    let document = &mut *(document_handle as *mut Document);

    let path_str = match CStr::from_ptr(path).to_str() {
        Ok(s) => s,
        Err(e) => {
            return Box::into_raw(Box::new(FFIError::from(e).into()));
        }
    };

    let value_str = match CStr::from_ptr(value_json).to_str() {
        Ok(s) => s,
        Err(e) => {
            return Box::into_raw(Box::new(FFIError::from(e).into()));
        }
    };

    // Parse value JSON
    let value_json: serde_json::Value = match serde_json::from_str(value_str) {
        Ok(v) => v,
        Err(e) => {
            return Box::into_raw(Box::new(DashSDKError::new(
                DashSDKErrorCode::InvalidParameter,
                format!("Invalid value JSON: {}", e),
            )));
        }
    };

    // Convert JSON to platform Value
    let value = match serde_json::from_value::<Value>(value_json) {
        Ok(v) => v,
        Err(e) => {
            return Box::into_raw(Box::new(DashSDKError::new(
                DashSDKErrorCode::InvalidParameter,
                format!("Failed to convert value to platform value: {}", e),
            )));
        }
    };

    // Update the document property
    match document {
        Document::V0(ref mut doc_v0) => {
            doc_v0.set(path_str, value);
        }
    }

    // Return NULL to indicate success
    std::ptr::null_mut()
}

/// Remove a property from a document handle
///
/// This function removes a property from a document using a path.
/// The path supports lodash-style notation (e.g., "user.address.city").
///
/// # Safety
/// - `document_handle` must be a valid pointer to a DocumentHandle
/// - `path` must be a valid null-terminated C string
///
/// # Parameters
/// - `document_handle`: Handle to the document to update
/// - `path`: Path to the property to remove (lodash-style)
///
/// # Returns
/// - NULL on success
/// - DashSDKError pointer on failure (caller must free with dash_sdk_error_free)
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_document_remove(
    document_handle: *mut DocumentHandle,
    path: *const c_char,
) -> *mut DashSDKError {
    if document_handle.is_null() || path.is_null() {
        return Box::into_raw(Box::new(DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
            "Document handle or path is null".to_string(),
        )));
    }

    let document = &mut *(document_handle as *mut Document);

    let path_str = match CStr::from_ptr(path).to_str() {
        Ok(s) => s,
        Err(e) => {
            return Box::into_raw(Box::new(FFIError::from(e).into()));
        }
    };

    // Remove the property
    match document {
        Document::V0(ref mut doc_v0) => {
            doc_v0.remove(path_str);
        }
    }

    // Return NULL to indicate success
    std::ptr::null_mut()
}
