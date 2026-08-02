//! JNI bridge for the platform-wallet-ffi `EventHandlerCallbacks` vtable.
//!
//! Kotlin counterpart: `org.dashfoundation.dashsdk.ffi.NativeWalletEventBridge`,
//! reached from the manager built in [`crate::wallet_manager`].
//!
//! ## Trampoline contract (same discipline as `persistence.rs`)
//!
//! 1. **Attach the thread.** Completion / progress callbacks fire on Tokio
//!    worker threads ART knows nothing about, so each trampoline attaches
//!    via `JavaVM::attach_current_thread_as_daemon()` (daemon = never
//!    detach; ART reaps the attachment on thread death).
//! 2. **Never unwind.** The whole body runs under `catch_unwind`; a panic is
//!    swallowed rather than unwinding across the C ABI (UB). Events are
//!    fire-and-forget (`void` return), so there is no error code to report —
//!    a dropped event just means the host's poll fallback picks up liveness.
//! 3. **Copy before returning.** Every Rust-owned payload (`[u8; 32]` ids,
//!    `*const c_char` error messages, the result slice itself) is copied
//!    into JVM objects before the trampoline returns — the FFI pointers are
//!    valid only for the callback window.
//! 4. **Fan out per entry.** The completion callbacks hand a `results`
//!    array; we make one flat `call_method` per entry (mirroring how
//!    `persistence.rs` walks its changeset slices), plus a single boundary
//!    call marking the pass boundary with its unix timestamp. Kotlin never
//!    touches native memory.

#![allow(clippy::missing_safety_doc)]

use crate::support::JVM;
use jni::objects::{GlobalRef, JByteArray, JObject, JValue};
use jni::JNIEnv;
use platform_wallet_ffi::event_handler::EventHandlerCallbacks;
use platform_wallet_ffi::platform_address_sync::PlatformAddressSyncWalletResultFFI;
use platform_wallet_ffi::shielded_types::ShieldedSyncWalletResultFFI;
use std::ffi::{c_void, CStr};
use std::os::raw::c_char;
use std::panic::{catch_unwind, AssertUnwindSafe};

// ── Context ───────────────────────────────────────────────────────────

/// Boxed context for the event-handler vtable. Holds the Kotlin
/// `NativeWalletEventBridge` as a `GlobalRef` so it survives across the
/// vtable's lifetime and across threads. Ownership transfers to the
/// native manager at create (the vtable's `release_fn` is
/// [`release_event_ctx`]): Rust frees the box — and with it the
/// `GlobalRef` — exactly once, when the manager and every worker that
/// could still dispatch an event have dropped their references.
pub(crate) struct KotlinEventCtx {
    pub(crate) bridge: GlobalRef,
}

impl KotlinEventCtx {
    pub(crate) fn new(bridge: GlobalRef) -> Self {
        Self { bridge }
    }
}

// SAFETY: `GlobalRef` is valid from any attached thread; trampolines
// re-attach per call.
unsafe impl Send for KotlinEventCtx {}
unsafe impl Sync for KotlinEventCtx {}

// ── Attach helper ─────────────────────────────────────────────────────

/// Attach the current (Tokio) thread and hand `f` the env + the bridge
/// object. Any pending exception left by `f` (or an attach failure) is
/// cleared; the whole thing runs under `catch_unwind`. Fire-and-forget —
/// no value is returned to Rust.
///
/// # Safety
/// `context` must be a live `KotlinEventCtx` pointer.
unsafe fn with_bridge<F>(context: *mut c_void, f: F)
where
    F: FnOnce(&mut JNIEnv, &JObject) -> Result<(), jni::errors::Error>,
{
    let _ = catch_unwind(AssertUnwindSafe(|| {
        if context.is_null() {
            return;
        }
        let Some(vm) = JVM.get() else { return };
        let Ok(mut env) = vm.attach_current_thread_as_daemon() else {
            return;
        };
        let ctx = &*(context as *const KotlinEventCtx);
        let bridge = ctx.bridge.as_obj();
        let env: &mut JNIEnv = &mut env;
        if env.with_local_frame(32, |env| f(env, bridge)).is_err() {
            let _ = env.exception_clear();
        }
    }));
}

/// `count` items behind `ptr`, or an empty slice when null / zero.
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

/// Copy a 32-byte id into a JVM `byte[]`.
fn id32<'l>(env: &JNIEnv<'l>, id: &[u8; 32]) -> Result<JByteArray<'l>, jni::errors::Error> {
    env.byte_array_from_slice(id)
}

/// Copy a NUL-terminated C string into an optional JVM `String`; null → JVM null.
fn cstr_opt<'l>(env: &JNIEnv<'l>, ptr: *const c_char) -> Result<JObject<'l>, jni::errors::Error> {
    if ptr.is_null() {
        return Ok(JObject::null());
    }
    // SAFETY: the FFI guarantees the message is a valid CString for the
    // callback window.
    let s = unsafe { CStr::from_ptr(ptr) }.to_string_lossy();
    Ok(env.new_string(s)?.into())
}

// ── on_wallet_event / on_error (ABI-simple) ───────────────────────────

/// `on_wallet_event_fn`: Debug-formatted UTF-8 bytes (not necessarily
/// NUL-terminated) → `NativeWalletEventBridge.onWalletEvent(String)`.
unsafe extern "C" fn tramp_wallet_event(
    context: *mut c_void,
    event_json: *const u8,
    event_json_len: usize,
) {
    with_bridge(context, |env, bridge| {
        let text = if event_json.is_null() {
            String::new()
        } else {
            let bytes = std::slice::from_raw_parts(event_json, event_json_len);
            String::from_utf8_lossy(bytes).into_owned()
        };
        let jtext = env.new_string(text)?;
        env.call_method(
            bridge,
            "onWalletEvent",
            "(Ljava/lang/String;)V",
            &[(&jtext).into()],
        )?;
        Ok(())
    });
}

/// `on_error_fn`: NUL-terminated C string → `NativeWalletEventBridge.onError(String)`.
unsafe extern "C" fn tramp_error(context: *mut c_void, error_msg: *const c_char) {
    with_bridge(context, |env, bridge| {
        let text = if error_msg.is_null() {
            String::new()
        } else {
            CStr::from_ptr(error_msg).to_string_lossy().into_owned()
        };
        let jtext = env.new_string(text)?;
        env.call_method(
            bridge,
            "onError",
            "(Ljava/lang/String;)V",
            &[(&jtext).into()],
        )?;
        Ok(())
    });
}

// ── on_platform_address_sync_completed ────────────────────────────────

/// One flat call per wallet result, then a boundary call marking the pass
/// with its unix timestamp + the total wallet count.
unsafe extern "C" fn tramp_platform_address_sync_completed(
    context: *mut c_void,
    results: *const PlatformAddressSyncWalletResultFFI,
    count: usize,
    sync_unix_seconds: u64,
) {
    with_bridge(context, |env, bridge| {
        for r in slice_or_empty(results, count) {
            env.with_local_frame(16, |env| -> Result<(), jni::errors::Error> {
                let wid = id32(env, &r.wallet_id)?;
                let err = cstr_opt(env, r.error_message)?;
                env.call_method(
                    bridge,
                    "onPlatformAddressSyncCompleted",
                    "([BZJJJJJJLjava/lang/String;)V",
                    &[
                        (&wid).into(),
                        JValue::Bool(r.success as u8),
                        JValue::Long(r.found_count as i64),
                        JValue::Long(r.absent_count as i64),
                        JValue::Long(r.checkpoint_height as i64),
                        JValue::Long(r.new_sync_height as i64),
                        JValue::Long(r.new_sync_timestamp as i64),
                        JValue::Long(r.last_known_recent_block as i64),
                        (&err).into(),
                    ],
                )?;
                Ok(())
            })?;
        }
        env.call_method(
            bridge,
            "onPlatformAddressSyncPassCompleted",
            "(JI)V",
            &[
                JValue::Long(sync_unix_seconds as i64),
                JValue::Int(count as i32),
            ],
        )?;
        Ok(())
    });
}

// ── on_shielded_sync_completed ────────────────────────────────────────

unsafe extern "C" fn tramp_shielded_sync_completed(
    context: *mut c_void,
    results: *const ShieldedSyncWalletResultFFI,
    count: usize,
    sync_unix_seconds: u64,
) {
    with_bridge(context, |env, bridge| {
        for r in slice_or_empty(results, count) {
            env.with_local_frame(16, |env| -> Result<(), jni::errors::Error> {
                let wid = id32(env, &r.wallet_id)?;
                let err = cstr_opt(env, r.error_message)?;
                env.call_method(
                    bridge,
                    "onShieldedSyncCompleted",
                    "([BZZZIJIJLjava/lang/String;)V",
                    &[
                        (&wid).into(),
                        JValue::Bool(r.success as u8),
                        JValue::Bool(r.skipped as u8),
                        JValue::Bool(r.cooldown_skip as u8),
                        JValue::Int(r.new_notes as i32),
                        JValue::Long(r.total_scanned as i64),
                        JValue::Int(r.newly_spent as i32),
                        JValue::Long(r.balance as i64),
                        (&err).into(),
                    ],
                )?;
                Ok(())
            })?;
        }
        env.call_method(
            bridge,
            "onShieldedSyncPassCompleted",
            "(JI)V",
            &[
                JValue::Long(sync_unix_seconds as i64),
                JValue::Int(count as i32),
            ],
        )?;
        Ok(())
    });
}

// ── on_shielded_sync_progress / on_shielded_tree_progress ─────────────

unsafe extern "C" fn tramp_shielded_sync_progress(
    context: *mut c_void,
    cumulative_scanned: u64,
    block_height: u64,
) {
    with_bridge(context, |env, bridge| {
        env.call_method(
            bridge,
            "onShieldedSyncProgress",
            "(JJ)V",
            &[
                JValue::Long(cumulative_scanned as i64),
                JValue::Long(block_height as i64),
            ],
        )?;
        Ok(())
    });
}

unsafe extern "C" fn tramp_shielded_tree_progress(
    context: *mut c_void,
    leaves_committed: u64,
    total_target: u64,
) {
    with_bridge(context, |env, bridge| {
        env.call_method(
            bridge,
            "onShieldedTreeProgress",
            "(JJ)V",
            &[
                JValue::Long(leaves_committed as i64),
                JValue::Long(total_target as i64),
            ],
        )?;
        Ok(())
    });
}

// ── Vtable ────────────────────────────────────────────────────────────

/// Build the full event vtable pointing at `context` (a boxed
/// [`KotlinEventCtx`]). Every slot is wired: the two ABI-simple event /
/// error slots, plus the platform-address + shielded completion / progress
/// slots that marshal their payload arrays into per-entry flat calls on the
/// Kotlin bridge.
pub(crate) fn build_event_vtable(context: *mut c_void) -> EventHandlerCallbacks {
    EventHandlerCallbacks {
        context,
        on_wallet_event_fn: Some(tramp_wallet_event),
        on_error_fn: Some(tramp_error),
        on_platform_address_sync_completed_fn: Some(tramp_platform_address_sync_completed),
        on_shielded_sync_completed_fn: Some(tramp_shielded_sync_completed),
        on_shielded_sync_progress_fn: Some(tramp_shielded_sync_progress),
        on_shielded_tree_progress_fn: Some(tramp_shielded_tree_progress),
        release_fn: Some(release_event_ctx),
    }
}

/// `release_fn` for the event vtable: frees the boxed [`KotlinEventCtx`]
/// when the native manager's last event-handler reference drops. The FFI
/// guarantees exactly one call, which may land on any Rust thread —
/// `GlobalRef`'s own `Drop` attaches that thread to the JVM before
/// deleting the reference, so no manual attach is needed here.
///
/// # Safety
/// `context` must be the live boxed [`KotlinEventCtx`] this vtable was
/// built around, never freed elsewhere.
unsafe extern "C" fn release_event_ctx(context: *mut c_void) {
    if !context.is_null() {
        drop(Box::from_raw(context as *mut KotlinEventCtx));
    }
}
