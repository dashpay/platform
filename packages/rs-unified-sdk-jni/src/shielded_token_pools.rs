//! JNI bridge for a wallet's tokens in token shielded pools: watching a
//! token's pool, listing what the local scan found there, and spending those
//! notes with the Type 26 `TokenShieldedTransferWithShieldedFee` and Type 27
//! `TokenUnshieldWithShieldedFee` transitions, with the local fee preview
//! both use.
//!
//! Kotlin counterpart: `org.dashfoundation.dashsdk.ffi.ShieldedTokenPoolNative`,
//! driven by the `PlatformWalletManager.shieldedTokenPool*` and
//! `shieldedToken{Transfer,Unshield,SpendPreview}` wrappers. Swift
//! counterpart: the same names in `PlatformWalletManagerShieldedSync.swift`.
//!
//! Each export is one call into `platform_wallet_ffi`
//! (`shielded_token_pools` and `shielded_send`). Nothing is decided here: the
//! wallet checks the amount, the token, the network and the note counts,
//! selects both note sets, builds, broadcasts, records the activity (kind
//! tags 11 and 12) and requests the shielded sync that confirms a spend.
//!
//! The whole module compiles only under the `shielded` cargo feature.

#![allow(clippy::missing_safety_doc)]
#![cfg(feature = "shielded")]

use std::ffi::CStr;
use std::ptr;

use crate::shielded_token_purchase::{read_id32, read_token_position};
use crate::support::{guard, take_pwffi_error, throw_sdk_exception};
use jni::objects::{JByteArray, JClass, JObject, JValue};
use jni::sys::{jboolean, jbyte, jbyteArray, jint, jlong, jlongArray, jobjectArray};
use jni::JNIEnv;
use platform_wallet_ffi::handle::Handle;
use platform_wallet_ffi::shielded_send::{
    platform_wallet_manager_shielded_token_spend_preview,
    platform_wallet_manager_shielded_token_transfer,
    platform_wallet_manager_shielded_token_unshield,
};
use platform_wallet_ffi::shielded_token_pools::{
    platform_wallet_manager_shielded_token_pool_balances,
    platform_wallet_manager_shielded_token_pool_balances_free,
    platform_wallet_manager_shielded_token_pool_unwatch,
    platform_wallet_manager_shielded_token_pool_watch, ShieldedTokenPoolBalanceFFI,
};
use rs_sdk_ffi::MnemonicResolverHandle;

/// The Kotlin transport class one balance row becomes.
const BALANCE_ROW_CLASS: &str = "org/dashfoundation/dashsdk/ffi/ShieldedTokenPoolBalanceData";

/// Its constructor: token id, contract id, position, account, balance, note
/// count, reserved balance, then each optional value as a flag and the value,
/// then the last scan error (nullable).
const BALANCE_ROW_CTOR: &str = "([B[BIIJIJZJZJZJLjava/lang/String;)V";

/// One balance row copied out of the Rust allocation, so the array can be
/// freed before any JVM object is created (a JNI failure then leaks nothing).
#[derive(Debug, PartialEq, Eq)]
struct OwnedPoolBalance {
    token_id: [u8; 32],
    data_contract_id: [u8; 32],
    token_contract_position: u16,
    account_index: u32,
    balance: u64,
    unspent_note_count: u32,
    reserved_balance: u64,
    has_scanned_note_count: bool,
    scanned_note_count: u64,
    has_pool_note_count: bool,
    pool_note_count: u64,
    has_scanned_height: bool,
    scanned_height: u64,
    last_error: Option<String>,
}

impl OwnedPoolBalance {
    /// Copy every field of `row`, including its error string.
    ///
    /// # Safety
    /// `row.last_error` must be null or a live NUL-terminated string, as the
    /// balances export returns it.
    unsafe fn copy_from(row: &ShieldedTokenPoolBalanceFFI) -> Self {
        let last_error = if row.last_error.is_null() {
            None
        } else {
            Some(
                unsafe { CStr::from_ptr(row.last_error) }
                    .to_string_lossy()
                    .into_owned(),
            )
        };
        Self {
            token_id: row.token_id,
            data_contract_id: row.data_contract_id,
            token_contract_position: row.token_contract_position,
            account_index: row.account_index,
            balance: row.balance,
            unspent_note_count: row.unspent_note_count,
            reserved_balance: row.reserved_balance,
            has_scanned_note_count: row.has_scanned_note_count,
            scanned_note_count: row.scanned_note_count,
            has_pool_note_count: row.has_pool_note_count,
            pool_note_count: row.pool_note_count,
            has_scanned_height: row.has_scanned_height,
            scanned_height: row.scanned_height,
            last_error,
        }
    }
}

/// Copy the `count` rows at `rows` and free the array. An empty list for a
/// null array.
///
/// # Safety
/// `rows` must be null or an array the balances export returned with
/// exactly `count` elements, not yet freed.
unsafe fn take_pool_balances(
    rows: *mut ShieldedTokenPoolBalanceFFI,
    count: usize,
) -> Vec<OwnedPoolBalance> {
    if rows.is_null() || count == 0 {
        unsafe { platform_wallet_manager_shielded_token_pool_balances_free(rows, count) };
        return Vec::new();
    }
    let owned = unsafe { std::slice::from_raw_parts(rows, count) }
        .iter()
        .map(|row| unsafe { OwnedPoolBalance::copy_from(row) })
        .collect();
    unsafe { platform_wallet_manager_shielded_token_pool_balances_free(rows, count) };
    owned
}

/// Read a raw 43-byte Orchard address; throws and returns `None` on JVM
/// null, a JNI error or the wrong length. Whether the bytes are an Orchard
/// address is the FFI's check.
fn read_raw_43(env: &mut JNIEnv, arr: &JByteArray, field: &str) -> Option<[u8; 43]> {
    if arr.is_null() {
        throw_sdk_exception(env, 1, &format!("{field} byte[] was null"));
        return None;
    }
    let bytes = match env.convert_byte_array(arr) {
        Ok(bytes) => bytes,
        Err(_) => {
            let _ = env.exception_clear();
            throw_sdk_exception(env, 1, &format!("{field} byte[] was invalid"));
            return None;
        }
    };
    match <[u8; 43]>::try_from(bytes.as_slice()) {
        Ok(raw) => Some(raw),
        Err(_) => {
            throw_sdk_exception(
                env,
                1,
                &format!("{field} must be 43 bytes, got {}", bytes.len()),
            );
            None
        }
    }
}

/// An Orchard account index as the `u32` the FFI takes; throws and returns
/// `None` for a negative one.
fn read_account(env: &mut JNIEnv, account: jint) -> Option<u32> {
    match u32::try_from(account) {
        Ok(account) => Some(account),
        Err(_) => {
            throw_sdk_exception(env, 1, "account must be non-negative");
            None
        }
    }
}

/// Watch the shielded pool of the token at `tokenPosition` of
/// `dataContractId` for `walletId`: bridges
/// `platform_wallet_manager_shielded_token_pool_watch`. Returns the token's
/// 32-byte id. Rust reads the token's configuration with proofs and refuses a
/// token without a shielded pool (thrown with Rust's reason). Watching a
/// watched pool again changes nothing.
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_ShieldedTokenPoolNative_watch(
    mut env: JNIEnv,
    _class: JClass,
    manager_handle: jlong,
    wallet_id: JByteArray,
    data_contract_id: JByteArray,
    token_position: jint,
) -> jbyteArray {
    guard(&mut env, ptr::null_mut(), |env| {
        let Some(position) = read_token_position(env, token_position) else {
            return ptr::null_mut();
        };
        let Some(wid) = read_id32(env, &wallet_id, "walletId") else {
            return ptr::null_mut();
        };
        let Some(contract) = read_id32(env, &data_contract_id, "dataContractId") else {
            return ptr::null_mut();
        };
        let mut token_id = [0u8; 32];
        let result = unsafe {
            platform_wallet_manager_shielded_token_pool_watch(
                manager_handle as Handle,
                wid.as_ptr(),
                contract.as_ptr(),
                position,
                token_id.as_mut_ptr(),
            )
        };
        if take_pwffi_error(env, result) {
            return ptr::null_mut();
        }
        env.byte_array_from_slice(&token_id)
            .map(|array| array.into_raw())
            .unwrap_or(ptr::null_mut())
    })
}

/// Stop watching `tokenId`'s shielded pool for `walletId`, forgetting the
/// wallet's notes and scan state there: bridges
/// `platform_wallet_manager_shielded_token_pool_unwatch`. Unwatching a pool
/// not watched changes nothing.
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_ShieldedTokenPoolNative_unwatch(
    mut env: JNIEnv,
    _class: JClass,
    manager_handle: jlong,
    wallet_id: JByteArray,
    token_id: JByteArray,
) {
    guard(&mut env, (), |env| {
        let Some(wid) = read_id32(env, &wallet_id, "walletId") else {
            return;
        };
        let Some(token) = read_id32(env, &token_id, "tokenId") else {
            return;
        };
        let result = unsafe {
            platform_wallet_manager_shielded_token_pool_unwatch(
                manager_handle as Handle,
                wid.as_ptr(),
                token.as_ptr(),
            )
        };
        let _ = take_pwffi_error(env, result);
    })
}

/// `walletId`'s tokens in every shielded pool it watches, one row per
/// watched pool and bound account, from the local scan alone: bridges
/// `platform_wallet_manager_shielded_token_pool_balances` and frees its array
/// with `platform_wallet_manager_shielded_token_pool_balances_free` before any
/// JVM object is made. Returns `ShieldedTokenPoolBalanceData[]` in Rust's
/// order; unsigned values keep their bit patterns.
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_ShieldedTokenPoolNative_balances(
    mut env: JNIEnv,
    _class: JClass,
    manager_handle: jlong,
    wallet_id: JByteArray,
) -> jobjectArray {
    guard(&mut env, ptr::null_mut(), |env| {
        let Some(wid) = read_id32(env, &wallet_id, "walletId") else {
            return ptr::null_mut();
        };
        let mut rows: *mut ShieldedTokenPoolBalanceFFI = ptr::null_mut();
        let mut count = 0usize;
        let result = unsafe {
            platform_wallet_manager_shielded_token_pool_balances(
                manager_handle as Handle,
                wid.as_ptr(),
                &mut rows,
                &mut count,
            )
        };
        // Rust resets both slots before any fallible work, so this is safe
        // on the error path too.
        let owned = unsafe { take_pool_balances(rows, count) };
        if take_pwffi_error(env, result) {
            return ptr::null_mut();
        }
        let Ok(length) = jint::try_from(owned.len()) else {
            throw_sdk_exception(env, 1, "Too many shielded token pool balance rows");
            return ptr::null_mut();
        };
        let Ok(array) = env.new_object_array(length, BALANCE_ROW_CLASS, JObject::null()) else {
            return ptr::null_mut();
        };
        for (index, row) in owned.into_iter().enumerate() {
            let result: jni::errors::Result<()> = env.with_local_frame(8, |env| {
                let token_id = JObject::from(env.byte_array_from_slice(&row.token_id)?);
                let contract_id = JObject::from(env.byte_array_from_slice(&row.data_contract_id)?);
                let last_error = match &row.last_error {
                    Some(message) => JObject::from(env.new_string(message)?),
                    None => JObject::null(),
                };
                let object = env.new_object(
                    BALANCE_ROW_CLASS,
                    BALANCE_ROW_CTOR,
                    &[
                        JValue::Object(&token_id),
                        JValue::Object(&contract_id),
                        JValue::Int(jint::from(row.token_contract_position)),
                        JValue::Int(row.account_index as jint),
                        JValue::Long(row.balance as jlong),
                        JValue::Int(row.unspent_note_count as jint),
                        JValue::Long(row.reserved_balance as jlong),
                        JValue::Bool(row.has_scanned_note_count as jboolean),
                        JValue::Long(row.scanned_note_count as jlong),
                        JValue::Bool(row.has_pool_note_count as jboolean),
                        JValue::Long(row.pool_note_count as jlong),
                        JValue::Bool(row.has_scanned_height as jboolean),
                        JValue::Long(row.scanned_height as jlong),
                        JValue::Object(&last_error),
                    ],
                )?;
                env.set_object_array_element(&array, index as jint, object)
            });
            if result.is_err() {
                return ptr::null_mut();
            }
        }
        array.into_raw()
    })
}

/// Send `amount` of the token at `tokenPosition` of `dataContractId` from the
/// notes of `account` to the Orchard address `recipientRaw43` in that token's
/// shielded pool, the fee paid from the account's credit pool notes: bridges
/// `platform_wallet_manager_shielded_token_transfer` (Type 26).
///
/// `amount` is an unsigned 64-bit value in the token's smallest unit crossing
/// as its two's-complement `long`. Returning without an exception means
/// SUBMITTED, not confirmed: the transfer stays pending, its notes reserved,
/// until a shielded sync confirms it. The spend-unconfirmed error means it
/// may have reached a node and must not be retried.
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_ShieldedTokenPoolNative_transfer(
    mut env: JNIEnv,
    _class: JClass,
    manager_handle: jlong,
    wallet_id: JByteArray,
    resolver_handle: jlong,
    account: jint,
    data_contract_id: JByteArray,
    token_position: jint,
    recipient_raw_43: JByteArray,
    amount: jlong,
) {
    guard(&mut env, (), |env| {
        let Some(account) = read_account(env, account) else {
            return;
        };
        let Some(position) = read_token_position(env, token_position) else {
            return;
        };
        let Some(wid) = read_id32(env, &wallet_id, "walletId") else {
            return;
        };
        let Some(contract) = read_id32(env, &data_contract_id, "dataContractId") else {
            return;
        };
        let Some(recipient) = read_raw_43(env, &recipient_raw_43, "recipientRaw43") else {
            return;
        };
        let result = unsafe {
            platform_wallet_manager_shielded_token_transfer(
                manager_handle as Handle,
                wid.as_ptr(),
                resolver_handle as *mut MnemonicResolverHandle,
                account,
                contract.as_ptr(),
                position,
                recipient.as_ptr(),
                amount as u64,
            )
        };
        let _ = take_pwffi_error(env, result);
    })
}

/// Move `amount` of the token at `tokenPosition` of `dataContractId` from the
/// notes of `account` into the token balance of `recipientIdentityId`, the
/// fee paid from the account's credit pool notes: bridges
/// `platform_wallet_manager_shielded_token_unshield` (Type 27). Submitted,
/// never confirmed, exactly as for `transfer`.
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_ShieldedTokenPoolNative_unshield(
    mut env: JNIEnv,
    _class: JClass,
    manager_handle: jlong,
    wallet_id: JByteArray,
    resolver_handle: jlong,
    account: jint,
    data_contract_id: JByteArray,
    token_position: jint,
    recipient_identity_id: JByteArray,
    amount: jlong,
) {
    guard(&mut env, (), |env| {
        let Some(account) = read_account(env, account) else {
            return;
        };
        let Some(position) = read_token_position(env, token_position) else {
            return;
        };
        let Some(wid) = read_id32(env, &wallet_id, "walletId") else {
            return;
        };
        let Some(contract) = read_id32(env, &data_contract_id, "dataContractId") else {
            return;
        };
        let Some(recipient) = read_id32(env, &recipient_identity_id, "recipientIdentityId") else {
            return;
        };
        let result = unsafe {
            platform_wallet_manager_shielded_token_unshield(
                manager_handle as Handle,
                wid.as_ptr(),
                resolver_handle as *mut MnemonicResolverHandle,
                account,
                contract.as_ptr(),
                position,
                recipient.as_ptr(),
                amount as u64,
            )
        };
        let _ = take_pwffi_error(env, result);
    })
}

/// What a token pool spend of `kind` (`0` transfer, `1` unshield) moving
/// `amount` would take from `account`'s notes: bridges
/// `platform_wallet_manager_shielded_token_spend_preview`. Returns
/// `[fee, tokenNotes, feeNotes]` (the fee in credits, the counts as
/// non-negative values); throws Rust's refusal when the spend would be
/// refused on these notes. Local state only: nothing is reserved or sent.
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_ShieldedTokenPoolNative_spendPreview(
    mut env: JNIEnv,
    _class: JClass,
    manager_handle: jlong,
    wallet_id: JByteArray,
    account: jint,
    data_contract_id: JByteArray,
    token_position: jint,
    kind: jbyte,
    amount: jlong,
) -> jlongArray {
    guard(&mut env, ptr::null_mut(), |env| {
        let Some(account) = read_account(env, account) else {
            return ptr::null_mut();
        };
        let Some(position) = read_token_position(env, token_position) else {
            return ptr::null_mut();
        };
        let Some(wid) = read_id32(env, &wallet_id, "walletId") else {
            return ptr::null_mut();
        };
        let Some(contract) = read_id32(env, &data_contract_id, "dataContractId") else {
            return ptr::null_mut();
        };
        let mut fee: u64 = 0;
        let mut token_notes: u32 = 0;
        let mut fee_notes: u32 = 0;
        let result = unsafe {
            platform_wallet_manager_shielded_token_spend_preview(
                manager_handle as Handle,
                wid.as_ptr(),
                account,
                contract.as_ptr(),
                position,
                kind as u8,
                amount as u64,
                &mut fee,
                &mut token_notes,
                &mut fee_notes,
            )
        };
        if take_pwffi_error(env, result) {
            return ptr::null_mut();
        }
        let values = preview_values(fee, token_notes, fee_notes);
        let Ok(array) = env.new_long_array(values.len() as jint) else {
            return ptr::null_mut();
        };
        if env.set_long_array_region(&array, 0, &values).is_err() {
            return ptr::null_mut();
        }
        array.into_raw()
    })
}

/// The `long[]` the preview export returns: the fee's bit pattern, then the
/// two note counts widened without a sign.
fn preview_values(fee: u64, token_notes: u32, fee_notes: u32) -> [jlong; 3] {
    [
        fee as jlong,
        jlong::from(token_notes),
        jlong::from(fee_notes),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CString;

    fn ffi_row(
        last_error: *const std::os::raw::c_char,
        scanned: bool,
    ) -> ShieldedTokenPoolBalanceFFI {
        ShieldedTokenPoolBalanceFFI {
            token_id: [3; 32],
            data_contract_id: [4; 32],
            token_contract_position: 7,
            account_index: 2,
            balance: u64::MAX,
            unspent_note_count: 3,
            reserved_balance: 250,
            has_scanned_note_count: scanned,
            scanned_note_count: if scanned { 2_048 } else { 0 },
            has_pool_note_count: scanned,
            pool_note_count: if scanned { 4_096 } else { 0 },
            has_scanned_height: scanned,
            scanned_height: if scanned { 123_456 } else { 0 },
            last_error,
        }
    }

    #[test]
    fn should_copy_every_field_and_the_error_string_of_a_row() {
        let message = CString::new("chunk read failed").expect("no NUL");
        let row = ffi_row(message.as_ptr(), true);

        let owned = unsafe { OwnedPoolBalance::copy_from(&row) };

        assert_eq!(
            owned,
            OwnedPoolBalance {
                token_id: [3; 32],
                data_contract_id: [4; 32],
                token_contract_position: 7,
                account_index: 2,
                balance: u64::MAX,
                unspent_note_count: 3,
                reserved_balance: 250,
                has_scanned_note_count: true,
                scanned_note_count: 2_048,
                has_pool_note_count: true,
                pool_note_count: 4_096,
                has_scanned_height: true,
                scanned_height: 123_456,
                last_error: Some("chunk read failed".to_string()),
            }
        );
    }

    #[test]
    fn should_copy_a_null_error_as_none_and_keep_the_cleared_flags() {
        let owned = unsafe { OwnedPoolBalance::copy_from(&ffi_row(ptr::null(), false)) };

        assert_eq!(owned.last_error, None);
        assert!(!owned.has_scanned_note_count);
        assert!(!owned.has_pool_note_count);
        assert!(!owned.has_scanned_height);
    }

    #[test]
    fn should_take_an_empty_list_from_a_null_array() {
        assert!(unsafe { take_pool_balances(ptr::null_mut(), 0) }.is_empty());
        assert!(unsafe { take_pool_balances(ptr::null_mut(), 3) }.is_empty());
    }

    #[test]
    fn should_keep_the_fee_bits_and_widen_the_note_counts_without_a_sign() {
        assert_eq!(
            preview_values(u64::MAX, u32::MAX, 2),
            [-1, i64::from(u32::MAX), 2]
        );
    }
}
