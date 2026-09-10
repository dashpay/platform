//! Identity parsing operations

use dash_sdk::dpp::identity::accessors::IdentityGettersV0;
use dash_sdk::dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use dash_sdk::dpp::prelude::Identity;
use std::ffi::{c_char, CStr};

use crate::types::{DashSDKResultDataType, IdentityHandle};
use crate::{DashSDKError, DashSDKErrorCode, DashSDKResult, FFIError};

/// Parse an identity from JSON string to handle
///
/// This function takes a JSON string representation of an identity
/// (as returned by dash_sdk_identity_fetch) and converts it to an
/// identity handle that can be used with other FFI functions.
///
/// # Parameters
/// - `json_str`: JSON string containing the identity data
///
/// # Returns
/// - Handle to the parsed identity on success
/// - Error if JSON parsing fails
///
/// # Safety
/// - `json_str` must be a valid, non-null pointer to a NUL-terminated C string and remain valid for the duration of the call.
/// - On success, the returned `DashSDKResult` contains a heap-allocated handle which must be freed using the
///   appropriate SDK destroy function to avoid leaks.
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_identity_parse_json(json_str: *const c_char) -> DashSDKResult {
    if json_str.is_null() {
        return DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
            "JSON string is null".to_string(),
        ));
    }

    let json = match CStr::from_ptr(json_str).to_str() {
        Ok(s) => s,
        Err(e) => {
            return DashSDKResult::error(FFIError::from(e).into());
        }
    };

    eprintln!("🔵 dash_sdk_identity_parse_json: Parsing JSON: {}", json);

    match serde_json::from_str::<Identity>(json) {
        Ok(identity) => {
            eprintln!("🔵 dash_sdk_identity_parse_json: Successfully parsed identity");
            eprintln!(
                "🔵 dash_sdk_identity_parse_json: Identity ID: {:?}",
                identity.id()
            );
            eprintln!(
                "🔵 dash_sdk_identity_parse_json: Identity balance: {}",
                identity.balance()
            );
            eprintln!(
                "🔵 dash_sdk_identity_parse_json: Number of public keys: {}",
                identity.public_keys().len()
            );

            // Print public key details
            for (key_id, key) in identity.public_keys() {
                eprintln!(
                    "🔵 dash_sdk_identity_parse_json: Key {}: purpose={:?}, type={:?}",
                    key_id,
                    key.purpose(),
                    key.key_type()
                );
            }

            let handle = Box::into_raw(Box::new(identity)) as *mut IdentityHandle;
            DashSDKResult::success_handle(
                handle as *mut std::os::raw::c_void,
                DashSDKResultDataType::ResultIdentityHandle,
            )
        }
        Err(e) => DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::SerializationError,
            format!("Failed to parse identity JSON: {}", e),
        )),
    }
}

/// Normalize DPP contract-bounds JSON for native persistence. Scope encoding
/// stays in Rust so host clients never reconstruct the consensus wire format.
///
/// # Safety
/// `json_str` must point to a valid NUL-terminated UTF-8 string for this call.
/// Release the returned string with `dash_sdk_string_free`.
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_contract_bounds_parse_json(
    json_str: *const c_char,
) -> DashSDKResult {
    use dash_sdk::dpp::identity::contract_bounds::ContractBounds;
    let convert = || -> Result<String, String> {
        if json_str.is_null() {
            return Err("Contract bounds JSON is null".into());
        }
        let json = CStr::from_ptr(json_str)
            .to_str()
            .map_err(|e| e.to_string())?;
        let bounds: ContractBounds = serde_json::from_str(json).map_err(|e| e.to_string())?;
        let value = match bounds {
            ContractBounds::SingleContract { id } => {
                serde_json::json!({"kind": 1, "id": id.to_buffer().to_vec()})
            }
            ContractBounds::SingleContractDocumentType {
                id,
                document_type_name,
            } => {
                serde_json::json!({"kind": 2, "id": id.to_buffer().to_vec(), "documentType": document_type_name})
            }
            ContractBounds::Scoped(scope) => {
                serde_json::json!({"kind": 3, "scope": scope.to_bytes().map_err(|e| e.to_string())?})
            }
        };
        Ok(value.to_string())
    };
    match convert() {
        Ok(json) => DashSDKResult::success_string(
            std::ffi::CString::new(json)
                .expect("JSON has no NUL bytes")
                .into_raw(),
        ),
        Err(error) => DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::SerializationError,
            error,
        )),
    }
}

#[cfg(test)]
mod scoped_bounds_tests {
    use super::*;
    use dash_sdk::dpp::identity::contract_bounds::{
        AuthenticationScope, AuthenticationScopeV0, ContractBounds, ContractScope,
    };
    use dash_sdk::dpp::prelude::Identifier;

    #[test]
    fn scoped_json_normalization_preserves_complete_wire_payload() {
        let scope = AuthenticationScope::V0(AuthenticationScopeV0 {
            contracts: vec![ContractScope {
                id: Identifier::from([1; 32]),
                document_types: Some(vec!["post".into()]),
            }],
            permissions: 65,
            expires_at: Some(1_900_000_000_000),
        });
        let json = std::ffi::CString::new(
            serde_json::to_string(&ContractBounds::Scoped(scope.clone())).unwrap(),
        )
        .unwrap();
        let mut result = unsafe { dash_sdk_contract_bounds_parse_json(json.as_ptr()) };
        assert!(result.error.is_null());
        let output = unsafe { CStr::from_ptr(result.data as *const c_char) }
            .to_str()
            .unwrap();
        let normalized: serde_json::Value = serde_json::from_str(output).unwrap();
        assert_eq!(normalized["kind"], 3);
        let bytes: Vec<u8> = serde_json::from_value(normalized["scope"].clone()).unwrap();
        assert_eq!(AuthenticationScope::from_bytes(&bytes).unwrap(), scope);
        unsafe { crate::types::dash_sdk_result_free(&mut result) };
    }
}
