//! DPNS name registration operations

use crate::{
    signer::VTableSigner, utils, DashSDKError, DashSDKErrorCode, DashSDKResult, FFIError,
    SDKHandle, SDKWrapper,
};
use dash_sdk::dpp::identity::{Identity, IdentityPublicKey};
use dash_sdk::platform::dpns_usernames::RegisterDpnsNameInput;
use std::ffi::CStr;
use std::sync::Arc;

/// Result structure for DPNS registration
#[repr(C)]
pub struct DpnsRegistrationResult {
    /// JSON representation of the preorder document
    pub preorder_document_json: *mut std::os::raw::c_char,
    /// JSON representation of the domain document
    pub domain_document_json: *mut std::os::raw::c_char,
    /// The full domain name (e.g., "alice.dash")
    pub full_domain_name: *mut std::os::raw::c_char,
}

/// Register a DPNS username in a single operation
///
/// This method handles both the preorder and domain registration steps automatically.
/// It generates the necessary entropy, creates both documents, and submits them in order.
///
/// # Safety
/// - `handle` must be a valid SDK handle
/// - `label` must be a valid null-terminated C string
/// - `identity` must be a valid identity handle
/// - `identity_public_key` must be a valid identity public key handle  
/// - `signer` must be a valid signer handle
///
/// # Returns
/// Returns a DpnsRegistrationResult containing both created documents and the full domain name
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_dpns_register_name(
    handle: *const SDKHandle,
    label: *const std::os::raw::c_char,
    identity: *const std::os::raw::c_void,
    identity_public_key: *const std::os::raw::c_void,
    signer: *const std::os::raw::c_void,
) -> DashSDKResult {
    if handle.is_null() {
        return DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
            "SDK handle is null".to_string(),
        ));
    }

    if label.is_null() {
        return DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
            "Label is null".to_string(),
        ));
    }

    if identity.is_null() || identity_public_key.is_null() || signer.is_null() {
        return DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
            "Identity, public key, or signer is null".to_string(),
        ));
    }

    let wrapper = &*(handle as *const SDKWrapper);
    let sdk = &wrapper.sdk;

    // Parse label
    let label_str = match CStr::from_ptr(label).to_str() {
        Ok(s) => s.to_string(),
        Err(e) => {
            return DashSDKResult::error(DashSDKError::new(
                DashSDKErrorCode::InvalidParameter,
                format!("Invalid UTF-8 in label: {}", e),
            ));
        }
    };

    // Validate the username
    if !dash_sdk::platform::dpns_usernames::is_valid_username(&label_str) {
        return DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
            "Invalid username format".to_string(),
        ));
    }

    // Get identity from handle
    let identity_arc = Arc::from_raw(identity as *const Identity);
    let identity_clone = (*identity_arc).clone();
    // Don't drop the Arc, just forget it
    std::mem::forget(identity_arc);

    // Get identity public key from handle
    let key_arc = Arc::from_raw(identity_public_key as *const IdentityPublicKey);
    let key_clone = (*key_arc).clone();
    // Don't drop the Arc, just forget it
    std::mem::forget(key_arc);

    // Get signer from handle
    let signer_arc = Arc::from_raw(signer as *const VTableSigner);
    let signer_clone = (*signer_arc).clone();
    // Don't drop the Arc, just forget it
    std::mem::forget(signer_arc);

    // Create registration input
    let input = RegisterDpnsNameInput {
        label: label_str.clone(),
        identity: identity_clone,
        identity_public_key: key_clone,
        signer: signer_clone,
        preorder_callback: None,
    };

    // Register the name
    let result = wrapper
        .runtime
        .block_on(async { sdk.register_dpns_name(input).await.map_err(FFIError::from) });

    match result {
        Ok(registration_result) => {
            // Serialize documents to JSON
            let preorder_json = match serde_json::to_string(&registration_result.preorder_document)
            {
                Ok(json) => json,
                Err(e) => {
                    return DashSDKResult::error(DashSDKError::new(
                        DashSDKErrorCode::SerializationError,
                        format!("Failed to serialize preorder document: {}", e),
                    ));
                }
            };

            let domain_json = match serde_json::to_string(&registration_result.domain_document) {
                Ok(json) => json,
                Err(e) => {
                    return DashSDKResult::error(DashSDKError::new(
                        DashSDKErrorCode::SerializationError,
                        format!("Failed to serialize domain document: {}", e),
                    ));
                }
            };

            // Convert to C strings
            let preorder_cstring = match utils::c_string_from(preorder_json) {
                Ok(s) => s,
                Err(e) => return DashSDKResult::error(e.into()),
            };

            let domain_cstring = match utils::c_string_from(domain_json) {
                Ok(s) => s,
                Err(e) => {
                    // Clean up preorder string
                    let _ = std::ffi::CString::from_raw(preorder_cstring);
                    return DashSDKResult::error(e.into());
                }
            };

            let domain_name_cstring =
                match utils::c_string_from(registration_result.full_domain_name) {
                    Ok(s) => s,
                    Err(e) => {
                        // Clean up previous strings
                        let _ = std::ffi::CString::from_raw(preorder_cstring);
                        let _ = std::ffi::CString::from_raw(domain_cstring);
                        return DashSDKResult::error(e.into());
                    }
                };

            // Create result structure
            let result = Box::new(DpnsRegistrationResult {
                preorder_document_json: preorder_cstring,
                domain_document_json: domain_cstring,
                full_domain_name: domain_name_cstring,
            });

            DashSDKResult::success(Box::into_raw(result) as *mut std::os::raw::c_void)
        }
        Err(e) => DashSDKResult::error(e.into()),
    }
}

/// Free a DPNS registration result
///
/// # Safety
/// - `result` must be a valid DpnsRegistrationResult pointer created by dash_sdk_dpns_register_name
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_dpns_registration_result_free(
    result: *mut DpnsRegistrationResult,
) {
    if !result.is_null() {
        let result = Box::from_raw(result);

        // Free the C strings
        if !result.preorder_document_json.is_null() {
            let _ = std::ffi::CString::from_raw(result.preorder_document_json);
        }
        if !result.domain_document_json.is_null() {
            let _ = std::ffi::CString::from_raw(result.domain_document_json);
        }
        if !result.full_domain_name.is_null() {
            let _ = std::ffi::CString::from_raw(result.full_domain_name);
        }
    }
}
