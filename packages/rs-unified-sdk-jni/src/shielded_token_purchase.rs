//! JNI bridge for buying a token from the shielded credit pool: the Type 28
//! `TokenPurchaseFromShieldedPool` transition and the check for whether a
//! token can be bought that way.
//!
//! Kotlin counterpart:
//! `org.dashfoundation.dashsdk.ffi.ShieldedTokenPurchaseNative`, driven by
//! the `PlatformWalletManager.shieldedTokenPurchase*` wrappers. Swift
//! counterpart: `shieldedTokenPurchaseFromPool` /
//! `shieldedTokenPurchaseAvailability` in `PlatformWalletManagerShieldedSync.swift`.
//!
//! Each export is one call into `platform_wallet_ffi::shielded_send`.
//! Nothing is decided here: the wallet checks the token count, the network's
//! protocol version, the token's shielded pool and the note count, selects
//! the notes, builds, broadcasts, records the activity and requests the
//! shielded sync that looks for it.
//!
//! The whole module compiles only under the `shielded` cargo feature.

#![allow(clippy::missing_safety_doc)]
#![cfg(feature = "shielded")]

use crate::support::{guard, take_pwffi_error, throw_sdk_exception};
use jni::objects::{JByteArray, JClass};
use jni::sys::{jint, jlong};
use jni::JNIEnv;
use platform_wallet_ffi::handle::Handle;
use platform_wallet_ffi::shielded_send::{
    platform_wallet_manager_shielded_token_purchase_availability,
    platform_wallet_manager_shielded_token_purchase_from_pool,
};
use rs_sdk_ffi::MnemonicResolverHandle;

/// Returned by the availability export when the call failed and an
/// exception is pending. Not one of the FFI's availability bytes.
const AVAILABILITY_ON_ERROR: jint = -1;

/// Read a required 32-byte id; throws and returns `None` on JVM null, a JNI
/// error or the wrong length. `field` names the argument in the message.
fn read_id32(env: &mut JNIEnv, arr: &JByteArray, field: &str) -> Option<[u8; 32]> {
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
    match <[u8; 32]>::try_from(bytes.as_slice()) {
        Ok(id) => Some(id),
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

/// A token's position in its contract as the `u16` the FFI takes; throws and
/// returns `None` outside `0..=65535`.
fn read_token_position(env: &mut JNIEnv, position: jint) -> Option<u16> {
    match u16::try_from(position) {
        Ok(position) => Some(position),
        Err(_) => {
            throw_sdk_exception(
                env,
                1,
                &format!("tokenPosition must be 0..=65535, got {position}"),
            );
            None
        }
    }
}

/// Buy `tokenCount` tokens at their direct purchase price from the notes of
/// `account`: bridges
/// `platform_wallet_manager_shielded_token_purchase_from_pool`.
///
/// `tokenCount` and `totalAgreedPrice` are unsigned 64-bit values crossing
/// as their two's-complement `long`. `resolverHandle` supplies the transient
/// spend authority as for the other note-spending exports.
///
/// Returning without an exception means SUBMITTED, not confirmed: Platform
/// accepted the purchase, but its result proof does not show that it
/// executed, so it stays pending until a shielded sync finds it. Errors are
/// thrown as the other note-spending exports throw them; the
/// spend-unconfirmed code means the purchase may have reached a node and
/// must not be retried.
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_ShieldedTokenPurchaseNative_purchaseFromPool(
    mut env: JNIEnv,
    _class: JClass,
    manager_handle: jlong,
    wallet_id: JByteArray,
    resolver_handle: jlong,
    account: jint,
    data_contract_id: JByteArray,
    token_position: jint,
    token_count: jlong,
    total_agreed_price: jlong,
) {
    guard(&mut env, (), |env| {
        if account < 0 {
            throw_sdk_exception(env, 1, "account must be non-negative");
            return;
        }
        let Some(position) = read_token_position(env, token_position) else {
            return;
        };
        let Some(wid) = read_id32(env, &wallet_id, "walletId") else {
            return;
        };
        let Some(contract) = read_id32(env, &data_contract_id, "dataContractId") else {
            return;
        };
        let result = unsafe {
            platform_wallet_manager_shielded_token_purchase_from_pool(
                manager_handle as Handle,
                wid.as_ptr(),
                resolver_handle as *mut MnemonicResolverHandle,
                account as u32,
                contract.as_ptr(),
                position,
                token_count as u64,
                total_agreed_price as u64,
            )
        };
        let _ = take_pwffi_error(env, result);
    })
}

/// Whether the token at `tokenPosition` of `dataContractId` can be bought
/// from the shielded pool: bridges
/// `platform_wallet_manager_shielded_token_purchase_availability`. Returns
/// the FFI's byte: `0` available, `1` the token has no shielded pool, `2`
/// its configuration could not be read (a later call may succeed). Throws
/// when the question itself could not be asked.
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_ShieldedTokenPurchaseNative_availability(
    mut env: JNIEnv,
    _class: JClass,
    manager_handle: jlong,
    data_contract_id: JByteArray,
    token_position: jint,
) -> jint {
    guard(&mut env, AVAILABILITY_ON_ERROR, |env| {
        let Some(position) = read_token_position(env, token_position) else {
            return AVAILABILITY_ON_ERROR;
        };
        let Some(contract) = read_id32(env, &data_contract_id, "dataContractId") else {
            return AVAILABILITY_ON_ERROR;
        };
        // Not an availability byte: an FFI that returned success without
        // writing would be read as unreadable, never as available.
        let mut availability: u8 = 2;
        let result = unsafe {
            platform_wallet_manager_shielded_token_purchase_availability(
                manager_handle as Handle,
                contract.as_ptr(),
                position,
                &mut availability,
            )
        };
        if take_pwffi_error(env, result) {
            return AVAILABILITY_ON_ERROR;
        }
        jint::from(availability)
    })
}
