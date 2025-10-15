//! Data contract operations

mod put;
mod queries;
mod util;

use std::ffi::CStr;
use std::os::raw::c_char;

use dash_sdk::dpp::data_contract::DataContractFactory;
use dash_sdk::dpp::identity::accessors::IdentityGettersV0;
use dash_sdk::dpp::platform_value;
use dash_sdk::dpp::prelude::{DataContract, Identity};

use crate::sdk::SDKWrapper;
use crate::types::{DataContractHandle, IdentityHandle, SDKHandle};
use crate::{DashSDKError, DashSDKErrorCode, DashSDKResult, FFIError};

/// Data contract information
#[repr(C)]
pub struct DashSDKDataContractInfo {
    /// Contract ID as hex string (null-terminated)
    pub id: *mut c_char,
    /// Owner ID as hex string (null-terminated)
    pub owner_id: *mut c_char,
    /// Contract version
    pub version: u32,
    /// Schema version
    pub schema_version: u32,
    /// Number of document types
    pub document_types_count: u32,
}

/// Create a new data contract
///
/// # Safety
/// - `sdk_handle`, `owner_identity_handle`, and `documents_schema_json` must be valid, non-null pointers.
/// - `documents_schema_json` must point to a NUL-terminated C string valid for the duration of the call.
/// - On success, returns a heap-allocated handle which must be destroyed with the SDK's destroy function.
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_data_contract_create(
    sdk_handle: *mut SDKHandle,
    owner_identity_handle: *const IdentityHandle,
    documents_schema_json: *const c_char,
) -> DashSDKResult {
    if sdk_handle.is_null() || owner_identity_handle.is_null() || documents_schema_json.is_null() {
        return DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
            "Invalid parameters".to_string(),
        ));
    }

    let wrapper = &mut *(sdk_handle as *mut SDKWrapper);
    let identity = &*(owner_identity_handle as *const Identity);

    let schema_str = match CStr::from_ptr(documents_schema_json).to_str() {
        Ok(s) => s,
        Err(e) => return DashSDKResult::error(FFIError::from(e).into()),
    };

    // Parse the JSON schema
    let schema_value: serde_json::Value = match serde_json::from_str(schema_str) {
        Ok(v) => v,
        Err(e) => {
            return DashSDKResult::error(DashSDKError::new(
                DashSDKErrorCode::InvalidParameter,
                format!("Invalid schema JSON: {}", e),
            ))
        }
    };

    // Convert to platform Value
    let documents_value = match serde_json::from_value::<platform_value::Value>(schema_value) {
        Ok(v) => v,
        Err(e) => {
            return DashSDKResult::error(DashSDKError::new(
                DashSDKErrorCode::InvalidParameter,
                format!("Failed to convert schema: {}", e),
            ))
        }
    };

    let result: Result<DataContract, FFIError> = wrapper.runtime.block_on(async {
        // Get protocol version from SDK
        let platform_version = wrapper.sdk.version();

        // Create data contract factory
        let factory = DataContractFactory::new(platform_version.protocol_version)
            .map_err(|e| FFIError::InternalError(format!("Failed to create factory: {}", e)))?;

        // Get identity nonce
        let identity_nonce = identity.revision();

        // Create the data contract
        let created_contract = factory
            .create(
                identity.id(),
                identity_nonce,
                documents_value,
                None, // config
                None, // definitions
            )
            .map_err(|e| FFIError::InternalError(format!("Failed to create contract: {}", e)))?;

        // Note: Actually publishing the contract would require signing and broadcasting
        // For now, we just return the created contract's data contract part
        Ok(created_contract.data_contract().clone())
    });

    match result {
        Ok(contract) => {
            let handle = Box::into_raw(Box::new(contract)) as *mut DataContractHandle;
            DashSDKResult::success(handle as *mut std::os::raw::c_void)
        }
        Err(e) => DashSDKResult::error(e.into()),
    }
}

/// Destroy a data contract handle
///
/// # Safety
/// - `handle` must be a pointer previously returned by this SDK or null (no-op).
/// - After this call, `handle` becomes invalid and must not be used again.
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_data_contract_destroy(handle: *mut DataContractHandle) {
    if !handle.is_null() {
        let _ = Box::from_raw(handle as *mut DataContract);
    }
}

// Re-export query functions
pub use queries::{
    dash_sdk_data_contract_fetch, dash_sdk_data_contract_fetch_history,
    dash_sdk_data_contract_fetch_json, dash_sdk_data_contracts_fetch_many,
};
