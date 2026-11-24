use crate::types::SDKHandle;
use crate::{DashSDKError, DashSDKErrorCode, DashSDKResult, DashSDKResultDataType};
use dash_sdk::dpp::group::group_action::{GroupAction, GroupActionAccessors};
use dash_sdk::dpp::group::group_action_status::GroupActionStatus;
use dash_sdk::platform::{group_actions::GroupActionsQuery, FetchMany, Identifier};
use std::ffi::{c_char, c_void, CStr, CString};

/// Fetches group actions
///
/// # Parameters
/// * `sdk_handle` - Handle to the SDK instance
/// * `contract_id` - Base58-encoded contract identifier
/// * `group_contract_position` - Position of the group in the contract
/// * `status` - Action status (0=Pending, 1=Completed, 2=Expired)
/// * `start_at_action_id` - Optional starting action ID (Base58-encoded)
/// * `limit` - Maximum number of actions to return; 0 lets Platform use its default
///
/// # Returns
/// * JSON array of group actions, or null if none was found. Each entry is
///   `{"id": <base58>, "action": null | {"contract_id": <base58>,
///   "proposer_id": <base58>, "token_contract_position": <number>,
///   "event": <object>}}`. `event` is the `GroupActionEvent` in its serde
///   JSON shape, e.g. `{"$kind": "tokenEvent", "$type": "mint", ...}`
///   (earlier releases returned a string of Rust `Debug` text here).
/// * Error message if operation fails
///
/// # Safety
/// - `sdk_handle` and `contract_id` must be valid, non-null pointers.
/// - `contract_id` must point to a NUL-terminated C string; `start_at_action_id` may be null, otherwise must be a valid NUL-terminated C string.
/// - On success, returns a C string pointer inside `DashSDKResult`; caller must free it using SDK routines.
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_group_get_actions(
    sdk_handle: *const SDKHandle,
    contract_id: *const c_char,
    group_contract_position: u16,
    status: u8,
    start_at_action_id: *const c_char,
    limit: u16,
) -> DashSDKResult {
    match get_group_actions(
        sdk_handle,
        contract_id,
        group_contract_position,
        status,
        start_at_action_id,
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

fn get_group_actions(
    sdk_handle: *const SDKHandle,
    contract_id: *const c_char,
    group_contract_position: u16,
    status: u8,
    start_at_action_id: *const c_char,
    limit: u16,
) -> Result<Option<String>, String> {
    // Check for null pointers
    if sdk_handle.is_null() {
        return Err("SDK handle is null".to_string());
    }
    if contract_id.is_null() {
        return Err("Contract ID is null".to_string());
    }

    let rt = crate::runtime::BigStackRuntime::new_isolated()
        .map_err(|e| format!("Failed to create Tokio runtime: {}", e))?;

    let contract_id_str = unsafe {
        CStr::from_ptr(contract_id)
            .to_str()
            .map_err(|e| format!("Invalid UTF-8 in contract ID: {}", e))?
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

        let status = match status {
            0 => GroupActionStatus::ActionActive,
            1 => GroupActionStatus::ActionClosed,
            _ => return Err("Invalid status value".to_string()),
        };

        let start_at_action_id = if start_at_action_id.is_null() {
            None
        } else {
            let action_id_str = unsafe {
                CStr::from_ptr(start_at_action_id)
                    .to_str()
                    .map_err(|e| format!("Invalid UTF-8 in start action ID: {}", e))?
            };
            let action_id_bytes = bs58::decode(action_id_str)
                .into_vec()
                .map_err(|e| format!("Failed to decode start action ID: {}", e))?;
            let action_id: [u8; 32] = action_id_bytes
                .try_into()
                .map_err(|_| "Action ID must be exactly 32 bytes".to_string())?;
            Some((dash_sdk::platform::Identifier::new(action_id), true))
        };

        let query = GroupActionsQuery {
            contract_id,
            group_contract_position,
            status,
            start_at_action_id,
            // 0 means "not set", as in the other query entry points: Platform
            // refuses an explicit limit of 0.
            limit: if limit == 0 { None } else { Some(limit) },
        };

        match GroupAction::fetch_many(&sdk, query).await {
            Ok(actions) => {
                if actions.is_empty() {
                    return Ok(None);
                }
                group_actions_json(actions.iter()).map(Some)
            }
            Err(e) => Err(format!("Failed to fetch group actions: {}", e)),
        }
    })
}

/// The response JSON for [`dash_sdk_group_get_actions`]; see its "Returns".
fn group_actions_json<'a>(
    actions: impl IntoIterator<Item = (&'a Identifier, &'a Option<GroupAction>)>,
) -> Result<String, String> {
    let entries = actions
        .into_iter()
        .map(|(id, action_opt)| {
            let action_json = match action_opt {
                Some(action) => {
                    // The event in its serde wire shape, e.g.
                    // {"$kind":"tokenEvent","$type":"mint",...}.
                    let event = serde_json::to_value(action.event())
                        .map_err(|e| format!("Failed to encode group action event: {}", e))?;
                    serde_json::json!({
                        "contract_id": bs58::encode(action.contract_id().as_bytes()).into_string(),
                        "proposer_id": bs58::encode(action.proposer_id().as_bytes()).into_string(),
                        "token_contract_position": action.token_contract_position(),
                        "event": event,
                    })
                }
                None => serde_json::Value::Null,
            };
            Ok(serde_json::json!({
                "id": bs58::encode(id.as_bytes()).into_string(),
                "action": action_json,
            }))
        })
        .collect::<Result<Vec<_>, String>>()?;
    Ok(serde_json::Value::Array(entries).to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::test_utils::create_mock_sdk_handle;

    #[test]
    fn test_get_group_actions_null_handle() {
        unsafe {
            let result = dash_sdk_group_get_actions(
                std::ptr::null(),
                CString::new("test").unwrap().as_ptr(),
                0,
                0,
                std::ptr::null(),
                10,
            );
            assert!(!result.error.is_null());
        }
    }

    #[test]
    fn test_get_group_actions_null_contract_id() {
        let handle = create_mock_sdk_handle();
        unsafe {
            let result =
                dash_sdk_group_get_actions(handle, std::ptr::null(), 0, 0, std::ptr::null(), 10);
            assert!(!result.error.is_null());
            crate::test_utils::test_utils::destroy_mock_sdk_handle(handle);
        }
    }

    #[test]
    fn should_encode_group_actions_as_json_with_an_object_event() {
        use dash_sdk::dpp::group::action_event::GroupActionEvent;
        use dash_sdk::dpp::group::group_action::v0::GroupActionV0;
        use dash_sdk::dpp::tokens::token_event::TokenEvent;

        let action_id = Identifier::new([1; 32]);
        let closed_id = Identifier::new([2; 32]);
        let contract_id = Identifier::new([3; 32]);
        let proposer_id = Identifier::new([4; 32]);
        let recipient = Identifier::new([5; 32]);
        // A note with quotes and a backslash: the old hand-built JSON broke on it.
        let note = r#"pay "Alice" \ now"#.to_string();
        let action = Some(GroupAction::V0(GroupActionV0 {
            contract_id,
            proposer_id,
            token_contract_position: 2,
            event: GroupActionEvent::TokenEvent(TokenEvent::Mint(
                777,
                recipient,
                Some(note.clone()),
            )),
        }));
        let actions = [(action_id, action), (closed_id, None)];

        let json = group_actions_json(actions.iter().map(|(id, a)| (id, a))).expect("encodes");
        let parsed: serde_json::Value = serde_json::from_str(&json).expect("valid JSON");

        let first = &parsed[0];
        assert_eq!(
            first["id"],
            bs58::encode(action_id.as_bytes()).into_string()
        );
        let encoded = &first["action"];
        assert_eq!(
            encoded["contract_id"],
            bs58::encode(contract_id.as_bytes()).into_string()
        );
        assert_eq!(
            encoded["proposer_id"],
            bs58::encode(proposer_id.as_bytes()).into_string()
        );
        assert_eq!(encoded["token_contract_position"], 2);
        let event = &encoded["event"];
        assert!(event.is_object(), "event must be an object, got {event}");
        assert_eq!(event["$kind"], "tokenEvent");
        assert_eq!(event["$type"], "mint");
        assert!(
            json.contains(&serde_json::to_string(&note).expect("note encodes")),
            "the note must round-trip exactly: {json}"
        );
        assert_eq!(
            parsed[1]["id"],
            bs58::encode(closed_id.as_bytes()).into_string()
        );
        assert!(parsed[1]["action"].is_null());
    }
}
