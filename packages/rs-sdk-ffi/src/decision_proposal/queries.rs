//! Decision proposal reads: a proposal by id, the three listings, and the proposal's yes/no
//! poll (its id, its state and a masternode's answer on it). Each returns JSON written by
//! [`super::json`], or no data when what it reads is absent.

use super::json::{masternode_vote_json, proposal_json, proposals_json, vote_poll_state_json};
use super::{json_result, optional_identifier, required_identifier, sdk_wrapper, string_result};
use crate::types::SDKHandle;
use crate::voting::masternode_vote::pro_tx_hash_from_wire_order;
use crate::{DashSDKResult, FFIError};
use dash_sdk::dpp::platform_value::string_encoding::Encoding;
use dash_sdk::dpp::voting::vote_polls::yes_no_vote_poll::YesNoVotePollPurpose;
use dash_sdk::platform::decision_proposals::DecisionProposalEndDates;
use dash_sdk::platform::documents::DocumentsPage;
use dash_sdk::platform::yes_no_vote_polls::{
    MasternodeYesNoVote, MasternodeYesNoVoteQuery, YesNoVotePollState, YesNoVotePollStateQuery,
};
use dash_sdk::platform::Fetch;
use serde_json::Value as JsonValue;
use std::ffi::c_char;

/// The page a listing asks for: at most `limit` proposals (0 is Platform's cap of 100),
/// after the proposal `start_after` (null for the first page).
///
/// # Safety
/// `start_after` must be null or point to a NUL-terminated string.
unsafe fn page(limit: u32, start_after: *const c_char) -> Result<DocumentsPage, FFIError> {
    Ok(DocumentsPage {
        limit: (limit > 0).then_some(limit),
        start_after: optional_identifier(start_after, "start_after")?,
    })
}

/// Fetches a decision proposal by id.
///
/// # Parameters
/// * `sdk_handle` - Handle to the SDK instance.
/// * `proposal_id` - The proposal's id, base58.
///
/// # Returns
/// The proposal as a JSON object, or no data when there is none:
///
/// ```json
/// {"id": "<base58>", "ownerId": "<base58>", "revision": 1,
///  "createdAt": 1700000000000, "updatedAt": 1700000000000,
///  "title": "Raise the block size?", "description": "Yes doubles it; no keeps it.",
///  "url": "https://forum.dash.org/t/block-size",
///  "passPercent": 60, "minimumTurnoutPercent": 10, "endDate": 1701209600000}
/// ```
///
/// `url` is null when the proposal has none; `revision`, `createdAt` and `updatedAt` are
/// null when the document does not carry them. Times are milliseconds since the Unix epoch.
/// The proposal passes when the yes voting power is strictly more than `passPercent` percent
/// of yes plus no, and yes plus no reaches `minimumTurnoutPercent` percent of the network's
/// voting power when it closes, rounded up. Every listing returns proposals in this shape.
/// Free the string with `dash_sdk_string_free`.
///
/// # Safety
/// - `sdk_handle` must be a valid pointer to an initialized `SDKHandle`.
/// - `proposal_id` must be a valid pointer to a NUL-terminated string for the duration of the
///   call.
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_decision_proposal_fetch(
    sdk_handle: *const SDKHandle,
    proposal_id: *const c_char,
) -> DashSDKResult {
    json_result(fetch_inner(sdk_handle, proposal_id))
}

unsafe fn fetch_inner(
    sdk_handle: *const SDKHandle,
    proposal_id: *const c_char,
) -> Result<Option<JsonValue>, FFIError> {
    let wrapper = sdk_wrapper(sdk_handle)?;
    let proposal_id = required_identifier(proposal_id, "proposal_id")?;
    wrapper
        .runtime
        .block_on(wrapper.sdk.fetch_decision_proposal(proposal_id))?
        .map(proposal_json)
        .transpose()
}

/// Lists one page of the decision proposals whose end date is within the bounds, closing
/// soonest first (by end date, then by creation time).
///
/// # Parameters
/// * `sdk_handle` - Handle to the SDK instance.
/// * `end_date_from_ms` - The earliest end date listed, included, in milliseconds since the
///   Unix epoch; 0 for no lower bound. The current time lists the open proposals.
/// * `end_date_to_ms` - The end date the listing stops before (left out); 0 for no upper
///   bound.
/// * `limit` - The most proposals to return, at most 100; 0 for 100.
/// * `start_after` - The id (base58) of the last proposal of the previous page, to continue
///   after it; null for the first page.
///
/// # Returns
/// A JSON array of proposals in the shape `dash_sdk_decision_proposal_fetch` returns, `[]`
/// when there is none. A page shorter than the limit is the last. Free the string with
/// `dash_sdk_string_free`.
///
/// # Safety
/// - `sdk_handle` must be a valid pointer to an initialized `SDKHandle`.
/// - `start_after` must be null or a valid pointer to a NUL-terminated string for the duration
///   of the call.
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_decision_proposal_list_by_end_date(
    sdk_handle: *const SDKHandle,
    end_date_from_ms: u64,
    end_date_to_ms: u64,
    limit: u32,
    start_after: *const c_char,
) -> DashSDKResult {
    json_result(list_by_end_date_inner(
        sdk_handle,
        end_date_from_ms,
        end_date_to_ms,
        limit,
        start_after,
    ))
}

unsafe fn list_by_end_date_inner(
    sdk_handle: *const SDKHandle,
    end_date_from_ms: u64,
    end_date_to_ms: u64,
    limit: u32,
    start_after: *const c_char,
) -> Result<Option<JsonValue>, FFIError> {
    let wrapper = sdk_wrapper(sdk_handle)?;
    let end_dates = DecisionProposalEndDates {
        from: (end_date_from_ms > 0).then_some(end_date_from_ms),
        to: (end_date_to_ms > 0).then_some(end_date_to_ms),
    };
    let page = page(limit, start_after)?;
    let documents = wrapper.runtime.block_on(
        wrapper
            .sdk
            .fetch_decision_proposals_by_end_date(end_dates, page),
    )?;
    proposals_json(documents).map(Some)
}

/// Lists one page of an identity's decision proposals, in the order it filed them.
///
/// # Parameters
/// * `sdk_handle` - Handle to the SDK instance.
/// * `owner_id` - The proposer's identity id, base58.
/// * `limit` - The most proposals to return, at most 100; 0 for 100.
/// * `start_after` - The id (base58) of the last proposal of the previous page; null for the
///   first page.
///
/// # Returns
/// A JSON array of proposals in the shape `dash_sdk_decision_proposal_fetch` returns, `[]`
/// when there is none. Free the string with `dash_sdk_string_free`.
///
/// # Safety
/// - `sdk_handle` must be a valid pointer to an initialized `SDKHandle`.
/// - `owner_id` must be a valid pointer to a NUL-terminated string, and `start_after` null or
///   one, for the duration of the call.
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_decision_proposal_list_by_owner(
    sdk_handle: *const SDKHandle,
    owner_id: *const c_char,
    limit: u32,
    start_after: *const c_char,
) -> DashSDKResult {
    json_result(list_by_owner_inner(
        sdk_handle,
        owner_id,
        limit,
        start_after,
    ))
}

unsafe fn list_by_owner_inner(
    sdk_handle: *const SDKHandle,
    owner_id: *const c_char,
    limit: u32,
    start_after: *const c_char,
) -> Result<Option<JsonValue>, FFIError> {
    let wrapper = sdk_wrapper(sdk_handle)?;
    let owner_id = required_identifier(owner_id, "owner_id")?;
    let page = page(limit, start_after)?;
    let documents = wrapper.runtime.block_on(
        wrapper
            .sdk
            .fetch_decision_proposals_by_owner(owner_id, page),
    )?;
    proposals_json(documents).map(Some)
}

/// Lists one page of every decision proposal, newest first.
///
/// # Parameters
/// * `sdk_handle` - Handle to the SDK instance.
/// * `limit` - The most proposals to return, at most 100; 0 for 100.
/// * `start_after` - The id (base58) of the last proposal of the previous page; null for the
///   first page.
///
/// # Returns
/// A JSON array of proposals in the shape `dash_sdk_decision_proposal_fetch` returns, `[]`
/// when there is none. Free the string with `dash_sdk_string_free`.
///
/// # Safety
/// - `sdk_handle` must be a valid pointer to an initialized `SDKHandle`.
/// - `start_after` must be null or a valid pointer to a NUL-terminated string for the duration
///   of the call.
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_decision_proposal_list_by_created_at(
    sdk_handle: *const SDKHandle,
    limit: u32,
    start_after: *const c_char,
) -> DashSDKResult {
    json_result(list_by_created_at_inner(sdk_handle, limit, start_after))
}

unsafe fn list_by_created_at_inner(
    sdk_handle: *const SDKHandle,
    limit: u32,
    start_after: *const c_char,
) -> Result<Option<JsonValue>, FFIError> {
    let wrapper = sdk_wrapper(sdk_handle)?;
    let page = page(limit, start_after)?;
    let documents = wrapper
        .runtime
        .block_on(wrapper.sdk.fetch_decision_proposals_by_created_at(page))?;
    proposals_json(documents).map(Some)
}

/// The id of the yes/no poll a decision proposal opens: the id
/// `dash_sdk_voting_get_vote_polls_by_end_date` lists it under and votes name it by. Derived
/// with the protocol version the SDK speaks; a network below protocol version 15 has no polls,
/// which is an error.
///
/// # Parameters
/// * `sdk_handle` - Handle to the SDK instance.
/// * `proposal_id` - The proposal's id, base58.
///
/// # Returns
/// The poll's id as a base58 string (not JSON). Free it with `dash_sdk_string_free`.
///
/// # Safety
/// - `sdk_handle` must be a valid pointer to an initialized `SDKHandle`.
/// - `proposal_id` must be a valid pointer to a NUL-terminated string for the duration of the
///   call.
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_decision_proposal_get_vote_poll_id(
    sdk_handle: *const SDKHandle,
    proposal_id: *const c_char,
) -> DashSDKResult {
    string_result(get_vote_poll_id_inner(sdk_handle, proposal_id))
}

unsafe fn get_vote_poll_id_inner(
    sdk_handle: *const SDKHandle,
    proposal_id: *const c_char,
) -> Result<Option<String>, FFIError> {
    let wrapper = sdk_wrapper(sdk_handle)?;
    let proposal_id = required_identifier(proposal_id, "proposal_id")?;
    let vote_poll_id = wrapper
        .runtime
        .block_on(wrapper.sdk.decision_proposal_vote_poll_id(proposal_id))?;
    Ok(Some(vote_poll_id.to_string(Encoding::Base58)))
}

/// The status and tallies of a decision proposal's yes/no poll.
///
/// # Parameters
/// * `sdk_handle` - Handle to the SDK instance.
/// * `proposal_id` - The proposal's id, base58.
///
/// # Returns
/// A JSON object, or no data when the poll was never opened (no such proposal was created).
/// While the poll is open:
///
/// ```json
/// {"votePoll": {"$type": "yesNoVotePoll",
///               "purpose": {"$type": "decisionProposal", "documentId": "<base58>"},
///               "supermajorityNumerator": 60, "supermajorityDenominator": 100,
///               "minimumVotingPower": {"$type": "fractionOfTotal", "numerator": 10,
///                                      "denominator": 100, "rounding": "up"}},
///  "status": "started",
///  "startBlock": {"height": 7, "coreHeight": 42, "timeMs": 7000, "epoch": 3},
///  "yesVotingPower": 5, "noVotingPower": 1, "abstainVotingPower": 4}
/// ```
///
/// Once it finished, `status` is `"finished"`, `startBlock` is absent and `result` holds the
/// record: `{"passed": true, "yesVotingPower": 30, "noVotingPower": 10,
/// "abstainVotingPower": 8, "requiredVotingPower": 20, "startBlock": {...},
/// "finalizationBlock": {...}}`; the top-level tallies are then 0, as the votes are removed.
/// `votePoll` is the poll as it was opened, rule included (`minimumVotingPower` may also be
/// `{"$type": "absolute", "votingPower": 400}`). Voting powers count a masternode 1 and an
/// evonode 4. Free the string with `dash_sdk_string_free`.
///
/// # Safety
/// - `sdk_handle` must be a valid pointer to an initialized `SDKHandle`.
/// - `proposal_id` must be a valid pointer to a NUL-terminated string for the duration of the
///   call.
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_decision_proposal_get_vote_poll_state(
    sdk_handle: *const SDKHandle,
    proposal_id: *const c_char,
) -> DashSDKResult {
    json_result(get_vote_poll_state_inner(sdk_handle, proposal_id))
}

unsafe fn get_vote_poll_state_inner(
    sdk_handle: *const SDKHandle,
    proposal_id: *const c_char,
) -> Result<Option<JsonValue>, FFIError> {
    let wrapper = sdk_wrapper(sdk_handle)?;
    let document_id = required_identifier(proposal_id, "proposal_id")?;
    let query =
        YesNoVotePollStateQuery::new(YesNoVotePollPurpose::DecisionProposal { document_id });
    let state = wrapper
        .runtime
        .block_on(YesNoVotePollState::fetch(&wrapper.sdk, query))?;
    vote_poll_state_json(state)
}

/// A masternode's answer on a decision proposal and how many times it has voted on it.
///
/// # Parameters
/// * `sdk_handle` - Handle to the SDK instance.
/// * `voter_pro_tx_hash` - Pointer to the masternode's 32-byte pro_tx_hash in wire order, the
///   same bytes `dash_sdk_decision_proposal_cast_vote` takes (the reverse of the hex Core
///   displays). They are reversed here, as for the vote.
/// * `proposal_id` - The proposal's id, base58.
///
/// # Returns
/// `{"choice": "yes" | "no" | "abstain", "timesVoted": 2}`, or no data when the masternode
/// has not voted on the proposal, and once the poll finished, which removes its votes.
/// `timesVoted` counts the first vote. Free the string with `dash_sdk_string_free`.
///
/// # Safety
/// - `sdk_handle` must be a valid pointer to an initialized `SDKHandle`.
/// - `voter_pro_tx_hash` must point to 32 readable bytes and `proposal_id` to a
///   NUL-terminated string for the duration of the call.
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_decision_proposal_get_masternode_vote(
    sdk_handle: *const SDKHandle,
    voter_pro_tx_hash: *const u8,
    proposal_id: *const c_char,
) -> DashSDKResult {
    json_result(get_masternode_vote_inner(
        sdk_handle,
        voter_pro_tx_hash,
        proposal_id,
    ))
}

unsafe fn get_masternode_vote_inner(
    sdk_handle: *const SDKHandle,
    voter_pro_tx_hash: *const u8,
    proposal_id: *const c_char,
) -> Result<Option<JsonValue>, FFIError> {
    let wrapper = sdk_wrapper(sdk_handle)?;
    let pro_tx_hash = pro_tx_hash_from_wire_order(voter_pro_tx_hash)?;
    let document_id = required_identifier(proposal_id, "proposal_id")?;
    let query = MasternodeYesNoVoteQuery::new(
        pro_tx_hash,
        YesNoVotePollPurpose::DecisionProposal { document_id },
    );
    let vote = wrapper
        .runtime
        .block_on(MasternodeYesNoVote::fetch(&wrapper.sdk, query))?;
    Ok(masternode_vote_json(vote))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::test_utils::{create_mock_sdk_handle, destroy_mock_sdk_handle};
    use crate::{dash_sdk_error_free, DashSDKErrorCode};
    use dash_sdk::dpp::platform_value::Identifier;
    use std::ffi::CString;

    /// `result` failed with `InvalidParameter`; its error is freed.
    unsafe fn assert_invalid_parameter(result: DashSDKResult) {
        assert!(result.data.is_null());
        assert!(!result.error.is_null());
        let code = (*result.error).code;
        dash_sdk_error_free(result.error);
        assert_eq!(code, DashSDKErrorCode::InvalidParameter);
    }

    fn base58(id: Identifier) -> CString {
        CString::new(id.to_string(Encoding::Base58)).expect("no NUL")
    }

    /// Every read refuses a null SDK handle, a null or non-base58 id and a null pro_tx_hash
    /// before anything reaches the network.
    #[test]
    fn should_refuse_missing_or_malformed_arguments_before_any_request() {
        let handle = create_mock_sdk_handle();
        let id = base58(Identifier::new([4; 32]));
        let not_base58 = CString::new("not base58: 0OIl").expect("no NUL");
        let pro_tx_hash = [1u8; 32];
        unsafe {
            for proposal_id in [std::ptr::null(), not_base58.as_ptr()] {
                assert_invalid_parameter(dash_sdk_decision_proposal_fetch(handle, proposal_id));
                assert_invalid_parameter(dash_sdk_decision_proposal_get_vote_poll_id(
                    handle,
                    proposal_id,
                ));
                assert_invalid_parameter(dash_sdk_decision_proposal_get_vote_poll_state(
                    handle,
                    proposal_id,
                ));
                assert_invalid_parameter(dash_sdk_decision_proposal_get_masternode_vote(
                    handle,
                    pro_tx_hash.as_ptr(),
                    proposal_id,
                ));
                assert_invalid_parameter(dash_sdk_decision_proposal_list_by_owner(
                    handle,
                    proposal_id,
                    0,
                    std::ptr::null(),
                ));
            }
            assert_invalid_parameter(dash_sdk_decision_proposal_list_by_end_date(
                handle,
                0,
                0,
                0,
                not_base58.as_ptr(),
            ));
            assert_invalid_parameter(dash_sdk_decision_proposal_list_by_created_at(
                handle,
                0,
                not_base58.as_ptr(),
            ));
            assert_invalid_parameter(dash_sdk_decision_proposal_get_masternode_vote(
                handle,
                std::ptr::null(),
                id.as_ptr(),
            ));

            let no_sdk = std::ptr::null();
            assert_invalid_parameter(dash_sdk_decision_proposal_fetch(no_sdk, id.as_ptr()));
            assert_invalid_parameter(dash_sdk_decision_proposal_list_by_end_date(
                no_sdk,
                0,
                0,
                0,
                std::ptr::null(),
            ));
            assert_invalid_parameter(dash_sdk_decision_proposal_list_by_owner(
                no_sdk,
                id.as_ptr(),
                0,
                std::ptr::null(),
            ));
            assert_invalid_parameter(dash_sdk_decision_proposal_list_by_created_at(
                no_sdk,
                0,
                std::ptr::null(),
            ));
            assert_invalid_parameter(dash_sdk_decision_proposal_get_vote_poll_id(
                no_sdk,
                id.as_ptr(),
            ));
            assert_invalid_parameter(dash_sdk_decision_proposal_get_vote_poll_state(
                no_sdk,
                id.as_ptr(),
            ));
            assert_invalid_parameter(dash_sdk_decision_proposal_get_masternode_vote(
                no_sdk,
                pro_tx_hash.as_ptr(),
                id.as_ptr(),
            ));
            destroy_mock_sdk_handle(handle);
        }
    }

    #[test]
    fn should_page_from_a_limit_and_a_cursor() {
        let cursor = base58(Identifier::new([5; 32]));
        unsafe {
            assert_eq!(
                page(0, std::ptr::null()).expect("a page"),
                DocumentsPage::default()
            );
            assert_eq!(
                page(20, cursor.as_ptr()).expect("a page"),
                DocumentsPage {
                    limit: Some(20),
                    start_after: Some(Identifier::new([5; 32])),
                }
            );
        }
    }
}
