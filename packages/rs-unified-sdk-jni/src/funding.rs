//! JNI bridge for shielded-funding support helpers — the Halo 2 prover
//! warm-up / readiness probe and the consensus-pinned shielded fee
//! estimator.
//!
//! Kotlin counterpart: `org.dashfoundation.dashsdk.ffi.FundingNative`,
//! driven by `org.dashfoundation.dashsdk.funding.ShieldedProver`.
//!
//! ## What lives here (and what deliberately doesn't)
//!
//! These three entry points are process-global, take no wallet handle,
//! and back the shielded funding screens' prover-status indicator and
//! fee preview:
//! - [`platform_wallet_shielded_warm_up_prover`] — kick the ~30s Halo 2
//!   proving-key build onto a background thread.
//! - [`platform_wallet_shielded_prover_is_ready`] — poll whether that
//!   build has finished (UI "preparing prover…" affordance).
//! - [`platform_wallet_shielded_estimate_fee`] — the flat shielded fee in
//!   credits for a transition of a given kind + action count.
//!
//! The heavy shielded funding transitions themselves — the shielded
//! fund-from-asset-lock (+ its resume-by-outpoint variant) and the
//! shielded seed-pool batch builder — are bridged below. They are
//! **manager-handle** entry points (they take the manager `Handle` + the
//! 32-byte wallet id + a `MnemonicResolverHandle`, resolving the wallet and
//! its shielded coordinator Rust-side), so their Kotlin wrappers live on
//! `PlatformWalletManager` rather than `ManagedPlatformWallet` — matching
//! the Swift `PlatformWalletManager.shieldedFundFromAssetLock` /
//! `seedShieldedPoolNotes` shapes. No orchestration crosses the JNI
//! boundary: each is a single FFI call whose result carries only a status
//! code (the Orchard note arrives on the next shielded sync pass, not
//! synchronously). The seed-pool builder also drives an optional progress
//! callback bridged to a Kotlin `SeedPoolProgressBridge`.
//!
//! The whole module compiles only under the `shielded` cargo feature (on
//! by default), matching `platform-wallet-ffi`'s gate on `shielded_send`.

#![allow(clippy::missing_safety_doc)]
#![cfg(feature = "shielded")]

use crate::support::{guard, throw_sdk_exception, JVM};
use jni::objects::{GlobalRef, JByteArray, JClass, JObject};
use jni::sys::{jboolean, jint, jlong, JNI_FALSE, JNI_TRUE};
use jni::JNIEnv;
use platform_wallet_ffi::error::{
    platform_wallet_ffi_result_free, PlatformWalletFFIResult, PlatformWalletFFIResultCode,
};
use platform_wallet_ffi::handle::Handle;
use std::os::raw::c_void;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::ptr;

use rs_sdk_ffi::MnemonicResolverHandle;

/// If `result` carries a non-`Success` code: throw `DashSDKException`,
/// free its message, and return `true`. Local copy of the shared
/// `identity::take_pwffi_error` mapping.
fn take_pwffi_error(env: &mut JNIEnv, mut result: PlatformWalletFFIResult) -> bool {
    if result.code == PlatformWalletFFIResultCode::Success {
        return false;
    }
    let message = if result.message.is_null() {
        format!("platform-wallet error (code {})", result.code as i32)
    } else {
        // SAFETY: non-null message is a valid CString produced by the FFI.
        unsafe { std::ffi::CStr::from_ptr(result.message) }
            .to_string_lossy()
            .into_owned()
    };
    throw_sdk_exception(env, result.code as i32, &message);
    // SAFETY: `result` is a fresh PlatformWalletFFIResult; free its message.
    unsafe { platform_wallet_ffi_result_free(&mut result) };
    true
}

/// Kick the Halo 2 proving-key build onto a background thread so the
/// first shielded transition doesn't pay the ~30s cost inline. Idempotent
/// and process-global — safe to call at app start and on every entry to a
/// shielded screen.
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_FundingNative_warmUpProver(
    mut env: JNIEnv,
    _class: JClass,
) {
    guard(&mut env, (), |_env| unsafe {
        platform_wallet_ffi::platform_wallet_shielded_warm_up_prover();
    })
}

/// Whether the Halo 2 proving key is already built. `false` doesn't block
/// shielded sends — it just means the next one pays the build cost — so
/// the funding screens use it purely as a "preparing prover…" indicator.
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_FundingNative_proverIsReady(
    mut env: JNIEnv,
    _class: JClass,
) -> jboolean {
    guard(&mut env, JNI_FALSE, |_env| {
        let ready = unsafe { platform_wallet_ffi::platform_wallet_shielded_prover_is_ready() };
        if ready {
            JNI_TRUE
        } else {
            JNI_FALSE
        }
    })
}

/// The flat shielded fee in credits for a transition of the given `kind`
/// (`0` = ShieldedTransfer/Shield, `1` = Unshield, `2` = ShieldedWithdrawal)
/// and Orchard action `count` (a single-note spend with change is 2
/// actions). Pure computation — no wallet handle, no network. Throws on an
/// unknown kind or a fee-formula overflow.
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_FundingNative_estimateShieldedFee(
    mut env: JNIEnv,
    _class: JClass,
    kind: jint,
    num_actions: jint,
) -> jlong {
    guard(&mut env, 0i64, |env| {
        if kind < 0 || kind > u8::MAX as jint {
            throw_sdk_exception(env, 1, &format!("invalid shielded fee kind {kind}"));
            return 0;
        }
        let mut out_fee: u64 = 0;
        let result = unsafe {
            platform_wallet_ffi::platform_wallet_shielded_estimate_fee(
                kind as u8,
                num_actions.max(0) as usize,
                &mut out_fee as *mut u64,
            )
        };
        if take_pwffi_error(env, result) {
            return 0;
        }
        out_fee as i64
    })
}

// ── Shielded funding submits ──────────────────────────────────────────
//
// These are manager-handle entry points (see the module doc). Each is a
// single blocking FFI call — the Orchard note arrives on the next shielded
// sync pass, so there is no changeset / balance payload to marshal back;
// success is just `PlatformWalletFFIResult::ok()`.

/// Read a required 32-byte id from a Java `byte[]`; throws + returns None on
/// the wrong length or a JNI error. `field` names the argument for the message.
fn read_id32(env: &mut JNIEnv, arr: &JByteArray, field: &str) -> Option<[u8; 32]> {
    if arr.is_null() {
        throw_sdk_exception(env, 1, &format!("{field} byte[] was null"));
        return None;
    }
    let bytes = match env.convert_byte_array(arr) {
        Ok(b) => b,
        Err(_) => {
            let _ = env.exception_clear();
            throw_sdk_exception(env, 1, &format!("{field} byte[] was invalid"));
            return None;
        }
    };
    if bytes.len() != 32 {
        throw_sdk_exception(
            env,
            1,
            &format!("{field} must be 32 bytes, got {}", bytes.len()),
        );
        return None;
    }
    let mut id = [0u8; 32];
    id.copy_from_slice(&bytes);
    Some(id)
}

/// Read a required 43-byte raw Orchard recipient address from a Java
/// `byte[]` (11-byte diversifier + 32-byte pk_d); throws + returns None on
/// the wrong length / a JNI error.
fn read_recipient43(env: &mut JNIEnv, arr: &JByteArray) -> Option<[u8; 43]> {
    if arr.is_null() {
        throw_sdk_exception(env, 1, "recipientRaw43 byte[] was null");
        return None;
    }
    let bytes = match env.convert_byte_array(arr) {
        Ok(b) => b,
        Err(_) => {
            let _ = env.exception_clear();
            throw_sdk_exception(env, 1, "recipientRaw43 byte[] was invalid");
            return None;
        }
    };
    if bytes.len() != 43 {
        throw_sdk_exception(
            env,
            1,
            &format!("recipientRaw43 must be 43 bytes, got {}", bytes.len()),
        );
        return None;
    }
    let mut out = [0u8; 43];
    out.copy_from_slice(&bytes);
    Some(out)
}

/// Read an OPTIONAL byte array (e.g. the 21-byte surplus platform address).
/// JVM null → `Ok(None)`. Returns `Err(())` (after throwing) on a JNI error.
fn read_opt_bytes(env: &mut JNIEnv, arr: &JByteArray) -> Result<Option<Vec<u8>>, ()> {
    if arr.is_null() {
        return Ok(None);
    }
    match env.convert_byte_array(arr) {
        Ok(b) => Ok(Some(b)),
        Err(_) => {
            let _ = env.exception_clear();
            throw_sdk_exception(env, 1, "surplusOutput byte[] was invalid");
            Err(())
        }
    }
}

/// The default Orchard payment address for `account` on the wallet's bound
/// shielded sub-wallet — bridges `platform_wallet_manager_shielded_default_address`.
/// Returns the 43 raw bytes (11-byte diversifier + 32-byte pk_d) as a
/// `byte[]`, or null when the wallet has no bound shielded sub-wallet /
/// `account` isn't bound. The natural "shield to self" default recipient for
/// [`Java_..._shieldedFundFromAssetLock`] (← Swift `shieldedDefaultAddress`).
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_FundingNative_shieldedDefaultAddress(
    mut env: JNIEnv,
    _class: JClass,
    manager_handle: jlong,
    wallet_id: JByteArray,
    account: jint,
) -> jni::sys::jbyteArray {
    guard(&mut env, ptr::null_mut(), |env| {
        let Some(wid) = read_id32(env, &wallet_id, "walletId") else {
            return ptr::null_mut();
        };
        let mut out_bytes = [0u8; 43];
        let mut present = false;
        let result = unsafe {
            platform_wallet_ffi::platform_wallet_manager_shielded_default_address(
                manager_handle as Handle,
                wid.as_ptr(),
                account.max(0) as u32,
                out_bytes.as_mut_ptr(),
                &mut present as *mut bool,
            )
        };
        if take_pwffi_error(env, result) {
            return ptr::null_mut();
        }
        if !present {
            return ptr::null_mut();
        }
        env.byte_array_from_slice(&out_bytes)
            .map(|a| a.into_raw())
            .unwrap_or(ptr::null_mut())
    })
}

/// Fund the wallet's shielded (Orchard) pool from a fresh Core L1 asset lock
/// built from the wallet's balance — bridges
/// `platform_wallet_manager_shielded_fund_from_asset_lock`.
///
/// Mirrors Swift's `PlatformWalletManager.shieldedFundFromAssetLock`:
/// the manager resolves the wallet + shielded coordinator from `walletId`
/// Rust-side, builds the asset lock funding `amountDuffs` from
/// `fundingAccountIndex`, and shields a single real note to `recipientRaw43`
/// (43-byte raw Orchard address). `coreSignerHandle` is the manager's
/// `MnemonicResolverHandle` (asset-lock outer ST signature); `surplusOutput`
/// is the optional 21-byte remainder platform address (null = none). The
/// ~30s Halo 2 proof runs inside the call; nothing is returned on success.
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_FundingNative_shieldedFundFromAssetLock(
    mut env: JNIEnv,
    _class: JClass,
    manager_handle: jlong,
    wallet_id: JByteArray,
    funding_account_index: jint,
    amount_duffs: jlong,
    recipient_raw43: JByteArray,
    surplus_output: JByteArray,
    core_signer_handle: jlong,
) {
    guard(&mut env, (), |env| {
        let Some(wid) = read_id32(env, &wallet_id, "walletId") else {
            return;
        };
        let Some(recipient) = read_recipient43(env, &recipient_raw43) else {
            return;
        };
        let surplus = match read_opt_bytes(env, &surplus_output) {
            Ok(v) => v,
            Err(()) => return,
        };
        let (surplus_ptr, surplus_len) = surplus
            .as_ref()
            .map_or((ptr::null(), 0usize), |v| (v.as_ptr(), v.len()));
        let result = unsafe {
            platform_wallet_ffi::platform_wallet_manager_shielded_fund_from_asset_lock(
                manager_handle as Handle,
                wid.as_ptr(),
                funding_account_index.max(0) as u32,
                amount_duffs.max(0) as u64,
                recipient.as_ptr(),
                surplus_ptr,
                surplus_len,
                core_signer_handle as *mut MnemonicResolverHandle,
            )
        };
        let _ = take_pwffi_error(env, result);
    })
}

/// Resume a shielded fund-from-asset-lock from an already-tracked lock by
/// outpoint — bridges
/// `platform_wallet_manager_shielded_resume_fund_from_asset_lock`. Sibling
/// of [`Java_..._shieldedFundFromAssetLock`]: instead of a fresh amount, it
/// picks up the lock at `(outPointTxid, outPointVout)` and drives the
/// remaining stages. `outPointTxid` is the 32-byte raw txid (little-endian
/// wire order).
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_FundingNative_shieldedResumeFundFromAssetLock(
    mut env: JNIEnv,
    _class: JClass,
    manager_handle: jlong,
    wallet_id: JByteArray,
    out_point_txid: JByteArray,
    out_point_vout: jint,
    recipient_raw43: JByteArray,
    surplus_output: JByteArray,
    core_signer_handle: jlong,
) {
    guard(&mut env, (), |env| {
        let Some(wid) = read_id32(env, &wallet_id, "walletId") else {
            return;
        };
        let Some(txid) = read_id32(env, &out_point_txid, "outPointTxid") else {
            return;
        };
        let Some(recipient) = read_recipient43(env, &recipient_raw43) else {
            return;
        };
        let surplus = match read_opt_bytes(env, &surplus_output) {
            Ok(v) => v,
            Err(()) => return,
        };
        let (surplus_ptr, surplus_len) = surplus
            .as_ref()
            .map_or((ptr::null(), 0usize), |v| (v.as_ptr(), v.len()));
        let out_point = platform_wallet_ffi::OutPointFFI {
            txid,
            vout: out_point_vout.max(0) as u32,
        };
        let result = unsafe {
            platform_wallet_ffi::platform_wallet_manager_shielded_resume_fund_from_asset_lock(
                manager_handle as Handle,
                wid.as_ptr(),
                &out_point as *const platform_wallet_ffi::OutPointFFI,
                recipient.as_ptr(),
                surplus_ptr,
                surplus_len,
                core_signer_handle as *mut MnemonicResolverHandle,
            )
        };
        let _ = take_pwffi_error(env, result);
    })
}

/// Bridge the shielded seed-pool progress C callback into a Kotlin
/// `SeedPoolProgressBridge.onProgress(long,long,long,long)`. `context` is a
/// live [`GlobalRef`] to the bridge object (or null when no progress
/// listener was supplied). Fires on the blocking call's worker thread, so it
/// attaches the thread as a daemon and swallows panics — fire-and-forget,
/// like the event trampolines in `events.rs`.
///
/// # Safety
/// `context`, when non-null, must be a live `GlobalRef` pointer for the
/// duration of the seed-pool call.
unsafe extern "C" fn seed_pool_progress_trampoline(
    context: *mut c_void,
    batch_index: u64,
    batches_total_estimate: u64,
    pool_notes_now: u64,
    target: u64,
) {
    let _ = catch_unwind(AssertUnwindSafe(|| {
        if context.is_null() {
            return;
        }
        let Some(vm) = JVM.get() else { return };
        let Ok(mut env) = vm.attach_current_thread_as_daemon() else {
            return;
        };
        // SAFETY: `context` is a live GlobalRef pinned for the call duration.
        let bridge = &*(context as *const GlobalRef);
        let call = env.call_method(
            bridge.as_obj(),
            "onProgress",
            "(JJJJ)V",
            &[
                jni::objects::JValue::Long(batch_index as i64),
                jni::objects::JValue::Long(batches_total_estimate as i64),
                jni::objects::JValue::Long(pool_notes_now as i64),
                jni::objects::JValue::Long(target as i64),
            ],
        );
        if call.is_err() {
            let _ = env.exception_clear();
        }
    }));
}

/// Seed the wallet's shielded (Orchard) note pool with real + zero-value
/// filler notes in batches — bridges
/// `platform_wallet_manager_shielded_seed_pool_notes`.
///
/// Mirrors Swift's `PlatformWalletManager.seedShieldedPoolNotes`: the manager
/// resolves the wallet + coordinator from `walletId`, builds notes toward
/// `targetTotalNotes` on shielded `account` (funded from
/// `fundingAccountIndex`), running one ~30s Halo 2 proof per batch serially.
/// `coreSignerHandle` is the manager's `MnemonicResolverHandle`.
/// `progressBridge` is an optional Kotlin `SeedPoolProgressBridge` (null =
/// no progress); it is held as a local `GlobalRef` for the (blocking) call
/// duration and released before returning. Nothing is returned on success.
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_FundingNative_shieldedSeedPoolNotes(
    mut env: JNIEnv,
    _class: JClass,
    manager_handle: jlong,
    wallet_id: JByteArray,
    account: jint,
    target_total_notes: jlong,
    funding_account_index: jint,
    core_signer_handle: jlong,
    progress_bridge: JObject,
) {
    guard(&mut env, (), |env| {
        let Some(wid) = read_id32(env, &wallet_id, "walletId") else {
            return;
        };

        // Hold the optional progress bridge as a GlobalRef for the call
        // duration; box it so the trampoline gets a stable `*const GlobalRef`.
        let progress_global: Option<Box<GlobalRef>> = if progress_bridge.is_null() {
            None
        } else {
            match env.new_global_ref(&progress_bridge) {
                Ok(g) => Some(Box::new(g)),
                Err(_) => {
                    let _ = env.exception_clear();
                    throw_sdk_exception(env, 99, "NewGlobalRef(seed-pool progress bridge) failed");
                    return;
                }
            }
        };
        let (progress_fn, progress_ctx): (
            Option<unsafe extern "C" fn(*mut c_void, u64, u64, u64, u64)>,
            *mut c_void,
        ) = match progress_global.as_ref() {
            Some(g) => (
                Some(seed_pool_progress_trampoline),
                g.as_ref() as *const GlobalRef as *mut c_void,
            ),
            None => (None, ptr::null_mut()),
        };

        let result = unsafe {
            platform_wallet_ffi::platform_wallet_manager_shielded_seed_pool_notes(
                manager_handle as Handle,
                wid.as_ptr(),
                account.max(0) as u32,
                target_total_notes.max(0) as u64,
                funding_account_index.max(0) as u32,
                core_signer_handle as *mut MnemonicResolverHandle,
                progress_fn,
                progress_ctx,
            )
        };
        // `progress_global` (and the boxed GlobalRef the trampoline
        // dereferenced) stays alive through the blocking call above; it is
        // dropped here, after the FFI can no longer fire the callback.
        drop(progress_global);
        let _ = take_pwffi_error(env, result);
    })
}
