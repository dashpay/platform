//! Contract moderation and fee pot reads. Each reads its C arguments, calls one rs-sdk entry
//! point and returns JSON written by [`super::json`], or no data when rs-sdk returns none.
//!
//! Arguments are refused before any request only when they cannot be represented in the rs-sdk
//! query: a null handle, a null or non-base58 id, a byte that names no list or status, a limit
//! above `u16::MAX`, an inclusion flag without the action it includes. Everything else (a limit
//! above the page cap, an empty id list, a list the contract does not keep) is the node's to
//! refuse.

use super::json::{
    action_counts_json, entries_json, fee_pots_json, removals_json, status_json,
    team_action_signers_json, team_actions_json,
};
use super::{moderation_list, team_action_status};
use crate::decision_proposal::{
    json_result, optional_identifier, required_identifier, required_str, sdk_wrapper,
};
use crate::types::SDKHandle;
use crate::{DashSDKResult, FFIError};
use dash_sdk::dpp::prelude::Identifier;
use dash_sdk::platform::contract_fee_pots::ContractFeePots;
use dash_sdk::platform::contract_moderation::{
    ContractDocumentRemovalsPageQuery, ContractDocumentRemovalsSelection,
    ContractModerationActionCounts, ContractModerationEntries, ContractModerationEntriesPageQuery,
    ContractModerationList, ContractModerationListStatuses, ContractModerationStatusQuery,
    ContractTeamActionSigners, ContractTeamActionSignersQuery, ContractTeamActions,
    ContractTeamActionsPageQuery,
};
use dash_sdk::platform::Fetch;
use serde_json::Value as JsonValue;
use std::ffi::c_char;

/// The page bound `limit` asks for: `None` for 0 (rs-sdk's default, the page cap of the
/// network queried), refused above `u16::MAX`, which no rs-sdk page query can carry.
fn page_limit(limit: u32) -> Result<Option<u16>, FFIError> {
    if limit == 0 {
        return Ok(None);
    }
    u16::try_from(limit).map(Some).map_err(|_| {
        FFIError::InvalidParameter(format!("limit {limit} exceeds the maximum of {}", u16::MAX))
    })
}

/// The lists `lists` names, `None` for a null pointer or a count of 0 (every list the contract
/// keeps).
///
/// # Safety
/// `lists` must be null or point to `lists_count` readable bytes.
unsafe fn moderation_lists(
    lists: *const u8,
    lists_count: usize,
) -> Result<Option<Vec<ContractModerationList>>, FFIError> {
    if lists_count == 0 {
        return Ok(None);
    }
    if lists.is_null() {
        return Err(FFIError::InvalidParameter(format!(
            "lists is null but lists_count is {lists_count}"
        )));
    }
    std::slice::from_raw_parts(lists, lists_count)
        .iter()
        .map(|list| moderation_list(*list, "lists entry"))
        .collect::<Result<Vec<_>, _>>()
        .map(Some)
}

/// The base58 ids `ids` points to.
///
/// # Safety
/// `ids` must be null or point to `count` pointers, each null or a NUL-terminated string.
unsafe fn identifiers(
    ids: *const *const c_char,
    count: usize,
    field: &str,
) -> Result<Vec<Identifier>, FFIError> {
    if count == 0 {
        return Ok(Vec::new());
    }
    if ids.is_null() {
        return Err(FFIError::InvalidParameter(format!(
            "{field} is null but its count is {count}"
        )));
    }
    std::slice::from_raw_parts(ids, count)
        .iter()
        .map(|id| required_identifier(*id, field))
        .collect()
}

/// One identity's status on a moderated contract: whether it is banned, until when it is
/// suspended, and the warnings it carries, on the lists read.
///
/// # Parameters
/// * `sdk_handle` - Handle to the SDK instance.
/// * `contract_id` - The moderated contract's id, base58.
/// * `identity_id` - The identity's id, base58.
/// * `lists` - The lists to read, each a `ContractModerationListFFI` byte (0 banlist,
///   1 suspensions, 2 warnings) that must be a list the contract keeps; null (or a
///   `lists_count` of 0) reads every list the contract keeps, which fetches the contract first
///   (`ContractModerationStatusQuery::for_contract_id`).
/// * `lists_count` - The number of bytes `lists` points to.
///
/// # Returns
/// A JSON object, or no data when rs-sdk returns none:
///
/// ```json
/// {"lists": ["banlist", "suspensions", "warnings"],
///  "banned": true, "banReason": {"code": null, "text": "spam"},
///  "suspendedUntil": 1700000000000, "suspensionReason": {"code": null, "text": "cool off"},
///  "warnings": [{"warnedAt": 1600000000000, "reason": {"code": null, "text": "first"}}]}
/// ```
///
/// Only the fields of the lists read are present: `banned` when `lists` includes `banlist`,
/// `suspendedUntil` when it includes `suspensions` and the identity is suspended (a lapsed
/// suspension stays until it is swept), `warnings` (oldest first, `[]` for none) when it
/// includes `warnings`. A reason is `{"code": <u16 or null>, "text": "...",
/// "documents"?: [{"documentTypeName": "...", "documentId": "<base58>"}],
/// "reasonDocumentId"?: "<base58>"}`. Times are milliseconds. Without `lists`, a contract that
/// does not exist, is not moderated or keeps no list is an error. Free the string with
/// `dash_sdk_string_free`.
///
/// # Safety
/// - `sdk_handle` must be a valid pointer to an initialized `SDKHandle`.
/// - `contract_id` and `identity_id` must be valid pointers to NUL-terminated strings for the
///   duration of the call.
/// - `lists` must be null or point to `lists_count` readable bytes.
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_contract_moderation_get_status(
    sdk_handle: *const SDKHandle,
    contract_id: *const c_char,
    identity_id: *const c_char,
    lists: *const u8,
    lists_count: usize,
) -> DashSDKResult {
    json_result(get_status_inner(
        sdk_handle,
        contract_id,
        identity_id,
        lists,
        lists_count,
    ))
}

unsafe fn get_status_inner(
    sdk_handle: *const SDKHandle,
    contract_id: *const c_char,
    identity_id: *const c_char,
    lists: *const u8,
    lists_count: usize,
) -> Result<Option<JsonValue>, FFIError> {
    let wrapper = sdk_wrapper(sdk_handle)?;
    let contract_id = required_identifier(contract_id, "contract_id")?;
    let identity_id = required_identifier(identity_id, "identity_id")?;
    let lists = moderation_lists(lists, lists_count)?;
    let sdk = &wrapper.sdk;
    let statuses = wrapper.runtime.block_on(async {
        let query = match lists {
            Some(lists) => ContractModerationStatusQuery {
                contract_id,
                identity_id,
                lists,
            },
            None => {
                ContractModerationStatusQuery::for_contract_id(sdk, contract_id, identity_id)
                    .await?
            }
        };
        ContractModerationListStatuses::fetch(sdk, query).await
    })?;
    Ok(statuses.as_ref().map(status_json))
}

/// One page of a moderated contract's banlist, suspension list or warning list, in identity id
/// order.
///
/// # Parameters
/// * `sdk_handle` - Handle to the SDK instance.
/// * `contract_id` - The moderated contract's id, base58.
/// * `list` - The list, a `ContractModerationListFFI` byte (0 banlist, 1 suspensions,
///   2 warnings); the contract must keep it.
/// * `start_after` - The page's `nextStartAfter` (base58) to continue after it; null for the
///   first page.
/// * `limit` - The most entries to return; 0 for the page cap of the network queried. A value
///   above `u16::MAX` is refused; the node refuses one above its cap.
///
/// # Returns
/// A JSON object, or no data when rs-sdk returns none:
///
/// ```json
/// {"entries": [{"identityId": "<base58>", "until": 1700000000000,
///               "reason": {"code": null, "text": "cool off"}}],
///  "nextStartAfter": "<base58>"}
/// ```
///
/// `until` is present on a suspension list entry (when it lapses, in milliseconds); `warnings`
/// (`[{"warnedAt": <ms>, "reason": {...}}]`, oldest first) on a warning list entry, whose
/// `reason` is the latest warning's. `nextStartAfter` is rs-sdk's cursor of the next page and
/// is absent on the last page. Free the string with `dash_sdk_string_free`.
///
/// # Safety
/// - `sdk_handle` must be a valid pointer to an initialized `SDKHandle`.
/// - `contract_id` must be a valid pointer to a NUL-terminated string, and `start_after` null
///   or one, for the duration of the call.
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_contract_moderation_get_entries(
    sdk_handle: *const SDKHandle,
    contract_id: *const c_char,
    list: u8,
    start_after: *const c_char,
    limit: u32,
) -> DashSDKResult {
    json_result(get_entries_inner(
        sdk_handle,
        contract_id,
        list,
        start_after,
        limit,
    ))
}

unsafe fn get_entries_inner(
    sdk_handle: *const SDKHandle,
    contract_id: *const c_char,
    list: u8,
    start_after: *const c_char,
    limit: u32,
) -> Result<Option<JsonValue>, FFIError> {
    let wrapper = sdk_wrapper(sdk_handle)?;
    let contract_id = required_identifier(contract_id, "contract_id")?;
    let list = moderation_list(list, "list")?;
    let start_after = optional_identifier(start_after, "start_after")?;
    let limit = page_limit(limit)?;
    let mut query =
        ContractModerationEntriesPageQuery::new(contract_id, list, wrapper.sdk.version());
    query.query.start_after = start_after;
    if let Some(limit) = limit {
        query = query.with_limit(limit);
    }
    let page = wrapper.runtime.block_on(ContractModerationEntries::fetch(
        &wrapper.sdk,
        query.clone(),
    ))?;
    Ok(page.map(|page| entries_json(&page, &query)))
}

/// One page of the records of the documents a contract's moderators deleted within one
/// document type, in document id order, each with the values it keeps of its document.
///
/// # Parameters
/// * `sdk_handle` - Handle to the SDK instance.
/// * `contract_id` - The moderated contract's id, base58.
/// * `document_type_name` - The document type; the node refuses one that keeps no records (no
///   `moderatorAbilities.delete`, or `deleteKeepsRecord: false`).
/// * `start_after` - The page's `nextStartAfter` (base58) to continue after it; null for the
///   first page.
/// * `limit` - The most records to return; 0 for the page cap of the network queried. A value
///   above `u16::MAX` is refused; the node refuses one above its cap.
///
/// # Returns
/// A JSON object, or no data when rs-sdk returns none:
///
/// ```json
/// {"removals": [{"documentId": "<base58>", "documentOwnerId": "<base58>",
///                "moderatorId": "<base58>", "reason": {"code": null, "text": "off topic"},
///                "removedAt": 1700000000000, "documentHash": "<64 hex>",
///                "restoredBy": "<base58>", "restoredAt": 1700000100000,
///                "keptFields": {"hashtag": "dash", "$createdAt": 1600000000000}}],
///  "nextStartAfter": "<base58>"}
/// ```
///
/// `restoredBy` and `restoredAt` are present once a moderator restored the document.
/// `keptFields` holds the values the record keeps by property path
/// (`moderatorAbilities.deleteKeepsFields`), read by rs-sdk under the document's type (which
/// fetches the contract when a record keeps any), each as JSON shows a platform value:
/// identifiers as base58 strings, other bytes as base64 strings; a path the document held no
/// value at is absent, and the object is empty when the type keeps none. `nextStartAfter` is
/// rs-sdk's cursor and is absent on the last page. Free the string with
/// `dash_sdk_string_free`.
///
/// # Safety
/// - `sdk_handle` must be a valid pointer to an initialized `SDKHandle`.
/// - `contract_id` and `document_type_name` must be valid pointers to NUL-terminated strings,
///   and `start_after` null or one, for the duration of the call.
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_contract_moderation_get_document_removals(
    sdk_handle: *const SDKHandle,
    contract_id: *const c_char,
    document_type_name: *const c_char,
    start_after: *const c_char,
    limit: u32,
) -> DashSDKResult {
    json_result(get_document_removals_inner(
        sdk_handle,
        contract_id,
        document_type_name,
        start_after,
        limit,
    ))
}

unsafe fn get_document_removals_inner(
    sdk_handle: *const SDKHandle,
    contract_id: *const c_char,
    document_type_name: *const c_char,
    start_after: *const c_char,
    limit: u32,
) -> Result<Option<JsonValue>, FFIError> {
    let wrapper = sdk_wrapper(sdk_handle)?;
    let contract_id = required_identifier(contract_id, "contract_id")?;
    let document_type_name = required_str(document_type_name, "document_type_name")?;
    let start_after = optional_identifier(start_after, "start_after")?;
    let limit = page_limit(limit)?;
    let mut query = ContractDocumentRemovalsPageQuery::new(
        contract_id,
        document_type_name.to_string(),
        wrapper.sdk.version(),
    );
    if let Some(limit) = limit {
        query = query.with_limit(limit);
    }
    if let ContractDocumentRemovalsSelection::Page {
        start_after: cursor,
        ..
    } = &mut query.query.selection
    {
        *cursor = start_after;
    }
    let removals = wrapper
        .runtime
        .block_on(query.fetch_with_kept_values(&wrapper.sdk))?;
    removals.map(removals_json).transpose()
}

/// The records of the documents named, within one document type, that the contract's
/// moderators deleted, in document id order, each with the values it keeps of its document. A
/// document with no record is left out of the answer rather than refused.
///
/// # Parameters
/// * `sdk_handle` - Handle to the SDK instance.
/// * `contract_id` - The moderated contract's id, base58.
/// * `document_type_name` - The document type; the node refuses one that keeps no records.
/// * `document_ids` - Pointer to `document_ids_count` document ids, base58. How many the node
///   takes (at least one, at most its page cap) is the node's to judge.
/// * `document_ids_count` - The number of ids.
///
/// # Returns
/// A JSON object in the shape `dash_sdk_contract_moderation_get_document_removals` returns,
/// without `nextStartAfter` (a read by ids has no page after it), or no data when rs-sdk
/// returns none. Free the string with `dash_sdk_string_free`.
///
/// # Safety
/// - `sdk_handle` must be a valid pointer to an initialized `SDKHandle`.
/// - `contract_id` and `document_type_name` must be valid pointers to NUL-terminated strings
///   for the duration of the call.
/// - `document_ids` must be null or point to `document_ids_count` pointers, each to a
///   NUL-terminated string, for the duration of the call.
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_contract_moderation_get_document_removals_by_ids(
    sdk_handle: *const SDKHandle,
    contract_id: *const c_char,
    document_type_name: *const c_char,
    document_ids: *const *const c_char,
    document_ids_count: usize,
) -> DashSDKResult {
    json_result(get_document_removals_by_ids_inner(
        sdk_handle,
        contract_id,
        document_type_name,
        document_ids,
        document_ids_count,
    ))
}

unsafe fn get_document_removals_by_ids_inner(
    sdk_handle: *const SDKHandle,
    contract_id: *const c_char,
    document_type_name: *const c_char,
    document_ids: *const *const c_char,
    document_ids_count: usize,
) -> Result<Option<JsonValue>, FFIError> {
    let wrapper = sdk_wrapper(sdk_handle)?;
    let contract_id = required_identifier(contract_id, "contract_id")?;
    let document_type_name = required_str(document_type_name, "document_type_name")?;
    let document_ids = identifiers(document_ids, document_ids_count, "document_ids")?;
    let query = ContractDocumentRemovalsPageQuery::for_document_ids(
        contract_id,
        document_type_name.to_string(),
        document_ids,
    );
    let removals = wrapper
        .runtime
        .block_on(query.fetch_with_kept_values(&wrapper.sdk))?;
    removals.map(removals_json).transpose()
}

/// One page of the actions an elected contract's seated moderation team votes on, active (still
/// gathering approvals) or closed (their approvals met the rule and they ran), in action id
/// order.
///
/// # Parameters
/// * `sdk_handle` - Handle to the SDK instance.
/// * `contract_id` - The elected contract's id, base58.
/// * `status` - A `ContractTeamActionStatusFFI` byte: 0 active, 1 closed.
/// * `start_at_action_id` - The page's `nextStartAtActionId` (base58) to continue from it; null
///   for the first page.
/// * `start_at_action_id_included` - Whether the page includes the `start_at_action_id` action
///   itself; refused when set without `start_at_action_id`, which leaves nothing to include.
/// * `limit` - The most actions to return; 0 for the page cap of the network queried. A value
///   above `u16::MAX` is refused; the node refuses one above its cap.
///
/// # Returns
/// A JSON object, or no data when rs-sdk returns none:
///
/// ```json
/// {"actions": [{"actionId": "<base58>", "proposerId": "<base58>",
///               "proposedAt": 1700000000000,
///               "event": {"type": "deleteSettledDocument", "documentTypeName": "post",
///                         "documentId": "<base58>", "documentLastModifiedAt": 1690000000000,
///                         "documentRevision": 3, "reason": {"code": null, "text": "spam"}},
///               "approvalCount": 2}],
///  "nextStartAtActionId": "<base58>"}
/// ```
///
/// `documentRevision` is absent on a type whose documents carry none. An active action's
/// `approvalCount` is an upper bound (see `dash_sdk_contract_moderation_get_team_action_signers`
/// for who approved it). `nextStartAtActionId` is rs-sdk's cursor (to pass back with
/// `start_at_action_id_included` false) and is absent on the last page. Free the string with
/// `dash_sdk_string_free`.
///
/// # Safety
/// - `sdk_handle` must be a valid pointer to an initialized `SDKHandle`.
/// - `contract_id` must be a valid pointer to a NUL-terminated string, and
///   `start_at_action_id` null or one, for the duration of the call.
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_contract_moderation_get_team_actions(
    sdk_handle: *const SDKHandle,
    contract_id: *const c_char,
    status: u8,
    start_at_action_id: *const c_char,
    start_at_action_id_included: bool,
    limit: u32,
) -> DashSDKResult {
    json_result(get_team_actions_inner(
        sdk_handle,
        contract_id,
        status,
        start_at_action_id,
        start_at_action_id_included,
        limit,
    ))
}

unsafe fn get_team_actions_inner(
    sdk_handle: *const SDKHandle,
    contract_id: *const c_char,
    status: u8,
    start_at_action_id: *const c_char,
    start_at_action_id_included: bool,
    limit: u32,
) -> Result<Option<JsonValue>, FFIError> {
    let wrapper = sdk_wrapper(sdk_handle)?;
    let contract_id = required_identifier(contract_id, "contract_id")?;
    let status = team_action_status(status)?;
    let start_at_action_id = optional_identifier(start_at_action_id, "start_at_action_id")?;
    let limit = page_limit(limit)?;
    let mut query = ContractTeamActionsPageQuery::new(contract_id, status, wrapper.sdk.version());
    match start_at_action_id {
        Some(action_id) => {
            query = query.starting_at(action_id, start_at_action_id_included);
        }
        None if start_at_action_id_included => {
            return Err(FFIError::InvalidParameter(
                "start_at_action_id_included is set without start_at_action_id".to_string(),
            ));
        }
        None => {}
    }
    if let Some(limit) = limit {
        query = query.with_limit(limit);
    }
    let page = wrapper
        .runtime
        .block_on(ContractTeamActions::fetch(&wrapper.sdk, query.clone()))?;
    Ok(page.map(|page| team_actions_json(&page, &query)))
}

/// Who approved one of the actions an elected contract's seated moderation team votes on, the
/// proposer among them unless it left the team and its approval was dropped, in identity id
/// order. The action is looked up among the actions of `status` alone: one that closed is no
/// longer among the active ones.
///
/// # Parameters
/// * `sdk_handle` - Handle to the SDK instance.
/// * `contract_id` - The elected contract's id, base58.
/// * `status` - A `ContractTeamActionStatusFFI` byte: 0 active, 1 closed.
/// * `action_id` - The action's id, base58.
///
/// # Returns
/// `{"signerIds": ["<base58>", ...]}`, with no id when the contract holds no action of that id
/// with the status asked, or no data when rs-sdk returns none. Free the string with
/// `dash_sdk_string_free`.
///
/// # Safety
/// - `sdk_handle` must be a valid pointer to an initialized `SDKHandle`.
/// - `contract_id` and `action_id` must be valid pointers to NUL-terminated strings for the
///   duration of the call.
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_contract_moderation_get_team_action_signers(
    sdk_handle: *const SDKHandle,
    contract_id: *const c_char,
    status: u8,
    action_id: *const c_char,
) -> DashSDKResult {
    json_result(get_team_action_signers_inner(
        sdk_handle,
        contract_id,
        status,
        action_id,
    ))
}

unsafe fn get_team_action_signers_inner(
    sdk_handle: *const SDKHandle,
    contract_id: *const c_char,
    status: u8,
    action_id: *const c_char,
) -> Result<Option<JsonValue>, FFIError> {
    let wrapper = sdk_wrapper(sdk_handle)?;
    let contract_id = required_identifier(contract_id, "contract_id")?;
    let status = team_action_status(status)?;
    let action_id = required_identifier(action_id, "action_id")?;
    let query = ContractTeamActionSignersQuery {
        contract_id,
        status,
        action_id,
    };
    let signers = wrapper
        .runtime
        .block_on(ContractTeamActionSigners::fetch(&wrapper.sdk, query))?;
    Ok(signers.as_ref().map(team_action_signers_json))
}

/// How many counted moderation actions (a ban, a suspension, a warning or a document deletion)
/// each member of an elected contract's seated team signed since the moderators pot was last
/// paid out, which resets every count, in identity id order. A member that did not act since
/// has no count. Only an elected contract keeps counts; the node refuses any other.
///
/// # Parameters
/// * `sdk_handle` - Handle to the SDK instance.
/// * `contract_id` - The elected contract's id, base58.
///
/// # Returns
/// `{"counts": [{"identityId": "<base58>", "count": 3}]}`, or no data when rs-sdk returns none.
/// Free the string with `dash_sdk_string_free`.
///
/// # Safety
/// - `sdk_handle` must be a valid pointer to an initialized `SDKHandle`.
/// - `contract_id` must be a valid pointer to a NUL-terminated string for the duration of the
///   call.
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_contract_moderation_get_action_counts(
    sdk_handle: *const SDKHandle,
    contract_id: *const c_char,
) -> DashSDKResult {
    json_result(get_action_counts_inner(sdk_handle, contract_id))
}

unsafe fn get_action_counts_inner(
    sdk_handle: *const SDKHandle,
    contract_id: *const c_char,
) -> Result<Option<JsonValue>, FFIError> {
    let wrapper = sdk_wrapper(sdk_handle)?;
    let contract_id = required_identifier(contract_id, "contract_id")?;
    let counts = wrapper
        .runtime
        .block_on(ContractModerationActionCounts::fetch(
            &wrapper.sdk,
            contract_id,
        ))?;
    Ok(counts.as_ref().map(action_counts_json))
}

/// What the document action fees of a contract have collected for its owner and for its
/// moderation team, and the last claim of each pot. A contract that charges no fees, or whose
/// fees nobody has paid yet, reads as two empty pots; a contract the network does not hold is
/// an error on the node.
///
/// # Parameters
/// * `sdk_handle` - Handle to the SDK instance.
/// * `contract_id` - The contract's id, base58.
///
/// # Returns
/// A JSON object, or no data when rs-sdk returns none:
///
/// ```json
/// {"owner": {"credits": 10000000},
///  "moderators": {"credits": 500000, "lastClaimEpoch": 12,
///                 "lastClaimTimeMs": 1700000000000, "lastClaimantId": "<base58>"}}
/// ```
///
/// `credits` is what a claim would pay out. `lastClaimEpoch`, `lastClaimTimeMs` and
/// `lastClaimantId` are present together, once the pot was ever paid out (epoch 0 included); a
/// pot is paid out at most once per epoch. Free the string with `dash_sdk_string_free`.
///
/// # Safety
/// - `sdk_handle` must be a valid pointer to an initialized `SDKHandle`.
/// - `contract_id` must be a valid pointer to a NUL-terminated string for the duration of the
///   call.
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_contract_get_fee_pots(
    sdk_handle: *const SDKHandle,
    contract_id: *const c_char,
) -> DashSDKResult {
    json_result(get_fee_pots_inner(sdk_handle, contract_id))
}

unsafe fn get_fee_pots_inner(
    sdk_handle: *const SDKHandle,
    contract_id: *const c_char,
) -> Result<Option<JsonValue>, FFIError> {
    let wrapper = sdk_wrapper(sdk_handle)?;
    let contract_id = required_identifier(contract_id, "contract_id")?;
    let pots = wrapper
        .runtime
        .block_on(ContractFeePots::fetch(&wrapper.sdk, contract_id))?;
    Ok(pots.as_ref().map(fee_pots_json))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::test_utils::{create_mock_sdk_handle, destroy_mock_sdk_handle};
    use crate::{dash_sdk_error_free, DashSDKErrorCode};
    use dash_sdk::dpp::platform_value::string_encoding::Encoding;
    use std::ffi::CString;

    /// `result` failed with `InvalidParameter`; its error is freed.
    unsafe fn assert_invalid_parameter(result: DashSDKResult) {
        assert!(result.data.is_null());
        assert!(!result.error.is_null());
        let code = (*result.error).code;
        dash_sdk_error_free(result.error);
        assert_eq!(code, DashSDKErrorCode::InvalidParameter);
    }

    fn base58(seed: u8) -> CString {
        CString::new(Identifier::new([seed; 32]).to_string(Encoding::Base58)).expect("no NUL")
    }

    /// Every read refuses a null SDK handle and a null or non-base58 id before anything reaches
    /// the network: the mock SDK holds no expectation, so a request would fail differently.
    #[test]
    fn should_refuse_a_null_handle_or_a_missing_or_malformed_id_before_any_request() {
        let handle = create_mock_sdk_handle();
        let id = base58(4);
        let not_base58 = CString::new("not base58: 0OIl").expect("no NUL");
        let post = CString::new("post").expect("no NUL");
        let ids = [id.as_ptr()];
        unsafe {
            for (sdk, bad_id) in [
                (std::ptr::null(), id.as_ptr()),
                (handle as *const SDKHandle, std::ptr::null()),
                (handle as *const SDKHandle, not_base58.as_ptr()),
            ] {
                assert_invalid_parameter(dash_sdk_contract_moderation_get_status(
                    sdk,
                    bad_id,
                    id.as_ptr(),
                    std::ptr::null(),
                    0,
                ));
                assert_invalid_parameter(dash_sdk_contract_moderation_get_status(
                    sdk,
                    id.as_ptr(),
                    bad_id,
                    std::ptr::null(),
                    0,
                ));
                assert_invalid_parameter(dash_sdk_contract_moderation_get_entries(
                    sdk,
                    bad_id,
                    0,
                    std::ptr::null(),
                    0,
                ));
                assert_invalid_parameter(dash_sdk_contract_moderation_get_document_removals(
                    sdk,
                    bad_id,
                    post.as_ptr(),
                    std::ptr::null(),
                    0,
                ));
                assert_invalid_parameter(
                    dash_sdk_contract_moderation_get_document_removals_by_ids(
                        sdk,
                        bad_id,
                        post.as_ptr(),
                        ids.as_ptr(),
                        ids.len(),
                    ),
                );
                assert_invalid_parameter(dash_sdk_contract_moderation_get_team_actions(
                    sdk,
                    bad_id,
                    0,
                    std::ptr::null(),
                    false,
                    0,
                ));
                assert_invalid_parameter(dash_sdk_contract_moderation_get_team_action_signers(
                    sdk,
                    bad_id,
                    0,
                    id.as_ptr(),
                ));
                assert_invalid_parameter(dash_sdk_contract_moderation_get_team_action_signers(
                    sdk,
                    id.as_ptr(),
                    0,
                    bad_id,
                ));
                assert_invalid_parameter(dash_sdk_contract_moderation_get_action_counts(
                    sdk, bad_id,
                ));
                assert_invalid_parameter(dash_sdk_contract_get_fee_pots(sdk, bad_id));
            }

            // A malformed cursor or listed id
            assert_invalid_parameter(dash_sdk_contract_moderation_get_entries(
                handle,
                id.as_ptr(),
                0,
                not_base58.as_ptr(),
                0,
            ));
            assert_invalid_parameter(dash_sdk_contract_moderation_get_document_removals(
                handle,
                id.as_ptr(),
                post.as_ptr(),
                not_base58.as_ptr(),
                0,
            ));
            assert_invalid_parameter(dash_sdk_contract_moderation_get_team_actions(
                handle,
                id.as_ptr(),
                0,
                not_base58.as_ptr(),
                false,
                0,
            ));
            let bad_ids = [id.as_ptr(), not_base58.as_ptr()];
            assert_invalid_parameter(dash_sdk_contract_moderation_get_document_removals_by_ids(
                handle,
                id.as_ptr(),
                post.as_ptr(),
                bad_ids.as_ptr(),
                bad_ids.len(),
            ));
            let null_id = [std::ptr::null()];
            assert_invalid_parameter(dash_sdk_contract_moderation_get_document_removals_by_ids(
                handle,
                id.as_ptr(),
                post.as_ptr(),
                null_id.as_ptr(),
                null_id.len(),
            ));
            assert_invalid_parameter(dash_sdk_contract_moderation_get_document_removals_by_ids(
                handle,
                id.as_ptr(),
                post.as_ptr(),
                std::ptr::null(),
                1,
            ));

            // A null document type name
            assert_invalid_parameter(dash_sdk_contract_moderation_get_document_removals(
                handle,
                id.as_ptr(),
                std::ptr::null(),
                std::ptr::null(),
                0,
            ));
            assert_invalid_parameter(dash_sdk_contract_moderation_get_document_removals_by_ids(
                handle,
                id.as_ptr(),
                std::ptr::null(),
                ids.as_ptr(),
                ids.len(),
            ));
            destroy_mock_sdk_handle(handle);
        }
    }

    /// A byte that names no list or status, a list pointer missing its bytes, a limit no rs-sdk
    /// page carries and an inclusion flag without its action are refused before any request.
    #[test]
    fn should_refuse_arguments_rs_sdk_cannot_represent_before_any_request() {
        let handle = create_mock_sdk_handle();
        let id = base58(4);
        let post = CString::new("post").expect("no NUL");
        let over_u16 = u32::from(u16::MAX) + 1;
        unsafe {
            let unknown_list = [0u8, 3];
            assert_invalid_parameter(dash_sdk_contract_moderation_get_status(
                handle,
                id.as_ptr(),
                id.as_ptr(),
                unknown_list.as_ptr(),
                unknown_list.len(),
            ));
            assert_invalid_parameter(dash_sdk_contract_moderation_get_status(
                handle,
                id.as_ptr(),
                id.as_ptr(),
                std::ptr::null(),
                2,
            ));
            assert_invalid_parameter(dash_sdk_contract_moderation_get_entries(
                handle,
                id.as_ptr(),
                3,
                std::ptr::null(),
                0,
            ));
            assert_invalid_parameter(dash_sdk_contract_moderation_get_entries(
                handle,
                id.as_ptr(),
                0,
                std::ptr::null(),
                over_u16,
            ));
            assert_invalid_parameter(dash_sdk_contract_moderation_get_document_removals(
                handle,
                id.as_ptr(),
                post.as_ptr(),
                std::ptr::null(),
                over_u16,
            ));
            for status in [2, u8::MAX] {
                assert_invalid_parameter(dash_sdk_contract_moderation_get_team_actions(
                    handle,
                    id.as_ptr(),
                    status,
                    std::ptr::null(),
                    false,
                    0,
                ));
                assert_invalid_parameter(dash_sdk_contract_moderation_get_team_action_signers(
                    handle,
                    id.as_ptr(),
                    status,
                    id.as_ptr(),
                ));
            }
            assert_invalid_parameter(dash_sdk_contract_moderation_get_team_actions(
                handle,
                id.as_ptr(),
                0,
                std::ptr::null(),
                false,
                over_u16,
            ));
            assert_invalid_parameter(dash_sdk_contract_moderation_get_team_actions(
                handle,
                id.as_ptr(),
                0,
                std::ptr::null(),
                true,
                0,
            ));
            destroy_mock_sdk_handle(handle);
        }
    }

    #[test]
    fn should_read_a_limit_of_zero_as_the_default_and_refuse_one_above_u16() {
        assert_eq!(page_limit(0).expect("default"), None);
        assert_eq!(page_limit(1).expect("one"), Some(1));
        assert_eq!(
            page_limit(u32::from(u16::MAX)).expect("the largest"),
            Some(u16::MAX)
        );
        assert!(matches!(
            page_limit(u32::from(u16::MAX) + 1),
            Err(FFIError::InvalidParameter(_))
        ));
    }

    #[test]
    fn should_read_no_lists_as_every_list_the_contract_keeps() {
        let lists = [2u8, 0];
        unsafe {
            assert_eq!(moderation_lists(std::ptr::null(), 0).expect("none"), None);
            assert_eq!(moderation_lists(lists.as_ptr(), 0).expect("none"), None);
            assert_eq!(
                moderation_lists(lists.as_ptr(), lists.len()).expect("two"),
                Some(vec![
                    ContractModerationList::Warnings,
                    ContractModerationList::Banlist
                ])
            );
        }
    }
}
