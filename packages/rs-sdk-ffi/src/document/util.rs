use crate::sdk::SDKWrapper;
use crate::{DashSDKError, DashSDKErrorCode, DocumentHandle, FFIError, SDKHandle};
use dash_sdk::platform::Document;

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
