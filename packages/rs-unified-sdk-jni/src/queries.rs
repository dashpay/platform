//! JNI exports for read-only Platform queries: identities, DPNS names,
//! data contracts, documents. All payloads are JSON strings produced by
//! `rs-sdk-ffi`; parsing happens on the Kotlin side.
//!
//! Kotlin counterpart: `org.dashfoundation.dashsdk.ffi.QueriesNative`.

use crate::results::{
    unwrap_address_info, unwrap_address_info_map, unwrap_handle, unwrap_identity_balance_map,
    unwrap_string,
};
use crate::support::{guard, throw_sdk_exception};
use jni::objects::{JByteArray, JClass, JObject, JObjectArray, JString};
use jni::sys::{jboolean, jint, jlong, jobject, jstring};
use jni::JNIEnv;
use rs_sdk_ffi::{
    dash_sdk_add_known_contracts, dash_sdk_address_fetch_info, dash_sdk_addresses_fetch_infos,
    dash_sdk_calculate_token_id, dash_sdk_contested_resource_get_identity_votes,
    dash_sdk_contested_resource_get_resources, dash_sdk_contested_resource_get_vote_state,
    dash_sdk_contested_resource_get_voters_for_identity, dash_sdk_data_contract_destroy,
    dash_sdk_data_contract_fetch, dash_sdk_data_contract_fetch_json,
    dash_sdk_data_contract_fetch_result_free, dash_sdk_data_contract_fetch_with_serialization,
    dash_sdk_document_average, dash_sdk_document_count, dash_sdk_document_search,
    dash_sdk_document_sum, dash_sdk_dpns_check_availability, dash_sdk_dpns_get_usernames,
    dash_sdk_dpns_resolve, dash_sdk_dpns_search, dash_sdk_evonode_get_proposed_epoch_blocks_by_ids,
    dash_sdk_evonode_get_proposed_epoch_blocks_by_range, dash_sdk_group_get_action_signers,
    dash_sdk_group_get_actions, dash_sdk_group_get_info, dash_sdk_group_get_infos,
    dash_sdk_identities_fetch_balances, dash_sdk_identities_fetch_contract_keys,
    dash_sdk_identity_fetch, dash_sdk_identity_fetch_balance,
    dash_sdk_identity_fetch_balance_and_revision,
    dash_sdk_identity_fetch_by_non_unique_public_key_hash,
    dash_sdk_identity_fetch_by_public_key_hash, dash_sdk_identity_fetch_contract_nonce,
    dash_sdk_identity_fetch_nonce, dash_sdk_identity_fetch_public_keys,
    dash_sdk_identity_fetch_token_balances, dash_sdk_protocol_version_get_upgrade_state,
    dash_sdk_protocol_version_get_upgrade_vote_status, dash_sdk_system_get_current_quorums_info,
    dash_sdk_system_get_epochs_info, dash_sdk_system_get_path_elements,
    dash_sdk_system_get_prefunded_specialized_balance,
    dash_sdk_system_get_total_credits_in_platform, dash_sdk_token_get_contract_info,
    dash_sdk_token_get_direct_purchase_prices,
    dash_sdk_token_get_perpetual_distribution_last_claim,
    dash_sdk_token_get_pre_programmed_distributions, dash_sdk_token_get_statuses,
    dash_sdk_token_get_total_supply, dash_sdk_voting_get_vote_polls_by_end_date,
    DashSDKDocumentSearchParams, DataContractHandle, SDKHandle,
};
use std::ffi::{CStr, CString};
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

// ── Identity: keys, revision, nonce, by-pubkey-hash, batch balances ──────

/// Fetch an identity's public keys as a JSON array. Null if absent.
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_QueriesNative_identityFetchPublicKeys(
    mut env: JNIEnv,
    _class: JClass,
    sdk: jlong,
    identity_id: JString,
) -> jstring {
    guard(&mut env, ptr::null_mut(), |env| {
        let id = require_cstr!(env, identity_id);
        let result =
            unsafe { dash_sdk_identity_fetch_public_keys(sdk as *const SDKHandle, id.as_ptr()) };
        unsafe { unwrap_string(env, result) }
            .map(|s| s.into_raw())
            .unwrap_or(ptr::null_mut())
    })
}

/// Fetch an identity's balance + revision as a JSON object. Null if absent.
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_QueriesNative_identityFetchBalanceAndRevision(
    mut env: JNIEnv,
    _class: JClass,
    sdk: jlong,
    identity_id: JString,
) -> jstring {
    guard(&mut env, ptr::null_mut(), |env| {
        let id = require_cstr!(env, identity_id);
        let result = unsafe {
            dash_sdk_identity_fetch_balance_and_revision(sdk as *const SDKHandle, id.as_ptr())
        };
        unsafe { unwrap_string(env, result) }
            .map(|s| s.into_raw())
            .unwrap_or(ptr::null_mut())
    })
}

/// Fetch the identity that owns a unique public-key hash (hex). JSON, or
/// null if none.
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_QueriesNative_identityFetchByPublicKeyHash(
    mut env: JNIEnv,
    _class: JClass,
    sdk: jlong,
    public_key_hash: JString,
) -> jstring {
    guard(&mut env, ptr::null_mut(), |env| {
        let hash = require_cstr!(env, public_key_hash);
        let result = unsafe {
            dash_sdk_identity_fetch_by_public_key_hash(sdk as *const SDKHandle, hash.as_ptr())
        };
        unsafe { unwrap_string(env, result) }
            .map(|s| s.into_raw())
            .unwrap_or(ptr::null_mut())
    })
}

/// Fetch identities sharing a non-unique public-key hash (hex). Returns a
/// JSON array; `startAfter` (base58 identity id) may be null for the first
/// page.
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_QueriesNative_identityFetchByNonUniquePublicKeyHash(
    mut env: JNIEnv,
    _class: JClass,
    sdk: jlong,
    public_key_hash: JString,
    start_after: JString,
) -> jstring {
    guard(&mut env, ptr::null_mut(), |env| {
        let hash = require_cstr!(env, public_key_hash);
        let start = opt_c_string(env, &start_after);
        let result = unsafe {
            dash_sdk_identity_fetch_by_non_unique_public_key_hash(
                sdk as *const SDKHandle,
                hash.as_ptr(),
                c_ptr(&start),
            )
        };
        unsafe { unwrap_string(env, result) }
            .map(|s| s.into_raw())
            .unwrap_or(ptr::null_mut())
    })
}

/// Fetch balances for many identities. `identityIds` is a flat `byte[]` of
/// `n * 32` bytes (each 32-byte id concatenated); Kotlin decodes base58 ids
/// to raw bytes. Returns a JSON array
/// `[{"identityIdHex":..,"balance":..,"found":..}]`, or null after throwing.
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_QueriesNative_identitiesFetchBalances(
    mut env: JNIEnv,
    _class: JClass,
    sdk: jlong,
    identity_ids: JByteArray,
) -> jstring {
    guard(&mut env, ptr::null_mut(), |env| {
        let bytes = match env.convert_byte_array(&identity_ids) {
            Ok(b) => b,
            Err(_) => {
                let _ = env.exception_clear();
                throw_sdk_exception(env, 1, "identityIds byte[] was null/invalid");
                return ptr::null_mut();
            }
        };
        if bytes.len() % 32 != 0 {
            throw_sdk_exception(
                env,
                1,
                &format!(
                    "identityIds length must be a multiple of 32, got {}",
                    bytes.len()
                ),
            );
            return ptr::null_mut();
        }
        let count = bytes.len() / 32;
        // SAFETY: `bytes` is `count * 32` bytes; reinterpreting as `count`
        // contiguous `[u8; 32]` is layout-compatible and the pointer stays
        // valid for the synchronous FFI call (the Vec outlives it).
        let ids_ptr = bytes.as_ptr() as *const [u8; 32];
        let result =
            unsafe { dash_sdk_identities_fetch_balances(sdk as *const SDKHandle, ids_ptr, count) };
        unsafe { unwrap_identity_balance_map(env, result) }
            .map(|s| s.into_raw())
            .unwrap_or(ptr::null_mut())
    })
}

// ── Addresses ────────────────────────────────────────────────────────────

/// Fetch a single Platform address's balance + nonce. `address` is the raw
/// 21-byte address (type byte + 20-byte hash). Returns a JSON object
/// `{"addressHex":..,"nonce":..,"balance":..,"found":..}`, or null after
/// throwing.
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_QueriesNative_addressFetchInfo(
    mut env: JNIEnv,
    _class: JClass,
    sdk: jlong,
    address: JByteArray,
) -> jstring {
    guard(&mut env, ptr::null_mut(), |env| {
        let bytes = match env.convert_byte_array(&address) {
            Ok(b) => b,
            Err(_) => {
                let _ = env.exception_clear();
                throw_sdk_exception(env, 1, "address byte[] was null/invalid");
                return ptr::null_mut();
            }
        };
        // SAFETY: `bytes` outlives the synchronous FFI call.
        let result = unsafe {
            dash_sdk_address_fetch_info(sdk as *const SDKHandle, bytes.as_ptr(), bytes.len())
        };
        unsafe { unwrap_address_info(env, result) }
            .map(|s| s.into_raw())
            .unwrap_or(ptr::null_mut())
    })
}

/// Fetch balance + nonce for many Platform addresses. `addresses` is a flat
/// `byte[]` of `count * addressLen` bytes (each address of identical
/// `addressLen`, typically 21). Returns a JSON array of per-address objects,
/// or null after throwing.
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_QueriesNative_addressesFetchInfos(
    mut env: JNIEnv,
    _class: JClass,
    sdk: jlong,
    addresses: JByteArray,
    address_len: jint,
) -> jstring {
    guard(&mut env, ptr::null_mut(), |env| {
        let addr_len = address_len.max(0) as usize;
        let bytes = match env.convert_byte_array(&addresses) {
            Ok(b) => b,
            Err(_) => {
                let _ = env.exception_clear();
                throw_sdk_exception(env, 1, "addresses byte[] was null/invalid");
                return ptr::null_mut();
            }
        };
        if addr_len == 0 || bytes.len() % addr_len != 0 {
            throw_sdk_exception(
                env,
                1,
                &format!(
                    "addresses length ({}) must be a positive multiple of addressLen ({})",
                    bytes.len(),
                    addr_len
                ),
            );
            return ptr::null_mut();
        }
        let count = bytes.len() / addr_len;
        // Build the pointer + length arrays the FFI expects.
        let ptrs: Vec<*const u8> = (0..count)
            .map(|i| unsafe { bytes.as_ptr().add(i * addr_len) })
            .collect();
        let lengths: Vec<usize> = std::iter::repeat_n(addr_len, count).collect();
        // SAFETY: `bytes`, `ptrs`, and `lengths` all outlive the call.
        let result = unsafe {
            dash_sdk_addresses_fetch_infos(
                sdk as *const SDKHandle,
                ptrs.as_ptr(),
                lengths.as_ptr(),
                count,
            )
        };
        unsafe { unwrap_address_info_map(env, result) }
            .map(|s| s.into_raw())
            .unwrap_or(ptr::null_mut())
    })
}

// ── Contested resources / voting (read-only) ─────────────────────────────

/// Contested resources for a contract/document-type/index. `resultType` is
/// the rs-sdk-ffi u8 enum discriminant; the index-value JSON bounds and
/// `orderAscending` mirror the iOS query catalog. Returns JSON, or null.
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_QueriesNative_contestedResourcesGet(
    mut env: JNIEnv,
    _class: JClass,
    sdk: jlong,
    contract_id: JString,
    document_type_name: JString,
    index_name: JString,
    start_index_values_json: JString,
    end_index_values_json: JString,
    count: jint,
    order_ascending: jboolean,
) -> jstring {
    guard(&mut env, ptr::null_mut(), |env| {
        let contract = require_cstr!(env, contract_id);
        let doc_type = require_cstr!(env, document_type_name);
        let index = require_cstr!(env, index_name);
        let start = opt_c_string(env, &start_index_values_json);
        let end = opt_c_string(env, &end_index_values_json);
        let result = unsafe {
            dash_sdk_contested_resource_get_resources(
                sdk as *const SDKHandle,
                contract.as_ptr(),
                doc_type.as_ptr(),
                index.as_ptr(),
                c_ptr(&start),
                c_ptr(&end),
                count.max(0) as u32,
                order_ascending != 0,
            )
        };
        unsafe { unwrap_string(env, result) }
            .map(|s| s.into_raw())
            .unwrap_or(ptr::null_mut())
    })
}

/// Vote state (contenders + tallies) for a contested resource. `indexValues`
/// is a JSON array of the index path values; `resultType` is the u8 enum
/// discriminant. Returns JSON, or null.
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_QueriesNative_contestedResourceVoteState(
    mut env: JNIEnv,
    _class: JClass,
    sdk: jlong,
    contract_id: JString,
    document_type_name: JString,
    index_name: JString,
    index_values_json: JString,
    result_type: jint,
    allow_include_locked_and_abstaining: jboolean,
    count: jint,
) -> jstring {
    guard(&mut env, ptr::null_mut(), |env| {
        let contract = require_cstr!(env, contract_id);
        let doc_type = require_cstr!(env, document_type_name);
        let index = require_cstr!(env, index_name);
        let values = require_cstr!(env, index_values_json);
        let result = unsafe {
            dash_sdk_contested_resource_get_vote_state(
                sdk as *const SDKHandle,
                contract.as_ptr(),
                doc_type.as_ptr(),
                index.as_ptr(),
                values.as_ptr(),
                result_type.clamp(0, u8::MAX as jint) as u8,
                allow_include_locked_and_abstaining != 0,
                count.max(0) as u32,
            )
        };
        unsafe { unwrap_string(env, result) }
            .map(|s| s.into_raw())
            .unwrap_or(ptr::null_mut())
    })
}

/// Voters who voted for a specific contestant on a contested resource.
/// Returns JSON, or null.
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_QueriesNative_contestedResourceVotersForIdentity(
    mut env: JNIEnv,
    _class: JClass,
    sdk: jlong,
    contract_id: JString,
    document_type_name: JString,
    index_name: JString,
    index_values_json: JString,
    contestant_id: JString,
    count: jint,
    order_ascending: jboolean,
) -> jstring {
    guard(&mut env, ptr::null_mut(), |env| {
        let contract = require_cstr!(env, contract_id);
        let doc_type = require_cstr!(env, document_type_name);
        let index = require_cstr!(env, index_name);
        let values = require_cstr!(env, index_values_json);
        let contestant = require_cstr!(env, contestant_id);
        let result = unsafe {
            dash_sdk_contested_resource_get_voters_for_identity(
                sdk as *const SDKHandle,
                contract.as_ptr(),
                doc_type.as_ptr(),
                index.as_ptr(),
                values.as_ptr(),
                contestant.as_ptr(),
                count.max(0) as u32,
                order_ascending != 0,
            )
        };
        unsafe { unwrap_string(env, result) }
            .map(|s| s.into_raw())
            .unwrap_or(ptr::null_mut())
    })
}

/// All contested-resource votes cast by an identity. Returns JSON, or null.
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_QueriesNative_contestedResourceIdentityVotes(
    mut env: JNIEnv,
    _class: JClass,
    sdk: jlong,
    identity_id: JString,
    limit: jint,
    offset: jint,
    order_ascending: jboolean,
) -> jstring {
    guard(&mut env, ptr::null_mut(), |env| {
        let id = require_cstr!(env, identity_id);
        let result = unsafe {
            dash_sdk_contested_resource_get_identity_votes(
                sdk as *const SDKHandle,
                id.as_ptr(),
                limit.max(0) as u32,
                offset.max(0) as u32,
                order_ascending != 0,
            )
        };
        unsafe { unwrap_string(env, result) }
            .map(|s| s.into_raw())
            .unwrap_or(ptr::null_mut())
    })
}

/// Vote polls whose end date falls in `[startTimeMs, endTimeMs]`. Bounds are
/// inclusive per the `*Included` flags. Returns JSON, or null.
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_QueriesNative_votingVotePollsByEndDate(
    mut env: JNIEnv,
    _class: JClass,
    sdk: jlong,
    start_time_ms: jlong,
    start_time_included: jboolean,
    end_time_ms: jlong,
    end_time_included: jboolean,
    limit: jint,
    offset: jint,
    ascending: jboolean,
) -> jstring {
    guard(&mut env, ptr::null_mut(), |env| {
        let result = unsafe {
            dash_sdk_voting_get_vote_polls_by_end_date(
                sdk as *const SDKHandle,
                start_time_ms.max(0) as u64,
                start_time_included != 0,
                end_time_ms.max(0) as u64,
                end_time_included != 0,
                limit.max(0) as u32,
                offset.max(0) as u32,
                ascending != 0,
            )
        };
        unsafe { unwrap_string(env, result) }
            .map(|s| s.into_raw())
            .unwrap_or(ptr::null_mut())
    })
}

// ── Evonode ──────────────────────────────────────────────────────────────

/// Evonode proposed-block counts for an epoch, by pro-tx-hash list.
/// `idsJson` is a JSON array of pro-tx-hash strings. Returns JSON, or null.
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_QueriesNative_evonodeProposedEpochBlocksByIds(
    mut env: JNIEnv,
    _class: JClass,
    sdk: jlong,
    epoch: jint,
    ids_json: JString,
) -> jstring {
    guard(&mut env, ptr::null_mut(), |env| {
        let ids = require_cstr!(env, ids_json);
        let result = unsafe {
            dash_sdk_evonode_get_proposed_epoch_blocks_by_ids(
                sdk as *const SDKHandle,
                epoch.max(0) as u32,
                ids.as_ptr(),
            )
        };
        unsafe { unwrap_string(env, result) }
            .map(|s| s.into_raw())
            .unwrap_or(ptr::null_mut())
    })
}

/// Evonode proposed-block counts for an epoch, paginated by range.
/// `startAfter`/`startAt` (pro-tx-hash) may be null. Returns JSON, or null.
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_QueriesNative_evonodeProposedEpochBlocksByRange(
    mut env: JNIEnv,
    _class: JClass,
    sdk: jlong,
    epoch: jint,
    limit: jint,
    start_after: JString,
    start_at: JString,
) -> jstring {
    guard(&mut env, ptr::null_mut(), |env| {
        let after = opt_c_string(env, &start_after);
        let at = opt_c_string(env, &start_at);
        let result = unsafe {
            dash_sdk_evonode_get_proposed_epoch_blocks_by_range(
                sdk as *const SDKHandle,
                epoch.max(0) as u32,
                limit.max(0) as u32,
                c_ptr(&after),
                c_ptr(&at),
            )
        };
        unsafe { unwrap_string(env, result) }
            .map(|s| s.into_raw())
            .unwrap_or(ptr::null_mut())
    })
}

// ── Protocol version ─────────────────────────────────────────────────────

/// Protocol-version upgrade state (per-version vote counts) as JSON, or null.
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_QueriesNative_protocolVersionUpgradeState(
    mut env: JNIEnv,
    _class: JClass,
    sdk: jlong,
) -> jstring {
    guard(&mut env, ptr::null_mut(), |env| {
        let result =
            unsafe { dash_sdk_protocol_version_get_upgrade_state(sdk as *const SDKHandle) };
        unsafe { unwrap_string(env, result) }
            .map(|s| s.into_raw())
            .unwrap_or(ptr::null_mut())
    })
}

/// Protocol-version upgrade vote status by evonode. `startProTxHash` may be
/// null. Returns JSON, or null.
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_QueriesNative_protocolVersionUpgradeVoteStatus(
    mut env: JNIEnv,
    _class: JClass,
    sdk: jlong,
    start_pro_tx_hash: JString,
    count: jint,
) -> jstring {
    guard(&mut env, ptr::null_mut(), |env| {
        let start = opt_c_string(env, &start_pro_tx_hash);
        let result = unsafe {
            dash_sdk_protocol_version_get_upgrade_vote_status(
                sdk as *const SDKHandle,
                c_ptr(&start),
                count.max(0) as u32,
            )
        };
        unsafe { unwrap_string(env, result) }
            .map(|s| s.into_raw())
            .unwrap_or(ptr::null_mut())
    })
}

// ── System ───────────────────────────────────────────────────────────────

/// GroveDB path-elements query. `pathJson` is a JSON array of the path
/// segments; `keysJson` (JSON array of keys) may be null. Returns a JSON
/// array of elements, or null after throwing. Backs `GroveDBPathElementsView`.
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_QueriesNative_systemGetPathElements(
    mut env: JNIEnv,
    _class: JClass,
    sdk: jlong,
    path_json: JString,
    keys_json: JString,
) -> jstring {
    guard(&mut env, ptr::null_mut(), |env| {
        let path = require_cstr!(env, path_json);
        let keys = opt_c_string(env, &keys_json);
        let result = unsafe {
            dash_sdk_system_get_path_elements(sdk as *const SDKHandle, path.as_ptr(), c_ptr(&keys))
        };
        unsafe { unwrap_string(env, result) }
            .map(|s| s.into_raw())
            .unwrap_or(ptr::null_mut())
    })
}

/// Prefunded specialized balance for a base58 id, as JSON. Null if absent.
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_QueriesNative_systemGetPrefundedSpecializedBalance(
    mut env: JNIEnv,
    _class: JClass,
    sdk: jlong,
    id: JString,
) -> jstring {
    guard(&mut env, ptr::null_mut(), |env| {
        let id = require_cstr!(env, id);
        let result = unsafe {
            dash_sdk_system_get_prefunded_specialized_balance(sdk as *const SDKHandle, id.as_ptr())
        };
        unsafe { unwrap_string(env, result) }
            .map(|s| s.into_raw())
            .unwrap_or(ptr::null_mut())
    })
}

/// Total credits currently in Platform, as a decimal string. Null on error.
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_QueriesNative_systemGetTotalCreditsInPlatform(
    mut env: JNIEnv,
    _class: JClass,
    sdk: jlong,
) -> jstring {
    guard(&mut env, ptr::null_mut(), |env| {
        let result =
            unsafe { dash_sdk_system_get_total_credits_in_platform(sdk as *const SDKHandle) };
        unsafe { unwrap_string(env, result) }
            .map(|s| s.into_raw())
            .unwrap_or(ptr::null_mut())
    })
}

/// Current quorums info as a JSON array, or null on error.
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_QueriesNative_systemGetCurrentQuorumsInfo(
    mut env: JNIEnv,
    _class: JClass,
    sdk: jlong,
) -> jstring {
    guard(&mut env, ptr::null_mut(), |env| {
        let result = unsafe { dash_sdk_system_get_current_quorums_info(sdk as *const SDKHandle) };
        unsafe { unwrap_string(env, result) }
            .map(|s| s.into_raw())
            .unwrap_or(ptr::null_mut())
    })
}

/// Epochs info as a JSON array. `startEpoch` (decimal string) may be null to
/// start from the current epoch. Returns JSON, or null.
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_QueriesNative_systemGetEpochsInfo(
    mut env: JNIEnv,
    _class: JClass,
    sdk: jlong,
    start_epoch: JString,
    count: jint,
    ascending: jboolean,
) -> jstring {
    guard(&mut env, ptr::null_mut(), |env| {
        let start = opt_c_string(env, &start_epoch);
        let result = unsafe {
            dash_sdk_system_get_epochs_info(
                sdk as *const SDKHandle,
                c_ptr(&start),
                count.max(0) as u32,
                ascending != 0,
            )
        };
        unsafe { unwrap_string(env, result) }
            .map(|s| s.into_raw())
            .unwrap_or(ptr::null_mut())
    })
}

// ── Group ────────────────────────────────────────────────────────────────

/// Group info (required power + members) at a contract position, as JSON.
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_QueriesNative_groupGetInfo(
    mut env: JNIEnv,
    _class: JClass,
    sdk: jlong,
    contract_id: JString,
    group_contract_position: jint,
) -> jstring {
    guard(&mut env, ptr::null_mut(), |env| {
        let contract = require_cstr!(env, contract_id);
        let result = unsafe {
            dash_sdk_group_get_info(
                sdk as *const SDKHandle,
                contract.as_ptr(),
                group_contract_position.clamp(0, u16::MAX as jint) as u16,
            )
        };
        unsafe { unwrap_string(env, result) }
            .map(|s| s.into_raw())
            .unwrap_or(ptr::null_mut())
    })
}

/// All groups in a contract, as a JSON array. `startAtPosition` (decimal
/// string) may be null. Returns JSON, or null.
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_QueriesNative_groupGetInfos(
    mut env: JNIEnv,
    _class: JClass,
    sdk: jlong,
    start_at_position: JString,
    limit: jint,
) -> jstring {
    guard(&mut env, ptr::null_mut(), |env| {
        let start = opt_c_string(env, &start_at_position);
        let result = unsafe {
            dash_sdk_group_get_infos(sdk as *const SDKHandle, c_ptr(&start), limit.max(0) as u32)
        };
        unsafe { unwrap_string(env, result) }
            .map(|s| s.into_raw())
            .unwrap_or(ptr::null_mut())
    })
}

/// Group actions at a contract position, filtered by `status` (u8 enum
/// discriminant). `startAtActionId` may be null. Returns JSON, or null.
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_QueriesNative_groupGetActions(
    mut env: JNIEnv,
    _class: JClass,
    sdk: jlong,
    contract_id: JString,
    group_contract_position: jint,
    status: jint,
    start_at_action_id: JString,
    limit: jint,
) -> jstring {
    guard(&mut env, ptr::null_mut(), |env| {
        let contract = require_cstr!(env, contract_id);
        let start = opt_c_string(env, &start_at_action_id);
        let result = unsafe {
            dash_sdk_group_get_actions(
                sdk as *const SDKHandle,
                contract.as_ptr(),
                group_contract_position.clamp(0, u16::MAX as jint) as u16,
                status.clamp(0, u8::MAX as jint) as u8,
                c_ptr(&start),
                limit.clamp(0, u16::MAX as jint) as u16,
            )
        };
        unsafe { unwrap_string(env, result) }
            .map(|s| s.into_raw())
            .unwrap_or(ptr::null_mut())
    })
}

/// Signers of a specific group action, as JSON. `status` is the u8 enum
/// discriminant. Returns JSON, or null.
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_QueriesNative_groupGetActionSigners(
    mut env: JNIEnv,
    _class: JClass,
    sdk: jlong,
    contract_id: JString,
    group_contract_position: jint,
    status: jint,
    action_id: JString,
) -> jstring {
    guard(&mut env, ptr::null_mut(), |env| {
        let contract = require_cstr!(env, contract_id);
        let action = require_cstr!(env, action_id);
        let result = unsafe {
            dash_sdk_group_get_action_signers(
                sdk as *const SDKHandle,
                contract.as_ptr(),
                group_contract_position.clamp(0, u16::MAX as jint) as u16,
                status.clamp(0, u8::MAX as jint) as u8,
                action.as_ptr(),
            )
        };
        unsafe { unwrap_string(env, result) }
            .map(|s| s.into_raw())
            .unwrap_or(ptr::null_mut())
    })
}

// ── Tokens (additional read queries) ─────────────────────────────────────

/// A token's total supply as a decimal string, or null.
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_QueriesNative_tokenGetTotalSupply(
    mut env: JNIEnv,
    _class: JClass,
    sdk: jlong,
    token_id: JString,
) -> jstring {
    guard(&mut env, ptr::null_mut(), |env| {
        let id = require_cstr!(env, token_id);
        let result =
            unsafe { dash_sdk_token_get_total_supply(sdk as *const SDKHandle, id.as_ptr()) };
        unsafe { unwrap_string(env, result) }
            .map(|s| s.into_raw())
            .unwrap_or(ptr::null_mut())
    })
}

/// An identity's last perpetual-distribution claim for a token, as JSON,
/// or null.
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_QueriesNative_tokenGetPerpetualDistributionLastClaim(
    mut env: JNIEnv,
    _class: JClass,
    sdk: jlong,
    token_id: JString,
    identity_id: JString,
) -> jstring {
    guard(&mut env, ptr::null_mut(), |env| {
        let token = require_cstr!(env, token_id);
        let identity = require_cstr!(env, identity_id);
        let result = unsafe {
            dash_sdk_token_get_perpetual_distribution_last_claim(
                sdk as *const SDKHandle,
                token.as_ptr(),
                identity.as_ptr(),
            )
        };
        unsafe { unwrap_string(env, result) }
            .map(|s| s.into_raw())
            .unwrap_or(ptr::null_mut())
    })
}

/// Direct-purchase prices for a comma-separated base58 token-id list, as
/// JSON, or null.
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_QueriesNative_tokenGetDirectPurchasePrices(
    mut env: JNIEnv,
    _class: JClass,
    sdk: jlong,
    token_ids: JString,
) -> jstring {
    guard(&mut env, ptr::null_mut(), |env| {
        let ids = require_cstr!(env, token_ids);
        let result = unsafe {
            dash_sdk_token_get_direct_purchase_prices(sdk as *const SDKHandle, ids.as_ptr())
        };
        unsafe { unwrap_string(env, result) }
            .map(|s| s.into_raw())
            .unwrap_or(ptr::null_mut())
    })
}

/// Pre-programmed distributions for a token, paginated. `startRecipient`
/// (base58) may be null. Returns JSON, or null.
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_QueriesNative_tokenGetPreProgrammedDistributions(
    mut env: JNIEnv,
    _class: JClass,
    sdk: jlong,
    token_id: JString,
    start_time_ms: jlong,
    start_recipient: JString,
    start_recipient_included: jboolean,
    limit: jint,
) -> jstring {
    guard(&mut env, ptr::null_mut(), |env| {
        let token = require_cstr!(env, token_id);
        let recipient = opt_c_string(env, &start_recipient);
        let result = unsafe {
            dash_sdk_token_get_pre_programmed_distributions(
                sdk as *const SDKHandle,
                token.as_ptr(),
                start_time_ms.max(0) as u64,
                c_ptr(&recipient),
                start_recipient_included != 0,
                limit.max(0) as u32,
            )
        };
        unsafe { unwrap_string(env, result) }
            .map(|s| s.into_raw())
            .unwrap_or(ptr::null_mut())
    })
}

// ── Identity nonces & multi-identity contract keys (Wave-2C) ─────────────

/// An identity's current nonce as a decimal string, or null.
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_QueriesNative_identityFetchNonce(
    mut env: JNIEnv,
    _class: JClass,
    sdk: jlong,
    identity_id: JString,
) -> jstring {
    guard(&mut env, ptr::null_mut(), |env| {
        let id = require_cstr!(env, identity_id);
        let result = unsafe { dash_sdk_identity_fetch_nonce(sdk as *const SDKHandle, id.as_ptr()) };
        unsafe { unwrap_string(env, result) }
            .map(|s| s.into_raw())
            .unwrap_or(ptr::null_mut())
    })
}

/// An identity's nonce for a specific contract as a decimal string, or null.
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_QueriesNative_identityFetchContractNonce(
    mut env: JNIEnv,
    _class: JClass,
    sdk: jlong,
    identity_id: JString,
    contract_id: JString,
) -> jstring {
    guard(&mut env, ptr::null_mut(), |env| {
        let identity = require_cstr!(env, identity_id);
        let contract = require_cstr!(env, contract_id);
        let result = unsafe {
            dash_sdk_identity_fetch_contract_nonce(
                sdk as *const SDKHandle,
                identity.as_ptr(),
                contract.as_ptr(),
            )
        };
        unsafe { unwrap_string(env, result) }
            .map(|s| s.into_raw())
            .unwrap_or(ptr::null_mut())
    })
}

/// Contract keys for many identities as a JSON object keyed by base58 id, or
/// null. `identity_ids` and `purposes` are comma-separated lists;
/// `document_type_name` may be null.
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_QueriesNative_identitiesFetchContractKeys(
    mut env: JNIEnv,
    _class: JClass,
    sdk: jlong,
    identity_ids: JString,
    contract_id: JString,
    document_type_name: JString,
    purposes: JString,
) -> jstring {
    guard(&mut env, ptr::null_mut(), |env| {
        let ids = require_cstr!(env, identity_ids);
        let contract = require_cstr!(env, contract_id);
        let purposes = require_cstr!(env, purposes);
        let document_type = opt_c_string(env, &document_type_name);
        let result = unsafe {
            dash_sdk_identities_fetch_contract_keys(
                sdk as *const SDKHandle,
                ids.as_ptr(),
                contract.as_ptr(),
                c_ptr(&document_type),
                purposes.as_ptr(),
            )
        };
        unsafe { unwrap_string(env, result) }
            .map(|s| s.into_raw())
            .unwrap_or(ptr::null_mut())
    })
}

/// Fetch a data contract by base58 id, returning both its JSON form and its
/// canonical serialized bytes in a single round-trip. The result is a
/// two-element `Object[]` — `[String json, byte[] serialization]` — so the
/// Kotlin `Contracts.fetchWithSerialization` wrapper can hydrate its
/// `ContractWithSerialization` without coupling this symbol to an app class
/// name. Either element may be null; returns null (no array) after throwing.
///
/// The FFI returns [`DashSDKDataContractFetchResult`] by value (never a
/// `DashSDKResult`), so its inner buffers are reclaimed here via
/// `dash_sdk_data_contract_fetch_result_free` before returning. The contract
/// handle carried by the result is freed too — this query surfaces the
/// contract as data, not as a document-query handle (callers wanting a handle
/// use `dataContractFetch`).
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_QueriesNative_dataContractFetchWithSerialization(
    mut env: JNIEnv,
    _class: JClass,
    sdk: jlong,
    contract_id: JString,
) -> jobject {
    guard(&mut env, ptr::null_mut(), |env| {
        let id = require_cstr!(env, contract_id);
        let mut result = unsafe {
            dash_sdk_data_contract_fetch_with_serialization(
                sdk as *const SDKHandle,
                id.as_ptr(),
                true,
                true,
            )
        };

        // Error path: throw, free inner buffers, bail with null.
        if !result.error.is_null() {
            // SAFETY: non-null error is a valid CString-backed DashSDKError.
            let err = unsafe { &*result.error };
            let message = if err.message.is_null() {
                String::from("Unknown SDK error")
            } else {
                unsafe { CStr::from_ptr(err.message) }
                    .to_string_lossy()
                    .into_owned()
            };
            throw_sdk_exception(env, err.code as i32, &message);
            unsafe { dash_sdk_data_contract_fetch_result_free(&mut result) };
            return ptr::null_mut();
        }

        // Copy the JSON string and serialized bytes out of the FFI-owned
        // buffers into JVM-owned values before freeing them.
        let json_obj: JObject = if result.json_string.is_null() {
            JObject::null()
        } else {
            let json = unsafe { CStr::from_ptr(result.json_string) }
                .to_string_lossy()
                .into_owned();
            match env.new_string(json) {
                Ok(s) => s.into(),
                Err(_) => {
                    unsafe { dash_sdk_data_contract_fetch_result_free(&mut result) };
                    return ptr::null_mut();
                }
            }
        };

        let bytes_obj: JObject =
            if result.serialized_data.is_null() || result.serialized_data_len == 0 {
                JObject::null()
            } else {
                let slice = unsafe {
                    std::slice::from_raw_parts(result.serialized_data, result.serialized_data_len)
                };
                match env.byte_array_from_slice(slice) {
                    Ok(arr) => arr.into(),
                    Err(_) => {
                        unsafe { dash_sdk_data_contract_fetch_result_free(&mut result) };
                        return ptr::null_mut();
                    }
                }
            };

        // Every inner buffer/handle is now copied out; reclaim FFI memory.
        unsafe { dash_sdk_data_contract_fetch_result_free(&mut result) };

        // Assemble the [json, serialization] Object[] payload.
        let object_class = match env.find_class("java/lang/Object") {
            Ok(c) => c,
            Err(_) => return ptr::null_mut(),
        };
        let array = match env.new_object_array(2, &object_class, JObject::null()) {
            Ok(a) => a,
            Err(_) => return ptr::null_mut(),
        };
        if env.set_object_array_element(&array, 0, &json_obj).is_err()
            || env.set_object_array_element(&array, 1, &bytes_obj).is_err()
        {
            return ptr::null_mut();
        }
        array.into_raw()
    })
}

/// Pre-load locally-stored data contracts into the SDK's trusted context
/// provider so proof verification resolves them without a network fetch —
/// the JNI bridge over `dash_sdk_add_known_contracts` (Swift
/// `SDK.loadKnownContracts`, called at SDK init from
/// `AppState.loadKnownContractsIntoSDK`).
///
/// * `contract_ids` — a comma-separated list of base58 contract ids.
/// * `serialized_contracts` — a `byte[][]` of each contract's **versioned
///   binary serialization** (the bytes `DataContract::versioned_deserialize`
///   expects — i.e. `DataContractEntity.binarySerialization`, NOT the JSON),
///   in the SAME order as `contract_ids`.
///
/// Trusted-provider config keyed off the SDK handle (like
/// `dataContractFetchWithSerialization`), not a per-query fetch. The FFI
/// splits the ids CSV itself and rejects an id/contract count mismatch, so
/// this must pass exactly one `byte[]` per id. Returns nothing on success
/// (throws on error): a missing trusted provider, a malformed contract, or a
/// count mismatch all surface as a thrown `DashSDKException`.
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_QueriesNative_addKnownContracts(
    mut env: JNIEnv,
    _class: JClass,
    sdk: jlong,
    contract_ids: JString,
    serialized_contracts: JObjectArray,
) {
    guard(&mut env, (), |env| {
        let ids = match opt_c_string(env, &contract_ids) {
            Some(s) => s,
            None => {
                throw_sdk_exception(env, 1, "contractIds was null");
                return;
            }
        };

        if serialized_contracts.is_null() {
            throw_sdk_exception(env, 1, "serializedContracts was null");
            return;
        }

        let count = match env.get_array_length(&serialized_contracts) {
            Ok(len) if len >= 0 => len as usize,
            _ => {
                let _ = env.exception_clear();
                throw_sdk_exception(env, 1, "serializedContracts length was invalid");
                return;
            }
        };

        // Empty input is a legitimate no-op (nothing stored to preload).
        if count == 0 {
            return;
        }

        // Copy every contract's bytes into owned buffers that outlive the FFI
        // call (the FFI borrows each pointer for the call duration).
        let mut buffers: Vec<Vec<u8>> = Vec::with_capacity(count);
        for i in 0..count {
            let bytes = match env.get_object_array_element(&serialized_contracts, i as i32) {
                Ok(obj) => {
                    let arr: JByteArray = obj.into();
                    if arr.is_null() {
                        throw_sdk_exception(env, 1, &format!("serializedContracts[{i}] was null"));
                        return;
                    }
                    match env.convert_byte_array(&arr) {
                        Ok(b) => b,
                        Err(_) => {
                            let _ = env.exception_clear();
                            throw_sdk_exception(
                                env,
                                1,
                                &format!("serializedContracts[{i}] byte[] was invalid"),
                            );
                            return;
                        }
                    }
                }
                Err(_) => {
                    let _ = env.exception_clear();
                    throw_sdk_exception(
                        env,
                        1,
                        &format!("serializedContracts[{i}] could not be read"),
                    );
                    return;
                }
            };
            buffers.push(bytes);
        }

        // Build the pointer + length arrays the FFI expects. Both `buffers`
        // and these two Vecs outlive the synchronous FFI call.
        let ptrs: Vec<*const u8> = buffers.iter().map(|b| b.as_ptr()).collect();
        let lengths: Vec<usize> = buffers.iter().map(|b| b.len()).collect();

        let result = unsafe {
            dash_sdk_add_known_contracts(
                sdk as *const SDKHandle,
                ids.as_ptr(),
                ptrs.as_ptr(),
                lengths.as_ptr(),
                count,
            )
        };
        // Success carries no payload; take_error throws + frees on error.
        unsafe { crate::results::take_error(env, &result) };
    })
}
