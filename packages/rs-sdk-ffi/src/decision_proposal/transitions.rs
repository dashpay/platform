//! Decision proposal writes from rs-sdk-ffi: a masternode's vote on a proposal, and the
//! names of the decision proposals contract and of its `proposal` type. A proposal is
//! created and edited through the platform wallet (`platform_wallet_decision_proposal_create`
//! and `platform_wallet_decision_proposal_edit` in `rs-platform-wallet-ffi`).

use super::{required_identifier, sdk_wrapper};
use crate::types::{FFINetwork, SDKHandle};
use crate::voting::masternode_vote::MasternodeVoter;
use crate::{DashSDKResult, FFIError};
use dash_sdk::dpp::decision_proposal::{
    DECISION_PROPOSALS_CONTRACT_ID, PROPOSAL_DOCUMENT_TYPE_NAME,
};
use dash_sdk::dpp::platform_value::string_encoding::Encoding;
use dash_sdk::dpp::voting::vote_choices::yes_no_abstain_vote_choice::YesNoAbstainVoteChoice;
use std::ffi::{c_char, CString};
use std::sync::OnceLock;

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
/// generic document functions take to read proposals.
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

/// Writes the 32 bytes of the decision proposals system contract id to `out_contract_id`,
/// for the functions that take a contract id as bytes.
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
/// generic document functions take to read proposals.
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
    use crate::decision_proposal::json::tests::proposals_contract;
    use crate::decision_proposal::json::vote_choice_name;
    use crate::test_utils::test_utils::{create_mock_sdk_handle, destroy_mock_sdk_handle};
    use crate::{dash_sdk_error_free, DashSDKErrorCode};
    use dash_sdk::dpp::data_contract::accessors::v0::DataContractV0Getters;
    use dash_sdk::dpp::platform_value::Identifier;
    use std::ffi::CStr;

    /// `result`'s error code; the error is freed.
    unsafe fn error_code(result: DashSDKResult) -> DashSDKErrorCode {
        assert!(result.data.is_null());
        assert!(!result.error.is_null(), "expected an error");
        let code = (*result.error).code;
        dash_sdk_error_free(result.error);
        code
    }

    fn c_string(value: &str) -> CString {
        CString::new(value).expect("no NUL")
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
}
