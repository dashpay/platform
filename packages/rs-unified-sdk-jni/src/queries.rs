//! JNI exports for read-only Platform queries: identities, DPNS names,
//! data contracts, documents. All payloads are JSON strings produced by
//! `rs-sdk-ffi`; parsing happens on the Kotlin side.
//!
//! Kotlin counterpart: `org.dashfoundation.dashsdk.ffi.QueriesNative`.

use crate::results::{unwrap_handle, unwrap_string};
use crate::support::guard;
use jni::objects::{JClass, JString};
use jni::sys::{jint, jlong, jstring};
use jni::JNIEnv;
use rs_sdk_ffi::{
    dash_sdk_calculate_token_id, dash_sdk_data_contract_destroy, dash_sdk_data_contract_fetch,
    dash_sdk_data_contract_fetch_json, dash_sdk_document_average, dash_sdk_document_count,
    dash_sdk_document_search, dash_sdk_document_sum, dash_sdk_dpns_check_availability,
    dash_sdk_dpns_get_usernames, dash_sdk_dpns_resolve, dash_sdk_dpns_search,
    dash_sdk_identity_fetch, dash_sdk_identity_fetch_balance,
    dash_sdk_identity_fetch_token_balances, dash_sdk_token_get_contract_info,
    dash_sdk_token_get_statuses, DashSDKDocumentSearchParams, DataContractHandle, SDKHandle,
};
use std::ffi::CString;
use std::os::raw::c_char;
use std::ptr;

/// Convert a nullable JString into an owned CString (None for null/invalid).
fn opt_c_string(env: &mut JNIEnv, value: &JString) -> Option<CString> {
    if value.is_null() {
        return None;
    }
    let s: String = env.get_string(value).ok()?.into();
    CString::new(s).ok()
}

fn c_ptr(opt: &Option<CString>) -> *const c_char {
    opt.as_ref().map_or(ptr::null(), |s| s.as_ptr())
}

/// Shorthand for the guard + required-string preamble every query shares.
macro_rules! require_cstr {
    ($env:expr, $val:expr) => {
        match opt_c_string($env, &$val) {
            Some(s) => s,
            None => {
                crate::support::throw_sdk_exception($env, 1, "required string argument was null");
                return ptr::null_mut();
            }
        }
    };
}

/// Fetch an identity as JSON. Returns null if the identity does not exist.
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_QueriesNative_identityFetch(
    mut env: JNIEnv,
    _class: JClass,
    sdk: jlong,
    identity_id: JString,
) -> jstring {
    guard(&mut env, ptr::null_mut(), |env| {
        let id = require_cstr!(env, identity_id);
        let result = unsafe { dash_sdk_identity_fetch(sdk as *const SDKHandle, id.as_ptr()) };
        unsafe { unwrap_string(env, result) }
            .map(|s| s.into_raw())
            .unwrap_or(ptr::null_mut())
    })
}

/// Fetch an identity's balance in credits, as a decimal string.
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_QueriesNative_identityFetchBalance(
    mut env: JNIEnv,
    _class: JClass,
    sdk: jlong,
    identity_id: JString,
) -> jstring {
    guard(&mut env, ptr::null_mut(), |env| {
        let id = require_cstr!(env, identity_id);
        let result =
            unsafe { dash_sdk_identity_fetch_balance(sdk as *const SDKHandle, id.as_ptr()) };
        unsafe { unwrap_string(env, result) }
            .map(|s| s.into_raw())
            .unwrap_or(ptr::null_mut())
    })
}

/// Resolve a DPNS name to its identity record (JSON), or null if unregistered.
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_QueriesNative_dpnsResolve(
    mut env: JNIEnv,
    _class: JClass,
    sdk: jlong,
    name: JString,
) -> jstring {
    guard(&mut env, ptr::null_mut(), |env| {
        let name = require_cstr!(env, name);
        let result = unsafe { dash_sdk_dpns_resolve(sdk as *const SDKHandle, name.as_ptr()) };
        unsafe { unwrap_string(env, result) }
            .map(|s| s.into_raw())
            .unwrap_or(ptr::null_mut())
    })
}

/// Check DPNS label availability; returns a JSON availability object.
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_QueriesNative_dpnsCheckAvailability(
    mut env: JNIEnv,
    _class: JClass,
    sdk: jlong,
    label: JString,
) -> jstring {
    guard(&mut env, ptr::null_mut(), |env| {
        let label = require_cstr!(env, label);
        let result =
            unsafe { dash_sdk_dpns_check_availability(sdk as *const SDKHandle, label.as_ptr()) };
        unsafe { unwrap_string(env, result) }
            .map(|s| s.into_raw())
            .unwrap_or(ptr::null_mut())
    })
}

/// DPNS usernames owned by an identity (JSON array).
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_QueriesNative_dpnsGetUsernames(
    mut env: JNIEnv,
    _class: JClass,
    sdk: jlong,
    identity_id: JString,
    limit: jint,
) -> jstring {
    guard(&mut env, ptr::null_mut(), |env| {
        let id = require_cstr!(env, identity_id);
        let result = unsafe {
            dash_sdk_dpns_get_usernames(sdk as *const SDKHandle, id.as_ptr(), limit.max(0) as u32)
        };
        unsafe { unwrap_string(env, result) }
            .map(|s| s.into_raw())
            .unwrap_or(ptr::null_mut())
    })
}

/// Search DPNS names by prefix (JSON array).
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_QueriesNative_dpnsSearch(
    mut env: JNIEnv,
    _class: JClass,
    sdk: jlong,
    prefix: JString,
    limit: jint,
) -> jstring {
    guard(&mut env, ptr::null_mut(), |env| {
        let prefix = require_cstr!(env, prefix);
        let result = unsafe {
            dash_sdk_dpns_search(
                sdk as *const SDKHandle,
                prefix.as_ptr(),
                limit.max(0) as u32,
            )
        };
        unsafe { unwrap_string(env, result) }
            .map(|s| s.into_raw())
            .unwrap_or(ptr::null_mut())
    })
}

/// Fetch a data contract as JSON.
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_QueriesNative_dataContractFetchJson(
    mut env: JNIEnv,
    _class: JClass,
    sdk: jlong,
    contract_id: JString,
) -> jstring {
    guard(&mut env, ptr::null_mut(), |env| {
        let id = require_cstr!(env, contract_id);
        let result =
            unsafe { dash_sdk_data_contract_fetch_json(sdk as *const SDKHandle, id.as_ptr()) };
        unsafe { unwrap_string(env, result) }
            .map(|s| s.into_raw())
            .unwrap_or(ptr::null_mut())
    })
}

/// Fetch a data contract and return an opaque handle for document queries.
/// Must be released with [`Java_org_dashfoundation_dashsdk_ffi_QueriesNative_dataContractDestroy`].
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_QueriesNative_dataContractFetch(
    mut env: JNIEnv,
    _class: JClass,
    sdk: jlong,
    contract_id: JString,
) -> jlong {
    guard(&mut env, 0, |env| {
        let id = match opt_c_string(env, &contract_id) {
            Some(s) => s,
            None => {
                crate::support::throw_sdk_exception(env, 1, "contractId was null");
                return 0;
            }
        };
        let result = unsafe { dash_sdk_data_contract_fetch(sdk as *const SDKHandle, id.as_ptr()) };
        unsafe { unwrap_handle(env, result) }
    })
}

/// Release a data contract handle from `dataContractFetch`. Safe on 0.
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_QueriesNative_dataContractDestroy(
    mut env: JNIEnv,
    _class: JClass,
    handle: jlong,
) {
    guard(&mut env, (), |_| unsafe {
        dash_sdk_data_contract_destroy(handle as *mut DataContractHandle)
    });
}

/// Search documents of a type; returns a JSON array of documents.
/// `whereJson`/`orderByJson` may be null.
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_QueriesNative_documentSearch(
    mut env: JNIEnv,
    _class: JClass,
    sdk: jlong,
    contract_handle: jlong,
    document_type: JString,
    where_json: JString,
    order_by_json: JString,
    limit: jint,
    start_at: jint,
) -> jstring {
    guard(&mut env, ptr::null_mut(), |env| {
        let doc_type = require_cstr!(env, document_type);
        let where_c = opt_c_string(env, &where_json);
        let order_c = opt_c_string(env, &order_by_json);
        let params = DashSDKDocumentSearchParams {
            data_contract_handle: contract_handle as *const DataContractHandle,
            document_type: doc_type.as_ptr(),
            where_json: c_ptr(&where_c),
            order_by_json: c_ptr(&order_c),
            limit: limit.max(0) as u32,
            start_at: start_at.max(0) as u32,
        };
        let result = unsafe { dash_sdk_document_search(sdk as *const SDKHandle, &params) };
        unsafe { unwrap_string(env, result) }
            .map(|s| s.into_raw())
            .unwrap_or(ptr::null_mut())
    })
}

/// Count documents (optionally grouped); returns a JSON result object.
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_QueriesNative_documentCount(
    mut env: JNIEnv,
    _class: JClass,
    sdk: jlong,
    contract_handle: jlong,
    document_type: JString,
    where_json: JString,
    order_by_json: JString,
    group_by_json: JString,
    limit: jlong,
) -> jstring {
    guard(&mut env, ptr::null_mut(), |env| {
        let doc_type = require_cstr!(env, document_type);
        let where_c = opt_c_string(env, &where_json);
        let order_c = opt_c_string(env, &order_by_json);
        let group_c = opt_c_string(env, &group_by_json);
        let result = unsafe {
            dash_sdk_document_count(
                sdk as *const SDKHandle,
                contract_handle as *const DataContractHandle,
                doc_type.as_ptr(),
                c_ptr(&where_c),
                c_ptr(&order_c),
                c_ptr(&group_c),
                limit,
            )
        };
        unsafe { unwrap_string(env, result) }
            .map(|s| s.into_raw())
            .unwrap_or(ptr::null_mut())
    })
}

/// Sum a numeric document property; returns a JSON result object.
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_QueriesNative_documentSum(
    mut env: JNIEnv,
    _class: JClass,
    sdk: jlong,
    contract_handle: jlong,
    document_type: JString,
    sum_property: JString,
    where_json: JString,
    order_by_json: JString,
    group_by_json: JString,
    limit: jlong,
) -> jstring {
    guard(&mut env, ptr::null_mut(), |env| {
        let doc_type = require_cstr!(env, document_type);
        let property = require_cstr!(env, sum_property);
        let where_c = opt_c_string(env, &where_json);
        let order_c = opt_c_string(env, &order_by_json);
        let group_c = opt_c_string(env, &group_by_json);
        let result = unsafe {
            dash_sdk_document_sum(
                sdk as *const SDKHandle,
                contract_handle as *const DataContractHandle,
                doc_type.as_ptr(),
                property.as_ptr(),
                c_ptr(&where_c),
                c_ptr(&order_c),
                c_ptr(&group_c),
                limit,
            )
        };
        unsafe { unwrap_string(env, result) }
            .map(|s| s.into_raw())
            .unwrap_or(ptr::null_mut())
    })
}

/// Average a numeric document property; returns a JSON result object.
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_QueriesNative_documentAverage(
    mut env: JNIEnv,
    _class: JClass,
    sdk: jlong,
    contract_handle: jlong,
    document_type: JString,
    sum_property: JString,
    where_json: JString,
    order_by_json: JString,
    group_by_json: JString,
    limit: jlong,
) -> jstring {
    guard(&mut env, ptr::null_mut(), |env| {
        let doc_type = require_cstr!(env, document_type);
        let property = require_cstr!(env, sum_property);
        let where_c = opt_c_string(env, &where_json);
        let order_c = opt_c_string(env, &order_by_json);
        let group_c = opt_c_string(env, &group_by_json);
        let result = unsafe {
            dash_sdk_document_average(
                sdk as *const SDKHandle,
                contract_handle as *const DataContractHandle,
                doc_type.as_ptr(),
                property.as_ptr(),
                c_ptr(&where_c),
                c_ptr(&order_c),
                c_ptr(&group_c),
                limit,
            )
        };
        unsafe { unwrap_string(env, result) }
            .map(|s| s.into_raw())
            .unwrap_or(ptr::null_mut())
    })
}

/// Compute the canonical base58 token id for a data-contract id + token
/// position. Pure computation — no SDK handle, no network. Returns the
/// base58 token-id string, or null after throwing on a malformed contract
/// id. Backs the proven-balance row keying (`ProvenBalances`) and the
/// token-action permission screen's live-balance lookups.
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_QueriesNative_calculateTokenId(
    mut env: JNIEnv,
    _class: JClass,
    contract_id: JString,
    position: jint,
) -> jstring {
    guard(&mut env, ptr::null_mut(), |env| {
        let contract = require_cstr!(env, contract_id);
        let pos = position.clamp(0, u16::MAX as jint) as u16;
        let result = unsafe { dash_sdk_calculate_token_id(contract.as_ptr(), pos) };
        unsafe { unwrap_string(env, result) }
            .map(|s| s.into_raw())
            .unwrap_or(ptr::null_mut())
    })
}

/// Fetch an identity's balances for the given `token_ids` (a
/// comma-separated list of base58 token ids). Returns a JSON object
/// `{"<base58_tokenId>": <u64_balance>, ...}`, or null after throwing.
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_QueriesNative_identityFetchTokenBalances(
    mut env: JNIEnv,
    _class: JClass,
    sdk: jlong,
    identity_id: JString,
    token_ids: JString,
) -> jstring {
    guard(&mut env, ptr::null_mut(), |env| {
        let id = require_cstr!(env, identity_id);
        let ids = require_cstr!(env, token_ids);
        let result = unsafe {
            dash_sdk_identity_fetch_token_balances(
                sdk as *const SDKHandle,
                id.as_ptr(),
                ids.as_ptr(),
            )
        };
        unsafe { unwrap_string(env, result) }
            .map(|s| s.into_raw())
            .unwrap_or(ptr::null_mut())
    })
}

/// Fetch on-chain status flags for the given `token_ids` (comma-separated
/// base58). Returns a JSON object `{"<base58_tokenId>": {"paused": bool},
/// ...}`, or null after throwing. Backs the pause-flag reconciliation in
/// the token-action permission screen.
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_QueriesNative_tokenGetStatuses(
    mut env: JNIEnv,
    _class: JClass,
    sdk: jlong,
    token_ids: JString,
) -> jstring {
    guard(&mut env, ptr::null_mut(), |env| {
        let ids = require_cstr!(env, token_ids);
        let result = unsafe { dash_sdk_token_get_statuses(sdk as *const SDKHandle, ids.as_ptr()) };
        unsafe { unwrap_string(env, result) }
            .map(|s| s.into_raw())
            .unwrap_or(ptr::null_mut())
    })
}

/// Fetch the data-contract locator for a single base58 `token_id`.
/// Returns a JSON object `{"contract_id": "<base58>",
/// "token_contract_position": <u16>}`, null if the token is unknown, or
/// null after throwing on error. Backs the token contract-info fallback in
/// the tokens list.
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_QueriesNative_tokenGetContractInfo(
    mut env: JNIEnv,
    _class: JClass,
    sdk: jlong,
    token_id: JString,
) -> jstring {
    guard(&mut env, ptr::null_mut(), |env| {
        let id = require_cstr!(env, token_id);
        let result =
            unsafe { dash_sdk_token_get_contract_info(sdk as *const SDKHandle, id.as_ptr()) };
        unsafe { unwrap_string(env, result) }
            .map(|s| s.into_raw())
            .unwrap_or(ptr::null_mut())
    })
}
