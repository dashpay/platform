//! Decision proposal writes: building a proposal to create, giving a stored proposal new
//! text, the properties the platform wallet puts, and a masternode's vote on a proposal. The
//! module docs of [`super`] show how they chain into the document put functions.

use super::json::properties_json;
use super::{json_result, optional_str, required_identifier, required_str, sdk_wrapper};
use crate::document::DashSDKDocumentCreateResult;
use crate::types::{DashSDKResultDataType, DocumentHandle, FFINetwork, SDKHandle};
use crate::voting::masternode_vote::MasternodeVoter;
use crate::{DashSDKResult, FFIError};
use dash_sdk::dpp::decision_proposal::{
    DECISION_PROPOSALS_CONTRACT_ID, PROPOSAL_DOCUMENT_TYPE_NAME,
};
use dash_sdk::dpp::document::Document;
use dash_sdk::dpp::platform_value::string_encoding::Encoding;
use dash_sdk::dpp::voting::vote_choices::yes_no_abstain_vote_choice::YesNoAbstainVoteChoice;
use dash_sdk::platform::decision_proposals::{
    build_decision_proposal_edit, DecisionProposalDocument, DecisionProposalEdit,
    DecisionProposalInput,
};
use std::ffi::{c_char, c_void, CString};
use std::sync::OnceLock;

/// What a new decision proposal asks and the rule it is decided by
/// (`dash_sdk_decision_proposal_build`). Platform refuses values outside the limits noted
/// here when the proposal is created; nothing is checked before.
#[repr(C)]
pub struct DashSDKDecisionProposalParams {
    /// The question, in one line: 1 to 128 characters. NUL-terminated UTF-8, required.
    pub title: *const c_char,
    /// What a yes and a no mean: 1 to 4096 characters. NUL-terminated UTF-8, required.
    pub description: *const c_char,
    /// Where the proposal is discussed, a URI of at most 512 characters; null for none.
    pub url: *const c_char,
    /// The proposal passes when the yes voting power is strictly more than this percent of
    /// the yes plus no voting power: 50 to 80.
    pub pass_percent: u8,
    /// The least yes plus no voting power the proposal needs, as a percent of the masternode
    /// list's total voting power when it closes, rounded up: 0 to 50.
    pub minimum_turnout_percent: u8,
    /// When voting closes, in milliseconds since the Unix epoch: from a week to four weeks
    /// after the block that creates the proposal, which is a little later than now.
    pub end_date_ms: u64,
}

/// New text for a stored decision proposal (`dash_sdk_decision_proposal_build_edit`). A
/// field left null keeps its value. Platform accepts the edit for a day after the proposal
/// was created; the rule and the end date never change.
#[repr(C)]
pub struct DashSDKDecisionProposalEdit {
    /// The new title, NUL-terminated UTF-8; null keeps it.
    pub title: *const c_char,
    /// The new description, NUL-terminated UTF-8; null keeps it.
    pub description: *const c_char,
    /// The new url, NUL-terminated UTF-8; null keeps it, unless `remove_url` is set.
    pub url: *const c_char,
    /// Removes the url. Setting it together with a non-null `url` is refused.
    pub remove_url: bool,
}

/// The answer of a masternode's vote on a decision proposal, the `vote_choice` byte of
/// `dash_sdk_decision_proposal_cast_vote`, in the order of `YesNoAbstainVoteChoice`:
/// `0` = yes, `1` = no, `2` = abstain.
///
/// The FFI parameter is a plain `u8`, not this enum: a C caller can pass any byte, and an
/// out-of-range value read as a `#[repr(u8)]` enum is undefined behavior. The byte is
/// compared with these discriminants, and any other value is refused with
/// `InvalidParameter`. Not exported to the C header for that reason; Swift and Kotlin mirror
/// the three values.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum YesNoVoteChoiceFFI {
    Yes = 0,
    No = 1,
    Abstain = 2,
}

/// The vote choice the `vote_choice` byte names.
fn vote_choice_from_ffi(vote_choice: u8) -> Result<YesNoAbstainVoteChoice, FFIError> {
    match vote_choice {
        x if x == YesNoVoteChoiceFFI::Yes as u8 => Ok(YesNoAbstainVoteChoice::Yes),
        x if x == YesNoVoteChoiceFFI::No as u8 => Ok(YesNoAbstainVoteChoice::No),
        x if x == YesNoVoteChoiceFFI::Abstain as u8 => Ok(YesNoAbstainVoteChoice::Abstain),
        other => Err(FFIError::InvalidParameter(format!(
            "vote_choice must be 0 (yes), 1 (no) or 2 (abstain); got {other}"
        ))),
    }
}

/// The id of the decision proposals system contract, base58: the `data_contract_id` the
/// document put functions take for a proposal.
///
/// # Returns
/// A static NUL-terminated string. Do not free it.
#[no_mangle]
pub extern "C" fn dash_sdk_decision_proposal_contract_id() -> *const c_char {
    static CONTRACT_ID: OnceLock<CString> = OnceLock::new();
    CONTRACT_ID
        .get_or_init(|| {
            CString::new(DECISION_PROPOSALS_CONTRACT_ID.to_string(Encoding::Base58))
                .expect("base58 has no NUL byte")
        })
        .as_ptr()
}

/// Writes the 32 bytes of the decision proposals system contract id to `out_contract_id`:
/// the `contract_id` the platform wallet's document functions take for a proposal.
///
/// # Returns
/// `false` when `out_contract_id` is null, `true` once the id is written.
///
/// # Safety
/// `out_contract_id` must be null or point to 32 writable bytes.
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_decision_proposal_contract_id_bytes(
    out_contract_id: *mut u8,
) -> bool {
    if out_contract_id.is_null() {
        return false;
    }
    std::slice::from_raw_parts_mut(out_contract_id, 32)
        .copy_from_slice(DECISION_PROPOSALS_CONTRACT_ID.as_slice());
    true
}

/// The document type name of a proposal (`"proposal"`), the `document_type_name` the
/// document put and platform wallet functions take.
///
/// # Returns
/// A static NUL-terminated string. Do not free it.
#[no_mangle]
pub extern "C" fn dash_sdk_decision_proposal_document_type_name() -> *const c_char {
    static DOCUMENT_TYPE_NAME: OnceLock<CString> = OnceLock::new();
    DOCUMENT_TYPE_NAME
        .get_or_init(|| {
            CString::new(PROPOSAL_DOCUMENT_TYPE_NAME).expect("the type name has no NUL byte")
        })
        .as_ptr()
}

/// Builds a decision proposal of `owner_identity_id`: a `proposal` document of the decision
/// proposals contract asking what `params` says, with a fresh entropy. Nothing is broadcast.
///
/// Put the document with `dash_sdk_document_put_to_platform_and_wait(sdk_handle,
/// result.document_handle, dash_sdk_decision_proposal_contract_id(),
/// dash_sdk_decision_proposal_document_type_name(), &result.entropy, key, signer, ...)`, or
/// pass `dash_sdk_decision_proposal_properties_json(result.document_handle)` to the platform
/// wallet's `platform_wallet_create_document_with_signer`. The create states the decision
/// proposal fund (0.5 Dash), paid by the owner on top of the fees. Platform refuses a rule
/// outside the contract's limits and an end date less than a week or more than four weeks
/// after the block that creates the proposal. The document id is a placeholder until the put
/// returns the stored document.
///
/// Reads the decision proposals contract from the context provider, or fetches it; a network
/// below protocol version 15 has no such contract, which is an error.
///
/// # Parameters
/// * `sdk_handle` - Handle to the SDK instance.
/// * `owner_identity_id` - The proposer's identity id, base58.
/// * `params` - The question and its rule.
///
/// # Returns
/// On success `data` points to a `DashSDKDocumentCreateResult` (`data_type` is `NoData`, as
/// for `dash_sdk_document_create`): the document handle and the 32-byte entropy the put
/// must reuse. Free the struct with `dash_sdk_document_create_result_free`, which leaves the
/// document handle alive, and the handle with `dash_sdk_document_handle_destroy`.
///
/// # Safety
/// - `sdk_handle` must be a valid pointer to an initialized `SDKHandle`.
/// - `owner_identity_id` must be a valid pointer to a NUL-terminated string.
/// - `params` must be a valid pointer to a `DashSDKDecisionProposalParams` whose strings are
///   NUL-terminated (`url` may be null). All must stay valid for the duration of the call.
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_decision_proposal_build(
    sdk_handle: *const SDKHandle,
    owner_identity_id: *const c_char,
    params: *const DashSDKDecisionProposalParams,
) -> DashSDKResult {
    match build_inner(sdk_handle, owner_identity_id, params) {
        Ok(built) => create_result(built),
        Err(error) => DashSDKResult::error(error.into()),
    }
}

/// `built` as `dash_sdk_document_create` returns a document to create: a
/// `DashSDKDocumentCreateResult` holding the document handle and the entropy.
fn create_result(built: DecisionProposalDocument) -> DashSDKResult {
    let document_handle = Box::into_raw(Box::new(built.document)) as *mut DocumentHandle;
    let create_result = Box::new(DashSDKDocumentCreateResult {
        document_handle,
        entropy: built.entropy.0,
    });
    DashSDKResult::success(Box::into_raw(create_result) as *mut c_void)
}

unsafe fn build_inner(
    sdk_handle: *const SDKHandle,
    owner_identity_id: *const c_char,
    params: *const DashSDKDecisionProposalParams,
) -> Result<DecisionProposalDocument, FFIError> {
    let wrapper = sdk_wrapper(sdk_handle)?;
    let owner_id = required_identifier(owner_identity_id, "owner_identity_id")?;
    let input = proposal_input(params)?;
    Ok(wrapper
        .runtime
        .block_on(wrapper.sdk.build_decision_proposal(owner_id, &input))?)
}

/// The proposal input `params` holds.
///
/// # Safety
/// `params` must be null or point to a `DashSDKDecisionProposalParams` whose string fields
/// are null or NUL-terminated.
unsafe fn proposal_input(
    params: *const DashSDKDecisionProposalParams,
) -> Result<DecisionProposalInput, FFIError> {
    if params.is_null() {
        return Err(FFIError::InvalidParameter("params is null".to_string()));
    }
    let params = &*params;
    Ok(DecisionProposalInput {
        title: required_str(params.title, "title")?.to_string(),
        description: required_str(params.description, "description")?.to_string(),
        url: optional_str(params.url, "url")?.map(str::to_string),
        pass_percent: params.pass_percent,
        minimum_turnout_percent: params.minimum_turnout_percent,
        end_date: params.end_date_ms,
    })
}

/// Fetches a decision proposal by id as a document handle, the stored proposal
/// `dash_sdk_decision_proposal_build_edit` edits.
///
/// # Parameters
/// * `sdk_handle` - Handle to the SDK instance.
/// * `proposal_id` - The proposal's id, base58.
///
/// # Returns
/// A document handle (`data_type` `ResultDocumentHandle`), or no data when there is no such
/// proposal. Free the handle with `dash_sdk_document_handle_destroy`.
///
/// # Safety
/// - `sdk_handle` must be a valid pointer to an initialized `SDKHandle`.
/// - `proposal_id` must be a valid pointer to a NUL-terminated string for the duration of the
///   call.
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_decision_proposal_fetch_document(
    sdk_handle: *const SDKHandle,
    proposal_id: *const c_char,
) -> DashSDKResult {
    match fetch_document_inner(sdk_handle, proposal_id) {
        Ok(Some(document)) => document_handle_result(document),
        Ok(None) => DashSDKResult::success(std::ptr::null_mut()),
        Err(error) => DashSDKResult::error(error.into()),
    }
}

unsafe fn fetch_document_inner(
    sdk_handle: *const SDKHandle,
    proposal_id: *const c_char,
) -> Result<Option<Document>, FFIError> {
    let wrapper = sdk_wrapper(sdk_handle)?;
    let proposal_id = required_identifier(proposal_id, "proposal_id")?;
    Ok(wrapper
        .runtime
        .block_on(wrapper.sdk.fetch_decision_proposal(proposal_id))?)
}

fn document_handle_result(document: Document) -> DashSDKResult {
    DashSDKResult::success_handle(
        Box::into_raw(Box::new(document)) as *mut c_void,
        DashSDKResultDataType::ResultDocumentHandle,
    )
}

/// Gives a stored decision proposal new text: a copy of `proposal` with the text `edit`
/// sets and its next revision. Nothing is broadcast and `proposal` is left as it is.
///
/// rs-sdk refuses, with `InvalidParameter`, a document that is not a `proposal` of the
/// decision proposals contract (as `data_contract_id` and `document_type_name` name it), an
/// edit that leaves the text as it is (a replace that changes nothing still pays its fees),
/// and a document past its last revision. Platform refuses the replace more than a day after
/// the proposal was created, and any change to its rule or end date.
///
/// Put the edited document with `dash_sdk_document_put_to_platform_and_wait`: its revision is
/// above 1, so it is put as a replace, and the `entropy` argument is not read (pass any 32
/// bytes). Do not use `dash_sdk_document_replace_on_platform_and_wait`, which moves the
/// revision once more and is refused. For the platform wallet, pass
/// `dash_sdk_decision_proposal_properties_json` of the edited document to
/// `platform_wallet_document_replace` with the proposal's id; it fetches the stored proposal
/// and moves it to the same revision.
///
/// # Parameters
/// * `proposal` - The proposal as stored, from `dash_sdk_decision_proposal_fetch_document`.
/// * `data_contract_id` - The contract the document belongs to, base58
///   (`dash_sdk_decision_proposal_contract_id`).
/// * `document_type_name` - The document's type (`dash_sdk_decision_proposal_document_type_name`).
/// * `edit` - The new text.
///
/// # Returns
/// A new document handle (`data_type` `ResultDocumentHandle`). Free it with
/// `dash_sdk_document_handle_destroy`; the caller still owns and frees `proposal`.
///
/// # Safety
/// - `proposal` must be a valid document handle from this SDK.
/// - `data_contract_id` and `document_type_name` must be valid pointers to NUL-terminated
///   strings.
/// - `edit` must be a valid pointer to a `DashSDKDecisionProposalEdit` whose string fields are
///   null or NUL-terminated. All must stay valid for the duration of the call.
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_decision_proposal_build_edit(
    proposal: *const DocumentHandle,
    data_contract_id: *const c_char,
    document_type_name: *const c_char,
    edit: *const DashSDKDecisionProposalEdit,
) -> DashSDKResult {
    match build_edit_inner(proposal, data_contract_id, document_type_name, edit) {
        Ok(document) => document_handle_result(document),
        Err(error) => DashSDKResult::error(error.into()),
    }
}

unsafe fn build_edit_inner(
    proposal: *const DocumentHandle,
    data_contract_id: *const c_char,
    document_type_name: *const c_char,
    edit: *const DashSDKDecisionProposalEdit,
) -> Result<Document, FFIError> {
    if proposal.is_null() {
        return Err(FFIError::InvalidParameter("proposal is null".to_string()));
    }
    let proposal = &*(proposal as *const Document);
    let data_contract_id = required_identifier(data_contract_id, "data_contract_id")?;
    let document_type_name = required_str(document_type_name, "document_type_name")?;
    let edit = proposal_edit(edit)?;
    // The edit reads nothing but its arguments, so whatever it refuses is the caller's input.
    build_decision_proposal_edit(&data_contract_id, document_type_name, proposal, &edit)
        .map_err(|e| FFIError::InvalidParameter(e.to_string()))
}

/// The edit `edit` holds.
///
/// # Safety
/// `edit` must be null or point to a `DashSDKDecisionProposalEdit` whose string fields are
/// null or NUL-terminated.
unsafe fn proposal_edit(
    edit: *const DashSDKDecisionProposalEdit,
) -> Result<DecisionProposalEdit, FFIError> {
    if edit.is_null() {
        return Err(FFIError::InvalidParameter("edit is null".to_string()));
    }
    let edit = &*edit;
    let url = match (optional_str(edit.url, "url")?, edit.remove_url) {
        (Some(_), true) => {
            return Err(FFIError::InvalidParameter(
                "url and remove_url cannot both be set".to_string(),
            ))
        }
        (Some(url), false) => Some(Some(url.to_string())),
        (None, true) => Some(None),
        (None, false) => None,
    };
    Ok(DecisionProposalEdit {
        title: optional_str(edit.title, "title")?.map(str::to_string),
        description: optional_str(edit.description, "description")?.map(str::to_string),
        url,
    })
}

/// The properties of a proposal document as a JSON object: the `properties_json` the platform
/// wallet's `platform_wallet_create_document_with_signer` (for a document from
/// `dash_sdk_decision_proposal_build`) and `platform_wallet_document_replace` (for one from
/// `dash_sdk_decision_proposal_build_edit`) take, for example
/// `{"title": "...", "description": "...", "url": "...", "passPercent": 60,
/// "minimumTurnoutPercent": 10, "endDate": 1701209600000}`. A proposal without a url has no
/// `url` key.
///
/// # Returns
/// A JSON string. Free it with `dash_sdk_string_free`; the caller still owns `document`.
///
/// # Safety
/// `document` must be a valid document handle from this SDK.
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_decision_proposal_properties_json(
    document: *const DocumentHandle,
) -> DashSDKResult {
    if document.is_null() {
        return DashSDKResult::error(
            FFIError::InvalidParameter("document is null".to_string()).into(),
        );
    }
    json_result(properties_json(&*(document as *const Document)).map(Some))
}

/// Votes on a decision proposal for a masternode: builds the vote on the proposal's yes/no
/// poll, signs it with the masternode voting key derived from `voting_private_key`,
/// broadcasts it for `voter_pro_tx_hash` and waits for the response. Every argument is read
/// before anything reaches the network. A masternode may vote 5 times on a proposal, but not
/// give the answer it already gave; polls exist from protocol version 15.
///
/// The signer, the voting key and the voter identity are derived exactly as
/// `dash_sdk_contested_resource_cast_vote` derives them, and a refused voting key is
/// explained the same way.
///
/// # Parameters
/// * `sdk_handle` - Handle to the SDK instance.
/// * `proposal_id` - The proposal's id, base58.
/// * `vote_choice` - `0` = yes, `1` = no, `2` = abstain. Any other value is refused with
///   `InvalidParameter`.
/// * `voter_pro_tx_hash` - Pointer to the masternode's 32-byte pro_tx_hash in wire order: the
///   orientation `Txid` stores, which a parsed ProRegTx yields and a wallet holds. This is NOT
///   the byte order of the hex Core displays, which is its reverse. The bytes are reversed
///   here, as `dash_sdk_contested_resource_cast_vote` reverses them; passing display order
///   names a voter identity that never existed.
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
/// - `proposal_id` must be a valid pointer to a NUL-terminated string.
/// - `voter_pro_tx_hash` and `voting_private_key` must each point to 32 readable bytes. All
///   must stay valid for the duration of the call.
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_decision_proposal_cast_vote(
    sdk_handle: *const SDKHandle,
    proposal_id: *const c_char,
    vote_choice: u8,
    voter_pro_tx_hash: *const u8,
    voting_private_key: *const u8,
    network: FFINetwork,
) -> DashSDKResult {
    match cast_vote_inner(
        sdk_handle,
        proposal_id,
        vote_choice,
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
    voter_pro_tx_hash: *const u8,
    voting_private_key: *const u8,
    network: FFINetwork,
) -> Result<(), FFIError> {
    let wrapper = sdk_wrapper(sdk_handle)?;
    let proposal_id = required_identifier(proposal_id, "proposal_id")?;
    let vote_choice = vote_choice_from_ffi(vote_choice)?;
    let voter = MasternodeVoter::from_raw(voter_pro_tx_hash, voting_private_key, network)?;
    wrapper.runtime.block_on(async {
        let vote = wrapper
            .sdk
            .decision_proposal_vote(proposal_id, vote_choice)
            .await?;
        voter.put_vote(&wrapper.sdk, &vote).await
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::decision_proposal::json::tests::{
        input, proposals_contract, stored_proposal, END_DATE,
    };
    use crate::decision_proposal::json::vote_choice_name;
    use crate::document::{build_document_from_properties, parse_document_properties_json};
    use crate::test_utils::test_utils::{create_mock_sdk_handle, destroy_mock_sdk_handle};
    use crate::{
        dash_sdk_document_create_result_free, dash_sdk_document_handle_destroy,
        dash_sdk_error_free, dash_sdk_string_free, DashSDKErrorCode,
    };
    use dash_sdk::dpp::data_contract::accessors::v0::DataContractV0Getters;
    use dash_sdk::dpp::document::{DocumentV0Getters, DocumentV0Setters};
    use dash_sdk::dpp::platform_value::{Bytes32, Identifier};
    use dash_sdk::dpp::version::PlatformVersion;
    use dash_sdk::platform::decision_proposals::{DecisionProposal, DecisionProposalInput};
    use std::ffi::CStr;

    /// `result`'s error code; the error is freed.
    unsafe fn error_code(result: DashSDKResult) -> DashSDKErrorCode {
        assert!(result.data.is_null());
        assert!(!result.error.is_null(), "expected an error");
        let code = (*result.error).code;
        dash_sdk_error_free(result.error);
        code
    }

    /// The document `result` holds as a handle; the handle is freed.
    unsafe fn take_document(result: DashSDKResult) -> Document {
        assert!(result.error.is_null(), "{:?}", error_message(&result));
        assert_eq!(
            result.data_type,
            DashSDKResultDataType::ResultDocumentHandle
        );
        let document = (*(result.data as *const Document)).clone();
        dash_sdk_document_handle_destroy(result.data as *mut DocumentHandle);
        document
    }

    unsafe fn error_message(result: &DashSDKResult) -> Option<String> {
        (!result.error.is_null()).then(|| {
            CStr::from_ptr((*result.error).message)
                .to_string_lossy()
                .into_owned()
        })
    }

    /// The string `result` holds; the string is freed.
    unsafe fn take_string(result: DashSDKResult) -> String {
        assert!(result.error.is_null(), "{:?}", error_message(&result));
        assert_eq!(result.data_type, DashSDKResultDataType::String);
        let string = CStr::from_ptr(result.data as *const c_char)
            .to_str()
            .expect("UTF-8")
            .to_string();
        dash_sdk_string_free(result.data as *mut c_char);
        string
    }

    fn c_string(value: &str) -> CString {
        CString::new(value).expect("no NUL")
    }

    fn no_edit() -> DashSDKDecisionProposalEdit {
        DashSDKDecisionProposalEdit {
            title: std::ptr::null(),
            description: std::ptr::null(),
            url: std::ptr::null(),
            remove_url: false,
        }
    }

    /// `edit` applied by the FFI to `proposal`, named as a decision proposal.
    unsafe fn edit_proposal(
        proposal: &Document,
        edit: &DashSDKDecisionProposalEdit,
    ) -> DashSDKResult {
        dash_sdk_decision_proposal_build_edit(
            proposal as *const Document as *const DocumentHandle,
            dash_sdk_decision_proposal_contract_id(),
            dash_sdk_decision_proposal_document_type_name(),
            edit,
        )
    }

    #[test]
    fn should_name_the_decision_proposals_contract_and_the_proposal_type() {
        let contract = proposals_contract();
        unsafe {
            assert_eq!(
                CStr::from_ptr(dash_sdk_decision_proposal_contract_id())
                    .to_str()
                    .expect("UTF-8"),
                contract.id().to_string(Encoding::Base58)
            );
            assert_eq!(
                CStr::from_ptr(dash_sdk_decision_proposal_document_type_name())
                    .to_str()
                    .expect("UTF-8"),
                "proposal"
            );
            assert!(contract.document_type_for_name("proposal").is_ok());
            let mut bytes = [0u8; 32];
            assert!(dash_sdk_decision_proposal_contract_id_bytes(
                bytes.as_mut_ptr()
            ));
            assert_eq!(Identifier::new(bytes), contract.id());
            assert!(!dash_sdk_decision_proposal_contract_id_bytes(
                std::ptr::null_mut()
            ));
        }
    }

    #[test]
    fn should_map_each_vote_choice_byte_and_refuse_the_rest() {
        assert_eq!(
            vote_choice_from_ffi(0).ok(),
            Some(YesNoAbstainVoteChoice::Yes)
        );
        assert_eq!(
            vote_choice_from_ffi(1).ok(),
            Some(YesNoAbstainVoteChoice::No)
        );
        assert_eq!(
            vote_choice_from_ffi(2).ok(),
            Some(YesNoAbstainVoteChoice::Abstain)
        );
        for refused in [3, 99, u8::MAX] {
            assert!(matches!(
                vote_choice_from_ffi(refused),
                Err(FFIError::InvalidParameter(_))
            ));
        }
        // The byte and the name a masternode's answer is read back with agree
        for (byte, name) in [(0, "yes"), (1, "no"), (2, "abstain")] {
            assert_eq!(
                vote_choice_name(vote_choice_from_ffi(byte).expect("a choice")),
                name
            );
        }
    }

    #[test]
    fn should_read_the_proposal_params() {
        let title = c_string("Raise the block size?");
        let description = c_string("Yes doubles it; no keeps it.");
        let url = c_string("https://forum.dash.org/t/block-size");
        let mut params = DashSDKDecisionProposalParams {
            title: title.as_ptr(),
            description: description.as_ptr(),
            url: url.as_ptr(),
            pass_percent: 60,
            minimum_turnout_percent: 10,
            end_date_ms: END_DATE,
        };
        unsafe {
            assert_eq!(proposal_input(&params).expect("reads"), input());
            params.url = std::ptr::null();
            assert_eq!(proposal_input(&params).expect("reads").url, None);
            params.title = std::ptr::null();
            assert!(matches!(
                proposal_input(&params),
                Err(FFIError::InvalidParameter(_))
            ));
            assert!(matches!(
                proposal_input(std::ptr::null()),
                Err(FFIError::InvalidParameter(_))
            ));
        }
    }

    /// The build refuses arguments it cannot read before it asks the network anything.
    #[test]
    fn should_refuse_a_build_with_unreadable_arguments() {
        let handle = create_mock_sdk_handle();
        let owner = c_string(&Identifier::new([3; 32]).to_string(Encoding::Base58));
        let not_base58 = c_string("0OIl");
        let title = c_string("Raise the block size?");
        let description = c_string("Yes doubles it; no keeps it.");
        let params = DashSDKDecisionProposalParams {
            title: title.as_ptr(),
            description: description.as_ptr(),
            url: std::ptr::null(),
            pass_percent: 60,
            minimum_turnout_percent: 10,
            end_date_ms: END_DATE,
        };
        let no_title = DashSDKDecisionProposalParams {
            title: std::ptr::null(),
            ..params
        };
        unsafe {
            for result in [
                dash_sdk_decision_proposal_build(std::ptr::null(), owner.as_ptr(), &params),
                dash_sdk_decision_proposal_build(handle, std::ptr::null(), &params),
                dash_sdk_decision_proposal_build(handle, not_base58.as_ptr(), &params),
                dash_sdk_decision_proposal_build(handle, owner.as_ptr(), std::ptr::null()),
                dash_sdk_decision_proposal_build(handle, owner.as_ptr(), &no_title),
            ] {
                assert_eq!(error_code(result), DashSDKErrorCode::InvalidParameter);
            }
            destroy_mock_sdk_handle(handle);
        }
    }

    #[test]
    fn should_give_a_proposal_new_text_and_its_next_revision() {
        let proposal = stored_proposal(&input());
        let title = c_string("Raise the block size to 4 MB?");
        let url = c_string("https://forum.dash.org/t/block-size-4mb");
        unsafe {
            let edited = take_document(edit_proposal(
                &proposal,
                &DashSDKDecisionProposalEdit {
                    title: title.as_ptr(),
                    url: url.as_ptr(),
                    ..no_edit()
                },
            ));
            assert_eq!(edited.id(), proposal.id());
            assert_eq!(edited.revision(), Some(2));
            let read = DecisionProposal::from_document(edited).expect("a proposal");
            assert_eq!(read.title, "Raise the block size to 4 MB?");
            assert_eq!(read.description, input().description);
            assert_eq!(
                read.url.as_deref(),
                Some("https://forum.dash.org/t/block-size-4mb")
            );
            assert_eq!(
                (
                    read.pass_percent,
                    read.minimum_turnout_percent,
                    read.end_date
                ),
                (60, 10, END_DATE)
            );
            // The stored proposal the caller passed is left as it was
            assert_eq!(proposal.revision(), Some(1));

            let without_url = take_document(edit_proposal(
                &proposal,
                &DashSDKDecisionProposalEdit {
                    remove_url: true,
                    ..no_edit()
                },
            ));
            assert_eq!(
                DecisionProposal::from_document(without_url)
                    .expect("a proposal")
                    .url,
                None
            );
        }
    }

    /// rs-sdk refuses these edits; the FFI passes its refusal on as `InvalidParameter`.
    #[test]
    fn should_refuse_an_edit_that_changes_nothing_or_is_not_of_a_proposal() {
        let proposal = stored_proposal(&input());
        let same_title = c_string(&input().title);
        let new_title = c_string("Another question?");
        let same_url = c_string(input().url.as_deref().expect("a url"));
        let other_contract = c_string(&Identifier::new([1; 32]).to_string(Encoding::Base58));
        let other_type = c_string("note");
        unsafe {
            for edit in [
                no_edit(),
                DashSDKDecisionProposalEdit {
                    title: same_title.as_ptr(),
                    ..no_edit()
                },
                DashSDKDecisionProposalEdit {
                    url: same_url.as_ptr(),
                    ..no_edit()
                },
            ] {
                let result = edit_proposal(&proposal, &edit);
                assert!(error_message(&result)
                    .expect("refused")
                    .contains("changes nothing"));
                assert_eq!(error_code(result), DashSDKErrorCode::InvalidParameter);
            }

            let retitle = DashSDKDecisionProposalEdit {
                title: new_title.as_ptr(),
                ..no_edit()
            };
            for (data_contract_id, document_type_name) in [
                (
                    other_contract.as_ptr(),
                    dash_sdk_decision_proposal_document_type_name(),
                ),
                (
                    dash_sdk_decision_proposal_contract_id(),
                    other_type.as_ptr(),
                ),
            ] {
                let result = dash_sdk_decision_proposal_build_edit(
                    &proposal as *const Document as *const DocumentHandle,
                    data_contract_id,
                    document_type_name,
                    &retitle,
                );
                assert!(error_message(&result)
                    .expect("refused")
                    .contains("is not a decision proposal"));
                assert_eq!(error_code(result), DashSDKErrorCode::InvalidParameter);
            }

            let mut last_revision = proposal.clone();
            last_revision.set_revision(Some(u64::MAX));
            assert_eq!(
                error_code(edit_proposal(&last_revision, &retitle)),
                DashSDKErrorCode::InvalidParameter
            );
        }
    }

    #[test]
    fn should_refuse_an_edit_it_cannot_read() {
        let proposal = stored_proposal(&input());
        let url = c_string("https://example.com");
        let not_base58 = c_string("0OIl");
        let contradictory = DashSDKDecisionProposalEdit {
            url: url.as_ptr(),
            remove_url: true,
            ..no_edit()
        };
        let proposal_handle = &proposal as *const Document as *const DocumentHandle;
        unsafe {
            for result in [
                edit_proposal(&proposal, &contradictory),
                dash_sdk_decision_proposal_build_edit(
                    std::ptr::null(),
                    dash_sdk_decision_proposal_contract_id(),
                    dash_sdk_decision_proposal_document_type_name(),
                    &no_edit(),
                ),
                dash_sdk_decision_proposal_build_edit(
                    proposal_handle,
                    not_base58.as_ptr(),
                    dash_sdk_decision_proposal_document_type_name(),
                    &no_edit(),
                ),
                dash_sdk_decision_proposal_build_edit(
                    proposal_handle,
                    dash_sdk_decision_proposal_contract_id(),
                    std::ptr::null(),
                    &no_edit(),
                ),
                dash_sdk_decision_proposal_build_edit(
                    proposal_handle,
                    dash_sdk_decision_proposal_contract_id(),
                    dash_sdk_decision_proposal_document_type_name(),
                    std::ptr::null(),
                ),
            ] {
                assert_eq!(error_code(result), DashSDKErrorCode::InvalidParameter);
            }
        }
    }

    /// The properties JSON, read back the way a document create reads its properties (the
    /// platform wallet's create and `dash_sdk_document_create` share the parse, sanitize and
    /// build steps), gives a document with exactly the properties rs-sdk built, value types
    /// included.
    #[test]
    fn should_write_properties_a_document_create_reads_back_into_the_same_proposal() {
        let platform_version = PlatformVersion::latest();
        let contract = proposals_contract();
        let document_type = contract
            .document_type_for_name("proposal")
            .expect("the proposal type");
        for proposal in [
            stored_proposal(&input()),
            stored_proposal(&DecisionProposalInput {
                url: None,
                ..input()
            }),
        ] {
            unsafe {
                let json = take_string(dash_sdk_decision_proposal_properties_json(
                    &proposal as *const Document as *const DocumentHandle,
                ));
                let properties = parse_document_properties_json(&json)
                    .ok()
                    .expect("an object");
                let created = build_document_from_properties(
                    document_type,
                    properties,
                    proposal.owner_id(),
                    [9; 32],
                    platform_version,
                )
                .expect("builds");
                assert_eq!(created.properties(), proposal.properties(), "{json}");
            }
        }
        unsafe {
            assert_eq!(
                error_code(dash_sdk_decision_proposal_properties_json(std::ptr::null())),
                DashSDKErrorCode::InvalidParameter
            );
        }
    }

    /// The vote reads every argument before it builds the vote, which asks the network for
    /// its protocol version.
    #[test]
    fn should_refuse_a_vote_with_unreadable_arguments() {
        let handle = create_mock_sdk_handle();
        let proposal_id = c_string(&Identifier::new([4; 32]).to_string(Encoding::Base58));
        let not_base58 = c_string("0OIl");
        let pro_tx_hash = [1u8; 32];
        let voting_key = [2u8; 32];
        let zero_key = [0u8; 32];
        unsafe {
            for result in [
                dash_sdk_decision_proposal_cast_vote(
                    std::ptr::null(),
                    proposal_id.as_ptr(),
                    0,
                    pro_tx_hash.as_ptr(),
                    voting_key.as_ptr(),
                    FFINetwork::Testnet,
                ),
                dash_sdk_decision_proposal_cast_vote(
                    handle,
                    std::ptr::null(),
                    0,
                    pro_tx_hash.as_ptr(),
                    voting_key.as_ptr(),
                    FFINetwork::Testnet,
                ),
                dash_sdk_decision_proposal_cast_vote(
                    handle,
                    not_base58.as_ptr(),
                    0,
                    pro_tx_hash.as_ptr(),
                    voting_key.as_ptr(),
                    FFINetwork::Testnet,
                ),
                dash_sdk_decision_proposal_cast_vote(
                    handle,
                    proposal_id.as_ptr(),
                    3,
                    pro_tx_hash.as_ptr(),
                    voting_key.as_ptr(),
                    FFINetwork::Testnet,
                ),
                dash_sdk_decision_proposal_cast_vote(
                    handle,
                    proposal_id.as_ptr(),
                    1,
                    std::ptr::null(),
                    voting_key.as_ptr(),
                    FFINetwork::Testnet,
                ),
                dash_sdk_decision_proposal_cast_vote(
                    handle,
                    proposal_id.as_ptr(),
                    1,
                    pro_tx_hash.as_ptr(),
                    std::ptr::null(),
                    FFINetwork::Testnet,
                ),
                dash_sdk_decision_proposal_cast_vote(
                    handle,
                    proposal_id.as_ptr(),
                    2,
                    pro_tx_hash.as_ptr(),
                    zero_key.as_ptr(),
                    FFINetwork::Testnet,
                ),
            ] {
                assert_eq!(error_code(result), DashSDKErrorCode::InvalidParameter);
            }
            destroy_mock_sdk_handle(handle);
        }
    }

    #[test]
    fn should_refuse_to_fetch_a_document_with_unreadable_arguments() {
        let handle = create_mock_sdk_handle();
        let not_base58 = c_string("0OIl");
        let id = c_string(&Identifier::new([4; 32]).to_string(Encoding::Base58));
        unsafe {
            for result in [
                dash_sdk_decision_proposal_fetch_document(std::ptr::null(), id.as_ptr()),
                dash_sdk_decision_proposal_fetch_document(handle, std::ptr::null()),
                dash_sdk_decision_proposal_fetch_document(handle, not_base58.as_ptr()),
            ] {
                assert_eq!(error_code(result), DashSDKErrorCode::InvalidParameter);
            }
            destroy_mock_sdk_handle(handle);
        }
    }

    /// The built proposal reaches the caller as `dash_sdk_document_create`'s result does, and
    /// frees the same way: the struct, then the document handle it held.
    #[test]
    fn should_hand_a_built_proposal_over_as_a_document_create_result() {
        let document = stored_proposal(&input());
        let result = create_result(DecisionProposalDocument {
            document: document.clone(),
            entropy: Bytes32::new([9; 32]),
        });
        assert!(result.error.is_null());
        assert_eq!(result.data_type, DashSDKResultDataType::NoData);
        unsafe {
            let created = result.data as *mut DashSDKDocumentCreateResult;
            assert_eq!((*created).entropy, [9; 32]);
            let handle = (*created).document_handle;
            dash_sdk_document_create_result_free(created);
            assert_eq!(*(handle as *const Document), document);
            dash_sdk_document_handle_destroy(handle);
        }
    }
}
