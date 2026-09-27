use crate::sdk::SDKWrapper;
use crate::{DashSDKError, DashSDKErrorCode, DocumentHandle, FFIError, SDKHandle};
use dash_sdk::dpp::data_contract::accessors::v0::DataContractV0Getters;
use dash_sdk::dpp::data_contract::document_type::methods::DocumentTypeV0Methods;
use dash_sdk::dpp::document::{Document, DocumentV0Setters};
use dash_sdk::dpp::platform_value::string_encoding::Encoding;
use dash_sdk::dpp::platform_value::Value;
use dash_sdk::dpp::prelude::Identifier;
use drive_proof_verifier::ContextProvider;
use std::collections::BTreeMap;
use std::ffi::CStr;
use std::os::raw::c_char;

/// Destroy a document
///
/// # Safety
/// - `sdk_handle` and `document_handle` must be valid, non-null pointers.
/// - Returns a pointer to an error structure on failure; caller must free with `dash_sdk_error_free`.
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
///
/// # Safety
/// - `handle` must be a pointer previously returned by this SDK or null (no-op).
/// - After this call, `handle` becomes invalid and must not be used again.
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_document_handle_destroy(handle: *mut DocumentHandle) {
    if !handle.is_null() {
        let _ = Box::from_raw(handle as *mut Document);
    }
}

/// Free a document handle (alias for destroy)
///
/// # Safety
/// - Same as `dash_sdk_document_handle_destroy`.
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_document_free(handle: *mut DocumentHandle) {
    dash_sdk_document_handle_destroy(handle);
}

/// Set document properties from JSON
///
/// The values are sanitized against the document type the way
/// `dash_sdk_document_create` sanitizes them: base58 or hex strings become
/// identifiers, hex or base64 strings become byte arrays and integers narrow
/// to their declared width, typed array elements included. The data contract
/// comes from the trusted context provider, the same lookup
/// `dash_sdk_document_replace_on_platform` makes. On any error the document
/// is left unchanged.
///
/// # Safety
/// - `sdk_handle`, `document_handle`, `data_contract_id`, `document_type_name` and
///   `properties_json` must be valid, non-null pointers.
/// - `data_contract_id` (base58 encoded), `document_type_name` and `properties_json` must point
///   to NUL-terminated C strings valid for the duration of the call.
/// - Returns an error pointer on failure; caller must free with `dash_sdk_error_free`.
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_document_set_properties(
    sdk_handle: *const SDKHandle,
    document_handle: *mut DocumentHandle,
    data_contract_id: *const c_char,
    document_type_name: *const c_char,
    properties_json: *const c_char,
) -> *mut DashSDKError {
    if sdk_handle.is_null()
        || document_handle.is_null()
        || data_contract_id.is_null()
        || document_type_name.is_null()
        || properties_json.is_null()
    {
        return Box::into_raw(Box::new(DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
            "Invalid parameters".to_string(),
        )));
    }

    let wrapper = &*(sdk_handle as *const SDKWrapper);
    let document = &mut *(document_handle as *mut Document);

    let contract_id_str = match CStr::from_ptr(data_contract_id).to_str() {
        Ok(s) => s,
        Err(e) => return Box::into_raw(Box::new(FFIError::from(e).into())),
    };

    let document_type_name_str = match CStr::from_ptr(document_type_name).to_str() {
        Ok(s) => s,
        Err(e) => return Box::into_raw(Box::new(FFIError::from(e).into())),
    };

    let properties_str = match CStr::from_ptr(properties_json).to_str() {
        Ok(s) => s,
        Err(e) => {
            return Box::into_raw(Box::new(DashSDKError::new(
                DashSDKErrorCode::InvalidParameter,
                format!("Invalid UTF-8 in properties JSON: {}", e),
            )));
        }
    };

    // Parse JSON string to Value
    let properties_value: Value = match serde_json::from_str(properties_str) {
        Ok(v) => v,
        Err(e) => {
            return Box::into_raw(Box::new(DashSDKError::new(
                DashSDKErrorCode::InvalidParameter,
                format!("Failed to parse properties JSON: {}", e),
            )));
        }
    };

    // Convert Value to BTreeMap if it's an object
    let mut properties_map = match properties_value {
        Value::Map(vec_map) => {
            // Convert Vec<(Value, Value)> to BTreeMap<String, Value>
            let mut btree_map = BTreeMap::new();
            for (key, value) in vec_map {
                let key_str = match key {
                    Value::Text(s) => s,
                    _ => {
                        return Box::into_raw(Box::new(DashSDKError::new(
                            DashSDKErrorCode::InvalidParameter,
                            "Property keys must be strings".to_string(),
                        )));
                    }
                };
                btree_map.insert(key_str, value);
            }
            btree_map
        }
        _ => {
            return Box::into_raw(Box::new(DashSDKError::new(
                DashSDKErrorCode::InvalidParameter,
                "Properties must be a JSON object".to_string(),
            )));
        }
    };

    if let Err(e) = sanitize_properties_for_document_type(
        wrapper,
        contract_id_str,
        document_type_name_str,
        &mut properties_map,
    ) {
        return Box::into_raw(Box::new(e.into()));
    }

    // Set the properties on the document
    document.set_properties(properties_map);

    std::ptr::null_mut()
}

/// Sanitizes `properties` against the `document_type_name` type of the
/// contract the trusted context provider holds under `data_contract_id`.
fn sanitize_properties_for_document_type(
    wrapper: &SDKWrapper,
    data_contract_id: &str,
    document_type_name: &str,
    properties: &mut BTreeMap<String, Value>,
) -> Result<(), FFIError> {
    let contract_id = Identifier::from_string(data_contract_id, Encoding::Base58)
        .map_err(|e| FFIError::InternalError(format!("Invalid contract ID: {}", e)))?;

    let provider = wrapper.trusted_provider.as_ref().ok_or_else(|| {
        FFIError::InternalError("No trusted context provider configured".to_string())
    })?;

    let data_contract = provider
        .get_data_contract(&contract_id, wrapper.sdk.version())
        .map_err(|e| {
            FFIError::InternalError(format!("Failed to get contract from context: {}", e))
        })?
        .ok_or_else(|| {
            FFIError::InternalError(format!(
                "Contract {} not found in trusted context",
                data_contract_id
            ))
        })?;

    let document_type = data_contract
        .document_type_borrowed_for_name(document_type_name)
        .map_err(|e| FFIError::InternalError(format!("Failed to get document type: {}", e)))?;

    document_type.sanitize_document_properties(properties);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dash_sdk_error_free;
    use crate::test_utils::test_utils::destroy_mock_sdk_handle;
    use dash_sdk::dpp::dashcore::Network;
    use dash_sdk::dpp::data_contract::DataContractFactory;
    use dash_sdk::dpp::document::{DocumentV0, DocumentV0Getters};
    use dash_sdk::dpp::platform_value::platform_value;
    use dash_sdk::dpp::prelude::DataContract;
    use dash_sdk::dpp::version::PlatformVersion;
    use rs_sdk_trusted_context_provider::TrustedHttpContextProvider;
    use std::ffi::CString;
    use std::num::NonZeroUsize;
    use std::sync::Arc;

    /// A contract with one `note` type: an identifier, a byte array and a
    /// typed array of identifiers (protocol version 14).
    fn note_contract() -> DataContract {
        let documents = platform_value!({
            "note": {
                "type": "object",
                "properties": {
                    "author": {
                        "type": "array",
                        "byteArray": true,
                        "minItems": 32,
                        "maxItems": 32,
                        "contentMediaType": "application/x.dash.dpp.identifier",
                        "position": 0
                    },
                    "payload": {
                        "type": "array",
                        "byteArray": true,
                        "minItems": 1,
                        "maxItems": 64,
                        "position": 1
                    },
                    "members": {
                        "type": "array",
                        "minItems": 0,
                        "maxItems": 8,
                        "items": {
                            "type": "array",
                            "byteArray": true,
                            "minItems": 32,
                            "maxItems": 32,
                            "contentMediaType": "application/x.dash.dpp.identifier"
                        },
                        "position": 2
                    }
                },
                "additionalProperties": false
            }
        });

        let factory = DataContractFactory::new(PlatformVersion::latest().protocol_version)
            .expect("factory for the latest protocol version");
        factory
            .create_with_value_config(Identifier::new([1; 32]), 1, documents, None, None)
            .expect("note contract")
            .data_contract()
            .clone()
    }

    /// A mock SDK handle whose trusted context provider serves `contracts`.
    /// The provider's quorum URL is an address literal, so nothing resolves
    /// or connects.
    fn sdk_handle_with_known_contracts(contracts: Vec<DataContract>) -> *mut SDKHandle {
        let provider = TrustedHttpContextProvider::new_with_url(
            Network::Regtest,
            "http://127.0.0.1:22444".to_string(),
            NonZeroUsize::new(1).expect("non-zero cache size"),
        )
        .expect("offline trusted context provider")
        .with_known_contracts(contracts);
        let mut wrapper = SDKWrapper::new_mock();
        wrapper.trusted_provider = Some(Arc::new(provider));
        Box::into_raw(Box::new(wrapper)) as *mut SDKHandle
    }

    fn fetched_note() -> Document {
        Document::V0(DocumentV0 {
            id: Identifier::new([2; 32]),
            owner_id: Identifier::new([1; 32]),
            properties: BTreeMap::new(),
            revision: Some(1),
            created_at: None,
            updated_at: None,
            transferred_at: None,
            created_at_block_height: None,
            updated_at_block_height: None,
            transferred_at_block_height: None,
            created_at_core_block_height: None,
            updated_at_core_block_height: None,
            transferred_at_core_block_height: None,
            creator_id: None,
            contract_version: None,
        })
    }

    #[test]
    fn should_store_identifier_and_byte_properties_decoded_from_strings() {
        let contract = note_contract();
        let contract_id = CString::new(contract.id().to_string(Encoding::Base58))
            .expect("no NUL in the contract id");
        let document_type_name = CString::new("note").expect("no NUL in the type name");
        let author = Identifier::new([7; 32]);
        let first_member = Identifier::new([8; 32]);
        let second_member = Identifier::new([9; 32]);
        let properties_json = CString::new(format!(
            r#"{{"author":"{}","payload":"deadbeef","members":["{}","{}"]}}"#,
            author.to_string(Encoding::Base58),
            first_member.to_string(Encoding::Base58),
            second_member.to_string(Encoding::Base58),
        ))
        .expect("no NUL in the JSON");

        let sdk_handle = sdk_handle_with_known_contracts(vec![contract]);
        let document_handle = Box::into_raw(Box::new(fetched_note())) as *mut DocumentHandle;
        let error = unsafe {
            dash_sdk_document_set_properties(
                sdk_handle,
                document_handle,
                contract_id.as_ptr(),
                document_type_name.as_ptr(),
                properties_json.as_ptr(),
            )
        };
        assert!(error.is_null(), "set_properties failed");

        let document = unsafe { Box::from_raw(document_handle as *mut Document) };
        destroy_mock_sdk_handle(sdk_handle);
        let properties = document.properties();
        assert_eq!(
            properties.get("author"),
            Some(&Value::Identifier(author.to_buffer()))
        );
        assert_eq!(
            properties.get("payload"),
            Some(&Value::Bytes(vec![0xde, 0xad, 0xbe, 0xef]))
        );
        assert_eq!(
            properties.get("members"),
            Some(&Value::Array(vec![
                Value::Identifier(first_member.to_buffer()),
                Value::Identifier(second_member.to_buffer()),
            ]))
        );
    }

    #[test]
    fn should_leave_the_document_unchanged_when_the_document_type_cannot_be_resolved() {
        let contract = note_contract();
        let known_contract_id = CString::new(contract.id().to_string(Encoding::Base58))
            .expect("no NUL in the contract id");
        let unknown_contract_id =
            CString::new(Identifier::new([3; 32]).to_string(Encoding::Base58))
                .expect("no NUL in the contract id");
        let note = CString::new("note").expect("no NUL in the type name");
        let unknown_type = CString::new("letter").expect("no NUL in the type name");
        let properties_json =
            CString::new(r#"{"payload":"deadbeef"}"#).expect("no NUL in the JSON");

        let sdk_handle = sdk_handle_with_known_contracts(vec![contract]);
        for (contract_id, document_type_name) in [
            (&unknown_contract_id, &note),
            (&known_contract_id, &unknown_type),
        ] {
            let mut original = fetched_note();
            original.set("payload", Value::Bytes(vec![1, 2, 3]));
            let document_handle = Box::into_raw(Box::new(original.clone())) as *mut DocumentHandle;

            let error = unsafe {
                dash_sdk_document_set_properties(
                    sdk_handle,
                    document_handle,
                    contract_id.as_ptr(),
                    document_type_name.as_ptr(),
                    properties_json.as_ptr(),
                )
            };

            assert!(!error.is_null(), "an unresolvable type must be refused");
            assert_eq!(unsafe { (*error).code }, DashSDKErrorCode::InternalError);
            unsafe { dash_sdk_error_free(error) };
            let document = unsafe { Box::from_raw(document_handle as *mut Document) };
            assert_eq!(*document, original);
        }
        destroy_mock_sdk_handle(sdk_handle);
    }
}
