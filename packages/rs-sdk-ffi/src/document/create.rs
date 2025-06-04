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
