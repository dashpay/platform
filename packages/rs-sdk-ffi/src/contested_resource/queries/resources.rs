use crate::contested_resource::{parse_index_values_json, render_index_value};
use crate::types::SDKHandle;
use crate::{DashSDKError, DashSDKErrorCode, DashSDKResult, DashSDKResultDataType};
use dash_sdk::dpp::platform_value::Value;
use dash_sdk::drive::query::vote_polls_by_document_type_query::VotePollsByDocumentTypeQuery;
use dash_sdk::platform::FetchMany;
use dash_sdk::query_types::ContestedResource;
use serde_json::json;
use std::ffi::{c_char, c_void, CStr, CString};

/// Fetches contested resources
///
/// # Parameters
/// * `sdk_handle` - Handle to the SDK instance
/// * `contract_id` - Base58-encoded contract identifier
/// * `document_type_name` - Name of the document type
/// * `index_name` - Name of the index
/// * `start_index_values_json` - JSON array of start index values. An
///   element starting with a backslash is text with that one backslash
///   removed; otherwise a `"0x"`-prefixed element is hex-decoded to bytes;
///   otherwise the element is text, verbatim. To pass text that itself starts
///   with `"0x"` or a backslash (the DPNS label "oxford" normalizes to
///   `"0xf0rd"`), prepend one backslash. Shared with the read and vote FFIs.
/// * `end_index_values_json` - JSON array of end index values, read the same
///   way.
/// * `count` - Maximum number of resources to return
/// * `order_ascending` - Whether to order results in ascending order
///
/// # Returns
/// * JSON array of contested resources or null if not found. Each element is
///   `{"id", "contract_id", "document_type_name", "index_name", "index_values"}`
///   where `id` is the resource's index value and `index_values` is the array
///   `start_index_values + [id] + end_index_values`, each in the input
///   encoding: text verbatim, or with one backslash in front when it starts
///   with `"0x"` or a backslash; bytes and identifiers as `"0x"` hex. Text
///   and byte values parse back unchanged; other kinds (integers) are for
///   display only. `index_values` can be passed back as the key of
///   `dash_sdk_contested_resource_get_vote_state` only when it covers every
///   property of the index and every value is text or bytes.
/// * Error message if operation fails
///
/// # Safety
/// - `sdk_handle` must be a valid, non-null pointer to an initialized `SDKHandle`.
/// - All C string pointers (`contract_id`, `document_type_name`, `index_name`,
///   `start_index_values_json`, `end_index_values_json`) must be either null (when documented as optional)
///   or valid pointers to NUL-terminated strings that remain valid for the duration of the call.
/// - The function reads the `count` and `order_ascending` by value and does not retain references.
/// - On success, the returned `DashSDKResult` may contain a heap-allocated C string; the caller must
///   free it using the SDK-provided free routine. The result can also contain no data (null pointer).
/// - All pointers passed in must point to readable memory; behavior is undefined if they are dangling.
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_contested_resource_get_resources(
    sdk_handle: *const SDKHandle,
    contract_id: *const c_char,
    document_type_name: *const c_char,
    index_name: *const c_char,
    start_index_values_json: *const c_char,
    end_index_values_json: *const c_char,
    count: u32,
    order_ascending: bool,
) -> DashSDKResult {
    match get_contested_resources(
        sdk_handle,
        contract_id,
        document_type_name,
        index_name,
        start_index_values_json,
        end_index_values_json,
        count,
        order_ascending,
    ) {
        Ok(Some(json)) => {
            let c_str = match CString::new(json) {
                Ok(s) => s,
                Err(e) => {
                    return DashSDKResult {
                        data_type: DashSDKResultDataType::NoData,
                        data: std::ptr::null_mut(),
                        error: Box::into_raw(Box::new(DashSDKError::new(
                            DashSDKErrorCode::InternalError,
                            format!("Failed to create CString: {}", e),
                        ))),
                    }
                }
            };
            DashSDKResult {
                data_type: DashSDKResultDataType::String,
                data: c_str.into_raw() as *mut c_void,
                error: std::ptr::null_mut(),
            }
        }
        Ok(None) => DashSDKResult {
            data_type: DashSDKResultDataType::NoData,
            data: std::ptr::null_mut(),
            error: std::ptr::null_mut(),
        },
        Err(e) => DashSDKResult {
            data_type: DashSDKResultDataType::NoData,
            data: std::ptr::null_mut(),
            error: Box::into_raw(Box::new(DashSDKError::new(
                DashSDKErrorCode::InternalError,
                e,
            ))),
        },
    }
}

#[allow(clippy::too_many_arguments)]
fn get_contested_resources(
    sdk_handle: *const SDKHandle,
    contract_id: *const c_char,
    document_type_name: *const c_char,
    index_name: *const c_char,
    start_index_values_json: *const c_char,
    end_index_values_json: *const c_char,
    count: u32,
    order_ascending: bool,
) -> Result<Option<String>, String> {
    if sdk_handle.is_null() {
        return Err("SDK handle is null".to_string());
    }

    if contract_id.is_null() {
        return Err("Contract ID is null".to_string());
    }

    if document_type_name.is_null() {
        return Err("Document type name is null".to_string());
    }

    if index_name.is_null() {
        return Err("Index name is null".to_string());
    }

    let rt = crate::runtime::BigStackRuntime::new_isolated()
        .map_err(|e| format!("Failed to create Tokio runtime: {}", e))?;

    let contract_id_str = unsafe {
        CStr::from_ptr(contract_id)
            .to_str()
            .map_err(|e| format!("Invalid UTF-8 in contract ID: {}", e))?
    };
    let document_type_name_str = unsafe {
        CStr::from_ptr(document_type_name)
            .to_str()
            .map_err(|e| format!("Invalid UTF-8 in document type name: {}", e))?
    };
    let index_name_str = unsafe {
        CStr::from_ptr(index_name)
            .to_str()
            .map_err(|e| format!("Invalid UTF-8 in index name: {}", e))?
    };
    let wrapper = unsafe { &*(sdk_handle as *const crate::sdk::SDKWrapper) };
    let sdk = wrapper.sdk.clone();

    rt.block_on(async move {
        let contract_id_bytes = bs58::decode(contract_id_str)
            .into_vec()
            .map_err(|e| format!("Failed to decode contract ID: {}", e))?;

        let contract_id: [u8; 32] = contract_id_bytes
            .try_into()
            .map_err(|_| "Contract ID must be exactly 32 bytes".to_string())?;

        let contract_id = dash_sdk::platform::Identifier::new(contract_id);

        // Parse start index values via the shared explicit-tag helper (see
        // `parse_index_value`). Shared with the cast-vote write path so read
        // and write stay in lockstep.
        let start_index_values = if start_index_values_json.is_null() {
            Vec::new()
        } else {
            let start_values_str = unsafe {
                CStr::from_ptr(start_index_values_json)
                    .to_str()
                    .map_err(|e| format!("Invalid UTF-8 in start index values: {}", e))?
            };
            parse_index_values_json(start_values_str)
                .map_err(|e| format!("Start index values: {}", e))?
        };

        // Parse end index values via the same shared explicit-tag helper.
        let end_index_values = if end_index_values_json.is_null() {
            Vec::new()
        } else {
            let end_values_str = unsafe {
                CStr::from_ptr(end_index_values_json)
                    .to_str()
                    .map_err(|e| format!("Invalid UTF-8 in end index values: {}", e))?
            };
            parse_index_values_json(end_values_str)
                .map_err(|e| format!("End index values: {}", e))?
        };

        let query = VotePollsByDocumentTypeQuery {
            contract_id,
            document_type_name: document_type_name_str.to_string(),
            index_name: index_name_str.to_string(),
            start_index_values: start_index_values.clone(),
            end_index_values: end_index_values.clone(),
            start_at_value: None,
            // Match vectors: treat count=0 as no limit (null)
            limit: if count > 0 { Some(count as u16) } else { None },
            order_ascending,
        };

        match ContestedResource::fetch_many(&sdk, query).await {
            Ok(contested_resources) => {
                if contested_resources.0.is_empty() {
                    return Ok(None);
                }

                let contract_id_str = bs58::encode(contract_id.as_bytes()).into_string();
                let resources_json: Vec<serde_json::Value> = contested_resources
                    .0
                    .iter()
                    .map(|resource| {
                        contested_resource_json(
                            &resource.0,
                            &contract_id_str,
                            document_type_name_str,
                            index_name_str,
                            &start_index_values,
                            &end_index_values,
                        )
                    })
                    .collect();

                Ok(Some(serde_json::Value::Array(resources_json).to_string()))
            }
            Err(e) => Err(format!("Failed to fetch contested resources: {}", e)),
        }
    })
}

/// One contested resource as JSON.
///
/// `id` is the resource's own index value (for DPNS listed under
/// `["dash"]`, the normalized label such as `"a11ce"`) and `index_values` is
/// the index path `start_index_values + [id] + end_index_values`. Every value
/// is rendered with [`render_index_value`], so a path that covers every index
/// property can be passed unchanged to the vote state and voters queries.
fn contested_resource_json(
    resource_value: &Value,
    contract_id: &str,
    document_type_name: &str,
    index_name: &str,
    start_index_values: &[Value],
    end_index_values: &[Value],
) -> serde_json::Value {
    let index_values: Vec<String> = start_index_values
        .iter()
        .chain(std::iter::once(resource_value))
        .chain(end_index_values)
        .map(render_index_value)
        .collect();
    json!({
        "id": render_index_value(resource_value),
        "contract_id": contract_id,
        "document_type_name": document_type_name,
        "index_name": index_name,
        "index_values": index_values,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contested_resource::queries::vote_state::dash_sdk_contested_resource_get_vote_state;
    use crate::sdk::SDKWrapper;
    use crate::test_utils::test_utils::{create_mock_sdk_handle, destroy_mock_sdk_handle};
    use dash_sdk::dpp::platform_value::Identifier;
    use dash_sdk::dpp::voting::contender_structs::{
        ContenderWithSerializedDocument, ContenderWithSerializedDocumentV0,
    };
    use dash_sdk::dpp::voting::vote_polls::contested_document_resource_vote_poll::ContestedDocumentResourceVotePoll;
    use dash_sdk::drive::query::vote_poll_vote_state_query::{
        ContestedDocumentVotePollDriveQuery, ContestedDocumentVotePollDriveQueryResultType,
    };
    use dash_sdk::query_types::{Contenders, ContestedResources};
    use std::collections::BTreeMap;

    /// `dpns_contract::ID_BYTES`, base58 `GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec`.
    const DPNS_CONTRACT_ID: [u8; 32] =
        dash_sdk::dpp::system_data_contracts::dpns_contract::ID_BYTES;

    #[test]
    fn test_get_contested_resources_null_handle() {
        unsafe {
            let result = dash_sdk_contested_resource_get_resources(
                std::ptr::null(),
                CString::new("test").unwrap().as_ptr(),
                CString::new("type").unwrap().as_ptr(),
                CString::new("index").unwrap().as_ptr(),
                std::ptr::null(),
                std::ptr::null(),
                10,
                true,
            );
            assert!(!result.error.is_null());
        }
    }

    #[test]
    fn test_get_contested_resources_null_contract_id() {
        let handle = create_mock_sdk_handle();
        unsafe {
            let result = dash_sdk_contested_resource_get_resources(
                handle,
                std::ptr::null(),
                CString::new("type").unwrap().as_ptr(),
                CString::new("index").unwrap().as_ptr(),
                std::ptr::null(),
                std::ptr::null(),
                10,
                true,
            );
            assert!(!result.error.is_null());
            crate::test_utils::test_utils::destroy_mock_sdk_handle(handle);
        }
    }

    #[test]
    fn should_render_a_text_label_as_its_own_id_and_full_index_path() {
        let json = contested_resource_json(
            &Value::Text("a11ce0".to_string()),
            "GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec",
            "domain",
            "parentNameAndLabel",
            &[Value::Text("dash".to_string())],
            &[],
        );
        assert_eq!(
            json,
            json!({
                "id": "a11ce0",
                "contract_id": "GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec",
                "document_type_name": "domain",
                "index_name": "parentNameAndLabel",
                "index_values": ["dash", "a11ce0"],
            })
        );
    }

    #[test]
    fn should_pass_a_label_starting_with_0x_back_into_vote_state_and_cast_vote_as_text() {
        // DPNS normalizes "oxford" to "0xf0rd". The listed path is fed back
        // through `parse_index_values_json`, the parser the vote state,
        // voters and cast vote FFIs use.
        let json = contested_resource_json(
            &Value::Text("0xf0rd".to_string()),
            "GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec",
            "domain",
            "parentNameAndLabel",
            &[Value::Text("dash".to_string())],
            &[],
        );
        assert_eq!(json["id"], json!("\\0xf0rd"));
        assert_eq!(json["index_values"], json!(["dash", "\\0xf0rd"]));
        assert_eq!(
            parse_index_values_json(&json["index_values"].to_string()).unwrap(),
            vec![
                Value::Text("dash".to_string()),
                Value::Text("0xf0rd".to_string()),
            ]
        );
    }

    #[test]
    fn should_render_a_bytes_value_as_0x_hex_between_start_and_end_values() {
        let json = contested_resource_json(
            &Value::Bytes(vec![0x01, 0xff]),
            "contract",
            "type",
            "index",
            &[Value::Text("a".to_string())],
            &[Value::Bytes(vec![0x0a])],
        );
        assert_eq!(json["id"], json!("0x01ff"));
        assert_eq!(json["index_values"], json!(["a", "0x01ff", "0x0a"]));
    }

    /// Mock SDK handle that answers the DPNS listing under `["dash"]` with
    /// `listed_label`, and the vote state of `["dash", poll_label]` (tally
    /// only) with one contender holding 5 votes. Any other request finds no
    /// expectation and fails.
    fn create_mock_sdk_handle_with_dpns_contest(
        listed_label: &str,
        poll_label: Value,
    ) -> *mut SDKHandle {
        let mut wrapper = Box::new(SDKWrapper::new_mock());
        let contract_id = Identifier::new(DPNS_CONTRACT_ID);
        let contender_id = Identifier::new([7u8; 32]);

        // Registering expectations is async, and the FFI entry points below
        // spin up their own runtime and block on it, so this runtime must be
        // gone by the time they run.
        let setup_runtime = tokio::runtime::Runtime::new().expect("create setup runtime");
        setup_runtime.block_on(async {
            let mut mock = wrapper.sdk.mock();
            mock.expect_fetch_many::<_, ContestedResource, _, ContestedResources>(
                VotePollsByDocumentTypeQuery {
                    contract_id,
                    document_type_name: "domain".to_string(),
                    index_name: "parentNameAndLabel".to_string(),
                    start_index_values: vec![Value::Text("dash".to_string())],
                    end_index_values: vec![],
                    start_at_value: None,
                    limit: None,
                    order_ascending: true,
                },
                Some(ContestedResources(vec![ContestedResource(Value::Text(
                    listed_label.to_string(),
                ))])),
            )
            .await
            .expect("register listing expectation");
            mock.expect_fetch_many::<_, ContenderWithSerializedDocument, _, Contenders>(
                ContestedDocumentVotePollDriveQuery {
                    vote_poll: ContestedDocumentResourceVotePoll {
                        contract_id,
                        document_type_name: "domain".to_string(),
                        index_name: "parentNameAndLabel".to_string(),
                        index_values: vec![Value::Text("dash".to_string()), poll_label],
                    },
                    result_type: ContestedDocumentVotePollDriveQueryResultType::VoteTally,
                    limit: None,
                    start_at: None,
                    allow_include_locked_and_abstaining_vote_tally: true,
                    offset: None,
                },
                Some(Contenders {
                    winner: None,
                    contenders: BTreeMap::from([(
                        contender_id,
                        ContenderWithSerializedDocument::V0(ContenderWithSerializedDocumentV0 {
                            identity_id: contender_id,
                            serialized_document: None,
                            vote_tally: Some(5),
                        }),
                    )]),
                    abstain_vote_tally: Some(0),
                    lock_vote_tally: Some(0),
                }),
            )
            .await
            .expect("register vote state expectation");
        });
        drop(setup_runtime);

        Box::into_raw(wrapper) as *mut SDKHandle
    }

    unsafe fn json_of(result: DashSDKResult) -> serde_json::Value {
        assert!(result.error.is_null(), "the call must succeed");
        assert_eq!(result.data_type, DashSDKResultDataType::String);
        let json = CString::from_raw(result.data as *mut c_char)
            .into_string()
            .expect("utf-8 json");
        serde_json::from_str(&json).expect("valid json")
    }

    /// Lists the DPNS contests under `["dash"]` through the export, then
    /// passes the first listed `index_values` unchanged to the vote state
    /// export, returning that vote state.
    fn list_then_read_vote_state(handle: *mut SDKHandle) -> serde_json::Value {
        let contract_id = CString::new(bs58::encode(DPNS_CONTRACT_ID).into_string()).unwrap();
        let document_type = CString::new("domain").unwrap();
        let index_name = CString::new("parentNameAndLabel").unwrap();
        let start = CString::new(r#"["dash"]"#).unwrap();
        unsafe {
            let listing = json_of(dash_sdk_contested_resource_get_resources(
                handle,
                contract_id.as_ptr(),
                document_type.as_ptr(),
                index_name.as_ptr(),
                start.as_ptr(),
                std::ptr::null(),
                0,
                true,
            ));
            let index_values = CString::new(listing[0]["index_values"].to_string()).unwrap();
            json_of(dash_sdk_contested_resource_get_vote_state(
                handle,
                contract_id.as_ptr(),
                document_type.as_ptr(),
                index_name.as_ptr(),
                index_values.as_ptr(),
                1, // VoteTally: no contract fetch, no documents
                true,
                0,
            ))
        }
    }

    #[test]
    fn should_read_the_vote_state_of_a_listed_0x_label_as_text_through_the_exports() {
        // "0xface" is valid hex: read as bytes it would address another poll,
        // which has no expectation here, and the call would fail.
        let handle =
            create_mock_sdk_handle_with_dpns_contest("0xface", Value::Text("0xface".to_string()));
        let vote_state = list_then_read_vote_state(handle);
        assert_eq!(vote_state["contenders"][0]["vote_count"], 5);
        destroy_mock_sdk_handle(handle);
    }

    #[test]
    fn should_not_match_the_bytes_poll_with_an_escaped_0x_label() {
        // Control for the test above: the mock answers only the bytes poll,
        // so a text lookup of the listed "0xface" finds no expectation.
        let handle =
            create_mock_sdk_handle_with_dpns_contest("0xface", Value::Bytes(vec![0xfa, 0xce]));
        let contract_id = CString::new(bs58::encode(DPNS_CONTRACT_ID).into_string()).unwrap();
        let document_type = CString::new("domain").unwrap();
        let index_name = CString::new("parentNameAndLabel").unwrap();
        let index_values = CString::new(r#"["dash", "\\0xface"]"#).unwrap();
        unsafe {
            let result = dash_sdk_contested_resource_get_vote_state(
                handle,
                contract_id.as_ptr(),
                document_type.as_ptr(),
                index_name.as_ptr(),
                index_values.as_ptr(),
                1,
                true,
                0,
            );
            assert!(
                !result.error.is_null(),
                "a text lookup must not match the bytes poll"
            );
            crate::dash_sdk_error_free(result.error);
        }
        destroy_mock_sdk_handle(handle);
    }
}
