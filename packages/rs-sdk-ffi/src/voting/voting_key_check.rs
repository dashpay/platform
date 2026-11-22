//! Checking a masternode voting key before any vote.
//!
//! A host that loads a masternode or evonode identity with its voting private key asks whether
//! the pair is a masternode voter Platform knows, so a mismatched key is refused when it is
//! entered rather than when a vote is refused. rs-sdk's `check_masternode_voting_key` decides,
//! by one proof-verified fetch of the voter identity; this module reads the arguments, runs
//! it and words the verdict for the host.

use crate::sdk::SDKWrapper;
use crate::types::SDKHandle;
use crate::voting::masternode_vote::{pro_tx_hash_from_wire_order, voting_key_problem_message};
use crate::{DashSDKResult, FFIError};
use dash_sdk::dpp::dashcore::ProTxHash;
use dash_sdk::platform::transition::masternode_vote_keys::{
    check_masternode_voting_key, VotingKeyCheck,
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
    /// The key can vote for the masternode: Platform holds an enabled voting key for this
    /// address on the masternode's voter identity. The host stores the key as checked.
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
    let reason = error.to_string();
    format!(
        "The voting key could not be checked against Platform: {}.",
        reason.trim().trim_end_matches('.')
    )
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
        assert!(!message.contains(".."), "message ends twice: {message}");
        assert!(
            !message.contains("  "),
            "message has whitespace runs: {message}"
        );
    }
}
