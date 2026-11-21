use crate::contested_resource::contender_document::json_members;
use crate::contested_resource::request_limit;
use crate::types::SDKHandle;
use crate::{DashSDKError, DashSDKErrorCode, DashSDKResult, DashSDKResultDataType};
use dash_sdk::dpp::data_contract::accessors::v0::DataContractV0Getters;
use dash_sdk::dpp::data_contract::document_type::DocumentTypeRef;
use dash_sdk::dpp::platform_value::Value;
use dash_sdk::dpp::version::PlatformVersion;
use dash_sdk::dpp::voting::contender_structs::ContenderWithSerializedDocument;
use dash_sdk::dpp::voting::vote_info_storage::contested_document_vote_poll_winner_info::ContestedDocumentVotePollWinnerInfo;
use dash_sdk::dpp::voting::vote_polls::contested_document_resource_vote_poll::ContestedDocumentResourceVotePoll;
use dash_sdk::drive::query::vote_poll_vote_state_query::{
    ContestedDocumentVotePollDriveQuery, ContestedDocumentVotePollDriveQueryResultType,
};
use dash_sdk::platform::dpns_usernames::ContenderRequest;
use dash_sdk::platform::FetchMany;
use dash_sdk::platform::{DataContract, Fetch, Identifier};
use dash_sdk::query_types::Contenders;
use std::ffi::{c_char, c_void, CStr, CString};

/// Fetches contested resource vote state
///
/// # Parameters
/// * `sdk_handle` - Handle to the SDK instance
/// * `contract_id` - Base58-encoded contract identifier
/// * `document_type_name` - Name of the document type
/// * `index_name` - Name of the index
/// * `index_values_json` - JSON array of index values. A `"0x"`-prefixed
///   element is hex-decoded to bytes; any other element is taken verbatim as
///   text (matches the cast-vote write path).
/// * `result_type` - Result type (0=DOCUMENTS, 1=VOTE_TALLY, 2=DOCUMENTS_AND_VOTE_TALLY)
/// * `allow_include_locked_and_abstaining_vote_tally` - Whether to include locked and abstaining votes
/// * `count` - The Drive request limit for this single request: at most
///   `count` contenders are asked for. 0 sends no limit, so the node applies
///   its default page size. A value above `u16::MAX` is refused with
///   `InvalidParameter`. A finished poll's stored record is returned whole
///   whatever the limit. The FFI does not page: it serializes the rows the
///   query returns.
///
/// Result type 0 (DOCUMENTS) does not read a finalized poll's stored record:
/// for a contest that ended, use a tally-bearing result type (1 or 2), where
/// the contenders' document bytes may be unavailable.
///
/// # Returns
/// * A JSON object, or no data when Platform has no contenders on record:
///   `{"abstain_vote_tally":N,"lock_vote_tally":N,"winner_info":…,"block_info":{…},
///   "contenders":[{"identity_id":"…","vote_count":N,"document":"hex"|null,"label":"…","created_at":<ms>}]}`
///   - `abstain_vote_tally` and `lock_vote_tally` each only when Platform
///     returned that tally. A tally that was not read is left out, never
///     written as 0.
///   - `winner_info` and `block_info` only once the poll has finished.
///   - `contenders` for every result type, as the query returned them.
///     `identity_id` is the verified contender key: an identity
///     id for identity-keyed contests such as DPNS. `vote_count` only when
///     the contender's tally was read (result types 1 and 2). `document`
///     only for result types 0 and 2, with `label` and `created_at` when the
///     document decodes.
/// * Error message if operation fails
///
/// # Safety
/// - `sdk_handle` must be a valid, non-null pointer to an initialized `SDKHandle`.
/// - All C string pointers (`contract_id`, `document_type_name`, `index_name`, `index_values_json`)
///   must be valid, non-null pointers to NUL-terminated strings that remain valid for the duration of the call.
/// - `result_type` and `allow_include_locked_and_abstaining_vote_tally` are passed by value.
/// - The returned result may contain a heap-allocated C string which must be freed by the caller using
///   the SDK's free routine. It may also contain no data (null pointer) on success.
/// - All pointers must point to readable memory; passing invalid or dangling pointers results in undefined behavior.
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_contested_resource_get_vote_state(
    sdk_handle: *const SDKHandle,
    contract_id: *const c_char,
    document_type_name: *const c_char,
    index_name: *const c_char,
    index_values_json: *const c_char,
    result_type: u8,
    allow_include_locked_and_abstaining_vote_tally: bool,
    count: u32,
) -> DashSDKResult {
    let limit = match request_limit(count) {
        Ok(limit) => limit,
        Err(error) => return DashSDKResult::error(error),
    };
    match get_contested_resource_vote_state(
        sdk_handle,
        contract_id,
        document_type_name,
        index_name,
        index_values_json,
        result_type,
        allow_include_locked_and_abstaining_vote_tally,
        limit,
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
fn get_contested_resource_vote_state(
    sdk_handle: *const SDKHandle,
    contract_id: *const c_char,
    document_type_name: *const c_char,
    index_name: *const c_char,
    index_values_json: *const c_char,
    result_type: u8,
    allow_include_locked_and_abstaining_vote_tally: bool,
    limit: Option<u16>,
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

    if index_values_json.is_null() {
        return Err("Index values JSON is null".to_string());
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
    let index_values_str = unsafe {
        CStr::from_ptr(index_values_json)
            .to_str()
            .map_err(|e| format!("Invalid UTF-8 in index values: {}", e))?
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

        let contract_id = Identifier::new(contract_id);

        // Parse index values
        let index_values_array: Vec<String> = serde_json::from_str(index_values_str)
            .map_err(|e| format!("Failed to parse index values JSON: {}", e))?;

        // Explicit type tag: a `"0x"`-prefixed value is hex-decoded to
        // `Value::Bytes`, anything else is verbatim `Value::Text`. Shared with
        // the cast-vote write path so read and write stay in lockstep.
        let index_values: Vec<Value> = index_values_array
            .into_iter()
            .map(crate::contested_resource::parse_index_value)
            .collect::<Result<Vec<Value>, String>>()?;

        let result_type = match result_type {
            0 => ContestedDocumentVotePollDriveQueryResultType::Documents,
            1 => ContestedDocumentVotePollDriveQueryResultType::VoteTally,
            2 => ContestedDocumentVotePollDriveQueryResultType::DocumentsAndVoteTally,
            _ => return Err("Invalid result type".to_string()),
        };

        let vote_poll = ContestedDocumentResourceVotePoll {
            contract_id,
            document_type_name: document_type_name_str.to_string(),
            index_name: index_name_str.to_string(),
            index_values,
        };
        let query = ContestedDocumentVotePollDriveQuery {
            vote_poll,
            result_type,
            limit,
            start_at: None,
            allow_include_locked_and_abstaining_vote_tally,
            offset: None,
        };

        match ContenderWithSerializedDocument::fetch_many(&sdk, query).await {
            Ok(contenders) => {
                let contenders: Contenders = contenders;
                if contenders.contenders.is_empty() {
                    return Ok(None);
                }

                // Decoding each contender's document gives callers the label
                // the requester actually typed ("pizza") next to the
                // homograph-normalized index value ("p1zza"). Without this a
                // UI can only show the normalized form, which reads as a typo
                // to the person who submitted it.
                //
                // The same decode gives `created_at`, the document's
                // `$createdAt` in milliseconds: when the request was made,
                // passed on as the document holds it (0 included). The decode
                // is the SDK's `ContenderRequest`.
                //
                // Best-effort: the contract fetch or a single decode failing
                // must not fail the whole query. `label` and `created_at` are
                // then simply absent for that row: the caller shows the
                // normalized value instead of a label, and has no date for the
                // request. A tally-only query has no documents to decode, so
                // it skips the contract fetch.
                let contract: Option<DataContract> = if result_type.has_documents() {
                    DataContract::fetch(&sdk, contract_id).await.ok().flatten()
                } else {
                    None
                };
                let document_type = contract.as_ref().and_then(|contract| {
                    contract.document_type_for_name(document_type_name_str).ok()
                });

                Ok(Some(vote_state_json(
                    &contenders,
                    result_type,
                    document_type,
                    sdk.version(),
                )))
            }
            Err(e) => Err(format!(
                "Failed to fetch contested resource vote state: {}",
                e
            )),
        }
    })
}

/// The vote-state JSON object for `contenders` (see
/// [`dash_sdk_contested_resource_get_vote_state`] for its shape): each tally
/// only when Platform returned it, the winner once the poll has finished, and
/// every contender the query returned. Contenders are decoded against
/// `document_type` only for a result type with documents.
fn vote_state_json(
    contenders: &Contenders,
    result_type: ContestedDocumentVotePollDriveQueryResultType,
    document_type: Option<DocumentTypeRef>,
    platform_version: &PlatformVersion,
) -> String {
    let mut result_json_parts = Vec::new();
    if let Some(abstain_vote_tally) = contenders.abstain_vote_tally {
        result_json_parts.push(format!(r#""abstain_vote_tally":{}"#, abstain_vote_tally));
    }
    if let Some(lock_vote_tally) = contenders.lock_vote_tally {
        result_json_parts.push(format!(r#""lock_vote_tally":{}"#, lock_vote_tally));
    }
    if let Some((winner_info, block_info)) = &contenders.winner {
        let winner_json = match winner_info {
            ContestedDocumentVotePollWinnerInfo::NoWinner => {
                r#""winner_info":"NoWinner""#.to_string()
            }
            ContestedDocumentVotePollWinnerInfo::WonByIdentity(identifier) => format!(
                r#""winner_info":{{"type":"WonByIdentity","identity_id":"{}"}}"#,
                bs58::encode(identifier.as_bytes()).into_string()
            ),
            ContestedDocumentVotePollWinnerInfo::Locked => r#""winner_info":"Locked""#.to_string(),
        };
        result_json_parts.push(format!(
            r#"{},"block_info":{{"height":{},"core_height":{},"timestamp":{}}}"#,
            winner_json, block_info.height, block_info.core_height, block_info.time_ms
        ));
    }
    let contenders_json: Vec<String> = contenders
        .contenders
        .iter()
        .map(|(id, contender)| {
            let request = result_type
                .has_documents()
                .then(|| ContenderRequest::decode(contender, document_type, platform_version));
            contender_json(id, contender, request.as_ref())
        })
        .collect();
    result_json_parts.push(format!(r#""contenders":[{}]"#, contenders_json.join(",")));
    format!("{{{}}}", result_json_parts.join(","))
}

/// One contender object of the vote-state JSON:
/// `{"identity_id":"…","vote_count":N}`, with `vote_count` only when the
/// contender's tally was read. When `request` is given (the query
/// asked for documents) the object also carries `"document"` (the serialized
/// document in hex, or null) and the decoded request's `label` and
/// `created_at` (see [`json_members`]).
fn contender_json(
    id: &Identifier,
    contender: &ContenderWithSerializedDocument,
    request: Option<&ContenderRequest>,
) -> String {
    let document_members = request
        .map(|request| {
            let document = match contender.serialized_document() {
                Some(document) => format!(r#""{}""#, hex::encode(document)),
                None => "null".to_string(),
            };
            format!(r#","document":{}{}"#, document, json_members(request))
        })
        .unwrap_or_default();
    let vote_count_member = contender
        .vote_tally()
        .map(|vote_count| format!(r#","vote_count":{}"#, vote_count))
        .unwrap_or_default();
    format!(
        r#"{{"identity_id":"{}"{}{}}}"#,
        bs58::encode(id.as_bytes()).into_string(),
        vote_count_member,
        document_members
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contested_resource::contender_document::test_support::{
        requester_document, serialized_contender, CREATED_AT_MS,
    };
    use crate::test_utils::test_utils::create_mock_sdk_handle;
    use dash_sdk::dpp::document::DocumentV0Getters;
    use dash_sdk::dpp::system_data_contracts::{load_system_data_contract, SystemDataContract};
    use dash_sdk::dpp::version::PlatformVersion;
    use dash_sdk::dpp::voting::contender_structs::ContenderWithSerializedDocumentV0;
    use std::collections::BTreeMap;

    #[test]
    fn should_write_a_tally_only_contender_with_identity_id_and_vote_count_and_no_document() {
        let id = Identifier::from([7u8; 32]);
        let contender = ContenderWithSerializedDocument::V0(ContenderWithSerializedDocumentV0 {
            identity_id: id,
            serialized_document: None,
            vote_tally: Some(3),
        });
        assert_eq!(
            contender_json(&id, &contender, None),
            format!(
                r#"{{"identity_id":"{}","vote_count":3}}"#,
                bs58::encode(id.as_bytes()).into_string()
            )
        );
    }

    #[test]
    fn should_write_a_document_contender_with_its_document_label_and_created_at() {
        let document = requester_document(1, "pizza", Some(CREATED_AT_MS));
        let id = document.owner_id();
        let contender = serialized_contender(document);
        let platform_version = PlatformVersion::latest();
        let contract = load_system_data_contract(SystemDataContract::DPNS, platform_version)
            .expect("DPNS system contract");
        let request = ContenderRequest::decode(
            &contender,
            contract.document_type_for_name("domain").ok(),
            platform_version,
        );
        let hex_document = hex::encode(
            contender
                .serialized_document()
                .as_ref()
                .expect("serialized document"),
        );
        assert_eq!(
            contender_json(&id, &contender, Some(&request)),
            format!(
                r#"{{"identity_id":"{}","vote_count":3,"document":"{}","label":"pizza","created_at":{}}}"#,
                bs58::encode(id.as_bytes()).into_string(),
                hex_document,
                CREATED_AT_MS
            )
        );
    }

    fn tally_contender(
        owner: u8,
        vote_tally: Option<u32>,
    ) -> (Identifier, ContenderWithSerializedDocument) {
        let identity_id = Identifier::from([owner; 32]);
        (
            identity_id,
            ContenderWithSerializedDocument::V0(ContenderWithSerializedDocumentV0 {
                identity_id,
                serialized_document: None,
                vote_tally,
            }),
        )
    }

    fn parse(json: &str) -> serde_json::Value {
        serde_json::from_str(json).expect("vote state JSON")
    }

    #[test]
    fn should_answer_a_vote_tally_query_with_contender_ids_and_vote_counts_and_no_documents() {
        let (id, contender) = tally_contender(7, Some(5));
        let contenders = Contenders {
            winner: None,
            contenders: BTreeMap::from([(id, contender)]),
            abstain_vote_tally: Some(2),
            lock_vote_tally: Some(1),
        };

        let json = parse(&vote_state_json(
            &contenders,
            ContestedDocumentVotePollDriveQueryResultType::VoteTally,
            None,
            PlatformVersion::latest(),
        ));

        assert_eq!(json["abstain_vote_tally"], 2);
        assert_eq!(json["lock_vote_tally"], 1);
        let rows = json["contenders"].as_array().expect("contenders array");
        assert_eq!(rows.len(), 1);
        assert_eq!(
            rows[0]["identity_id"],
            bs58::encode(id.as_bytes()).into_string()
        );
        assert_eq!(rows[0]["vote_count"], 5);
        assert!(rows[0].get("document").is_none());
    }

    #[test]
    fn should_leave_out_tallies_that_were_not_read() {
        let (id, contender) = tally_contender(7, None);
        let contenders = Contenders {
            winner: None,
            contenders: BTreeMap::from([(id, contender)]),
            abstain_vote_tally: None,
            lock_vote_tally: None,
        };

        let json = parse(&vote_state_json(
            &contenders,
            ContestedDocumentVotePollDriveQueryResultType::Documents,
            None,
            PlatformVersion::latest(),
        ));

        assert!(json.get("abstain_vote_tally").is_none());
        assert!(json.get("lock_vote_tally").is_none());
        let rows = json["contenders"].as_array().expect("contenders array");
        assert_eq!(rows.len(), 1);
        assert!(rows[0].get("vote_count").is_none());
        assert!(rows[0]["document"].is_null());
    }

    #[test]
    fn test_get_contested_resource_vote_state_null_handle() {
        unsafe {
            let result = dash_sdk_contested_resource_get_vote_state(
                std::ptr::null(),
                CString::new("test").unwrap().as_ptr(),
                CString::new("type").unwrap().as_ptr(),
                CString::new("index").unwrap().as_ptr(),
                CString::new(r#"["00"]"#).unwrap().as_ptr(),
                0,
                false,
                10,
            );
            assert!(!result.error.is_null());
        }
    }

    #[test]
    fn test_get_contested_resource_vote_state_null_contract_id() {
        let handle = create_mock_sdk_handle();
        unsafe {
            let result = dash_sdk_contested_resource_get_vote_state(
                handle,
                std::ptr::null(),
                CString::new("type").unwrap().as_ptr(),
                CString::new("index").unwrap().as_ptr(),
                CString::new(r#"["00"]"#).unwrap().as_ptr(),
                0,
                false,
                10,
            );
            assert!(!result.error.is_null());
            crate::test_utils::test_utils::destroy_mock_sdk_handle(handle);
        }
    }
}
