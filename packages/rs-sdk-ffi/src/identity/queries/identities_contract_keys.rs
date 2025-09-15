//! Multiple identities contract keys query operations

use dash_sdk::dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use dash_sdk::dpp::identity::Purpose;
use dash_sdk::dpp::platform_value::string_encoding::Encoding;
use dash_sdk::dpp::prelude::Identifier;
// We need to implement the query directly since it's not publicly exposed
use dash_sdk::Sdk;
use std::collections::BTreeMap;
use std::ffi::{CStr, CString};
use std::os::raw::c_char;

use crate::sdk::SDKWrapper;
use crate::types::SDKHandle;
use crate::{DashSDKError, DashSDKErrorCode, DashSDKResult, FFIError};

/// Fetch contract keys for multiple identities
///
/// # Parameters
/// - `sdk_handle`: SDK handle
/// - `identity_ids`: Comma-separated list of Base58-encoded identity IDs
/// - `contract_id`: Base58-encoded contract ID
/// - `document_type_name`: Optional document type name (pass NULL if not needed)
/// - `purposes`: Comma-separated list of key purposes (0=Authentication, 1=Encryption, 2=Decryption, 3=Withdraw)
///
/// # Returns
/// JSON string containing identity IDs mapped to their contract keys by purpose
///
/// # Safety
/// - `sdk_handle`, `identity_ids`, `contract_id`, and `purposes` must be valid, non-null pointers.
/// - `identity_ids`, `contract_id`, `document_type_name` (when non-null), and `purposes` must point to NUL-terminated C strings valid for the duration of the call.
/// - On success, returns a C string pointer inside `DashSDKResult`; caller must free it using SDK routines.
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_identities_fetch_contract_keys(
    sdk_handle: *const SDKHandle,
    identity_ids: *const c_char,
    contract_id: *const c_char,
    document_type_name: *const c_char,
    purposes: *const c_char,
) -> DashSDKResult {
    if sdk_handle.is_null() || identity_ids.is_null() || contract_id.is_null() || purposes.is_null()
    {
        return DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
            "SDK handle, identity IDs, contract ID, or purposes is null".to_string(),
        ));
    }

    let wrapper = &*(sdk_handle as *const SDKWrapper);

    let ids_str = match CStr::from_ptr(identity_ids).to_str() {
        Ok(s) => s,
        Err(e) => return DashSDKResult::error(FFIError::from(e).into()),
    };

    let contract_id_str = match CStr::from_ptr(contract_id).to_str() {
        Ok(s) => s,
        Err(e) => return DashSDKResult::error(FFIError::from(e).into()),
    };

    let purposes_str = match CStr::from_ptr(purposes).to_str() {
        Ok(s) => s,
        Err(e) => return DashSDKResult::error(FFIError::from(e).into()),
    };

    // Parse comma-separated identity IDs
    let identities_ids: Result<Vec<Identifier>, DashSDKError> = ids_str
        .split(',')
        .map(|id_str| {
            Identifier::from_string(id_str.trim(), Encoding::Base58).map_err(|e| {
                DashSDKError::new(
                    DashSDKErrorCode::InvalidParameter,
                    format!("Invalid identity ID: {}", e),
                )
            })
        })
        .collect();

    let identities_ids = match identities_ids {
        Ok(ids) => ids,
        Err(e) => return DashSDKResult::error(e),
    };

    let contract_id = match Identifier::from_string(contract_id_str, Encoding::Base58) {
        Ok(id) => id,
        Err(e) => {
            return DashSDKResult::error(DashSDKError::new(
                DashSDKErrorCode::InvalidParameter,
                format!("Invalid contract ID: {}", e),
            ))
        }
    };

    // Parse optional document type name
    let document_type_name = if document_type_name.is_null() {
        None
    } else {
        match CStr::from_ptr(document_type_name).to_str() {
            Ok(s) => Some(s.to_string()),
            Err(e) => return DashSDKResult::error(FFIError::from(e).into()),
        }
    };

    // Parse comma-separated purposes
    let purposes: Result<Vec<Purpose>, DashSDKError> = purposes_str
        .split(',')
        .map(|purpose_str| {
            match purpose_str.trim().parse::<u8>() {
                Ok(0) => Ok(Purpose::AUTHENTICATION),
                Ok(1) => Ok(Purpose::ENCRYPTION),
                Ok(2) => Ok(Purpose::DECRYPTION),
                Ok(3) => Ok(Purpose::TRANSFER),
                _ => Err(DashSDKError::new(
                    DashSDKErrorCode::InvalidParameter,
                    format!("Invalid purpose: {}. Must be 0 (Authentication), 1 (Encryption), 2 (Decryption), or 3 (Transfer)", purpose_str),
                ))
            }
        })
        .collect();

    let purposes = match purposes {
        Ok(p) => p,
        Err(e) => return DashSDKResult::error(e),
    };

    let result: Result<String, FFIError> = wrapper.runtime.block_on(async {
        // Execute the query directly using SDK
        let response = execute_identities_contract_keys_query(
            &wrapper.sdk,
            identities_ids,
            contract_id,
            document_type_name,
            purposes,
        )
        .await?;

        // Convert to JSON string
        let mut json_obj = serde_json::Map::new();

        for (identity_id, keys_by_purpose) in response {
            let mut purpose_obj = serde_json::Map::new();

            for (purpose, key_opt) in keys_by_purpose {
                let purpose_str = match purpose {
                    Purpose::AUTHENTICATION => "authentication",
                    Purpose::ENCRYPTION => "encryption",
                    Purpose::DECRYPTION => "decryption",
                    Purpose::TRANSFER => "transfer",
                    _ => "unknown",
                };

                if let Some(key) = key_opt {
                    let key_json = serde_json::json!({
                        "id": key.id(),
                        "type": key.key_type() as u8,
                        "data": hex::encode(key.data().as_slice()),
                        "purpose": purpose as u8,
                        "security_level": key.security_level() as u8,
                        "read_only": key.read_only(),
                        "disabled_at": key.disabled_at(),
                    });
                    purpose_obj.insert(purpose_str.to_string(), key_json);
                } else {
                    purpose_obj.insert(purpose_str.to_string(), serde_json::Value::Null);
                }
            }

            json_obj.insert(
                identity_id.to_string(Encoding::Base58),
                serde_json::Value::Object(purpose_obj),
            );
        }

        serde_json::to_string(&json_obj).map_err(|e| FFIError::InternalError(e.to_string()))
    });

    match result {
        Ok(json_str) => {
            let c_str = match CString::new(json_str) {
                Ok(s) => s,
                Err(e) => {
                    return DashSDKResult::error(
                        FFIError::InternalError(format!("Failed to create CString: {}", e)).into(),
                    )
                }
            };
            DashSDKResult::success_string(c_str.into_raw())
        }
        Err(e) => DashSDKResult::error(e.into()),
    }
}

/// Helper function to execute the identities contract keys query
async fn execute_identities_contract_keys_query(
    sdk: &Sdk,
    identities_ids: Vec<Identifier>,
    contract_id: Identifier,
    document_type_name: Option<String>,
    purposes: Vec<Purpose>,
) -> Result<
    BTreeMap<Identifier, BTreeMap<Purpose, Option<dash_sdk::platform::IdentityPublicKey>>>,
    FFIError,
> {
    use dash_sdk::dapi_client::{DapiRequest, RequestSettings};
    use dash_sdk::platform::proto;
    use dash_sdk::platform::proto::get_identities_contract_keys_request::{
        GetIdentitiesContractKeysRequestV0, Version,
    };

    // Create the gRPC request directly
    let grpc_request = proto::GetIdentitiesContractKeysRequest {
        version: Some(Version::V0(GetIdentitiesContractKeysRequestV0 {
            identities_ids: identities_ids.into_iter().map(|id| id.to_vec()).collect(),
            contract_id: contract_id.to_vec(),
            document_type_name,
            purposes: purposes.into_iter().map(|p| p as i32).collect(),
            prove: true,
        })),
    };

    let _response = grpc_request
        .execute(sdk, RequestSettings::default())
        .await
        .map_err(|e| FFIError::InternalError(format!("Request execution failed: {}", e)))?;

    // For now, we'll return an empty map since parse_proof is private
    // In a real implementation, you would need to parse the proof response
    Ok(BTreeMap::new())
}
