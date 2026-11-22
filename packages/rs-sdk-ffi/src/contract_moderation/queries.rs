//! Contract moderation and fee pot reads. Each reads its C arguments, calls one rs-sdk entry
//! point and returns JSON written by [`super::json`], or no data when rs-sdk returns none.
//! Each drives its read with `BigStackRuntime::try_block_on`, so a worker thread the OS will
//! not create comes back as an error rather than aborting the host app.
//!
//! Arguments are refused before any request only when they cannot be represented in the rs-sdk
//! query: a null handle, a null or non-base58 id, a byte that names no list or status, a limit
//! above `u16::MAX`, an inclusion flag without the action it includes, a null array with a
//! count. Everything else (a limit of 0 or above the page cap, an empty id or list selection, a
//! list the contract does not keep) is the node's to refuse. Below protocol version 14 every
//! function here is refused by the SDK's protocol gate before its request is sent; see the
//! module documentation for how to tell why.

use super::json::{
    action_counts_json, entries_json, fee_pots_json, removals_json, status_json,
    team_action_signers_json, team_actions_json, RemovalsPageJson,
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
    ContractModerationList, ContractTeamActionSigners, ContractTeamActionSignersQuery,
    ContractTeamActions, ContractTeamActionsPageQuery,
};
use dash_sdk::platform::Fetch;
use serde_json::Value as JsonValue;
use std::ffi::c_char;

/// The page bound `limit` points to: `None` for a null pointer (rs-sdk's default, the page cap
/// of the network queried), the value as it is otherwise (0 included, which the node judges),
/// refused above `u16::MAX`, which no rs-sdk page query can carry.
///
/// # Safety
/// `limit` must be null or point to a readable `u32`.
unsafe fn page_limit(limit: *const u32) -> Result<Option<u16>, FFIError> {
    if limit.is_null() {
        return Ok(None);
    }
    let limit = *limit;
    u16::try_from(limit).map(Some).map_err(|_| {
        FFIError::InvalidParameter(format!("limit {limit} exceeds the maximum of {}", u16::MAX))
    })
}

/// The lists `lists` names: `None` for a null pointer with a count of 0 (no selection), the
/// lists as given for a non-null pointer, an empty selection for a count of 0 included;
/// a null pointer with a count above 0 is refused.
///
/// # Safety
/// `lists` must be null or point to `lists_count` readable bytes.
unsafe fn moderation_lists(
    lists: *const u8,
    lists_count: usize,
) -> Result<Option<Vec<ContractModerationList>>, FFIError> {
    if lists.is_null() {
        if lists_count == 0 {
            return Ok(None);
        }
        return Err(FFIError::InvalidParameter(format!(
            "lists is null but lists_count is {lists_count}"
        )));
    }
    if lists_count == 0 {
        return Ok(Some(Vec::new()));
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
///   1 suspensions, 2 warnings) that must be a list the contract keeps. The selection is
///   resolved by `ContractModerationStatusQuery::resolve`, inside
///   `Sdk::fetch_contract_moderation_status`, which this calls:
///   - null with a `lists_count` of 0: no selection; every list the contract keeps is read,
///     which fetches the contract first (refused by the SDK's protocol gate, before the
///     contract is fetched, below the status read's protocol version 14);
///   - non-null with a `lists_count` of 0: an explicit empty selection, sent as it is for the
///     node to judge;
///   - non-null with a `lists_count` above 0: the lists named, sent as they are;
///   - null with a `lists_count` above 0: refused with `InvalidParameter`.
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
    let statuses =
        wrapper
            .runtime
            .try_block_on(wrapper.sdk.fetch_contract_moderation_status(
                contract_id,
                identity_id,
                lists,
            ))??;
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
/// * `limit` - Null for the page cap of the network queried, or a pointer to the most entries
///   to return, passed as it is (0 included, which the node judges). A value above
///   `u16::MAX` is refused; the node refuses one above its cap.
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
/// - `limit` must be null or point to a readable `u32`.
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_contract_moderation_get_entries(
    sdk_handle: *const SDKHandle,
    contract_id: *const c_char,
    list: u8,
    start_after: *const c_char,
    limit: *const u32,
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
    limit: *const u32,
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
    let page = wrapper
        .runtime
        .try_block_on(ContractModerationEntries::fetch(
            &wrapper.sdk,
            query.clone(),
        ))??;
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
/// * `limit` - Null for the page cap of the network queried, or a pointer to the most records
///   to return, passed as it is (0 included, which the node judges). A value above
///   `u16::MAX` is refused; the node refuses one above its cap.
///
/// # Returns
/// A JSON object, or no data when rs-sdk returns none:
///
/// ```json
/// {"removals": [{"documentId": "<base58>", "documentOwnerId": "<base58>",
///                "moderatorId": "<base58>", "reason": {"code": null, "text": "off topic"},
///                "removedAt": 1700000000000, "documentHash": "<64 hex>",
///                "restoredBy": "<base58>", "restoredAt": 1700000100000,
///                "keptFields": {"hashtag": {"type": "string", "value": "dash"},
///                               "$createdAt": {"type": "uint", "value": 1600000000000}}}],
///  "nextStartAfter": "<base58>"}
/// ```
///
/// `restoredBy` and `restoredAt` are present once a moderator restored the document.
/// `keptFields` holds the values the record keeps by property path
/// (`moderatorAbilities.deleteKeepsFields`), read by rs-sdk under the document's type (which
/// fetches the contract when a record keeps any), each tagged with its kind as
/// `{"type": ..., "value": ...}` so no kind is inferred from a JSON number:
/// * `{"type": "null"}`; `bool`; `string`;
/// * `int` (a signed integer up to 64 bits) and `uint` (an unsigned one), JSON integers;
///   `int128` and `uint128` for a 128-bit value outside that range, as a decimal string;
/// * `double`, a JSON number written with its fraction and sign (`1.0`, `-0.0`);
/// * `identifier` (base58 string) and `bytes` (base64 string);
/// * `array` of tagged values, and `object` of tagged values by member name.
///
/// A path the document held no value at is absent, and the object is empty when the type
/// keeps none. The values are read under the document type as the contract holds it now, as
/// Drive reads them: a record stores a kept object's members by position, not by name, so on a
/// contract whose protocol 14 update reordered a kept object's members (refused from protocol
/// 15), a record written before that update reads under the new order's names.
/// `nextStartAfter` is
/// rs-sdk's cursor and is absent on the last page. Free the string with
/// `dash_sdk_string_free`.
///
/// # Safety
/// - `sdk_handle` must be a valid pointer to an initialized `SDKHandle`.
/// - `contract_id` and `document_type_name` must be valid pointers to NUL-terminated strings,
///   and `start_after` null or one, for the duration of the call.
/// - `limit` must be null or point to a readable `u32`.
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_contract_moderation_get_document_removals(
    sdk_handle: *const SDKHandle,
    contract_id: *const c_char,
    document_type_name: *const c_char,
    start_after: *const c_char,
    limit: *const u32,
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
    limit: *const u32,
) -> Result<Option<RemovalsPageJson>, FFIError> {
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
        .try_block_on(query.fetch_with_kept_values(&wrapper.sdk))??;
    Ok(removals.map(removals_json))
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
) -> Result<Option<RemovalsPageJson>, FFIError> {
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
        .try_block_on(query.fetch_with_kept_values(&wrapper.sdk))??;
    Ok(removals.map(removals_json))
}

/// One page of the actions an elected contract's seated moderation team votes on, active (still
/// gathering approvals) or closed (their approvals met the rule and they ran), in action id
/// order.
///
/// The node refuses the query unless at least one of the contract's document types sets
/// `moderatorAbilities.deleteSettled` (which only an elected contract may set): such a refusal
/// is an error, while a contract that keeps team actions but has none answers an empty page.
///
/// # Parameters
/// * `sdk_handle` - Handle to the SDK instance.
/// * `contract_id` - The elected contract's id, base58.
/// * `status` - A `ContractTeamActionStatusFFI` byte: 0 active, 1 closed.
/// * `start_at_action_id` - The previous page's `nextStartAtActionId` (base58) to continue from
///   it; null for the first page.
/// * `start_at_action_id_included` - The previous page's `nextStartAtActionIdIncluded`: whether
///   the page includes the `start_at_action_id` action itself; refused when set without
///   `start_at_action_id`, which leaves nothing to include.
/// * `limit` - Null for the page cap of the network queried, or a pointer to the most actions
///   to return, passed as it is (0 included, which the node judges). A value above
///   `u16::MAX` is refused; the node refuses one above its cap.
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
///  "nextStartAtActionId": "<base58>", "nextStartAtActionIdIncluded": false}
/// ```
///
/// `documentRevision` is absent on a type whose documents carry none. An active action's
/// `approvalCount` is an upper bound (see `dash_sdk_contract_moderation_get_team_action_signers`
/// for who approved it). `nextStartAtActionId` and `nextStartAtActionIdIncluded` are rs-sdk's
/// cursor of the next page, to pass back as `start_at_action_id` and
/// `start_at_action_id_included` as they are; both are absent on the last page. Free the string
/// with `dash_sdk_string_free`.
///
/// # Safety
/// - `sdk_handle` must be a valid pointer to an initialized `SDKHandle`.
/// - `contract_id` must be a valid pointer to a NUL-terminated string, and
///   `start_at_action_id` null or one, for the duration of the call.
/// - `limit` must be null or point to a readable `u32`.
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_contract_moderation_get_team_actions(
    sdk_handle: *const SDKHandle,
    contract_id: *const c_char,
    status: u8,
    start_at_action_id: *const c_char,
    start_at_action_id_included: bool,
    limit: *const u32,
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
    limit: *const u32,
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
        .try_block_on(ContractTeamActions::fetch(&wrapper.sdk, query.clone()))??;
    Ok(page.map(|page| team_actions_json(&page, &query)))
}

/// Who approved one of the actions an elected contract's seated moderation team votes on, the
/// proposer among them unless it left the team and its approval was dropped, in identity id
/// order. The action is looked up among the actions of `status` alone: one that closed is no
/// longer among the active ones.
///
/// The node refuses the query unless at least one of the contract's document types sets
/// `moderatorAbilities.deleteSettled` (which only an elected contract may set).
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
        .try_block_on(ContractTeamActionSigners::fetch(&wrapper.sdk, query))??;
    Ok(signers.as_ref().map(team_action_signers_json))
}

/// How many counted moderation actions (a ban, a suspension, a warning or a document deletion)
/// each member of an elected contract's seated team signed since the moderators pot was last
/// claimed or settled, in identity id order. A claim resets every count, and so does a
/// moderators pot settlement (a membership change of the team), even one that pays nothing
/// out. A member that did not act since has no count. Only an elected contract keeps counts;
/// the node refuses any other.
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
        .try_block_on(ContractModerationActionCounts::fetch(
            &wrapper.sdk,
            contract_id,
        ))??;
    Ok(counts.as_ref().map(action_counts_json))
}

/// What the document action fees of a contract have collected for its owner and for its
/// moderation team, and the last explicit claim of each pot. A contract that charges no fees, or whose
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
/// `credits` is the pot's balance; it does not promise what a claim will distribute.
/// `lastClaimEpoch`, `lastClaimTimeMs` and `lastClaimantId` describe the last explicit claim
/// and are present together once the pot was ever claimed (epoch 0 included); a pot is claimed
/// at most once per epoch. They are not the last payout: a membership change of an elected
/// team can also pay out the moderators pot (a moderators pot settlement), which takes credits
/// from it and resets the action counts without touching these fields. Free the string with
/// `dash_sdk_string_free`.
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
        .try_block_on(ContractFeePots::fetch(&wrapper.sdk, contract_id))??;
    Ok(pots.as_ref().map(fee_pots_json))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sdk::SDKWrapper;
    use crate::test_utils::test_utils::{create_mock_sdk_handle, destroy_mock_sdk_handle};
    use crate::{
        dash_sdk_error_free, dash_sdk_string_free, DashSDKErrorCode, DashSDKResultDataType,
    };
    use dash_sdk::dapi_grpc::platform::v0::get_contract_moderation_entries_request::{
        self, GetContractModerationEntriesRequestV0,
    };
    use dash_sdk::dapi_grpc::platform::v0::{
        ContractModerationList as ContractModerationListProto, GetContractModerationEntriesRequest,
    };
    use dash_sdk::dpp::data_contract::config::moderation::ContractModerationReason;
    use dash_sdk::dpp::platform_value::string_encoding::Encoding;
    use dash_sdk::platform::contract_moderation::{
        ContractDocumentRemoval, ContractDocumentRemovalEntry, ContractDocumentRemovals,
        ContractModerationListStatuses, ContractModerationStatusQuery, ContractTeamAction,
        ContractTeamActionEntry, ContractTeamActionEvent, GroupActionStatus,
    };
    use std::ffi::{CStr, CString};

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
                    std::ptr::null(),
                ));
                assert_invalid_parameter(dash_sdk_contract_moderation_get_document_removals(
                    sdk,
                    bad_id,
                    post.as_ptr(),
                    std::ptr::null(),
                    std::ptr::null(),
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
                    std::ptr::null(),
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
                std::ptr::null(),
            ));
            assert_invalid_parameter(dash_sdk_contract_moderation_get_document_removals(
                handle,
                id.as_ptr(),
                post.as_ptr(),
                not_base58.as_ptr(),
                std::ptr::null(),
            ));
            assert_invalid_parameter(dash_sdk_contract_moderation_get_team_actions(
                handle,
                id.as_ptr(),
                0,
                not_base58.as_ptr(),
                false,
                std::ptr::null(),
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
                std::ptr::null(),
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
                std::ptr::null(),
            ));
            assert_invalid_parameter(dash_sdk_contract_moderation_get_entries(
                handle,
                id.as_ptr(),
                0,
                std::ptr::null(),
                &over_u16,
            ));
            assert_invalid_parameter(dash_sdk_contract_moderation_get_document_removals(
                handle,
                id.as_ptr(),
                post.as_ptr(),
                std::ptr::null(),
                &over_u16,
            ));
            for status in [2, u8::MAX] {
                assert_invalid_parameter(dash_sdk_contract_moderation_get_team_actions(
                    handle,
                    id.as_ptr(),
                    status,
                    std::ptr::null(),
                    false,
                    std::ptr::null(),
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
                &over_u16,
            ));
            assert_invalid_parameter(dash_sdk_contract_moderation_get_team_actions(
                handle,
                id.as_ptr(),
                0,
                std::ptr::null(),
                true,
                std::ptr::null(),
            ));
            destroy_mock_sdk_handle(handle);
        }
    }

    #[test]
    fn should_read_no_limit_as_the_default_and_pass_any_value_up_to_u16() {
        let read = |limit: u32| unsafe { page_limit(&limit) };
        assert_eq!(
            unsafe { page_limit(std::ptr::null()) }.expect("default"),
            None
        );
        assert_eq!(read(0).expect("zero, for the node"), Some(0));
        assert_eq!(read(1).expect("one"), Some(1));
        assert_eq!(
            read(u32::from(u16::MAX)).expect("the largest"),
            Some(u16::MAX)
        );
        assert!(matches!(
            read(u32::from(u16::MAX) + 1),
            Err(FFIError::InvalidParameter(_))
        ));
    }

    #[test]
    fn should_read_a_null_list_pointer_as_no_selection_and_an_empty_one_as_explicit() {
        let lists = [2u8, 0];
        unsafe {
            assert_eq!(moderation_lists(std::ptr::null(), 0).expect("none"), None);
            assert_eq!(
                moderation_lists(lists.as_ptr(), 0).expect("an empty selection"),
                Some(vec![])
            );
            assert_eq!(
                moderation_lists(lists.as_ptr(), lists.len()).expect("two"),
                Some(vec![
                    ContractModerationList::Warnings,
                    ContractModerationList::Banlist
                ])
            );
            assert!(matches!(
                moderation_lists(std::ptr::null(), 1),
                Err(FFIError::InvalidParameter(_))
            ));
        }
    }

    /// A mock SDK handle on which `register` sets the answers, and the SDK's version for the
    /// queries the answers are registered under.
    fn mock_handle_answering(
        register: impl FnOnce(
            &mut SDKWrapper,
        ) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + '_>>,
    ) -> *mut SDKHandle {
        let mut wrapper = Box::new(SDKWrapper::new_mock());
        // Registering expectations is async, and the FFI entry point spins up its own runtime
        // and blocks on it, so this runtime must be gone by the time it runs.
        let setup_runtime = tokio::runtime::Runtime::new().expect("create setup runtime");
        setup_runtime.block_on(register(&mut wrapper));
        drop(setup_runtime);
        Box::into_raw(wrapper) as *mut SDKHandle
    }

    /// The JSON a successful call returned; the string is freed with `dash_sdk_string_free`.
    unsafe fn json_of(result: DashSDKResult) -> JsonValue {
        assert!(result.error.is_null(), "the call must succeed");
        assert_eq!(result.data_type, DashSDKResultDataType::String);
        let json: JsonValue = serde_json::from_str(
            CStr::from_ptr(result.data as *const c_char)
                .to_str()
                .expect("UTF-8"),
        )
        .expect("JSON");
        dash_sdk_string_free(result.data as *mut c_char);
        json
    }

    fn b58(seed: u8) -> String {
        Identifier::new([seed; 32]).to_string(Encoding::Base58)
    }

    /// A page of removal records read after a cursor with a limit, whose records keep no
    /// fields, comes back with rs-sdk's cursor and without a contract request (the mock holds
    /// no contract).
    #[test]
    fn should_return_a_removals_page_after_a_cursor_with_its_next_cursor() {
        let contract_id = Identifier::new([1; 32]);
        let entry = ContractDocumentRemovalEntry {
            document_id: Identifier::new([3; 32]),
            removal: ContractDocumentRemoval {
                document_owner_id: Identifier::new([20; 32]),
                moderator_id: Identifier::new([21; 32]),
                reason: ContractModerationReason::from_text("off topic"),
                removed_at: 1_700_000_000_000,
                document_hash: [0xab; 32],
                restoration: None,
                kept_fields: vec![],
            },
        };
        let handle = mock_handle_answering(|wrapper| {
            Box::pin(async move {
                let mut query = ContractDocumentRemovalsPageQuery::new(
                    contract_id,
                    "post".to_string(),
                    wrapper.sdk.version(),
                )
                .with_limit(1);
                query.query.selection = ContractDocumentRemovalsSelection::Page {
                    start_after: Some(Identifier::new([2; 32])),
                    limit: 1,
                };
                wrapper
                    .sdk
                    .mock()
                    .expect_fetch(query, Some(ContractDocumentRemovals(vec![entry])))
                    .await
                    .expect("register the removals");
            })
        });
        let contract = CString::new(b58(1)).expect("no NUL");
        let post = CString::new("post").expect("no NUL");
        let cursor = CString::new(b58(2)).expect("no NUL");
        let limit = 1u32;
        unsafe {
            let json = json_of(dash_sdk_contract_moderation_get_document_removals(
                handle,
                contract.as_ptr(),
                post.as_ptr(),
                cursor.as_ptr(),
                &limit,
            ));
            assert_eq!(json["removals"][0]["documentId"], b58(3));
            assert_eq!(json["removals"][0]["keptFields"], serde_json::json!({}));
            assert_eq!(json["nextStartAfter"], b58(3));
            destroy_mock_sdk_handle(handle);
        }
    }

    fn team_action_entry(seed: u8) -> ContractTeamActionEntry {
        ContractTeamActionEntry {
            action_id: Identifier::new([seed; 32]),
            action: ContractTeamAction {
                proposer_id: Identifier::new([30; 32]),
                proposed_at: 1_000,
                event: ContractTeamActionEvent::DeleteSettledDocument {
                    document_type_name: "post".to_string(),
                    document_id: Identifier::new([31; 32]),
                    document_last_modified_at: 10,
                    document_revision: Some(1),
                    reason: ContractModerationReason::from_text("spam"),
                },
            },
            approval_count: 2,
        }
    }

    /// A page of team actions started at an action, the action included, comes back with
    /// rs-sdk's cursor of the next page and its include flag.
    #[test]
    fn should_return_a_team_actions_page_from_a_cursor_with_its_next_cursor() {
        let contract_id = Identifier::new([1; 32]);
        let handle = mock_handle_answering(|wrapper| {
            Box::pin(async move {
                let query = ContractTeamActionsPageQuery::new(
                    contract_id,
                    GroupActionStatus::ActionClosed,
                    wrapper.sdk.version(),
                )
                .starting_at(Identifier::new([3; 32]), true)
                .with_limit(2);
                wrapper
                    .sdk
                    .mock()
                    .expect_fetch(
                        query,
                        Some(ContractTeamActions(vec![
                            team_action_entry(3),
                            team_action_entry(4),
                        ])),
                    )
                    .await
                    .expect("register the team actions");
            })
        });
        let contract = CString::new(b58(1)).expect("no NUL");
        let cursor = CString::new(b58(3)).expect("no NUL");
        let limit = 2u32;
        unsafe {
            let json = json_of(dash_sdk_contract_moderation_get_team_actions(
                handle,
                contract.as_ptr(),
                1,
                cursor.as_ptr(),
                true,
                &limit,
            ));
            assert_eq!(json["actions"][0]["actionId"], b58(3));
            assert_eq!(json["actions"][1]["actionId"], b58(4));
            assert_eq!(json["nextStartAtActionId"], b58(4));
            assert_eq!(json["nextStartAtActionIdIncluded"], false);
            destroy_mock_sdk_handle(handle);
        }
    }

    /// A non-null list pointer with a count of 0 is an explicit empty selection: the status
    /// request goes out with no list and no contract request is made (the mock answers only
    /// that exact status request, so any other request fails the call).
    #[test]
    fn should_send_an_explicit_empty_list_selection_as_it_is() {
        let contract_id = Identifier::new([1; 32]);
        let identity_id = Identifier::new([2; 32]);
        let handle = mock_handle_answering(|wrapper| {
            Box::pin(async move {
                wrapper
                    .sdk
                    .mock()
                    .expect_fetch(
                        ContractModerationStatusQuery {
                            contract_id,
                            identity_id,
                            lists: vec![],
                        },
                        Some(ContractModerationListStatuses(vec![])),
                    )
                    .await
                    .expect("register the status");
            })
        });
        let contract = CString::new(b58(1)).expect("no NUL");
        let identity = CString::new(b58(2)).expect("no NUL");
        let lists = [0u8];
        unsafe {
            let json = json_of(dash_sdk_contract_moderation_get_status(
                handle,
                contract.as_ptr(),
                identity.as_ptr(),
                lists.as_ptr(),
                0,
            ));
            assert_eq!(json, serde_json::json!({"lists": []}));
            destroy_mock_sdk_handle(handle);
        }
    }

    /// A pointer to a limit of 0 sends `limit: Some(0)` as it is, for the node to judge. The
    /// expected request is written out as the wire proto, not lowered from a page query as the
    /// call is, so a lowering that dropped the 0 would not match: the mock answers only that
    /// exact request, and any other fails the call.
    #[test]
    fn should_send_a_limit_of_zero_as_it_is() {
        let handle = mock_handle_answering(|wrapper| {
            Box::pin(async move {
                let request = GetContractModerationEntriesRequest {
                    version: Some(get_contract_moderation_entries_request::Version::V0(
                        GetContractModerationEntriesRequestV0 {
                            contract_id: vec![1; 32],
                            list: ContractModerationListProto::Banlist as i32,
                            start_after: None,
                            limit: Some(0),
                            prove: true,
                        },
                    )),
                };
                wrapper
                    .sdk
                    .mock()
                    .expect_fetch(request, Some(ContractModerationEntries(vec![])))
                    .await
                    .expect("register the entries");
            })
        });
        let contract = CString::new(b58(1)).expect("no NUL");
        let limit = 0u32;
        unsafe {
            let json = json_of(dash_sdk_contract_moderation_get_entries(
                handle,
                contract.as_ptr(),
                0,
                std::ptr::null(),
                &limit,
            ));
            assert_eq!(json, serde_json::json!({"entries": []}));
            destroy_mock_sdk_handle(handle);
        }
    }
}
