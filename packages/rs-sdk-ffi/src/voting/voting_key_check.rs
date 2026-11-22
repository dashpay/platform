//! Checking a masternode voting key before any vote.
//!
//! A host that loads a masternode or evonode identity with its voting private key asks whether
//! the pair is a masternode voter Platform knows, so a mismatched key is refused when it is
//! entered rather than when a vote is refused. rs-sdk's `check_masternode_voting_key` decides,
//! by one proof-verified fetch of the voter identity; this module reads the arguments, runs
//! it and words the verdict for the host.
//!
//! What the host then does with the key (store it or refuse it, what to record for it, and
//! whether a stored key may vote or is checked again first) is decided by rs-sdk's
//! `voting_key_check_outcome` and `recorded_voting_key_use`. The two pure functions here only
//! marshal their arguments and results.

use crate::sdk::SDKWrapper;
use crate::types::SDKHandle;
use crate::voting::masternode_vote::{pro_tx_hash_from_wire_order, voting_key_problem_message};
use crate::{DashSDKResult, FFIError};
use dash_sdk::dpp::dashcore::ProTxHash;
use dash_sdk::platform::transition::masternode_vote_keys::{
    check_masternode_voting_key, recorded_voting_key_use, voting_key_check_outcome,
    RecordedVotingKeyCheck, RecordedVotingKeyUse, VotingKeyCheck, VotingKeyCheckOutcome,
    VotingKeySlot,
};
use dash_sdk::Error;
use std::ffi::CString;

/// What `dash_sdk_masternode_check_voting_key` found about a `(pro_tx_hash, voting address)`
/// pair.
///
/// cbindgen:prefix-with-name
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DashSDKVotingKeyVerdict {
    /// Platform holds an enabled voting key for this address on the masternode's voter
    /// identity, so the key can vote for it, apart from the one history rs-sdk's
    /// `VotingKeyCheck` describes. The host stores the key as checked.
    Valid = 0,
    /// Platform proves the key cannot vote for the masternode. The host refuses the key.
    Invalid = 1,
    /// The voter identity could not be fetched, so nothing is known about the key. The host
    /// may keep the key marked unchecked and check it again later.
    Undetermined = 2,
}

/// Checks whether the masternode voting key with `voting_address` can vote for the masternode
/// `voter_pro_tx_hash`, as Platform's state proves it (rs-sdk's `check_masternode_voting_key`:
/// one proof-verified fetch of the voter identity, no Core or SPV sync).
///
/// A masternode registered, or a voting key changed, in the last few blocks reads as
/// `Invalid` until Platform processes the masternode list update.
///
/// # Parameters
/// * `sdk_handle` - Handle to the SDK instance.
/// * `voter_pro_tx_hash` - Pointer to the masternode's 32-byte pro_tx_hash in wire order, as
///   every masternode vote function takes it (see "pro_tx_hash byte order" in
///   `voting/masternode_vote.rs`).
/// * `voting_address` - Pointer to the 20-byte voting address, the hash160 of the voting
///   public key (`dash_sdk_masternode_voting_address_from_private_key` forms it from the
///   voting private key).
/// * `out_verdict` - Receives the verdict. Need not be initialized.
///
/// # Returns
/// Success with the verdict written to `out_verdict`, and data that is null for `Valid` and,
/// for `Invalid` and `Undetermined`, a C string saying why, to show the user (free it with
/// `dash_sdk_string_free`). A failed fetch is `Undetermined`, never an error.
/// `InvalidParameter` for a null pointer, and nothing is written then.
///
/// # Safety
/// `sdk_handle` must be null or a valid `SDKHandle`, `voter_pro_tx_hash` null or point to 32
/// readable bytes, `voting_address` null or to 20 readable bytes, and `out_verdict` null or
/// point to a writable `DashSDKVotingKeyVerdict`, for the duration of the call.
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_masternode_check_voting_key(
    sdk_handle: *const SDKHandle,
    voter_pro_tx_hash: *const u8,
    voting_address: *const u8,
    out_verdict: *mut DashSDKVotingKeyVerdict,
) -> DashSDKResult {
    match check_voting_key_inner(sdk_handle, voter_pro_tx_hash, voting_address, out_verdict) {
        Ok(message) => match message {
            Some(message) => DashSDKResult::success_string(message.into_raw()),
            None => DashSDKResult::success(std::ptr::null_mut()),
        },
        Err(error) => DashSDKResult::error(error.into()),
    }
}

/// Reads every argument, runs the check, and writes the verdict only once its message is
/// ready, so an error leaves `out_verdict` untouched.
unsafe fn check_voting_key_inner(
    sdk_handle: *const SDKHandle,
    voter_pro_tx_hash: *const u8,
    voting_address: *const u8,
    out_verdict: *mut DashSDKVotingKeyVerdict,
) -> Result<Option<CString>, FFIError> {
    if sdk_handle.is_null() {
        return Err(FFIError::InvalidParameter("sdk_handle is null".to_string()));
    }
    if out_verdict.is_null() {
        return Err(FFIError::InvalidParameter(
            "out_verdict is null".to_string(),
        ));
    }
    let pro_tx_hash = pro_tx_hash_from_wire_order(voter_pro_tx_hash)?;
    if voting_address.is_null() {
        return Err(FFIError::InvalidParameter(
            "voting_address is null".to_string(),
        ));
    }
    let mut address = [0u8; 20];
    address.copy_from_slice(std::slice::from_raw_parts(voting_address, 20));
    let wrapper = &*(sdk_handle as *const SDKWrapper);

    let check = wrapper.runtime.block_on(check_masternode_voting_key(
        &wrapper.sdk,
        ProTxHash::from_byte_array(pro_tx_hash.to_buffer()),
        &address,
    ));
    let (verdict, message) = verdict_and_message(check);
    let message = message
        .map(|message| {
            CString::new(message).map_err(|_| {
                FFIError::InternalError("The voting key verdict has a NUL byte".to_string())
            })
        })
        .transpose()?;
    // A raw write: the caller's verdict may be uninitialized.
    out_verdict.write(verdict);
    Ok(message)
}

/// The verdict a host receives for `check`, and the sentence it shows for `Invalid` and
/// `Undetermined`.
fn verdict_and_message(check: VotingKeyCheck) -> (DashSDKVotingKeyVerdict, Option<String>) {
    match check {
        VotingKeyCheck::Valid { .. } => (DashSDKVotingKeyVerdict::Valid, None),
        VotingKeyCheck::Invalid(problem) => (
            DashSDKVotingKeyVerdict::Invalid,
            Some(voting_key_problem_message(problem)),
        ),
        VotingKeyCheck::Undetermined(error) => (
            DashSDKVotingKeyVerdict::Undetermined,
            Some(undetermined_message(&error)),
        ),
    }
}

/// Why a check could not tell, phrased for the host.
fn undetermined_message(error: &Error) -> String {
    // A diagnostic can carry an interior NUL (a CBOR dump); escaped, the reason still fits in
    // a C string, so the verdict stays Undetermined instead of becoming an error.
    let reason = error.to_string().replace('\0', "\\0");
    format!(
        "The voting key could not be checked against Platform: {}.",
        reason.trim().trim_end_matches('.')
    )
}

/// What a host recorded for its stored voting key: rs-sdk's `RecordedVotingKeyCheck`, or
/// `None` when nothing was recorded.
///
/// cbindgen:prefix-with-name
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DashSDKRecordedVotingKeyCheck {
    /// Nothing is recorded: the key was never checked, such as a key stored before checks
    /// existed.
    None = 0,
    /// The last check was `Valid`.
    Valid = 1,
    /// The last check was `Invalid`.
    Invalid = 2,
    /// The last check was `Undetermined`.
    Undetermined = 3,
}

impl From<DashSDKRecordedVotingKeyCheck> for Option<RecordedVotingKeyCheck> {
    fn from(recorded: DashSDKRecordedVotingKeyCheck) -> Self {
        match recorded {
            DashSDKRecordedVotingKeyCheck::None => None,
            DashSDKRecordedVotingKeyCheck::Valid => Some(RecordedVotingKeyCheck::Valid),
            DashSDKRecordedVotingKeyCheck::Invalid => Some(RecordedVotingKeyCheck::Invalid),
            DashSDKRecordedVotingKeyCheck::Undetermined => {
                Some(RecordedVotingKeyCheck::Undetermined)
            }
        }
    }
}

impl From<Option<RecordedVotingKeyCheck>> for DashSDKRecordedVotingKeyCheck {
    fn from(recorded: Option<RecordedVotingKeyCheck>) -> Self {
        match recorded {
            None => DashSDKRecordedVotingKeyCheck::None,
            Some(RecordedVotingKeyCheck::Valid) => DashSDKRecordedVotingKeyCheck::Valid,
            Some(RecordedVotingKeyCheck::Invalid) => DashSDKRecordedVotingKeyCheck::Invalid,
            Some(RecordedVotingKeyCheck::Undetermined) => {
                DashSDKRecordedVotingKeyCheck::Undetermined
            }
        }
    }
}

impl From<DashSDKVotingKeyVerdict> for RecordedVotingKeyCheck {
    fn from(verdict: DashSDKVotingKeyVerdict) -> Self {
        match verdict {
            DashSDKVotingKeyVerdict::Valid => RecordedVotingKeyCheck::Valid,
            DashSDKVotingKeyVerdict::Invalid => RecordedVotingKeyCheck::Invalid,
            DashSDKVotingKeyVerdict::Undetermined => RecordedVotingKeyCheck::Undetermined,
        }
    }
}

/// What the host's voting key slot for the masternode holds, compared with the key that was
/// checked: rs-sdk's `VotingKeySlot` without its payload, which travels beside it
/// (`recorded_for_same_key` of `dash_sdk_masternode_voting_key_check_outcome`).
///
/// cbindgen:prefix-with-name
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DashSDKVotingKeySlot {
    /// No voting key is stored for the masternode.
    Empty = 0,
    /// Exactly the checked key is stored.
    SameKey = 1,
    /// A different key is stored.
    DifferentKey = 2,
}

/// The rs-sdk slot for `slot`, with `recorded_for_same_key` read only for `SameKey`.
fn voting_key_slot(
    slot: DashSDKVotingKeySlot,
    recorded_for_same_key: DashSDKRecordedVotingKeyCheck,
) -> VotingKeySlot {
    match slot {
        DashSDKVotingKeySlot::Empty => VotingKeySlot::Empty,
        DashSDKVotingKeySlot::SameKey => VotingKeySlot::SameKey(recorded_for_same_key.into()),
        DashSDKVotingKeySlot::DifferentKey => VotingKeySlot::DifferentKey,
    }
}

/// What a host does with a voting key once checked: rs-sdk's `VotingKeyCheckOutcome`.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DashSDKVotingKeyCheckOutcome {
    /// Whether the host stores the checked key.
    pub store_key: bool,
    /// Whether the host keeps what it already recorded for the key (rs-sdk's `record` is
    /// `None`). `record` is then `None`.
    pub keeps_record: bool,
    /// What the host records for the checked key once it is in the slot, or for the stored key
    /// when it is the same one. Read only when `keeps_record` is false.
    pub record: DashSDKRecordedVotingKeyCheck,
}

impl From<VotingKeyCheckOutcome> for DashSDKVotingKeyCheckOutcome {
    fn from(outcome: VotingKeyCheckOutcome) -> Self {
        DashSDKVotingKeyCheckOutcome {
            store_key: outcome.store_key,
            keeps_record: outcome.record.is_none(),
            record: outcome.record.into(),
        }
    }
}

/// What a host does with a voting key after checking it
/// (`dash_sdk_masternode_check_voting_key`), given what its voting key slot holds: rs-sdk's
/// `voting_key_check_outcome`, which holds the rules.
///
/// # Parameters
/// * `verdict` - The verdict of the check.
/// * `slot` - What the host's voting key slot for the masternode holds, compared with the
///   checked key.
/// * `recorded_for_same_key` - What the host recorded for the stored key. Read only when
///   `slot` is `SameKey`.
///
/// # Returns
/// Whether to store the key, and what to record for it (or that what is recorded is kept).
#[no_mangle]
pub extern "C" fn dash_sdk_masternode_voting_key_check_outcome(
    verdict: DashSDKVotingKeyVerdict,
    slot: DashSDKVotingKeySlot,
    recorded_for_same_key: DashSDKRecordedVotingKeyCheck,
) -> DashSDKVotingKeyCheckOutcome {
    voting_key_check_outcome(verdict.into(), voting_key_slot(slot, recorded_for_same_key)).into()
}

/// How a host may use a stored voting key: rs-sdk's `RecordedVotingKeyUse`.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DashSDKRecordedVotingKeyUse {
    /// Whether the key is offered for voting.
    pub can_vote: bool,
    /// Whether the host checks the key (`dash_sdk_masternode_check_voting_key`) before using
    /// it.
    pub check_before_use: bool,
}

impl From<RecordedVotingKeyUse> for DashSDKRecordedVotingKeyUse {
    fn from(key_use: RecordedVotingKeyUse) -> Self {
        DashSDKRecordedVotingKeyUse {
            can_vote: key_use.can_vote,
            check_before_use: key_use.check_before_use,
        }
    }
}

/// How a host may use a stored voting key whose last recorded check is `recorded`: rs-sdk's
/// `recorded_voting_key_use`, which holds the rules.
///
/// # Parameters
/// * `recorded` - What the host recorded for the stored key, `None` when nothing was.
///
/// # Returns
/// Whether the key may vote, and whether it is checked before it is used.
#[no_mangle]
pub extern "C" fn dash_sdk_masternode_recorded_voting_key_use(
    recorded: DashSDKRecordedVotingKeyCheck,
) -> DashSDKRecordedVotingKeyUse {
    recorded_voting_key_use(recorded.into()).into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::test_utils::{create_mock_sdk_handle, destroy_mock_sdk_handle};
    use crate::{dash_sdk_error_free, dash_sdk_string_free, DashSDKErrorCode};
    use dash_sdk::dpp::platform_value::Identifier;
    use dash_sdk::platform::transition::masternode_vote_keys::VotingKeyProblem;
    use std::ffi::{c_char, CStr};

    /// A verdict no call writes, to show that a refusal leaves the caller's value alone.
    const UNTOUCHED: DashSDKVotingKeyVerdict = DashSDKVotingKeyVerdict::Valid;

    #[test]
    fn should_refuse_each_null_argument_and_write_no_verdict() {
        let handle = create_mock_sdk_handle();
        let sdk_handle: *const SDKHandle = handle;
        let pro_tx_hash = [1u8; 32];
        let voting_address = [3u8; 20];
        let mut verdict = UNTOUCHED;
        let out: *mut DashSDKVotingKeyVerdict = &mut verdict;
        for (sdk_handle, pro_tx_hash, voting_address, out_verdict, named) in [
            (
                std::ptr::null(),
                pro_tx_hash.as_ptr(),
                voting_address.as_ptr(),
                out,
                "sdk_handle",
            ),
            (
                sdk_handle,
                std::ptr::null(),
                voting_address.as_ptr(),
                out,
                "voter_pro_tx_hash",
            ),
            (
                sdk_handle,
                pro_tx_hash.as_ptr(),
                std::ptr::null(),
                out,
                "voting_address",
            ),
            (
                sdk_handle,
                pro_tx_hash.as_ptr(),
                voting_address.as_ptr(),
                std::ptr::null_mut(),
                "out_verdict",
            ),
        ] {
            unsafe {
                let result = dash_sdk_masternode_check_voting_key(
                    sdk_handle,
                    pro_tx_hash,
                    voting_address,
                    out_verdict,
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
        assert_eq!(verdict, UNTOUCHED, "nothing is written on a refusal");
        destroy_mock_sdk_handle(handle);
    }

    /// The mock SDK holds no answer for the voter identity, so the fetch fails: the check
    /// cannot tell, which is undetermined with a reason, never invalid and never an error.
    #[test]
    fn should_report_a_failed_fetch_as_undetermined_with_a_message() {
        let handle = create_mock_sdk_handle();
        let pro_tx_hash = [1u8; 32];
        let voting_address = [3u8; 20];
        let mut verdict = DashSDKVotingKeyVerdict::Valid;
        unsafe {
            let result = dash_sdk_masternode_check_voting_key(
                handle,
                pro_tx_hash.as_ptr(),
                voting_address.as_ptr(),
                &mut verdict,
            );
            assert!(result.error.is_null(), "a failed fetch is not an error");
            assert!(!result.data.is_null(), "undetermined carries its reason");
            let pointer = result.data as *mut c_char;
            let message = CStr::from_ptr(pointer).to_string_lossy().into_owned();
            dash_sdk_string_free(pointer);
            assert_eq!(verdict, DashSDKVotingKeyVerdict::Undetermined);
            assert!(
                message.starts_with("The voting key could not be checked against Platform: "),
                "{message}"
            );
            assert!(
                !message.contains("  "),
                "message has whitespace runs: {message}"
            );
            assert!(!message.contains(".."), "message ends twice: {message}");
            destroy_mock_sdk_handle(handle);
        }
    }

    // ---- marshalling the host rules -----------------------------------------
    //
    // The rules are rs-sdk's and tested there; these only show that each case crosses the C
    // ABI as rs-sdk says it.

    const EVERY_RECORDED: [(
        DashSDKRecordedVotingKeyCheck,
        Option<RecordedVotingKeyCheck>,
    ); 4] = [
        (DashSDKRecordedVotingKeyCheck::None, None),
        (
            DashSDKRecordedVotingKeyCheck::Valid,
            Some(RecordedVotingKeyCheck::Valid),
        ),
        (
            DashSDKRecordedVotingKeyCheck::Invalid,
            Some(RecordedVotingKeyCheck::Invalid),
        ),
        (
            DashSDKRecordedVotingKeyCheck::Undetermined,
            Some(RecordedVotingKeyCheck::Undetermined),
        ),
    ];

    const EVERY_VERDICT: [(DashSDKVotingKeyVerdict, RecordedVotingKeyCheck); 3] = [
        (
            DashSDKVotingKeyVerdict::Valid,
            RecordedVotingKeyCheck::Valid,
        ),
        (
            DashSDKVotingKeyVerdict::Invalid,
            RecordedVotingKeyCheck::Invalid,
        ),
        (
            DashSDKVotingKeyVerdict::Undetermined,
            RecordedVotingKeyCheck::Undetermined,
        ),
    ];

    #[test]
    fn should_map_each_recorded_check_both_ways() {
        for (ffi, recorded) in EVERY_RECORDED {
            assert_eq!(Option::<RecordedVotingKeyCheck>::from(ffi), recorded);
            assert_eq!(DashSDKRecordedVotingKeyCheck::from(recorded), ffi);
        }
        for (ffi, recorded) in EVERY_VERDICT {
            assert_eq!(RecordedVotingKeyCheck::from(ffi), recorded);
        }
    }

    /// Every verdict in every slot, with every record of the same key, returns what rs-sdk
    /// returns, and `keeps_record` says exactly when rs-sdk keeps the record.
    #[test]
    fn should_return_rs_sdk_outcome_for_every_verdict_and_slot() {
        let mut slots = vec![
            (
                DashSDKVotingKeySlot::Empty,
                DashSDKRecordedVotingKeyCheck::None,
                VotingKeySlot::Empty,
            ),
            (
                DashSDKVotingKeySlot::DifferentKey,
                DashSDKRecordedVotingKeyCheck::None,
                VotingKeySlot::DifferentKey,
            ),
        ];
        for (ffi, recorded) in EVERY_RECORDED {
            slots.push((
                DashSDKVotingKeySlot::SameKey,
                ffi,
                VotingKeySlot::SameKey(recorded),
            ));
        }
        for (verdict, check) in EVERY_VERDICT {
            for (slot, recorded_for_same_key, rs_slot) in &slots {
                let expected = voting_key_check_outcome(check, *rs_slot);
                let outcome = dash_sdk_masternode_voting_key_check_outcome(
                    verdict,
                    *slot,
                    *recorded_for_same_key,
                );
                assert_eq!(
                    outcome.store_key, expected.store_key,
                    "{verdict:?} {rs_slot:?}"
                );
                assert_eq!(
                    outcome.keeps_record,
                    expected.record.is_none(),
                    "{verdict:?} {rs_slot:?}"
                );
                assert_eq!(
                    outcome.record,
                    expected.record.into(),
                    "{verdict:?} {rs_slot:?}"
                );
            }
        }
    }

    /// The record of the stored key is read only for the same key.
    #[test]
    fn should_ignore_the_recorded_check_unless_the_slot_holds_the_same_key() {
        for slot in [
            DashSDKVotingKeySlot::Empty,
            DashSDKVotingKeySlot::DifferentKey,
        ] {
            for (verdict, _) in EVERY_VERDICT {
                let unrecorded = dash_sdk_masternode_voting_key_check_outcome(
                    verdict,
                    slot,
                    DashSDKRecordedVotingKeyCheck::None,
                );
                for (recorded, _) in EVERY_RECORDED {
                    assert_eq!(
                        dash_sdk_masternode_voting_key_check_outcome(verdict, slot, recorded),
                        unrecorded,
                        "{verdict:?} {slot:?} {recorded:?}"
                    );
                }
            }
        }
    }

    /// Two cases spelled out, so a mapping that agreed with itself but not with rs-sdk's
    /// meaning would still show: a refusal of the same stored key is recorded, and a check
    /// that could not run keeps a proven record of the same key.
    #[test]
    fn should_carry_a_recorded_refusal_and_a_kept_record() {
        assert_eq!(
            dash_sdk_masternode_voting_key_check_outcome(
                DashSDKVotingKeyVerdict::Invalid,
                DashSDKVotingKeySlot::SameKey,
                DashSDKRecordedVotingKeyCheck::Valid,
            ),
            DashSDKVotingKeyCheckOutcome {
                store_key: false,
                keeps_record: false,
                record: DashSDKRecordedVotingKeyCheck::Invalid,
            }
        );
        assert_eq!(
            dash_sdk_masternode_voting_key_check_outcome(
                DashSDKVotingKeyVerdict::Undetermined,
                DashSDKVotingKeySlot::SameKey,
                DashSDKRecordedVotingKeyCheck::Valid,
            ),
            DashSDKVotingKeyCheckOutcome {
                store_key: true,
                keeps_record: true,
                record: DashSDKRecordedVotingKeyCheck::None,
            }
        );
    }

    #[test]
    fn should_return_rs_sdk_use_for_every_recorded_check() {
        for (ffi, recorded) in EVERY_RECORDED {
            let expected = recorded_voting_key_use(recorded);
            assert_eq!(
                dash_sdk_masternode_recorded_voting_key_use(ffi),
                DashSDKRecordedVotingKeyUse {
                    can_vote: expected.can_vote,
                    check_before_use: expected.check_before_use,
                },
                "{ffi:?}"
            );
        }
        assert_eq!(
            dash_sdk_masternode_recorded_voting_key_use(DashSDKRecordedVotingKeyCheck::Invalid),
            DashSDKRecordedVotingKeyUse {
                can_vote: false,
                check_before_use: false,
            }
        );
    }

    #[test]
    fn should_word_each_verdict_for_the_host() {
        let (verdict, message) = verdict_and_message(VotingKeyCheck::Valid {
            voter_identity: Identifier::new([5u8; 32]),
        });
        assert_eq!(verdict, DashSDKVotingKeyVerdict::Valid);
        assert_eq!(message, None);

        let problem = VotingKeyProblem::NoUsableVotingKey {
            voter_identity: Identifier::new([5u8; 32]),
        };
        let (verdict, message) = verdict_and_message(VotingKeyCheck::Invalid(problem.clone()));
        assert_eq!(verdict, DashSDKVotingKeyVerdict::Invalid);
        // The check and a refused vote say the same sentence.
        assert_eq!(message, Some(voting_key_problem_message(problem)));

        let (verdict, message) = verdict_and_message(VotingKeyCheck::Undetermined(Box::new(
            Error::Generic("connection reset.".to_string()),
        )));
        assert_eq!(verdict, DashSDKVotingKeyVerdict::Undetermined);
        let message = message.expect("a reason");
        assert!(message.contains("connection reset"), "{message}");

        // An interior NUL in a diagnostic still yields Undetermined with a C string.
        let (verdict, message) = verdict_and_message(VotingKeyCheck::Undetermined(Box::new(
            Error::Generic("cbor \0 dump".to_string()),
        )));
        assert_eq!(verdict, DashSDKVotingKeyVerdict::Undetermined);
        let message = message.expect("a reason");
        assert!(CString::new(message.clone()).is_ok(), "{message}");
        assert!(message.contains("cbor \\0 dump"), "{message}");
        assert!(!message.contains(".."), "message ends twice: {message}");
        assert!(
            !message.contains("  "),
            "message has whitespace runs: {message}"
        );
    }
}
