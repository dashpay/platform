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
use crate::voting::voting_key_check::DashSDKRecordedVotingKey;
use crate::voting::voting_key_check::{
    verdict_and_message, DashSDKVotingAddress, DashSDKVotingKeyVerdict,
};
use crate::{DashSDKResult, FFIError};
use dash_sdk::dpp::dashcore::ProTxHash;
use dash_sdk::dpp::identity::accessors::IdentityGettersV0;
use dash_sdk::dpp::platform_value::Identifier;
use dash_sdk::platform::transition::masternode_vote_keys::{
    load_masternode_for_voting, sibling_voting_key_record, MasternodeVotingIdentity,
    MasternodeVotingLoad, MasternodeVotingLoadRefusal,
};
use std::ffi::CString;
use std::os::raw::c_char;
use std::panic::AssertUnwindSafe;

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
    /// Nothing is loaded; `message` says why. With `has_voting_key_verdict` (then always
    /// `Invalid`), the voting key was refused for the voter identity `identity_id`: a host that
    /// already stores a row for that identity holding the same key records the refusal on it
    /// (`dash_sdk_masternode_voting_key_check_outcome`), and creates or replaces nothing.
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
    /// The id of the loaded identity, to store its row under. For `Refused`, the voter
    /// identity the voting key was refused for when `has_voting_key_verdict`, all zero
    /// otherwise.
    pub identity_id: [u8; 32],
    /// The loaded identity as JSON, the shape `dash_sdk_identity_fetch` returns. Null for
    /// `Refused`.
    pub identity_json: *mut c_char,
    /// Whether the voting key was checked: a voting address was given and the load was not
    /// refused, or the load was refused because of the voting key.
    pub has_voting_key_verdict: bool,
    /// The voting key's verdict, meaningful only with `has_voting_key_verdict`. Always `Valid`
    /// for `VoterIdentity` and `Invalid` for `Refused`.
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
/// could not be fetched; when the worker thread cannot start; `InvalidParameter` for a null
/// pointer. Nothing is written on an error.
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
    // A panic on the worker resumes on this thread; it is caught here so it never unwinds
    // across the C boundary (an unwinding build, such as the JNI's), and comes back as an
    // error with `out_load` unwritten, since the write is the last step.
    let loaded = std::panic::catch_unwind(AssertUnwindSafe(|| {
        load_for_voting_inner(
            sdk_handle,
            voter_pro_tx_hash,
            has_voting_address,
            voting_address,
            out_load,
        )
    }));
    match loaded {
        Ok(Ok(())) => DashSDKResult::success(std::ptr::null_mut()),
        Ok(Err(error)) => DashSDKResult::error(error.into()),
        Err(_) => DashSDKResult::error(
            FFIError::InternalError("The masternode load panicked".to_string()).into(),
        ),
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
    let voting_address = has_voting_address.then_some(voting_address.bytes);
    let load = wrapper.runtime.try_block_on(load_masternode_for_voting(
        &wrapper.sdk,
        ProTxHash::from_byte_array(pro_tx_hash.to_buffer()),
        voting_address.as_ref(),
    ))??;
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
            MasternodeVotingLoad::Load(loaded) => {
                let (kind, identity, verdict, message) = match loaded {
                    MasternodeVotingIdentity::Owner {
                        identity,
                        voting_key,
                    } => {
                        let (verdict, message) = match voting_key.map(verdict_and_message) {
                            Some((verdict, message)) => (Some(verdict), message),
                            None => (None, None),
                        };
                        (
                            DashSDKMasternodeVotingLoadKind::OwnerIdentity,
                            identity,
                            verdict,
                            message,
                        )
                    }
                    // A voter identity is loaded only with its voting key proved valid.
                    MasternodeVotingIdentity::Voter(identity) => (
                        DashSDKMasternodeVotingLoadKind::VoterIdentity,
                        identity,
                        Some(DashSDKVotingKeyVerdict::Valid),
                        None,
                    ),
                };
                let json = serde_json::to_string(&identity).map_err(|e| {
                    FFIError::InternalError(format!("Failed to serialize identity: {}", e))
                })?;
                Ok(LoadDecision {
                    kind,
                    identity_id: identity.id().to_buffer(),
                    identity_json: Some(c_string(json, "identity JSON")?),
                    voting_key_verdict: verdict,
                    message: message.map(|m| c_string(m, "message")).transpose()?,
                })
            }
            MasternodeVotingLoad::Refused(refusal) => {
                let refused_voter_identity = refusal.refused_voter_identity();
                Ok(LoadDecision {
                    kind: DashSDKMasternodeVotingLoadKind::Refused,
                    identity_id: refused_voter_identity.map_or([0u8; 32], |id| id.to_buffer()),
                    identity_json: None,
                    voting_key_verdict: refused_voter_identity
                        .map(|_| DashSDKVotingKeyVerdict::Invalid),
                    message: Some(c_string(refusal_message(refusal), "message")?),
                })
            }
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

/// What a host records for a voting key it stores for the same masternode as a key it just
/// loaded and checked: rs-sdk's `sibling_voting_key_record`, which holds the rule (a valid
/// record of a different key is recorded as undetermined, so it is checked again before it
/// votes; nothing else changes).
///
/// # Parameters
/// * `loaded_verdict` - The verdict of the key just loaded.
/// * `loaded_address` - The voting address of the key just loaded.
/// * `sibling` - The record the host holds for the other key, with its address; `{0}` for none.
///
/// Every argument is passed by value and must be fully initialized.
///
/// # Returns
/// The record to persist for the other key; `check` `None` leaves its record unchanged.
#[no_mangle]
pub extern "C" fn dash_sdk_masternode_sibling_voting_key_record(
    loaded_verdict: DashSDKVotingKeyVerdict,
    loaded_address: DashSDKVotingAddress,
    sibling: DashSDKRecordedVotingKey,
) -> DashSDKRecordedVotingKey {
    sibling_voting_key_record(loaded_verdict.into(), &loaded_address.bytes, sibling.into()).into()
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
        MasternodeVotingLoadRefusal::VoterIdentityGivenAsProTxHash { identity } => format!(
            "Identity {} is a masternode's voter identity, not a masternode. Enter the \
             masternode's ProTxHash as Core displays it.",
            identity
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sdk::SDKWrapper;
    use crate::test_utils::test_utils::{create_mock_sdk_handle, destroy_mock_sdk_handle};
    use crate::voting::voting_key_check::DashSDKRecordedVotingKeyCheck;
    use crate::{dash_sdk_error_free, DashSDKErrorCode};
    use dash_sdk::dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeySettersV0;
    use dash_sdk::dpp::identity::v0::IdentityV0;
    use dash_sdk::dpp::identity::Identity;
    use dash_sdk::platform::transition::masternode_vote_keys::{
        voter_identity_id, voter_identity_voting_key, VotingKeyCheck, VotingKeyProblem,
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
        let mut out = sentinel();
        let out_ptr: *mut DashSDKMasternodeVotingLoad = &mut out;
        for (sdk_handle, pro_tx_hash, out_load, named) in [
            (
                std::ptr::null(),
                pro_tx_hash.as_ptr(),
                out_ptr,
                "sdk_handle",
            ),
            (
                handle as *const SDKHandle,
                std::ptr::null(),
                out_ptr,
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
        assert_untouched(&out);
        destroy_mock_sdk_handle(handle);
    }

    /// An output no call writes, to show an error leaves the caller's value alone.
    fn sentinel() -> DashSDKMasternodeVotingLoad {
        DashSDKMasternodeVotingLoad {
            kind: DashSDKMasternodeVotingLoadKind::VoterIdentity,
            identity_id: [0xee; 32],
            identity_json: std::ptr::null_mut(),
            has_voting_key_verdict: true,
            voting_key_verdict: DashSDKVotingKeyVerdict::Undetermined,
            message: std::ptr::null_mut(),
        }
    }

    fn assert_untouched(out: &DashSDKMasternodeVotingLoad) {
        assert_eq!(out.kind, DashSDKMasternodeVotingLoadKind::VoterIdentity);
        assert_eq!(out.identity_id, [0xee; 32]);
        assert!(out.identity_json.is_null());
        assert!(out.has_voting_key_verdict);
        assert_eq!(
            out.voting_key_verdict,
            DashSDKVotingKeyVerdict::Undetermined
        );
        assert!(out.message.is_null());
    }

    /// The mock SDK holds no answer for the owner identity, so nothing is proved: an error,
    /// never a refusal.
    #[test]
    fn should_report_a_failed_owner_fetch_as_an_error() {
        let handle = create_mock_sdk_handle();
        let pro_tx_hash = [1u8; 32];
        let mut out = sentinel();
        unsafe {
            let result = dash_sdk_masternode_load_for_voting(
                handle,
                pro_tx_hash.as_ptr(),
                false,
                DashSDKVotingAddress { bytes: [0u8; 20] },
                &mut out,
            );
            assert!(!result.error.is_null(), "a failed fetch is an error");
            dash_sdk_error_free(result.error);
        }
        assert_untouched(&out);
        destroy_mock_sdk_handle(handle);
    }

    #[test]
    fn should_marshal_a_voter_identity_load_with_a_valid_verdict() {
        let voter_id = voter_identity_id(pro_tx_hash(), &VOTING_ADDRESS);
        let decision = LoadDecision::of(MasternodeVotingLoad::Load(
            MasternodeVotingIdentity::Voter(identity(voter_id)),
        ))
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
        let decision = LoadDecision::of(MasternodeVotingLoad::Load(
            MasternodeVotingIdentity::Owner {
                identity: identity(owner_id),
                voting_key: Some(VotingKeyCheck::Invalid(problem.clone())),
            },
        ))
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

        let decision = LoadDecision::of(MasternodeVotingLoad::Load(
            MasternodeVotingIdentity::Owner {
                identity: identity(owner_id),
                voting_key: Some(VotingKeyCheck::Undetermined(Box::new(Error::Generic(
                    "timeout".to_string(),
                )))),
            },
        ))
        .expect("marshalled");
        assert_eq!(
            decision.voting_key_verdict,
            Some(DashSDKVotingKeyVerdict::Undetermined)
        );
        assert!(decision.message.is_some());

        let decision = LoadDecision::of(MasternodeVotingLoad::Load(
            MasternodeVotingIdentity::Owner {
                identity: identity(owner_id),
                voting_key: None,
            },
        ))
        .expect("marshalled");
        assert_eq!(decision.voting_key_verdict, None);
        assert_eq!(decision.message, None);
        assert!(decision.identity_json.is_some());
    }

    #[test]
    fn should_marshal_a_refusal_with_its_reason_and_no_identity() {
        let voter = voter_identity_id(pro_tx_hash(), &VOTING_ADDRESS);
        for (refusal, says, refused_for) in [
            (
                MasternodeVotingLoadRefusal::NoOwnerIdentity {
                    pro_tx_hash: pro_tx_hash(),
                },
                "voting private key",
                None,
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
                Some(voter),
            ),
            (
                MasternodeVotingLoadRefusal::NoOwnerIdentityAndUnusableVotingKey {
                    pro_tx_hash: pro_tx_hash(),
                    problem: VotingKeyProblem::NoUsableVotingKey {
                        voter_identity: voter,
                    },
                },
                "no enabled ECDSA_HASH160 voting key",
                Some(voter),
            ),
        ] {
            let decision =
                LoadDecision::of(MasternodeVotingLoad::Refused(refusal)).expect("marshalled");
            assert_eq!(decision.kind, DashSDKMasternodeVotingLoadKind::Refused);
            assert_eq!(decision.identity_json, None);
            // A refused voting key names the voter identity it was refused for, as invalid.
            assert_eq!(
                decision.identity_id,
                refused_for.map_or([0u8; 32], |id: Identifier| id.to_buffer())
            );
            assert_eq!(
                decision.voting_key_verdict,
                refused_for.map(|_| DashSDKVotingKeyVerdict::Invalid)
            );
            let message = decision.message.expect("a reason");
            let message = message.to_str().expect("utf-8");
            assert!(message.contains(says), "{message}");
            assert!(!message.contains("  "), "whitespace run: {message}");
        }
    }

    /// A mock SDK handle that proves `owner` at the pro_tx_hash and, when `voter` is given, that
    /// identity at the voter identity of `VOTING_ADDRESS`. An identity the mock is not told
    /// about fails its fetch.
    fn mock_sdk_proving(
        owner: Option<Identity>,
        voter: Option<Option<Identity>>,
    ) -> *mut SDKHandle {
        let mut wrapper = Box::new(SDKWrapper::new_mock());
        // The FFI entry point blocks on its own runtime, so this one must be gone first.
        let setup_runtime = tokio::runtime::Runtime::new().expect("create setup runtime");
        setup_runtime.block_on(async {
            wrapper
                .sdk
                .mock()
                .expect_fetch::<Identity, _>(Identifier::new(pro_tx_hash().to_byte_array()), owner)
                .await
                .expect("register the owner identity");
            if let Some(voter) = voter {
                wrapper
                    .sdk
                    .mock()
                    .expect_fetch::<Identity, _>(
                        voter_identity_id(pro_tx_hash(), &VOTING_ADDRESS),
                        voter,
                    )
                    .await
                    .expect("register the voter identity");
            }
        });
        drop(setup_runtime);
        Box::into_raw(wrapper) as *mut SDKHandle
    }

    /// Calls the export for `pro_tx_hash()` in wire order, the reverse of its Platform order,
    /// and returns the decision with its strings copied and freed.
    unsafe fn load_through_the_export(
        handle: *mut SDKHandle,
        with_voting_address: bool,
    ) -> (DashSDKMasternodeVotingLoad, Option<String>, Option<String>) {
        let mut wire_order = pro_tx_hash().to_byte_array();
        wire_order.reverse();
        let mut out = MaybeUninit::<DashSDKMasternodeVotingLoad>::uninit();
        let result = dash_sdk_masternode_load_for_voting(
            handle,
            wire_order.as_ptr(),
            with_voting_address,
            DashSDKVotingAddress {
                bytes: if with_voting_address {
                    VOTING_ADDRESS
                } else {
                    [0u8; 20]
                },
            },
            out.as_mut_ptr(),
        );
        if !result.error.is_null() {
            let message = CStr::from_ptr((*result.error).message)
                .to_string_lossy()
                .into_owned();
            dash_sdk_error_free(result.error);
            panic!("the load failed: {message}");
        }
        let mut load = out.assume_init();
        let take = |pointer: &mut *mut c_char| {
            if pointer.is_null() {
                return None;
            }
            let text = CStr::from_ptr(*pointer).to_string_lossy().into_owned();
            crate::dash_sdk_string_free(*pointer);
            *pointer = std::ptr::null_mut();
            Some(text)
        };
        let json = take(&mut load.identity_json);
        let message = take(&mut load.message);
        (load, json, message)
    }

    /// A shared masternode through the C export: the wire-order pro_tx_hash names the
    /// masternode, the owner identity is proved absent, and the voter identity is loaded.
    #[test]
    fn should_load_a_shared_masternode_through_the_export() {
        let voter_id = voter_identity_id(pro_tx_hash(), &VOTING_ADDRESS);
        let mut voter = identity(voter_id);
        voter.add_public_keys([voter_identity_voting_key(&VOTING_ADDRESS)]);
        let handle = mock_sdk_proving(None, Some(Some(voter)));
        let (load, json, message) = unsafe { load_through_the_export(handle, true) };
        destroy_mock_sdk_handle(handle);
        assert_eq!(load.kind, DashSDKMasternodeVotingLoadKind::VoterIdentity);
        assert_eq!(load.identity_id, voter_id.to_buffer());
        assert!(load.has_voting_key_verdict);
        assert_eq!(load.voting_key_verdict, DashSDKVotingKeyVerdict::Valid);
        assert!(json.is_some());
        assert_eq!(message, None);
    }

    /// Without a voting address the voter identity is never fetched (the mock holds no answer
    /// for it, so a fetch would fail the load): the owner identity loads with no verdict.
    #[test]
    fn should_load_the_owner_identity_without_looking_up_a_voter_through_the_export() {
        let owner_id = Identifier::new(pro_tx_hash().to_byte_array());
        let handle = mock_sdk_proving(Some(identity(owner_id)), None);
        let (load, json, message) = unsafe { load_through_the_export(handle, false) };
        destroy_mock_sdk_handle(handle);
        assert_eq!(load.kind, DashSDKMasternodeVotingLoadKind::OwnerIdentity);
        assert_eq!(load.identity_id, owner_id.to_buffer());
        assert!(!load.has_voting_key_verdict);
        assert!(json.is_some());
        assert_eq!(message, None);
    }

    /// A shared masternode whose voting key was disabled: refused, naming the voter identity
    /// the key was refused for, so a host can record the refusal on a row it holds there.
    #[test]
    fn should_refuse_a_disabled_key_and_name_its_voter_identity_through_the_export() {
        let voter_id = voter_identity_id(pro_tx_hash(), &VOTING_ADDRESS);
        let mut voter = identity(voter_id);
        let mut key = voter_identity_voting_key(&VOTING_ADDRESS);
        key.set_disabled_at(1_700_000_000);
        voter.add_public_keys([key]);
        let handle = mock_sdk_proving(None, Some(Some(voter)));
        let (load, json, message) = unsafe { load_through_the_export(handle, true) };
        destroy_mock_sdk_handle(handle);
        assert_eq!(load.kind, DashSDKMasternodeVotingLoadKind::Refused);
        assert_eq!(load.identity_id, voter_id.to_buffer());
        assert!(load.has_voting_key_verdict);
        assert_eq!(load.voting_key_verdict, DashSDKVotingKeyVerdict::Invalid);
        assert_eq!(json, None);
        assert!(message.is_some());
    }

    #[test]
    fn should_tell_the_user_a_voter_identity_is_not_a_pro_tx_hash() {
        let voter = voter_identity_id(pro_tx_hash(), &VOTING_ADDRESS);
        let decision = LoadDecision::of(MasternodeVotingLoad::Refused(
            MasternodeVotingLoadRefusal::VoterIdentityGivenAsProTxHash { identity: voter },
        ))
        .expect("marshalled");
        assert_eq!(decision.kind, DashSDKMasternodeVotingLoadKind::Refused);
        assert_eq!(
            decision.identity_id, [0u8; 32],
            "names no voter identity to record on"
        );
        assert_eq!(decision.voting_key_verdict, None);
        let message = decision.message.expect("a reason");
        let message = message.to_str().expect("utf-8");
        assert!(
            message.contains("voter identity, not a masternode"),
            "{message}"
        );
        assert!(!message.contains("  "), "whitespace run: {message}");
    }

    /// The sibling rule is rs-sdk's; this shows it crosses the C ABI as rs-sdk returns it.
    #[test]
    fn should_return_rs_sdk_sibling_record_for_every_case() {
        let other = [9u8; 20];
        for verdict in [
            DashSDKVotingKeyVerdict::Valid,
            DashSDKVotingKeyVerdict::Invalid,
            DashSDKVotingKeyVerdict::Undetermined,
        ] {
            for check in [
                DashSDKRecordedVotingKeyCheck::None,
                DashSDKRecordedVotingKeyCheck::Valid,
                DashSDKRecordedVotingKeyCheck::Invalid,
                DashSDKRecordedVotingKeyCheck::Undetermined,
            ] {
                for address in [VOTING_ADDRESS, other] {
                    let sibling = DashSDKRecordedVotingKey {
                        check,
                        voting_address: DashSDKVotingAddress { bytes: address },
                    };
                    let expected: DashSDKRecordedVotingKey =
                        sibling_voting_key_record(verdict.into(), &VOTING_ADDRESS, sibling.into())
                            .into();
                    assert_eq!(
                        dash_sdk_masternode_sibling_voting_key_record(
                            verdict,
                            DashSDKVotingAddress {
                                bytes: VOTING_ADDRESS
                            },
                            sibling,
                        ),
                        expected,
                        "{verdict:?} {check:?}"
                    );
                }
            }
        }
        // The case the rule exists for, spelled out.
        assert_eq!(
            dash_sdk_masternode_sibling_voting_key_record(
                DashSDKVotingKeyVerdict::Valid,
                DashSDKVotingAddress {
                    bytes: VOTING_ADDRESS
                },
                DashSDKRecordedVotingKey {
                    check: DashSDKRecordedVotingKeyCheck::Valid,
                    voting_address: DashSDKVotingAddress { bytes: other },
                },
            ),
            DashSDKRecordedVotingKey {
                check: DashSDKRecordedVotingKeyCheck::Undetermined,
                voting_address: DashSDKVotingAddress { bytes: other },
            }
        );
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
