//! JNI exports for decision proposals (protocol version 15): yes/no questions any identity
//! puts to the masternodes, as `proposal` documents of the decision proposals system
//! contract. Each export marshals ONE C function: a `dash_sdk_decision_proposal_*` function
//! of `rs-sdk-ffi` (the contract's names, the reads and the masternode vote), or
//! `platform_wallet_decision_proposal_create` / `_edit` of `platform-wallet-ffi` (the create
//! and the edit, which the platform wallet builds, signs, broadcasts and confirms itself). It
//! reads the JVM arguments, calls the C function, copies the result into a JVM value and
//! frees every Rust allocation the result carried before returning.
//!
//! Errors follow the convention of the C function's crate: a `DashSDKResult` error throws
//! `DashSDKException` through [`crate::results::take_error`], and a `PlatformWalletFFIResult`
//! error through [`crate::support::take_pwffi_error`] (its code shifted by
//! `PWFFI_CODE_OFFSET`, a refusal by Platform carrying its consensus code). An argument the
//! JVM value cannot carry into the C type (a null required string, a byte array of the wrong
//! length, a number out of the C type's range) throws code 1 (`InvalidParameter`) before any
//! C call; everything the C functions and Rust behind them check is left to them.
//!
//! Kotlin counterpart: `org.dashfoundation.dashsdk.ffi.DecisionProposalsNative`.

use crate::results::{take_error, unwrap_string};
use crate::support::{guard, net_from_ord, take_pwffi_error, throw_sdk_exception};
use jni::objects::{JByteArray, JClass, JString};
use jni::sys::{jboolean, jbyte, jbyteArray, jint, jlong, jstring};
use jni::JNIEnv;
use platform_wallet_ffi::handle::Handle;
use platform_wallet_ffi::{
    platform_wallet_decision_proposal_create, platform_wallet_decision_proposal_edit,
    platform_wallet_string_free, DecisionProposalEditFFI, DecisionProposalParamsFFI,
};
use rs_sdk_ffi::{
    dash_sdk_decision_proposal_cast_vote, dash_sdk_decision_proposal_contract_id,
    dash_sdk_decision_proposal_contract_id_bytes, dash_sdk_decision_proposal_document_type_name,
    dash_sdk_decision_proposal_fetch, dash_sdk_decision_proposal_get_masternode_vote,
    dash_sdk_decision_proposal_get_vote_poll_id, dash_sdk_decision_proposal_get_vote_poll_state,
    dash_sdk_decision_proposal_list_by_created_at, dash_sdk_decision_proposal_list_by_end_date,
    dash_sdk_decision_proposal_list_by_owner, DashSDKResult, SDKHandle, SignerHandle,
};
use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::ptr;

// ── Marshalling helpers ──────────────────────────────────────────────────

/// Read a nullable JVM string into an owned `CString`.
///
/// `Some(None)` for a null string, `Some(Some(_))` for a string, and `None` after throwing
/// for a string JNI cannot read or one holding a NUL byte (the caller bails out). An
/// unreadable string is never mistaken for an absent one.
fn read_optional_cstring(env: &mut JNIEnv, s: &JString, field: &str) -> Option<Option<CString>> {
    if s.is_null() {
        return Some(None);
    }
    let raw: String = match env.get_string(s) {
        Ok(js) => js.into(),
        Err(_) => {
            let _ = env.exception_clear();
            throw_sdk_exception(env, 1, &format!("{field} string was invalid"));
            return None;
        }
    };
    match CString::new(raw) {
        Ok(c) => Some(Some(c)),
        Err(_) => {
            throw_sdk_exception(env, 1, &format!("{field} contained an interior NUL"));
            None
        }
    }
}

/// Read a required JVM string into an owned `CString`; throws and returns `None` for a null
/// or unreadable string.
fn read_cstring(env: &mut JNIEnv, s: &JString, field: &str) -> Option<CString> {
    match read_optional_cstring(env, s, field)? {
        Some(c) => Some(c),
        None => {
            throw_sdk_exception(env, 1, &format!("{field} was null"));
            None
        }
    }
}

/// The pointer the C functions take for an optional string: null for `None`.
fn c_ptr(value: &Option<CString>) -> *const c_char {
    value.as_ref().map_or(ptr::null(), |s| s.as_ptr())
}

/// Read a required 32-byte array; throws and returns `None` for a null array or any other
/// length (the C functions read exactly 32 bytes through the pointer).
fn read_bytes32(env: &mut JNIEnv, arr: &JByteArray, field: &str) -> Option<[u8; 32]> {
    let bytes = match env.convert_byte_array(arr) {
        Ok(b) => b,
        Err(_) => {
            let _ = env.exception_clear();
            throw_sdk_exception(env, 1, &format!("{field} byte[] was null/invalid"));
            return None;
        }
    };
    match <[u8; 32]>::try_from(bytes.as_slice()) {
        Ok(value) => Some(value),
        Err(_) => {
            throw_sdk_exception(
                env,
                1,
                &format!("{field} must be 32 bytes, got {}", bytes.len()),
            );
            None
        }
    }
}

/// Secret-key sibling of [`read_bytes32`]: the returned buffer is scrubbed on drop and the
/// intermediate JNI copy is zeroized before it is dropped. For the voting private key only.
fn read_key32_zeroizing(
    env: &mut JNIEnv,
    arr: &JByteArray,
    field: &str,
) -> Option<zeroize::Zeroizing<[u8; 32]>> {
    use zeroize::Zeroize;

    let mut bytes = match env.convert_byte_array(arr) {
        Ok(b) => b,
        Err(_) => {
            let _ = env.exception_clear();
            throw_sdk_exception(env, 1, &format!("{field} byte[] was null/invalid"));
            return None;
        }
    };
    if bytes.len() != 32 {
        let len = bytes.len();
        bytes.zeroize();
        throw_sdk_exception(env, 1, &format!("{field} must be 32 bytes, got {len}"));
        return None;
    }
    let mut key = zeroize::Zeroizing::new([0u8; 32]);
    key.copy_from_slice(&bytes);
    bytes.zeroize();
    Some(key)
}

/// `value` as the unsigned C type `T`; throws and returns `None` for a value `T` cannot
/// hold (negative, or too large) instead of wrapping it into a different, valid one.
fn to_unsigned<T: TryFrom<i64>>(env: &mut JNIEnv, value: i64, field: &str) -> Option<T> {
    match T::try_from(value) {
        Ok(converted) => Some(converted),
        Err(_) => {
            throw_sdk_exception(env, 1, &format!("{field} is out of range: {value}"));
            None
        }
    }
}

/// The JVM string a result carries (its Rust string freed), or null for no data or after
/// throwing.
///
/// # Safety
/// `result` must be a `DashSDKResult` freshly returned by an rs-sdk-ffi function whose
/// success payload is a `char*` or no data.
unsafe fn string_or_null(env: &mut JNIEnv, result: DashSDKResult) -> jstring {
    unwrap_string(env, result)
        .map(|s| s.into_raw())
        .unwrap_or(ptr::null_mut())
}

/// A JVM copy of a static C string owned by rs-sdk-ffi, which is never freed.
///
/// # Safety
/// `value` must point to a NUL-terminated string that lives for the whole program.
unsafe fn static_string(env: &mut JNIEnv, value: *const c_char) -> jstring {
    let value = CStr::from_ptr(value).to_string_lossy();
    env.new_string(value)
        .map(|s| s.into_raw())
        .unwrap_or(ptr::null_mut())
}

/// Whether `arr` is a non-null `byte[len]`. Clears the exception a failed length read
/// leaves; the caller throws.
fn is_byte_array_of_len(env: &mut JNIEnv, arr: &JByteArray, len: i32) -> bool {
    if arr.is_null() {
        return false;
    }
    match env.get_array_length(arr) {
        Ok(actual) => actual == len,
        Err(_) => {
            let _ = env.exception_clear();
            false
        }
    }
}

/// The confirmed document JSON a platform wallet write handed over through its
/// `out_document_json`, copied into a Rust string before the C string is freed with
/// `platform_wallet_string_free`. Throws and returns `None` when a successful call left it
/// null.
///
/// # Safety
/// `json` must be null or a string the platform wallet allocated and nothing else frees.
unsafe fn take_wallet_json(env: &mut JNIEnv, json: *mut c_char, call: &str) -> Option<String> {
    if json.is_null() {
        throw_sdk_exception(
            env,
            99,
            &format!("{call} returned success but no document JSON"),
        );
        return None;
    }
    let copied = CStr::from_ptr(json).to_string_lossy().into_owned();
    platform_wallet_string_free(json);
    Some(copied)
}

// ── Constants ────────────────────────────────────────────────────────────

/// The id of the decision proposals system contract, base58
/// (`dash_sdk_decision_proposal_contract_id`): the `dataContractId` the generic document
/// reads take to read proposals.
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_DecisionProposalsNative_contractId(
    mut env: JNIEnv,
    _class: JClass,
) -> jstring {
    guard(&mut env, ptr::null_mut(), |env| unsafe {
        static_string(env, dash_sdk_decision_proposal_contract_id())
    })
}

/// The 32 bytes of the decision proposals contract id
/// (`dash_sdk_decision_proposal_contract_id_bytes`), for the calls that take a contract id
/// as bytes.
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_DecisionProposalsNative_contractIdBytes(
    mut env: JNIEnv,
    _class: JClass,
) -> jbyteArray {
    guard(&mut env, ptr::null_mut(), |env| {
        let mut contract_id = [0u8; 32];
        // SAFETY: `contract_id` is 32 writable bytes.
        if !unsafe { dash_sdk_decision_proposal_contract_id_bytes(contract_id.as_mut_ptr()) } {
            throw_sdk_exception(
                env,
                99,
                "the decision proposals contract id was not written",
            );
            return ptr::null_mut();
        }
        env.byte_array_from_slice(&contract_id)
            .map(|a| a.into_raw())
            .unwrap_or(ptr::null_mut())
    })
}

/// The document type name of a proposal (`dash_sdk_decision_proposal_document_type_name`).
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_DecisionProposalsNative_documentTypeName(
    mut env: JNIEnv,
    _class: JClass,
) -> jstring {
    guard(&mut env, ptr::null_mut(), |env| unsafe {
        static_string(env, dash_sdk_decision_proposal_document_type_name())
    })
}

// ── Create and edit (platform wallet) ────────────────────────────────────

/// Create + broadcast a decision proposal of `ownerId`, signed via `signerHandle`: the JNI
/// bridge over `platform_wallet_decision_proposal_create` (Swift
/// `ManagedPlatformWallet.createDecisionProposal`). The wallet builds the proposal, whose
/// create states the decision proposal fund (paid by the owner on top of the fees), picks
/// the signing key itself, broadcasts and waits for Platform to confirm.
///
/// `url` may be null (no url). `passPercent` and `minimumTurnoutPercent` must fit a `u8` and
/// `endDateMs` a `u64`; the limits on them are Platform's. `outProposalId` must be a
/// non-null `byte[32]`, checked before anything is broadcast so that the id of a paid create
/// can always be written back; it receives the proposal's id.
///
/// Returns the confirmed document's canonical JSON (the shape
/// `TransactionsNative.documentCreate` returns), its Rust string freed here. Null after
/// throwing; a refusal by Platform carries its consensus code.
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_DecisionProposalsNative_create(
    mut env: JNIEnv,
    _class: JClass,
    wallet_handle: jlong,
    owner_id: JByteArray,
    title: JString,
    description: JString,
    url: JString,
    pass_percent: jint,
    minimum_turnout_percent: jint,
    end_date_ms: jlong,
    signer_handle: jlong,
    out_proposal_id: JByteArray,
) -> jstring {
    guard(&mut env, ptr::null_mut(), |env| {
        // Checked before the paid create: once it succeeded, writing the id back into a
        // validated array allocates nothing and cannot go out of bounds.
        if !is_byte_array_of_len(env, &out_proposal_id, 32) {
            throw_sdk_exception(env, 1, "outProposalId must be a non-null byte[32]");
            return ptr::null_mut();
        }
        let Some(owner) = read_bytes32(env, &owner_id, "ownerId") else {
            return ptr::null_mut();
        };
        let Some(title) = read_cstring(env, &title, "title") else {
            return ptr::null_mut();
        };
        let Some(description) = read_cstring(env, &description, "description") else {
            return ptr::null_mut();
        };
        let Some(url) = read_optional_cstring(env, &url, "url") else {
            return ptr::null_mut();
        };
        let Some(pass_percent) = to_unsigned(env, pass_percent.into(), "passPercent") else {
            return ptr::null_mut();
        };
        let Some(minimum_turnout_percent) =
            to_unsigned(env, minimum_turnout_percent.into(), "minimumTurnoutPercent")
        else {
            return ptr::null_mut();
        };
        let Some(end_date_ms) = to_unsigned(env, end_date_ms, "endDateMs") else {
            return ptr::null_mut();
        };

        let params = DecisionProposalParamsFFI {
            title: title.as_ptr(),
            description: description.as_ptr(),
            url: c_ptr(&url),
            pass_percent,
            minimum_turnout_percent,
            end_date_ms,
        };
        let mut proposal_id = [0u8; 32];
        let mut out_json: *mut c_char = ptr::null_mut();
        // SAFETY: `owner` and `proposal_id` are 32 bytes, `params` and the strings it points
        // to outlive the call, and the signer handle is the Kotlin manager's live signer.
        let result = unsafe {
            platform_wallet_decision_proposal_create(
                wallet_handle as Handle,
                owner.as_ptr(),
                &params,
                signer_handle as *mut SignerHandle,
                proposal_id.as_mut_ptr(),
                &mut out_json,
            )
        };
        if take_pwffi_error(env, result) {
            return ptr::null_mut();
        }
        // SAFETY: on success `out_json` is the wallet's string, freed by this call only.
        let Some(json) = (unsafe { take_wallet_json(env, out_json, "decision proposal create") })
        else {
            return ptr::null_mut();
        };

        let id = proposal_id.map(|b| b as jbyte);
        if env.set_byte_array_region(&out_proposal_id, 0, &id).is_err() {
            // Unreachable after the check above; kept as a backstop that says the proposal
            // exists, so a caller does not retry a paid create.
            let _ = env.exception_clear();
            throw_sdk_exception(
                env,
                99,
                "the decision proposal was created, but its id could not be returned",
            );
            return ptr::null_mut();
        }
        env.new_string(json)
            .map(|s| s.into_raw())
            .unwrap_or(ptr::null_mut())
    })
}

/// Give the decision proposal `proposalId` of `ownerId` new text and broadcast the replace,
/// signed via `signerHandle`: the JNI bridge over `platform_wallet_decision_proposal_edit`
/// (Swift `ManagedPlatformWallet.editDecisionProposal`). The wallet reads the proposal once,
/// edits it at the revision after the one read and picks the signing key as the create
/// does; if another edit landed in between, Platform refuses this one (consensus code
/// 40106) rather than overwrite it.
///
/// `title`, `description` and `url` null keep their value; `removeUrl` removes the url
/// (the wallet refuses it together with a non-null `url`). `ownerId` and `proposalId` are
/// 32-byte ids.
///
/// Returns the confirmed document's canonical JSON at its new revision, its Rust string
/// freed here. Null after throwing; a proposal that does not exist is the wallet's
/// `NotFound`.
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_DecisionProposalsNative_edit(
    mut env: JNIEnv,
    _class: JClass,
    wallet_handle: jlong,
    owner_id: JByteArray,
    proposal_id: JByteArray,
    title: JString,
    description: JString,
    url: JString,
    remove_url: jboolean,
    signer_handle: jlong,
) -> jstring {
    guard(&mut env, ptr::null_mut(), |env| {
        let Some(owner) = read_bytes32(env, &owner_id, "ownerId") else {
            return ptr::null_mut();
        };
        let Some(proposal) = read_bytes32(env, &proposal_id, "proposalId") else {
            return ptr::null_mut();
        };
        let Some(title) = read_optional_cstring(env, &title, "title") else {
            return ptr::null_mut();
        };
        let Some(description) = read_optional_cstring(env, &description, "description") else {
            return ptr::null_mut();
        };
        let Some(url) = read_optional_cstring(env, &url, "url") else {
            return ptr::null_mut();
        };

        let edit = DecisionProposalEditFFI {
            title: c_ptr(&title),
            description: c_ptr(&description),
            url: c_ptr(&url),
            remove_url: remove_url != 0,
        };
        let mut out_json: *mut c_char = ptr::null_mut();
        // SAFETY: `owner` and `proposal` are 32 bytes, `edit` and the strings it points to
        // outlive the call, and the signer handle is the Kotlin manager's live signer.
        let result = unsafe {
            platform_wallet_decision_proposal_edit(
                wallet_handle as Handle,
                owner.as_ptr(),
                proposal.as_ptr(),
                &edit,
                signer_handle as *mut SignerHandle,
                &mut out_json,
            )
        };
        if take_pwffi_error(env, result) {
            return ptr::null_mut();
        }
        // SAFETY: on success `out_json` is the wallet's string, freed by this call only.
        let Some(json) = (unsafe { take_wallet_json(env, out_json, "decision proposal edit") })
        else {
            return ptr::null_mut();
        };
        env.new_string(json)
            .map(|s| s.into_raw())
            .unwrap_or(ptr::null_mut())
    })
}

// ── Reads ────────────────────────────────────────────────────────────────

/// A decision proposal by id as the proposal JSON object
/// (`dash_sdk_decision_proposal_fetch`); null when there is none, and after throwing.
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_DecisionProposalsNative_fetch(
    mut env: JNIEnv,
    _class: JClass,
    sdk: jlong,
    proposal_id: JString,
) -> jstring {
    guard(&mut env, ptr::null_mut(), |env| {
        let Some(id) = read_cstring(env, &proposal_id, "proposalId") else {
            return ptr::null_mut();
        };
        unsafe {
            let result = dash_sdk_decision_proposal_fetch(sdk as *const SDKHandle, id.as_ptr());
            string_or_null(env, result)
        }
    })
}

/// One page of the proposals by end date, closing soonest first
/// (`dash_sdk_decision_proposal_list_by_end_date`), as a JSON array of proposals. `from` is
/// included and `to` left out; 0 is no bound. `limit` 0 is Platform's cap of 100;
/// `startAfter` (base58) is null for the first page. Null after throwing.
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_DecisionProposalsNative_listByEndDate(
    mut env: JNIEnv,
    _class: JClass,
    sdk: jlong,
    end_date_from_ms: jlong,
    end_date_to_ms: jlong,
    limit: jint,
    start_after: JString,
) -> jstring {
    guard(&mut env, ptr::null_mut(), |env| {
        let Some(from) = to_unsigned(env, end_date_from_ms, "endDateFromMs") else {
            return ptr::null_mut();
        };
        let Some(to) = to_unsigned(env, end_date_to_ms, "endDateToMs") else {
            return ptr::null_mut();
        };
        let Some(limit) = to_unsigned(env, limit.into(), "limit") else {
            return ptr::null_mut();
        };
        let Some(start_after) = read_optional_cstring(env, &start_after, "startAfter") else {
            return ptr::null_mut();
        };
        unsafe {
            let result = dash_sdk_decision_proposal_list_by_end_date(
                sdk as *const SDKHandle,
                from,
                to,
                limit,
                c_ptr(&start_after),
            );
            string_or_null(env, result)
        }
    })
}

/// One page of an identity's proposals in filing order
/// (`dash_sdk_decision_proposal_list_by_owner`), as a JSON array of proposals. Paging as for
/// `listByEndDate`. Null after throwing.
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_DecisionProposalsNative_listByOwner(
    mut env: JNIEnv,
    _class: JClass,
    sdk: jlong,
    owner_id: JString,
    limit: jint,
    start_after: JString,
) -> jstring {
    guard(&mut env, ptr::null_mut(), |env| {
        let Some(owner) = read_cstring(env, &owner_id, "ownerId") else {
            return ptr::null_mut();
        };
        let Some(limit) = to_unsigned(env, limit.into(), "limit") else {
            return ptr::null_mut();
        };
        let Some(start_after) = read_optional_cstring(env, &start_after, "startAfter") else {
            return ptr::null_mut();
        };
        unsafe {
            let result = dash_sdk_decision_proposal_list_by_owner(
                sdk as *const SDKHandle,
                owner.as_ptr(),
                limit,
                c_ptr(&start_after),
            );
            string_or_null(env, result)
        }
    })
}

/// One page of every proposal, newest first
/// (`dash_sdk_decision_proposal_list_by_created_at`), as a JSON array of proposals. Paging
/// as for `listByEndDate`. Null after throwing.
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_DecisionProposalsNative_listByCreatedAt(
    mut env: JNIEnv,
    _class: JClass,
    sdk: jlong,
    limit: jint,
    start_after: JString,
) -> jstring {
    guard(&mut env, ptr::null_mut(), |env| {
        let Some(limit) = to_unsigned(env, limit.into(), "limit") else {
            return ptr::null_mut();
        };
        let Some(start_after) = read_optional_cstring(env, &start_after, "startAfter") else {
            return ptr::null_mut();
        };
        unsafe {
            let result = dash_sdk_decision_proposal_list_by_created_at(
                sdk as *const SDKHandle,
                limit,
                c_ptr(&start_after),
            );
            string_or_null(env, result)
        }
    })
}

/// The id of a proposal's yes/no poll as a plain base58 string, not JSON
/// (`dash_sdk_decision_proposal_get_vote_poll_id`). Null after throwing.
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_DecisionProposalsNative_getVotePollId(
    mut env: JNIEnv,
    _class: JClass,
    sdk: jlong,
    proposal_id: JString,
) -> jstring {
    guard(&mut env, ptr::null_mut(), |env| {
        let Some(id) = read_cstring(env, &proposal_id, "proposalId") else {
            return ptr::null_mut();
        };
        unsafe {
            let result =
                dash_sdk_decision_proposal_get_vote_poll_id(sdk as *const SDKHandle, id.as_ptr());
            string_or_null(env, result)
        }
    })
}

/// The status and tallies of a proposal's yes/no poll as JSON
/// (`dash_sdk_decision_proposal_get_vote_poll_state`); null when the poll was never opened,
/// and after throwing.
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_DecisionProposalsNative_getVotePollState(
    mut env: JNIEnv,
    _class: JClass,
    sdk: jlong,
    proposal_id: JString,
) -> jstring {
    guard(&mut env, ptr::null_mut(), |env| {
        let Some(id) = read_cstring(env, &proposal_id, "proposalId") else {
            return ptr::null_mut();
        };
        unsafe {
            let result = dash_sdk_decision_proposal_get_vote_poll_state(
                sdk as *const SDKHandle,
                id.as_ptr(),
            );
            string_or_null(env, result)
        }
    })
}

/// A masternode's answer on a proposal, `{"choice", "timesVoted"}`
/// (`dash_sdk_decision_proposal_get_masternode_vote`); null when it has not voted or the
/// poll finished, and after throwing. `voterProTxHash` is the 32-byte pro_tx_hash in wire
/// order, the same bytes `castVote` takes.
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_DecisionProposalsNative_getMasternodeVote(
    mut env: JNIEnv,
    _class: JClass,
    sdk: jlong,
    voter_pro_tx_hash: JByteArray,
    proposal_id: JString,
) -> jstring {
    guard(&mut env, ptr::null_mut(), |env| {
        let Some(pro_tx_hash) = read_bytes32(env, &voter_pro_tx_hash, "voterProTxHash") else {
            return ptr::null_mut();
        };
        let Some(id) = read_cstring(env, &proposal_id, "proposalId") else {
            return ptr::null_mut();
        };
        unsafe {
            let result = dash_sdk_decision_proposal_get_masternode_vote(
                sdk as *const SDKHandle,
                pro_tx_hash.as_ptr(),
                id.as_ptr(),
            );
            string_or_null(env, result)
        }
    })
}

// ── Vote ─────────────────────────────────────────────────────────────────

/// Vote on a decision proposal for a masternode and wait for the response
/// (`dash_sdk_decision_proposal_cast_vote`).
///
/// * `voteChoice` - `0` yes, `1` no, `2` abstain; rs-sdk-ffi refuses any other value. A
///   value outside `u8` is refused here rather than wrapped onto a valid answer.
/// * `voterProTxHash` / `votingPrivateKey` - each a 32-byte `byte[]`; the pro_tx_hash in wire
///   order, as for `castContestedResourceVote`. The key is read through a zeroizing buffer.
/// * `networkOrd` - `Network.ffiValue`, used for the signer's WIF encoding only.
///
/// Returns nothing on success; throws on error. A vote from a wallet that is not a
/// masternode is refused by Platform (no voter identity), the expected outcome.
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_DecisionProposalsNative_castVote(
    mut env: JNIEnv,
    _class: JClass,
    sdk: jlong,
    proposal_id: JString,
    vote_choice: jint,
    voter_pro_tx_hash: JByteArray,
    voting_private_key: JByteArray,
    network_ord: jint,
) {
    guard(&mut env, (), |env| {
        let Some(id) = read_cstring(env, &proposal_id, "proposalId") else {
            return;
        };
        let Some(choice) = to_unsigned(env, vote_choice.into(), "voteChoice") else {
            return;
        };
        let Some(pro_tx_hash) = read_bytes32(env, &voter_pro_tx_hash, "voterProTxHash") else {
            return;
        };
        let Some(voting_key) = read_key32_zeroizing(env, &voting_private_key, "votingPrivateKey")
        else {
            return;
        };
        let result = unsafe {
            dash_sdk_decision_proposal_cast_vote(
                sdk as *const SDKHandle,
                id.as_ptr(),
                choice,
                pro_tx_hash.as_ptr(),
                voting_key.as_ptr(),
                net_from_ord(network_ord),
            )
        };
        // Success carries no payload; take_error throws and frees on error.
        unsafe { take_error(env, &result) };
    })
}

#[cfg(test)]
mod tests {
    const DECISION_PROPOSALS_NATIVE_KT: &str = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../kotlin-sdk/sdk/src/main/kotlin/org/dashfoundation/dashsdk/ffi/DecisionProposalsNative.kt"
    ));

    /// The parameter types and the return type of `external fun {name}(...)` in
    /// `DecisionProposalsNative.kt`.
    fn kotlin_signature(name: &str) -> (Vec<String>, String) {
        let declaration = DECISION_PROPOSALS_NATIVE_KT
            .split(&format!("external fun {name}("))
            .nth(1)
            .unwrap_or_else(|| panic!("DecisionProposalsNative.kt declares {name}"));
        let (parameters, rest) = declaration
            .split_once(')')
            .expect("the parameter list closes");
        let types = parameters
            .split(',')
            .map(str::trim)
            .filter(|parameter| !parameter.is_empty())
            .map(|parameter| {
                let (_, kind) = parameter
                    .split_once(':')
                    .expect("a parameter is `name: Type`");
                kind.trim().to_string()
            })
            .collect();
        let returns = rest
            .lines()
            .next()
            .and_then(|line| line.trim().strip_prefix(':'))
            .map_or_else(|| "Unit".to_string(), |kind| kind.trim().to_string());
        (types, returns)
    }

    /// JNI binds an export by its name alone: a Kotlin declaration whose parameters drifted
    /// from the export's would hand it the wrong JVM values at run time, which only an
    /// instrumented test would otherwise catch. The wallet writes take the most arguments,
    /// and the create's last is the id out-array.
    #[test]
    fn should_declare_the_wallet_writes_as_the_exports_take_them() {
        assert_eq!(
            kotlin_signature("create"),
            (
                [
                    "Long",      // wallet_handle
                    "ByteArray", // owner_id
                    "String",    // title
                    "String",    // description
                    "String?",   // url
                    "Int",       // pass_percent
                    "Int",       // minimum_turnout_percent
                    "Long",      // end_date_ms
                    "Long",      // signer_handle
                    "ByteArray", // out_proposal_id
                ]
                .map(String::from)
                .to_vec(),
                "String".to_string()
            )
        );
        assert_eq!(
            kotlin_signature("edit"),
            (
                [
                    "Long",      // wallet_handle
                    "ByteArray", // owner_id
                    "ByteArray", // proposal_id
                    "String?",   // title
                    "String?",   // description
                    "String?",   // url
                    "Boolean",   // remove_url
                    "Long",      // signer_handle
                ]
                .map(String::from)
                .to_vec(),
                "String".to_string()
            )
        );
    }
}
