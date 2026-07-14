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
//! shielded seed-pool batch builder — are bridged below, as are the three
//! **outgoing** shielded spends (transfer / unshield / withdraw, transition
//! types 16/17/19). They are all **manager-handle** entry points (they take
//! the manager `Handle` + the 32-byte wallet id, resolving the wallet and
//! its shielded coordinator Rust-side), so their Kotlin wrappers live on
//! `PlatformWalletManager` rather than `ManagedPlatformWallet` — matching
//! the Swift `PlatformWalletManager.shieldedFundFromAssetLock` /
//! `seedShieldedPoolNotes` / `shieldedTransfer` / `shieldedUnshield` /
//! `shieldedWithdraw` shapes. No orchestration crosses the JNI
//! boundary: each is a single FFI call whose result carries only a status
//! code (the Orchard note arrives on the next shielded sync pass, not
//! synchronously). The seed-pool builder also drives an optional progress
//! callback bridged to a Kotlin `SeedPoolProgressBridge`.
//!
//! The whole module compiles only under the `shielded` cargo feature (on
//! by default), matching `platform-wallet-ffi`'s gate on `shielded_send`.

#![allow(clippy::missing_safety_doc)]
#![cfg(feature = "shielded")]

use crate::identity::{decode_pubkeys_blob, role_for_registration_key_id};
use crate::support::{guard, take_pwffi_error, throw_sdk_exception, JVM};
use jni::objects::{GlobalRef, JByteArray, JClass, JObject, JString};
use jni::sys::{jboolean, jint, jlong, JNI_FALSE, JNI_TRUE};
use jni::JNIEnv;
use platform_wallet_ffi::handle::Handle;
use platform_wallet_ffi::identity_registration_with_signer::IdentityPubkeyFFI;
use std::os::raw::c_void;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::ptr;

use rs_sdk_ffi::{MnemonicResolverHandle, SignerHandle};

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

/// Read a REQUIRED Java `String` into an owned `CString`; throws + returns
/// None on JVM null, a JNI read error, an empty string, or an interior NUL.
/// `field` names the argument in the thrown message. Local sibling of
/// `wallet_manager::read_cstring_required` (that helper is module-private).
fn read_cstring_required(env: &mut JNIEnv, s: &JString, field: &str) -> Option<std::ffi::CString> {
    if s.is_null() {
        throw_sdk_exception(env, 1, &format!("{field} was null"));
        return None;
    }
    let owned: String = match env.get_string(s) {
        Ok(v) => v.into(),
        Err(_) => {
            let _ = env.exception_clear();
            throw_sdk_exception(env, 1, &format!("{field} string was invalid"));
            return None;
        }
    };
    if owned.is_empty() {
        throw_sdk_exception(env, 1, &format!("{field} was empty"));
        return None;
    }
    match std::ffi::CString::new(owned) {
        Ok(c) => Some(c),
        Err(_) => {
            throw_sdk_exception(env, 1, &format!("{field} contained an interior NUL"));
            None
        }
    }
}

/// Read an OPTIONAL Java `String` into `Option<CString>`. JVM null (or an
/// empty string) → `Ok(None)` — the FFI treats a null memo pointer as "no
/// memo", and Rust's `encode_memo_text` maps an empty string to the same
/// all-zero memo anyway, so both normalize to null here. Returns `Err(())`
/// (after throwing) on an interior NUL; a JNI read error is treated as null.
fn read_cstring_opt(
    env: &mut JNIEnv,
    s: &JString,
    field: &str,
) -> Result<Option<std::ffi::CString>, ()> {
    if s.is_null() {
        return Ok(None);
    }
    let owned: String = match env.get_string(s) {
        Ok(v) => v.into(),
        Err(_) => {
            let _ = env.exception_clear();
            return Ok(None);
        }
    };
    if owned.is_empty() {
        return Ok(None);
    }
    match std::ffi::CString::new(owned) {
        Ok(c) => Ok(Some(c)),
        Err(_) => {
            throw_sdk_exception(env, 1, &format!("{field} contained an interior NUL"));
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
        // Reject a negative account index at the boundary — it would
        // otherwise bit-cast to a huge u32 on the FFI call.
        if account < 0 {
            throw_sdk_exception(env, 1, "account must be non-negative");
            return ptr::null_mut();
        }
        let Some(wid) = read_id32(env, &wallet_id, "walletId") else {
            return ptr::null_mut();
        };
        let mut out_bytes = [0u8; 43];
        let mut present = false;
        let result = unsafe {
            platform_wallet_ffi::platform_wallet_manager_shielded_default_address(
                manager_handle as Handle,
                wid.as_ptr(),
                account as u32,
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
        // Reject sign errors at the boundary — negatives would otherwise
        // bit-cast to huge unsigned values (and a clamped 0 amount would
        // build a meaningless 0-duff asset lock).
        if amount_duffs <= 0 {
            throw_sdk_exception(env, 1, "amountDuffs must be positive");
            return;
        }
        if funding_account_index < 0 {
            throw_sdk_exception(env, 1, "fundingAccountIndex must be non-negative");
            return;
        }
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
                funding_account_index as u32,
                amount_duffs as u64,
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
        // Reject a negative vout at the boundary — it would otherwise
        // bit-cast to a huge u32 on the FFI call.
        if out_point_vout < 0 {
            throw_sdk_exception(env, 1, "outPointVout must be non-negative");
            return;
        }
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
            vout: out_point_vout as u32,
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
        let call = env.with_local_frame(8, |env| -> Result<(), jni::errors::Error> {
            env.call_method(
                bridge.as_obj(),
                "onProgress",
                "(JJJJ)V",
                &[
                    jni::objects::JValue::Long(batch_index as i64),
                    jni::objects::JValue::Long(batches_total_estimate as i64),
                    jni::objects::JValue::Long(pool_notes_now as i64),
                    jni::objects::JValue::Long(target as i64),
                ],
            )?;
            Ok(())
        });
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
        // Reject sign errors at the boundary — negatives would otherwise
        // bit-cast to huge unsigned values. A target of 0 is legal (the
        // Rust side treats an already-met target as a no-op success).
        if account < 0 {
            throw_sdk_exception(env, 1, "account must be non-negative");
            return;
        }
        if target_total_notes < 0 {
            throw_sdk_exception(env, 1, "targetTotalNotes must be non-negative");
            return;
        }
        if funding_account_index < 0 {
            throw_sdk_exception(env, 1, "fundingAccountIndex must be non-negative");
            return;
        }
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
                account as u32,
                target_total_notes as u64,
                funding_account_index as u32,
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

// ── Shielded outgoing spends (types 16/17/19) ─────────────────────────
//
// Manager-handle entry points like the funding submits above. Each spend
// signs with the bound shielded sub-wallet's own Orchard
// `SpendAuthorizingKey` (cached by `platform_wallet_manager_bind_shielded`),
// so — unlike the shield / fund paths — NO host signer or mnemonic-resolver
// handle crosses the boundary; the host only supplies recipient + amount.
// Each is a single blocking FFI call that runs the ~30s Halo 2 proof on a
// Rust worker thread and returns only a status code. The retry-semantics-
// bearing error codes (`ErrorShieldedSpendUnconfirmed` = do NOT retry,
// `ErrorShieldedNoRecordedAnchor` / `ErrorShieldedBroadcastFailed` =
// retryable) surface through `take_pwffi_error`'s offset codes and map to
// the dedicated `DashSdkError.PlatformWallet` types on the Kotlin side.

/// Shield from Platform balance (Type 15) — bridges
/// `platform_wallet_manager_shielded_shield`.
///
/// Mirrors Swift's `PlatformWalletManager.shieldedShield`
/// (`PlatformWalletManagerShieldedSync.swift`): spends `amount` credits
/// (1 DASH = 1e11) from the wallet's `payment_account` Platform-Payment
/// addresses (auto-selected in ascending derivation order) into the
/// wallet's own bound shielded pool (`shielded_account`). Unlike the
/// transfer/unshield/withdraw spends — which move notes already inside the
/// pool and need no host signer — the shield spends the *transparent*
/// Platform-address side, so it takes the Keystore address signer
/// (`signer_address_handle` = `mgr.signerHandle`, a `*const SignerHandle`
/// / `VTableSigner` callback variant), NOT a `MnemonicResolverHandle`. The
/// ~30s Halo 2 proof runs inside the call; nothing is returned on success.
/// Rust always shields to this wallet's own default Orchard address, so
/// there is no recipient parameter (self-shield only).
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_FundingNative_shieldedShield(
    mut env: JNIEnv,
    _class: JClass,
    manager_handle: jlong,
    wallet_id: JByteArray,
    shielded_account: jint,
    payment_account: jint,
    amount: jlong,
    signer_address_handle: jlong,
) {
    guard(&mut env, (), |env| {
        // Reject sign errors at the boundary — negatives would otherwise
        // bit-cast to huge unsigned values (never clamp).
        if amount <= 0 {
            throw_sdk_exception(env, 1, "amount must be positive");
            return;
        }
        if shielded_account < 0 {
            throw_sdk_exception(env, 1, "shieldedAccount must be non-negative");
            return;
        }
        if payment_account < 0 {
            throw_sdk_exception(env, 1, "paymentAccount must be non-negative");
            return;
        }
        if signer_address_handle == 0 {
            throw_sdk_exception(env, 1, "signerAddressHandle must be non-null");
            return;
        }
        let Some(wid) = read_id32(env, &wallet_id, "walletId") else {
            return;
        };
        let result = unsafe {
            platform_wallet_ffi::platform_wallet_manager_shielded_shield(
                manager_handle as Handle,
                wid.as_ptr(),
                shielded_account as u32,
                payment_account as u32,
                amount as u64,
                signer_address_handle as *const SignerHandle,
            )
        };
        let _ = take_pwffi_error(env, result);
    })
}

/// Create an identity funded from the shielded pool, Type 20 (bridges
/// `platform_wallet_manager_shielded_identity_create_from_pool`).
///
/// Mirrors Swift's `PlatformWalletManager.shieldedIdentityCreateFromPool`:
/// spends a note of the fixed exit `denomination` (credits — one of the
/// on-chain `shielded_identity_create_denominations`: 0.1 / 0.3 / 0.5 /
/// 1.0 DASH) from the wallet's bound Orchard pool (`account`) to fund a new
/// identity at `identity_index`. `pubkeys_blob` is the SAME flat
/// registration-key blob ID-08 uses (`IdentityKeyPreview.encodeForRegistration`),
/// decoded + role-stamped by keyId exactly like
/// [`Java_..._registerIdentityFromAddresses`]. `fallback_address` is the
/// REQUIRED 21-byte `PlatformAddress` (1 variant tag + 20 hash) that
/// receives the value (minus a penalty) if creation fails a stateful check
/// — it is bound into the transition sighash. `signer_handle` is the
/// Keystore identity signer (`mgr.signerHandle`). Blocks for the ~30s Halo 2
/// proof; returns the new 32-byte identity id (written on success AND on the
/// penalty-fallback path).
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_FundingNative_shieldedIdentityCreateFromPool(
    mut env: JNIEnv,
    _class: JClass,
    manager_handle: jlong,
    wallet_id: JByteArray,
    account: jint,
    identity_index: jint,
    pubkeys_blob: JByteArray,
    denomination: jlong,
    fallback_address: JByteArray,
    signer_handle: jlong,
) -> jni::sys::jbyteArray {
    guard(&mut env, ptr::null_mut(), |env| {
        if account < 0 {
            throw_sdk_exception(env, 1, "account must be non-negative");
            return ptr::null_mut();
        }
        if identity_index < 0 {
            throw_sdk_exception(env, 1, "identityIndex must be non-negative");
            return ptr::null_mut();
        }
        if denomination <= 0 {
            throw_sdk_exception(env, 1, "denomination must be positive");
            return ptr::null_mut();
        }
        if signer_handle == 0 {
            throw_sdk_exception(env, 1, "signerHandle must be non-null");
            return ptr::null_mut();
        }
        let Some(wid) = read_id32(env, &wallet_id, "walletId") else {
            return ptr::null_mut();
        };

        let Some(decoded) = decode_pubkeys_blob(env, &pubkeys_blob) else {
            return ptr::null_mut();
        };
        if decoded.is_empty() {
            throw_sdk_exception(env, 1, "pubkeysBlob contained no keys");
            return ptr::null_mut();
        }

        // The 21-byte fallback PlatformAddress (1 variant tag + 20 hash),
        // REQUIRED for Type-20 — validated exactly here.
        let fallback = match read_opt_bytes(env, &fallback_address) {
            Ok(Some(v)) => v,
            Ok(None) => {
                throw_sdk_exception(env, 1, "fallbackAddress must not be null");
                return ptr::null_mut();
            }
            Err(()) => return ptr::null_mut(),
        };
        if fallback.len() != 21 {
            throw_sdk_exception(
                env,
                1,
                &format!("fallbackAddress must be 21 bytes, got {}", fallback.len()),
            );
            return ptr::null_mut();
        }

        // Same positional keyId → DPP role assignment as ID-01 / ID-08.
        let ffi_rows: Vec<IdentityPubkeyFFI> = decoded
            .iter()
            .map(|(key_id, bytes)| {
                let (key_type, purpose, security_level) = role_for_registration_key_id(*key_id);
                IdentityPubkeyFFI {
                    key_id: *key_id,
                    key_type,
                    purpose,
                    security_level,
                    pubkey_bytes: bytes.as_ptr(),
                    pubkey_len: bytes.len(),
                    read_only: false,
                    contract_bounds_kind: 0,
                    contract_bounds_id: ptr::null(),
                    contract_bounds_document_type: ptr::null(),
                }
            })
            .collect();

        let mut out_id = [0u8; 32];
        let result = unsafe {
            platform_wallet_ffi::platform_wallet_manager_shielded_identity_create_from_pool(
                manager_handle as Handle,
                wid.as_ptr(),
                account as u32,
                identity_index as u32,
                ffi_rows.as_ptr(),
                ffi_rows.len(),
                denomination as u64,
                fallback.as_ptr(),
                signer_handle as *mut SignerHandle,
                &mut out_id as *mut [u8; 32],
            )
        };
        // `decoded` / `ffi_rows` / `fallback` own the pointed-to buffers
        // through the blocking FFI call above.
        //
        // ErrorShieldedBroadcastUnconfirmed (17) is NOT routed through
        // take_pwffi_error: the C ABI writes `out_id` on that outcome too —
        // the identity may already be live on-chain, so the host must
        // retain the id and hold its derivation slot instead of retrying
        // into a duplicate. Return a tagged 33-byte payload
        // (`[0|1] || identity_id`) so Kotlin can surface a typed
        // unconfirmed result instead of a generic error that loses the id.
        let unconfirmed = result.code
            == platform_wallet_ffi::error::PlatformWalletFFIResultCode::ErrorShieldedBroadcastUnconfirmed;
        let mut diagnostic = Vec::new();
        if unconfirmed {
            // Preserve the native diagnostic (the underlying DAPI /
            // result-proof confirmation failure) before freeing — the
            // registration controller surfaces it, and Swift keeps both
            // fields.
            let mut result = result;
            if !result.message.is_null() {
                diagnostic = unsafe { std::ffi::CStr::from_ptr(result.message) }
                    .to_bytes()
                    .to_vec();
            }
            unsafe { platform_wallet_ffi::error::platform_wallet_ffi_result_free(&mut result) };
        } else if take_pwffi_error(env, result) {
            return ptr::null_mut();
        }
        let mut packed = Vec::with_capacity(33 + diagnostic.len());
        packed.push(u8::from(unconfirmed));
        packed.extend_from_slice(&out_id);
        packed.extend_from_slice(&diagnostic);
        env.byte_array_from_slice(&packed)
            .map(|a| a.into_raw())
            .unwrap_or(ptr::null_mut())
    })
}

/// Shielded → shielded transfer (Type 16) — bridges
/// `platform_wallet_manager_shielded_transfer`.
///
/// Mirrors Swift's `PlatformWalletManager.shieldedTransfer` (
/// `PlatformWalletManagerShieldedSync.swift`): spends notes from `account`
/// on `walletId` and creates a new note for `recipientRaw43` (the
/// recipient's raw 43-byte Orchard payment address — same shape
/// [`Java_..._shieldedDefaultAddress`] returns). `amount` is in credits
/// (1 DASH = 1e11). `memoText` is an optional UTF-8 memo attached to the
/// recipient's note (null / empty = no memo); Rust validates the 32-byte
/// UTF-8 limit and does the 36-byte on-chain encoding.
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_FundingNative_shieldedTransfer(
    mut env: JNIEnv,
    _class: JClass,
    manager_handle: jlong,
    wallet_id: JByteArray,
    account: jint,
    recipient_raw43: JByteArray,
    amount: jlong,
    memo_text: JString,
) {
    guard(&mut env, (), |env| {
        // Reject sign errors at the boundary — negatives would otherwise
        // bit-cast to huge unsigned values (never clamp).
        if amount <= 0 {
            throw_sdk_exception(env, 1, "amount must be positive");
            return;
        }
        if account < 0 {
            throw_sdk_exception(env, 1, "account must be non-negative");
            return;
        }
        let Some(wid) = read_id32(env, &wallet_id, "walletId") else {
            return;
        };
        let Some(recipient) = read_recipient43(env, &recipient_raw43) else {
            return;
        };
        // null / empty memo → null pointer (no memo). The CString owns the
        // bytes through the blocking FFI call below.
        let memo = match read_cstring_opt(env, &memo_text, "memoText") {
            Ok(m) => m,
            Err(()) => return,
        };
        let memo_ptr = memo.as_ref().map_or(ptr::null(), |c| c.as_ptr());
        let result = unsafe {
            platform_wallet_ffi::platform_wallet_manager_shielded_transfer(
                manager_handle as Handle,
                wid.as_ptr(),
                account as u32,
                recipient.as_ptr(),
                amount as u64,
                memo_ptr,
            )
        };
        let _ = take_pwffi_error(env, result);
    })
}

/// Shielded → Platform unshield (Type 17) — bridges
/// `platform_wallet_manager_shielded_unshield`.
///
/// Mirrors Swift's `PlatformWalletManager.shieldedUnshield`
/// (`PlatformWalletManagerShieldedSync.swift`): spends notes from
/// `account` on `walletId` and credits `toPlatformAddress`, a bech32m
/// string (`"dash1…"` mainnet / `"tdash1…"` testnet). Rust parses it via
/// `PlatformAddress::from_bech32m_string` and network-checks it, so the
/// host never hand-rolls the storage variant tag. `amount` is in credits.
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_FundingNative_shieldedUnshield(
    mut env: JNIEnv,
    _class: JClass,
    manager_handle: jlong,
    wallet_id: JByteArray,
    account: jint,
    to_platform_address: JString,
    amount: jlong,
) {
    guard(&mut env, (), |env| {
        if amount <= 0 {
            throw_sdk_exception(env, 1, "amount must be positive");
            return;
        }
        if account < 0 {
            throw_sdk_exception(env, 1, "account must be non-negative");
            return;
        }
        let Some(wid) = read_id32(env, &wallet_id, "walletId") else {
            return;
        };
        let Some(to_addr) = read_cstring_required(env, &to_platform_address, "toPlatformAddress")
        else {
            return;
        };
        let result = unsafe {
            platform_wallet_ffi::platform_wallet_manager_shielded_unshield(
                manager_handle as Handle,
                wid.as_ptr(),
                account as u32,
                to_addr.as_ptr(),
                amount as u64,
            )
        };
        let _ = take_pwffi_error(env, result);
    })
}

/// Shielded → Core L1 withdrawal (Type 19) — bridges
/// `platform_wallet_manager_shielded_withdraw`.
///
/// Mirrors Swift's `PlatformWalletManager.shieldedWithdraw`
/// (`PlatformWalletManagerShieldedSync.swift`): spends notes from
/// `account` on `walletId` and creates an L1 withdrawal to `toCoreAddress`
/// (Base58Check string; Rust parses it and verifies the network).
/// `amount` is in credits — the network converts to L1 duffs at the
/// 1000:1 rate. `coreFeePerByte` is the L1 fee rate in duffs/byte
/// (`1` is the dashmate default).
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_FundingNative_shieldedWithdraw(
    mut env: JNIEnv,
    _class: JClass,
    manager_handle: jlong,
    wallet_id: JByteArray,
    account: jint,
    to_core_address: JString,
    amount: jlong,
    core_fee_per_byte: jint,
) {
    guard(&mut env, (), |env| {
        if amount <= 0 {
            throw_sdk_exception(env, 1, "amount must be positive");
            return;
        }
        if account < 0 {
            throw_sdk_exception(env, 1, "account must be non-negative");
            return;
        }
        // A zero fee rate would build an unrelayable L1 withdrawal and a
        // negative one would bit-cast huge — reject both at the boundary.
        if core_fee_per_byte <= 0 {
            throw_sdk_exception(env, 1, "coreFeePerByte must be positive");
            return;
        }
        let Some(wid) = read_id32(env, &wallet_id, "walletId") else {
            return;
        };
        let Some(to_addr) = read_cstring_required(env, &to_core_address, "toCoreAddress") else {
            return;
        };
        let result = unsafe {
            platform_wallet_ffi::platform_wallet_manager_shielded_withdraw(
                manager_handle as Handle,
                wid.as_ptr(),
                account as u32,
                to_addr.as_ptr(),
                amount as u64,
                core_fee_per_byte as u32,
            )
        };
        let _ = take_pwffi_error(env, result);
    })
}
