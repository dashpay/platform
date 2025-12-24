use crate::DataContractHandle;
use dash_sdk::dpp::data_contract::accessors::v0::DataContractV0Getters;
use dash_sdk::dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use dash_sdk::platform::DataContract;
use std::ffi::{CStr, CString};
use std::os::raw::c_char;

/// Get schema for a specific document type
///
/// # Safety
/// - `contract_handle` and `document_type` must be valid, non-null pointers.
/// - `document_type` must point to a NUL-terminated C string valid for the duration of the call.
/// - Returns a heap-allocated C string pointer on success; caller must free it using SDK routines.
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_data_contract_get_schema(
    contract_handle: *const DataContractHandle,
    document_type: *const c_char,
) -> *mut c_char {
    if contract_handle.is_null() || document_type.is_null() {
        return std::ptr::null_mut();
    }

    let contract = &*(contract_handle as *const DataContract);

    let document_type_str = match CStr::from_ptr(document_type).to_str() {
        Ok(s) => s,
        Err(_) => return std::ptr::null_mut(),
    };

    match contract.document_type_for_name(document_type_str) {
        Ok(doc_type) => {
            // Convert schema to JSON string
            match serde_json::to_string(doc_type.schema()) {
                Ok(json_str) => match CString::new(json_str) {
                    Ok(s) => s.into_raw(),
                    Err(_) => std::ptr::null_mut(),
                },
                Err(_) => std::ptr::null_mut(),
            }
        }
        Err(_) => std::ptr::null_mut(),
    }
}
