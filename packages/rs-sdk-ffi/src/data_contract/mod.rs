//! Data contract operations

use std::ffi::{CStr, CString};
use std::os::raw::c_char;

use dash_sdk::dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use dash_sdk::dpp::data_contract::{accessors::v0::DataContractV0Getters, DataContractFactory};
use dash_sdk::dpp::identity::accessors::IdentityGettersV0;
use dash_sdk::dpp::platform_value;
use dash_sdk::dpp::platform_value::string_encoding::Encoding;
use dash_sdk::dpp::prelude::{DataContract, Identifier, Identity};
use dash_sdk::platform::{Fetch, IdentityPublicKey};

use crate::sdk::SDKWrapper;
use crate::types::{
    DashSDKResultDataType, DataContractHandle, IdentityHandle, SDKHandle, SignerHandle,
};
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

/// Fetch a data contract by ID
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_data_contract_fetch(
    sdk_handle: *const SDKHandle,
    contract_id: *const c_char,
) -> DashSDKResult {
    if sdk_handle.is_null() || contract_id.is_null() {
        return DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
            "SDK handle or contract ID is null".to_string(),
        ));
    }

    let wrapper = &*(sdk_handle as *const SDKWrapper);

    let id_str = match CStr::from_ptr(contract_id).to_str() {
        Ok(s) => s,
        Err(e) => return DashSDKResult::error(FFIError::from(e).into()),
    };

    let id = match Identifier::from_string(id_str, Encoding::Base58) {
        Ok(id) => id,
        Err(e) => {
            return DashSDKResult::error(DashSDKError::new(
                DashSDKErrorCode::InvalidParameter,
                format!("Invalid contract ID: {}", e),
            ))
        }
    };

    let result = wrapper.runtime.block_on(async {
        DataContract::fetch(&wrapper.sdk, id)
            .await
            .map_err(FFIError::from)
    });

    match result {
        Ok(Some(contract)) => {
            let handle = Box::into_raw(Box::new(contract)) as *mut DataContractHandle;
            DashSDKResult::success(handle as *mut std::os::raw::c_void)
        }
        Ok(None) => DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::NotFound,
            "Data contract not found".to_string(),
        )),
        Err(e) => DashSDKResult::error(e.into()),
    }
}

/// Create a new data contract
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
        let identity_nonce = identity.revision() as u64;

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

/// Get data contract information
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_data_contract_get_info(
    contract_handle: *const DataContractHandle,
) -> *mut DashSDKDataContractInfo {
    if contract_handle.is_null() {
        return std::ptr::null_mut();
    }

    let contract = &*(contract_handle as *const DataContract);

    let id_str = match CString::new(contract.id().to_string(Encoding::Base58)) {
        Ok(s) => s.into_raw(),
        Err(_) => return std::ptr::null_mut(),
    };

    let owner_id_str = match CString::new(contract.owner_id().to_string(Encoding::Base58)) {
        Ok(s) => s.into_raw(),
        Err(_) => {
            dash_sdk_string_free(id_str);
            return std::ptr::null_mut();
        }
    };

    let info = DashSDKDataContractInfo {
        id: id_str,
        owner_id: owner_id_str,
        version: contract.version(),
        schema_version: contract.version() as u32, // Use version as schema version for now
        document_types_count: contract.document_types().len() as u32,
    };

    Box::into_raw(Box::new(info))
}

/// Get schema for a specific document type
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

/// Destroy a data contract handle
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_data_contract_destroy(handle: *mut DataContractHandle) {
    if !handle.is_null() {
        let _ = Box::from_raw(handle as *mut DataContract);
    }
}

/// Free a data contract info structure
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_data_contract_info_free(info: *mut DashSDKDataContractInfo) {
    if info.is_null() {
        return;
    }

    let info = Box::from_raw(info);
    dash_sdk_string_free(info.id);
    dash_sdk_string_free(info.owner_id);
}

/// Put data contract to platform (broadcast state transition)
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_data_contract_put_to_platform(
    sdk_handle: *mut SDKHandle,
    data_contract_handle: *const DataContractHandle,
    identity_public_key_handle: *const crate::types::IdentityPublicKeyHandle,
    signer_handle: *const SignerHandle,
) -> DashSDKResult {
    // Validate parameters
    if sdk_handle.is_null()
        || data_contract_handle.is_null()
        || identity_public_key_handle.is_null()
        || signer_handle.is_null()
    {
        return DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
            "One or more required parameters is null".to_string(),
        ));
    }

    let wrapper = &mut *(sdk_handle as *mut SDKWrapper);
    let data_contract = &*(data_contract_handle as *const DataContract);
    let identity_public_key = &*(identity_public_key_handle as *const IdentityPublicKey);
    let signer = &*(signer_handle as *const super::signer::IOSSigner);

    let result: Result<Vec<u8>, FFIError> = wrapper.runtime.block_on(async {
        // Put data contract to platform using the PutContract trait
        use dash_sdk::platform::transition::put_contract::PutContract;

        let state_transition = data_contract
            .put_to_platform(
                &wrapper.sdk,
                identity_public_key.clone(),
                signer,
                None, // settings (use defaults)
            )
            .await
            .map_err(|e| {
                FFIError::InternalError(format!("Failed to put data contract to platform: {}", e))
            })?;

        // Serialize the state transition with bincode
        let config = bincode::config::standard();
        bincode::encode_to_vec(&state_transition, config).map_err(|e| {
            FFIError::InternalError(format!("Failed to serialize state transition: {}", e))
        })
    });

    match result {
        Ok(serialized_data) => DashSDKResult::success_binary(serialized_data),
        Err(e) => DashSDKResult::error(e.into()),
    }
}

/// Put data contract to platform and wait for confirmation (broadcast state transition and wait for response)
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_data_contract_put_to_platform_and_wait(
    sdk_handle: *mut SDKHandle,
    data_contract_handle: *const DataContractHandle,
    identity_public_key_handle: *const crate::types::IdentityPublicKeyHandle,
    signer_handle: *const SignerHandle,
) -> DashSDKResult {
    // Validate parameters
    if sdk_handle.is_null()
        || data_contract_handle.is_null()
        || identity_public_key_handle.is_null()
        || signer_handle.is_null()
    {
        return DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
            "One or more required parameters is null".to_string(),
        ));
    }

    let wrapper = &mut *(sdk_handle as *mut SDKWrapper);
    let data_contract = &*(data_contract_handle as *const DataContract);
    let identity_public_key = &*(identity_public_key_handle as *const IdentityPublicKey);
    let signer = &*(signer_handle as *const super::signer::IOSSigner);

    let result: Result<DataContract, FFIError> = wrapper.runtime.block_on(async {
        // Put data contract to platform and wait for response
        use dash_sdk::platform::transition::put_contract::PutContract;

        let confirmed_contract = data_contract
            .put_to_platform_and_wait_for_response(
                &wrapper.sdk,
                identity_public_key.clone(),
                signer,
                None, // settings (use defaults)
            )
            .await
            .map_err(|e| {
                FFIError::InternalError(format!(
                    "Failed to put data contract to platform and wait: {}",
                    e
                ))
            })?;

        Ok(confirmed_contract)
    });

    match result {
        Ok(confirmed_contract) => {
            let handle = Box::into_raw(Box::new(confirmed_contract)) as *mut DataContractHandle;
            DashSDKResult::success_handle(
                handle as *mut std::os::raw::c_void,
                DashSDKResultDataType::DataContractHandle,
            )
        }
        Err(e) => DashSDKResult::error(e.into()),
    }
}

// Helper function for freeing strings
use crate::types::dash_sdk_string_free;
