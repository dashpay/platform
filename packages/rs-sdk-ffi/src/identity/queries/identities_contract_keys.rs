//! Multiple identities contract keys query operations

use dash_sdk::dpp::identity::identities_contract_keys::IdentitiesContractKeys;
use dash_sdk::dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use dash_sdk::dpp::identity::Purpose;
use dash_sdk::dpp::platform_value::string_encoding::Encoding;
use dash_sdk::dpp::prelude::Identifier;
use dash_sdk::platform::identities_contract_keys_query::IdentitiesContractKeysQuery;
use dash_sdk::platform::Fetch;
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
/// A JSON object read from a proved `getIdentitiesContractKeys` response. Each key is the current
/// key Drive keeps for the identity, contract (or document type) and purpose: the newest key
/// registered for it, which stays the current one when it is disabled (`disabled_at` is then set).
///
/// Every requested identity ID (Base58) is a top-level key, and every requested purpose
/// (`"authentication"`, `"encryption"`, `"decryption"`, `"transfer"`) is a key under it. A purpose
/// the proof shows the identity has no key for is `null`. A missing identity or purpose is never a
/// valid answer, so a caller can treat one as malformed:
///
/// ```json
/// { "<identity id>": {
///     "encryption": { "id": 5, "type": 0, "data": "<hex>", "purpose": 1,
///                     "security_level": 2, "read_only": false, "disabled_at": null },
///     "decryption": null } }
/// ```
///
/// A failed or unprovable query returns an error, never an empty object.
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
        let query = IdentitiesContractKeysQuery::new(
            identities_ids.clone(),
            contract_id,
            document_type_name,
            purposes.clone(),
        )
        .map_err(FFIError::from)?;
        let keys = IdentitiesContractKeys::fetch(&wrapper.sdk, query)
            .await
            .map_err(FFIError::from)?;
        let json = contract_keys_json(&identities_ids, &purposes, keys.as_ref());
        serde_json::to_string(&json).map_err(|e| FFIError::InternalError(e.to_string()))
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

fn purpose_name(purpose: Purpose) -> &'static str {
    match purpose {
        Purpose::AUTHENTICATION => "authentication",
        Purpose::ENCRYPTION => "encryption",
        Purpose::DECRYPTION => "decryption",
        Purpose::TRANSFER => "transfer",
        _ => "unknown",
    }
}

/// The JSON answer for the requested identities and purposes. Every requested identity is a
/// key, and every requested purpose is a key under it: `null` when the proof shows the identity
/// has no current key of that purpose for the contract. Keys the proof carries for anything not
/// requested are left out.
fn contract_keys_json(
    identities_ids: &[Identifier],
    purposes: &[Purpose],
    keys: Option<&IdentitiesContractKeys>,
) -> serde_json::Map<String, serde_json::Value> {
    let mut json_obj = serde_json::Map::new();
    for identity_id in identities_ids {
        let keys_by_purpose = keys.and_then(|keys| keys.get(identity_id));
        let mut purpose_obj = serde_json::Map::new();
        for purpose in purposes {
            let key = keys_by_purpose
                .and_then(|keys_by_purpose| keys_by_purpose.get(purpose))
                .and_then(Option::as_ref);
            let value = match key {
                Some(key) => serde_json::json!({
                    "id": key.id(),
                    "type": key.key_type() as u8,
                    "data": hex::encode(key.data().as_slice()),
                    "purpose": key.purpose() as u8,
                    "security_level": key.security_level() as u8,
                    "read_only": key.read_only(),
                    "disabled_at": key.disabled_at(),
                }),
                None => serde_json::Value::Null,
            };
            purpose_obj.insert(purpose_name(*purpose).to_string(), value);
        }
        json_obj.insert(
            identity_id.to_string(Encoding::Base58),
            serde_json::Value::Object(purpose_obj),
        );
    }
    json_obj
}
