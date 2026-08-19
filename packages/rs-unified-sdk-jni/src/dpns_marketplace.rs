//! JNI bridge for the platform-wallet DPNS marketplace surface.
//!
//! All policy and transition construction remains in `platform-wallet`.
//! This module only validates JVM values, calls the C FFI entry points,
//! copies Rust-owned rows into compact JSON, and releases every allocation.

#![allow(clippy::missing_safety_doc)]

use crate::support::{guard, take_pwffi_error, throw_sdk_exception};
use jni::objects::{JByteArray, JClass, JString};
use jni::sys::{jboolean, jint, jlong, jlongArray, jstring, JNI_FALSE, JNI_TRUE};
use jni::JNIEnv;
use platform_wallet_ffi::dpns_marketplace::{
    DpnsMarketplaceNameFFI, DpnsMarketplaceSyncSummaryFFI, DpnsNameHistoryEventFFI,
    DpnsNameStateRowFFI,
};
use platform_wallet_ffi::handle::Handle;
use rs_sdk_ffi::SignerHandle;
use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::ptr;

fn read_id32(env: &mut JNIEnv, arr: &JByteArray, field: &str) -> Option<[u8; 32]> {
    if arr.is_null() {
        throw_sdk_exception(env, 1, &format!("{field} must not be null"));
        return None;
    }
    let len = env.get_array_length(arr).ok()? as usize;
    if len != 32 {
        throw_sdk_exception(env, 1, &format!("{field} must be 32 bytes, got {len}"));
        return None;
    }
    let mut bytes = [0i8; 32];
    env.get_byte_array_region(arr, 0, &mut bytes).ok()?;
    Some(bytes.map(|b| b as u8))
}

fn read_optional_id32(
    env: &mut JNIEnv,
    arr: &JByteArray,
    field: &str,
) -> Result<Option<[u8; 32]>, ()> {
    if arr.is_null() {
        return Ok(None);
    }
    read_id32(env, arr, field).map(Some).ok_or(())
}

fn read_cstring(env: &mut JNIEnv, value: &JString, field: &str) -> Option<CString> {
    if value.is_null() {
        throw_sdk_exception(env, 1, &format!("{field} must not be null"));
        return None;
    }
    let value: String = env.get_string(value).ok()?.into();
    match CString::new(value) {
        Ok(value) => Some(value),
        Err(_) => {
            throw_sdk_exception(env, 1, &format!("{field} must not contain NUL"));
            None
        }
    }
}

fn nonnegative_u64(env: &mut JNIEnv, value: jlong, field: &str) -> Option<u64> {
    if value < 0 {
        throw_sdk_exception(env, 1, &format!("{field} must be non-negative"));
        None
    } else {
        Some(value as u64)
    }
}

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

fn hex32(value: &[u8; 32]) -> String {
    value.iter().map(|b| format!("{b:02x}")).collect()
}

unsafe fn cstr(value: *const c_char) -> String {
    if value.is_null() {
        String::new()
    } else {
        CStr::from_ptr(value).to_string_lossy().into_owned()
    }
}

fn new_jstring(env: &mut JNIEnv, value: String) -> jstring {
    env.new_string(value)
        .map(|value| value.into_raw())
        .unwrap_or(ptr::null_mut())
}

fn name_json(row: &DpnsMarketplaceNameFFI) -> String {
    format!(
        "{{\"documentId\":\"{}\",\"ownerId\":\"{}\",\"recordsIdentityId\":{},\"label\":{},\"normalizedLabel\":{},\"priceCredits\":{},\"createdAtMs\":{},\"updatedAtMs\":{},\"transferredAtMs\":{}}}",
        hex32(&row.document_id),
        hex32(&row.owner_id),
        if row.has_records_identity {
            format!("\"{}\"", hex32(&row.records_identity_id))
        } else {
            "null".into()
        },
        json_string(&unsafe { cstr(row.label) }),
        json_string(&unsafe { cstr(row.normalized_label) }),
        if row.has_price {
            format!("\"{}\"", row.price)
        } else {
            "null".into()
        },
        row.created_at_ms,
        row.updated_at_ms,
        row.transferred_at_ms,
    )
}

fn state_json(row: &DpnsNameStateRowFFI) -> String {
    format!(
        "{{\"documentId\":\"{}\",\"walletIdentityId\":\"{}\",\"label\":{},\"normalizedLabel\":{},\"priceCredits\":{},\"status\":{},\"counterpartyId\":{},\"createdAtMs\":{},\"updatedAtMs\":{},\"transferredAtMs\":{},\"lastSyncedAtMs\":{}}}",
        hex32(&row.document_id),
        hex32(&row.wallet_identity_id),
        json_string(&unsafe { cstr(row.label) }),
        json_string(&unsafe { cstr(row.normalized_label) }),
        if row.has_price {
            format!("\"{}\"", row.price)
        } else {
            "null".into()
        },
        row.status,
        if row.has_counterparty {
            format!("\"{}\"", hex32(&row.counterparty_id))
        } else {
            "null".into()
        },
        row.created_at_ms,
        row.updated_at_ms,
        row.transferred_at_ms,
        row.last_synced_at_ms,
    )
}

fn history_json(row: &DpnsNameHistoryEventFFI) -> String {
    format!(
        "{{\"kind\":{},\"atMs\":{},\"blockHeight\":{},\"priceCredits\":{},\"fromId\":{},\"toId\":{}}}",
        row.kind,
        row.at_ms,
        if row.has_block_height {
            row.block_height.to_string()
        } else {
            "null".into()
        },
        if row.has_price {
            format!("\"{}\"", row.price)
        } else {
            "null".into()
        },
        if row.has_from {
            format!("\"{}\"", hex32(&row.from_id))
        } else {
            "null".into()
        },
        if row.has_to {
            format!("\"{}\"", hex32(&row.to_id))
        } else {
            "null".into()
        },
    )
}

#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_DpnsMarketplaceNative_search(
    mut env: JNIEnv,
    _class: JClass,
    wallet_handle: jlong,
    prefix: JString,
    limit: jint,
    start_after: JByteArray,
) -> jstring {
    guard(&mut env, ptr::null_mut(), |env| {
        if limit < 0 {
            throw_sdk_exception(env, 1, "limit must be non-negative");
            return ptr::null_mut();
        }
        let Some(prefix) = read_cstring(env, &prefix, "prefix") else {
            return ptr::null_mut();
        };
        let start_after = match read_optional_id32(env, &start_after, "startAfter") {
            Ok(value) => value,
            Err(()) => return ptr::null_mut(),
        };
        let mut rows: *mut DpnsMarketplaceNameFFI = ptr::null_mut();
        let mut count = 0usize;
        let result = unsafe {
            platform_wallet_ffi::platform_wallet_dpns_marketplace_search(
                wallet_handle as Handle,
                prefix.as_ptr(),
                limit as u32,
                start_after.as_ref().map_or(ptr::null(), |id| id.as_ptr()),
                &mut rows,
                &mut count,
            )
        };
        if take_pwffi_error(env, result) {
            return ptr::null_mut();
        }
        let json = if rows.is_null() || count == 0 {
            "[]".to_string()
        } else {
            let values = unsafe { std::slice::from_raw_parts(rows, count) };
            format!(
                "[{}]",
                values.iter().map(name_json).collect::<Vec<_>>().join(",")
            )
        };
        unsafe { platform_wallet_ffi::dpns_marketplace_names_free(rows, count) };
        new_jstring(env, json)
    })
}

#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_DpnsMarketplaceNative_nameState(
    mut env: JNIEnv,
    _class: JClass,
    wallet_handle: jlong,
    name: JString,
) -> jstring {
    guard(&mut env, ptr::null_mut(), |env| {
        let Some(name) = read_cstring(env, &name, "name") else {
            return ptr::null_mut();
        };
        let mut row: *mut DpnsMarketplaceNameFFI = ptr::null_mut();
        let result = unsafe {
            platform_wallet_ffi::platform_wallet_dpns_marketplace_name_state(
                wallet_handle as Handle,
                name.as_ptr(),
                &mut row,
            )
        };
        if take_pwffi_error(env, result) || row.is_null() {
            return ptr::null_mut();
        }
        let json = name_json(unsafe { &*row });
        unsafe { platform_wallet_ffi::dpns_marketplace_name_free(row) };
        new_jstring(env, json)
    })
}

#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_DpnsMarketplaceNative_myNames(
    mut env: JNIEnv,
    _class: JClass,
    wallet_handle: jlong,
    identity_id: JByteArray,
) -> jstring {
    guard(&mut env, ptr::null_mut(), |env| {
        let identity_id = match read_optional_id32(env, &identity_id, "identityId") {
            Ok(value) => value,
            Err(()) => return ptr::null_mut(),
        };
        let mut rows: *mut DpnsNameStateRowFFI = ptr::null_mut();
        let mut count = 0usize;
        let result = unsafe {
            platform_wallet_ffi::platform_wallet_dpns_marketplace_my_names(
                wallet_handle as Handle,
                identity_id.as_ref().map_or(ptr::null(), |id| id.as_ptr()),
                &mut rows,
                &mut count,
            )
        };
        if take_pwffi_error(env, result) {
            return ptr::null_mut();
        }
        let json = if rows.is_null() || count == 0 {
            "[]".to_string()
        } else {
            let values = unsafe { std::slice::from_raw_parts(rows, count) };
            format!(
                "[{}]",
                values.iter().map(state_json).collect::<Vec<_>>().join(",")
            )
        };
        unsafe { platform_wallet_ffi::dpns_name_state_rows_free(rows, count) };
        new_jstring(env, json)
    })
}

#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_DpnsMarketplaceNative_history(
    mut env: JNIEnv,
    _class: JClass,
    wallet_handle: jlong,
    name: JString,
) -> jstring {
    guard(&mut env, ptr::null_mut(), |env| {
        let Some(name) = read_cstring(env, &name, "name") else {
            return ptr::null_mut();
        };
        let mut rows: *mut DpnsNameHistoryEventFFI = ptr::null_mut();
        let mut count = 0usize;
        let result = unsafe {
            platform_wallet_ffi::platform_wallet_dpns_name_history(
                wallet_handle as Handle,
                name.as_ptr(),
                &mut rows,
                &mut count,
            )
        };
        if take_pwffi_error(env, result) {
            return ptr::null_mut();
        }
        let json = if rows.is_null() || count == 0 {
            "[]".to_string()
        } else {
            let values = unsafe { std::slice::from_raw_parts(rows, count) };
            format!(
                "[{}]",
                values
                    .iter()
                    .map(history_json)
                    .collect::<Vec<_>>()
                    .join(",")
            )
        };
        unsafe { platform_wallet_ffi::dpns_name_history_events_free(rows, count) };
        new_jstring(env, json)
    })
}

fn trade(
    env: &mut JNIEnv,
    wallet_handle: jlong,
    identity_id: &JByteArray,
    name: &JString,
    amount_or_recipient: TradeArgument<'_>,
    signer_handle: jlong,
) -> jstring {
    let Some(identity_id) = read_id32(env, identity_id, "identityId") else {
        return ptr::null_mut();
    };
    let Some(name) = read_cstring(env, name, "name") else {
        return ptr::null_mut();
    };
    if signer_handle == 0 {
        throw_sdk_exception(env, 1, "signerHandle must not be 0");
        return ptr::null_mut();
    }
    let mut out: *mut DpnsMarketplaceNameFFI = ptr::null_mut();
    let result = unsafe {
        match amount_or_recipient {
            TradeArgument::Price(price) => {
                platform_wallet_ffi::platform_wallet_dpns_set_name_price(
                    wallet_handle as Handle,
                    identity_id.as_ptr(),
                    name.as_ptr(),
                    price,
                    signer_handle as *mut SignerHandle,
                    &mut out,
                )
            }
            TradeArgument::Delist => platform_wallet_ffi::platform_wallet_dpns_delist_name(
                wallet_handle as Handle,
                identity_id.as_ptr(),
                name.as_ptr(),
                signer_handle as *mut SignerHandle,
                &mut out,
            ),
            TradeArgument::Transfer(recipient) => {
                platform_wallet_ffi::platform_wallet_dpns_transfer_name(
                    wallet_handle as Handle,
                    identity_id.as_ptr(),
                    name.as_ptr(),
                    recipient.as_ptr(),
                    signer_handle as *mut SignerHandle,
                    &mut out,
                )
            }
            TradeArgument::Purchase(price) => {
                platform_wallet_ffi::platform_wallet_dpns_purchase_name(
                    wallet_handle as Handle,
                    identity_id.as_ptr(),
                    name.as_ptr(),
                    price,
                    signer_handle as *mut SignerHandle,
                    &mut out,
                )
            }
        }
    };
    if take_pwffi_error(env, result) || out.is_null() {
        return ptr::null_mut();
    }
    let json = name_json(unsafe { &*out });
    unsafe { platform_wallet_ffi::dpns_marketplace_name_free(out) };
    new_jstring(env, json)
}

enum TradeArgument<'a> {
    Price(u64),
    Delist,
    Transfer(&'a [u8; 32]),
    Purchase(u64),
}

#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_DpnsMarketplaceNative_setPrice(
    mut env: JNIEnv,
    _class: JClass,
    wallet: jlong,
    identity: JByteArray,
    name: JString,
    price: jlong,
    signer: jlong,
) -> jstring {
    guard(&mut env, ptr::null_mut(), |env| {
        trade(
            env,
            wallet,
            &identity,
            &name,
            TradeArgument::Price(price as u64),
            signer,
        )
    })
}

#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_DpnsMarketplaceNative_delist(
    mut env: JNIEnv,
    _class: JClass,
    wallet: jlong,
    identity: JByteArray,
    name: JString,
    signer: jlong,
) -> jstring {
    guard(&mut env, ptr::null_mut(), |env| {
        trade(env, wallet, &identity, &name, TradeArgument::Delist, signer)
    })
}

#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_DpnsMarketplaceNative_transfer(
    mut env: JNIEnv,
    _class: JClass,
    wallet: jlong,
    identity: JByteArray,
    name: JString,
    recipient: JByteArray,
    signer: jlong,
) -> jstring {
    guard(&mut env, ptr::null_mut(), |env| {
        let Some(recipient) = read_id32(env, &recipient, "recipientId") else {
            return ptr::null_mut();
        };
        trade(
            env,
            wallet,
            &identity,
            &name,
            TradeArgument::Transfer(&recipient),
            signer,
        )
    })
}

#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_DpnsMarketplaceNative_purchase(
    mut env: JNIEnv,
    _class: JClass,
    wallet: jlong,
    identity: JByteArray,
    name: JString,
    price: jlong,
    signer: jlong,
) -> jstring {
    guard(&mut env, ptr::null_mut(), |env| {
        trade(
            env,
            wallet,
            &identity,
            &name,
            TradeArgument::Purchase(price as u64),
            signer,
        )
    })
}

#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_DpnsMarketplaceNative_sync(
    mut env: JNIEnv,
    _class: JClass,
    wallet: jlong,
) -> jstring {
    guard(&mut env, ptr::null_mut(), |env| {
        let mut summary = DpnsMarketplaceSyncSummaryFFI::default();
        let result = unsafe {
            platform_wallet_ffi::platform_wallet_dpns_marketplace_sync_detailed(
                wallet as Handle,
                &mut summary,
            )
        };
        if take_pwffi_error(env, result) {
            return ptr::null_mut();
        }
        let added = if summary.names_added.is_null() {
            &[][..]
        } else {
            unsafe { std::slice::from_raw_parts(summary.names_added, summary.names_added_count) }
        };
        let departed = if summary.names_departed.is_null() {
            &[][..]
        } else {
            unsafe {
                std::slice::from_raw_parts(summary.names_departed, summary.names_departed_count)
            }
        };
        let prices = if summary.prices_changed.is_null() {
            &[][..]
        } else {
            unsafe {
                std::slice::from_raw_parts(summary.prices_changed, summary.prices_changed_count)
            }
        };
        let added_json = added
            .iter()
            .map(|row| {
                format!(
                    "{{\"identityId\":\"{}\",\"label\":{}}}",
                    hex32(&row.identity_id),
                    json_string(&unsafe { cstr(row.label) })
                )
            })
            .collect::<Vec<_>>()
            .join(",");
        let departed_json = departed.iter().map(|row| format!(
            "{{\"identityId\":\"{}\",\"label\":{},\"documentId\":{},\"status\":{},\"counterpartyId\":{}}}",
            hex32(&row.identity_id), json_string(&unsafe { cstr(row.label) }),
            if row.has_document_id { format!("\"{}\"", hex32(&row.document_id)) } else { "null".into() },
            if row.has_status { row.status.to_string() } else { "null".into() },
            if row.has_status { format!("\"{}\"", hex32(&row.counterparty_id)) } else { "null".into() },
        )).collect::<Vec<_>>().join(",");
        let prices_json = prices
            .iter()
            .map(|row| {
                format!(
            "{{\"documentId\":\"{}\",\"label\":{},\"previousCredits\":{},\"currentCredits\":{}}}",
            hex32(&row.document_id), json_string(&unsafe { cstr(row.label) }),
            if row.has_previous { format!("\"{}\"", row.previous) } else { "null".into() },
            if row.has_current { format!("\"{}\"", row.current) } else { "null".into() },
        )
            })
            .collect::<Vec<_>>()
            .join(",");
        let json = format!(
            "{{\"tracked\":{},\"added\":[{}],\"departed\":[{}],\"pricesChanged\":[{}],\"syncUnixMs\":{}}}",
            summary.names_tracked, added_json, departed_json, prices_json, summary.sync_unix_ms,
        );
        unsafe { platform_wallet_ffi::dpns_marketplace_sync_summary_free(&mut summary) };
        new_jstring(env, json)
    })
}

#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_DpnsMarketplaceNative_syncStart(
    mut env: JNIEnv,
    _class: JClass,
    manager: jlong,
) -> jboolean {
    guard(&mut env, JNI_FALSE, |env| {
        let result = unsafe {
            platform_wallet_ffi::platform_wallet_manager_dpns_sync_start(manager as Handle)
        };
        if take_pwffi_error(env, result) {
            JNI_FALSE
        } else {
            JNI_TRUE
        }
    })
}

#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_DpnsMarketplaceNative_syncStop(
    mut env: JNIEnv,
    _class: JClass,
    manager: jlong,
) -> jboolean {
    guard(&mut env, JNI_FALSE, |env| {
        let result = unsafe {
            platform_wallet_ffi::platform_wallet_manager_dpns_sync_stop(manager as Handle)
        };
        if take_pwffi_error(env, result) {
            JNI_FALSE
        } else {
            JNI_TRUE
        }
    })
}

fn sync_bool(env: &mut JNIEnv, manager: jlong, syncing: bool) -> jboolean {
    let mut out = false;
    let result = unsafe {
        if syncing {
            platform_wallet_ffi::platform_wallet_manager_dpns_sync_is_syncing(
                manager as Handle,
                &mut out,
            )
        } else {
            platform_wallet_ffi::platform_wallet_manager_dpns_sync_is_running(
                manager as Handle,
                &mut out,
            )
        }
    };
    if take_pwffi_error(env, result) {
        JNI_FALSE
    } else if out {
        JNI_TRUE
    } else {
        JNI_FALSE
    }
}

#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_DpnsMarketplaceNative_syncIsRunning(
    mut env: JNIEnv,
    _class: JClass,
    manager: jlong,
) -> jboolean {
    guard(&mut env, JNI_FALSE, |env| sync_bool(env, manager, false))
}

#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_DpnsMarketplaceNative_syncIsSyncing(
    mut env: JNIEnv,
    _class: JClass,
    manager: jlong,
) -> jboolean {
    guard(&mut env, JNI_FALSE, |env| sync_bool(env, manager, true))
}

#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_DpnsMarketplaceNative_syncLastUnixSeconds(
    mut env: JNIEnv,
    _class: JClass,
    manager: jlong,
) -> jlong {
    guard(&mut env, 0, |env| {
        let mut out = 0u64;
        let result = unsafe {
            platform_wallet_ffi::platform_wallet_manager_dpns_sync_last_sync_unix_seconds(
                manager as Handle,
                &mut out,
            )
        };
        if take_pwffi_error(env, result) {
            0
        } else {
            out as jlong
        }
    })
}

#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_DpnsMarketplaceNative_syncSetInterval(
    mut env: JNIEnv,
    _class: JClass,
    manager: jlong,
    seconds: jlong,
) -> jboolean {
    guard(&mut env, JNI_FALSE, |env| {
        let Some(seconds) = nonnegative_u64(env, seconds, "seconds") else {
            return JNI_FALSE;
        };
        let result = unsafe {
            platform_wallet_ffi::platform_wallet_manager_dpns_sync_set_interval(
                manager as Handle,
                seconds,
            )
        };
        if take_pwffi_error(env, result) {
            JNI_FALSE
        } else {
            JNI_TRUE
        }
    })
}

#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_DpnsMarketplaceNative_syncNow(
    mut env: JNIEnv,
    _class: JClass,
    manager: jlong,
) -> jlongArray {
    guard(&mut env, ptr::null_mut(), |env| {
        let mut success = 0usize;
        let mut errors = 0usize;
        let mut unix = 0u64;
        let result = unsafe {
            platform_wallet_ffi::platform_wallet_manager_dpns_sync_sync_now(
                manager as Handle,
                &mut success,
                &mut errors,
                &mut unix,
            )
        };
        if take_pwffi_error(env, result) {
            return ptr::null_mut();
        }
        let Ok(out) = env.new_long_array(3) else {
            return ptr::null_mut();
        };
        if env
            .set_long_array_region(&out, 0, &[success as i64, errors as i64, unix as i64])
            .is_err()
        {
            return ptr::null_mut();
        }
        out.into_raw()
    })
}
