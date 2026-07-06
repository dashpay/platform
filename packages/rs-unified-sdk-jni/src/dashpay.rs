//! JNI bridge for the DashPay read surface added by the iOS DashPay
//! completion (upstream #3841): payment history, cached contact profiles,
//! per-identity sync state, wallet-scoped DPNS search and per-account
//! wallet balances.
//!
//! Kotlin counterpart: `org.dashfoundation.dashsdk.ffi.DashpayNative`,
//! driven by `org.dashfoundation.dashsdk.tokens.Dashpay` and
//! `org.dashfoundation.dashsdk.wallet.PlatformWalletManager`. The 17
//! pre-existing DashPay exports (send/accept/ignore/sync pipeline) live
//! in [`crate::tokens`]; new DashPay exports land here.
//!
//! ## Result convention
//!
//! Same as [`crate::tokens`]: every platform-wallet call returns
//! `PlatformWalletFFIResult` consumed Rust-side;
//! [`crate::support::take_pwffi_error`] maps failures to a thrown
//! `DashSDKException`. Read results surface as compact JSON strings
//! (the `getDashPayProfile` precedent — parsing happens Kotlin-side,
//! keeping descriptors trivial); byte ids are lower-hex.

use crate::support::{guard, take_pwffi_error};
use jni::objects::{JByteArray, JClass, JString};
use jni::sys::{jboolean, jint, jlong, jstring};
use jni::JNIEnv;
use platform_wallet_ffi::dashpay_profile::DashPayProfileFFI;
use platform_wallet_ffi::handle::Handle;
use rs_sdk_ffi::{MnemonicResolverHandle, SignerHandle};
use std::ffi::CStr;
use std::os::raw::c_char;
use std::ptr;

fn read_id32(env: &mut JNIEnv, arr: &JByteArray, field: &str) -> Option<[u8; 32]> {
    let len = env.get_array_length(arr).ok()? as usize;
    if len != 32 {
        let _ = env.throw_new(
            "org/dashfoundation/dashsdk/ffi/DashSDKException",
            format!("{field} must be 32 bytes, got {len}"),
        );
        return None;
    }
    let mut buf = [0i8; 32];
    env.get_byte_array_region(arr, 0, &mut buf).ok()?;
    Some(buf.map(|b| b as u8))
}

/// Render an optional FFI C-string as an owned Rust string.
///
/// # Safety
/// `ptr`, when non-null, must be a valid NUL-terminated C string.
unsafe fn opt_cstr(ptr: *const c_char) -> Option<String> {
    if ptr.is_null() {
        None
    } else {
        Some(CStr::from_ptr(ptr).to_string_lossy().into_owned())
    }
}

/// Minimal JSON string escaping for the fields emitted here (same rules
/// as `tokens::profile_to_json`).
fn json_string(value: &str) -> String {
    let mut out = String::with_capacity(value.len() + 2);
    out.push('"');
    for c in value.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

fn hex32(bytes: &[u8; 32]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

fn new_jstring(env: &mut JNIEnv, s: String) -> jstring {
    env.new_string(s)
        .map(|s| s.into_raw())
        .unwrap_or(ptr::null_mut())
}

// ── Payment history ───────────────────────────────────────────────────

/// Read the DashPay payment history off a managed-identity handle
/// (bridges `managed_identity_get_dashpay_payments`). Returns a JSON
/// array string — one object per payment: `txid`, `counterpartyId`
/// (lower-hex 32 bytes), `amountDuffs`, `direction` (0 Sent, 1 Received),
/// `status` (0 Pending, 1 Confirmed, 2 Failed), optional `memo`. The
/// Rust-owned array is freed here via `dashpay_payment_array_free`.
///
/// This getter is the ONLY durable source of payment rows: the Kotlin
/// refresh path (`PlatformWalletManager.refreshDashPayPayments`) upserts
/// its result into Room, mirroring iOS — the recurring DashPay sweep
/// reconciles payments in-memory without persisting them.
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_DashpayNative_managedIdentityDashPayPayments(
    mut env: JNIEnv,
    _class: JClass,
    identity_handle: jlong,
) -> jstring {
    guard(&mut env, ptr::null_mut(), |env| {
        let mut array = platform_wallet_ffi::DashpayPaymentArray {
            items: ptr::null_mut(),
            count: 0,
        };
        let result = unsafe {
            platform_wallet_ffi::managed_identity_get_dashpay_payments(
                identity_handle as Handle,
                &mut array as *mut platform_wallet_ffi::DashpayPaymentArray,
            )
        };
        if take_pwffi_error(env, result) {
            return ptr::null_mut();
        }
        let mut rows: Vec<String> = Vec::with_capacity(array.count);
        if !array.items.is_null() && array.count > 0 {
            let items = unsafe { std::slice::from_raw_parts(array.items, array.count) };
            for p in items {
                let mut fields: Vec<String> = Vec::with_capacity(6);
                if let Some(txid) = unsafe { opt_cstr(p.txid) } {
                    fields.push(format!("\"txid\":{}", json_string(&txid)));
                }
                fields.push(format!(
                    "\"counterpartyId\":{}",
                    json_string(&hex32(&p.counterparty_id))
                ));
                fields.push(format!("\"amountDuffs\":{}", p.amount_duffs));
                fields.push(format!("\"direction\":{}", p.direction as u8));
                fields.push(format!("\"status\":{}", p.status as u8));
                if let Some(memo) = unsafe { opt_cstr(p.memo) } {
                    fields.push(format!("\"memo\":{}", json_string(&memo)));
                }
                rows.push(format!("{{{}}}", fields.join(",")));
            }
        }
        unsafe {
            platform_wallet_ffi::dashpay_payment_array_free(
                &mut array as *mut platform_wallet_ffi::DashpayPaymentArray,
            )
        };
        new_jstring(env, format!("[{}]", rows.join(",")))
    })
}

// ── Profiles ──────────────────────────────────────────────────────────

/// Read the cached profile of `contactIdentityId` as seen by
/// `ownerIdentityId` (bridges `platform_wallet_get_contact_profile`).
/// Returns the same JSON object shape as `TokensNative.getDashPayProfile`
/// or null when no present profile is cached.
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_DashpayNative_getContactProfile(
    mut env: JNIEnv,
    _class: JClass,
    wallet_handle: jlong,
    owner_identity_id: JByteArray,
    contact_identity_id: JByteArray,
) -> jstring {
    guard(&mut env, ptr::null_mut(), |env| {
        let Some(owner) = read_id32(env, &owner_identity_id, "ownerIdentityId") else {
            return ptr::null_mut();
        };
        let Some(contact) = read_id32(env, &contact_identity_id, "contactIdentityId") else {
            return ptr::null_mut();
        };
        let mut profile = DashPayProfileFFI::empty();
        let mut has_profile = false;
        let result = unsafe {
            platform_wallet_ffi::platform_wallet_get_contact_profile(
                wallet_handle as Handle,
                owner.as_ptr(),
                contact.as_ptr(),
                &mut profile as *mut DashPayProfileFFI,
                &mut has_profile as *mut bool,
            )
        };
        if take_pwffi_error(env, result) {
            return ptr::null_mut();
        }
        if !has_profile {
            return ptr::null_mut();
        }
        let json = crate::tokens::profile_to_json(&profile);
        unsafe {
            platform_wallet_ffi::dashpay_profile_ffi_free(&mut profile as *mut DashPayProfileFFI)
        };
        new_jstring(env, json)
    })
}

/// Read the managed identity's own cached DashPay profile off its handle
/// (bridges `managed_identity_get_dashpay_profile`). Same JSON shape /
/// null convention as [`Java_org_dashfoundation_dashsdk_ffi_DashpayNative_getContactProfile`].
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_DashpayNative_managedIdentityDashPayProfile(
    mut env: JNIEnv,
    _class: JClass,
    identity_handle: jlong,
) -> jstring {
    guard(&mut env, ptr::null_mut(), |env| {
        let mut profile = DashPayProfileFFI::empty();
        let mut has_profile = false;
        let result = unsafe {
            platform_wallet_ffi::managed_identity_get_dashpay_profile(
                identity_handle as Handle,
                &mut profile as *mut DashPayProfileFFI,
                &mut has_profile as *mut bool,
            )
        };
        if take_pwffi_error(env, result) {
            return ptr::null_mut();
        }
        if !has_profile {
            return ptr::null_mut();
        }
        let json = crate::tokens::profile_to_json(&profile);
        unsafe {
            platform_wallet_ffi::dashpay_profile_ffi_free(&mut profile as *mut DashPayProfileFFI)
        };
        new_jstring(env, json)
    })
}

// ── Sync state ────────────────────────────────────────────────────────

/// Read the managed identity's DashPay sync state (bridges
/// `managed_identity_get_dashpay_sync_state`). Returns a JSON object of
/// the collection counts + high-water cursors; the optional cursors omit
/// their key when unset (mirroring the Swift `DashPaySyncState` optionals).
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_DashpayNative_managedIdentityDashPaySyncState(
    mut env: JNIEnv,
    _class: JClass,
    identity_handle: jlong,
) -> jstring {
    guard(&mut env, ptr::null_mut(), |env| {
        let mut state = platform_wallet_ffi::dashpay_profile::DashPaySyncStateFFI {
            established_contacts: 0,
            incoming_requests: 0,
            sent_requests: 0,
            ignored_senders: 0,
            contact_profiles: 0,
            present_contact_profiles: 0,
            dashpay_payments: 0,
            has_dashpay_profile: false,
            has_high_water_received: false,
            high_water_received_ms: 0,
            has_high_water_sent: false,
            high_water_sent_ms: 0,
        };
        let result = unsafe {
            platform_wallet_ffi::managed_identity_get_dashpay_sync_state(
                identity_handle as Handle,
                &mut state as *mut platform_wallet_ffi::dashpay_profile::DashPaySyncStateFFI,
            )
        };
        if take_pwffi_error(env, result) {
            return ptr::null_mut();
        }
        let mut fields = vec![
            format!("\"establishedContacts\":{}", state.established_contacts),
            format!("\"incomingRequests\":{}", state.incoming_requests),
            format!("\"sentRequests\":{}", state.sent_requests),
            format!("\"ignoredSenders\":{}", state.ignored_senders),
            format!("\"contactProfiles\":{}", state.contact_profiles),
            format!(
                "\"presentContactProfiles\":{}",
                state.present_contact_profiles
            ),
            format!("\"dashpayPayments\":{}", state.dashpay_payments),
            format!("\"hasDashpayProfile\":{}", state.has_dashpay_profile),
        ];
        if state.has_high_water_received {
            fields.push(format!(
                "\"highWaterReceivedMs\":{}",
                state.high_water_received_ms
            ));
        }
        if state.has_high_water_sent {
            fields.push(format!("\"highWaterSentMs\":{}", state.high_water_sent_ms));
        }
        new_jstring(env, format!("{{{}}}", fields.join(",")))
    })
}

// ── Wallet-scoped DPNS search ─────────────────────────────────────────

/// Live DPNS prefix search against Platform, wallet-scoped (bridges
/// `platform_wallet_search_dpns_names` — the call path the iOS
/// `AddContactView` drives; distinct from the SDK-handle-scoped
/// `QueriesNative.dpnsSearch`). Returns a JSON array of
/// `{"label":…,"identityId":…hex}` rows; `limit == 0` means no limit.
/// Blocking (network). The Rust-owned results are freed here via
/// `dpns_search_results_free`.
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_DashpayNative_searchDpnsNames(
    mut env: JNIEnv,
    _class: JClass,
    wallet_handle: jlong,
    prefix: JString,
    limit: jint,
) -> jstring {
    guard(&mut env, ptr::null_mut(), |env| {
        let prefix_str: String = match env.get_string(&prefix) {
            Ok(s) => s.into(),
            Err(_) => return ptr::null_mut(),
        };
        let Ok(prefix_c) = std::ffi::CString::new(prefix_str) else {
            return ptr::null_mut();
        };
        let mut results: *mut platform_wallet_ffi::DpnsSearchResultFFI = ptr::null_mut();
        let mut count: usize = 0;
        let result = unsafe {
            platform_wallet_ffi::platform_wallet_search_dpns_names(
                wallet_handle as Handle,
                prefix_c.as_ptr(),
                limit.max(0) as u32,
                &mut results as *mut *mut platform_wallet_ffi::DpnsSearchResultFFI,
                &mut count as *mut usize,
            )
        };
        if take_pwffi_error(env, result) {
            return ptr::null_mut();
        }
        let mut rows: Vec<String> = Vec::with_capacity(count);
        if !results.is_null() && count > 0 {
            let items = unsafe { std::slice::from_raw_parts(results, count) };
            for r in items {
                let label = unsafe { opt_cstr(r.label) }.unwrap_or_default();
                rows.push(format!(
                    "{{\"label\":{},\"identityId\":{}}}",
                    json_string(&label),
                    json_string(&hex32(&r.identity_id))
                ));
            }
        }
        unsafe { platform_wallet_ffi::dpns_search_results_free(results, count) };
        new_jstring(env, format!("[{}]", rows.join(",")))
    })
}

// ── Per-account wallet balances ───────────────────────────────────────

/// Read the per-account balance snapshot for a wallet off the manager
/// (bridges `platform_wallet_manager_get_account_balances` — drives the
/// iOS DashPay tab's account balance display). Returns a JSON array —
/// one object per account with the type tags, indices, identity ids
/// (lower-hex; all-zero when unset) and the four balance buckets. The
/// Rust-owned array is freed here.
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_DashpayNative_walletManagerAccountBalances(
    mut env: JNIEnv,
    _class: JClass,
    manager_handle: jlong,
    wallet_id: JByteArray,
) -> jstring {
    guard(&mut env, ptr::null_mut(), |env| {
        let Some(wid) = read_id32(env, &wallet_id, "walletId") else {
            return ptr::null_mut();
        };
        let mut entries: *const platform_wallet_ffi::AccountBalanceEntryFFI = ptr::null();
        let mut count: usize = 0;
        let result = unsafe {
            platform_wallet_ffi::platform_wallet_manager_get_account_balances(
                manager_handle as Handle,
                wid.as_ptr(),
                &mut entries as *mut *const platform_wallet_ffi::AccountBalanceEntryFFI,
                &mut count as *mut usize,
            )
        };
        if take_pwffi_error(env, result) {
            return ptr::null_mut();
        }
        let mut rows: Vec<String> = Vec::with_capacity(count);
        if !entries.is_null() && count > 0 {
            let items = unsafe { std::slice::from_raw_parts(entries, count) };
            for e in items {
                rows.push(format!(
                    "{{\"typeTag\":{},\"standardTag\":{},\"index\":{},\
                     \"registrationIndex\":{},\"keyClass\":{},\
                     \"userIdentityId\":{},\"friendIdentityId\":{},\
                     \"confirmed\":{},\"unconfirmed\":{},\"immature\":{},\
                     \"locked\":{},\"keysUsed\":{},\"keysTotal\":{}}}",
                    e.type_tag as u8,
                    e.standard_tag as u8,
                    e.index,
                    e.registration_index,
                    e.key_class,
                    json_string(&hex32(&e.user_identity_id)),
                    json_string(&hex32(&e.friend_identity_id)),
                    e.confirmed,
                    e.unconfirmed,
                    e.immature,
                    e.locked,
                    e.keys_used,
                    e.keys_total,
                ));
            }
        }
        unsafe {
            platform_wallet_ffi::platform_wallet_manager_free_account_balances(
                entries as *mut platform_wallet_ffi::AccountBalanceEntryFFI,
                count,
            )
        };
        new_jstring(env, format!("[{}]", rows.join(",")))
    })
}

// ── Recurring DashPay sync service (manager-scoped) ───────────────────
//
// Bridges the `platform_wallet_manager_dashpay_sync_*` family — the
// recurring background sweep (contact requests + profiles + reconcile)
// that `DashpaySyncService` owns on the Kotlin side, mirroring
// `PlatformWalletManagerDashPaySync.swift`.

/// Start the recurring DashPay sweep. Idempotent Rust-side.
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_DashpayNative_dashPaySyncStart(
    mut env: JNIEnv,
    _class: JClass,
    manager_handle: jlong,
) {
    guard(&mut env, (), |env| {
        let result = unsafe {
            platform_wallet_ffi::platform_wallet_manager_dashpay_sync_start(
                manager_handle as Handle,
            )
        };
        let _ = take_pwffi_error(env, result);
    })
}

/// Stop the recurring DashPay sweep. Leaves it restartable.
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_DashpayNative_dashPaySyncStop(
    mut env: JNIEnv,
    _class: JClass,
    manager_handle: jlong,
) {
    guard(&mut env, (), |env| {
        let result = unsafe {
            platform_wallet_ffi::platform_wallet_manager_dashpay_sync_stop(manager_handle as Handle)
        };
        let _ = take_pwffi_error(env, result);
    })
}

/// Whether the recurring sweep loop is running.
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_DashpayNative_dashPaySyncIsRunning(
    mut env: JNIEnv,
    _class: JClass,
    manager_handle: jlong,
) -> jboolean {
    guard(&mut env, 0, |env| {
        let mut running = false;
        let result = unsafe {
            platform_wallet_ffi::platform_wallet_manager_dashpay_sync_is_running(
                manager_handle as Handle,
                &mut running as *mut bool,
            )
        };
        if take_pwffi_error(env, result) {
            return 0;
        }
        running as jboolean
    })
}

/// Whether a sweep pass is executing right now (the 1 Hz poll target).
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_DashpayNative_dashPaySyncIsSyncing(
    mut env: JNIEnv,
    _class: JClass,
    manager_handle: jlong,
) -> jboolean {
    guard(&mut env, 0, |env| {
        let mut syncing = false;
        let result = unsafe {
            platform_wallet_ffi::platform_wallet_manager_dashpay_sync_is_syncing(
                manager_handle as Handle,
                &mut syncing as *mut bool,
            )
        };
        if take_pwffi_error(env, result) {
            return 0;
        }
        syncing as jboolean
    })
}

/// Unix seconds of the last completed sweep; 0 when never.
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_DashpayNative_dashPaySyncLastSyncUnixSeconds(
    mut env: JNIEnv,
    _class: JClass,
    manager_handle: jlong,
) -> jlong {
    guard(&mut env, 0, |env| {
        let mut last: u64 = 0;
        let result = unsafe {
            platform_wallet_ffi::platform_wallet_manager_dashpay_sync_last_sync_unix_seconds(
                manager_handle as Handle,
                &mut last as *mut u64,
            )
        };
        if take_pwffi_error(env, result) {
            return 0;
        }
        last as jlong
    })
}

/// Set the sweep interval in seconds (takes effect from the next tick).
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_DashpayNative_dashPaySyncSetInterval(
    mut env: JNIEnv,
    _class: JClass,
    manager_handle: jlong,
    interval_seconds: jlong,
) {
    guard(&mut env, (), |env| {
        let result = unsafe {
            platform_wallet_ffi::platform_wallet_manager_dashpay_sync_set_interval(
                manager_handle as Handle,
                interval_seconds.max(0) as u64,
            )
        };
        let _ = take_pwffi_error(env, result);
    })
}

/// Run one sweep pass NOW (pull-to-refresh), blocking until it
/// completes. Returns a JSON object
/// `{"success":…,"errors":…,"syncUnixSeconds":…}` mirroring the Swift
/// `DashPaySyncSummary`.
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_DashpayNative_dashPaySyncNow(
    mut env: JNIEnv,
    _class: JClass,
    manager_handle: jlong,
) -> jstring {
    guard(&mut env, ptr::null_mut(), |env| {
        let mut success: usize = 0;
        let mut errors: usize = 0;
        let mut sync_unix: u64 = 0;
        let result = unsafe {
            platform_wallet_ffi::platform_wallet_manager_dashpay_sync_sync_now(
                manager_handle as Handle,
                &mut success as *mut usize,
                &mut errors as *mut usize,
                &mut sync_unix as *mut u64,
            )
        };
        if take_pwffi_error(env, result) {
            return ptr::null_mut();
        }
        new_jstring(
            env,
            format!(
                "{{\"success\":{},\"errors\":{},\"syncUnixSeconds\":{}}}",
                success, errors, sync_unix
            ),
        )
    })
}

// ── Seedless unlock (verify seed binding + deferred-crypto drain) ─────

/// Verify that the Keystore-resolved mnemonic reproduces this wallet's
/// key material (bridges `platform_wallet_verify_seed_binds_to_wallet`).
/// A stored-but-foreign seed surfaces as `ErrorInvalidParameter` — the
/// Kotlin caller disambiguates the seed-mismatch case ONLY by scoping
/// its catch to this call (the Swift contract).
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_DashpayNative_verifySeedBindsToWallet(
    mut env: JNIEnv,
    _class: JClass,
    wallet_handle: jlong,
    core_signer_handle: jlong,
) {
    guard(&mut env, (), |env| {
        let result = unsafe {
            platform_wallet_ffi::platform_wallet_verify_seed_binds_to_wallet(
                wallet_handle as Handle,
                core_signer_handle as *mut MnemonicResolverHandle,
            )
        };
        let _ = take_pwffi_error(env, result);
    })
}

/// Number of deferred contact-crypto entries queued on the wallet
/// (in-memory queue — rebuilt by the sweep, cleared by the drain).
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_DashpayNative_pendingContactCryptoCount(
    mut env: JNIEnv,
    _class: JClass,
    wallet_handle: jlong,
) -> jint {
    guard(&mut env, 0, |env| {
        let mut count: u32 = 0;
        let result = unsafe {
            platform_wallet_ffi::platform_wallet_pending_contact_crypto_count(
                wallet_handle as Handle,
                &mut count as *mut u32,
            )
        };
        if take_pwffi_error(env, result) {
            return 0;
        }
        count as jint
    })
}

/// Drain the deferred contact-crypto queue: per entry, run the seed-side
/// op (receiving-xpub register / external-account build / contactInfo
/// decrypt / auto-accept) through [`core_signer_handle`], signing any
/// reciprocal transitions with [`signer_handle`] (nullable — pass 0 for
/// resolver-only drains). Returns the drained-entry count. Blocking and
/// potentially slow (network + ECDH per entry) — never call on the main
/// thread; the caller keeps both bridge objects strongly reachable for
/// the whole call.
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_DashpayNative_drainPendingContactCrypto(
    mut env: JNIEnv,
    _class: JClass,
    wallet_handle: jlong,
    signer_handle: jlong,
    core_signer_handle: jlong,
) -> jint {
    guard(&mut env, 0, |env| {
        let mut drained: u32 = 0;
        let result = unsafe {
            platform_wallet_ffi::platform_wallet_drain_pending_contact_crypto(
                wallet_handle as Handle,
                signer_handle as *mut SignerHandle,
                core_signer_handle as *mut MnemonicResolverHandle,
                &mut drained as *mut u32,
            )
        };
        if take_pwffi_error(env, result) {
            return 0;
        }
        drained as jint
    })
}

// ── Profile / contactInfo writes ──────────────────────────────────────

/// Create (`doCreate == true`) or update the DashPay profile for
/// `identityId`, signing with `signer_handle`. `avatarBytes` is the raw
/// image — Rust computes the SHA-256 hash + perceptual fingerprint.
/// Broadcasts a real document state transition (blocking, network).
/// Returns the resulting profile as JSON (same shape as the readers).
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_DashpayNative_createOrUpdateProfile(
    mut env: JNIEnv,
    _class: JClass,
    wallet_handle: jlong,
    identity_id: JByteArray,
    display_name: JString,
    public_message: JString,
    avatar_url: JString,
    avatar_bytes: JByteArray,
    do_create: jboolean,
    signer_handle: jlong,
) -> jstring {
    guard(&mut env, ptr::null_mut(), |env| {
        let Some(id) = read_id32(env, &identity_id, "identityId") else {
            return ptr::null_mut();
        };
        let display = opt_jstring_to_cstring(env, &display_name);
        let message = opt_jstring_to_cstring(env, &public_message);
        let url = opt_jstring_to_cstring(env, &avatar_url);
        let avatar: Option<Vec<u8>> = if avatar_bytes.is_null() {
            None
        } else {
            env.convert_byte_array(&avatar_bytes).ok()
        };

        let mut profile = DashPayProfileFFI::empty();
        let result = unsafe {
            platform_wallet_ffi::platform_wallet_create_or_update_dashpay_profile_with_signer(
                wallet_handle as Handle,
                id.as_ptr(),
                display.as_ref().map_or(ptr::null(), |c| c.as_ptr()),
                message.as_ref().map_or(ptr::null(), |c| c.as_ptr()),
                url.as_ref().map_or(ptr::null(), |c| c.as_ptr()),
                avatar.as_ref().map_or(ptr::null(), |v| v.as_ptr()),
                avatar.as_ref().map_or(0, |v| v.len()),
                do_create != 0,
                signer_handle as *mut SignerHandle,
                &mut profile as *mut DashPayProfileFFI,
            )
        };
        if take_pwffi_error(env, result) {
            // The FFI zero-initializes out_profile before fallible work,
            // so nothing was allocated on the error path.
            return ptr::null_mut();
        }
        let json = crate::tokens::profile_to_json(&profile);
        unsafe {
            platform_wallet_ffi::dashpay_profile_ffi_free(&mut profile as *mut DashPayProfileFFI)
        };
        new_jstring(env, json)
    })
}

/// Set the owner-private contactInfo (alias / note / displayHidden) for
/// `(identityId, contactId)`. Local state always updates; the encrypted
/// on-chain publish is gated by DIP-15 (needs ≥ 2 established contacts)
/// and by the wallet's signing capability. Returns the
/// `CONTACT_INFO_*` outcome discriminant: 0 published, 1 deferred until
/// two contacts, 2 skipped (watch-only). Blocking when it publishes.
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_DashpayNative_setContactInfo(
    mut env: JNIEnv,
    _class: JClass,
    wallet_handle: jlong,
    identity_id: JByteArray,
    contact_id: JByteArray,
    alias: JString,
    note: JString,
    display_hidden: jboolean,
    signer_handle: jlong,
    core_signer_handle: jlong,
) -> jint {
    guard(&mut env, -1, |env| {
        let Some(id) = read_id32(env, &identity_id, "identityId") else {
            return -1;
        };
        let Some(contact) = read_id32(env, &contact_id, "contactId") else {
            return -1;
        };
        let alias_c = opt_jstring_to_cstring(env, &alias);
        let note_c = opt_jstring_to_cstring(env, &note);

        let mut outcome: u8 = 0;
        let result = unsafe {
            platform_wallet_ffi::contact_info::platform_wallet_set_dashpay_contact_info_with_signer(
                wallet_handle as Handle,
                id.as_ptr(),
                contact.as_ptr(),
                alias_c.as_ref().map_or(ptr::null(), |c| c.as_ptr()),
                note_c.as_ref().map_or(ptr::null(), |c| c.as_ptr()),
                display_hidden != 0,
                signer_handle as *mut SignerHandle,
                core_signer_handle as *mut MnemonicResolverHandle,
                &mut outcome as *mut u8,
            )
        };
        if take_pwffi_error(env, result) {
            return -1;
        }
        outcome as jint
    })
}

// ── Signer capability preflight ───────────────────────────────────────

/// Whether the mnemonic resolver can derive-sign identity keys of
/// `keyType` (bridges `dash_sdk_resolver_supports_key_type`). Preflight
/// only — keeps `canSignWith` consistent with the sign path's
/// `UNSUPPORTED_KEY_TYPE` rejection by reading the one Rust source of
/// truth (Swift `KeychainSigner.resolverCanDeriveSign`).
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_DashpayNative_resolverSupportsKeyType(
    mut env: JNIEnv,
    _class: JClass,
    key_type: jint,
) -> jboolean {
    guard(&mut env, 0, |_| {
        let Ok(kt) = u8::try_from(key_type) else {
            return 0;
        };
        let supported = unsafe { platform_wallet_ffi::dash_sdk_resolver_supports_key_type(kt) };
        supported as jboolean
    })
}

/// Convert a nullable JString into an owned CString; None on null /
/// conversion failure (treated as "field absent").
fn opt_jstring_to_cstring(env: &mut JNIEnv, s: &JString) -> Option<std::ffi::CString> {
    if s.is_null() {
        return None;
    }
    let value: String = env.get_string(s).ok()?.into();
    std::ffi::CString::new(value).ok()
}
