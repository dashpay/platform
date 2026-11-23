//! Loading a masternode or evonode to vote for it.
//!
//! rs-sdk's `load_masternode_for_voting` decides which identity a host stores for the
//! masternode: its owner identity when Platform holds one, or else, for a masternode without
//! an owner address (a shared masternode, from protocol version 14), its voter identity when
//! Platform proves that one holds the enabled voting key. This module reads the arguments,
//! runs it, and marshals the decision: which identity, its id and JSON, the voting key's
//! verdict, and a sentence for the host to show.

use crate::sdk::SDKWrapper;
use crate::types::SDKHandle;
use crate::voting::masternode_vote::{pro_tx_hash_from_wire_order, voting_key_problem_message};
use crate::voting::voting_key_check::{
    verdict_and_message, DashSDKVotingAddress, DashSDKVotingKeyVerdict,
};
use crate::{DashSDKResult, FFIError};
use dash_sdk::dpp::dashcore::ProTxHash;
use dash_sdk::dpp::identity::accessors::IdentityGettersV0;
use dash_sdk::dpp::platform_value::Identifier;
use dash_sdk::platform::transition::masternode_vote_keys::{
    load_masternode_for_voting, MasternodeVotingIdentity, MasternodeVotingLoad,
    MasternodeVotingLoadRefusal,
};
use std::ffi::CString;
use std::os::raw::c_char;

/// Which identity `dash_sdk_masternode_load_for_voting` loads for the masternode.
///
/// cbindgen:prefix-with-name
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DashSDKMasternodeVotingLoadKind {
    /// The masternode's owner identity, whose id is the pro_tx_hash. The host stores it and
    /// applies the voting key's verdict as before (`dash_sdk_masternode_voting_key_check_outcome`).
    OwnerIdentity = 0,
    /// The masternode's voter identity: Platform proves there is no owner identity (a
    /// masternode without an owner address, such as a shared masternode) and that the voter
    /// identity holds the enabled voting key. Its id is not the pro_tx_hash; the host keeps
    /// the pro_tx_hash beside it and names the masternode by it in votes.
    VoterIdentity = 1,
    /// Nothing is loaded; `message` says why.
    Refused = 2,
}

/// What `dash_sdk_masternode_load_for_voting` decided.
///
/// The two strings are owned by the caller: free each non-null one with
/// `dash_sdk_string_free`.
#[repr(C)]
#[derive(Debug)]
pub struct DashSDKMasternodeVotingLoad {
    /// Which identity is loaded.
    pub kind: DashSDKMasternodeVotingLoadKind,
    /// The id of the loaded identity, to store its row under. All zero for `Refused`.
    pub identity_id: [u8; 32],
    /// The loaded identity as JSON, the shape `dash_sdk_identity_fetch` returns. Null for
    /// `Refused`.
    pub identity_json: *mut c_char,
    /// Whether the voting key was checked: a voting address was given and the load was not
    /// refused.
    pub has_voting_key_verdict: bool,
    /// The voting key's verdict, meaningful only with `has_voting_key_verdict`. Always `Valid`
    /// for `VoterIdentity`.
    pub voting_key_verdict: DashSDKVotingKeyVerdict,
    /// Why the load was refused, or, with an `Invalid` or `Undetermined` verdict, why the
    /// voting key was; to show the user. Null otherwise.
    pub message: *mut c_char,
}

/// Loads the masternode `voter_pro_tx_hash` for voting, with the voting key of
/// `voting_address` when one is given (rs-sdk's `load_masternode_for_voting`, which holds the
/// rule): its owner identity when Platform holds one, or else its voter identity when Platform
/// proves the owner identity absent and the voter identity holding the enabled voting key.
///
/// # Parameters
/// * `sdk_handle` - Handle to the SDK instance.
/// * `voter_pro_tx_hash` - Pointer to the masternode's 32-byte pro_tx_hash in wire order, as
///   every masternode vote function takes it (see "pro_tx_hash byte order" in
///   `voting/masternode_vote.rs`).
/// * `has_voting_address` - Whether the user gave a voting key.
/// * `voting_address` - Its 20-byte voting address, the hash160 of the voting public key
///   (`dash_sdk_masternode_voting_address_from_private_key`). Read only with
///   `has_voting_address`; pass `{0}` otherwise.
/// * `out_load` - Receives the decision. Need not be initialized.
///
/// # Returns
/// Success with no data and the decision written to `out_load`. An error when nothing is
/// proved: the owner identity could not be fetched, or it is absent and the voter identity
/// could not be fetched; `InvalidParameter` for a null pointer. Nothing is written on an
/// error.
///
/// # Safety
/// `sdk_handle` must be null or a valid `SDKHandle`, `voter_pro_tx_hash` null or point to 32
/// readable bytes, and `out_load` null or point to a writable `DashSDKMasternodeVotingLoad`,
/// for the duration of the call.
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_masternode_load_for_voting(
    sdk_handle: *const SDKHandle,
    voter_pro_tx_hash: *const u8,
    has_voting_address: bool,
    voting_address: DashSDKVotingAddress,
    out_load: *mut DashSDKMasternodeVotingLoad,
) -> DashSDKResult {
    match load_for_voting_inner(
        sdk_handle,
        voter_pro_tx_hash,
        has_voting_address,
        voting_address,
        out_load,
    ) {
        Ok(()) => DashSDKResult::success(std::ptr::null_mut()),
        Err(error) => DashSDKResult::error(error.into()),
    }
}

/// Reads every argument, runs the load, and writes the decision only once its strings are
/// ready, so an error leaves `out_load` untouched and leaks nothing.
unsafe fn load_for_voting_inner(
    sdk_handle: *const SDKHandle,
    voter_pro_tx_hash: *const u8,
    has_voting_address: bool,
    voting_address: DashSDKVotingAddress,
    out_load: *mut DashSDKMasternodeVotingLoad,
) -> Result<(), FFIError> {
    if sdk_handle.is_null() {
        return Err(FFIError::InvalidParameter("sdk_handle is null".to_string()));
    }
    if out_load.is_null() {
        return Err(FFIError::InvalidParameter("out_load is null".to_string()));
    }
    let pro_tx_hash = pro_tx_hash_from_wire_order(voter_pro_tx_hash)?;
    let wrapper = &*(sdk_handle as *const SDKWrapper);
    let load = wrapper.runtime.block_on(load_masternode_for_voting(
        &wrapper.sdk,
        ProTxHash::from_byte_array(pro_tx_hash.to_buffer()),
        has_voting_address.then_some(&voting_address.bytes),
    ))?;
    let decision = LoadDecision::of(load)?;
    out_load.write(decision.into_ffi());
    Ok(())
}

/// The decision with its strings ready but not yet handed to the caller, so that a failure
/// before the write frees them.
#[derive(Debug, PartialEq, Eq)]
struct LoadDecision {
    kind: DashSDKMasternodeVotingLoadKind,
    identity_id: [u8; 32],
    identity_json: Option<CString>,
    voting_key_verdict: Option<DashSDKVotingKeyVerdict>,
    message: Option<CString>,
}

impl LoadDecision {
    fn of(load: MasternodeVotingLoad) -> Result<Self, FFIError> {
        match load {
            MasternodeVotingLoad::Load {
                identity,
                voting_key,
            } => {
                let kind = match identity {
                    MasternodeVotingIdentity::Owner(_) => {
                        DashSDKMasternodeVotingLoadKind::OwnerIdentity
                    }
                    MasternodeVotingIdentity::Voter(_) => {
                        DashSDKMasternodeVotingLoadKind::VoterIdentity
                    }
                };
                let identity = identity.identity();
                let json = serde_json::to_string(identity).map_err(|e| {
                    FFIError::InternalError(format!("Failed to serialize identity: {}", e))
                })?;
                let (verdict, message) = match voting_key.map(verdict_and_message) {
                    Some((verdict, message)) => (Some(verdict), message),
                    None => (None, None),
                };
                Ok(LoadDecision {
                    kind,
                    identity_id: identity.id().to_buffer(),
                    identity_json: Some(c_string(json, "identity JSON")?),
                    voting_key_verdict: verdict,
                    message: message.map(|m| c_string(m, "message")).transpose()?,
                })
            }
            MasternodeVotingLoad::Refused(refusal) => Ok(LoadDecision {
                kind: DashSDKMasternodeVotingLoadKind::Refused,
                identity_id: [0u8; 32],
                identity_json: None,
                voting_key_verdict: None,
                message: Some(c_string(refusal_message(refusal), "message")?),
            }),
        }
    }

    fn into_ffi(self) -> DashSDKMasternodeVotingLoad {
        DashSDKMasternodeVotingLoad {
            kind: self.kind,
            identity_id: self.identity_id,
            identity_json: self
                .identity_json
                .map_or(std::ptr::null_mut(), CString::into_raw),
            has_voting_key_verdict: self.voting_key_verdict.is_some(),
            voting_key_verdict: self
                .voting_key_verdict
                .unwrap_or(DashSDKVotingKeyVerdict::Undetermined),
            message: self.message.map_or(std::ptr::null_mut(), CString::into_raw),
        }
    }
}

fn c_string(text: String, named: &str) -> Result<CString, FFIError> {
    CString::new(text)
        .map_err(|_| FFIError::InternalError(format!("The masternode load {named} has a NUL byte")))
}

/// Why nothing was loaded, phrased for the host.
fn refusal_message(refusal: MasternodeVotingLoadRefusal) -> String {
    match refusal {
        MasternodeVotingLoadRefusal::NoOwnerIdentity { pro_tx_hash } => format!(
            "Platform holds no identity for masternode {}. Either it is not a masternode, or \
             it has no owner address (such as a shared masternode), which has no owner \
             identity: enter its voting private key to load it through its voter identity.",
            Identifier::new(pro_tx_hash.to_byte_array())
        ),
        MasternodeVotingLoadRefusal::NoOwnerIdentityAndUnusableVotingKey {
            pro_tx_hash,
            problem,
        } => format!(
            "Platform holds no owner identity for masternode {}, and the voting key cannot \
             vote for it either. {}",
            Identifier::new(pro_tx_hash.to_byte_array()),
            voting_key_problem_message(problem)
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::test_utils::{create_mock_sdk_handle, destroy_mock_sdk_handle};
    use crate::{dash_sdk_error_free, DashSDKErrorCode};
    use dash_sdk::dpp::identity::v0::IdentityV0;
    use dash_sdk::dpp::identity::Identity;
    use dash_sdk::platform::transition::masternode_vote_keys::{
        voter_identity_id, VotingKeyCheck, VotingKeyProblem,
    };
    use dash_sdk::Error;
    use std::collections::BTreeMap;
    use std::ffi::CStr;
    use std::mem::MaybeUninit;

    const VOTING_ADDRESS: [u8; 20] = [7u8; 20];

    fn pro_tx_hash() -> ProTxHash {
        ProTxHash::from_byte_array(std::array::from_fn(|i| i as u8 + 1))
    }

    fn identity(id: Identifier) -> Identity {
        Identity::V0(IdentityV0 {
            id,
            public_keys: BTreeMap::new(),
            balance: 11,
            revision: 0,
        })
    }

    #[test]
    fn should_refuse_each_null_argument_and_write_nothing() {
        let handle = create_mock_sdk_handle();
        let pro_tx_hash = [1u8; 32];
        let address = DashSDKVotingAddress {
            bytes: VOTING_ADDRESS,
        };
        let mut out = MaybeUninit::<DashSDKMasternodeVotingLoad>::uninit();
        for (sdk_handle, pro_tx_hash, out_load, named) in [
            (
                std::ptr::null(),
                pro_tx_hash.as_ptr(),
                out.as_mut_ptr(),
                "sdk_handle",
            ),
            (
                handle as *const SDKHandle,
                std::ptr::null(),
                out.as_mut_ptr(),
                "voter_pro_tx_hash",
            ),
            (
                handle as *const SDKHandle,
                pro_tx_hash.as_ptr(),
                std::ptr::null_mut(),
                "out_load",
            ),
        ] {
            unsafe {
                let result = dash_sdk_masternode_load_for_voting(
                    sdk_handle,
                    pro_tx_hash,
                    true,
                    address,
                    out_load,
                );
                assert!(result.data.is_null());
                assert!(!result.error.is_null(), "{named}: expected an error");
                let code = (*result.error).code;
                let message = CStr::from_ptr((*result.error).message)
                    .to_string_lossy()
                    .into_owned();
                dash_sdk_error_free(result.error);
                assert_eq!(code, DashSDKErrorCode::InvalidParameter);
                assert!(message.contains(named), "{message} names {named}");
            }
        }
        destroy_mock_sdk_handle(handle);
    }

    /// The mock SDK holds no answer for the owner identity, so nothing is proved: an error,
    /// never a refusal.
    #[test]
    fn should_report_a_failed_owner_fetch_as_an_error() {
        let handle = create_mock_sdk_handle();
        let pro_tx_hash = [1u8; 32];
        let mut out = MaybeUninit::<DashSDKMasternodeVotingLoad>::uninit();
        unsafe {
            let result = dash_sdk_masternode_load_for_voting(
                handle,
                pro_tx_hash.as_ptr(),
                false,
                DashSDKVotingAddress { bytes: [0u8; 20] },
                out.as_mut_ptr(),
            );
            assert!(!result.error.is_null(), "a failed fetch is an error");
            dash_sdk_error_free(result.error);
        }
        destroy_mock_sdk_handle(handle);
    }

    #[test]
    fn should_marshal_a_voter_identity_load_with_a_valid_verdict() {
        let voter_id = voter_identity_id(pro_tx_hash(), &VOTING_ADDRESS);
        let decision = LoadDecision::of(MasternodeVotingLoad::Load {
            identity: MasternodeVotingIdentity::Voter(identity(voter_id)),
            voting_key: Some(VotingKeyCheck::Valid {
                voter_identity: voter_id,
            }),
        })
        .expect("marshalled");
        assert_eq!(
            decision.kind,
            DashSDKMasternodeVotingLoadKind::VoterIdentity
        );
        assert_eq!(decision.identity_id, voter_id.to_buffer());
        assert_eq!(
            decision.voting_key_verdict,
            Some(DashSDKVotingKeyVerdict::Valid)
        );
        assert_eq!(decision.message, None);
        let json = decision.identity_json.expect("identity JSON");
        let value: serde_json::Value =
            serde_json::from_str(json.to_str().expect("utf-8")).expect("JSON");
        assert_eq!(value["balance"], 11);
    }

    /// An owner identity load carries the voting key's verdict and its message as the check
    /// does, so the host keeps applying its rules to it.
    #[test]
    fn should_marshal_an_owner_identity_load_with_the_verdict_and_its_message() {
        let owner_id = Identifier::new(pro_tx_hash().to_byte_array());
        let problem = VotingKeyProblem::NoUsableVotingKey {
            voter_identity: voter_identity_id(pro_tx_hash(), &VOTING_ADDRESS),
        };
        let decision = LoadDecision::of(MasternodeVotingLoad::Load {
            identity: MasternodeVotingIdentity::Owner(identity(owner_id)),
            voting_key: Some(VotingKeyCheck::Invalid(problem.clone())),
        })
        .expect("marshalled");
        assert_eq!(
            decision.kind,
            DashSDKMasternodeVotingLoadKind::OwnerIdentity
        );
        assert_eq!(decision.identity_id, owner_id.to_buffer());
        assert_eq!(
            decision.voting_key_verdict,
            Some(DashSDKVotingKeyVerdict::Invalid)
        );
        assert_eq!(
            decision.message,
            Some(CString::new(voting_key_problem_message(problem)).expect("no NUL"))
        );

        let decision = LoadDecision::of(MasternodeVotingLoad::Load {
            identity: MasternodeVotingIdentity::Owner(identity(owner_id)),
            voting_key: Some(VotingKeyCheck::Undetermined(Box::new(Error::Generic(
                "timeout".to_string(),
            )))),
        })
        .expect("marshalled");
        assert_eq!(
            decision.voting_key_verdict,
            Some(DashSDKVotingKeyVerdict::Undetermined)
        );
        assert!(decision.message.is_some());

        let decision = LoadDecision::of(MasternodeVotingLoad::Load {
            identity: MasternodeVotingIdentity::Owner(identity(owner_id)),
            voting_key: None,
        })
        .expect("marshalled");
        assert_eq!(decision.voting_key_verdict, None);
        assert_eq!(decision.message, None);
        assert!(decision.identity_json.is_some());
    }

    #[test]
    fn should_marshal_a_refusal_with_its_reason_and_no_identity() {
        for (refusal, says) in [
            (
                MasternodeVotingLoadRefusal::NoOwnerIdentity {
                    pro_tx_hash: pro_tx_hash(),
                },
                "voting private key",
            ),
            (
                MasternodeVotingLoadRefusal::NoOwnerIdentityAndUnusableVotingKey {
                    pro_tx_hash: pro_tx_hash(),
                    problem: VotingKeyProblem::NoVoterIdentity {
                        pro_tx_hash: pro_tx_hash(),
                        expected_voter_identity: voter_identity_id(pro_tx_hash(), &VOTING_ADDRESS),
                    },
                },
                "No voting identity exists",
            ),
        ] {
            let decision =
                LoadDecision::of(MasternodeVotingLoad::Refused(refusal)).expect("marshalled");
            assert_eq!(decision.kind, DashSDKMasternodeVotingLoadKind::Refused);
            assert_eq!(decision.identity_id, [0u8; 32]);
            assert_eq!(decision.identity_json, None);
            assert_eq!(decision.voting_key_verdict, None);
            let message = decision.message.expect("a reason");
            let message = message.to_str().expect("utf-8");
            assert!(message.contains(says), "{message}");
            assert!(!message.contains("  "), "whitespace run: {message}");
        }
    }

    /// The decision crosses the C ABI with its strings owned by the caller, and no verdict as
    /// `has_voting_key_verdict` false.
    #[test]
    fn should_hand_the_strings_to_the_caller() {
        let ffi = LoadDecision::of(MasternodeVotingLoad::Refused(
            MasternodeVotingLoadRefusal::NoOwnerIdentity {
                pro_tx_hash: pro_tx_hash(),
            },
        ))
        .expect("marshalled")
        .into_ffi();
        assert!(!ffi.has_voting_key_verdict);
        assert!(ffi.identity_json.is_null());
        assert!(!ffi.message.is_null());
        unsafe { crate::dash_sdk_string_free(ffi.message) };
    }
}
