//! JNI exports for raising one identity key's limits (protocol version 14) from a reviewed
//! preview, and for the proved identity refresh that review starts from. Each export marshals
//! ONE C function of `platform-wallet-ffi`, the ones Swift's `ManagedPlatformWallet` calls
//! from `UpdateKeyLimitsView` and `KeyDetailView`:
//!
//! - `previewKeyLimitsUpdate` -> `platform_wallet_preview_identity_key_limits_update`
//!   (`previewIdentityKeyLimitsUpdate`): whether the key can be raised, by which key, and
//!   the limits a change would set. Nothing is signed or sent.
//! - `updateKeyLimitsAsReviewed` ->
//!   `platform_wallet_update_identity_key_limits_as_reviewed_with_signer`
//!   (`updateIdentityKeyLimits(asReviewed:)`): signs exactly the reviewed limits, or nothing.
//! - `refreshIdentity` -> `platform_wallet_refresh_identity` (`refreshIdentity`).
//!
//! Rust decides everything (which key signs, whether a change is a raise, whether the key
//! would be expired). This module copies values between the JVM and the C struct; a JVM value
//! the C type cannot carry (an id that is not 32 bytes, a null signer, a preview array of the
//! wrong shape) throws code 1 (`InvalidParameter`) before any C call. Key ids, credits and
//! milliseconds cross as their unsigned bit patterns, as the persister writes key ids, so a
//! value past the signed range round-trips unchanged.
//!
//! Kotlin counterpart: `org.dashfoundation.dashsdk.ffi.IdentityKeyLimitsNative`.

use crate::decision_proposals::read_fixed_bytes;
use crate::support::{guard, take_pwffi_error, throw_sdk_exception};
use jni::objects::{JByteArray, JClass, JLongArray};
use jni::sys::{jboolean, jint, jlong, jlongArray};
use jni::JNIEnv;
use platform_wallet_ffi::handle::Handle;
use platform_wallet_ffi::identity_key_limits::{
    platform_wallet_preview_identity_key_limits_update,
    platform_wallet_update_identity_key_limits_as_reviewed_with_signer,
    IdentityKeyLimitsUpdatePreviewFFI,
};
use platform_wallet_ffi::platform_wallet_refresh_identity;
use rs_sdk_ffi::SignerHandle;
use std::ptr;

/// Number of `long` slots a preview crosses JNI in.
const PREVIEW_SLOTS: usize = 6;
/// `flags` bit: the key carries a total budget now.
const HAS_CURRENT_TOTAL_BUDGET: i64 = 1;
/// `flags` bit: the key carries an expiry now.
const HAS_CURRENT_EXPIRES_AT: i64 = 1 << 1;
/// `flags` bit: the update would set a new total budget.
const HAS_NEW_TOTAL_BUDGET: i64 = 1 << 2;
/// `flags` bit: the update would set a new expiry.
const HAS_NEW_EXPIRES_AT: i64 = 1 << 3;
/// Every `flags` bit this layout defines.
const KNOWN_FLAGS: i64 =
    HAS_CURRENT_TOTAL_BUDGET | HAS_CURRENT_EXPIRES_AT | HAS_NEW_TOTAL_BUDGET | HAS_NEW_EXPIRES_AT;

/// The preview's slots, without the identity and key it names (the Kotlin caller holds
/// those): `[signingKeyId, flags, currentTotalBudget, currentExpiresAt, newTotalBudget,
/// newExpiresAt]`. Each value is the `u32`/`u64` bit pattern; a value whose flag is clear
/// is 0.
fn preview_slots(preview: &IdentityKeyLimitsUpdatePreviewFFI) -> [i64; PREVIEW_SLOTS] {
    let mut flags = 0;
    if preview.has_current_total_budget {
        flags |= HAS_CURRENT_TOTAL_BUDGET;
    }
    if preview.has_current_expires_at {
        flags |= HAS_CURRENT_EXPIRES_AT;
    }
    if preview.has_new_total_budget {
        flags |= HAS_NEW_TOTAL_BUDGET;
    }
    if preview.has_new_expires_at {
        flags |= HAS_NEW_EXPIRES_AT;
    }
    let value = |has: bool, value: u64| if has { value as i64 } else { 0 };
    [
        i64::from(preview.signing_key_id),
        flags,
        value(
            preview.has_current_total_budget,
            preview.current_total_budget,
        ),
        value(preview.has_current_expires_at, preview.current_expires_at),
        value(preview.has_new_total_budget, preview.new_total_budget),
        value(preview.has_new_expires_at, preview.new_expires_at),
    ]
}

/// The C preview of key `key_id` of `identity_id` that `slots` (as [`preview_slots`] wrote
/// them) describe, or why the slots are not such a preview.
fn preview_from_slots(
    identity_id: [u8; 32],
    key_id: u32,
    slots: &[i64],
) -> Result<IdentityKeyLimitsUpdatePreviewFFI, String> {
    let [signing_key_id, flags, current_total, current_expires, new_total, new_expires] =
        <[i64; PREVIEW_SLOTS]>::try_from(slots).map_err(|_| {
            format!(
                "reviewed preview must hold {PREVIEW_SLOTS} values, got {}",
                slots.len()
            )
        })?;
    let signing_key_id = u32::try_from(signing_key_id)
        .map_err(|_| format!("reviewed signing key id {signing_key_id} is not a key id"))?;
    if flags & !KNOWN_FLAGS != 0 {
        return Err(format!("reviewed preview flags {flags:#x} are not known"));
    }
    Ok(IdentityKeyLimitsUpdatePreviewFFI {
        identity_id,
        key_id,
        signing_key_id,
        has_current_total_budget: flags & HAS_CURRENT_TOTAL_BUDGET != 0,
        current_total_budget: current_total as u64,
        has_current_expires_at: flags & HAS_CURRENT_EXPIRES_AT != 0,
        current_expires_at: current_expires as u64,
        has_new_total_budget: flags & HAS_NEW_TOTAL_BUDGET != 0,
        new_total_budget: new_total as u64,
        has_new_expires_at: flags & HAS_NEW_EXPIRES_AT != 0,
        new_expires_at: new_expires as u64,
    })
}

/// The signer handle as the C pointer, or `None` after throwing code 1 for 0.
fn read_signer(env: &mut JNIEnv, signer_handle: jlong) -> Option<*mut SignerHandle> {
    if signer_handle == 0 {
        throw_sdk_exception(env, 1, "signerHandle must not be 0");
        return None;
    }
    Some(signer_handle as *mut SignerHandle)
}

/// What raising the limits of key `keyId` of `identityId` would do
/// (`platform_wallet_preview_identity_key_limits_update`), as the six preview slots
/// [`preview_slots`] documents; null after throwing. `addBudget` (credits) counts only when
/// `hasAddBudget`, `expiresAt` (block time in milliseconds) only when `hasExpiresAt`; with
/// neither, Rust answers whether the key can be raised at all and by which key. A refusal
/// (missing, disabled or unlimited key, a change that is not a raise, a key that would be
/// expired, no allowed signing key) throws with Rust's reason.
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_IdentityKeyLimitsNative_previewKeyLimitsUpdate(
    mut env: JNIEnv,
    _class: JClass,
    wallet_handle: jlong,
    identity_id: JByteArray,
    key_id: jint,
    has_add_budget: jboolean,
    add_budget: jlong,
    has_expires_at: jboolean,
    expires_at: jlong,
    signer_handle: jlong,
) -> jlongArray {
    guard(&mut env, ptr::null_mut(), |env| {
        let Some(id) = read_fixed_bytes::<32>(env, &identity_id, "identityId") else {
            return ptr::null_mut();
        };
        let Some(signer) = read_signer(env, signer_handle) else {
            return ptr::null_mut();
        };
        let mut preview = IdentityKeyLimitsUpdatePreviewFFI::default();
        let result = unsafe {
            platform_wallet_preview_identity_key_limits_update(
                wallet_handle as Handle,
                id.as_ptr(),
                key_id as u32,
                has_add_budget != 0,
                add_budget as u64,
                has_expires_at != 0,
                expires_at as u64,
                signer,
                &mut preview,
            )
        };
        if take_pwffi_error(env, result) {
            return ptr::null_mut();
        }
        let Ok(out) = env.new_long_array(PREVIEW_SLOTS as i32) else {
            return ptr::null_mut();
        };
        if env
            .set_long_array_region(&out, 0, &preview_slots(&preview))
            .is_err()
        {
            return ptr::null_mut();
        }
        out.into_raw()
    })
}

/// Raise the limits of key `keyId` of `identityId` to exactly what `reviewed` (the slots
/// `previewKeyLimitsUpdate` returned for that key) shows
/// (`platform_wallet_update_identity_key_limits_as_reviewed_with_signer`). Rust refuses,
/// before a nonce is taken, when the wallet's copy moved so the same change would now sign
/// other limits or another key. Throws on any failure; the key row reaches Room through the
/// persistence changeset.
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_IdentityKeyLimitsNative_updateKeyLimitsAsReviewed(
    mut env: JNIEnv,
    _class: JClass,
    wallet_handle: jlong,
    identity_id: JByteArray,
    key_id: jint,
    reviewed: JLongArray,
    signer_handle: jlong,
) {
    guard(&mut env, (), |env| {
        let Some(id) = read_fixed_bytes::<32>(env, &identity_id, "identityId") else {
            return;
        };
        let Some(signer) = read_signer(env, signer_handle) else {
            return;
        };
        if reviewed.is_null() {
            throw_sdk_exception(env, 1, "reviewed must not be null");
            return;
        }
        let mut slots = [0i64; PREVIEW_SLOTS];
        let length = match env.get_array_length(&reviewed) {
            Ok(length) => length,
            Err(_) => {
                let _ = env.exception_clear();
                throw_sdk_exception(env, 1, "reviewed long[] could not be read");
                return;
            }
        };
        if length as usize != PREVIEW_SLOTS {
            throw_sdk_exception(
                env,
                1,
                &format!("reviewed preview must hold {PREVIEW_SLOTS} values, got {length}"),
            );
            return;
        }
        if env.get_long_array_region(&reviewed, 0, &mut slots).is_err() {
            let _ = env.exception_clear();
            throw_sdk_exception(env, 1, "reviewed long[] could not be read");
            return;
        }
        let preview = match preview_from_slots(id, key_id as u32, &slots) {
            Ok(preview) => preview,
            Err(reason) => {
                throw_sdk_exception(env, 1, &reason);
                return;
            }
        };
        let result = unsafe {
            platform_wallet_update_identity_key_limits_as_reviewed_with_signer(
                wallet_handle as Handle,
                &preview,
                signer,
            )
        };
        take_pwffi_error(env, result);
    })
}

/// Fetch `identityId` from Platform with a proof and fold it into the wallet
/// (`platform_wallet_refresh_identity`): keys added or raised on another client are taken
/// in, and changed key rows reach Room through the persister. Throws on failure.
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_IdentityKeyLimitsNative_refreshIdentity(
    mut env: JNIEnv,
    _class: JClass,
    wallet_handle: jlong,
    identity_id: JByteArray,
) {
    guard(&mut env, (), |env| {
        let Some(id) = read_fixed_bytes::<32>(env, &identity_id, "identityId") else {
            return;
        };
        let result =
            unsafe { platform_wallet_refresh_identity(wallet_handle as Handle, id.as_ptr()) };
        take_pwffi_error(env, result);
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::decision_proposals::tests::kotlin_signature_in;

    const IDENTITY_KEY_LIMITS_NATIVE_KT: &str = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../kotlin-sdk/sdk/src/main/kotlin/org/dashfoundation/dashsdk/ffi/IdentityKeyLimitsNative.kt"
    ));

    fn types(kinds: &[&str]) -> Vec<String> {
        kinds.iter().map(|kind| kind.to_string()).collect()
    }

    /// JNI binds an export by its name alone: a Kotlin declaration whose parameters drifted
    /// from the export's would hand it the wrong JVM values at run time.
    #[test]
    fn should_declare_each_export_as_it_takes_its_arguments() {
        assert_eq!(
            kotlin_signature_in(IDENTITY_KEY_LIMITS_NATIVE_KT, "previewKeyLimitsUpdate"),
            (
                types(&[
                    "Long",
                    "ByteArray",
                    "Int",
                    "Boolean",
                    "Long",
                    "Boolean",
                    "Long",
                    "Long",
                ]),
                "LongArray".to_string()
            )
        );
        assert_eq!(
            kotlin_signature_in(IDENTITY_KEY_LIMITS_NATIVE_KT, "updateKeyLimitsAsReviewed"),
            (
                types(&["Long", "ByteArray", "Int", "LongArray", "Long"]),
                "Unit".to_string()
            )
        );
        assert_eq!(
            kotlin_signature_in(IDENTITY_KEY_LIMITS_NATIVE_KT, "refreshIdentity"),
            (types(&["Long", "ByteArray"]), "Unit".to_string())
        );
    }

    #[test]
    fn should_round_trip_a_preview_through_its_slots_unchanged() {
        let preview = IdentityKeyLimitsUpdatePreviewFFI {
            identity_id: [7; 32],
            key_id: u32::MAX,
            signing_key_id: u32::MAX - 1,
            has_current_total_budget: true,
            current_total_budget: u64::MAX,
            has_current_expires_at: true,
            current_expires_at: 1_700_000_000_000,
            has_new_total_budget: true,
            new_total_budget: u64::MAX - 5,
            has_new_expires_at: false,
            new_expires_at: 0,
        };
        let slots = preview_slots(&preview);
        assert_eq!(slots[0], i64::from(u32::MAX - 1));
        assert_eq!(
            slots[1],
            HAS_CURRENT_TOTAL_BUDGET | HAS_CURRENT_EXPIRES_AT | HAS_NEW_TOTAL_BUDGET
        );
        assert_eq!(slots[2], -1, "u64::MAX crosses as its bit pattern");
        assert_eq!(preview_from_slots([7; 32], u32::MAX, &slots), Ok(preview));
    }

    #[test]
    fn should_write_zero_for_a_value_whose_flag_is_clear() {
        let preview = IdentityKeyLimitsUpdatePreviewFFI {
            signing_key_id: 0,
            has_current_expires_at: true,
            current_expires_at: 42,
            // A value without its flag is not meaningful and does not cross.
            new_total_budget: 99,
            ..Default::default()
        };
        assert_eq!(
            preview_slots(&preview),
            [0, HAS_CURRENT_EXPIRES_AT, 0, 42, 0, 0]
        );
    }

    #[test]
    fn should_refuse_slots_that_are_not_a_preview() {
        assert!(preview_from_slots([0; 32], 1, &[0; 5]).is_err());
        assert!(preview_from_slots([0; 32], 1, &[0; 7]).is_err());
        assert!(preview_from_slots([0; 32], 1, &[-1, 0, 0, 0, 0, 0]).is_err());
        assert!(preview_from_slots([0; 32], 1, &[i64::from(u32::MAX) + 1, 0, 0, 0, 0, 0]).is_err());
        assert!(preview_from_slots([0; 32], 1, &[0, 1 << 4, 0, 0, 0, 0]).is_err());
    }
}
