//! JNI bridge for the platform-wallet `PlatformWalletManager` lifecycle
//! and per-wallet accessors.
//!
//! Kotlin counterpart: `org.dashfoundation.dashsdk.ffi.WalletManagerNative`
//! + `NativeWalletEventBridge`, driven by
//! `org.dashfoundation.dashsdk.wallet.PlatformWalletManager`.
//!
//! ## What the manager takes at construction
//!
//! `platform_wallet_manager_create(sdk_ptr, persistence, event_handler)`.
//! The mnemonic resolver and signer are **SDK-level, per-call** handles
//! (see `mnemonic.rs` / `signer.rs`) — they are NOT wired into the
//! manager, matching `PlatformWalletManager.swift`, which holds only a
//! persistence handler + event handler.
//!
//! ## Context ownership (the subtle part)
//!
//! `platform_wallet_manager_create` consumes both vtables by value via
//! `std::ptr::read`, copying the `context` pointer into the manager's
//! `FFIPersister` / `FFIEventHandler`. Neither has a `Drop`, so Rust
//! never frees the context — exactly the `passUnretained` model the Swift
//! SDK uses (the host owns the callback object's lifetime).
//!
//! We therefore box each context (persistence bridge + event bridge as
//! JNI `GlobalRef`s) and keep the two box pointers alongside the manager
//! handle in a [`ManagerBundle`]. [`Java_..._nativeDestroy`] runs
//! `platform_wallet_manager_destroy` first — which calls `shutdown()` to
//! quiesce every callback-firing task — and only then drops the context
//! boxes, so no task can fire against a freed `GlobalRef`.
//!
//! ## Result convention
//!
//! platform-wallet-ffi returns [`PlatformWalletFFIResult`] (its own error
//! enum), not `rs-sdk-ffi`'s `DashSDKResult`. The shared
//! [`crate::support::take_pwffi_error`] maps a non-`Success` code to a
//! thrown `DashSDKException` (namespaced by
//! [`crate::support::PWFFI_CODE_OFFSET`]) and frees the result's message,
//! mirroring `results::take_error`.

#![allow(clippy::missing_safety_doc)]

use crate::events::{build_event_vtable, KotlinEventCtx};
use crate::persistence::{build_vtable, KotlinPersistenceCtx};
use crate::support::{guard, take_pwffi_error, throw_sdk_exception};
use jni::objects::{JByteArray, JClass, JObject, JObjectArray, JString};
use jni::sys::{jboolean, jbyteArray, jdoubleArray, jlong, jlongArray, JNI_FALSE, JNI_TRUE};
use jni::JNIEnv;
use platform_wallet_ffi::error::platform_wallet_ffi_result_free;
use platform_wallet_ffi::event_handler::EventHandlerCallbacks;
use platform_wallet_ffi::handle::Handle;
use platform_wallet_ffi::persistence::PersistenceCallbacks;
use std::ffi::c_void;
use std::ptr;

use rs_sdk_ffi::{dash_sdk_get_inner_sdk_ptr, SDKHandle};

// ── Manager bundle ────────────────────────────────────────────────────

/// Owns the native manager handle plus the two context boxes whose
/// `GlobalRef`s back the persistence + event vtables the manager copied.
/// Boxed and returned to Kotlin as a single `jlong`; freed by
/// [`Java_..._nativeDestroy`].
struct ManagerBundle {
    manager_handle: Handle,
    persistence_ctx: *mut KotlinPersistenceCtx,
    event_ctx: *mut KotlinEventCtx,
}

// ── Exports: lifecycle ────────────────────────────────────────────────

/// Create a `PlatformWalletManager`.
///
/// `sdk_handle` is the `SDKHandle` jlong from `SdkNative.createTrusted`;
/// we resolve its inner `Sdk` pointer via `dash_sdk_get_inner_sdk_ptr`.
/// `persistence_bridge` / `event_bridge` are the Kotlin
/// `NativePersistenceBridge` / `NativeWalletEventBridge` objects, held as
/// `GlobalRef`s for the manager's lifetime.
///
/// Returns a boxed [`ManagerBundle`] pointer as jlong (0 on failure,
/// after throwing).
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_WalletManagerNative_nativeCreate(
    mut env: JNIEnv,
    _class: JClass,
    sdk_handle: jlong,
    persistence_bridge: JObject,
    event_bridge: JObject,
) -> jlong {
    guard(&mut env, 0, |env| {
        if sdk_handle == 0 {
            throw_sdk_exception(env, 1, "SDK handle is 0");
            return 0;
        }
        // Resolve the inner Sdk pointer the manager stores.
        let inner = unsafe { dash_sdk_get_inner_sdk_ptr(sdk_handle as *const SDKHandle) };
        if inner.is_null() {
            throw_sdk_exception(env, 1, "dash_sdk_get_inner_sdk_ptr returned NULL");
            return 0;
        }

        // Box the persistence context (GlobalRef → boxed KotlinPersistenceCtx).
        let persistence_global = match env.new_global_ref(&persistence_bridge) {
            Ok(g) => g,
            Err(_) => {
                throw_sdk_exception(env, 99, "NewGlobalRef(persistence bridge) failed");
                return 0;
            }
        };
        let persistence_ctx =
            Box::into_raw(Box::new(KotlinPersistenceCtx::new(persistence_global)));
        let persistence: PersistenceCallbacks = build_vtable(persistence_ctx as *mut c_void);

        // Box the event context.
        let event_global = match env.new_global_ref(&event_bridge) {
            Ok(g) => g,
            Err(_) => {
                // Reclaim the persistence box we just leaked.
                unsafe { drop(Box::from_raw(persistence_ctx)) };
                throw_sdk_exception(env, 99, "NewGlobalRef(event bridge) failed");
                return 0;
            }
        };
        let event_ctx = Box::into_raw(Box::new(KotlinEventCtx::new(event_global)));
        let mut event_callbacks: EventHandlerCallbacks =
            build_event_vtable(event_ctx as *mut c_void);

        let mut manager_handle: Handle = 0;
        // SAFETY: `inner` is a live Sdk pointer for the duration of this
        // call; the manager clones the Sdk and reads both vtables by value.
        let result = unsafe {
            platform_wallet_ffi::platform_wallet_manager_create(
                inner,
                &persistence as *const PersistenceCallbacks,
                &mut event_callbacks as *const EventHandlerCallbacks,
                &mut manager_handle as *mut Handle,
            )
        };
        if take_pwffi_error(env, result) {
            // Manager was not created — drop both context boxes.
            unsafe {
                drop(Box::from_raw(persistence_ctx));
                drop(Box::from_raw(event_ctx));
            }
            return 0;
        }

        let bundle = Box::new(ManagerBundle {
            manager_handle,
            persistence_ctx,
            event_ctx,
        });
        Box::into_raw(bundle) as jlong
    })
}

/// The raw manager `Handle` for a bundle, as jlong (for the FFI sync /
/// wallet-accessor calls). 0 if the bundle pointer is 0.
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_WalletManagerNative_nativeManagerHandle(
    mut env: JNIEnv,
    _class: JClass,
    bundle: jlong,
) -> jlong {
    guard(&mut env, 0, |_| {
        if bundle == 0 {
            return 0;
        }
        // SAFETY: bundle is a live ManagerBundle pointer from nativeCreate.
        let b = unsafe { &*(bundle as *const ManagerBundle) };
        b.manager_handle as jlong
    })
}

/// Destroy a manager bundle: shut down the native manager (quiesces every
/// callback-firing task), then drop the persistence + event context
/// boxes. Safe on 0. Idempotent is the caller's responsibility (Kotlin's
/// `AtomicLong` handle guard calls this exactly once).
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_WalletManagerNative_nativeDestroy(
    mut env: JNIEnv,
    _class: JClass,
    bundle: jlong,
) {
    guard(&mut env, (), |_| {
        if bundle == 0 {
            return;
        }
        // SAFETY: bundle is a live ManagerBundle pointer from nativeCreate,
        // consumed exactly once here.
        let b = unsafe { Box::from_raw(bundle as *mut ManagerBundle) };
        // shutdown() runs to completion before returning — no task may
        // fire a callback after this.
        let result =
            unsafe { platform_wallet_ffi::platform_wallet_manager_destroy(b.manager_handle) };
        // destroy is documented to always return ok; free the message if any.
        let mut result = result;
        unsafe { platform_wallet_ffi_result_free(&mut result) };
        // Now safe to drop the context boxes (their GlobalRefs release the
        // Kotlin bridges).
        unsafe {
            if !b.persistence_ctx.is_null() {
                drop(Box::from_raw(b.persistence_ctx));
            }
            if !b.event_ctx.is_null() {
                drop(Box::from_raw(b.event_ctx));
            }
        }
    })
}

// ── Exports: wallet creation / restore ────────────────────────────────

/// Create a wallet from a BIP39 mnemonic. Returns the 32-byte wallet id
/// as a `byte[]`; the created `PlatformWallet` handle is written into
/// `out_wallet_handle[0]`. `createDefaultAccounts` maps to account
/// options 1 (Default) / 0 (None).
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_WalletManagerNative_createWalletFromMnemonic(
    mut env: JNIEnv,
    _class: JClass,
    manager_handle: jlong,
    mnemonic: JString,
    network: jni::sys::jint,
    create_default_accounts: jboolean,
    out_wallet_handle: jni::objects::JLongArray,
) -> jbyteArray {
    guard(&mut env, ptr::null_mut(), |env| {
        let phrase: String = match env.get_string(&mnemonic) {
            Ok(s) => s.into(),
            Err(_) => {
                let _ = env.exception_clear();
                throw_sdk_exception(env, 1, "mnemonic string was null/invalid");
                return ptr::null_mut();
            }
        };
        let c_phrase = match std::ffi::CString::new(phrase) {
            Ok(c) => c,
            Err(_) => {
                throw_sdk_exception(env, 1, "mnemonic contained an interior NUL");
                return ptr::null_mut();
            }
        };
        let account_options: u32 = if create_default_accounts == JNI_TRUE {
            1
        } else {
            0
        };
        let mut wallet_handle: Handle = 0;
        let mut wallet_id = [0u8; 32];
        let result = unsafe {
            platform_wallet_ffi::platform_wallet_manager_create_wallet_from_mnemonic(
                manager_handle as Handle,
                c_phrase.as_ptr(),
                ffi_network(network),
                account_options,
                &mut wallet_handle as *mut Handle,
                &mut wallet_id as *mut [u8; 32],
            )
        };
        if take_pwffi_error(env, result) {
            return ptr::null_mut();
        }
        // Publish the wallet handle out-param, then the id return value.
        if !out_wallet_handle.is_null() {
            let one = [wallet_handle as jlong];
            let _ = env.set_long_array_region(&out_wallet_handle, 0, &one);
        }
        env.byte_array_from_slice(&wallet_id)
            .map(|a| a.into_raw())
            .unwrap_or(ptr::null_mut())
    })
}

/// Rehydrate the manager from its persister. Fires the Kotlin
/// `onLoadWalletList` callback; reconstructs each persisted wallet as
/// watch-only. Does not produce wallet handles — the caller follows up
/// with [`getWallet`] per known id.
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_WalletManagerNative_loadFromPersistor(
    mut env: JNIEnv,
    _class: JClass,
    manager_handle: jlong,
) {
    guard(&mut env, (), |env| {
        let result = unsafe {
            platform_wallet_ffi::platform_wallet_manager_load_from_persistor(
                manager_handle as Handle,
            )
        };
        let _ = take_pwffi_error(env, result);
    })
}

/// Get a `PlatformWallet` handle (jlong) for a wallet registered in the
/// manager. Throws `NotFound` if the wallet is not held. `wallet_id` must
/// be 32 bytes.
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_WalletManagerNative_getWallet(
    mut env: JNIEnv,
    _class: JClass,
    manager_handle: jlong,
    wallet_id: JByteArray,
) -> jlong {
    guard(&mut env, 0, |env| {
        let Some(id) = read_id32(env, &wallet_id) else {
            return 0;
        };
        let mut wallet_handle: Handle = 0;
        let result = unsafe {
            platform_wallet_ffi::platform_wallet_manager_get_wallet(
                manager_handle as Handle,
                &id as *const [u8; 32],
                &mut wallet_handle as *mut Handle,
            )
        };
        if take_pwffi_error(env, result) {
            return 0;
        }
        wallet_handle as jlong
    })
}

/// Remove one wallet from the manager. Idempotent on missing wallets.
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_WalletManagerNative_removeWallet(
    mut env: JNIEnv,
    _class: JClass,
    manager_handle: jlong,
    wallet_id: JByteArray,
) {
    guard(&mut env, (), |env| {
        let Some(id) = read_id32(env, &wallet_id) else {
            return;
        };
        let result = unsafe {
            platform_wallet_ffi::platform_wallet_manager_remove_wallet(
                manager_handle as Handle,
                &id as *const [u8; 32],
            )
        };
        let _ = take_pwffi_error(env, result);
    })
}

// ── Exports: per-wallet accessors ─────────────────────────────────────

/// The 32-byte wallet id of a `PlatformWallet` handle, as `byte[]`.
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_WalletManagerNative_walletGetId(
    mut env: JNIEnv,
    _class: JClass,
    wallet_handle: jlong,
) -> jbyteArray {
    guard(&mut env, ptr::null_mut(), |env| {
        let mut id = [0u8; 32];
        let result = unsafe {
            platform_wallet_ffi::platform_wallet_get_id(
                wallet_handle as Handle,
                &mut id as *mut [u8; 32],
            )
        };
        if take_pwffi_error(env, result) {
            return ptr::null_mut();
        }
        env.byte_array_from_slice(&id)
            .map(|a| a.into_raw())
            .unwrap_or(ptr::null_mut())
    })
}

/// Lock-free balance of a `PlatformWallet` handle as a
/// `long[4]` = {confirmed, unconfirmed, immature, locked}.
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_WalletManagerNative_walletGetBalance(
    mut env: JNIEnv,
    _class: JClass,
    wallet_handle: jlong,
) -> jni::sys::jlongArray {
    guard(&mut env, ptr::null_mut(), |env| {
        let mut confirmed: u64 = 0;
        let mut unconfirmed: u64 = 0;
        let mut immature: u64 = 0;
        let mut locked: u64 = 0;
        let result = unsafe {
            platform_wallet_ffi::platform_wallet_get_balance(
                wallet_handle as Handle,
                &mut confirmed as *mut u64,
                &mut unconfirmed as *mut u64,
                &mut immature as *mut u64,
                &mut locked as *mut u64,
            )
        };
        if take_pwffi_error(env, result) {
            return ptr::null_mut();
        }
        let out = [
            confirmed as jlong,
            unconfirmed as jlong,
            immature as jlong,
            locked as jlong,
        ];
        let Ok(arr) = env.new_long_array(4) else {
            return ptr::null_mut();
        };
        if env.set_long_array_region(&arr, 0, &out).is_err() {
            return ptr::null_mut();
        }
        arr.into_raw()
    })
}

/// Build, sign, and broadcast a Core payment from this platform wallet
/// to `addresses`/`amounts`, returning the serialized signed transaction
/// as a `byte[]`.
///
/// Single composite entry point (item 2): acquires the core-wallet handle
/// via `platform_wallet_get_core`, invokes `core_wallet_send_to_addresses`
/// (which builds + signs via the resolver-backed core signer AND
/// broadcasts), then releases the transient core-wallet handle and the
/// Rust-owned tx buffer before returning. No orchestration crosses the
/// JNI boundary — the get-core / send / free sequence is the exact shape
/// Swift's `ManagedCoreWallet.sendToAddresses` performs, kept on the Rust
/// side here.
///
/// `account_type`: 0 = BIP44, 1 = BIP32. `addresses` is a `String[]`,
/// `amounts` a matching `long[]` (duffs). `core_signer_handle` is a
/// `MnemonicResolverHandle` (the manager's resolver) used for the Core
/// ECDSA signatures. Returns the tx bytes on success, or null (after
/// throwing) on error.
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_WalletManagerNative_walletCoreSendToAddresses(
    mut env: JNIEnv,
    _class: JClass,
    wallet_handle: jlong,
    account_type: jni::sys::jint,
    account_index: jni::sys::jint,
    addresses: JObjectArray,
    amounts: jlongArray,
    core_signer_handle: jlong,
) -> jbyteArray {
    guard(&mut env, ptr::null_mut(), |env| {
        // Resolve the transient core-wallet handle from the platform wallet.
        let mut core_handle: Handle = 0;
        let result = unsafe {
            platform_wallet_ffi::platform_wallet_get_core(
                wallet_handle as Handle,
                &mut core_handle as *mut Handle,
            )
        };
        if take_pwffi_error(env, result) {
            return ptr::null_mut();
        }

        // From here on, the core handle must be released on every exit path.
        let send_and_encode = |env: &mut JNIEnv| -> jbyteArray {
            // Marshal amounts (long[]) → Vec<u64>.
            let count = match env
                .get_array_length(&unsafe { jni::objects::JLongArray::from_raw(amounts) })
            {
                Ok(n) if n >= 0 => n as usize,
                _ => {
                    throw_sdk_exception(env, 1, "amounts array was null/invalid");
                    return ptr::null_mut();
                }
            };
            let mut amount_buf = vec![0i64; count];
            if env
                .get_long_array_region(
                    &unsafe { jni::objects::JLongArray::from_raw(amounts) },
                    0,
                    &mut amount_buf,
                )
                .is_err()
            {
                throw_sdk_exception(env, 1, "failed to read amounts array");
                return ptr::null_mut();
            }
            let amounts_u64: Vec<u64> = amount_buf.iter().map(|&v| v as u64).collect();

            // Marshal addresses (String[]) → owned CStrings → *const c_char.
            let addr_count = match env.get_array_length(&addresses) {
                Ok(n) if n as usize == count => n as usize,
                _ => {
                    throw_sdk_exception(
                        env,
                        1,
                        "addresses/amounts length mismatch or null addresses",
                    );
                    return ptr::null_mut();
                }
            };
            let mut owned: Vec<std::ffi::CString> = Vec::with_capacity(addr_count);
            for i in 0..addr_count {
                let obj = match env.get_object_array_element(&addresses, i as jni::sys::jsize) {
                    Ok(o) => o,
                    Err(_) => {
                        throw_sdk_exception(env, 1, "failed to read an address element");
                        return ptr::null_mut();
                    }
                };
                let jstr = jni::objects::JString::from(obj);
                let s: String = match env.get_string(&jstr) {
                    Ok(js) => js.into(),
                    Err(_) => {
                        throw_sdk_exception(env, 1, "an address element was not a String");
                        return ptr::null_mut();
                    }
                };
                match std::ffi::CString::new(s) {
                    Ok(c) => owned.push(c),
                    Err(_) => {
                        throw_sdk_exception(env, 1, "an address contained a NUL byte");
                        return ptr::null_mut();
                    }
                }
            }
            let addr_ptrs: Vec<*const std::os::raw::c_char> =
                owned.iter().map(|c| c.as_ptr()).collect();

            let mut out_tx: *mut u8 = ptr::null_mut();
            let mut out_len: usize = 0;
            let send_result = unsafe {
                platform_wallet_ffi::core_wallet_send_to_addresses(
                    core_handle,
                    account_type.max(0) as u32,
                    account_index.max(0) as u32,
                    addr_ptrs.as_ptr(),
                    amounts_u64.as_ptr(),
                    count,
                    core_signer_handle as *mut rs_sdk_ffi::MnemonicResolverHandle,
                    &mut out_tx as *mut *mut u8,
                    &mut out_len as *mut usize,
                )
            };
            if take_pwffi_error(env, send_result) {
                return ptr::null_mut();
            }

            let jarr = if out_tx.is_null() || out_len == 0 {
                // Success but empty buffer — return an empty array rather
                // than null so the caller can distinguish from an error.
                env.byte_array_from_slice(&[])
                    .map(|a| a.into_raw())
                    .unwrap_or(ptr::null_mut())
            } else {
                let bytes = unsafe { std::slice::from_raw_parts(out_tx, out_len) };
                let a = env
                    .byte_array_from_slice(bytes)
                    .map(|a| a.into_raw())
                    .unwrap_or(ptr::null_mut());
                unsafe { platform_wallet_ffi::core_wallet_free_tx_bytes(out_tx, out_len) };
                a
            };
            jarr
        };

        let out = send_and_encode(env);

        // Release the transient core-wallet handle regardless of outcome.
        let destroy_result = unsafe { platform_wallet_ffi::core_wallet_destroy(core_handle) };
        // Only surface a destroy error if we don't already have a pending
        // exception / result to report.
        if !env.exception_check().unwrap_or(false) {
            let _ = take_pwffi_error(env, destroy_result);
        } else {
            unsafe { platform_wallet_ffi_result_free(&mut { destroy_result }) };
        }

        out
    })
}

/// Enumerate this wallet's Platform-payment addresses with their cached
/// credit balances, returning a flat `byte[]` BLOB for the top-up
/// funding-input builder (`TopUpIdentityScreen`).
///
/// Single composite entry point (mirrors [`walletCoreSendToAddresses`]):
/// resolves the transient platform-address wallet handle via
/// `platform_wallet_get_platform`, reads
/// `platform_address_wallet_addresses_with_balances`, frees the Rust-owned
/// balances array and the transient handle, then returns. No orchestration
/// crosses the JNI boundary.
///
/// BLOB layout (big-endian): `u32 rowCount` then per row
/// `u8 addressType (0 P2PKH / 1 P2SH), u8[20] hash, u64 balance` — the
/// same shape the top-up inputs BLOB consumes, so the Kotlin caller can
/// filter/greedily pack rows into a funding request without re-marshalling.
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_WalletManagerNative_walletAddressesWithBalances(
    mut env: JNIEnv,
    _class: JClass,
    wallet_handle: jlong,
) -> jbyteArray {
    guard(&mut env, ptr::null_mut(), |env| {
        // Resolve the transient platform-address wallet handle.
        let mut addr_handle: Handle = 0;
        let result = unsafe {
            platform_wallet_ffi::platform_wallet_get_platform(
                wallet_handle as Handle,
                &mut addr_handle as *mut Handle,
            )
        };
        if take_pwffi_error(env, result) {
            return ptr::null_mut();
        }

        // Enumerate; must free the balances array and the transient handle
        // on every exit path.
        let mut entries: *mut platform_wallet_ffi::AddressBalanceEntryFFI = ptr::null_mut();
        let mut count: usize = 0;
        let enum_result = unsafe {
            platform_wallet_ffi::platform_address_wallet_addresses_with_balances(
                addr_handle,
                &mut entries as *mut *mut platform_wallet_ffi::AddressBalanceEntryFFI,
                &mut count as *mut usize,
            )
        };

        let out = if take_pwffi_error(env, enum_result) {
            ptr::null_mut()
        } else {
            // Serialize into the BLOB before freeing the Rust-owned array.
            let mut blob = Vec::with_capacity(4 + count * 29);
            blob.extend_from_slice(&(count as u32).to_be_bytes());
            if !entries.is_null() && count > 0 {
                let rows = unsafe { std::slice::from_raw_parts(entries, count) };
                for row in rows {
                    blob.push(row.address.address_type);
                    blob.extend_from_slice(&row.address.hash);
                    blob.extend_from_slice(&row.balance.to_be_bytes());
                }
            }
            unsafe {
                platform_wallet_ffi::platform_address_wallet_free_address_balances(entries, count)
            };
            env.byte_array_from_slice(&blob)
                .map(|a| a.into_raw())
                .unwrap_or(ptr::null_mut())
        };

        // Release the transient platform-address wallet handle.
        let destroy_result =
            unsafe { platform_wallet_ffi::platform_address_wallet_destroy(addr_handle) };
        if !env.exception_check().unwrap_or(false) {
            let _ = take_pwffi_error(env, destroy_result);
        } else {
            unsafe { platform_wallet_ffi_result_free(&mut { destroy_result }) };
        }

        out
    })
}

// ── Asset-lock funding of Platform addresses ──────────────────────────

/// Decode the recipient BLOB into `FundingAddressEntryFFI` rows and derive
/// the `ReduceOutput` fee step (at the first `hasBalance = false` remainder
/// recipient, mirroring `ManagedPlatformAddressWallet.fundFromAssetLock`).
///
/// BLOB layout (big-endian): `u32 rowCount` then per row
/// `u8 addressType, u8[20] hash, u8 hasBalance (0/1), u64 balance`.
///
/// Returns `(entries, feeRows)` or throws + returns `None` on a malformed
/// blob / missing remainder recipient.
fn decode_funding_recipients(
    env: &mut JNIEnv,
    arr: &JByteArray,
) -> Option<(
    Vec<platform_wallet_ffi::FundingAddressEntryFFI>,
    Vec<platform_wallet_ffi::FeeStrategyStepFFI>,
)> {
    let bytes = match env.convert_byte_array(arr) {
        Ok(b) => b,
        Err(_) => {
            let _ = env.exception_clear();
            throw_sdk_exception(env, 1, "recipients blob was null/invalid");
            return None;
        }
    };
    let mut cursor = 0usize;
    let read = |cursor: &mut usize, n: usize| -> Option<Vec<u8>> {
        if *cursor + n > bytes.len() {
            return None;
        }
        let s = bytes[*cursor..*cursor + n].to_vec();
        *cursor += n;
        Some(s)
    };
    let Some(count_bytes) = read(&mut cursor, 4) else {
        throw_sdk_exception(env, 1, "recipients blob truncated (row count)");
        return None;
    };
    let count = u32::from_be_bytes(count_bytes.as_slice().try_into().ok()?) as usize;
    let mut entries = Vec::with_capacity(count);
    let mut remainder_index: Option<u16> = None;
    for i in 0..count {
        let Some(type_byte) = read(&mut cursor, 1) else {
            throw_sdk_exception(
                env,
                1,
                &format!("recipients blob truncated at row {i} type"),
            );
            return None;
        };
        let Some(hash_bytes) = read(&mut cursor, 20) else {
            throw_sdk_exception(
                env,
                1,
                &format!("recipients blob truncated at row {i} hash"),
            );
            return None;
        };
        let Some(has_balance_byte) = read(&mut cursor, 1) else {
            throw_sdk_exception(
                env,
                1,
                &format!("recipients blob truncated at row {i} hasBalance"),
            );
            return None;
        };
        let Some(balance_bytes) = read(&mut cursor, 8) else {
            throw_sdk_exception(
                env,
                1,
                &format!("recipients blob truncated at row {i} balance"),
            );
            return None;
        };
        let mut hash = [0u8; 20];
        hash.copy_from_slice(&hash_bytes);
        let has_balance = has_balance_byte[0] != 0;
        if !has_balance && remainder_index.is_none() {
            remainder_index = Some(i as u16);
        }
        entries.push(platform_wallet_ffi::FundingAddressEntryFFI {
            address: platform_wallet_ffi::PlatformAddressFFI {
                address_type: type_byte[0],
                hash,
            },
            has_balance,
            balance: u64::from_be_bytes(balance_bytes.as_slice().try_into().ok()?),
        });
    }
    // Exactly one remainder recipient must be present (the fee-absorbing
    // output). The Rust FFI enforces this too, but we need its index for
    // the ReduceOutput fee step.
    let remainder = remainder_index.unwrap_or(0);
    let fee_rows = vec![platform_wallet_ffi::FeeStrategyStepFFI {
        step_type: 1, // 1 = ReduceOutput
        index: remainder,
    }];
    Some((entries, fee_rows))
}

/// Decode the credit-outputs BLOB for a wallet-signed platform-address
/// transfer into `AddressBalanceEntryFFI` rows.
///
/// BLOB layout (big-endian): `u32 rowCount` then per row
/// `u8 addressType (0 P2PKH only), u8[20] hash, u64 credits`. The row's
/// `balance` carries the credits to route to that recipient; `nonce` /
/// `account_index` / `address_index` are left `0` (the FFI only reads the
/// address + amount for a transfer output). Returns `None` (after throwing)
/// on a malformed / truncated blob.
fn decode_credit_outputs(
    env: &mut JNIEnv,
    arr: &JByteArray,
) -> Option<Vec<platform_wallet_ffi::AddressBalanceEntryFFI>> {
    let bytes = match env.convert_byte_array(arr) {
        Ok(b) => b,
        Err(_) => {
            let _ = env.exception_clear();
            throw_sdk_exception(env, 1, "outputs blob was null/invalid");
            return None;
        }
    };
    let mut cursor = 0usize;
    let read = |cursor: &mut usize, n: usize| -> Option<Vec<u8>> {
        if *cursor + n > bytes.len() {
            return None;
        }
        let s = bytes[*cursor..*cursor + n].to_vec();
        *cursor += n;
        Some(s)
    };
    let Some(count_bytes) = read(&mut cursor, 4) else {
        throw_sdk_exception(env, 1, "outputs blob truncated (row count)");
        return None;
    };
    let count = u32::from_be_bytes(count_bytes.as_slice().try_into().ok()?) as usize;
    let mut outputs = Vec::with_capacity(count);
    for i in 0..count {
        let Some(type_byte) = read(&mut cursor, 1) else {
            throw_sdk_exception(env, 1, &format!("outputs blob truncated at row {i} type"));
            return None;
        };
        let Some(hash_bytes) = read(&mut cursor, 20) else {
            throw_sdk_exception(env, 1, &format!("outputs blob truncated at row {i} hash"));
            return None;
        };
        let Some(credit_bytes) = read(&mut cursor, 8) else {
            throw_sdk_exception(
                env,
                1,
                &format!("outputs blob truncated at row {i} credits"),
            );
            return None;
        };
        let mut hash = [0u8; 20];
        hash.copy_from_slice(&hash_bytes);
        outputs.push(platform_wallet_ffi::AddressBalanceEntryFFI {
            address: platform_wallet_ffi::PlatformAddressFFI {
                address_type: type_byte[0],
                hash,
            },
            balance: u64::from_be_bytes(credit_bytes.as_slice().try_into().ok()?),
            nonce: 0,
            account_index: 0,
            address_index: 0,
        });
    }
    Some(outputs)
}

/// Serialize a `PlatformAddressChangeSetFFI` into a `byte[]` blob:
/// `u32 rowCount` then per row `u8 addressType, u8[20] hash, u64 balance`.
///
/// # Safety
/// `changeset` must be a valid changeset freshly returned by the FFI.
unsafe fn encode_changeset(
    changeset: &platform_wallet_ffi::PlatformAddressChangeSetFFI,
) -> Vec<u8> {
    let count = changeset.updated_count;
    let mut blob = Vec::with_capacity(4 + count * 29);
    blob.extend_from_slice(&(count as u32).to_be_bytes());
    if !changeset.updated.is_null() && count > 0 {
        let rows = std::slice::from_raw_parts(changeset.updated, count);
        for row in rows {
            blob.push(row.address.address_type);
            blob.extend_from_slice(&row.address.hash);
            blob.extend_from_slice(&row.balance.to_be_bytes());
        }
    }
    blob
}

/// Fund Platform addresses from a Core L1 asset lock built from the
/// wallet's balance, returning the resulting changeset (updated address
/// balances) as a `byte[]` blob.
///
/// Composite Rust call: resolve the transient platform-address wallet
/// handle (`platform_wallet_get_platform`), invoke
/// `platform_address_wallet_fund_from_asset_lock_signer`, then free the
/// changeset and destroy the transient handle. The `signerHandle` is the
/// platform-address per-input signer (`PlatformWalletManager.signerHandle`);
/// the `coreSignerHandle` is the manager's `MnemonicResolverHandle` for the
/// asset-lock's outer ST signature. No orchestration crosses this boundary
/// — the get/fund/free sequence is `ManagedPlatformAddressWallet`'s exact
/// shape kept on the Rust side.
///
/// Changeset blob layout (big-endian): `u32 rowCount` then per row
/// `u8 addressType, u8[20] hash, u64 balance`. See
/// [`decode_funding_recipients`] for the recipient blob layout.
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_WalletManagerNative_walletFundFromAssetLock(
    mut env: JNIEnv,
    _class: JClass,
    wallet_handle: jlong,
    amount_duffs: jlong,
    account_index: jni::sys::jint,
    platform_account_index: jni::sys::jint,
    recipients_blob: JByteArray,
    signer_handle: jlong,
    core_signer_handle: jlong,
) -> jbyteArray {
    guard(&mut env, ptr::null_mut(), |env| {
        let Some((entries, fee_rows)) = decode_funding_recipients(env, &recipients_blob) else {
            return ptr::null_mut();
        };

        // Resolve the transient platform-address wallet handle.
        let mut addr_handle: Handle = 0;
        let result = unsafe {
            platform_wallet_ffi::platform_wallet_get_platform(
                wallet_handle as Handle,
                &mut addr_handle as *mut Handle,
            )
        };
        if take_pwffi_error(env, result) {
            return ptr::null_mut();
        }

        let mut changeset = platform_wallet_ffi::PlatformAddressChangeSetFFI {
            updated: ptr::null_mut(),
            updated_count: 0,
        };
        let fund_result = unsafe {
            platform_wallet_ffi::platform_address_wallet_fund_from_asset_lock_signer(
                addr_handle,
                amount_duffs.max(0) as u64,
                account_index.max(0) as u32,
                platform_account_index.max(0) as u32,
                entries.as_ptr(),
                entries.len(),
                fee_rows.as_ptr(),
                fee_rows.len(),
                signer_handle as *mut rs_sdk_ffi::SignerHandle,
                core_signer_handle as *mut rs_sdk_ffi::MnemonicResolverHandle,
                &mut changeset as *mut platform_wallet_ffi::PlatformAddressChangeSetFFI,
            )
        };

        let out = if take_pwffi_error(env, fund_result) {
            ptr::null_mut()
        } else {
            let blob = unsafe { encode_changeset(&changeset) };
            unsafe {
                platform_wallet_ffi::platform_address_wallet_free_changeset(
                    &changeset as *const platform_wallet_ffi::PlatformAddressChangeSetFFI,
                )
            };
            env.byte_array_from_slice(&blob)
                .map(|a| a.into_raw())
                .unwrap_or(ptr::null_mut())
        };

        let destroy_result =
            unsafe { platform_wallet_ffi::platform_address_wallet_destroy(addr_handle) };
        if !env.exception_check().unwrap_or(false) {
            let _ = take_pwffi_error(env, destroy_result);
        } else {
            unsafe { platform_wallet_ffi_result_free(&mut { destroy_result }) };
        }

        out
    })
}

/// Resume a stuck Platform-address asset-lock funding from an already-
/// tracked lock by outpoint. Sibling of [`walletFundFromAssetLock`]: same
/// get/fund/free composite, but calls
/// `platform_address_wallet_resume_fund_from_asset_lock_signer` with the
/// 32-byte little-endian `outPointTxid` + `outPointVout` instead of a fresh
/// amount. Returns the changeset blob (same layout).
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_WalletManagerNative_walletResumeFundFromAssetLock(
    mut env: JNIEnv,
    _class: JClass,
    wallet_handle: jlong,
    out_point_txid: JByteArray,
    out_point_vout: jni::sys::jint,
    platform_account_index: jni::sys::jint,
    recipients_blob: JByteArray,
    signer_handle: jlong,
    core_signer_handle: jlong,
) -> jbyteArray {
    guard(&mut env, ptr::null_mut(), |env| {
        let txid = match env.convert_byte_array(&out_point_txid) {
            Ok(b) if b.len() == 32 => b,
            Ok(_) => {
                throw_sdk_exception(env, 1, "outPointTxid must be exactly 32 bytes");
                return ptr::null_mut();
            }
            Err(_) => {
                let _ = env.exception_clear();
                throw_sdk_exception(env, 1, "outPointTxid was null/invalid");
                return ptr::null_mut();
            }
        };
        let mut txid_arr = [0u8; 32];
        txid_arr.copy_from_slice(&txid);
        let out_point = platform_wallet_ffi::OutPointFFI {
            txid: txid_arr,
            vout: out_point_vout.max(0) as u32,
        };

        let Some((entries, fee_rows)) = decode_funding_recipients(env, &recipients_blob) else {
            return ptr::null_mut();
        };

        let mut addr_handle: Handle = 0;
        let result = unsafe {
            platform_wallet_ffi::platform_wallet_get_platform(
                wallet_handle as Handle,
                &mut addr_handle as *mut Handle,
            )
        };
        if take_pwffi_error(env, result) {
            return ptr::null_mut();
        }

        let mut changeset = platform_wallet_ffi::PlatformAddressChangeSetFFI {
            updated: ptr::null_mut(),
            updated_count: 0,
        };
        let fund_result = unsafe {
            platform_wallet_ffi::platform_address_wallet_resume_fund_from_asset_lock_signer(
                addr_handle,
                &out_point as *const platform_wallet_ffi::OutPointFFI,
                platform_account_index.max(0) as u32,
                entries.as_ptr(),
                entries.len(),
                fee_rows.as_ptr(),
                fee_rows.len(),
                signer_handle as *mut rs_sdk_ffi::SignerHandle,
                core_signer_handle as *mut rs_sdk_ffi::MnemonicResolverHandle,
                &mut changeset as *mut platform_wallet_ffi::PlatformAddressChangeSetFFI,
            )
        };

        let out = if take_pwffi_error(env, fund_result) {
            ptr::null_mut()
        } else {
            let blob = unsafe { encode_changeset(&changeset) };
            unsafe {
                platform_wallet_ffi::platform_address_wallet_free_changeset(
                    &changeset as *const platform_wallet_ffi::PlatformAddressChangeSetFFI,
                )
            };
            env.byte_array_from_slice(&blob)
                .map(|a| a.into_raw())
                .unwrap_or(ptr::null_mut())
        };

        let destroy_result =
            unsafe { platform_wallet_ffi::platform_address_wallet_destroy(addr_handle) };
        if !env.exception_check().unwrap_or(false) {
            let _ = take_pwffi_error(env, destroy_result);
        } else {
            unsafe { platform_wallet_ffi_result_free(&mut { destroy_result }) };
        }

        out
    })
}

// ── Wallet-signed Platform-address credit movement (ADDR-02/04, #3923) ─
//
// Transfer / withdraw platform-address credits, signed by the wallet's
// platform-address signer (NOT an identity key). Each is a composite
// mirroring `walletFundFromAssetLock`: resolve the transient
// platform-address wallet handle (`platform_wallet_get_platform`), call the
// single Rust entry point, free the changeset, destroy the transient
// handle. Input selection is AUTO (null explicit inputs) and the fee
// strategy is left null (the FFI defaults to `[DeductFromInput(0)]`); Rust
// owns selection / balancing / nonces / signing — the exact shape Swift's
// rewritten `ManagedPlatformAddressWallet.transfer` / `.withdraw` performs.
// The Rust side polls the transition on an 8 MB-stack worker thread, so the
// GroveDB proof-verification recursion survives regardless of the JNI
// thread's stack; we call it synchronously here (Kotlin confines the call
// to `Dispatchers.IO`).

/// Transfer platform-address credits to [outputs] recipients, wallet-signed.
///
/// `outputs_blob` (big-endian): `u32 rowCount` then per row
/// `u8 addressType (0 P2PKH only), u8[20] hash, u64 credits`. Only P2PKH
/// (type 0) is honored on the way in (the FFI rejects P2SH). Returns the
/// resulting changeset blob (`u32 rowCount` then per row
/// `u8 addressType, u8[20] hash, u64 balance`).
///
/// `signer_handle` is the platform-address per-input `SignerHandle`
/// (`PlatformWalletManager.signerHandle`).
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_WalletManagerNative_walletPlatformAddressTransfer(
    mut env: JNIEnv,
    _class: JClass,
    wallet_handle: jlong,
    account_index: jni::sys::jint,
    outputs_blob: JByteArray,
    signer_handle: jlong,
) -> jbyteArray {
    guard(&mut env, ptr::null_mut(), |env| {
        let Some(outputs) = decode_credit_outputs(env, &outputs_blob) else {
            return ptr::null_mut();
        };

        let mut addr_handle: Handle = 0;
        let result = unsafe {
            platform_wallet_ffi::platform_wallet_get_platform(
                wallet_handle as Handle,
                &mut addr_handle as *mut Handle,
            )
        };
        if take_pwffi_error(env, result) {
            return ptr::null_mut();
        }

        let mut changeset = platform_wallet_ffi::PlatformAddressChangeSetFFI {
            updated: ptr::null_mut(),
            updated_count: 0,
        };
        let transfer_result = unsafe {
            platform_wallet_ffi::platform_address_wallet_transfer(
                addr_handle,
                account_index.max(0) as u32,
                platform_wallet_ffi::InputSelectionType::Auto,
                ptr::null(),
                0,
                ptr::null(),
                0,
                outputs.as_ptr(),
                outputs.len(),
                ptr::null(),
                0,
                signer_handle as *mut rs_sdk_ffi::SignerHandle,
                &mut changeset as *mut platform_wallet_ffi::PlatformAddressChangeSetFFI,
            )
        };

        let out = if take_pwffi_error(env, transfer_result) {
            ptr::null_mut()
        } else {
            let blob = unsafe { encode_changeset(&changeset) };
            unsafe {
                platform_wallet_ffi::platform_address_wallet_free_changeset(
                    &changeset as *const platform_wallet_ffi::PlatformAddressChangeSetFFI,
                )
            };
            env.byte_array_from_slice(&blob)
                .map(|a| a.into_raw())
                .unwrap_or(ptr::null_mut())
        };

        let destroy_result =
            unsafe { platform_wallet_ffi::platform_address_wallet_destroy(addr_handle) };
        if !env.exception_check().unwrap_or(false) {
            let _ = take_pwffi_error(env, destroy_result);
        } else {
            unsafe { platform_wallet_ffi_result_free(&mut { destroy_result }) };
        }

        out
    })
}

/// Withdraw platform-address credits (full account balance, AUTO input
/// selection) to a Core L1 address, wallet-signed. The address is
/// network-checked Rust-side against the wallet's own network. Returns the
/// resulting changeset blob (same layout as [`walletPlatformAddressTransfer`]).
///
/// `core_address` is a base58 Core address; `core_fee_per_byte` must be a
/// Fibonacci-sequence value (DPP rejects non-Fibonacci rates). `signer_handle`
/// is the platform-address per-input `SignerHandle`.
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_WalletManagerNative_walletPlatformAddressWithdraw(
    mut env: JNIEnv,
    _class: JClass,
    wallet_handle: jlong,
    account_index: jni::sys::jint,
    core_address: JString,
    core_fee_per_byte: jni::sys::jint,
    signer_handle: jlong,
) -> jbyteArray {
    guard(&mut env, ptr::null_mut(), |env| {
        let Some(core_address_c) = read_cstring_required(env, &core_address, "core_address") else {
            return ptr::null_mut();
        };

        let mut addr_handle: Handle = 0;
        let result = unsafe {
            platform_wallet_ffi::platform_wallet_get_platform(
                wallet_handle as Handle,
                &mut addr_handle as *mut Handle,
            )
        };
        if take_pwffi_error(env, result) {
            return ptr::null_mut();
        }

        let mut changeset = platform_wallet_ffi::PlatformAddressChangeSetFFI {
            updated: ptr::null_mut(),
            updated_count: 0,
        };
        let withdraw_result = unsafe {
            platform_wallet_ffi::platform_address_wallet_withdraw_to_address(
                addr_handle,
                account_index.max(0) as u32,
                platform_wallet_ffi::InputSelectionType::Auto,
                ptr::null(),
                0,
                ptr::null(),
                0,
                core_address_c.as_ptr(),
                core_fee_per_byte.max(0) as u32,
                ptr::null(),
                0,
                signer_handle as *mut rs_sdk_ffi::SignerHandle,
                &mut changeset as *mut platform_wallet_ffi::PlatformAddressChangeSetFFI,
            )
        };

        let out = if take_pwffi_error(env, withdraw_result) {
            ptr::null_mut()
        } else {
            let blob = unsafe { encode_changeset(&changeset) };
            unsafe {
                platform_wallet_ffi::platform_address_wallet_free_changeset(
                    &changeset as *const platform_wallet_ffi::PlatformAddressChangeSetFFI,
                )
            };
            env.byte_array_from_slice(&blob)
                .map(|a| a.into_raw())
                .unwrap_or(ptr::null_mut())
        };

        let destroy_result =
            unsafe { platform_wallet_ffi::platform_address_wallet_destroy(addr_handle) };
        if !env.exception_check().unwrap_or(false) {
            let _ = take_pwffi_error(env, destroy_result);
        } else {
            unsafe { platform_wallet_ffi_result_free(&mut { destroy_result }) };
        }

        out
    })
}

/// Preflight an AUTO withdrawal WITHOUT signing / broadcasting / consuming a
/// Core address. Returns a `long[3]` = `[canWithdraw (0/1), netWithdrawable,
/// estimatedFee]`; the figures are `0` when `canWithdraw == 0`. The "can't
/// fund" reason is a Success-coded message Rust-side (never thrown), so the
/// UI gates purely on the `canWithdraw` flag — the authoritative signal.
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_WalletManagerNative_walletPlatformAddressPreflightWithdrawal(
    mut env: JNIEnv,
    _class: JClass,
    wallet_handle: jlong,
    account_index: jni::sys::jint,
    core_fee_per_byte: jni::sys::jint,
) -> jlongArray {
    guard(&mut env, ptr::null_mut(), |env| {
        let mut addr_handle: Handle = 0;
        let result = unsafe {
            platform_wallet_ffi::platform_wallet_get_platform(
                wallet_handle as Handle,
                &mut addr_handle as *mut Handle,
            )
        };
        if take_pwffi_error(env, result) {
            return ptr::null_mut();
        }

        let mut preflight = platform_wallet_ffi::WithdrawalPreflightFFI {
            can_withdraw: false,
            net_withdrawable: 0,
            estimated_fee: 0,
        };
        let preflight_result = unsafe {
            platform_wallet_ffi::platform_address_wallet_preflight_withdrawal(
                addr_handle,
                account_index.max(0) as u32,
                core_fee_per_byte.max(0) as u32,
                &mut preflight as *mut platform_wallet_ffi::WithdrawalPreflightFFI,
            )
        };

        let out = if take_pwffi_error(env, preflight_result) {
            ptr::null_mut()
        } else {
            let triple = [
                if preflight.can_withdraw { 1 } else { 0 },
                preflight.net_withdrawable as jlong,
                preflight.estimated_fee as jlong,
            ];
            let Ok(arr) = env.new_long_array(3) else {
                return ptr::null_mut();
            };
            if env.set_long_array_region(&arr, 0, &triple).is_err() {
                return ptr::null_mut();
            }
            arr.into_raw()
        };

        let destroy_result =
            unsafe { platform_wallet_ffi::platform_address_wallet_destroy(addr_handle) };
        if !env.exception_check().unwrap_or(false) {
            let _ = take_pwffi_error(env, destroy_result);
        } else {
            unsafe { platform_wallet_ffi_result_free(&mut { destroy_result }) };
        }

        out
    })
}

/// The version-locked minimum input / output amounts (credits) that gate the
/// platform-address transfer/withdraw UI, as a `long[2]` = `[minInput,
/// minOutput]`. Two getters folded into one composite (get-platform → read
/// both → destroy-handle) so the UI needs one JNI hop.
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_WalletManagerNative_walletPlatformAddressMinAmounts(
    mut env: JNIEnv,
    _class: JClass,
    wallet_handle: jlong,
) -> jlongArray {
    guard(&mut env, ptr::null_mut(), |env| {
        let mut addr_handle: Handle = 0;
        let result = unsafe {
            platform_wallet_ffi::platform_wallet_get_platform(
                wallet_handle as Handle,
                &mut addr_handle as *mut Handle,
            )
        };
        if take_pwffi_error(env, result) {
            return ptr::null_mut();
        }

        let mut min_input: u64 = 0;
        let mut min_output: u64 = 0;
        let input_result = unsafe {
            platform_wallet_ffi::platform_address_wallet_min_input_amount(
                addr_handle,
                &mut min_input as *mut u64,
            )
        };
        let out = if take_pwffi_error(env, input_result) {
            ptr::null_mut()
        } else {
            let output_result = unsafe {
                platform_wallet_ffi::platform_address_wallet_min_output_amount(
                    addr_handle,
                    &mut min_output as *mut u64,
                )
            };
            if take_pwffi_error(env, output_result) {
                ptr::null_mut()
            } else {
                let pair = [min_input as jlong, min_output as jlong];
                let Ok(arr) = env.new_long_array(2) else {
                    return ptr::null_mut();
                };
                if env.set_long_array_region(&arr, 0, &pair).is_err() {
                    return ptr::null_mut();
                }
                arr.into_raw()
            }
        };

        let destroy_result =
            unsafe { platform_wallet_ffi::platform_address_wallet_destroy(addr_handle) };
        if !env.exception_check().unwrap_or(false) {
            let _ = take_pwffi_error(env, destroy_result);
        } else {
            unsafe { platform_wallet_ffi_result_free(&mut { destroy_result }) };
        }

        out
    })
}

/// Destroy a `PlatformWallet` handle (drops the manager's `Arc` clone).
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_WalletManagerNative_walletDestroy(
    mut env: JNIEnv,
    _class: JClass,
    wallet_handle: jlong,
) {
    guard(&mut env, (), |env| {
        let result =
            unsafe { platform_wallet_ffi::platform_wallet_destroy(wallet_handle as Handle) };
        let _ = take_pwffi_error(env, result);
    })
}

// ── Exports: sync lifecycle ───────────────────────────────────────────
//
// Each loop follows start / stop / is_running (bool out-param). They map
// 1:1 onto the platform-wallet-ffi entry points; no composites.

macro_rules! sync_start_stop {
    ($start_name:ident, $stop_name:ident, $running_name:ident, $ffi_start:path, $ffi_stop:path, $ffi_running:path) => {
        #[no_mangle]
        pub extern "system" fn $start_name(mut env: JNIEnv, _class: JClass, manager_handle: jlong) {
            guard(&mut env, (), |env| {
                let result = unsafe { $ffi_start(manager_handle as Handle) };
                let _ = take_pwffi_error(env, result);
            })
        }

        #[no_mangle]
        pub extern "system" fn $stop_name(mut env: JNIEnv, _class: JClass, manager_handle: jlong) {
            guard(&mut env, (), |env| {
                let result = unsafe { $ffi_stop(manager_handle as Handle) };
                let _ = take_pwffi_error(env, result);
            })
        }

        #[no_mangle]
        pub extern "system" fn $running_name(
            mut env: JNIEnv,
            _class: JClass,
            manager_handle: jlong,
        ) -> jboolean {
            guard(&mut env, JNI_FALSE, |env| {
                let mut running = false;
                let result =
                    unsafe { $ffi_running(manager_handle as Handle, &mut running as *mut bool) };
                if take_pwffi_error(env, result) {
                    return JNI_FALSE;
                }
                if running {
                    JNI_TRUE
                } else {
                    JNI_FALSE
                }
            })
        }
    };
}

sync_start_stop!(
    Java_org_dashfoundation_dashsdk_ffi_WalletManagerNative_platformAddressSyncStart,
    Java_org_dashfoundation_dashsdk_ffi_WalletManagerNative_platformAddressSyncStop,
    Java_org_dashfoundation_dashsdk_ffi_WalletManagerNative_platformAddressSyncIsRunning,
    platform_wallet_ffi::platform_wallet_manager_platform_address_sync_start,
    platform_wallet_ffi::platform_wallet_manager_platform_address_sync_stop,
    platform_wallet_ffi::platform_wallet_manager_platform_address_sync_is_running
);

/// Reset the platform-address (BLAST) sync state — the native half of the
/// Sync tab's "Clear" action (#3959). Quiesces the sync loop (stops it,
/// leaves it restartable — does NOT auto-restart), then per wallet clears
/// the managed-account credit balances and the provider's watermark +
/// found/absent seed, preserving the durable address bijection. The next
/// start is therefore a full rescan. Mirrors Swift's
/// `PlatformWalletManager.resetPlatformAddressSyncState()`.
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_WalletManagerNative_platformAddressSyncReset(
    mut env: JNIEnv,
    _class: JClass,
    manager_handle: jlong,
) {
    guard(&mut env, (), |env| {
        let result = unsafe {
            platform_wallet_ffi::platform_wallet_manager_platform_address_sync_reset(
                manager_handle as Handle,
            )
        };
        let _ = take_pwffi_error(env, result);
    })
}

sync_start_stop!(
    Java_org_dashfoundation_dashsdk_ffi_WalletManagerNative_identitySyncStart,
    Java_org_dashfoundation_dashsdk_ffi_WalletManagerNative_identitySyncStop,
    Java_org_dashfoundation_dashsdk_ffi_WalletManagerNative_identitySyncIsRunning,
    platform_wallet_ffi::platform_wallet_manager_identity_sync_start,
    platform_wallet_ffi::platform_wallet_manager_identity_sync_stop,
    platform_wallet_ffi::platform_wallet_manager_identity_sync_is_running
);

#[cfg(feature = "shielded")]
sync_start_stop!(
    Java_org_dashfoundation_dashsdk_ffi_WalletManagerNative_shieldedSyncStart,
    Java_org_dashfoundation_dashsdk_ffi_WalletManagerNative_shieldedSyncStop,
    Java_org_dashfoundation_dashsdk_ffi_WalletManagerNative_shieldedSyncIsRunning,
    platform_wallet_ffi::platform_wallet_manager_shielded_sync_start,
    platform_wallet_ffi::platform_wallet_manager_shielded_sync_stop,
    platform_wallet_ffi::platform_wallet_manager_shielded_sync_is_running
);

/// Start the Core SPV client. Flattened form of
/// `platform_wallet_manager_spv_start` — the FFI takes 11 discrete
/// parameters (no config struct), so we take the same flat set here:
///
/// - `data_dir`: SPV storage directory (required, non-empty).
/// - `network`: `Network.ffiValue`.
/// - `user_agent`: optional; JVM null → FFI null (default user agent).
/// - `peers`: `String[]` of `host:port` seeds (may be empty).
/// - `restrict_to_configured_peers`: connect ONLY to `peers` when true.
/// - `start_from_height`: 0 = genesis / checkpoint default.
/// - `devnet_name`: required iff `network == Devnet`, JVM null otherwise.
/// - `llmq_devnet_size` / `llmq_devnet_threshold`: devnet LLMQ override;
///   both 0 = no override. The FFI validates the pairing + devnet scoping
///   and throws `ErrorInvalidParameter` on a mismatch.
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_WalletManagerNative_spvStart(
    mut env: JNIEnv,
    _class: JClass,
    manager_handle: jlong,
    data_dir: JString,
    network: jni::sys::jint,
    user_agent: JString,
    peers: JObjectArray,
    restrict_to_configured_peers: jboolean,
    start_from_height: jni::sys::jint,
    devnet_name: JString,
    llmq_devnet_size: jni::sys::jint,
    llmq_devnet_threshold: jni::sys::jint,
) {
    guard(&mut env, (), |env| {
        // data_dir is required.
        let Some(data_dir_c) = read_cstring_required(env, &data_dir, "data_dir") else {
            return;
        };
        // Optional strings: JVM null → Rust None (null C ptr).
        let user_agent_c = match read_cstring_opt(env, &user_agent) {
            Ok(c) => c,
            Err(_) => {
                throw_sdk_exception(env, 1, "user_agent contained an interior NUL");
                return;
            }
        };
        let devnet_name_c = match read_cstring_opt(env, &devnet_name) {
            Ok(c) => c,
            Err(_) => {
                throw_sdk_exception(env, 1, "devnet_name contained an interior NUL");
                return;
            }
        };

        // Peers: String[] → Vec<CString> → Vec<*const c_char>. The CStrings
        // must outlive the FFI call, so keep them alive alongside the ptr vec.
        // `_peer_cstrings` owns the peer buffers `peer_ptrs` references; the
        // leading underscore keeps it live to end-of-scope without a warning.
        let (_peer_cstrings, peer_ptrs) = match read_cstring_array(env, &peers) {
            Ok(v) => v,
            Err(_) => {
                let _ = env.exception_clear();
                throw_sdk_exception(env, 1, "peers array was null/invalid");
                return;
            }
        };
        let ua_ptr = user_agent_c.as_ref().map_or(ptr::null(), |c| c.as_ptr());
        let dn_ptr = devnet_name_c.as_ref().map_or(ptr::null(), |c| c.as_ptr());
        let peers_ptr = if peer_ptrs.is_empty() {
            ptr::null()
        } else {
            peer_ptrs.as_ptr()
        };

        let result = unsafe {
            platform_wallet_ffi::platform_wallet_manager_spv_start(
                manager_handle as Handle,
                data_dir_c.as_ptr(),
                ffi_network(network),
                ua_ptr,
                peers_ptr,
                peer_ptrs.len(),
                restrict_to_configured_peers == JNI_TRUE,
                start_from_height as u32,
                dn_ptr,
                llmq_devnet_size as u32,
                llmq_devnet_threshold as u32,
            )
        };
        // `_peer_cstrings` / `user_agent_c` / `devnet_name_c` / `data_dir_c`
        // own the memory the pointers referenced; they stay in scope through
        // the FFI call above and are dropped at the end of this closure.
        let _ = take_pwffi_error(env, result);
    })
}

/// Poll the SPV sync progress. Fills two caller-allocated arrays with the
/// flattened `FFISpvSyncProgress`:
///
/// - `out_longs` (`long[17]`): `[overallState, hasHeaders, headersState,
///   headersCurrent, headersTarget, hasFilterHeaders, filterHeadersState,
///   filterHeadersCurrent, filterHeadersTarget, hasFilters, filtersState,
///   filtersCurrent, filtersTarget, hasMasternodes, masternodesState,
///   masternodesCurrent, masternodesTarget]` (bools as 0/1).
/// - `out_percentages` (`double[5]`): `[overall, headers, filterHeaders,
///   filters, masternodes]`.
///
/// Both arrays are written in full on success. `SyncState` u32 constants:
/// 0=WaitForEvents, 1=WaitingForConnections, 2=Syncing, 3=Synced, 4=Error.
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_WalletManagerNative_spvSyncProgress(
    mut env: JNIEnv,
    _class: JClass,
    manager_handle: jlong,
    out_longs: jlongArray,
    out_percentages: jdoubleArray,
) {
    guard(&mut env, (), |env| {
        let mut p = platform_wallet_ffi::spv::FFISpvSyncProgress::default();
        let result = unsafe {
            platform_wallet_ffi::platform_wallet_manager_sync_progress(
                manager_handle as Handle,
                &mut p as *mut platform_wallet_ffi::spv::FFISpvSyncProgress,
            )
        };
        if take_pwffi_error(env, result) {
            return;
        }
        let longs: [jlong; 17] = [
            p.overall_state as jlong,
            p.has_headers as jlong,
            p.headers_state as jlong,
            p.headers_current as jlong,
            p.headers_target as jlong,
            p.has_filter_headers as jlong,
            p.filter_headers_state as jlong,
            p.filter_headers_current as jlong,
            p.filter_headers_target as jlong,
            p.has_filters as jlong,
            p.filters_state as jlong,
            p.filters_current as jlong,
            p.filters_target as jlong,
            p.has_masternodes as jlong,
            p.masternodes_state as jlong,
            p.masternodes_current as jlong,
            p.masternodes_target as jlong,
        ];
        let percents: [f64; 5] = [
            p.overall_percentage,
            p.headers_percentage,
            p.filter_headers_percentage,
            p.filters_percentage,
            p.masternodes_percentage,
        ];
        // SAFETY: the arrays are Kotlin-allocated LongArray(17) / DoubleArray(5).
        if !out_longs.is_null() {
            let out_longs = unsafe { jni::objects::JLongArray::from_raw(out_longs) };
            let _ = env.set_long_array_region(&out_longs, 0, &longs);
        }
        if !out_percentages.is_null() {
            let out_percentages = unsafe { jni::objects::JDoubleArray::from_raw(out_percentages) };
            let _ = env.set_double_array_region(&out_percentages, 0, &percents);
        }
    })
}

/// The unix-seconds block time of the SPV header tip, or 0 when the client
/// isn't running / no headers stored. A stale value across polls means the
/// chain has stalled (Swift's `currentSpvTipBlockTime`).
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_WalletManagerNative_spvTipUnixSeconds(
    mut env: JNIEnv,
    _class: JClass,
    manager_handle: jlong,
) -> jlong {
    guard(&mut env, 0, |env| {
        let mut secs: u64 = 0;
        let result = unsafe {
            platform_wallet_ffi::platform_wallet_manager_spv_tip_unix_seconds(
                manager_handle as Handle,
                &mut secs as *mut u64,
            )
        };
        if take_pwffi_error(env, result) {
            return 0;
        }
        secs as jlong
    })
}

/// Clear all persisted SPV storage (headers, filters, state).
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_WalletManagerNative_spvClearStorage(
    mut env: JNIEnv,
    _class: JClass,
    manager_handle: jlong,
) {
    guard(&mut env, (), |env| {
        let result = unsafe {
            platform_wallet_ffi::platform_wallet_manager_spv_clear_storage(manager_handle as Handle)
        };
        let _ = take_pwffi_error(env, result);
    })
}

/// SPV `is_running`.
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_WalletManagerNative_spvIsRunning(
    mut env: JNIEnv,
    _class: JClass,
    manager_handle: jlong,
) -> jboolean {
    guard(&mut env, JNI_FALSE, |env| {
        let mut running = false;
        let result = unsafe {
            platform_wallet_ffi::platform_wallet_manager_spv_is_running(
                manager_handle as Handle,
                &mut running as *mut bool,
            )
        };
        if take_pwffi_error(env, result) {
            return JNI_FALSE;
        }
        if running {
            JNI_TRUE
        } else {
            JNI_FALSE
        }
    })
}

#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_WalletManagerNative_spvStop(
    mut env: JNIEnv,
    _class: JClass,
    manager_handle: jlong,
) {
    guard(&mut env, (), |env| {
        let result = unsafe {
            platform_wallet_ffi::platform_wallet_manager_spv_stop(manager_handle as Handle)
        };
        let _ = take_pwffi_error(env, result);
    })
}

// ── Shared helpers ────────────────────────────────────────────────────

/// Map the Kotlin `Network.ffiValue` int (0=Mainnet,1=Testnet,2=Devnet,
/// 3=Regtest) to `FFINetwork`.
fn ffi_network(value: jni::sys::jint) -> dash_network::ffi::FFINetwork {
    match value {
        0 => dash_network::ffi::FFINetwork::Mainnet,
        2 => dash_network::ffi::FFINetwork::Devnet,
        3 => dash_network::ffi::FFINetwork::Regtest,
        _ => dash_network::ffi::FFINetwork::Testnet,
    }
}

/// Read a 32-byte id from a Java `byte[]`; throws + returns None on the
/// wrong length or a JNI error.
fn read_id32(env: &mut JNIEnv, arr: &JByteArray) -> Option<[u8; 32]> {
    let bytes = match env.convert_byte_array(arr) {
        Ok(b) => b,
        Err(_) => {
            let _ = env.exception_clear();
            throw_sdk_exception(env, 1, "wallet_id byte[] was null/invalid");
            return None;
        }
    };
    if bytes.len() != 32 {
        throw_sdk_exception(
            env,
            1,
            &format!("wallet_id must be 32 bytes, got {}", bytes.len()),
        );
        return None;
    }
    let mut id = [0u8; 32];
    id.copy_from_slice(&bytes);
    Some(id)
}

/// Read a required Java `String` into an owned `CString`; throws + returns
/// None on JVM null, a JNI error, an empty string, or an interior NUL.
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

/// Read an optional Java `String` into `Option<CString>`; JVM null →
/// `Ok(None)`. Returns `Err` only on an interior NUL (the caller throws).
/// A JNI read error is treated as null.
fn read_cstring_opt(env: &mut JNIEnv, s: &JString) -> Result<Option<std::ffi::CString>, ()> {
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
    match std::ffi::CString::new(owned) {
        Ok(c) => Ok(Some(c)),
        Err(_) => Err(()),
    }
}

/// Read a Java `String[]` into owned `CString`s plus a parallel `*const
/// c_char` pointer vec. The returned `Vec<CString>` OWNS the memory the
/// pointer vec references — both must outlive the FFI call. Null / invalid
/// elements are skipped (mirrors the FFI's per-peer tolerance).
fn read_cstring_array(
    env: &mut JNIEnv,
    arr: &JObjectArray,
) -> Result<(Vec<std::ffi::CString>, Vec<*const std::os::raw::c_char>), jni::errors::Error> {
    if arr.is_null() {
        return Ok((Vec::new(), Vec::new()));
    }
    let len = env.get_array_length(arr)? as usize;
    let mut owned: Vec<std::ffi::CString> = Vec::with_capacity(len);
    for i in 0..len {
        let element = env.get_object_array_element(arr, i as i32)?;
        if element.is_null() {
            continue;
        }
        let jstr = JString::from(element);
        let s: String = match env.get_string(&jstr) {
            Ok(v) => v.into(),
            Err(_) => {
                let _ = env.exception_clear();
                continue;
            }
        };
        if let Ok(c) = std::ffi::CString::new(s) {
            owned.push(c);
        }
    }
    let ptrs: Vec<*const std::os::raw::c_char> = owned.iter().map(|c| c.as_ptr()).collect();
    Ok((owned, ptrs))
}
