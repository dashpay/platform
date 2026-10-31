//! JNI exports for decision proposals (protocol version 15): yes/no questions any identity
//! puts to the masternodes, as `proposal` documents of the decision proposals system
//! contract. Each export marshals ONE `dash_sdk_decision_proposal_*` function of
//! `rs-sdk-ffi` (or the document free its handles need): it reads the JVM arguments, calls
//! the C function, copies the result into a JVM value and frees every Rust allocation the
//! result carried before returning.
//!
//! Creating and editing a proposal are sequences of these calls, run by Kotlin
//! (`voting.DecisionProposalTransactions`), never stitched together here: build (or fetch and
//! edit) a document, read its properties JSON, free the document handles, then put the
//! properties through the platform wallet's document create or replace
//! (`TransactionsNative.documentCreate` / `documentReplace`). Document handles cross as
//! `jlong` and are released with `documentFree`.
//!
//! Errors follow the rs-sdk-ffi convention of `queries.rs`: a `DashSDKResult` error throws
//! `DashSDKException` through [`crate::results::take_error`]. An argument the JVM value
//! cannot carry into the C type (a null required string, a byte array of the wrong length,
//! a number out of the C type's range) throws code 1 (`InvalidParameter`) before any C call;
//! everything rs-sdk-ffi and rs-sdk check is left to them.
//!
//! Kotlin counterpart: `org.dashfoundation.dashsdk.ffi.DecisionProposalsNative`.

use crate::results::{take_error, unwrap_handle, unwrap_string};
use crate::support::{guard, net_from_ord, throw_sdk_exception};
use jni::objects::{JByteArray, JClass, JString, JValue};
use jni::sys::{jboolean, jbyteArray, jint, jlong, jobject, jstring};
use jni::JNIEnv;
use rs_sdk_ffi::{
    dash_sdk_decision_proposal_build, dash_sdk_decision_proposal_build_edit,
    dash_sdk_decision_proposal_cast_vote, dash_sdk_decision_proposal_contract_id,
    dash_sdk_decision_proposal_contract_id_bytes, dash_sdk_decision_proposal_document_type_name,
    dash_sdk_decision_proposal_fetch, dash_sdk_decision_proposal_fetch_document,
    dash_sdk_decision_proposal_get_masternode_vote, dash_sdk_decision_proposal_get_vote_poll_id,
    dash_sdk_decision_proposal_get_vote_poll_state, dash_sdk_decision_proposal_list_by_created_at,
    dash_sdk_decision_proposal_list_by_end_date, dash_sdk_decision_proposal_list_by_owner,
    dash_sdk_decision_proposal_properties_json, dash_sdk_document_create_result_free,
    dash_sdk_document_handle_destroy, DashSDKDecisionProposalEdit, DashSDKDecisionProposalParams,
    DashSDKDocumentCreateResult, DashSDKResult, DocumentHandle, SDKHandle,
};
use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::ptr;

/// JNI-internal name of the Kotlin class `build` returns.
const BUILD_RESULT_CLASS: &str = "org/dashfoundation/dashsdk/ffi/DecisionProposalBuildNativeResult";

/// Constructor descriptor of [`BUILD_RESULT_CLASS`]: `(documentHandle: Long, entropy: ByteArray)`.
const BUILD_RESULT_CONSTRUCTOR: &str = "(J[B)V";

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

/// Frees a document handle on drop unless disarmed: it holds a handle from the C call that
/// returned it until the JVM value carrying it to Kotlin exists, so no failure in between
/// leaks it.
struct DocumentHandleGuard(*mut DocumentHandle);

impl DocumentHandleGuard {
    /// Hands the handle on; the guard no longer frees it.
    fn disarm(&mut self) -> *mut DocumentHandle {
        std::mem::replace(&mut self.0, ptr::null_mut())
    }
}

impl Drop for DocumentHandleGuard {
    fn drop(&mut self) {
        // SAFETY: the handle came from rs-sdk-ffi and nothing else owns it; null is a no-op.
        unsafe { dash_sdk_document_handle_destroy(self.0) };
    }
}

// ── Constants ────────────────────────────────────────────────────────────

/// The id of the decision proposals system contract, base58
/// (`dash_sdk_decision_proposal_contract_id`): the `dataContractId` `buildEdit` takes.
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
/// (`dash_sdk_decision_proposal_contract_id_bytes`): the `contractId` the platform wallet's
/// document create and replace take.
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

// ── Create ───────────────────────────────────────────────────────────────

/// Build a decision proposal of `ownerIdentityId` (`dash_sdk_decision_proposal_build`).
/// Nothing is broadcast.
///
/// Returns a `DecisionProposalBuildNativeResult(documentHandle, entropy)`: the C result
/// struct is freed here with `dash_sdk_document_create_result_free`, which leaves the
/// document alive, and the document handle passes to Kotlin, which frees it with
/// `documentFree`. If the result object cannot be made, the handle is freed here before
/// throwing. Null after throwing.
///
/// `url` may be null (no url). `passPercent` and `minimumTurnoutPercent` must fit a `u8` and
/// `endDateMs` a `u64`; the limits on them are Platform's.
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_DecisionProposalsNative_build(
    mut env: JNIEnv,
    _class: JClass,
    sdk: jlong,
    owner_identity_id: JString,
    title: JString,
    description: JString,
    url: JString,
    pass_percent: jint,
    minimum_turnout_percent: jint,
    end_date_ms: jlong,
) -> jobject {
    guard(&mut env, ptr::null_mut(), |env| {
        let Some(owner) = read_cstring(env, &owner_identity_id, "ownerIdentityId") else {
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

        let params = DashSDKDecisionProposalParams {
            title: title.as_ptr(),
            description: description.as_ptr(),
            url: c_ptr(&url),
            pass_percent,
            minimum_turnout_percent,
            end_date_ms,
        };
        let result = unsafe {
            dash_sdk_decision_proposal_build(sdk as *const SDKHandle, owner.as_ptr(), &params)
        };
        unsafe { build_result_object(env, result) }
    })
}

/// The `DecisionProposalBuildNativeResult` for the result of
/// `dash_sdk_decision_proposal_build`, whose payload is a `DashSDKDocumentCreateResult`.
/// The struct is freed; the document handle is handed to Kotlin, or freed when the object
/// cannot be made. Null after throwing.
///
/// # Safety
/// `result` must be freshly returned by `dash_sdk_decision_proposal_build`.
unsafe fn build_result_object(env: &mut JNIEnv, result: DashSDKResult) -> jobject {
    if take_error(env, &result) {
        return ptr::null_mut();
    }
    if result.data.is_null() {
        throw_sdk_exception(
            env,
            99,
            "decision proposal build returned success but no document",
        );
        return ptr::null_mut();
    }
    let created = result.data as *mut DashSDKDocumentCreateResult;
    // Take the handle into the guard before the struct is freed, so it is never unowned.
    let mut document = DocumentHandleGuard((*created).document_handle);
    let entropy = (*created).entropy;
    // Frees the struct only; the document stays alive in the guard.
    dash_sdk_document_create_result_free(created);

    let Ok(entropy) = env.byte_array_from_slice(&entropy) else {
        let _ = env.exception_clear();
        throw_sdk_exception(
            env,
            99,
            "decision proposal entropy byte[] allocation failed",
        );
        return ptr::null_mut();
    };
    let object = env.new_object(
        BUILD_RESULT_CLASS,
        BUILD_RESULT_CONSTRUCTOR,
        &[
            JValue::Long(document.0 as jlong),
            JValue::Object(entropy.as_ref()),
        ],
    );
    match object {
        Ok(object) => {
            document.disarm();
            object.into_raw()
        }
        Err(_) => {
            let _ = env.exception_clear();
            throw_sdk_exception(env, 99, "decision proposal build result allocation failed");
            ptr::null_mut()
        }
    }
}

/// The properties of a proposal document as a JSON object
/// (`dash_sdk_decision_proposal_properties_json`): the `propertiesJson` the platform wallet's
/// document create (for a document from `build`) and replace (for one from `buildEdit`)
/// take. The caller still owns and frees `document`. Null after throwing.
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_DecisionProposalsNative_propertiesJson(
    mut env: JNIEnv,
    _class: JClass,
    document: jlong,
) -> jstring {
    guard(&mut env, ptr::null_mut(), |env| unsafe {
        let result = dash_sdk_decision_proposal_properties_json(document as *const DocumentHandle);
        string_or_null(env, result)
    })
}

// ── Edit ─────────────────────────────────────────────────────────────────

/// Fetch a decision proposal as a document handle
/// (`dash_sdk_decision_proposal_fetch_document`), the stored proposal `buildEdit` edits.
/// Returns 0 when there is no such proposal, and after throwing. Free a non-zero handle with
/// `documentFree`.
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_DecisionProposalsNative_fetchDocument(
    mut env: JNIEnv,
    _class: JClass,
    sdk: jlong,
    proposal_id: JString,
) -> jlong {
    guard(&mut env, 0, |env| {
        let Some(id) = read_cstring(env, &proposal_id, "proposalId") else {
            return 0;
        };
        let result = unsafe {
            dash_sdk_decision_proposal_fetch_document(sdk as *const SDKHandle, id.as_ptr())
        };
        unsafe { unwrap_handle(env, result) }
    })
}

/// A copy of the stored proposal `proposal` with new text and its next revision
/// (`dash_sdk_decision_proposal_build_edit`). Pure: no network. `title`, `description` and
/// `url` null keep their value; `removeUrl` removes the url. Returns a new document handle,
/// 0 after throwing; the caller still owns and frees `proposal`, and frees the new handle
/// with `documentFree`.
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_DecisionProposalsNative_buildEdit(
    mut env: JNIEnv,
    _class: JClass,
    proposal: jlong,
    data_contract_id: JString,
    document_type_name: JString,
    title: JString,
    description: JString,
    url: JString,
    remove_url: jboolean,
) -> jlong {
    guard(&mut env, 0, |env| {
        let Some(contract_id) = read_cstring(env, &data_contract_id, "dataContractId") else {
            return 0;
        };
        let Some(type_name) = read_cstring(env, &document_type_name, "documentTypeName") else {
            return 0;
        };
        let Some(title) = read_optional_cstring(env, &title, "title") else {
            return 0;
        };
        let Some(description) = read_optional_cstring(env, &description, "description") else {
            return 0;
        };
        let Some(url) = read_optional_cstring(env, &url, "url") else {
            return 0;
        };

        let edit = DashSDKDecisionProposalEdit {
            title: c_ptr(&title),
            description: c_ptr(&description),
            url: c_ptr(&url),
            remove_url: remove_url != 0,
        };
        let result = unsafe {
            dash_sdk_decision_proposal_build_edit(
                proposal as *const DocumentHandle,
                contract_id.as_ptr(),
                type_name.as_ptr(),
                &edit,
            )
        };
        unsafe { unwrap_handle(env, result) }
    })
}

/// Free a document handle from `build`, `fetchDocument` or `buildEdit`
/// (`dash_sdk_document_handle_destroy`, which `dash_sdk_document_free` aliases). Safe on 0.
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_DecisionProposalsNative_documentFree(
    mut env: JNIEnv,
    _class: JClass,
    document: jlong,
) {
    guard(&mut env, (), |_| unsafe {
        dash_sdk_document_handle_destroy(document as *mut DocumentHandle)
    });
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
    use super::{BUILD_RESULT_CLASS, BUILD_RESULT_CONSTRUCTOR};

    const DECISION_PROPOSALS_NATIVE_KT: &str = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../kotlin-sdk/sdk/src/main/kotlin/org/dashfoundation/dashsdk/ffi/DecisionProposalsNative.kt"
    ));

    /// `build` constructs the Kotlin result by class name and descriptor, which only an
    /// instrumented test would otherwise catch drifting: the class must sit where
    /// [`BUILD_RESULT_CLASS`] says and take `(Long, ByteArray)` in that order.
    #[test]
    fn should_construct_the_build_result_kotlin_declares() {
        let (package, class) = BUILD_RESULT_CLASS
            .rsplit_once('/')
            .expect("a JNI class name has a package");
        assert!(
            DECISION_PROPOSALS_NATIVE_KT
                .lines()
                .any(|line| line.trim() == format!("package {}", package.replace('/', "."))),
            "DecisionProposalsNative.kt is not in package {package}"
        );
        let declaration = DECISION_PROPOSALS_NATIVE_KT
            .split(&format!("class {class}("))
            .nth(1)
            .expect("DecisionProposalsNative.kt declares the build result class");
        let parameters: Vec<&str> = declaration
            .split(')')
            .next()
            .expect("the constructor has a parameter list")
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .collect();
        assert_eq!(
            parameters,
            [
                "@JvmField val documentHandle: Long,",
                "@JvmField val entropy: ByteArray,"
            ]
        );
        assert_eq!(BUILD_RESULT_CONSTRUCTOR, "(J[B)V");
    }
}
