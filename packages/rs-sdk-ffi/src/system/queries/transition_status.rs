//! A state transition's status from its hash (protocol version 15): thin bindings over
//! `Sdk::recent_state_transition_status` (one node's word, unproved) and
//! `Sdk::state_transition_status_by_hash` (proved by the block the node names when that block
//! holds the transition). rs-sdk decides everything, including which answers are proved; these
//! functions read the C arguments, call it and write the answer as JSON.

use super::blocks::{block_on, json_result, required_str, sdk_wrapper, transaction_json};
use crate::types::SDKHandle;
use crate::{DashSDKResult, FFIError};
use dash_sdk::platform::transition::node_status::{
    NodeReportedError, NodeTransitionStatus, NodeTransitionView,
};
use dash_sdk::platform::transition::status_by_hash::TransitionStatusByHash;
use serde_json::{json, Value as JsonValue};
use std::ffi::c_char;

/// Asks one node what it knows of the state transition with this hash, without waiting
/// (`Sdk::recent_state_transition_status`). UNPROVED: a node can answer anything, and
/// "unknown" from one node does not mean the transition will not land.
///
/// # Parameters
/// * `sdk_handle` - Handle to the SDK instance.
/// * `state_transition_hash` - The transition's hash, 64 hex characters.
///
/// # Returns
/// A JSON object, free it with `dash_sdk_string_free`:
///
/// ```json
/// {"proved": false, "status": "committed", "height": 120,
///  "error": {"code": 40100, "message": "...", "consensus_error": "..."},
///  "node_height": 122}
/// ```
///
/// `status` is `committed`, `pending` (in the node's mempool) or `unknown`. `height` and
/// `error` are null unless committed; `error` is null when the node reports the transition
/// executed, and `consensus_error` is null when the node attached none that decodes.
/// `node_height` is the node's latest block height when it answered, null when it reported
/// none. `proved` is always false.
///
/// # Safety
/// - `sdk_handle` must be a valid pointer to an initialized `SDKHandle`.
/// - `state_transition_hash` must be a valid pointer to a NUL-terminated string for the
///   duration of the call.
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_system_get_recent_state_transition_status(
    sdk_handle: *const SDKHandle,
    state_transition_hash: *const c_char,
) -> DashSDKResult {
    json_result(get_recent_state_transition_status(
        sdk_handle,
        state_transition_hash,
    ))
}

unsafe fn get_recent_state_transition_status(
    sdk_handle: *const SDKHandle,
    state_transition_hash: *const c_char,
) -> Result<JsonValue, FFIError> {
    let wrapper = sdk_wrapper(sdk_handle)?;
    let hash = transition_hash(state_transition_hash)?;
    let view = block_on(wrapper.sdk.recent_state_transition_status(hash, None))??;
    Ok(node_view_json(&view))
}

/// The status of the state transition with this hash, proved where the network can prove it
/// (`Sdk::state_transition_status_by_hash`): when the node says the transition is committed,
/// rs-sdk reads that block with its proof and looks for the transition in it.
///
/// # Parameters
/// * `sdk_handle` - Handle to the SDK instance. It must verify proofs.
/// * `state_transition_hash` - The transition's hash, 64 hex characters.
///
/// # Returns
/// A JSON object, free it with `dash_sdk_string_free`. `proved` is true only for `in_block`:
///
/// ```json
/// {"proved": true, "status": "in_block", "block_height": 120, "block_hash": "<hex>",
///  "block_time_ms": 1700000000000, "transaction": <transaction>}
/// {"proved": false, "status": "committed_not_proved", "height": 120,
///  "error": <error or null>, "node_height": 122}
/// {"proved": false, "status": "committed_refuted", "height": 120, "node_height": 122}
/// {"proved": false, "status": "pending", "node_height": 122}
/// {"proved": false, "status": "unknown", "node_height": 122}
/// ```
///
/// `transaction` is written as in `dash_sdk_system_get_blocks` (its `code` is 0 when it
/// executed, otherwise the code of the error it paid for) and `error` as in
/// [`dash_sdk_system_get_recent_state_transition_status`]. `committed_refuted` means the
/// proved block at the height the node named does not hold the transition. `node_height` is
/// null when the node reported none.
///
/// # Safety
/// - `sdk_handle` must be a valid pointer to an initialized `SDKHandle`.
/// - `state_transition_hash` must be a valid pointer to a NUL-terminated string for the
///   duration of the call.
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_system_get_state_transition_status_by_hash(
    sdk_handle: *const SDKHandle,
    state_transition_hash: *const c_char,
) -> DashSDKResult {
    json_result(get_state_transition_status_by_hash(
        sdk_handle,
        state_transition_hash,
    ))
}

unsafe fn get_state_transition_status_by_hash(
    sdk_handle: *const SDKHandle,
    state_transition_hash: *const c_char,
) -> Result<JsonValue, FFIError> {
    let wrapper = sdk_wrapper(sdk_handle)?;
    let hash = transition_hash(state_transition_hash)?;
    let status = block_on(wrapper.sdk.state_transition_status_by_hash(hash, None))??;
    Ok(status_by_hash_json(&status))
}

/// The 32-byte hash the hex string `ptr` points to.
///
/// # Safety
/// `ptr` must be null or point to a NUL-terminated string.
unsafe fn transition_hash(ptr: *const c_char) -> Result<[u8; 32], FFIError> {
    let bytes = hex::decode(required_str(ptr, "state_transition_hash")?).map_err(|e| {
        FFIError::InvalidParameter(format!("state_transition_hash is not hex: {e}"))
    })?;
    <[u8; 32]>::try_from(bytes.as_slice()).map_err(|_| {
        FFIError::InvalidParameter(format!(
            "state_transition_hash must be 32 bytes (64 hex characters), got {} bytes",
            bytes.len()
        ))
    })
}

fn reported_error_json(error: &NodeReportedError) -> JsonValue {
    json!({
        "code": error.code,
        "message": error.message,
        "consensus_error": error.cause.as_ref().map(|cause| cause.to_string()),
    })
}

fn node_view_json(view: &NodeTransitionView) -> JsonValue {
    let (status, height, error) = match &view.status {
        NodeTransitionStatus::Committed { height, error } => {
            ("committed", Some(*height), error.as_ref())
        }
        NodeTransitionStatus::Pending => ("pending", None, None),
        NodeTransitionStatus::Unknown => ("unknown", None, None),
    };
    json!({
        "proved": false,
        "status": status,
        "height": height,
        "error": error.map(reported_error_json),
        "node_height": view.node_height,
    })
}

fn status_by_hash_json(status: &TransitionStatusByHash) -> JsonValue {
    let proved = status.is_proved();
    match status {
        TransitionStatusByHash::InBlock {
            block_height,
            block_hash,
            block_time_ms,
            transaction,
        } => json!({
            "proved": proved,
            "status": "in_block",
            "block_height": block_height,
            "block_hash": hex::encode(block_hash),
            "block_time_ms": block_time_ms,
            "transaction": transaction_json(transaction),
        }),
        TransitionStatusByHash::CommittedNotProved {
            height,
            error,
            node_height,
        } => json!({
            "proved": proved,
            "status": "committed_not_proved",
            "height": height,
            "error": error.as_ref().map(reported_error_json),
            "node_height": node_height,
        }),
        TransitionStatusByHash::CommittedRefuted {
            height,
            node_height,
        } => json!({
            "proved": proved,
            "status": "committed_refuted",
            "height": height,
            "node_height": node_height,
        }),
        TransitionStatusByHash::Pending { node_height } => json!({
            "proved": proved,
            "status": "pending",
            "node_height": node_height,
        }),
        TransitionStatusByHash::Unknown { node_height } => json!({
            "proved": proved,
            "status": "unknown",
            "node_height": node_height,
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::super::blocks::tests::{error_code, transaction};
    use super::*;
    use crate::test_utils::test_utils::{create_mock_sdk_handle, destroy_mock_sdk_handle};
    use crate::DashSDKErrorCode;
    use std::ffi::CString;

    #[test]
    fn should_refuse_a_null_handle() {
        let hash = CString::new("07".repeat(32)).expect("no NUL");
        unsafe {
            assert_eq!(
                error_code(dash_sdk_system_get_recent_state_transition_status(
                    std::ptr::null(),
                    hash.as_ptr(),
                )),
                DashSDKErrorCode::InvalidParameter
            );
            assert_eq!(
                error_code(dash_sdk_system_get_state_transition_status_by_hash(
                    std::ptr::null(),
                    hash.as_ptr(),
                )),
                DashSDKErrorCode::InvalidParameter
            );
        }
    }

    #[test]
    fn should_refuse_a_hash_that_is_not_32_hex_bytes_before_asking_the_network() {
        let handle = create_mock_sdk_handle();
        let not_hex = CString::new("zz".repeat(32)).expect("no NUL");
        let too_short = CString::new("07".repeat(31)).expect("no NUL");
        unsafe {
            for hash in [std::ptr::null(), not_hex.as_ptr(), too_short.as_ptr()] {
                assert_eq!(
                    error_code(dash_sdk_system_get_recent_state_transition_status(
                        handle, hash,
                    )),
                    DashSDKErrorCode::InvalidParameter
                );
                assert_eq!(
                    error_code(dash_sdk_system_get_state_transition_status_by_hash(
                        handle, hash,
                    )),
                    DashSDKErrorCode::InvalidParameter
                );
            }
            destroy_mock_sdk_handle(handle);
        }
    }

    #[test]
    fn should_write_a_node_view_as_unproved() {
        let committed = NodeTransitionView {
            status: NodeTransitionStatus::Committed {
                height: 120,
                error: Some(NodeReportedError {
                    code: 40_100,
                    message: "insufficient balance".to_string(),
                    cause: None,
                }),
            },
            node_height: Some(122),
        };
        assert_eq!(
            node_view_json(&committed),
            json!({
                "proved": false,
                "status": "committed",
                "height": 120,
                "error": {"code": 40_100, "message": "insufficient balance", "consensus_error": null},
                "node_height": 122,
            })
        );
        let unknown = NodeTransitionView {
            status: NodeTransitionStatus::Unknown,
            node_height: None,
        };
        assert_eq!(
            node_view_json(&unknown),
            json!({
                "proved": false,
                "status": "unknown",
                "height": null,
                "error": null,
                "node_height": null,
            })
        );
    }

    #[test]
    fn should_write_only_a_transition_found_in_its_block_as_proved() {
        let in_block = TransitionStatusByHash::InBlock {
            block_height: 120,
            block_hash: [9; 32],
            block_time_ms: 1_700_000_000_000,
            transaction: Box::new(transaction([7; 32], 0)),
        };
        let json = status_by_hash_json(&in_block);
        assert_eq!(json["proved"], true);
        assert_eq!(json["status"], "in_block");
        assert_eq!(json["block_height"], 120);
        assert_eq!(json["block_hash"], "09".repeat(32));
        assert_eq!(json["block_time_ms"], 1_700_000_000_000u64);
        assert_eq!(json["transaction"]["hash"], "07".repeat(32));
        assert_eq!(json["transaction"]["code"], 0);

        for (status, name) in [
            (
                TransitionStatusByHash::CommittedNotProved {
                    height: 120,
                    error: None,
                    node_height: Some(122),
                },
                "committed_not_proved",
            ),
            (
                TransitionStatusByHash::CommittedRefuted {
                    height: 120,
                    node_height: Some(122),
                },
                "committed_refuted",
            ),
            (
                TransitionStatusByHash::Pending {
                    node_height: Some(122),
                },
                "pending",
            ),
            (
                TransitionStatusByHash::Unknown { node_height: None },
                "unknown",
            ),
        ] {
            let json = status_by_hash_json(&status);
            assert_eq!(json["proved"], false, "{name}");
            assert_eq!(json["status"], name);
        }
    }
}
