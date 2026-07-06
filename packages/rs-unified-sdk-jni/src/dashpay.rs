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
use jni::sys::{jint, jlong, jstring};
use jni::JNIEnv;
use platform_wallet_ffi::dashpay_profile::DashPayProfileFFI;
use platform_wallet_ffi::handle::Handle;
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
