//! Multiple identities contract keys query operations

use dash_sdk::dpp::identity::identities_contract_keys::IdentitiesContractKeys;
use dash_sdk::dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use dash_sdk::dpp::identity::Purpose;
use dash_sdk::dpp::platform_value::string_encoding::Encoding;
use dash_sdk::dpp::prelude::Identifier;
use dash_sdk::platform::identities_contract_keys_query::IdentitiesContractKeysQuery;
use dash_sdk::platform::Fetch;
use std::collections::BTreeSet;
use std::ffi::{CStr, CString};
use std::os::raw::c_char;

use crate::sdk::SDKWrapper;
use crate::types::SDKHandle;
use crate::{DashSDKError, DashSDKErrorCode, DashSDKResult, FFIError};

/// Fetch contract keys for multiple identities
///
/// # Parameters
/// - `sdk_handle`: SDK handle
/// - `identity_ids`: Comma-separated list of Base58-encoded identity IDs; an empty string answers `{}`
/// - `contract_id`: Base58-encoded contract ID
/// - `document_type_name`: Optional document type name (pass NULL if not needed; an empty name is refused)
/// - `purposes`: Comma-separated list of key purposes (0=Authentication, 1=Encryption, 2=Decryption, 3=Transfer). Repeated IDs and purposes are asked for once.
///
/// # Returns
/// A JSON object read from a proved `getIdentitiesContractKeys` response. Each value is the key
/// Drive's current-key reference names for the identity, contract (or document type) and purpose.
/// What that reference holds depends on the contract's storage requirement for the purpose:
/// - `MultipleReferenceToLatest` (every bound authentication key): the key registered last. It
///   stays the current one when it is disabled (`disabled_at` is then set).
/// - `Unique`: for a contract, the first key registered, also once it is disabled; for a document
///   type, the last.
/// - `Multiple`: there is no reference, so the value is always `null`, whatever keys exist.
///
/// Only authentication, encryption and decryption keys can be bound to a contract, so
/// `"transfer"` is always `null`. A contract or document type that does not exist answers `null`
/// too.
///
/// Every requested identity ID is a top-level key, in its Base58 form with any whitespace around it
/// dropped, and every requested purpose
/// (`"authentication"`, `"encryption"`, `"decryption"`, `"transfer"`) is a key under it. A purpose
/// the proof shows no current-key reference for is `null`. A missing identity or purpose is never
/// a valid answer, so a caller can treat one as malformed:
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

    // Parse comma-separated identity IDs; an empty list asks for nothing
    let ids_str = ids_str.trim();
    let identities_ids: Result<Vec<Identifier>, DashSDKError> = ids_str
        .split(',')
        .filter(|_| !ids_str.is_empty())
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
        Ok(ids) => unique(ids),
        Err(e) => return DashSDKResult::error(e),
    };

    let contract_id = match Identifier::from_string(contract_id_str.trim(), Encoding::Base58) {
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
            // Drive appends the name to the contract id, so an empty one would ask for the
            // contract's own keys.
            Ok("") => {
                return DashSDKResult::error(DashSDKError::new(
                    DashSDKErrorCode::InvalidParameter,
                    "Document type name is empty".to_string(),
                ))
            }
            Ok(s) => Some(s.to_string()),
            Err(e) => return DashSDKResult::error(FFIError::from(e).into()),
        }
    };

    // Parse comma-separated purposes
    let purposes: Result<Vec<RequestedPurpose>, DashSDKError> = purposes_str
        .split(',')
        .map(str::trim)
        .map(|purpose_str| {
            let number = purpose_str.parse::<u8>().ok();
            PURPOSES
                .iter()
                .find(|(n, _, _)| Some(*n) == number)
                .map(|(_, purpose, name)| (*purpose, *name))
                .ok_or_else(|| DashSDKError::new(
                    DashSDKErrorCode::InvalidParameter,
                    format!("Invalid purpose: {}. Must be 0 (Authentication), 1 (Encryption), 2 (Decryption), or 3 (Transfer)", purpose_str),
                ))
        })
        .collect();

    let purposes = match purposes {
        Ok(p) => unique(p),
        Err(e) => return DashSDKResult::error(e),
    };

    if identities_ids.is_empty() {
        return DashSDKResult::success_string(c"{}".to_owned().into_raw());
    }

    let result: Result<String, FFIError> = wrapper.runtime.block_on(async {
        let query = IdentitiesContractKeysQuery::new(
            identities_ids.clone(),
            contract_id,
            document_type_name,
            purposes.iter().map(|(purpose, _)| *purpose).collect(),
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

/// A purpose the caller asked for, with its name in the JSON answer.
type RequestedPurpose = (Purpose, &'static str);

/// The purposes this query takes: the number the caller passes, and the name it is answered under.
const PURPOSES: [(u8, Purpose, &str); 4] = [
    (0, Purpose::AUTHENTICATION, "authentication"),
    (1, Purpose::ENCRYPTION, "encryption"),
    (2, Purpose::DECRYPTION, "decryption"),
    (3, Purpose::TRANSFER, "transfer"),
];

/// `items` in their first order, each once. Drive sizes the proof by the number of identities
/// times the number of purposes, so a repeat would only stretch that limit.
fn unique<T: Ord + Clone>(items: Vec<T>) -> Vec<T> {
    let mut seen = BTreeSet::new();
    items
        .into_iter()
        .filter(|item| seen.insert(item.clone()))
        .collect()
}

/// The JSON answer for the requested identities and purposes. Every requested identity is a
/// key, and every requested purpose is a key under it: `null` when the proof shows no current-key
/// reference for it. Keys the proof carries for anything not
/// requested are left out.
fn contract_keys_json(
    identities_ids: &[Identifier],
    purposes: &[RequestedPurpose],
    keys: Option<&IdentitiesContractKeys>,
) -> serde_json::Map<String, serde_json::Value> {
    let mut json_obj = serde_json::Map::new();
    for identity_id in identities_ids {
        let keys_by_purpose = keys.and_then(|keys| keys.get(identity_id));
        let mut purpose_obj = serde_json::Map::new();
        for (purpose, name) in purposes {
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
            purpose_obj.insert(name.to_string(), value);
        }
        json_obj.insert(
            identity_id.to_string(Encoding::Base58),
            serde_json::Value::Object(purpose_obj),
        );
    }
    json_obj
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::test_utils::destroy_mock_sdk_handle;
    use crate::{dash_sdk_error_free, DashSDKResultDataType};
    use dash_sdk::dpp::identity::identity_public_key::v0::IdentityPublicKeyV0;
    use dash_sdk::dpp::identity::{IdentityPublicKey, KeyType, SecurityLevel};
    use dash_sdk::dpp::platform_value::BinaryData;
    use std::collections::BTreeMap;

    const IDENTITY_A: [u8; 32] = [1u8; 32];
    const IDENTITY_B: [u8; 32] = [2u8; 32];
    const CONTRACT: [u8; 32] = [3u8; 32];

    fn base58(bytes: [u8; 32]) -> String {
        bs58::encode(bytes).into_string()
    }

    fn encryption_key(id: u32, disabled_at: Option<u64>) -> IdentityPublicKey {
        IdentityPublicKey::V0(IdentityPublicKeyV0 {
            id,
            purpose: Purpose::ENCRYPTION,
            security_level: SecurityLevel::MEDIUM,
            key_type: KeyType::ECDSA_SECP256K1,
            read_only: false,
            data: BinaryData::new(vec![id as u8; 33]),
            disabled_at,
            contract_bounds: None,
        })
    }

    /// Identity A's current encryption key is 9, disabled; it has no decryption key.
    fn proved_keys() -> IdentitiesContractKeys {
        let mut by_purpose = BTreeMap::new();
        by_purpose.insert(Purpose::ENCRYPTION, Some(encryption_key(9, Some(1_700))));
        by_purpose.insert(Purpose::DECRYPTION, None);
        let mut keys = BTreeMap::new();
        keys.insert(Identifier::new(IDENTITY_A), by_purpose);
        keys
    }

    fn requested_purposes() -> Vec<RequestedPurpose> {
        vec![
            (Purpose::ENCRYPTION, "encryption"),
            (Purpose::DECRYPTION, "decryption"),
        ]
    }

    fn query() -> IdentitiesContractKeysQuery {
        IdentitiesContractKeysQuery::new(
            vec![Identifier::new(IDENTITY_A), Identifier::new(IDENTITY_B)],
            Identifier::new(CONTRACT),
            None,
            vec![Purpose::ENCRYPTION, Purpose::DECRYPTION],
        )
        .expect("query")
    }

    /// Mock SDK handle that answers `query()` with `keys`.
    fn create_mock_sdk_handle_with_keys(keys: Option<IdentitiesContractKeys>) -> *mut SDKHandle {
        create_mock_sdk_handle_answering(query(), keys)
    }

    /// Mock SDK handle that answers `query` with `keys`.
    fn create_mock_sdk_handle_answering(
        query: IdentitiesContractKeysQuery,
        keys: Option<IdentitiesContractKeys>,
    ) -> *mut SDKHandle {
        let mut wrapper = Box::new(SDKWrapper::new_mock());
        // The FFI entry point blocks on its own runtime, so this one must be gone first.
        let setup_runtime = tokio::runtime::Runtime::new().expect("create setup runtime");
        setup_runtime.block_on(async {
            wrapper
                .sdk
                .mock()
                .expect_fetch::<IdentitiesContractKeys, _>(query, keys)
                .await
                .expect("register contract keys expectation");
        });
        drop(setup_runtime);
        Box::into_raw(wrapper) as *mut SDKHandle
    }

    unsafe fn fetch(handle: *mut SDKHandle, contract: [u8; 32]) -> DashSDKResult {
        let ids = format!("{},{}", base58(IDENTITY_A), base58(IDENTITY_B));
        fetch_with(handle, &ids, contract, None, "1,2")
    }

    unsafe fn fetch_with(
        handle: *mut SDKHandle,
        ids: &str,
        contract: [u8; 32],
        document_type: Option<&str>,
        purposes: &str,
    ) -> DashSDKResult {
        let ids = CString::new(ids).unwrap();
        let contract = CString::new(base58(contract)).unwrap();
        let document_type = document_type.map(|name| CString::new(name).unwrap());
        let purposes = CString::new(purposes).unwrap();
        dash_sdk_identities_fetch_contract_keys(
            handle,
            ids.as_ptr(),
            contract.as_ptr(),
            document_type
                .as_ref()
                .map_or(std::ptr::null(), |name| name.as_ptr()),
            purposes.as_ptr(),
        )
    }

    unsafe fn error_code_of(result: DashSDKResult) -> DashSDKErrorCode {
        assert!(result.data.is_null());
        assert!(!result.error.is_null(), "the call must fail");
        let code = (*result.error).code;
        dash_sdk_error_free(result.error);
        code
    }

    unsafe fn json_of(result: DashSDKResult) -> serde_json::Value {
        assert!(result.error.is_null(), "the call must succeed");
        assert_eq!(result.data_type, DashSDKResultDataType::String);
        let json = CString::from_raw(result.data as *mut c_char)
            .into_string()
            .expect("utf-8 json");
        serde_json::from_str(&json).expect("valid json")
    }

    fn all_null() -> serde_json::Value {
        serde_json::json!({ "encryption": null, "decryption": null })
    }

    #[test]
    fn should_list_every_requested_identity_and_purpose() {
        let json = serde_json::Value::Object(contract_keys_json(
            &[Identifier::new(IDENTITY_A), Identifier::new(IDENTITY_B)],
            &requested_purposes(),
            Some(&proved_keys()),
        ));
        assert_eq!(
            json,
            serde_json::json!({
                base58(IDENTITY_A): {
                    "encryption": {
                        "id": 9,
                        "type": KeyType::ECDSA_SECP256K1 as u8,
                        "data": hex::encode([9u8; 33]),
                        "purpose": Purpose::ENCRYPTION as u8,
                        "security_level": SecurityLevel::MEDIUM as u8,
                        "read_only": false,
                        "disabled_at": 1_700,
                    },
                    "decryption": null,
                },
                base58(IDENTITY_B): all_null(),
            })
        );
    }

    #[test]
    fn should_list_every_requested_purpose_when_the_proof_holds_nothing() {
        let json = serde_json::Value::Object(contract_keys_json(
            &[Identifier::new(IDENTITY_A)],
            &requested_purposes(),
            None,
        ));
        assert_eq!(json, serde_json::json!({ base58(IDENTITY_A): all_null() }));
    }

    #[test]
    fn should_leave_out_keys_that_were_not_requested() {
        let json = serde_json::Value::Object(contract_keys_json(
            &[Identifier::new(IDENTITY_B)],
            &[(Purpose::DECRYPTION, "decryption")],
            Some(&proved_keys()),
        ));
        assert_eq!(
            json,
            serde_json::json!({ base58(IDENTITY_B): { "decryption": null } })
        );
    }

    #[test]
    fn should_return_the_fetched_keys() {
        let handle = create_mock_sdk_handle_with_keys(Some(proved_keys()));
        let json = unsafe { json_of(fetch(handle, CONTRACT)) };
        destroy_mock_sdk_handle(handle);
        assert_eq!(json[base58(IDENTITY_A)]["encryption"]["id"], 9);
        assert_eq!(json[base58(IDENTITY_A)]["encryption"]["disabled_at"], 1_700);
        assert_eq!(
            json[base58(IDENTITY_A)]["decryption"],
            serde_json::Value::Null
        );
        assert_eq!(json[base58(IDENTITY_B)], all_null());
    }

    #[test]
    fn should_ask_for_repeated_identities_and_purposes_once() {
        // The mock knows only the request without repeats.
        let handle = create_mock_sdk_handle_with_keys(Some(proved_keys()));
        let ids = format!(
            "{},{},{}",
            base58(IDENTITY_A),
            base58(IDENTITY_B),
            base58(IDENTITY_A)
        );
        let json = unsafe { json_of(fetch_with(handle, &ids, CONTRACT, None, "1,2,1")) };
        destroy_mock_sdk_handle(handle);
        assert_eq!(json.as_object().map(|o| o.len()), Some(2));
        assert_eq!(json[base58(IDENTITY_A)]["encryption"]["id"], 9);
        assert_eq!(json[base58(IDENTITY_B)], all_null());
    }

    #[test]
    fn should_ask_for_the_document_type_it_was_given() {
        let query = IdentitiesContractKeysQuery::new(
            vec![Identifier::new(IDENTITY_A)],
            Identifier::new(CONTRACT),
            Some("note".to_string()),
            vec![Purpose::ENCRYPTION],
        )
        .expect("query");
        let handle = create_mock_sdk_handle_answering(query, Some(proved_keys()));
        let json = unsafe {
            json_of(fetch_with(
                handle,
                &base58(IDENTITY_A),
                CONTRACT,
                Some("note"),
                "1",
            ))
        };
        destroy_mock_sdk_handle(handle);
        assert_eq!(json[base58(IDENTITY_A)]["encryption"]["id"], 9);
    }

    #[test]
    fn should_refuse_a_purpose_it_does_not_take() {
        let handle = create_mock_sdk_handle_with_keys(Some(proved_keys()));
        for purposes in ["4", "1,x", ""] {
            let code = unsafe {
                error_code_of(fetch_with(
                    handle,
                    &base58(IDENTITY_A),
                    CONTRACT,
                    None,
                    purposes,
                ))
            };
            assert_eq!(
                code,
                DashSDKErrorCode::InvalidParameter,
                "purposes {purposes:?}"
            );
        }
        destroy_mock_sdk_handle(handle);
    }

    #[test]
    fn should_answer_no_identities_with_an_empty_object() {
        let handle = create_mock_sdk_handle_with_keys(Some(proved_keys()));
        let json = unsafe { json_of(fetch_with(handle, "", CONTRACT, None, "1")) };
        destroy_mock_sdk_handle(handle);
        assert_eq!(json, serde_json::json!({}));
    }

    #[test]
    fn should_refuse_an_empty_document_type() {
        let handle = create_mock_sdk_handle_with_keys(Some(proved_keys()));
        let code = unsafe {
            error_code_of(fetch_with(
                handle,
                &base58(IDENTITY_A),
                CONTRACT,
                Some(""),
                "1",
            ))
        };
        destroy_mock_sdk_handle(handle);
        assert_eq!(code, DashSDKErrorCode::InvalidParameter);
    }

    #[test]
    fn should_return_nulls_when_nothing_is_found() {
        let handle = create_mock_sdk_handle_with_keys(None);
        let json = unsafe { json_of(fetch(handle, CONTRACT)) };
        destroy_mock_sdk_handle(handle);
        assert_eq!(
            json,
            serde_json::json!({ base58(IDENTITY_A): all_null(), base58(IDENTITY_B): all_null() })
        );
    }

    #[test]
    fn should_fail_rather_than_answer_empty_when_the_fetch_fails() {
        // The mock knows only `query()`, so a request for another contract fails.
        let handle = create_mock_sdk_handle_with_keys(Some(proved_keys()));
        let code = unsafe { error_code_of(fetch(handle, [4u8; 32])) };
        destroy_mock_sdk_handle(handle);
        assert_ne!(code, DashSDKErrorCode::InvalidParameter);
    }

    #[test]
    fn should_refuse_a_null_handle() {
        let code = unsafe { error_code_of(fetch(std::ptr::null_mut(), CONTRACT)) };
        assert_eq!(code, DashSDKErrorCode::InvalidParameter);
    }
}
