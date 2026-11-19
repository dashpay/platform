//! Multiple choice proposals (protocol version 15): questions whose answers are the choices
//! entered for them, as `multipleChoiceProposal` and `choice` documents of the decision
//! proposals system contract. Thin bindings over
//! `dash_sdk::platform::decision_proposals::multiple_choice`: rs-sdk decides everything
//! (which documents are proposals and choices, the vote's contest, the funds and fees); these
//! functions read C arguments, call it, and write the result.
//!
//! # Creating, editing and entering choices
//!
//! Through the platform wallet, which signs with the owner's identity keys:
//! `platform_wallet_multiple_choice_proposal_create`, `_edit` and `_enter_choice` in
//! `rs-platform-wallet-ffi`. Creating a proposal costs the processing fee
//! [`dash_sdk_multiple_choice_proposal_get_processing_fee`] reads, on top of the fees; a choice
//! states the contest fund [`dash_sdk_multiple_choice_proposal_get_choice_fund_to_join`] reads.
//!
//! # Reading
//!
//! The reads return JSON, null when what they read is absent. A proposal is always the same
//! object (see [`dash_sdk_multiple_choice_proposal_fetch`]), and so is a choice (see
//! [`dash_sdk_multiple_choice_proposal_get_kept_choice`]); the listings return arrays of
//! proposals.
//!
//! # Voting
//!
//! The choices contend in a masternode vote, in which a choice is named by its document id.
//! [`dash_sdk_multiple_choice_proposal_cast_vote`] answers for a masternode from its voting
//! private key and pro_tx_hash, exactly as `dash_sdk_decision_proposal_cast_vote` answers a
//! yes/no proposal (see `crate::voting::masternode_vote`).

mod json;

use self::json::{
    choice_document_json, masternode_vote_json, multiple_choice_proposal_json,
    multiple_choice_proposal_vote_json, multiple_choice_proposals_json,
};
use super::queries::page;
use super::{json_result, optional_identifier, required_identifier, sdk_wrapper, string_result};
use crate::types::{FFINetwork, SDKHandle};
use crate::voting::masternode_vote::{pro_tx_hash_from_wire_order, MasternodeVoter};
use crate::{DashSDKResult, FFIError};
use dash_sdk::platform::decision_proposals::multiple_choice::{
    multiple_choice_proposal_vote, MultipleChoiceVoteChoice,
};
use serde_json::Value as JsonValue;
use std::ffi::c_char;

/// The answer of a masternode's vote among a multiple choice proposal's choices, the
/// `vote_choice` byte of `dash_sdk_multiple_choice_proposal_cast_vote`: `0` = for the choice
/// `choice_id` names, `1` = none of the choices, `2` = abstain.
///
/// The FFI parameter is a plain `u8`, not this enum: a C caller can pass any byte, and an
/// out-of-range value read as a `#[repr(u8)]` enum is undefined behavior. The byte is
/// compared with these discriminants, and any other value is refused with
/// `InvalidParameter`. Not exported to the C header for that reason; Swift and Kotlin mirror
/// the three values.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MultipleChoiceVoteChoiceFFI {
    Choice = 0,
    NoneOfTheChoices = 1,
    Abstain = 2,
}

/// The answer the `vote_choice` byte and `choice_id` name. `choice_id` names the choice for
/// `0` and must be null for the other answers.
///
/// # Safety
/// `choice_id` must be null or point to a NUL-terminated string.
unsafe fn vote_choice_from_ffi(
    vote_choice: u8,
    choice_id: *const c_char,
) -> Result<MultipleChoiceVoteChoice, FFIError> {
    let choice_id = optional_identifier(choice_id, "choice_id")?;
    match (vote_choice, choice_id) {
        (x, Some(choice_id)) if x == MultipleChoiceVoteChoiceFFI::Choice as u8 => {
            Ok(MultipleChoiceVoteChoice::Choice(choice_id))
        }
        (x, None) if x == MultipleChoiceVoteChoiceFFI::Choice as u8 => Err(
            FFIError::InvalidParameter("choice_id is null for a vote for a choice".to_string()),
        ),
        (x, None) if x == MultipleChoiceVoteChoiceFFI::NoneOfTheChoices as u8 => {
            Ok(MultipleChoiceVoteChoice::NoneOfTheChoices)
        }
        (x, None) if x == MultipleChoiceVoteChoiceFFI::Abstain as u8 => {
            Ok(MultipleChoiceVoteChoice::Abstain)
        }
        (x, Some(_))
            if x == MultipleChoiceVoteChoiceFFI::NoneOfTheChoices as u8
                || x == MultipleChoiceVoteChoiceFFI::Abstain as u8 =>
        {
            Err(FFIError::InvalidParameter(format!(
                "choice_id must be null for vote_choice {x}, which names no choice"
            )))
        }
        (other, _) => Err(FFIError::InvalidParameter(format!(
            "vote_choice must be 0 (a choice), 1 (none of the choices) or 2 (abstain); got \
             {other}"
        ))),
    }
}

/// Fetches a multiple choice proposal by id.
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
///  "title": "Which block size?",
///  "descriptionDocument": {"id": "<base58>", "contractId": "<base58>", "documentType": "post"},
///  "url": "https://forum.dash.org/t/block-size", "open": true}
/// ```
///
/// `descriptionDocument` and `url` are null when the proposal has none (it has at least one);
/// `revision`, `createdAt` and `updatedAt` are null when the document does not carry them.
/// Times are milliseconds since the Unix epoch. `open` says whether anyone may enter choices
/// (only the owner may when it is false). Every listing returns proposals in this shape.
/// Free the string with `dash_sdk_string_free`.
///
/// # Safety
/// - `sdk_handle` must be a valid pointer to an initialized `SDKHandle`.
/// - `proposal_id` must be a valid pointer to a NUL-terminated string for the duration of the
///   call.
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_multiple_choice_proposal_fetch(
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
        .block_on(wrapper.sdk.fetch_multiple_choice_proposal(proposal_id))?
        .map(multiple_choice_proposal_json)
        .transpose()
}

/// Lists one page of an identity's multiple choice proposals, in the order it filed them.
///
/// # Parameters
/// * `sdk_handle` - Handle to the SDK instance.
/// * `owner_id` - The proposer's identity id, base58.
/// * `limit` - The most proposals to return, at most 100; 0 for 100.
/// * `start_after` - The id (base58) of the last proposal of the previous page; null for the
///   first page.
///
/// # Returns
/// A JSON array of proposals in the shape `dash_sdk_multiple_choice_proposal_fetch` returns,
/// `[]` when there is none. A page shorter than the limit is the last. Free the string with
/// `dash_sdk_string_free`.
///
/// # Safety
/// - `sdk_handle` must be a valid pointer to an initialized `SDKHandle`.
/// - `owner_id` must be a valid pointer to a NUL-terminated string, and `start_after` null or
///   one, for the duration of the call.
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_multiple_choice_proposal_list_by_owner(
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
            .fetch_multiple_choice_proposals_by_owner(owner_id, page),
    )?;
    multiple_choice_proposals_json(documents).map(Some)
}

/// Lists one page of every multiple choice proposal, newest first.
///
/// # Parameters
/// * `sdk_handle` - Handle to the SDK instance.
/// * `limit` - The most proposals to return, at most 100; 0 for 100.
/// * `start_after` - The id (base58) of the last proposal of the previous page; null for the
///   first page.
///
/// # Returns
/// A JSON array of proposals in the shape `dash_sdk_multiple_choice_proposal_fetch` returns,
/// `[]` when there is none. Free the string with `dash_sdk_string_free`.
///
/// # Safety
/// - `sdk_handle` must be a valid pointer to an initialized `SDKHandle`.
/// - `start_after` must be null or a valid pointer to a NUL-terminated string for the duration
///   of the call.
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_multiple_choice_proposal_list_by_created_at(
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
    let documents = wrapper.runtime.block_on(
        wrapper
            .sdk
            .fetch_multiple_choice_proposals_by_created_at(page),
    )?;
    multiple_choice_proposals_json(documents).map(Some)
}

/// The vote among a multiple choice proposal's choices: every choice with its tally while the
/// vote runs, and how it ended with every choice's final tally once it did.
///
/// # Parameters
/// * `sdk_handle` - Handle to the SDK instance.
/// * `proposal_id` - The proposal's id, base58.
///
/// # Returns
/// A JSON object. While the vote runs:
///
/// ```json
/// {"choices": [{"id": "<base58>",
///               "choice": {"id": "<base58>", "ownerId": "<base58>",
///                          "proposalId": "<base58>", "revision": null,
///                          "createdAt": 1700000000000, "updatedAt": null,
///                          "title": "Double it",
///                          "description": "Twice the size it is today."},
///               "voteTally": 12}],
///  "abstainVoteTally": 3, "noneOfTheChoicesVoteTally": 1, "result": null}
/// ```
///
/// `choices` is ordered by choice id, and `[]` for a proposal no choice was entered for. Each
/// `id` is the choice's document id, the one a vote for it names; `choice` is the choice in
/// the shape `dash_sdk_multiple_choice_proposal_get_kept_choice` returns, null once the vote
/// ended (the choices not kept are gone, and the kept one is read with that function). The
/// tallies are null when Platform does not report them. Once the vote ended, `result` is
/// `{"outcome": "keptChoice" | "noneOfTheChoices" | "noWinner", "keptChoiceId": "<base58>",
/// "finalizationBlock": {"height": 9, "coreHeight": 42, "timeMs": 9000, "epoch": 3}}`, with
/// `keptChoiceId` null unless `outcome` is `"keptChoice"`. Free the string with
/// `dash_sdk_string_free`.
///
/// # Safety
/// - `sdk_handle` must be a valid pointer to an initialized `SDKHandle`.
/// - `proposal_id` must be a valid pointer to a NUL-terminated string for the duration of the
///   call.
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_multiple_choice_proposal_get_vote(
    sdk_handle: *const SDKHandle,
    proposal_id: *const c_char,
) -> DashSDKResult {
    json_result(get_vote_inner(sdk_handle, proposal_id))
}

unsafe fn get_vote_inner(
    sdk_handle: *const SDKHandle,
    proposal_id: *const c_char,
) -> Result<Option<JsonValue>, FFIError> {
    let wrapper = sdk_wrapper(sdk_handle)?;
    let proposal_id = required_identifier(proposal_id, "proposal_id")?;
    let vote = wrapper
        .runtime
        .block_on(wrapper.sdk.fetch_multiple_choice_proposal_vote(proposal_id))?;
    multiple_choice_proposal_vote_json(&vote).map(Some)
}

/// The choice the finished vote of a multiple choice proposal kept, an ordinary document of
/// the identity that entered it.
///
/// # Parameters
/// * `sdk_handle` - Handle to the SDK instance.
/// * `proposal_id` - The proposal's id, base58.
///
/// # Returns
/// The choice as a JSON object, or no data while the vote runs, when it kept none of the
/// choices, and for a proposal no choice was entered for:
///
/// ```json
/// {"id": "<base58>", "ownerId": "<base58>", "proposalId": "<base58>", "revision": null,
///  "createdAt": 1700000000000, "updatedAt": null,
///  "title": "Double it", "description": "Twice the size it is today."}
/// ```
///
/// `description` is null when the choice has none; `revision`, `createdAt` and `updatedAt`
/// are null when the document does not carry them. Free the string with
/// `dash_sdk_string_free`.
///
/// # Safety
/// - `sdk_handle` must be a valid pointer to an initialized `SDKHandle`.
/// - `proposal_id` must be a valid pointer to a NUL-terminated string for the duration of the
///   call.
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_multiple_choice_proposal_get_kept_choice(
    sdk_handle: *const SDKHandle,
    proposal_id: *const c_char,
) -> DashSDKResult {
    json_result(get_kept_choice_inner(sdk_handle, proposal_id))
}

unsafe fn get_kept_choice_inner(
    sdk_handle: *const SDKHandle,
    proposal_id: *const c_char,
) -> Result<Option<JsonValue>, FFIError> {
    let wrapper = sdk_wrapper(sdk_handle)?;
    let proposal_id = required_identifier(proposal_id, "proposal_id")?;
    wrapper
        .runtime
        .block_on(wrapper.sdk.fetch_kept_choice(proposal_id))?
        .map(choice_document_json)
        .transpose()
}

/// The answer a masternode gave in the vote among a multiple choice proposal's choices.
///
/// # Parameters
/// * `sdk_handle` - Handle to the SDK instance.
/// * `voter_pro_tx_hash` - Pointer to the masternode's 32-byte pro_tx_hash in wire order, the
///   same bytes `dash_sdk_multiple_choice_proposal_cast_vote` takes (the reverse of the hex
///   Core displays). They are reversed here, as for the vote.
/// * `proposal_id` - The proposal's id, base58.
///
/// # Returns
/// `{"choice": "choice", "choiceId": "<base58>"}` for a vote for a choice, or
/// `{"choice": "noneOfTheChoices" | "abstain", "choiceId": null}`; no data when the
/// masternode has not voted in it, and once the vote ended. Free the string with
/// `dash_sdk_string_free`.
///
/// # Safety
/// - `sdk_handle` must be a valid pointer to an initialized `SDKHandle`.
/// - `voter_pro_tx_hash` must point to 32 readable bytes and `proposal_id` to a
///   NUL-terminated string for the duration of the call.
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_multiple_choice_proposal_get_masternode_vote(
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
    let proposal_id = required_identifier(proposal_id, "proposal_id")?;
    let vote_choice = wrapper.runtime.block_on(
        wrapper
            .sdk
            .fetch_masternode_multiple_choice_vote(pro_tx_hash, proposal_id),
    )?;
    Ok(masternode_vote_json(vote_choice))
}

/// What creating one multiple choice proposal adds to the processing fee of its batch, which
/// Platform charges the owner on top of the fees; the create does not state it. A network
/// below protocol version 15 has no multiple choice proposals, which is an error.
///
/// # Parameters
/// * `sdk_handle` - Handle to the SDK instance.
///
/// # Returns
/// The fee in credits as a decimal string (not JSON), for example `"20000000000"`. Free it
/// with `dash_sdk_string_free`.
///
/// # Safety
/// - `sdk_handle` must be a valid pointer to an initialized `SDKHandle`.
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_multiple_choice_proposal_get_processing_fee(
    sdk_handle: *const SDKHandle,
) -> DashSDKResult {
    string_result(get_processing_fee_inner(sdk_handle))
}

unsafe fn get_processing_fee_inner(
    sdk_handle: *const SDKHandle,
) -> Result<Option<String>, FFIError> {
    let wrapper = sdk_wrapper(sdk_handle)?;
    let fee = wrapper
        .runtime
        .block_on(wrapper.sdk.multiple_choice_proposal_processing_fee())?;
    Ok(Some(fee.to_string()))
}

/// The fund a choice entered now for a multiple choice proposal pays: the contest's fund,
/// doubled once the vote holds 250 choices and again for every 50 more. The put of a choice
/// reads and states it itself; this is for showing it beforehand.
///
/// # Parameters
/// * `sdk_handle` - Handle to the SDK instance.
/// * `proposal_id` - The proposal's id, base58.
///
/// # Returns
/// The fund in credits as a decimal string (not JSON), for example `"10000000000"`. Free it
/// with `dash_sdk_string_free`.
///
/// # Safety
/// - `sdk_handle` must be a valid pointer to an initialized `SDKHandle`.
/// - `proposal_id` must be a valid pointer to a NUL-terminated string for the duration of the
///   call.
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_multiple_choice_proposal_get_choice_fund_to_join(
    sdk_handle: *const SDKHandle,
    proposal_id: *const c_char,
) -> DashSDKResult {
    string_result(get_choice_fund_to_join_inner(sdk_handle, proposal_id))
}

unsafe fn get_choice_fund_to_join_inner(
    sdk_handle: *const SDKHandle,
    proposal_id: *const c_char,
) -> Result<Option<String>, FFIError> {
    let wrapper = sdk_wrapper(sdk_handle)?;
    let proposal_id = required_identifier(proposal_id, "proposal_id")?;
    let fund = wrapper
        .runtime
        .block_on(wrapper.sdk.choice_fund_to_join(proposal_id))?;
    Ok(Some(fund.to_string()))
}

/// Votes in the vote among a multiple choice proposal's choices for a masternode: builds the
/// vote, signs it with the masternode voting key derived from `voting_private_key`,
/// broadcasts it for `voter_pro_tx_hash` and waits for the response. Every argument is read
/// before anything reaches the network.
///
/// The signer, the voting key and the voter identity are derived exactly as
/// `dash_sdk_decision_proposal_cast_vote` derives them, and a refused voting key is explained
/// the same way.
///
/// # Parameters
/// * `sdk_handle` - Handle to the SDK instance.
/// * `proposal_id` - The proposal's id, base58.
/// * `vote_choice` - `0` = for the choice `choice_id` names, `1` = none of the choices (keeps
///   no choice when these votes outnumber the leading choice), `2` = abstain. Any other value
///   is refused with `InvalidParameter`.
/// * `choice_id` - The document id (base58) of the choice voted for, not the id of the
///   identity that entered it. Required for `vote_choice` `0` and null otherwise; anything
///   else is refused with `InvalidParameter`.
/// * `voter_pro_tx_hash` - Pointer to the masternode's 32-byte pro_tx_hash in wire order, as
///   `dash_sdk_decision_proposal_cast_vote` takes it (NOT the byte order of the hex Core
///   displays, which is its reverse). The bytes are reversed here.
/// * `voting_private_key` - Pointer to the 32-byte masternode voting private key. It never
///   leaves this call.
/// * `network` - The network, used for the WIF encoding inside the signer only.
///
/// # Returns
/// No data on success, or an error. A vote from a wallet that is not a masternode fails with
/// a no-voter-identity error from Platform, the expected outcome.
///
/// # Safety
/// - `sdk_handle` must be a valid pointer to an initialized `SDKHandle`.
/// - `proposal_id` must be a valid pointer to a NUL-terminated string, and `choice_id` null or
///   one.
/// - `voter_pro_tx_hash` and `voting_private_key` must each point to 32 readable bytes. All
///   must stay valid for the duration of the call.
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_multiple_choice_proposal_cast_vote(
    sdk_handle: *const SDKHandle,
    proposal_id: *const c_char,
    vote_choice: u8,
    choice_id: *const c_char,
    voter_pro_tx_hash: *const u8,
    voting_private_key: *const u8,
    network: FFINetwork,
) -> DashSDKResult {
    match cast_vote_inner(
        sdk_handle,
        proposal_id,
        vote_choice,
        choice_id,
        voter_pro_tx_hash,
        voting_private_key,
        network,
    ) {
        Ok(()) => DashSDKResult::success(std::ptr::null_mut()),
        Err(error) => DashSDKResult::error(error.into()),
    }
}

unsafe fn cast_vote_inner(
    sdk_handle: *const SDKHandle,
    proposal_id: *const c_char,
    vote_choice: u8,
    choice_id: *const c_char,
    voter_pro_tx_hash: *const u8,
    voting_private_key: *const u8,
    network: FFINetwork,
) -> Result<(), FFIError> {
    let wrapper = sdk_wrapper(sdk_handle)?;
    let proposal_id = required_identifier(proposal_id, "proposal_id")?;
    let vote_choice = vote_choice_from_ffi(vote_choice, choice_id)?;
    let voter = MasternodeVoter::from_raw(voter_pro_tx_hash, voting_private_key, network)?;
    let vote = multiple_choice_proposal_vote(proposal_id, vote_choice);
    wrapper
        .runtime
        .block_on(voter.put_vote(&wrapper.sdk, &vote))
}

#[cfg(test)]
mod tests {
    use super::json::vote_choice_name;
    use super::*;
    use crate::test_utils::test_utils::{create_mock_sdk_handle, destroy_mock_sdk_handle};
    use crate::{dash_sdk_error_free, DashSDKErrorCode};
    use dash_sdk::dpp::platform_value::string_encoding::Encoding;
    use dash_sdk::dpp::platform_value::Identifier;
    use std::ffi::CString;

    /// `result` failed with `InvalidParameter`; its error is freed.
    unsafe fn assert_invalid_parameter(result: DashSDKResult) {
        assert!(result.data.is_null());
        assert!(!result.error.is_null(), "expected an error");
        let code = (*result.error).code;
        dash_sdk_error_free(result.error);
        assert_eq!(code, DashSDKErrorCode::InvalidParameter);
    }

    fn base58(id: Identifier) -> CString {
        CString::new(id.to_string(Encoding::Base58)).expect("no NUL")
    }

    #[test]
    fn should_map_each_vote_choice_byte_and_refuse_the_rest() {
        let choice_id = Identifier::new([9; 32]);
        let choice = base58(choice_id);
        let not_base58 = CString::new("0OIl").expect("no NUL");
        unsafe {
            assert_eq!(
                vote_choice_from_ffi(0, choice.as_ptr()).ok(),
                Some(MultipleChoiceVoteChoice::Choice(choice_id))
            );
            assert_eq!(
                vote_choice_from_ffi(1, std::ptr::null()).ok(),
                Some(MultipleChoiceVoteChoice::NoneOfTheChoices)
            );
            assert_eq!(
                vote_choice_from_ffi(2, std::ptr::null()).ok(),
                Some(MultipleChoiceVoteChoice::Abstain)
            );
            for (byte, choice_id) in [
                // A vote for a choice names it
                (0, std::ptr::null()),
                (0, not_base58.as_ptr()),
                // The other answers name none
                (1, choice.as_ptr()),
                (2, choice.as_ptr()),
                (3, std::ptr::null()),
                (3, choice.as_ptr()),
                (u8::MAX, std::ptr::null()),
            ] {
                assert!(
                    matches!(
                        vote_choice_from_ffi(byte, choice_id),
                        Err(FFIError::InvalidParameter(_))
                    ),
                    "vote_choice {byte}"
                );
            }
            // The byte and the name a masternode's answer is read back with agree
            for (byte, choice_id, name) in [
                (0, choice.as_ptr(), "choice"),
                (1, std::ptr::null(), "noneOfTheChoices"),
                (2, std::ptr::null(), "abstain"),
            ] {
                assert_eq!(
                    vote_choice_name(&vote_choice_from_ffi(byte, choice_id).expect("an answer")),
                    name
                );
            }
        }
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
                assert_invalid_parameter(dash_sdk_multiple_choice_proposal_fetch(
                    handle,
                    proposal_id,
                ));
                assert_invalid_parameter(dash_sdk_multiple_choice_proposal_list_by_owner(
                    handle,
                    proposal_id,
                    0,
                    std::ptr::null(),
                ));
                assert_invalid_parameter(dash_sdk_multiple_choice_proposal_get_vote(
                    handle,
                    proposal_id,
                ));
                assert_invalid_parameter(dash_sdk_multiple_choice_proposal_get_kept_choice(
                    handle,
                    proposal_id,
                ));
                assert_invalid_parameter(dash_sdk_multiple_choice_proposal_get_masternode_vote(
                    handle,
                    pro_tx_hash.as_ptr(),
                    proposal_id,
                ));
                assert_invalid_parameter(
                    dash_sdk_multiple_choice_proposal_get_choice_fund_to_join(handle, proposal_id),
                );
            }
            assert_invalid_parameter(dash_sdk_multiple_choice_proposal_list_by_owner(
                handle,
                id.as_ptr(),
                0,
                not_base58.as_ptr(),
            ));
            assert_invalid_parameter(dash_sdk_multiple_choice_proposal_list_by_created_at(
                handle,
                0,
                not_base58.as_ptr(),
            ));
            assert_invalid_parameter(dash_sdk_multiple_choice_proposal_get_masternode_vote(
                handle,
                std::ptr::null(),
                id.as_ptr(),
            ));

            let no_sdk = std::ptr::null();
            assert_invalid_parameter(dash_sdk_multiple_choice_proposal_fetch(no_sdk, id.as_ptr()));
            assert_invalid_parameter(dash_sdk_multiple_choice_proposal_list_by_owner(
                no_sdk,
                id.as_ptr(),
                0,
                std::ptr::null(),
            ));
            assert_invalid_parameter(dash_sdk_multiple_choice_proposal_list_by_created_at(
                no_sdk,
                0,
                std::ptr::null(),
            ));
            assert_invalid_parameter(dash_sdk_multiple_choice_proposal_get_vote(
                no_sdk,
                id.as_ptr(),
            ));
            assert_invalid_parameter(dash_sdk_multiple_choice_proposal_get_kept_choice(
                no_sdk,
                id.as_ptr(),
            ));
            assert_invalid_parameter(dash_sdk_multiple_choice_proposal_get_masternode_vote(
                no_sdk,
                pro_tx_hash.as_ptr(),
                id.as_ptr(),
            ));
            assert_invalid_parameter(dash_sdk_multiple_choice_proposal_get_processing_fee(no_sdk));
            assert_invalid_parameter(dash_sdk_multiple_choice_proposal_get_choice_fund_to_join(
                no_sdk,
                id.as_ptr(),
            ));
            destroy_mock_sdk_handle(handle);
        }
    }

    /// The vote reads every argument before it reaches the network.
    #[test]
    fn should_refuse_a_vote_with_unreadable_arguments() {
        let handle = create_mock_sdk_handle();
        let proposal_id = base58(Identifier::new([4; 32]));
        let choice_id = base58(Identifier::new([9; 32]));
        let not_base58 = CString::new("0OIl").expect("no NUL");
        let pro_tx_hash = [1u8; 32];
        let voting_key = [2u8; 32];
        let zero_key = [0u8; 32];
        unsafe {
            for result in [
                dash_sdk_multiple_choice_proposal_cast_vote(
                    std::ptr::null(),
                    proposal_id.as_ptr(),
                    0,
                    choice_id.as_ptr(),
                    pro_tx_hash.as_ptr(),
                    voting_key.as_ptr(),
                    FFINetwork::Testnet,
                ),
                dash_sdk_multiple_choice_proposal_cast_vote(
                    handle,
                    std::ptr::null(),
                    0,
                    choice_id.as_ptr(),
                    pro_tx_hash.as_ptr(),
                    voting_key.as_ptr(),
                    FFINetwork::Testnet,
                ),
                dash_sdk_multiple_choice_proposal_cast_vote(
                    handle,
                    not_base58.as_ptr(),
                    0,
                    choice_id.as_ptr(),
                    pro_tx_hash.as_ptr(),
                    voting_key.as_ptr(),
                    FFINetwork::Testnet,
                ),
                dash_sdk_multiple_choice_proposal_cast_vote(
                    handle,
                    proposal_id.as_ptr(),
                    0,
                    std::ptr::null(),
                    pro_tx_hash.as_ptr(),
                    voting_key.as_ptr(),
                    FFINetwork::Testnet,
                ),
                dash_sdk_multiple_choice_proposal_cast_vote(
                    handle,
                    proposal_id.as_ptr(),
                    2,
                    choice_id.as_ptr(),
                    pro_tx_hash.as_ptr(),
                    voting_key.as_ptr(),
                    FFINetwork::Testnet,
                ),
                dash_sdk_multiple_choice_proposal_cast_vote(
                    handle,
                    proposal_id.as_ptr(),
                    3,
                    std::ptr::null(),
                    pro_tx_hash.as_ptr(),
                    voting_key.as_ptr(),
                    FFINetwork::Testnet,
                ),
                dash_sdk_multiple_choice_proposal_cast_vote(
                    handle,
                    proposal_id.as_ptr(),
                    1,
                    std::ptr::null(),
                    std::ptr::null(),
                    voting_key.as_ptr(),
                    FFINetwork::Testnet,
                ),
                dash_sdk_multiple_choice_proposal_cast_vote(
                    handle,
                    proposal_id.as_ptr(),
                    1,
                    std::ptr::null(),
                    pro_tx_hash.as_ptr(),
                    std::ptr::null(),
                    FFINetwork::Testnet,
                ),
                dash_sdk_multiple_choice_proposal_cast_vote(
                    handle,
                    proposal_id.as_ptr(),
                    2,
                    std::ptr::null(),
                    pro_tx_hash.as_ptr(),
                    zero_key.as_ptr(),
                    FFINetwork::Testnet,
                ),
            ] {
                assert_invalid_parameter(result);
            }
            destroy_mock_sdk_handle(handle);
        }
    }
}
