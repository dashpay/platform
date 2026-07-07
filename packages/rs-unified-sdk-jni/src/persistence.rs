//! JNI bridge for the platform-wallet persistence callback vtable.
//!
//! Kotlin counterpart: `org.dashfoundation.dashsdk.ffi.PersistenceNative`
//! + `NativePersistenceBridge`.
//!
//! [`Java_..._createCallbacks`] wraps a Kotlin `NativePersistenceBridge`
//! object in a heap-boxed [`platform_wallet_ffi::PersistenceCallbacks`]
//! whose 32 slots are filled with trampolines defined below, plus a boxed
//! [`KotlinPersistenceCtx`] holding the bridge as a JNI `GlobalRef`. Both
//! boxes are freed by [`Java_..._destroyCallbacks`].
//!
//! ## Trampoline contract (every slot)
//!
//! 1. **Attach the thread.** Persistence callbacks fire on Tokio worker
//!    threads that ART knows nothing about, so each trampoline attaches
//!    via `JavaVM::attach_current_thread_as_daemon()`. The daemon variant
//!    means we never have to detach (ART reaps the attachment when the
//!    thread dies), which is correct for the pooled Tokio workers.
//! 2. **Never unwind.** The whole body runs under `catch_unwind`; a panic
//!    returns a non-zero error code (persist slots) or a null / default
//!    (load slots) instead of unwinding across the C ABI (UB).
//! 3. **Copy before returning.** Every Rust-owned pointer payload
//!    (`*const u8`, `*const c_char`, nested slices) is copied into JVM
//!    objects (`byte[]` / `String` / boxed longs) before the trampoline
//!    returns — the FFI pointers are valid only for the callback window.
//! 4. **Fail safe.** On any JNI error the trampoline clears the pending
//!    exception and returns non-zero so the round's `success` flag flips
//!    and `on_changeset_end` delivers the rollback.
//!
//! ## Load-callback allocation scheme
//!
//! The four shielded loaders + `on_load_wallet_list_fn` +
//! `on_get_core_tx_record_fn` require Rust to read arrays the *callee*
//! allocates, paired with a free callback. The upstream docs say "the
//! callee allocates, the caller frees via the free callback" — since both
//! the loader trampoline AND the free trampoline are ours, we make the
//! shim allocate the FFI structs (`Box::into_raw` on a boxed slice) and
//! the paired free trampoline reconstruct+drop that exact `Box`. Kotlin
//! only ever returns flat holder objects; it never touches native memory.
//!
//! Ownership invariant on partial failure: a load trampoline stages every
//! JNI read in owned Rust values (`Vec`s) first and mints the raw buffers
//! the paired free trampoline expects only after the *entire* load has
//! succeeded. An `ERR_JNI` return (whose free trampoline never runs)
//! therefore never strands a Rust-owned buffer.

#![allow(clippy::missing_safety_doc)]

use crate::support::{guard, JVM};
use jni::objects::{GlobalRef, JByteArray, JClass, JObject, JString, JValue};
use jni::sys::jlong;
use jni::JNIEnv;
use platform_wallet_ffi::{
    AccountAddressPoolFFI, AccountChangeSetFFI, AccountSpecFFI, AddressBalanceEntryFFI,
    AssetLockEntryFFI, ContactIgnoredSenderFFI, ContactRequestFFI, ContactRequestRemovalFFI,
    CoreAddressEntryFFI, IdentityEntryFFI, IdentityKeyEntryFFI, IdentityKeyRemovalFFI,
    IdentityKeyRestoreFFI, IdentityRestoreEntryFFI, PersistenceCallbacks, PlatformAddressFFI,
    SpentOutPointFFI, TokenBalanceRemovalFFI, TokenBalanceUpsertFFI, TransactionRecordFFI,
    UtxoEntryFFI, UtxoRestoreEntryFFI, WalletChangeSetFFI, WalletRestoreEntryFFI,
};
use std::ffi::{c_void, CStr, CString};
use std::os::raw::c_char;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::ptr;

#[cfg(feature = "shielded")]
use platform_wallet_ffi::shielded_persistence::{
    ShieldedActivityFFI, ShieldedActivityRestoreFFI, ShieldedNoteFFI, ShieldedNoteRestoreFFI,
    ShieldedNullifierSpentFFI, ShieldedOutgoingNoteFFI, ShieldedOutgoingNoteRestoreFFI,
    ShieldedSubwalletSyncStateFFI, ShieldedSyncedIndexFFI,
};

// ── Context ───────────────────────────────────────────────────────────

/// Boxed context handed to every trampoline via `callbacks.context`.
/// Holds the Kotlin bridge as a `GlobalRef` so it survives across the
/// vtable's lifetime and across threads.
pub struct KotlinPersistenceCtx {
    pub(crate) bridge: GlobalRef,
}

impl KotlinPersistenceCtx {
    /// Box a Kotlin `NativePersistenceBridge` `GlobalRef` as the context a
    /// [`PersistenceCallbacks`] vtable points at. Shared by the standalone
    /// [`Java_..._createCallbacks`] export and the wallet-manager path
    /// (`wallet_manager.rs`), which builds the vtable inline so its
    /// context is owned by the manager for its lifetime.
    pub(crate) fn new(bridge: GlobalRef) -> Self {
        Self { bridge }
    }
}

// SAFETY: `GlobalRef` is Send + Sync (a JNI global reference is valid
// from any attached thread); the trampolines re-attach per call.
unsafe impl Send for KotlinPersistenceCtx {}
unsafe impl Sync for KotlinPersistenceCtx {}

// Bytecode names of the Kotlin bridge + load-holder classes. Documented
// here as the contract the trampolines resolve against; the trampolines
// themselves use JNI *virtual-method* dispatch on the bridge object and
// *field* reads on the holders (via their inline descriptors), so these
// names never need a `FindClass` and live only as reference.
//   NativePersistenceBridge   org/dashfoundation/dashsdk/ffi/NativePersistenceBridge
//   WalletRestoreData         org/dashfoundation/dashsdk/ffi/WalletRestoreData
//   AccountSpecData           org/dashfoundation/dashsdk/ffi/AccountSpecData
//   ShieldedNoteData          org/dashfoundation/dashsdk/ffi/ShieldedNoteData
//   ShieldedOutgoingNoteData  org/dashfoundation/dashsdk/ffi/ShieldedOutgoingNoteData
//   ShieldedSyncStateData     org/dashfoundation/dashsdk/ffi/ShieldedSyncStateData
//   ShieldedActivityData      org/dashfoundation/dashsdk/ffi/ShieldedActivityData
//   CoreTxRecordData          org/dashfoundation/dashsdk/ffi/CoreTxRecordData

// ── Exports ───────────────────────────────────────────────────────────

/// Build a native `PersistenceCallbacks` vtable delegating to the Kotlin
/// bridge object. Returns the boxed pointer as jlong, 0 on failure.
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_PersistenceNative_createCallbacks(
    mut env: JNIEnv,
    _class: JClass,
    bridge: JObject,
) -> jlong {
    guard(&mut env, 0, |env| {
        let global = match env.new_global_ref(&bridge) {
            Ok(g) => g,
            Err(_) => {
                crate::support::throw_sdk_exception(env, 99, "NewGlobalRef(bridge) failed");
                return 0;
            }
        };
        let ctx = Box::new(KotlinPersistenceCtx { bridge: global });
        let ctx_ptr = Box::into_raw(ctx) as *mut c_void;

        let callbacks = Box::new(build_vtable(ctx_ptr));
        Box::into_raw(callbacks) as jlong
    })
}

/// Free a vtable handle and its context `GlobalRef`. Safe on 0.
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_PersistenceNative_destroyCallbacks(
    mut env: JNIEnv,
    _class: JClass,
    handle: jlong,
) {
    guard(&mut env, (), |_| {
        if handle == 0 {
            return;
        }
        // SAFETY: `handle` is a pointer returned by `createCallbacks`; it
        // is not used after this call (Kotlin drops it). Reconstruct both
        // boxes so their `Drop` runs — the context box drops the
        // `GlobalRef`, releasing the Kotlin bridge.
        unsafe {
            let callbacks = Box::from_raw(handle as *mut PersistenceCallbacks);
            let ctx_ptr = callbacks.context as *mut KotlinPersistenceCtx;
            drop(callbacks);
            if !ctx_ptr.is_null() {
                drop(Box::from_raw(ctx_ptr));
            }
        }
    })
}

/// Assemble the full 32-slot vtable. `context` is the boxed
/// [`KotlinPersistenceCtx`] pointer.
pub(crate) fn build_vtable(context: *mut c_void) -> PersistenceCallbacks {
    PersistenceCallbacks {
        context,
        on_changeset_begin_fn: Some(tramp_changeset_begin),
        on_changeset_end_fn: Some(tramp_changeset_end),
        on_store_fn: Some(tramp_store),
        on_flush_fn: Some(tramp_flush),
        on_persist_address_balances_fn: Some(tramp_persist_address_balances),
        on_persist_wallet_changeset_fn: Some(tramp_persist_wallet_changeset),
        on_persist_sync_state_fn: Some(tramp_persist_sync_state),
        on_persist_account_registrations_fn: Some(tramp_persist_account_registrations),
        on_load_wallet_list_fn: Some(tramp_load_wallet_list),
        on_load_wallet_list_free_fn: Some(tramp_load_wallet_list_free),
        on_persist_wallet_metadata_fn: Some(tramp_persist_wallet_metadata),
        on_persist_account_address_pools_fn: Some(tramp_persist_account_address_pools),
        on_persist_identities_fn: Some(tramp_persist_identities),
        on_persist_identity_keys_fn: Some(tramp_persist_identity_keys),
        on_persist_token_balances_fn: Some(tramp_persist_token_balances),
        on_persist_contacts_fn: Some(tramp_persist_contacts),
        #[cfg(feature = "shielded")]
        on_persist_shielded_notes_fn: Some(tramp_persist_shielded_notes),
        #[cfg(feature = "shielded")]
        on_persist_shielded_nullifiers_spent_fn: Some(tramp_persist_shielded_nullifiers_spent),
        #[cfg(feature = "shielded")]
        on_persist_shielded_outgoing_notes_fn: Some(tramp_persist_shielded_outgoing_notes),
        #[cfg(feature = "shielded")]
        on_persist_shielded_synced_indices_fn: Some(tramp_persist_shielded_synced_indices),
        #[cfg(feature = "shielded")]
        on_persist_shielded_activity_fn: Some(tramp_persist_shielded_activity),
        #[cfg(feature = "shielded")]
        on_load_shielded_notes_fn: Some(tramp_load_shielded_notes),
        #[cfg(feature = "shielded")]
        on_load_shielded_notes_free_fn: Some(tramp_load_shielded_notes_free),
        #[cfg(feature = "shielded")]
        on_load_shielded_outgoing_notes_fn: Some(tramp_load_shielded_outgoing_notes),
        #[cfg(feature = "shielded")]
        on_load_shielded_outgoing_notes_free_fn: Some(tramp_load_shielded_outgoing_notes_free),
        #[cfg(feature = "shielded")]
        on_load_shielded_sync_states_fn: Some(tramp_load_shielded_sync_states),
        #[cfg(feature = "shielded")]
        on_load_shielded_sync_states_free_fn: Some(tramp_load_shielded_sync_states_free),
        #[cfg(feature = "shielded")]
        on_load_shielded_activity_fn: Some(tramp_load_shielded_activity),
        #[cfg(feature = "shielded")]
        on_load_shielded_activity_free_fn: Some(tramp_load_shielded_activity_free),
        on_get_core_tx_record_fn: Some(tramp_get_core_tx_record),
        on_get_core_tx_record_free_fn: Some(tramp_get_core_tx_record_free),
        on_persist_asset_locks_fn: Some(tramp_persist_asset_locks),
    }
}

// ── Attach / invoke helpers ───────────────────────────────────────────

/// Error code returned by a persist trampoline when the JVM side cannot
/// be reached or a JNI call fails. Non-zero flips the round's success
/// flag so `on_changeset_end` rolls back.
const ERR_JNI: i32 = 1;

/// Attach the current (Tokio) thread and hand `f` the env + the bridge
/// object. Returns `ERR_JNI` on attach failure. Any pending exception
/// left by `f` is cleared and `ERR_JNI` returned.
///
/// # Safety
/// `context` must be a live `KotlinPersistenceCtx` pointer produced by
/// `createCallbacks`.
unsafe fn with_bridge<F>(context: *mut c_void, f: F) -> i32
where
    F: FnOnce(&mut JNIEnv, &JObject) -> Result<i32, jni::errors::Error>,
{
    let result = catch_unwind(AssertUnwindSafe(|| {
        let Some(vm) = JVM.get() else {
            return ERR_JNI;
        };
        // jni 0.21's `attach_current_thread_as_daemon` hands back an owned
        // `JNIEnv` (the daemon attachment is reaped by ART on thread exit,
        // so there is no guard to keep alive / detach explicitly).
        let Ok(mut env) = vm.attach_current_thread_as_daemon() else {
            return ERR_JNI;
        };
        let ctx = &*(context as *const KotlinPersistenceCtx);
        let bridge = ctx.bridge.as_obj();
        let env: &mut JNIEnv = &mut env;
        match env.with_local_frame(64, |env| f(env, bridge)) {
            Ok(code) => code,
            Err(_) => {
                let _ = env.exception_clear();
                ERR_JNI
            }
        }
    }));
    result.unwrap_or(ERR_JNI)
}

/// Like [`with_bridge`] but for load callbacks: returns `f`'s value or
/// `None` on any failure/panic, and clears pending exceptions.
///
/// # Safety
/// Same as [`with_bridge`].
unsafe fn with_bridge_load<T, F>(context: *mut c_void, f: F) -> Option<T>
where
    F: FnOnce(&mut JNIEnv, &JObject) -> Result<T, jni::errors::Error>,
{
    let result = catch_unwind(AssertUnwindSafe(|| {
        let vm = JVM.get()?;
        let mut env = vm.attach_current_thread_as_daemon().ok()?;
        let ctx = &*(context as *const KotlinPersistenceCtx);
        let bridge = ctx.bridge.as_obj();
        let env: &mut JNIEnv = &mut env;
        match env.with_local_frame(64, |env| f(env, bridge)) {
            Ok(v) => Some(v),
            Err(_) => {
                let _ = env.exception_clear();
                None
            }
        }
    }));
    result.unwrap_or(None)
}

/// Copy a 32-byte id pointer into a JVM `byte[]`.
fn id32<'l>(env: &JNIEnv<'l>, ptr: *const u8) -> Result<JByteArray<'l>, jni::errors::Error> {
    bytes(env, ptr, 32)
}

/// Copy `len` bytes from `ptr` (or 0 bytes when null) into a JVM `byte[]`.
fn bytes<'l>(
    env: &JNIEnv<'l>,
    ptr: *const u8,
    len: usize,
) -> Result<JByteArray<'l>, jni::errors::Error> {
    if ptr.is_null() || len == 0 {
        return env.new_byte_array(0);
    }
    // SAFETY: caller guarantees `ptr` is readable for `len` bytes for the
    // callback window.
    let slice = unsafe { std::slice::from_raw_parts(ptr, len) };
    env.byte_array_from_slice(slice)
}

/// Copy a NUL-terminated C string into a JVM `String`; null → "".
fn cstr<'l>(env: &JNIEnv<'l>, ptr: *const c_char) -> Result<JString<'l>, jni::errors::Error> {
    if ptr.is_null() {
        return env.new_string("");
    }
    // SAFETY: caller guarantees the string is valid for the callback window.
    let s = unsafe { CStr::from_ptr(ptr) }.to_string_lossy();
    env.new_string(s)
}

/// Copy a NUL-terminated C string into an optional JVM `String`; null → JVM null.
fn cstr_opt<'l>(env: &JNIEnv<'l>, ptr: *const c_char) -> Result<JObject<'l>, jni::errors::Error> {
    if ptr.is_null() {
        return Ok(JObject::null());
    }
    let s = unsafe { CStr::from_ptr(ptr) }.to_string_lossy();
    Ok(env.new_string(s)?.into())
}

/// Copy `len` bytes into an optional JVM `byte[]`; null → JVM null.
fn bytes_opt<'l>(
    env: &JNIEnv<'l>,
    ptr: *const u8,
    len: usize,
) -> Result<JObject<'l>, jni::errors::Error> {
    if ptr.is_null() {
        return Ok(JObject::null());
    }
    Ok(bytes(env, ptr, len)?.into())
}

// ── Bracketing + notification slots ───────────────────────────────────

unsafe extern "C" fn tramp_changeset_begin(context: *mut c_void, wallet_id: *const u8) -> i32 {
    with_bridge(context, |env, bridge| {
        let wid = id32(env, wallet_id)?;
        env.call_method(bridge, "onChangesetBegin", "([B)I", &[(&wid).into()])?
            .i()
    })
}

unsafe extern "C" fn tramp_changeset_end(
    context: *mut c_void,
    wallet_id: *const u8,
    success: bool,
) -> i32 {
    with_bridge(context, |env, bridge| {
        let wid = id32(env, wallet_id)?;
        env.call_method(
            bridge,
            "onChangesetEnd",
            "([BZ)I",
            &[(&wid).into(), JValue::Bool(success as u8)],
        )?
        .i()
    })
}

unsafe extern "C" fn tramp_store(context: *mut c_void, wallet_id: *const u8) -> i32 {
    with_bridge(context, |env, bridge| {
        let wid = id32(env, wallet_id)?;
        env.call_method(bridge, "onStore", "([B)I", &[(&wid).into()])?
            .i()
    })
}

unsafe extern "C" fn tramp_flush(context: *mut c_void, wallet_id: *const u8) -> i32 {
    with_bridge(context, |env, bridge| {
        let wid = id32(env, wallet_id)?;
        env.call_method(bridge, "onFlush", "([B)I", &[(&wid).into()])?
            .i()
    })
}

// ── Address balances ──────────────────────────────────────────────────

unsafe extern "C" fn tramp_persist_address_balances(
    context: *mut c_void,
    wallet_id: *const u8,
    entries: *const AddressBalanceEntryFFI,
    count: usize,
) -> i32 {
    with_bridge(context, |env, bridge| {
        let wid = id32(env, wallet_id)?;
        let slice = slice_or_empty(entries, count);
        for e in slice {
            let code = env.with_local_frame(16, |env| {
                let PlatformAddressFFI { address_type, hash } = e.address;
                let hash_arr = env.byte_array_from_slice(&hash)?;
                env.call_method(
                    bridge,
                    "onPersistAddressBalance",
                    "([BB[BJIIIJ)I",
                    &[
                        (&wid).into(),
                        JValue::Byte(address_type as i8),
                        (&hash_arr).into(),
                        JValue::Long(e.balance as i64),
                        JValue::Int(e.nonce as i32),
                        JValue::Int(e.account_index as i32),
                        JValue::Int(e.address_index as i32),
                        JValue::Long(e.as_of_height as i64),
                    ],
                )?
                .i()
            })?;
            if code != 0 {
                return Ok(code);
            }
        }
        Ok(0)
    })
}

// ── Sync state ────────────────────────────────────────────────────────

unsafe extern "C" fn tramp_persist_sync_state(
    context: *mut c_void,
    wallet_id: *const u8,
    sync_height: u64,
    sync_timestamp: u64,
    last_known_recent_block: u64,
) -> i32 {
    with_bridge(context, |env, bridge| {
        let wid = id32(env, wallet_id)?;
        env.call_method(
            bridge,
            "onPersistSyncState",
            "([BJJJ)I",
            &[
                (&wid).into(),
                JValue::Long(sync_height as i64),
                JValue::Long(sync_timestamp as i64),
                JValue::Long(last_known_recent_block as i64),
            ],
        )?
        .i()
    })
}

// ── Wallet metadata ───────────────────────────────────────────────────

unsafe extern "C" fn tramp_persist_wallet_metadata(
    context: *mut c_void,
    wallet_id: *const u8,
    network: platform_wallet_ffi::FFINetwork,
    wallet_group_id: *const u8,
    birth_height: u32,
) -> i32 {
    with_bridge(context, |env, bridge| {
        let wid = id32(env, wallet_id)?;
        let gid = id32(env, wallet_group_id)?;
        env.call_method(
            bridge,
            "onPersistWalletMetadata",
            "([BI[BI)I",
            &[
                (&wid).into(),
                JValue::Int(network as i32),
                (&gid).into(),
                JValue::Int(birth_height as i32),
            ],
        )?
        .i()
    })
}

// ── Account registrations ─────────────────────────────────────────────

unsafe extern "C" fn tramp_persist_account_registrations(
    context: *mut c_void,
    wallet_id: *const u8,
    specs: *const AccountSpecFFI,
    count: usize,
) -> i32 {
    with_bridge(context, |env, bridge| {
        let wid = id32(env, wallet_id)?;
        for s in slice_or_empty(specs, count) {
            let code = env.with_local_frame(16, |env| {
                call_persist_account_registration(env, bridge, &wid, s)
            })?;
            if code != 0 {
                return Ok(code);
            }
        }
        Ok(0)
    })
}

fn call_persist_account_registration(
    env: &mut JNIEnv,
    bridge: &JObject,
    wid: &JByteArray,
    s: &AccountSpecFFI,
) -> Result<i32, jni::errors::Error> {
    let user = env.byte_array_from_slice(&s.user_identity_id)?;
    let friend = env.byte_array_from_slice(&s.friend_identity_id)?;
    let xpub = bytes(env, s.account_xpub_bytes, s.account_xpub_bytes_len)?;
    env.call_method(
        bridge,
        "onPersistAccountRegistration",
        "([BBBIII[B[B[B)I",
        &[
            wid.into(),
            JValue::Byte(s.type_tag as i8),
            JValue::Byte(s.standard_tag as i8),
            JValue::Int(s.index as i32),
            JValue::Int(s.registration_index as i32),
            JValue::Int(s.key_class as i32),
            (&user).into(),
            (&friend).into(),
            (&xpub).into(),
        ],
    )?
    .i()
}

// ── Account address pools ─────────────────────────────────────────────

unsafe extern "C" fn tramp_persist_account_address_pools(
    context: *mut c_void,
    wallet_id: *const u8,
    pools: *const AccountAddressPoolFFI,
    count: usize,
) -> i32 {
    with_bridge(context, |env, bridge| {
        let wid = id32(env, wallet_id)?;
        for pool in slice_or_empty(pools, count) {
            let spec = &pool.account;
            let addrs = slice_or_empty(pool.addresses_ptr, pool.addresses_count);
            for a in addrs {
                let code = env.with_local_frame(32, |env| {
                    let user = env.byte_array_from_slice(&spec.user_identity_id)?;
                    let friend = env.byte_array_from_slice(&spec.friend_identity_id)?;
                    let pubkey = env.byte_array_from_slice(&a.public_key)?;
                    let base58 = cstr(env, a.address_base58)?;
                    let path = cstr(env, a.derivation_path)?;
                    env.call_method(
                        bridge,
                        "onPersistAccountAddressPoolEntry",
                        "([BBBIII[B[BB[BZBIZJLjava/lang/String;Ljava/lang/String;)I",
                        &[
                            (&wid).into(),
                            JValue::Byte(spec.type_tag as i8),
                            JValue::Byte(spec.standard_tag as i8),
                            JValue::Int(spec.index as i32),
                            JValue::Int(spec.registration_index as i32),
                            JValue::Int(spec.key_class as i32),
                            (&user).into(),
                            (&friend).into(),
                            JValue::Byte(pool.pool_type_tag as i8),
                            (&pubkey).into(),
                            JValue::Bool(a.has_public_key as u8),
                            JValue::Byte(a.pool_type_tag as i8),
                            JValue::Int(a.address_index as i32),
                            JValue::Bool(a.is_used as u8),
                            JValue::Long(a.balance as i64),
                            (&base58).into(),
                            (&path).into(),
                        ],
                    )?
                    .i()
                })?;
                if code != 0 {
                    return Ok(code);
                }
            }
        }
        Ok(0)
    })
}

// ── Wallet (core) changeset ───────────────────────────────────────────

unsafe extern "C" fn tramp_persist_wallet_changeset(
    context: *mut c_void,
    wallet_id: *const u8,
    changeset: *const WalletChangeSetFFI,
) -> i32 {
    with_bridge(context, |env, bridge| {
        if changeset.is_null() {
            return Ok(0);
        }
        let cs = &*changeset;
        let wid = id32(env, wallet_id)?;

        // Header (chain + balance + chainlock bytes).
        let (synced_height, has_synced) = if cs.has_chain {
            (cs.chain.synced_height, cs.chain.has_synced_height)
        } else {
            (0, false)
        };
        let cl_bytes = bytes(
            env,
            cs.last_applied_chain_lock_bytes,
            cs.last_applied_chain_lock_bytes_len,
        )?;
        let code = env
            .call_method(
                bridge,
                "onWalletChangesetHeader",
                "([BZIZJJJJ[B)I",
                &[
                    (&wid).into(),
                    JValue::Bool(has_synced as u8),
                    JValue::Int(synced_height as i32),
                    JValue::Bool(cs.has_balance as u8),
                    JValue::Long(cs.balance.confirmed_delta),
                    JValue::Long(cs.balance.unconfirmed_delta),
                    JValue::Long(cs.balance.immature_delta),
                    JValue::Long(cs.balance.locked_delta),
                    (&cl_bytes).into(),
                ],
            )?
            .i()?;
        if code != 0 {
            return Ok(code);
        }

        for acc in slice_or_empty(cs.accounts, cs.accounts_count) {
            let code =
                env.with_local_frame(32, |env| persist_changeset_account(env, bridge, &wid, acc))?;
            if code != 0 {
                return Ok(code);
            }
        }
        Ok(0)
    })
}

unsafe fn persist_changeset_account(
    env: &mut JNIEnv,
    bridge: &JObject,
    wid: &JByteArray,
    acc: &AccountChangeSetFFI,
) -> Result<i32, jni::errors::Error> {
    let user = env.byte_array_from_slice(&acc.user_identity_id)?;
    let friend = env.byte_array_from_slice(&acc.friend_identity_id)?;
    let code = env
        .call_method(
            bridge,
            "onWalletChangesetAccountBegin",
            "([BIBBII[B[BIZIZ)I",
            &[
                wid.into(),
                JValue::Int(acc.account_index as i32),
                JValue::Byte(acc.type_tag as u8 as i8),
                JValue::Byte(acc.standard_tag as u8 as i8),
                JValue::Int(acc.registration_index as i32),
                JValue::Int(acc.key_class as i32),
                (&user).into(),
                (&friend).into(),
                JValue::Int(acc.external_highest_used),
                JValue::Bool(acc.has_external_highest_used as u8),
                JValue::Int(acc.internal_highest_used),
                JValue::Bool(acc.has_internal_highest_used as u8),
            ],
        )?
        .i()?;
    if code != 0 {
        return Ok(code);
    }

    for u in slice_or_empty(acc.utxos_added, acc.utxos_added_count) {
        let code =
            env.with_local_frame(24, |env| persist_changeset_utxo_added(env, bridge, wid, u))?;
        if code != 0 {
            return Ok(code);
        }
    }
    for s in slice_or_empty(acc.utxos_spent, acc.utxos_spent_count) {
        let code =
            env.with_local_frame(16, |env| persist_changeset_utxo_spent(env, bridge, wid, s))?;
        if code != 0 {
            return Ok(code);
        }
    }
    for t in slice_or_empty(acc.transactions, acc.transactions_count) {
        let code =
            env.with_local_frame(32, |env| persist_changeset_transaction(env, bridge, wid, t))?;
        if code != 0 {
            return Ok(code);
        }
    }

    env.call_method(
        bridge,
        "onWalletChangesetAccountEnd",
        "([BI)I",
        &[wid.into(), JValue::Int(acc.account_index as i32)],
    )?
    .i()
}

unsafe fn persist_changeset_utxo_added(
    env: &mut JNIEnv,
    bridge: &JObject,
    wid: &JByteArray,
    u: &UtxoEntryFFI,
) -> Result<i32, jni::errors::Error> {
    let txid = env.byte_array_from_slice(&u.outpoint.txid)?;
    let addr = cstr(env, u.address)?;
    let spk = bytes(env, u.script_pubkey, u.script_pubkey_len)?;
    env.call_method(
        bridge,
        "onWalletChangesetUtxoAdded",
        "([B[BIJLjava/lang/String;[BIZZZZ)I",
        &[
            wid.into(),
            (&txid).into(),
            JValue::Int(u.outpoint.vout as i32),
            JValue::Long(u.amount as i64),
            (&addr).into(),
            (&spk).into(),
            JValue::Int(u.height as i32),
            JValue::Bool(u.is_coinbase as u8),
            JValue::Bool(u.is_confirmed as u8),
            JValue::Bool(u.is_instantlocked as u8),
            JValue::Bool(u.is_locked as u8),
        ],
    )?
    .i()
}

unsafe fn persist_changeset_utxo_spent(
    env: &mut JNIEnv,
    bridge: &JObject,
    wid: &JByteArray,
    s: &SpentOutPointFFI,
) -> Result<i32, jni::errors::Error> {
    let txid = env.byte_array_from_slice(&s.outpoint.txid)?;
    let spending = env.byte_array_from_slice(&s.spending_txid)?;
    env.call_method(
        bridge,
        "onWalletChangesetUtxoSpent",
        "([B[BI[B)I",
        &[
            wid.into(),
            (&txid).into(),
            JValue::Int(s.outpoint.vout as i32),
            (&spending).into(),
        ],
    )?
    .i()
}

unsafe fn persist_changeset_transaction(
    env: &mut JNIEnv,
    bridge: &JObject,
    wid: &JByteArray,
    t: &TransactionRecordFFI,
) -> Result<i32, jni::errors::Error> {
    let txid = env.byte_array_from_slice(&t.txid)?;
    let tx_data = bytes(env, t.tx_data, t.tx_data_len)?;
    let block_hash = env.byte_array_from_slice(&t.block_hash)?;
    let tx_type = cstr(env, t.transaction_type)?;
    let label = cstr(env, t.label)?;
    env.call_method(
        bridge,
        "onWalletChangesetTransaction",
        "([B[B[BII[BIILjava/lang/String;IJJZLjava/lang/String;J)I",
        &[
            wid.into(),
            (&txid).into(),
            (&tx_data).into(),
            JValue::Int(t.context as i32),
            JValue::Int(t.block_height as i32),
            (&block_hash).into(),
            JValue::Int(t.block_timestamp as i32),
            JValue::Int(t.direction as i32),
            (&tx_type).into(),
            JValue::Int(t.transaction_type_kind as i32),
            JValue::Long(t.net_amount),
            JValue::Long(t.fee as i64),
            JValue::Bool(t.has_fee as u8),
            (&label).into(),
            JValue::Long(t.first_seen as i64),
        ],
    )?
    .i()
}

// ── Identities ────────────────────────────────────────────────────────

unsafe extern "C" fn tramp_persist_identities(
    context: *mut c_void,
    wallet_id: *const u8,
    upserts_ptr: *const IdentityEntryFFI,
    upserts_count: usize,
    removed_ptr: *const [u8; 32],
    removed_count: usize,
) -> i32 {
    with_bridge(context, |env, bridge| {
        let wid = id32(env, wallet_id)?;
        for e in slice_or_empty(upserts_ptr, upserts_count) {
            let code =
                env.with_local_frame(64, |env| persist_identity_upsert(env, bridge, &wid, e))?;
            if code != 0 {
                return Ok(code);
            }
        }
        for id in slice_or_empty(removed_ptr, removed_count) {
            let code = env.with_local_frame(16, |env| {
                let idb = env.byte_array_from_slice(id)?;
                env.call_method(
                    bridge,
                    "onPersistIdentityRemoval",
                    "([B[B)I",
                    &[(&wid).into(), (&idb).into()],
                )?
                .i()
            })?;
            if code != 0 {
                return Ok(code);
            }
        }
        Ok(0)
    })
}

unsafe fn persist_identity_upsert(
    env: &mut JNIEnv,
    bridge: &JObject,
    wid: &JByteArray,
    e: &IdentityEntryFFI,
) -> Result<i32, jni::errors::Error> {
    let identity_id = env.byte_array_from_slice(&e.identity_id)?;
    let identity_wallet_id = env.byte_array_from_slice(&e.wallet_id)?;

    // DPNS labels: *const *const c_char → String[]; acquired-at parallel u64[].
    let string_cls = env.find_class("java/lang/String")?;
    let empty = env.new_string("")?;
    let dpns_arr = env.new_object_array(e.dpns_names_count as i32, &string_cls, &empty)?;
    let mut acquired: Vec<i64> = Vec::with_capacity(e.dpns_names_count);
    if !e.dpns_names.is_null() {
        let name_ptrs = std::slice::from_raw_parts(e.dpns_names, e.dpns_names_count);
        for (i, &nptr) in name_ptrs.iter().enumerate() {
            env.with_local_frame(4, |env| {
                let s = cstr(env, nptr)?;
                env.set_object_array_element(&dpns_arr, i as i32, &s)
            })?;
        }
    }
    if !e.dpns_names_acquired_at.is_null() {
        let ts = std::slice::from_raw_parts(e.dpns_names_acquired_at, e.dpns_names_count);
        acquired.extend(ts.iter().map(|&t| t as i64));
    } else {
        acquired.resize(e.dpns_names_count, 0);
    }
    let acquired_arr = env.new_long_array(e.dpns_names_count as i32)?;
    if !acquired.is_empty() {
        env.set_long_array_region(&acquired_arr, 0, &acquired)?;
    }

    let display = cstr_opt(env, e.dashpay_profile_display_name)?;
    let bio = cstr_opt(env, e.dashpay_profile_bio)?;
    let avatar_url = cstr_opt(env, e.dashpay_profile_avatar_url)?;
    let public_message = cstr_opt(env, e.dashpay_profile_public_message)?;
    let avatar_hash = env.byte_array_from_slice(&e.dashpay_profile_avatar_hash)?;
    let avatar_fp = env.byte_array_from_slice(&e.dashpay_profile_avatar_fingerprint)?;

    env.call_method(
        bridge,
        "onPersistIdentityUpsert",
        "([B[BJJZIBZ[B[Ljava/lang/String;[JZLjava/lang/String;Ljava/lang/String;\
         Ljava/lang/String;[BZ[BZLjava/lang/String;)I",
        &[
            wid.into(),
            (&identity_id).into(),
            JValue::Long(e.balance as i64),
            JValue::Long(e.revision as i64),
            JValue::Bool(e.identity_index_is_some as u8),
            JValue::Int(e.identity_index as i32),
            JValue::Byte(e.status as i8),
            JValue::Bool(e.wallet_id_is_some as u8),
            (&identity_wallet_id).into(),
            (&dpns_arr).into(),
            (&acquired_arr).into(),
            JValue::Bool(e.dashpay_profile_present as u8),
            (&display).into(),
            (&bio).into(),
            (&avatar_url).into(),
            (&avatar_hash).into(),
            JValue::Bool(e.dashpay_profile_avatar_hash_present as u8),
            (&avatar_fp).into(),
            JValue::Bool(e.dashpay_profile_avatar_fingerprint_present as u8),
            (&public_message).into(),
        ],
    )?
    .i()
}

// ── Identity keys ─────────────────────────────────────────────────────

unsafe extern "C" fn tramp_persist_identity_keys(
    context: *mut c_void,
    wallet_id: *const u8,
    upserts_ptr: *const IdentityKeyEntryFFI,
    upserts_count: usize,
    removed_ptr: *const IdentityKeyRemovalFFI,
    removed_count: usize,
) -> i32 {
    with_bridge(context, |env, bridge| {
        let wid = id32(env, wallet_id)?;
        for e in slice_or_empty(upserts_ptr, upserts_count) {
            let code =
                env.with_local_frame(32, |env| persist_identity_key_upsert(env, bridge, &wid, e))?;
            if code != 0 {
                return Ok(code);
            }
        }
        for r in slice_or_empty(removed_ptr, removed_count) {
            let code = env.with_local_frame(16, |env| {
                let idb = env.byte_array_from_slice(&r.identity_id)?;
                env.call_method(
                    bridge,
                    "onPersistIdentityKeyRemoval",
                    "([B[BI)I",
                    &[(&wid).into(), (&idb).into(), JValue::Int(r.key_id as i32)],
                )?
                .i()
            })?;
            if code != 0 {
                return Ok(code);
            }
        }
        Ok(0)
    })
}

unsafe fn persist_identity_key_upsert(
    env: &mut JNIEnv,
    bridge: &JObject,
    wid: &JByteArray,
    e: &IdentityKeyEntryFFI,
) -> Result<i32, jni::errors::Error> {
    let identity_id = env.byte_array_from_slice(&e.identity_id)?;
    let pk_data = bytes(env, e.public_key_data_ptr, e.public_key_data_len)?;
    let pk_hash = env.byte_array_from_slice(&e.public_key_hash)?;
    let key_wallet_id = env.byte_array_from_slice(&e.wallet_id)?;
    let cb_id = env.byte_array_from_slice(&e.contract_bounds_id)?;
    let cb_doctype = cstr_opt(env, e.contract_bounds_document_type)?;
    env.call_method(
        bridge,
        "onPersistIdentityKeyUpsert",
        "([B[BIBBBZZJ[B[BZ[BZIIB[BLjava/lang/String;)I",
        &[
            wid.into(),
            (&identity_id).into(),
            JValue::Int(e.key_id as i32),
            JValue::Byte(e.purpose as i8),
            JValue::Byte(e.security_level as i8),
            JValue::Byte(e.key_type as i8),
            JValue::Bool(e.read_only as u8),
            JValue::Bool(e.disabled_at_is_some as u8),
            JValue::Long(e.disabled_at as i64),
            (&pk_data).into(),
            (&pk_hash).into(),
            JValue::Bool(e.wallet_id_is_some as u8),
            (&key_wallet_id).into(),
            JValue::Bool(e.derivation_indices_is_some as u8),
            JValue::Int(e.identity_index as i32),
            JValue::Int(e.key_index as i32),
            JValue::Byte(e.contract_bounds_kind as i8),
            (&cb_id).into(),
            (&cb_doctype).into(),
        ],
    )?
    .i()
}

// ── Token balances ────────────────────────────────────────────────────

unsafe extern "C" fn tramp_persist_token_balances(
    context: *mut c_void,
    wallet_id: *const u8,
    upserts_ptr: *const TokenBalanceUpsertFFI,
    upserts_count: usize,
    removed_ptr: *const TokenBalanceRemovalFFI,
    removed_count: usize,
) -> i32 {
    with_bridge(context, |env, bridge| {
        let wid = id32(env, wallet_id)?;
        for u in slice_or_empty(upserts_ptr, upserts_count) {
            let code = env.with_local_frame(16, |env| {
                let iid = env.byte_array_from_slice(&u.identity_id)?;
                let tid = env.byte_array_from_slice(&u.token_id)?;
                env.call_method(
                    bridge,
                    "onPersistTokenBalanceUpsert",
                    "([B[B[BJ)I",
                    &[
                        (&wid).into(),
                        (&iid).into(),
                        (&tid).into(),
                        JValue::Long(u.balance as i64),
                    ],
                )?
                .i()
            })?;
            if code != 0 {
                return Ok(code);
            }
        }
        for r in slice_or_empty(removed_ptr, removed_count) {
            let code = env.with_local_frame(16, |env| {
                let iid = env.byte_array_from_slice(&r.identity_id)?;
                let tid = env.byte_array_from_slice(&r.token_id)?;
                env.call_method(
                    bridge,
                    "onPersistTokenBalanceRemoval",
                    "([B[B[B)I",
                    &[(&wid).into(), (&iid).into(), (&tid).into()],
                )?
                .i()
            })?;
            if code != 0 {
                return Ok(code);
            }
        }
        Ok(0)
    })
}

// ── Contacts ──────────────────────────────────────────────────────────

unsafe extern "C" fn tramp_persist_contacts(
    context: *mut c_void,
    wallet_id: *const u8,
    upserts_ptr: *const ContactRequestFFI,
    upserts_count: usize,
    removed_sent_ptr: *const ContactRequestRemovalFFI,
    removed_sent_count: usize,
    removed_incoming_ptr: *const ContactRequestRemovalFFI,
    removed_incoming_count: usize,
    ignored_ptr: *const ContactIgnoredSenderFFI,
    ignored_count: usize,
) -> i32 {
    with_bridge(context, |env, bridge| {
        let wid = id32(env, wallet_id)?;
        for c in slice_or_empty(upserts_ptr, upserts_count) {
            let code =
                env.with_local_frame(48, |env| persist_contact_upsert(env, bridge, &wid, c))?;
            if code != 0 {
                return Ok(code);
            }
        }
        for r in slice_or_empty(removed_sent_ptr, removed_sent_count) {
            let code = env.with_local_frame(16, |env| {
                persist_contact_removal(env, bridge, &wid, r, "onPersistContactRemovalSent")
            })?;
            if code != 0 {
                return Ok(code);
            }
        }
        for r in slice_or_empty(removed_incoming_ptr, removed_incoming_count) {
            let code = env.with_local_frame(16, |env| {
                persist_contact_removal(env, bridge, &wid, r, "onPersistContactRemovalIncoming")
            })?;
            if code != 0 {
                return Ok(code);
            }
        }
        // Per-sender ignore deltas: `is_ignored == true` persists the
        // ignored-sender row (an ignore); `false` deletes it (an
        // un-ignore). POD rows — the Kotlin handler copies what it keeps.
        for g in slice_or_empty(ignored_ptr, ignored_count) {
            let code = env.with_local_frame(16, |env| {
                let owner = env.byte_array_from_slice(&g.owner_id)?;
                let sender = env.byte_array_from_slice(&g.sender_id)?;
                env.call_method(
                    bridge,
                    "onPersistContactIgnored",
                    "([B[B[BZ)I",
                    &[
                        (&wid).into(),
                        (&owner).into(),
                        (&sender).into(),
                        JValue::Bool(g.is_ignored as u8),
                    ],
                )?
                .i()
            })?;
            if code != 0 {
                return Ok(code);
            }
        }
        Ok(0)
    })
}

unsafe fn persist_contact_upsert(
    env: &mut JNIEnv,
    bridge: &JObject,
    wid: &JByteArray,
    c: &ContactRequestFFI,
) -> Result<i32, jni::errors::Error> {
    let owner = env.byte_array_from_slice(&c.owner_id)?;
    let contact = env.byte_array_from_slice(&c.contact_id)?;
    let epk = bytes(env, c.encrypted_public_key, c.encrypted_public_key_len)?;
    let label = bytes_opt(
        env,
        c.encrypted_account_label,
        c.encrypted_account_label_len,
    )?;
    let proof = bytes_opt(env, c.auto_accept_proof, c.auto_accept_proof_len)?;
    // Established-row metadata (contactInfo alias/note/hidden, the
    // contact's decrypted account label, the broken-channel flag and the
    // DIP-15 accepted accounts) — null / false / empty on pending rows.
    let alias = cstr_opt(env, c.alias)?;
    let note = cstr_opt(env, c.note)?;
    let contact_account_label = cstr_opt(env, c.contact_account_label)?;
    let accepted = int_array(env, c.accepted_accounts, c.accepted_accounts_len)?;
    env.call_method(
        bridge,
        "onPersistContactUpsert",
        "([B[B[BZIII[B[B[BIJZLjava/lang/String;Ljava/lang/String;ZLjava/lang/String;[I)I",
        &[
            wid.into(),
            (&owner).into(),
            (&contact).into(),
            JValue::Bool(c.is_outgoing as u8),
            JValue::Int(c.sender_key_index as i32),
            JValue::Int(c.recipient_key_index as i32),
            JValue::Int(c.account_reference as i32),
            (&epk).into(),
            (&label).into(),
            (&proof).into(),
            JValue::Int(c.core_height_created_at as i32),
            JValue::Long(c.created_at as i64),
            JValue::Bool(c.payment_channel_broken as u8),
            (&alias).into(),
            (&note).into(),
            JValue::Bool(c.is_hidden as u8),
            (&contact_account_label).into(),
            (&accepted).into(),
        ],
    )?
    .i()
}

/// Copy `len` `u32`s from `ptr` (or 0 when null) into a JVM `int[]`
/// (bit-pattern cast — DIP-15 account indices never exceed `i32::MAX`
/// in practice, and the Kotlin side reads them back with the same cast).
fn int_array<'l>(
    env: &JNIEnv<'l>,
    ptr: *const u32,
    len: usize,
) -> Result<jni::objects::JIntArray<'l>, jni::errors::Error> {
    let arr = env.new_int_array(len as i32)?;
    if !ptr.is_null() && len > 0 {
        // SAFETY: caller guarantees `ptr` points to `len` u32s for the
        // callback window.
        let values: Vec<i32> = unsafe { std::slice::from_raw_parts(ptr, len) }
            .iter()
            .map(|&v| v as i32)
            .collect();
        env.set_int_array_region(&arr, 0, &values)?;
    }
    Ok(arr)
}

unsafe fn persist_contact_removal(
    env: &mut JNIEnv,
    bridge: &JObject,
    wid: &JByteArray,
    r: &ContactRequestRemovalFFI,
    method: &str,
) -> Result<i32, jni::errors::Error> {
    let owner = env.byte_array_from_slice(&r.owner_id)?;
    let contact = env.byte_array_from_slice(&r.contact_id)?;
    env.call_method(
        bridge,
        method,
        "([B[B[B)I",
        &[wid.into(), (&owner).into(), (&contact).into()],
    )?
    .i()
}

// ── Asset locks ───────────────────────────────────────────────────────

unsafe extern "C" fn tramp_persist_asset_locks(
    context: *mut c_void,
    wallet_id: *const u8,
    upserts_ptr: *const AssetLockEntryFFI,
    upserts_count: usize,
    removed_ptr: *const [u8; 36],
    removed_count: usize,
) -> i32 {
    with_bridge(context, |env, bridge| {
        let wid = id32(env, wallet_id)?;
        for e in slice_or_empty(upserts_ptr, upserts_count) {
            let code = env.with_local_frame(24, |env| {
                let outpoint = env.byte_array_from_slice(&e.out_point)?;
                let tx = bytes(env, e.transaction_bytes, e.transaction_bytes_len)?;
                let proof = bytes_opt(env, e.proof_bytes, e.proof_bytes_len)?;
                env.call_method(
                    bridge,
                    "onPersistAssetLockUpsert",
                    "([B[B[BIBIJB[B)I",
                    &[
                        (&wid).into(),
                        (&outpoint).into(),
                        (&tx).into(),
                        JValue::Int(e.account_index as i32),
                        JValue::Byte(e.funding_type as i8),
                        JValue::Int(e.identity_index as i32),
                        JValue::Long(e.amount_duffs as i64),
                        JValue::Byte(e.status as i8),
                        (&proof).into(),
                    ],
                )?
                .i()
            })?;
            if code != 0 {
                return Ok(code);
            }
        }
        for op in slice_or_empty(removed_ptr, removed_count) {
            let code = env.with_local_frame(16, |env| {
                let opb = env.byte_array_from_slice(op)?;
                env.call_method(
                    bridge,
                    "onPersistAssetLockRemoval",
                    "([B[B)I",
                    &[(&wid).into(), (&opb).into()],
                )?
                .i()
            })?;
            if code != 0 {
                return Ok(code);
            }
        }
        Ok(0)
    })
}

// ── Shielded persist ──────────────────────────────────────────────────

#[cfg(feature = "shielded")]
unsafe extern "C" fn tramp_persist_shielded_notes(
    context: *mut c_void,
    wallet_id: *const u8,
    entries: *const ShieldedNoteFFI,
    count: usize,
) -> i32 {
    with_bridge(context, |env, bridge| {
        let wid = id32(env, wallet_id)?;
        for e in slice_or_empty(entries, count) {
            let code = env.with_local_frame(24, |env| {
                let nwid = env.byte_array_from_slice(&e.wallet_id)?;
                let cmx = env.byte_array_from_slice(&e.cmx)?;
                let nullifier = env.byte_array_from_slice(&e.nullifier)?;
                let note_data = bytes(env, e.note_data_ptr, e.note_data_len)?;
                env.call_method(
                    bridge,
                    "onPersistShieldedNote",
                    "([B[BIJ[B[BJBJ[B)I",
                    &[
                        (&wid).into(),
                        (&nwid).into(),
                        JValue::Int(e.account_index as i32),
                        JValue::Long(e.position as i64),
                        (&cmx).into(),
                        (&nullifier).into(),
                        JValue::Long(e.block_height as i64),
                        JValue::Byte(e.is_spent as i8),
                        JValue::Long(e.value as i64),
                        (&note_data).into(),
                    ],
                )?
                .i()
            })?;
            if code != 0 {
                return Ok(code);
            }
        }
        Ok(0)
    })
}

#[cfg(feature = "shielded")]
unsafe extern "C" fn tramp_persist_shielded_nullifiers_spent(
    context: *mut c_void,
    wallet_id: *const u8,
    entries: *const ShieldedNullifierSpentFFI,
    count: usize,
) -> i32 {
    with_bridge(context, |env, bridge| {
        let wid = id32(env, wallet_id)?;
        for e in slice_or_empty(entries, count) {
            let code = env.with_local_frame(16, |env| {
                let nwid = env.byte_array_from_slice(&e.wallet_id)?;
                let nullifier = env.byte_array_from_slice(&e.nullifier)?;
                env.call_method(
                    bridge,
                    "onPersistShieldedNullifierSpent",
                    "([B[BI[B)I",
                    &[
                        (&wid).into(),
                        (&nwid).into(),
                        JValue::Int(e.account_index as i32),
                        (&nullifier).into(),
                    ],
                )?
                .i()
            })?;
            if code != 0 {
                return Ok(code);
            }
        }
        Ok(0)
    })
}

#[cfg(feature = "shielded")]
unsafe extern "C" fn tramp_persist_shielded_outgoing_notes(
    context: *mut c_void,
    wallet_id: *const u8,
    entries: *const ShieldedOutgoingNoteFFI,
    count: usize,
) -> i32 {
    with_bridge(context, |env, bridge| {
        let wid = id32(env, wallet_id)?;
        for e in slice_or_empty(entries, count) {
            let code = env.with_local_frame(24, |env| {
                let nwid = env.byte_array_from_slice(&e.wallet_id)?;
                let cmx = env.byte_array_from_slice(&e.cmx)?;
                let recipient = env.byte_array_from_slice(&e.recipient)?;
                let memo = bytes(env, e.memo_ptr, e.memo_len)?;
                env.call_method(
                    bridge,
                    "onPersistShieldedOutgoingNote",
                    "([B[BI[B[BJJ[B)I",
                    &[
                        (&wid).into(),
                        (&nwid).into(),
                        JValue::Int(e.account_index as i32),
                        (&cmx).into(),
                        (&recipient).into(),
                        JValue::Long(e.value as i64),
                        JValue::Long(e.block_height as i64),
                        (&memo).into(),
                    ],
                )?
                .i()
            })?;
            if code != 0 {
                return Ok(code);
            }
        }
        Ok(0)
    })
}

#[cfg(feature = "shielded")]
unsafe extern "C" fn tramp_persist_shielded_synced_indices(
    context: *mut c_void,
    wallet_id: *const u8,
    entries: *const ShieldedSyncedIndexFFI,
    count: usize,
) -> i32 {
    with_bridge(context, |env, bridge| {
        let wid = id32(env, wallet_id)?;
        for e in slice_or_empty(entries, count) {
            let code = env.with_local_frame(16, |env| {
                let nwid = env.byte_array_from_slice(&e.wallet_id)?;
                env.call_method(
                    bridge,
                    "onPersistShieldedSyncedIndex",
                    "([B[BIJ)I",
                    &[
                        (&wid).into(),
                        (&nwid).into(),
                        JValue::Int(e.account_index as i32),
                        JValue::Long(e.last_synced_index as i64),
                    ],
                )?
                .i()
            })?;
            if code != 0 {
                return Ok(code);
            }
        }
        Ok(0)
    })
}

#[cfg(feature = "shielded")]
unsafe extern "C" fn tramp_persist_shielded_activity(
    context: *mut c_void,
    wallet_id: *const u8,
    entries: *const ShieldedActivityFFI,
    count: usize,
) -> i32 {
    with_bridge(context, |env, bridge| {
        let wid = id32(env, wallet_id)?;
        for e in slice_or_empty(entries, count) {
            let code = env.with_local_frame(32, |env| {
                let nwid = env.byte_array_from_slice(&e.wallet_id)?;
                let entry_id = env.byte_array_from_slice(&e.entry_id)?;
                let identity_id = env.byte_array_from_slice(&e.identity_id)?;
                let counterparty = bytes(env, e.counterparty_ptr, e.counterparty_len)?;
                let memo = bytes(env, e.memo_ptr, e.memo_len)?;
                let cmxs = bytes(env, e.note_cmxs_ptr, e.note_cmxs_count.saturating_mul(32))?;
                let nullifiers = bytes(
                    env,
                    e.spent_nullifiers_ptr,
                    e.spent_nullifiers_count.saturating_mul(32),
                )?;
                env.call_method(
                    bridge,
                    "onPersistShieldedActivity",
                    "([B[BI[BBBBJJZJZJ[BZ[B[B[B[B)I",
                    &[
                        (&wid).into(),
                        (&nwid).into(),
                        JValue::Int(e.account_index as i32),
                        (&entry_id).into(),
                        JValue::Byte(e.kind_tag as i8),
                        JValue::Byte(e.direction as i8),
                        JValue::Byte(e.status as i8),
                        JValue::Long(e.amount as i64),
                        JValue::Long(e.fee as i64),
                        JValue::Bool((e.has_fee != 0) as u8),
                        JValue::Long(e.block_height as i64),
                        JValue::Bool((e.has_block_height != 0) as u8),
                        JValue::Long(e.created_at_ms as i64),
                        (&identity_id).into(),
                        JValue::Bool((e.has_identity_id != 0) as u8),
                        (&counterparty).into(),
                        (&memo).into(),
                        (&cmxs).into(),
                        (&nullifiers).into(),
                    ],
                )?
                .i()
            })?;
            if code != 0 {
                return Ok(code);
            }
        }
        Ok(0)
    })
}

// ── Load: wallet list ─────────────────────────────────────────────────

/// Staged wallet-list row: all JNI reads land in owned Rust memory, so a
/// mid-load failure drops plain `Vec`s and never strands a raw buffer.
/// [`seal_wallet_entries`] mints the raw pointers once the whole load
/// succeeded.
struct WalletRestoreStaged {
    /// FFI row with `accounts` / `identities` / `platform_address_balances`
    /// / `utxos` still null / 0 until sealed.
    entry: WalletRestoreEntryFFI,
    specs: Vec<AccountSpecStaged>,
    identities: Vec<IdentityRestoreStaged>,
    /// Cached platform-address balances. `AddressBalanceEntryFFI` is
    /// `Copy` POD (the `hash` is an inline `[u8; 20]`), so the whole row
    /// stages as a plain owned `Vec` and the raw pointer minted at seal is
    /// freed with a single `free_raw_slice` — no nested buffers, mirroring
    /// the flat `ignored_senders` array.
    platform_address_balances: Vec<AddressBalanceEntryFFI>,
    /// Unspent Core UTXOs. Each row carries an owned `script_pubkey`
    /// buffer (variable length), so it stages like the account xpubs —
    /// per-row buffer pointer minted at seal, freed row-by-row before
    /// the array itself (CORE-06).
    utxos: Vec<UtxoRestoreStaged>,
    /// Persisted Core address pools. Each pool owns a nested `Vec` of
    /// address rows, and each row owns two `CString`s
    /// (`address_base58` / `derivation_path`); the raw pointers are minted
    /// at seal (nested address arrays + per-row strings) and freed in the
    /// same nested order (strings → inner array → outer array) by
    /// [`tramp_load_wallet_list_free`].
    core_address_pools: Vec<CoreAddressPoolStaged>,
}

/// Staged account spec: FFI struct with a null xpub pointer plus the
/// owned xpub bytes.
struct AccountSpecStaged {
    /// FFI spec with `account_xpub_bytes` still null / 0 until sealed.
    spec: AccountSpecFFI,
    xpub: Vec<u8>,
}

/// Staged unspent-UTXO row: FFI struct with a null `script_pubkey`
/// pointer plus the owned script bytes (CORE-06 restore path).
struct UtxoRestoreStaged {
    /// FFI row with `script_pubkey` still null / 0 until sealed.
    entry: UtxoRestoreEntryFFI,
    script: Vec<u8>,
}

/// Staged Core on-chain address row: FFI struct with both C-string
/// pointers (`address_base58` / `derivation_path`) still null until
/// sealed, plus the owned `CString`s that back them. Both strings are
/// REQUIRED on the platform-wallet load path (`address_info_from_ffi`
/// rejects a null for either), so they stage as plain `CString`s (not
/// `Option`) and mint via `CString::into_raw` at seal.
struct CoreAddressRowStaged {
    /// FFI row with `address_base58` / `derivation_path` still null until
    /// sealed.
    entry: CoreAddressEntryFFI,
    address: CString,
    path: CString,
}

/// Staged Core address pool: FFI struct with its nested address-row
/// pointer (`addresses_ptr`) still null / 0 until sealed, plus the owned
/// per-row staging. The `account` `AccountSpecFFI` carries a null xpub on
/// this path (the loader ignores it — the account already re-derives the
/// xpub from `accounts`), so there is nothing owned for the xpub to free.
struct CoreAddressPoolStaged {
    /// FFI pool with `addresses_ptr` / `addresses_count` still null / 0
    /// until sealed.
    entry: AccountAddressPoolFFI,
    rows: Vec<CoreAddressRowStaged>,
}

/// Staged identity-restore row: FFI struct with `keys` / `contacts` /
/// `ignored_senders` still null / 0 until sealed, plus the owned per-key
/// and per-contact staging. `dpns_names` / `contested_dpns_names` are left
/// null / 0 this pass (not yet ported), and `payments` /
/// `contact_profiles` stay null / 0 because Kotlin has no persist source
/// for them yet (no Room analog of `PersistentDashpayPayment` /
/// `PersistentDashpayContactProfile`), so there is nothing owned for any
/// of those to free.
struct IdentityRestoreStaged {
    /// FFI entry with `keys` / `contacts` / `ignored_senders` (and the
    /// dpns / payments / contact-profile arrays) still null / 0 until
    /// sealed.
    entry: IdentityRestoreEntryFFI,
    keys: Vec<IdentityKeyRestoreStaged>,
    contacts: Vec<ContactRestoreStaged>,
    /// Bare 32-byte ignored-sender ids (per-sender mute) — POD, minted
    /// as a flat `[u8; 32]` array at seal.
    ignored_senders: Vec<[u8; 32]>,
}

/// Staged DashPay contact-restore row: FFI struct with every pointer
/// field still null / 0 until sealed, plus the owned buffers that back
/// them. Mirrors the Swift `buildIdentityRestoreBuffer` contact block:
/// empty byte `Vec`s map back to `(null, 0)` (absent optionals), `None`
/// strings stay null, and `accepted_accounts` mints a `u32` buffer.
struct ContactRestoreStaged {
    /// FFI row with `encrypted_public_key` / `encrypted_account_label` /
    /// `auto_accept_proof` / `alias` / `note` / `contact_account_label` /
    /// `accepted_accounts` still null / 0 until sealed.
    row: ContactRequestFFI,
    encrypted_public_key: Vec<u8>,
    encrypted_account_label: Vec<u8>,
    auto_accept_proof: Vec<u8>,
    alias: Option<CString>,
    note: Option<CString>,
    contact_account_label: Option<CString>,
    accepted_accounts: Vec<u32>,
}

/// Staged identity-public-key row: FFI struct with a null `data` pointer
/// and null `contract_bounds_document_type` pointer, plus the owned
/// buffers that back them. `doc_type` is `Some` only for
/// `contract_bounds_kind == 2`.
struct IdentityKeyRestoreStaged {
    /// FFI key with `data` / `contract_bounds_document_type` still
    /// null / 0 until sealed.
    key: IdentityKeyRestoreFFI,
    data: Vec<u8>,
    doc_type: Option<CString>,
}

/// Mint the raw FFI pointers for a fully staged wallet list. Infallible:
/// runs only after every JNI read succeeded; every pointer minted here is
/// freed by [`tramp_load_wallet_list_free`].
fn seal_wallet_entries(staged: Vec<WalletRestoreStaged>) -> Vec<WalletRestoreEntryFFI> {
    staged
        .into_iter()
        .map(
            |WalletRestoreStaged {
                 mut entry,
                 specs,
                 identities,
                 platform_address_balances,
                 utxos,
                 core_address_pools,
             }| {
                // Flat POD array — no nested owned buffers, so the whole
                // `Vec<AddressBalanceEntryFFI>` mints in one shot and
                // `tramp_load_wallet_list_free` reclaims it with a single
                // `free_raw_slice`.
                (
                    entry.platform_address_balances,
                    entry.platform_address_balances_count,
                ) = vec_into_raw(platform_address_balances);

                let specs: Vec<AccountSpecFFI> = specs
                    .into_iter()
                    .map(|AccountSpecStaged { mut spec, xpub }| {
                        (spec.account_xpub_bytes, spec.account_xpub_bytes_len) = vec_into_raw(xpub);
                        spec
                    })
                    .collect();
                (entry.accounts, entry.accounts_count) = vec_into_raw(specs);

                // Unspent Core UTXOs — mint each row's script buffer,
                // then the array (CORE-06; freed row-by-row in
                // `tramp_load_wallet_list_free`, mirroring the account
                // xpub discipline).
                let utxos: Vec<UtxoRestoreEntryFFI> = utxos
                    .into_iter()
                    .map(|UtxoRestoreStaged { mut entry, script }| {
                        (entry.script_pubkey, entry.script_pubkey_len) = vec_into_raw(script);
                        entry
                    })
                    .collect();
                (entry.utxos, entry.utxos_count) = vec_into_raw(utxos);

                // Persisted Core address pools — mint each pool's nested
                // address-row array (each row's two required C-strings
                // first, via `CString::into_raw`), then the pool array
                // itself. Freed in the reverse nested order
                // (strings → inner array → outer array) in
                // `tramp_load_wallet_list_free`.
                let core_address_pools: Vec<AccountAddressPoolFFI> = core_address_pools
                    .into_iter()
                    .map(|CoreAddressPoolStaged { mut entry, rows }| {
                        let rows: Vec<CoreAddressEntryFFI> = rows
                            .into_iter()
                            .map(
                                |CoreAddressRowStaged {
                                     mut entry,
                                     address,
                                     path,
                                 }| {
                                    // Both strings are required (the loader
                                    // rejects null); `into_raw` hands
                                    // ownership to the FFI row, reclaimed
                                    // via `CString::from_raw` in the free
                                    // path.
                                    entry.address_base58 = address.into_raw() as *const c_char;
                                    entry.derivation_path = path.into_raw() as *const c_char;
                                    entry
                                },
                            )
                            .collect();
                        (entry.addresses_ptr, entry.addresses_count) = vec_into_raw(rows);
                        entry
                    })
                    .collect();
                (entry.core_address_pools, entry.core_address_pools_count) =
                    vec_into_raw(core_address_pools);

                // Identities: mint each identity's nested key / contact /
                // ignored-sender arrays first, then the identity array
                // itself. Every pointer minted here is reclaimed by
                // `tramp_load_wallet_list_free`.
                let identities: Vec<IdentityRestoreEntryFFI> = identities
                    .into_iter()
                    .map(
                        |IdentityRestoreStaged {
                             mut entry,
                             keys,
                             contacts,
                             ignored_senders,
                         }| {
                            let keys: Vec<IdentityKeyRestoreFFI> = keys
                                .into_iter()
                                .map(
                                    |IdentityKeyRestoreStaged {
                                         mut key,
                                         data,
                                         doc_type,
                                     }| {
                                        (key.data, key.data_len) = vec_into_raw(data);
                                        // Only kind==2 carried a doc-type; `into_raw`
                                        // hands ownership to the FFI struct, reclaimed
                                        // via `CString::from_raw` in the free path.
                                        key.contract_bounds_document_type =
                                            opt_cstring_into_raw(doc_type);
                                        key
                                    },
                                )
                                .collect();
                            (entry.keys, entry.keys_count) = vec_into_raw(keys);

                            let contacts: Vec<ContactRequestFFI> = contacts
                                .into_iter()
                                .map(
                                    |ContactRestoreStaged {
                                         mut row,
                                         encrypted_public_key,
                                         encrypted_account_label,
                                         auto_accept_proof,
                                         alias,
                                         note,
                                         contact_account_label,
                                         accepted_accounts,
                                     }| {
                                        (row.encrypted_public_key, row.encrypted_public_key_len) =
                                            vec_into_raw(encrypted_public_key);
                                        (
                                            row.encrypted_account_label,
                                            row.encrypted_account_label_len,
                                        ) = vec_into_raw(encrypted_account_label);
                                        (row.auto_accept_proof, row.auto_accept_proof_len) =
                                            vec_into_raw(auto_accept_proof);
                                        row.alias = opt_cstring_into_raw(alias);
                                        row.note = opt_cstring_into_raw(note);
                                        row.contact_account_label =
                                            opt_cstring_into_raw(contact_account_label);
                                        (row.accepted_accounts, row.accepted_accounts_len) =
                                            vec_into_raw(accepted_accounts);
                                        row
                                    },
                                )
                                .collect();
                            (entry.contacts, entry.contacts_count) = vec_into_raw(contacts);
                            (entry.ignored_senders, entry.ignored_senders_count) =
                                vec_into_raw(ignored_senders);
                            entry
                        },
                    )
                    .collect();
                (entry.identities, entry.identities_count) = vec_into_raw(identities);

                entry
            },
        )
        .collect()
}

unsafe extern "C" fn tramp_load_wallet_list(
    context: *mut c_void,
    out_entries: *mut *const WalletRestoreEntryFFI,
    out_count: *mut usize,
) -> i32 {
    let built = with_bridge_load(context, |env, bridge| {
        let holders = env
            .call_method(
                bridge,
                "onLoadWalletList",
                "()[Lorg/dashfoundation/dashsdk/ffi/WalletRestoreData;",
                &[],
            )?
            .l()?;
        let arr: jni::objects::JObjectArray = holders.into();
        let len = env.get_array_length(&arr)? as usize;
        let mut out: Vec<WalletRestoreStaged> = Vec::with_capacity(len);
        for i in 0..len {
            let entry = env.with_local_frame(
                64,
                |env| -> Result<WalletRestoreStaged, jni::errors::Error> {
                    let h = env.get_object_array_element(&arr, i as i32)?;
                    build_wallet_restore_entry(env, &h)
                },
            )?;
            out.push(entry);
        }
        Ok(out)
    });

    match built {
        Some(staged) => {
            let entries = seal_wallet_entries(staged);
            let count = entries.len();
            let boxed = entries.into_boxed_slice();
            let ptr = Box::into_raw(boxed) as *const WalletRestoreEntryFFI;
            *out_entries = ptr;
            *out_count = count;
            0
        }
        None => {
            *out_entries = ptr::null();
            *out_count = 0;
            ERR_JNI
        }
    }
}

/// Rebuild one wallet row from a Kotlin `WalletRestoreData` into a
/// [`WalletRestoreStaged`] (owned buffers only; raw pointers are minted by
/// [`seal_wallet_entries`] and freed in [`tramp_load_wallet_list_free`]).
/// Nested arrays we don't rehydrate this milestone (tracked locks) are
/// null / 0. `identities`, `platform_address_balances` (re-seeds the
/// provider balance map + ADDR-09 height pins on cold start; see
/// [`build_platform_address_balances`]), `utxos` (re-seeds the funds
/// accounts' UTXO maps + Core balance; CORE-06, see
/// [`build_utxo_restore_entries`]), and `core_address_pools` (re-seeds
/// each funds account's `AddressPool` so out-of-window restored addresses
/// keep their derivation-path mapping — the core-to-core-signing fix; see
/// [`build_core_address_pools`]) ARE rehydrated.
fn build_wallet_restore_entry(
    env: &mut JNIEnv,
    holder: &JObject,
) -> Result<WalletRestoreStaged, jni::errors::Error> {
    let wallet_id = read_id32_field(env, holder, "walletId")?;
    let network_ord = env.get_field(holder, "network", "I")?.i()?;
    let platform_sync_height = env.get_field(holder, "platformSyncHeight", "J")?.j()? as u64;
    let platform_sync_timestamp = env.get_field(holder, "platformSyncTimestamp", "J")?.j()? as u64;
    let platform_last_known = env
        .get_field(holder, "platformLastKnownRecentBlock", "J")?
        .j()? as u64;
    let birth_height = env.get_field(holder, "birthHeight", "I")?.i()? as u32;
    let synced_height = env.get_field(holder, "syncedHeight", "I")?.i()? as u32;
    let last_processed_height = env.get_field(holder, "lastProcessedHeight", "I")?.i()? as u32;
    let last_synced = env.get_field(holder, "lastSynced", "J")?.j()? as u64;

    // accounts.
    let specs_obj = env
        .get_field(
            holder,
            "accountSpecs",
            "[Lorg/dashfoundation/dashsdk/ffi/AccountSpecData;",
        )?
        .l()?;
    let specs_arr: jni::objects::JObjectArray = specs_obj.into();
    let specs_len = env.get_array_length(&specs_arr)? as usize;
    let mut specs: Vec<AccountSpecStaged> = Vec::with_capacity(specs_len);
    for i in 0..specs_len {
        let spec = env.with_local_frame(32, |env| {
            let s = env.get_object_array_element(&specs_arr, i as i32)?;
            build_account_spec(env, &s)
        })?;
        specs.push(spec);
    }

    // identities (each carrying its public keys).
    let ids_obj = env
        .get_field(
            holder,
            "identities",
            "[Lorg/dashfoundation/dashsdk/ffi/IdentityRestoreData;",
        )?
        .l()?;
    let ids_arr: jni::objects::JObjectArray = ids_obj.into();
    let ids_len = env.get_array_length(&ids_arr)? as usize;
    let mut identities: Vec<IdentityRestoreStaged> = Vec::with_capacity(ids_len);
    for i in 0..ids_len {
        let id = env.with_local_frame(64, |env| {
            let h = env.get_object_array_element(&ids_arr, i as i32)?;
            build_identity_restore(env, &h)
        })?;
        identities.push(id);
    }

    // Cached platform-address balances (re-seed the provider balance map
    // + ADDR-09 `as_of_height` pins on cold start; see SH-06). Staged as
    // owned POD; the raw pointer is minted at seal.
    let platform_address_balances = build_platform_address_balances(env, holder)?;

    // Unspent Core UTXOs (re-seed the funds accounts' UTXO maps + the
    // Core balance on cold start; see CORE-06). Each row's script buffer
    // stays owned until seal.
    let utxos = build_utxo_restore_entries(env, holder)?;

    // Persisted Core address pools (re-seed each funds account's
    // `AddressPool` so out-of-window restored addresses keep their
    // derivation-path mapping; without it a restored UTXO on such an
    // address can't be signed after a cold restart). Each row's two
    // required C-strings stay owned until seal.
    let core_address_pools = build_core_address_pools(env, holder)?;

    let entry = WalletRestoreEntryFFI {
        wallet_id,
        network: net_from_ord(network_ord),
        accounts: ptr::null(),
        accounts_count: 0,
        platform_address_balances: ptr::null(),
        platform_address_balances_count: 0,
        platform_sync_height,
        platform_sync_timestamp,
        platform_last_known_recent_block: platform_last_known,
        identities: ptr::null(),
        identities_count: 0,
        birth_height,
        synced_height,
        last_processed_height,
        last_synced,
        utxos: ptr::null(),
        utxos_count: 0,
        tracked_asset_locks: ptr::null(),
        tracked_asset_locks_count: 0,
        unresolved_asset_lock_tx_records: ptr::null(),
        unresolved_asset_lock_tx_records_count: 0,
        core_address_pools: ptr::null(),
        core_address_pools_count: 0,
        last_applied_chain_lock_bytes: ptr::null(),
        last_applied_chain_lock_bytes_len: 0,
    };
    Ok(WalletRestoreStaged {
        entry,
        specs,
        identities,
        platform_address_balances,
        utxos,
        core_address_pools,
    })
}

/// Read the Kotlin `WalletRestoreData.utxos` array into staged
/// [`UtxoRestoreStaged`] rows (CORE-06 restore path). Empty / null field
/// → empty vec (staged as `(null, 0)` at seal).
///
/// Each holder is a `UtxoRestoreData`. The leading account-tag block
/// mirrors `AccountSpecData` (identity ids via the optional-id32 read —
/// ordinary accounts persist them empty); `prevTxid` is a fixed 32-byte
/// read (the Kotlin builder pre-drops wrong-length rows for the same
/// abort-on-corrupt rationale as the SH-06 hash filter); `scriptPubKey`
/// stages as an owned byte `Vec` whose pointer is minted at seal. The
/// platform-wallet load side routes each row into the matching funds
/// account and recomputes balances (`update_balance`). Mirror of the
/// Swift `buildUtxoRestoreBuffer`.
fn build_utxo_restore_entries(
    env: &mut JNIEnv,
    holder: &JObject,
) -> Result<Vec<UtxoRestoreStaged>, jni::errors::Error> {
    let arr_obj = env
        .get_field(
            holder,
            "utxos",
            "[Lorg/dashfoundation/dashsdk/ffi/UtxoRestoreData;",
        )?
        .l()?;
    if arr_obj.is_null() {
        return Ok(Vec::new());
    }
    let arr: jni::objects::JObjectArray = arr_obj.into();
    let len = env.get_array_length(&arr)? as usize;
    let mut out: Vec<UtxoRestoreStaged> = Vec::with_capacity(len);
    for i in 0..len {
        let staged =
            env.with_local_frame(32, |env| -> Result<UtxoRestoreStaged, jni::errors::Error> {
                let h = env.get_object_array_element(&arr, i as i32)?;
                let type_tag = env.get_field(&h, "typeTag", "B")?.b()? as u8;
                let standard_tag = env.get_field(&h, "standardTag", "B")?.b()? as u8;
                let account_index = env.get_field(&h, "accountIndex", "I")?.i()? as u32;
                let registration_index = env.get_field(&h, "registrationIndex", "I")?.i()? as u32;
                let key_class = env.get_field(&h, "keyClass", "I")?.i()? as u32;
                let user_identity_id = read_optional_id32_field(env, &h, "userIdentityId")?;
                let friend_identity_id = read_optional_id32_field(env, &h, "friendIdentityId")?;
                let prev_txid = read_bytes_field_fixed::<32>(env, &h, "prevTxid")?;
                let vout = env.get_field(&h, "vout", "I")?.i()? as u32;
                let value_duffs = env.get_field(&h, "valueDuffs", "J")?.j()? as u64;
                let script = read_bytes_field_vec(env, &h, "scriptPubKey")?;
                let height = env.get_field(&h, "height", "I")?.i()? as u32;
                let is_coinbase = env.get_field(&h, "isCoinbase", "Z")?.z()?;
                let is_confirmed = env.get_field(&h, "isConfirmed", "Z")?.z()?;
                let is_instantlocked = env.get_field(&h, "isInstantLocked", "Z")?.z()?;
                let is_locked = env.get_field(&h, "isLocked", "Z")?.z()?;
                Ok(UtxoRestoreStaged {
                    entry: UtxoRestoreEntryFFI {
                        type_tag,
                        standard_tag,
                        account_index,
                        registration_index,
                        key_class,
                        user_identity_id,
                        friend_identity_id,
                        prev_txid,
                        vout,
                        value_duffs,
                        script_pubkey: ptr::null(),
                        script_pubkey_len: 0,
                        height,
                        is_coinbase,
                        is_confirmed,
                        is_instantlocked,
                        is_locked,
                    },
                    script,
                })
            })?;
        out.push(staged);
    }
    Ok(out)
}

/// Read the Kotlin `WalletRestoreData.coreAddressPools` array into staged
/// [`CoreAddressPoolStaged`] rows. Empty / null field → empty vec (staged
/// as `(null, 0)` at seal).
///
/// Each holder is a `CoreAddressPoolRestoreData`; its nested `account` is
/// read via the shared [`build_account_spec`] reader (the account xpub is
/// staged empty and minted null at seal — the platform-wallet loader
/// ignores the xpub on this path). Each `CoreAddressRestoreData` row reads
/// the 33-byte `publicKey` (zero-filled and `has_public_key = false` when
/// absent / not exactly 33 bytes, matching the Swift
/// `publicKey.count == 33` rule) plus the two REQUIRED strings
/// (`addressBase58` / `derivationPath`) into owned `CString`s — a null or
/// interior-NUL for either aborts the whole load, because the loader
/// (`address_info_from_ffi`) rejects a null and an address without its
/// derivation path is exactly the row that breaks core-to-core signing
/// after cold restart. Mirror of the Swift `buildCoreAddressPoolBuffer`.
fn build_core_address_pools(
    env: &mut JNIEnv,
    holder: &JObject,
) -> Result<Vec<CoreAddressPoolStaged>, jni::errors::Error> {
    let arr_obj = env
        .get_field(
            holder,
            "coreAddressPools",
            "[Lorg/dashfoundation/dashsdk/ffi/CoreAddressPoolRestoreData;",
        )?
        .l()?;
    if arr_obj.is_null() {
        return Ok(Vec::new());
    }
    let arr: jni::objects::JObjectArray = arr_obj.into();
    let len = env.get_array_length(&arr)? as usize;
    let mut out: Vec<CoreAddressPoolStaged> = Vec::with_capacity(len);
    for i in 0..len {
        let staged = env.with_local_frame(
            32,
            |env| -> Result<CoreAddressPoolStaged, jni::errors::Error> {
                let h = env.get_object_array_element(&arr, i as i32)?;

                // The account tuple — reuse the shared spec reader. The
                // staged xpub is discarded (the pool's `account` carries a
                // null xpub; the loader ignores it), so only the spec is
                // kept.
                let account_obj = env
                    .get_field(
                        &h,
                        "account",
                        "Lorg/dashfoundation/dashsdk/ffi/AccountSpecData;",
                    )?
                    .l()?;
                let AccountSpecStaged { mut spec, xpub: _ } =
                    build_account_spec(env, &account_obj)?;
                // Kept explicit for the reader: the pool never carries an
                // xpub, so it stays null / 0 (already the value
                // `build_account_spec` leaves it at pre-seal).
                spec.account_xpub_bytes = ptr::null();
                spec.account_xpub_bytes_len = 0;

                let pool_type_tag = env.get_field(&h, "poolTypeTag", "B")?.b()? as u8;

                let rows_obj = env
                    .get_field(
                        &h,
                        "addresses",
                        "[Lorg/dashfoundation/dashsdk/ffi/CoreAddressRestoreData;",
                    )?
                    .l()?;
                let rows_arr: jni::objects::JObjectArray = rows_obj.into();
                let rows_len = env.get_array_length(&rows_arr)? as usize;
                let mut rows: Vec<CoreAddressRowStaged> = Vec::with_capacity(rows_len);
                for j in 0..rows_len {
                    let row = env.with_local_frame(
                        16,
                        |env| -> Result<CoreAddressRowStaged, jni::errors::Error> {
                            let r = env.get_object_array_element(&rows_arr, j as i32)?;
                            let pk_bytes = read_bytes_field_vec(env, &r, "publicKey")?;
                            // `has_public_key` is implied by the exact
                            // 33-byte length (Swift `publicKey.count == 33`);
                            // zero-fill the fixed field when absent / short.
                            let mut public_key = [0u8; 33];
                            let has_public_key = pk_bytes.len() == 33;
                            if has_public_key {
                                public_key.copy_from_slice(&pk_bytes);
                            }
                            let address_index = env.get_field(&r, "addressIndex", "I")?.i()? as u32;
                            let is_used = env.get_field(&r, "isUsed", "Z")?.z()?;
                            let balance = env.get_field(&r, "balance", "J")?.j()? as u64;
                            // Both required — a null / interior-NUL aborts
                            // the load (the loader rejects a null and a row
                            // missing its path is the signing-break case).
                            let address = read_req_cstring_field(env, &r, "addressBase58")?;
                            let path = read_req_cstring_field(env, &r, "derivationPath")?;
                            Ok(CoreAddressRowStaged {
                                entry: CoreAddressEntryFFI {
                                    public_key,
                                    has_public_key,
                                    pool_type_tag,
                                    address_index,
                                    is_used,
                                    balance,
                                    address_base58: ptr::null(),
                                    derivation_path: ptr::null(),
                                },
                                address,
                                path,
                            })
                        },
                    )?;
                    rows.push(row);
                }

                Ok(CoreAddressPoolStaged {
                    entry: AccountAddressPoolFFI {
                        account: spec,
                        pool_type_tag,
                        addresses_ptr: ptr::null(),
                        addresses_count: 0,
                    },
                    rows,
                })
            },
        )?;
        out.push(staged);
    }
    Ok(out)
}

/// Read the Kotlin `WalletRestoreData.platformAddressBalances` array into
/// an owned `Vec<AddressBalanceEntryFFI>` (the `#4019` layout carrying
/// `as_of_height`). Empty / null field → empty vec (staged as `(null, 0)`
/// at seal).
///
/// Each holder is a `PlatformAddressBalanceRestoreData`; the 20-byte
/// `addressHash` is read fixed-length (a wrong length aborts the whole
/// load — the same altered-key rationale as the id fields, and the reason
/// the Kotlin builder pre-drops non-20-byte hashes). `asOfHeight` is the
/// persisted `lastSeenHeight` height pin and MUST round-trip unchanged, or
/// the ADDR-09 double-count gate re-opens (SH-06). Mirror of the Swift
/// `loadCachedBalances` → `AddressBalanceEntryFFI` buffer path.
fn build_platform_address_balances(
    env: &mut JNIEnv,
    holder: &JObject,
) -> Result<Vec<AddressBalanceEntryFFI>, jni::errors::Error> {
    let arr_obj = env
        .get_field(
            holder,
            "platformAddressBalances",
            "[Lorg/dashfoundation/dashsdk/ffi/PlatformAddressBalanceRestoreData;",
        )?
        .l()?;
    if arr_obj.is_null() {
        return Ok(Vec::new());
    }
    let arr: jni::objects::JObjectArray = arr_obj.into();
    let len = env.get_array_length(&arr)? as usize;
    let mut out: Vec<AddressBalanceEntryFFI> = Vec::with_capacity(len);
    for i in 0..len {
        let e = env.with_local_frame(
            16,
            |env| -> Result<AddressBalanceEntryFFI, jni::errors::Error> {
                let h = env.get_object_array_element(&arr, i as i32)?;
                let address_type = env.get_field(&h, "addressType", "B")?.b()? as u8;
                let hash = read_bytes_field_fixed::<20>(env, &h, "addressHash")?;
                let balance = env.get_field(&h, "balance", "J")?.j()? as u64;
                let nonce = env.get_field(&h, "nonce", "I")?.i()? as u32;
                let account_index = env.get_field(&h, "accountIndex", "I")?.i()? as u32;
                let address_index = env.get_field(&h, "addressIndex", "I")?.i()? as u32;
                let as_of_height = env.get_field(&h, "asOfHeight", "J")?.j()? as u64;
                Ok(AddressBalanceEntryFFI {
                    address: PlatformAddressFFI { address_type, hash },
                    balance,
                    nonce,
                    account_index,
                    address_index,
                    as_of_height,
                })
            },
        )?;
        out.push(e);
    }
    Ok(out)
}

/// Rebuild one identity-restore row from a Kotlin `IdentityRestoreData`
/// into an [`IdentityRestoreStaged`]. Public-key `data` buffers and the
/// contract-bounds doc-type C-string stay owned here (empty vec / `None`
/// respectively when absent); the raw buffers the wallet-list free
/// trampoline frees are minted by [`seal_wallet_entries`] only once the
/// whole load succeeded. DPNS arrays are left null / 0 this pass.
fn build_identity_restore(
    env: &mut JNIEnv,
    holder: &JObject,
) -> Result<IdentityRestoreStaged, jni::errors::Error> {
    let identity_id = read_id32_field(env, holder, "identityId")?;
    let balance = env.get_field(holder, "balance", "J")?.j()? as u64;
    let revision = env.get_field(holder, "revision", "J")?.j()? as u64;
    let identity_index = env.get_field(holder, "identityIndex", "I")?.i()? as u32;
    let status = env.get_field(holder, "status", "B")?.b()? as u8;

    let keys_obj = env
        .get_field(
            holder,
            "keys",
            "[Lorg/dashfoundation/dashsdk/ffi/IdentityKeyRestoreData;",
        )?
        .l()?;
    let keys_arr: jni::objects::JObjectArray = keys_obj.into();
    let keys_len = env.get_array_length(&keys_arr)? as usize;
    let mut keys: Vec<IdentityKeyRestoreStaged> = Vec::with_capacity(keys_len);
    for i in 0..keys_len {
        let k = env.with_local_frame(32, |env| {
            let h = env.get_object_array_element(&keys_arr, i as i32)?;
            build_identity_key_restore(env, &h)
        })?;
        keys.push(k);
    }

    // DashPay contact rows — pending + established requests (with their
    // contactInfo metadata), assembled from the Room
    // `DashpayContactRequestEntity` rows Kotlin-side. Mirror of the
    // Swift `buildIdentityRestoreBuffer` contact block.
    let contacts_obj = env
        .get_field(
            holder,
            "contacts",
            "[Lorg/dashfoundation/dashsdk/ffi/ContactRequestRestoreData;",
        )?
        .l()?;
    let contacts_arr: jni::objects::JObjectArray = contacts_obj.into();
    let contacts_len = env.get_array_length(&contacts_arr)? as usize;
    let mut contacts: Vec<ContactRestoreStaged> = Vec::with_capacity(contacts_len);
    for i in 0..contacts_len {
        let c = env.with_local_frame(48, |env| {
            let h = env.get_object_array_element(&contacts_arr, i as i32)?;
            build_contact_restore(env, &h)
        })?;
        contacts.push(c);
    }

    // DashPay ignored senders (per-sender mute, local-only) — restores
    // the `ignored_senders` set so a previously-ignored sender doesn't
    // resurface after a relaunch when their still-on-platform immutable
    // contactRequest documents re-ingest on the next sweep.
    let ignored_senders = read_id32_array_field(env, holder, "ignoredSenders")?;

    let entry = IdentityRestoreEntryFFI {
        identity_id,
        balance,
        revision,
        identity_index,
        status,
        dpns_names: ptr::null(),
        dpns_names_count: 0,
        contested_dpns_names: ptr::null(),
        contested_dpns_names_count: 0,
        keys: ptr::null(),
        keys_count: 0,
        contacts: ptr::null(),
        contacts_count: 0,
        // Payments + cached contact profiles stay null / 0 this pass:
        // Kotlin has no persist source for them yet (no Room analog of
        // `PersistentDashpayPayment` / `PersistentDashpayContactProfile`),
        // so there is nothing to rehydrate — the reconcile / profile
        // sweeps rebuild them after load, exactly as before the arrays
        // existed. The free trampoline never has to reclaim them because
        // they are never minted.
        payments: ptr::null(),
        payments_count: 0,
        ignored_senders: ptr::null(),
        ignored_senders_count: 0,
        contact_profiles: ptr::null(),
        contact_profiles_count: 0,
    };
    Ok(IdentityRestoreStaged {
        entry,
        keys,
        contacts,
        ignored_senders,
    })
}

/// Rebuild one DashPay contact row from a Kotlin `ContactRequestRestoreData`
/// into a [`ContactRestoreStaged`]. Byte payloads and metadata strings stay
/// owned here (empty vec / `None` when absent); [`seal_wallet_entries`]
/// mints the raw buffers only once the whole load succeeded and
/// [`tramp_load_wallet_list_free`] reclaims them.
fn build_contact_restore(
    env: &mut JNIEnv,
    holder: &JObject,
) -> Result<ContactRestoreStaged, jni::errors::Error> {
    let owner_id = read_id32_field(env, holder, "ownerIdentityId")?;
    let contact_id = read_id32_field(env, holder, "contactIdentityId")?;
    let is_outgoing = env.get_field(holder, "isOutgoing", "Z")?.z()?;
    let sender_key_index = env.get_field(holder, "senderKeyIndex", "I")?.i()? as u32;
    let recipient_key_index = env.get_field(holder, "recipientKeyIndex", "I")?.i()? as u32;
    let account_reference = env.get_field(holder, "accountReference", "I")?.i()? as u32;
    let encrypted_public_key = read_bytes_field_vec(env, holder, "encryptedPublicKey")?;
    let encrypted_account_label = read_bytes_field_vec(env, holder, "encryptedAccountLabel")?;
    let auto_accept_proof = read_bytes_field_vec(env, holder, "autoAcceptProof")?;
    let core_height_created_at = env.get_field(holder, "coreHeightCreatedAt", "I")?.i()? as u32;
    let created_at = env.get_field(holder, "createdAtMillis", "J")?.j()? as u64;
    let payment_channel_broken = env.get_field(holder, "paymentChannelBroken", "Z")?.z()?;
    let alias = read_opt_cstring_field(env, holder, "alias")?;
    let note = read_opt_cstring_field(env, holder, "note")?;
    let is_hidden = env.get_field(holder, "isHidden", "Z")?.z()?;
    let contact_account_label = read_opt_cstring_field(env, holder, "contactAccountLabel")?;
    let accepted_accounts = read_u32_array_field(env, holder, "acceptedAccounts")?;

    let row = ContactRequestFFI {
        owner_id,
        contact_id,
        is_outgoing,
        sender_key_index,
        recipient_key_index,
        account_reference,
        encrypted_public_key: ptr::null(),
        encrypted_public_key_len: 0,
        encrypted_account_label: ptr::null(),
        encrypted_account_label_len: 0,
        auto_accept_proof: ptr::null(),
        auto_accept_proof_len: 0,
        core_height_created_at,
        created_at,
        payment_channel_broken,
        alias: ptr::null(),
        note: ptr::null(),
        is_hidden,
        contact_account_label: ptr::null(),
        accepted_accounts: ptr::null(),
        accepted_accounts_len: 0,
    };
    Ok(ContactRestoreStaged {
        row,
        encrypted_public_key,
        encrypted_account_label,
        auto_accept_proof,
        alias,
        note,
        contact_account_label,
        accepted_accounts,
    })
}

/// Rebuild one identity-public-key row from a Kotlin
/// `IdentityKeyRestoreData` into an [`IdentityKeyRestoreStaged`]. The
/// public-key `data` bytes and the optional contract-bounds doc-type
/// C-string stay owned here; both raw pointers are minted by
/// [`seal_wallet_entries`] and reclaimed by [`tramp_load_wallet_list_free`].
fn build_identity_key_restore(
    env: &mut JNIEnv,
    holder: &JObject,
) -> Result<IdentityKeyRestoreStaged, jni::errors::Error> {
    let key_id = env.get_field(holder, "keyId", "I")?.i()? as u32;
    let key_type = env.get_field(holder, "keyType", "B")?.b()? as u8;
    let purpose = env.get_field(holder, "purpose", "B")?.b()? as u8;
    let security_level = env.get_field(holder, "securityLevel", "B")?.b()? as u8;
    let read_only = env.get_field(holder, "readOnly", "Z")?.z()?;
    let data = read_bytes_field_vec(env, holder, "data")?;
    let contract_bounds_kind = env.get_field(holder, "contractBoundsKind", "B")?.b()? as u8;
    // `contractBoundsId` is 32 bytes for kind 1/2, empty for kind 0 — the
    // optional-id reader maps the empty sentinel to the all-zero id.
    let contract_bounds_id = read_optional_id32_field(env, holder, "contractBoundsId")?;
    // Doc-type C-string only meaningful for kind 2; read as a nullable
    // Java String. Interior NULs (impossible for a DPP document-type name)
    // would fail `CString::new` — degrade to `None` rather than fail the load.
    let doc_type = read_opt_cstring_field(env, holder, "contractBoundsDocumentType")?;

    let key = IdentityKeyRestoreFFI {
        key_id,
        key_type,
        purpose,
        security_level,
        read_only,
        data: ptr::null(),
        data_len: 0,
        contract_bounds_kind,
        contract_bounds_id,
        contract_bounds_document_type: ptr::null(),
    };
    Ok(IdentityKeyRestoreStaged {
        key,
        data,
        doc_type,
    })
}

/// Read an optional Java `String?` field into an owned `CString` (null /
/// interior-NUL → `None`). Used for the identity-key contract-bounds
/// doc-type; the `CString` is later handed to the FFI struct via
/// `into_raw` and reclaimed with `CString::from_raw` in the free path.
fn read_opt_cstring_field(
    env: &mut JNIEnv,
    holder: &JObject,
    field: &str,
) -> Result<Option<CString>, jni::errors::Error> {
    let obj = env.get_field(holder, field, "Ljava/lang/String;")?.l()?;
    if obj.is_null() {
        return Ok(None);
    }
    let s: String = env.get_string(&JString::from(obj))?.into();
    Ok(CString::new(s).ok())
}

/// Read a REQUIRED Java `String` field into an owned `CString`. Unlike
/// [`read_opt_cstring_field`], a null value or an interior NUL fails the
/// whole load (surfaced as `ERR_JNI` by the load trampoline): the two
/// Core-address strings (`addressBase58` / `derivation_path`) are both
/// mandatory on the platform-wallet load path (`address_info_from_ffi`
/// rejects a null for either), and a restored address without its
/// derivation path is exactly the row that breaks core-to-core signing
/// after a cold restart — dropping it silently would defeat the fix.
fn read_req_cstring_field(
    env: &mut JNIEnv,
    holder: &JObject,
    field: &str,
) -> Result<CString, jni::errors::Error> {
    let obj = env.get_field(holder, field, "Ljava/lang/String;")?.l()?;
    if obj.is_null() {
        log::error!("load: required string field `{field}` is null");
        return Err(jni::errors::Error::WrongJValueType(
            "non-null String",
            "null",
        ));
    }
    let s: String = env.get_string(&JString::from(obj))?.into();
    CString::new(s).map_err(|_| {
        log::error!("load: required string field `{field}` contains an interior NUL");
        jni::errors::Error::WrongJValueType("String without interior NUL", "String with NUL")
    })
}

/// Rebuild one account spec from a Kotlin `AccountSpecData` into an
/// [`AccountSpecStaged`]. The xpub stays an owned `Vec<u8>` here; the raw
/// buffer the wallet-list free trampoline frees is minted by
/// [`seal_wallet_entries`] only once the whole load succeeded.
fn build_account_spec(
    env: &mut JNIEnv,
    holder: &JObject,
) -> Result<AccountSpecStaged, jni::errors::Error> {
    let type_tag = env.get_field(holder, "typeTag", "B")?.b()? as u8;
    let standard_tag = env.get_field(holder, "standardTag", "B")?.b()? as u8;
    let index = env.get_field(holder, "index", "I")?.i()? as u32;
    let registration_index = env.get_field(holder, "registrationIndex", "I")?.i()? as u32;
    let key_class = env.get_field(holder, "keyClass", "I")?.i()? as u32;
    let user_identity_id = read_optional_id32_field(env, holder, "userIdentityId")?;
    let friend_identity_id = read_optional_id32_field(env, holder, "friendIdentityId")?;
    let xpub = read_bytes_field_vec(env, holder, "accountXpubBytes")?;
    Ok(AccountSpecStaged {
        spec: AccountSpecFFI {
            type_tag,
            standard_tag,
            index,
            registration_index,
            key_class,
            user_identity_id,
            friend_identity_id,
            account_xpub_bytes: ptr::null(),
            account_xpub_bytes_len: 0,
        },
        xpub,
    })
}

/// Free everything [`tramp_load_wallet_list`] allocated.
unsafe extern "C" fn tramp_load_wallet_list_free(
    _context: *mut c_void,
    entries: *const WalletRestoreEntryFFI,
    count: usize,
) {
    let _ = catch_unwind(AssertUnwindSafe(|| {
        if entries.is_null() || count == 0 {
            return;
        }
        let boxed: Box<[WalletRestoreEntryFFI]> = Box::from_raw(
            std::ptr::slice_from_raw_parts_mut(entries as *mut WalletRestoreEntryFFI, count),
        );
        for e in boxed.iter() {
            // Cached platform-address balances — flat POD slice minted by
            // `seal_wallet_entries` (`AddressBalanceEntryFFI` is `Copy`
            // with an inline `[u8; 20]` hash, no nested buffers).
            free_raw_slice(
                e.platform_address_balances,
                e.platform_address_balances_count,
            );

            // accounts + nested xpub buffers.
            if !e.accounts.is_null() && e.accounts_count > 0 {
                let specs: Box<[AccountSpecFFI]> =
                    Box::from_raw(std::ptr::slice_from_raw_parts_mut(
                        e.accounts as *mut AccountSpecFFI,
                        e.accounts_count,
                    ));
                for s in specs.iter() {
                    free_raw_bytes(s.account_xpub_bytes, s.account_xpub_bytes_len);
                }
                drop(specs);
            }

            // Unspent Core UTXOs + nested script buffers (CORE-06) —
            // mirrors exactly what `seal_wallet_entries` minted.
            if !e.utxos.is_null() && e.utxos_count > 0 {
                let utxos: Box<[UtxoRestoreEntryFFI]> =
                    Box::from_raw(std::ptr::slice_from_raw_parts_mut(
                        e.utxos as *mut UtxoRestoreEntryFFI,
                        e.utxos_count,
                    ));
                for u in utxos.iter() {
                    free_raw_bytes(u.script_pubkey, u.script_pubkey_len);
                }
                drop(utxos);
            }

            // Persisted Core address pools + nested address-row arrays and
            // each row's two required C-strings — mirrors exactly what
            // `seal_wallet_entries` minted. Free order matches the UTXO
            // discipline: each row's strings first, then the inner
            // `[CoreAddressEntryFFI]` array, then the outer
            // `[AccountAddressPoolFFI]` array. The pool's `account`
            // `AccountSpecFFI` carries a null xpub on this path (never
            // minted), so there is nothing to reclaim for it.
            if !e.core_address_pools.is_null() && e.core_address_pools_count > 0 {
                let pools: Box<[AccountAddressPoolFFI]> =
                    Box::from_raw(std::ptr::slice_from_raw_parts_mut(
                        e.core_address_pools as *mut AccountAddressPoolFFI,
                        e.core_address_pools_count,
                    ));
                for pool in pools.iter() {
                    if !pool.addresses_ptr.is_null() && pool.addresses_count > 0 {
                        let rows: Box<[CoreAddressEntryFFI]> =
                            Box::from_raw(std::ptr::slice_from_raw_parts_mut(
                                pool.addresses_ptr as *mut CoreAddressEntryFFI,
                                pool.addresses_count,
                            ));
                        for row in rows.iter() {
                            free_raw_cstring(row.address_base58);
                            free_raw_cstring(row.derivation_path);
                        }
                        drop(rows);
                    }
                }
                drop(pools);
            }

            // identities + nested key / contact / ignored-sender arrays
            // (each key's `data` buffer + contract-bounds doc-type C-string;
            // each contact's three byte payloads, three metadata C-strings
            // and `accepted_accounts` u32 buffer). Mirrors exactly what
            // `seal_wallet_entries` minted. The dpns_names /
            // contested_dpns_names / payments / contact_profiles arrays are
            // never minted this pass (staged null / 0), so there is nothing
            // to reclaim for them.
            if !e.identities.is_null() && e.identities_count > 0 {
                let idents: Box<[IdentityRestoreEntryFFI]> =
                    Box::from_raw(std::ptr::slice_from_raw_parts_mut(
                        e.identities as *mut IdentityRestoreEntryFFI,
                        e.identities_count,
                    ));
                for ident in idents.iter() {
                    if !ident.keys.is_null() && ident.keys_count > 0 {
                        let keys: Box<[IdentityKeyRestoreFFI]> =
                            Box::from_raw(std::ptr::slice_from_raw_parts_mut(
                                ident.keys as *mut IdentityKeyRestoreFFI,
                                ident.keys_count,
                            ));
                        for k in keys.iter() {
                            free_raw_bytes(k.data, k.data_len);
                            // Reclaim the doc-type C-string handed to the
                            // FFI struct via `CString::into_raw` in
                            // `seal_wallet_entries` (only kind==2 keys have
                            // a non-null pointer).
                            free_raw_cstring(k.contract_bounds_document_type);
                        }
                        drop(keys);
                    }
                    if !ident.contacts.is_null() && ident.contacts_count > 0 {
                        let contacts: Box<[ContactRequestFFI]> =
                            Box::from_raw(std::ptr::slice_from_raw_parts_mut(
                                ident.contacts as *mut ContactRequestFFI,
                                ident.contacts_count,
                            ));
                        for c in contacts.iter() {
                            free_raw_bytes(c.encrypted_public_key, c.encrypted_public_key_len);
                            free_raw_bytes(
                                c.encrypted_account_label,
                                c.encrypted_account_label_len,
                            );
                            free_raw_bytes(c.auto_accept_proof, c.auto_accept_proof_len);
                            free_raw_cstring(c.alias);
                            free_raw_cstring(c.note);
                            free_raw_cstring(c.contact_account_label);
                            free_raw_slice(c.accepted_accounts, c.accepted_accounts_len);
                        }
                        drop(contacts);
                    }
                    // Flat POD array of 32-byte sender ids — no nested
                    // buffers, just the boxed slice itself.
                    free_raw_slice(ident.ignored_senders, ident.ignored_senders_count);
                }
                drop(idents);
            }
        }
        drop(boxed);
    }));
}

// ── Load: shielded ────────────────────────────────────────────────────

#[cfg(feature = "shielded")]
unsafe extern "C" fn tramp_load_shielded_notes(
    context: *mut c_void,
    out_entries: *mut *const ShieldedNoteRestoreFFI,
    out_count: *mut usize,
) -> i32 {
    let built = with_bridge_load(context, |env, bridge| {
        let arr: jni::objects::JObjectArray = env
            .call_method(
                bridge,
                "onLoadShieldedNotes",
                "()[Lorg/dashfoundation/dashsdk/ffi/ShieldedNoteData;",
                &[],
            )?
            .l()?
            .into();
        let len = env.get_array_length(&arr)? as usize;
        // Staged rows: raw buffers are minted only after every JNI read
        // succeeded, so a mid-load failure drops owned Vecs and leaks nothing.
        let mut out: Vec<(ShieldedNoteRestoreFFI, Vec<u8>)> = Vec::with_capacity(len);
        for i in 0..len {
            let entry = env.with_local_frame(
                64,
                |env| -> Result<(ShieldedNoteRestoreFFI, Vec<u8>), jni::errors::Error> {
                    let h = env.get_object_array_element(&arr, i as i32)?;
                    let wallet_id = read_id32_field(env, &h, "walletId")?;
                    let account_index = env.get_field(&h, "accountIndex", "I")?.i()? as u32;
                    let position = env.get_field(&h, "position", "J")?.j()? as u64;
                    let cmx = read_id32_field(env, &h, "cmx")?;
                    let nullifier = read_id32_field(env, &h, "nullifier")?;
                    let block_height = env.get_field(&h, "blockHeight", "J")?.j()? as u64;
                    let is_spent = env.get_field(&h, "isSpent", "B")?.b()? as u8;
                    let value = env.get_field(&h, "value", "J")?.j()? as u64;
                    let note_data = read_bytes_field_vec(env, &h, "noteData")?;
                    Ok((
                        ShieldedNoteRestoreFFI {
                            wallet_id,
                            account_index,
                            position,
                            cmx,
                            nullifier,
                            block_height,
                            is_spent,
                            value,
                            note_data_ptr: ptr::null(),
                            note_data_len: 0,
                        },
                        note_data,
                    ))
                },
            )?;
            out.push(entry);
        }
        Ok(out)
    });
    // Seal: every raw buffer minted here is freed by
    // [`tramp_load_shielded_notes_free`].
    let sealed = built.map(|rows| {
        rows.into_iter()
            .map(|(mut e, note_data)| {
                (e.note_data_ptr, e.note_data_len) = vec_into_raw(note_data);
                e
            })
            .collect()
    });
    finish_load(sealed, out_entries, out_count)
}

#[cfg(feature = "shielded")]
unsafe extern "C" fn tramp_load_shielded_notes_free(
    _context: *mut c_void,
    entries: *const ShieldedNoteRestoreFFI,
    count: usize,
) {
    free_boxed_slice(entries, count, |e| {
        free_raw_bytes(e.note_data_ptr, e.note_data_len)
    });
}

#[cfg(feature = "shielded")]
unsafe extern "C" fn tramp_load_shielded_outgoing_notes(
    context: *mut c_void,
    out_entries: *mut *const ShieldedOutgoingNoteRestoreFFI,
    out_count: *mut usize,
) -> i32 {
    let built = with_bridge_load(context, |env, bridge| {
        let arr: jni::objects::JObjectArray = env
            .call_method(
                bridge,
                "onLoadShieldedOutgoingNotes",
                "()[Lorg/dashfoundation/dashsdk/ffi/ShieldedOutgoingNoteData;",
                &[],
            )?
            .l()?
            .into();
        let len = env.get_array_length(&arr)? as usize;
        // Staged rows: raw buffers are minted only after every JNI read
        // succeeded, so a mid-load failure drops owned Vecs and leaks nothing.
        let mut out: Vec<(ShieldedOutgoingNoteRestoreFFI, Vec<u8>)> = Vec::with_capacity(len);
        for i in 0..len {
            let entry = env.with_local_frame(
                64,
                |env| -> Result<(ShieldedOutgoingNoteRestoreFFI, Vec<u8>), jni::errors::Error> {
                    let h = env.get_object_array_element(&arr, i as i32)?;
                    let wallet_id = read_id32_field(env, &h, "walletId")?;
                    let account_index = env.get_field(&h, "accountIndex", "I")?.i()? as u32;
                    let cmx = read_id32_field(env, &h, "cmx")?;
                    let recipient = read_bytes_field_fixed::<43>(env, &h, "recipient")?;
                    let value = env.get_field(&h, "value", "J")?.j()? as u64;
                    let block_height = env.get_field(&h, "blockHeight", "J")?.j()? as u64;
                    let memo = read_bytes_field_vec(env, &h, "memo")?;
                    Ok((
                        ShieldedOutgoingNoteRestoreFFI {
                            wallet_id,
                            account_index,
                            cmx,
                            recipient,
                            value,
                            block_height,
                            memo_ptr: ptr::null(),
                            memo_len: 0,
                        },
                        memo,
                    ))
                },
            )?;
            out.push(entry);
        }
        Ok(out)
    });
    // Seal: every raw buffer minted here is freed by
    // [`tramp_load_shielded_outgoing_notes_free`].
    let sealed = built.map(|rows| {
        rows.into_iter()
            .map(|(mut e, memo)| {
                (e.memo_ptr, e.memo_len) = vec_into_raw(memo);
                e
            })
            .collect()
    });
    finish_load(sealed, out_entries, out_count)
}

#[cfg(feature = "shielded")]
unsafe extern "C" fn tramp_load_shielded_outgoing_notes_free(
    _context: *mut c_void,
    entries: *const ShieldedOutgoingNoteRestoreFFI,
    count: usize,
) {
    free_boxed_slice(entries, count, |e| free_raw_bytes(e.memo_ptr, e.memo_len));
}

#[cfg(feature = "shielded")]
unsafe extern "C" fn tramp_load_shielded_sync_states(
    context: *mut c_void,
    out_entries: *mut *const ShieldedSubwalletSyncStateFFI,
    out_count: *mut usize,
) -> i32 {
    let built = with_bridge_load(context, |env, bridge| {
        let arr: jni::objects::JObjectArray = env
            .call_method(
                bridge,
                "onLoadShieldedSyncStates",
                "()[Lorg/dashfoundation/dashsdk/ffi/ShieldedSyncStateData;",
                &[],
            )?
            .l()?
            .into();
        let len = env.get_array_length(&arr)? as usize;
        let mut out: Vec<ShieldedSubwalletSyncStateFFI> = Vec::with_capacity(len);
        for i in 0..len {
            let entry = env.with_local_frame(
                32,
                |env| -> Result<ShieldedSubwalletSyncStateFFI, jni::errors::Error> {
                    let h = env.get_object_array_element(&arr, i as i32)?;
                    let wallet_id = read_id32_field(env, &h, "walletId")?;
                    let account_index = env.get_field(&h, "accountIndex", "I")?.i()? as u32;
                    let last_synced_index = env.get_field(&h, "lastSyncedIndex", "J")?.j()? as u64;
                    Ok(ShieldedSubwalletSyncStateFFI {
                        wallet_id,
                        account_index,
                        last_synced_index,
                    })
                },
            )?;
            out.push(entry);
        }
        Ok(out)
    });
    finish_load(built, out_entries, out_count)
}

#[cfg(feature = "shielded")]
unsafe extern "C" fn tramp_load_shielded_sync_states_free(
    _context: *mut c_void,
    entries: *const ShieldedSubwalletSyncStateFFI,
    count: usize,
) {
    free_boxed_slice(entries, count, |_| {});
}

/// Owned staging for the four variable-length buffers of one shielded
/// activity row; raw pointers are minted only after the whole load
/// succeeded, so a mid-load failure drops plain `Vec`s and leaks nothing.
#[cfg(feature = "shielded")]
struct ShieldedActivityBuffersStaged {
    counterparty: Vec<u8>,
    memo: Vec<u8>,
    note_cmxs: Vec<u8>,
    spent_nullifiers: Vec<u8>,
}

#[cfg(feature = "shielded")]
unsafe extern "C" fn tramp_load_shielded_activity(
    context: *mut c_void,
    out_entries: *mut *const ShieldedActivityRestoreFFI,
    out_count: *mut usize,
) -> i32 {
    let built = with_bridge_load(context, |env, bridge| {
        let arr: jni::objects::JObjectArray = env
            .call_method(
                bridge,
                "onLoadShieldedActivity",
                "()[Lorg/dashfoundation/dashsdk/ffi/ShieldedActivityData;",
                &[],
            )?
            .l()?
            .into();
        let len = env.get_array_length(&arr)? as usize;
        type StagedActivity = (ShieldedActivityRestoreFFI, ShieldedActivityBuffersStaged);
        let mut out: Vec<StagedActivity> = Vec::with_capacity(len);
        for i in 0..len {
            let entry =
                env.with_local_frame(80, |env| -> Result<StagedActivity, jni::errors::Error> {
                    let h = env.get_object_array_element(&arr, i as i32)?;
                    let wallet_id = read_id32_field(env, &h, "walletId")?;
                    let account_index = env.get_field(&h, "accountIndex", "I")?.i()? as u32;
                    let entry_id = read_id32_field(env, &h, "entryId")?;
                    let kind_tag = env.get_field(&h, "kindTag", "B")?.b()? as u8;
                    let direction = env.get_field(&h, "direction", "B")?.b()? as u8;
                    let status = env.get_field(&h, "status", "B")?.b()? as u8;
                    let amount = env.get_field(&h, "amount", "J")?.j()? as u64;
                    let fee = env.get_field(&h, "fee", "J")?.j()? as u64;
                    let has_fee = env.get_field(&h, "hasFee", "Z")?.z()? as u8;
                    let block_height = env.get_field(&h, "blockHeight", "J")?.j()? as u64;
                    let has_block_height = env.get_field(&h, "hasBlockHeight", "Z")?.z()? as u8;
                    let created_at_ms = env.get_field(&h, "createdAtMs", "J")?.j()? as u64;
                    let has_identity_id = env.get_field(&h, "hasIdentityId", "Z")?.z()? as u8;
                    let identity_id = if has_identity_id != 0 {
                        read_id32_field(env, &h, "identityId")?
                    } else {
                        [0u8; 32]
                    };
                    let buffers = ShieldedActivityBuffersStaged {
                        counterparty: read_bytes_field_vec(env, &h, "counterparty")?,
                        memo: read_bytes_field_vec(env, &h, "memo")?,
                        note_cmxs: read_bytes_field_vec(env, &h, "noteCmxs")?,
                        spent_nullifiers: read_bytes_field_vec(env, &h, "spentNullifiers")?,
                    };
                    Ok((
                        ShieldedActivityRestoreFFI {
                            wallet_id,
                            account_index,
                            entry_id,
                            kind_tag,
                            direction,
                            status,
                            amount,
                            fee,
                            has_fee,
                            block_height,
                            has_block_height,
                            created_at_ms,
                            identity_id,
                            has_identity_id,
                            counterparty_ptr: ptr::null(),
                            counterparty_len: 0,
                            memo_ptr: ptr::null(),
                            memo_len: 0,
                            note_cmxs_ptr: ptr::null(),
                            note_cmxs_count: 0,
                            spent_nullifiers_ptr: ptr::null(),
                            spent_nullifiers_count: 0,
                        },
                        buffers,
                    ))
                })?;
            out.push(entry);
        }
        Ok(out)
    });
    // Seal: every raw buffer minted here is freed by
    // [`tramp_load_shielded_activity_free`].
    let sealed = built.map(|rows| {
        rows.into_iter()
            .map(|(mut e, bufs)| {
                (e.counterparty_ptr, e.counterparty_len) = vec_into_raw(bufs.counterparty);
                (e.memo_ptr, e.memo_len) = vec_into_raw(bufs.memo);
                (e.note_cmxs_ptr, e.note_cmxs_count) = vec_into_raw_32byte_items(bufs.note_cmxs);
                (e.spent_nullifiers_ptr, e.spent_nullifiers_count) =
                    vec_into_raw_32byte_items(bufs.spent_nullifiers);
                e
            })
            .collect()
    });
    finish_load(sealed, out_entries, out_count)
}

/// `Vec<u8>` of packed 32-byte items → `(*const u8, item_count)`. The free
/// trampoline reconstructs a `count * 32`-byte allocation, so any ragged
/// tail is trimmed to keep the allocation exactly that size.
#[cfg(feature = "shielded")]
fn vec_into_raw_32byte_items(mut bytes: Vec<u8>) -> (*const u8, usize) {
    let count = bytes.len() / 32;
    bytes.truncate(count * 32);
    let (ptr, _len) = vec_into_raw(bytes);
    (ptr, count)
}

#[cfg(feature = "shielded")]
unsafe extern "C" fn tramp_load_shielded_activity_free(
    _context: *mut c_void,
    entries: *const ShieldedActivityRestoreFFI,
    count: usize,
) {
    free_boxed_slice(entries, count, |e| {
        free_raw_bytes(e.counterparty_ptr, e.counterparty_len);
        free_raw_bytes(e.memo_ptr, e.memo_len);
        free_raw_bytes(e.note_cmxs_ptr, e.note_cmxs_count.saturating_mul(32));
        free_raw_bytes(
            e.spent_nullifiers_ptr,
            e.spent_nullifiers_count.saturating_mul(32),
        );
    });
}

// ── get_core_tx_record ────────────────────────────────────────────────

unsafe extern "C" fn tramp_get_core_tx_record(
    context: *mut c_void,
    wallet_id: *const u8,
    txid: *const u8,
    out_context_kind: *mut u8,
    out_block_height: *mut u32,
    out_block_hash: *mut u8,
    out_block_timestamp: *mut u32,
    out_tx_bytes: *mut *const u8,
    out_tx_bytes_len: *mut usize,
    out_found: *mut bool,
) -> i32 {
    // Defaults: not found.
    *out_found = false;
    *out_tx_bytes = ptr::null();
    *out_tx_bytes_len = 0;

    let record = with_bridge_load(context, |env, bridge| {
        let wid = id32(env, wallet_id)?;
        let tid = id32(env, txid)?;
        let obj = env
            .call_method(
                bridge,
                "onGetCoreTxRecord",
                "([B[B)Lorg/dashfoundation/dashsdk/ffi/CoreTxRecordData;",
                &[(&wid).into(), (&tid).into()],
            )?
            .l()?;
        if obj.is_null() {
            return Ok(None);
        }
        let context_kind = env.get_field(&obj, "contextKind", "B")?.b()? as u8;
        let block_height = env.get_field(&obj, "blockHeight", "I")?.i()? as u32;
        let block_hash = read_id32_field(env, &obj, "blockHash")?;
        let block_timestamp = env.get_field(&obj, "blockTimestamp", "I")?.i()? as u32;
        // Staged as an owned Vec: the raw buffer is minted only in the
        // all-reads-succeeded arm below, so a JNI failure leaks nothing.
        let tx_bytes = read_bytes_field_vec(env, &obj, "txBytes")?;
        Ok(Some((
            context_kind,
            block_height,
            block_hash,
            block_timestamp,
            tx_bytes,
        )))
    });

    match record {
        // JNI failure — treated as a transient miss (return non-zero so
        // the Rust proof flow surfaces None).
        None => ERR_JNI,
        Some(None) => {
            // No row for this txid.
            0
        }
        Some(Some((kind, height, hash, ts, tx_bytes))) => {
            *out_found = true;
            *out_context_kind = kind;
            *out_block_height = height;
            *out_block_timestamp = ts;
            if !out_block_hash.is_null() {
                ptr::copy_nonoverlapping(hash.as_ptr(), out_block_hash, 32);
            }
            // Raw buffer minted only now; freed by
            // [`tramp_get_core_tx_record_free`].
            (*out_tx_bytes, *out_tx_bytes_len) = vec_into_raw(tx_bytes);
            0
        }
    }
}

unsafe extern "C" fn tramp_get_core_tx_record_free(
    _context: *mut c_void,
    tx_bytes: *const u8,
    tx_bytes_len: usize,
) {
    let _ = catch_unwind(AssertUnwindSafe(|| {
        free_raw_bytes(tx_bytes, tx_bytes_len);
    }));
}

// ── Shared low-level helpers ──────────────────────────────────────────

/// FFINetwork ordinal → the crate's `FFINetwork` enum. Out-of-range
/// ordinals fall back to Testnet (matches sdk.rs's default arm).
fn net_from_ord(ord: i32) -> platform_wallet_ffi::FFINetwork {
    use platform_wallet_ffi::FFINetwork;
    match ord {
        0 => FFINetwork::Mainnet,
        2 => FFINetwork::Devnet,
        3 => FFINetwork::Regtest,
        _ => FFINetwork::Testnet,
    }
}

/// Build a `&[T]` from a raw `(ptr, count)` pair, treating null as empty.
///
/// # Safety
/// `ptr` must point to `count` valid `T`s (or be null with `count == 0`).
unsafe fn slice_or_empty<'a, T>(ptr: *const T, count: usize) -> &'a [T] {
    if ptr.is_null() || count == 0 {
        &[]
    } else {
        std::slice::from_raw_parts(ptr, count)
    }
}

/// `Vec<T>` → `(*const T, len)`; empty vec yields `(null, 0)`. A non-null
/// pointer is a leaked `Box<[T]>` the matching load-free trampoline
/// reconstructs and drops — mint it only once the whole load succeeded.
fn vec_into_raw<T>(v: Vec<T>) -> (*const T, usize) {
    if v.is_empty() {
        (ptr::null(), 0)
    } else {
        let len = v.len();
        (Box::into_raw(v.into_boxed_slice()) as *const T, len)
    }
}

/// Free a `Box<[u8]>` buffer minted by [`vec_into_raw`]. No-op on null/0.
unsafe fn free_raw_bytes(ptr: *const u8, len: usize) {
    if !ptr.is_null() && len > 0 {
        drop(Box::from_raw(std::ptr::slice_from_raw_parts_mut(
            ptr as *mut u8,
            len,
        )));
    }
}

/// Free a `Box<[T]>` buffer minted by [`vec_into_raw`] (non-`u8` element
/// types — `u32` accepted-account buffers, `[u8; 32]` ignored-sender
/// arrays). No-op on null/0.
unsafe fn free_raw_slice<T>(ptr: *const T, len: usize) {
    if !ptr.is_null() && len > 0 {
        drop(Box::from_raw(std::ptr::slice_from_raw_parts_mut(
            ptr as *mut T,
            len,
        )));
    }
}

/// Hand an optional owned `CString` to an FFI struct field (`None` →
/// null). The paired [`free_raw_cstring`] reclaims it via
/// `CString::from_raw` — mint only once the whole load succeeded.
fn opt_cstring_into_raw(c: Option<CString>) -> *const c_char {
    match c {
        Some(c) => c.into_raw() as *const c_char,
        None => ptr::null(),
    }
}

/// Reclaim a C string minted by [`opt_cstring_into_raw`]. No-op on null.
unsafe fn free_raw_cstring(ptr: *const c_char) {
    if !ptr.is_null() {
        drop(CString::from_raw(ptr as *mut c_char));
    }
}

/// Reconstruct a `Box<[T]>` from a load-callback allocation, run
/// `per_entry` for each element (to free its nested buffers), then drop it.
// Only the shielded load-free trampolines call this; without the feature
// the fn is dead but kept for a single code path.
#[cfg_attr(not(feature = "shielded"), allow(dead_code))]
unsafe fn free_boxed_slice<T>(ptr: *const T, count: usize, per_entry: impl Fn(&T)) {
    let _ = catch_unwind(AssertUnwindSafe(|| {
        if ptr.is_null() || count == 0 {
            return;
        }
        let boxed: Box<[T]> =
            Box::from_raw(std::ptr::slice_from_raw_parts_mut(ptr as *mut T, count));
        for e in boxed.iter() {
            per_entry(e);
        }
        drop(boxed);
    }));
}

/// Common tail for load trampolines: publish the boxed vec or signal miss.
///
/// # Safety
/// `out_entries` and `out_count` must be valid for writes.
#[cfg_attr(not(feature = "shielded"), allow(dead_code))]
unsafe fn finish_load<T>(
    built: Option<Vec<T>>,
    out_entries: *mut *const T,
    out_count: *mut usize,
) -> i32 {
    match built {
        Some(v) => {
            let (ptr, count) = vec_into_raw(v);
            *out_entries = ptr;
            *out_count = count;
            0
        }
        None => {
            *out_entries = ptr::null();
            *out_count = 0;
            ERR_JNI
        }
    }
}

/// Read a `ByteArray` field into a fixed `[u8; 32]`. Null stays the
/// all-zero absent sentinel; any other length fails the load (see
/// [`read_bytes_field_fixed`]).
fn read_id32_field(
    env: &mut JNIEnv,
    holder: &JObject,
    field: &str,
) -> Result<[u8; 32], jni::errors::Error> {
    read_bytes_field_fixed::<32>(env, holder, field)
}

/// Read an OPTIONAL 32-byte id field where the Kotlin holder uses a
/// non-null `ByteArray(0)` as the absent sentinel (e.g. the DashPay
/// `userIdentityId`/`friendIdentityId` on ordinary accounts). Empty and
/// null both map to the all-zero id; any other non-32 length still
/// fails the load.
fn read_optional_id32_field(
    env: &mut JNIEnv,
    holder: &JObject,
    field: &str,
) -> Result<[u8; 32], jni::errors::Error> {
    let obj = env.get_field(holder, field, "[B")?.l()?;
    if obj.is_null() {
        return Ok([0u8; 32]);
    }
    let arr: JByteArray = obj.into();
    if env.get_array_length(&arr)? == 0 {
        return Ok([0u8; 32]);
    }
    read_bytes_field_fixed::<32>(env, holder, field)
}

fn read_bytes_field_fixed<const N: usize>(
    env: &mut JNIEnv,
    holder: &JObject,
    field: &str,
) -> Result<[u8; N], jni::errors::Error> {
    let obj = env.get_field(holder, field, "[B")?.l()?;
    let mut out = [0u8; N];
    if obj.is_null() {
        return Ok(out);
    }
    let arr: JByteArray = obj.into();
    let len = env.get_array_length(&arr)? as usize;
    // Any wrong length — including ByteArray(0) — must fail the load
    // (surfaced as ERR_JNI by the load trampoline): zero-padding or
    // truncating would silently rehydrate the row under an altered key.
    // Fields with a legitimate empty absent sentinel go through
    // read_optional_id32_field instead.
    if len != N {
        log::error!("load: field `{field}` expected {N} bytes, got {len}");
        return Err(jni::errors::Error::WrongJValueType(
            "byte[] of expected fixed length",
            "byte[] of mismatched length",
        ));
    }
    let mut buf = vec![0i8; N];
    env.get_byte_array_region(&arr, 0, &mut buf)?;
    for (i, b) in buf.iter().enumerate() {
        out[i] = *b as u8;
    }
    Ok(out)
}

/// Read an `IntArray` field into an owned `Vec<u32>` (bit-pattern cast —
/// inverse of the persist-side [`int_array`] projection); null / empty →
/// empty vec.
fn read_u32_array_field(
    env: &mut JNIEnv,
    holder: &JObject,
    field: &str,
) -> Result<Vec<u32>, jni::errors::Error> {
    let obj = env.get_field(holder, field, "[I")?.l()?;
    if obj.is_null() {
        return Ok(Vec::new());
    }
    let arr: jni::objects::JIntArray = obj.into();
    let len = env.get_array_length(&arr)? as usize;
    if len == 0 {
        return Ok(Vec::new());
    }
    let mut buf = vec![0i32; len];
    env.get_int_array_region(&arr, 0, &mut buf)?;
    Ok(buf.into_iter().map(|v| v as u32).collect())
}

/// Read an `Array<ByteArray>` field of 32-byte ids into an owned
/// `Vec<[u8; 32]>`; a null field → empty vec. Any element that is not
/// exactly 32 bytes fails the load (same altered-key rationale as
/// [`read_bytes_field_fixed`]).
fn read_id32_array_field(
    env: &mut JNIEnv,
    holder: &JObject,
    field: &str,
) -> Result<Vec<[u8; 32]>, jni::errors::Error> {
    let obj = env.get_field(holder, field, "[[B")?.l()?;
    if obj.is_null() {
        return Ok(Vec::new());
    }
    let arr: jni::objects::JObjectArray = obj.into();
    let len = env.get_array_length(&arr)? as usize;
    let mut out: Vec<[u8; 32]> = Vec::with_capacity(len);
    for i in 0..len {
        let id = env.with_local_frame(4, |env| -> Result<[u8; 32], jni::errors::Error> {
            let element = env.get_object_array_element(&arr, i as i32)?;
            let bytes: JByteArray = element.into();
            let blen = env.get_array_length(&bytes)? as usize;
            if blen != 32 {
                log::error!("load: field `{field}`[{i}] expected 32 bytes, got {blen}");
                return Err(jni::errors::Error::WrongJValueType(
                    "byte[] of expected fixed length",
                    "byte[] of mismatched length",
                ));
            }
            let mut buf = [0i8; 32];
            env.get_byte_array_region(&bytes, 0, &mut buf)?;
            let mut id = [0u8; 32];
            for (dst, src) in id.iter_mut().zip(buf.iter()) {
                *dst = *src as u8;
            }
            Ok(id)
        })?;
        out.push(id);
    }
    Ok(out)
}

/// Read a `ByteArray` field into an owned `Vec<u8>`; null / empty → empty
/// vec. Load trampolines stage variable-length fields through this and
/// mint the raw buffer via [`vec_into_raw`] (which maps empty back to
/// `(null, 0)`) only after the whole load succeeded.
fn read_bytes_field_vec(
    env: &mut JNIEnv,
    holder: &JObject,
    field: &str,
) -> Result<Vec<u8>, jni::errors::Error> {
    let obj = env.get_field(holder, field, "[B")?.l()?;
    if obj.is_null() {
        return Ok(Vec::new());
    }
    let arr: JByteArray = obj.into();
    let len = env.get_array_length(&arr)? as usize;
    if len == 0 {
        return Ok(Vec::new());
    }
    let mut buf = vec![0i8; len];
    env.get_byte_array_region(&arr, 0, &mut buf)?;
    Ok(buf.into_iter().map(|b| b as u8).collect())
}
