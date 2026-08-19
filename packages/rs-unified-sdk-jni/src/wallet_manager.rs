//! JNI bridge for the platform-wallet `PlatformWalletManager` lifecycle
//! and per-wallet accessors.
//!
//! Kotlin counterpart: `org.dashfoundation.dashsdk.ffi.WalletManagerNative`
//! and `NativeWalletEventBridge`, driven by
//! `org.dashfoundation.dashsdk.wallet.PlatformWalletManager`.
//!
//! ## What the manager takes at construction
//!
//! `platform_wallet_manager_create_with_persistence_capabilities(...)`.
//! The mnemonic resolver and signer are **SDK-level, per-call** handles
//! (see `mnemonic.rs` / `signer.rs`) — they are NOT wired into the
//! manager, matching `PlatformWalletManager.swift`, which holds only a
//! persistence handler + event handler.
//!
//! ## Context ownership
//!
//! Both vtables are built with a `release_fn`, so
//! `platform_wallet_manager_create` takes **ownership** of the boxed
//! contexts (persistence bridge + event bridge as JNI `GlobalRef`s):
//! the native manager keeps each box alive for exactly as long as any
//! worker can still fire a callback through it, and frees it — on
//! whatever thread the last reference drops on — via the vtable's
//! `release_fn` (`GlobalRef`'s own `Drop` re-attaches the thread to the
//! JVM). [`Java_..._nativeDestroy`] therefore only destroys the manager;
//! it never touches the context boxes, and a worker that straggles past
//! destroy keeps its bridge alive instead of dereferencing a freed
//! `GlobalRef`. The create-failure path is the one place this JNI layer
//! still frees the boxes itself, because a failed create never took
//! ownership.
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

use crate::events::{build_event_extension, build_event_vtable, KotlinEventCtx};
use crate::persistence::{build_extension, build_vtable, KotlinPersistenceCtx};
use crate::support::{guard, take_pwffi_error, throw_sdk_exception, PWFFI_CODE_OFFSET};
use jni::objects::{JByteArray, JClass, JObject, JObjectArray, JString, JValue};
use jni::sys::{
    jboolean, jbyteArray, jdoubleArray, jint, jlong, jlongArray, jobject, jstring, JNI_FALSE,
    JNI_TRUE,
};
use jni::JNIEnv;
use platform_wallet_ffi::error::{
    platform_wallet_ffi_result_free, PlatformWalletFFIResult, PlatformWalletFFIResultCode,
};
use platform_wallet_ffi::event_handler::{EventHandlerCallbacks, EventHandlerCallbacksExtension};
use platform_wallet_ffi::handle::Handle;
use platform_wallet_ffi::persistence::{PersistenceCallbacks, PersistenceCapabilitiesFFI};
use platform_wallet_ffi::types::IdentifierArray;
use std::ffi::{c_void, CStr};
use std::os::raw::c_char;
use std::ptr;

#[cfg(feature = "shielded")]
use rs_sdk_ffi::MnemonicResolverHandle;
use rs_sdk_ffi::{dash_sdk_get_inner_sdk_ptr, SDKHandle};

// ── Manager bundle ────────────────────────────────────────────────────

/// Owns the native manager handle. The persistence/event context boxes
/// are owned by the native manager itself (their vtables carry a
/// `release_fn`), so the bundle no longer tracks them. Boxed and
/// returned to Kotlin as a single `jlong`; freed by
/// [`Java_..._nativeDestroy`].
struct ManagerBundle {
    manager_handle: Handle,
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

        // Read the backend's explicit semantic attestation before installing
        // any trampolines. NativePersistenceBridge defaults both methods to
        // zero, so a no-op subclass remains fail-closed even though JNI wires a
        // structurally complete callback table.
        let declared_capabilities_version = match env
            .call_method(
                &persistence_bridge,
                "persistenceCapabilitiesVersion",
                "()I",
                &[],
            )
            .and_then(|value| value.i())
        {
            Ok(value) => value as u32,
            Err(_) => {
                let _ = env.exception_clear();
                throw_sdk_exception(env, 99, "reading persistence capability version failed");
                return 0;
            }
        };
        let declared_capabilities_bits = match env
            .call_method(
                &persistence_bridge,
                "persistenceCapabilitiesBits",
                "()J",
                &[],
            )
            .and_then(|value| value.j())
        {
            Ok(value) => value as u64,
            Err(_) => {
                let _ = env.exception_clear();
                throw_sdk_exception(env, 99, "reading persistence capability mask failed");
                return 0;
            }
        };

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
        let persistence_extension = build_extension();
        let persistence_capabilities = PersistenceCapabilitiesFFI {
            version: declared_capabilities_version,
            reserved: 0,
            bits: declared_capabilities_bits,
        };

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
        let event_extension: EventHandlerCallbacksExtension = build_event_extension();

        let mut manager_handle: Handle = 0;
        // SAFETY: `inner` is a live Sdk pointer for the duration of this
        // call; the manager clones the Sdk and reads both vtables by value.
        let result = unsafe {
            platform_wallet_ffi::platform_wallet_manager_create_with_extensions(
                inner,
                &persistence as *const PersistenceCallbacks,
                &mut event_callbacks as *const EventHandlerCallbacks,
                &persistence_capabilities as *const PersistenceCapabilitiesFFI,
                &persistence_extension,
                &event_extension,
                &mut manager_handle as *mut Handle,
            )
        };
        if take_pwffi_error(env, result) {
            // Manager was not created, so it never took ownership of the
            // context boxes — reclaim them here (the only place this JNI
            // layer frees them; every success path leaves that to the
            // native manager's `release_fn`).
            unsafe {
                drop(Box::from_raw(persistence_ctx));
                drop(Box::from_raw(event_ctx));
            }
            return 0;
        }

        let bundle = Box::new(ManagerBundle { manager_handle });
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

/// Effective persistence capability contract exposed for managed-language
/// initialization diagnostics. Unknown/invalid bundles return zero after the
/// standard JNI error mapping.
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_WalletManagerNative_nativePersistenceCapabilitiesVersion(
    mut env: JNIEnv,
    _class: JClass,
    bundle: jlong,
) -> jint {
    guard(&mut env, 0, |env| {
        if bundle == 0 {
            throw_sdk_exception(env, 1, "manager bundle is 0");
            return 0;
        }
        let b = unsafe { &*(bundle as *const ManagerBundle) };
        let mut capabilities = PersistenceCapabilitiesFFI {
            version: 0,
            reserved: 0,
            bits: 0,
        };
        let result = unsafe {
            platform_wallet_ffi::platform_wallet_manager_persistence_capabilities(
                b.manager_handle,
                &mut capabilities,
            )
        };
        if take_pwffi_error(env, result) {
            return 0;
        }
        capabilities.version as jint
    })
}

#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_WalletManagerNative_nativePersistenceCapabilitiesBits(
    mut env: JNIEnv,
    _class: JClass,
    bundle: jlong,
) -> jlong {
    guard(&mut env, 0, |env| {
        if bundle == 0 {
            throw_sdk_exception(env, 1, "manager bundle is 0");
            return 0;
        }
        let b = unsafe { &*(bundle as *const ManagerBundle) };
        let mut capabilities = PersistenceCapabilitiesFFI {
            version: 0,
            reserved: 0,
            bits: 0,
        };
        let result = unsafe {
            platform_wallet_ffi::platform_wallet_manager_persistence_capabilities(
                b.manager_handle,
                &mut capabilities,
            )
        };
        if take_pwffi_error(env, result) {
            return 0;
        }
        capabilities.bits as jlong
    })
}

/// Destroy a manager bundle: shut down the native manager (bounded
/// quiesce + join of every callback-firing task). The persistence/event
/// context boxes are owned by the native manager, which frees them via
/// each vtable's `release_fn` once its last worker reference drops — at
/// destroy for a clean shutdown, or when a straggling worker finally
/// exits otherwise. Either way this layer has nothing to free and
/// nothing to deliberately leak. Safe on 0. Idempotent is the caller's
/// responsibility (Kotlin's `AtomicLong` handle guard calls this exactly
/// once).
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
        let mut result =
            unsafe { platform_wallet_ffi::platform_wallet_manager_destroy(b.manager_handle) };
        if result.code != PlatformWalletFFIResultCode::Success {
            log::error!("manager destroy failed with code {:?}", result.code);
        }
        unsafe { platform_wallet_ffi_result_free(&mut result) };
    })
}

// ── Exports: wallet creation / restore ────────────────────────────────

/// Shared body for the two create-from-mnemonic JNI exports. Always routes
/// through the platform-wallet `_with_birth_height` FFI: `(false, 0)` maps
/// to `None` (birth height resolved from SPV's confirmed header tip — the
/// fresh-wallet default, identical to the legacy no-override entry point),
/// `(true, h)` pins the SPV compact-filter scan start to height `h`
/// (imported/restored mnemonics pass `0` for a full historical scan).
/// The 32-byte wallet id is written into the caller-allocated
/// `out_wallet_id`; the created `PlatformWallet` handle is written into
/// `out_wallet_handle[0]`. `createDefaultAccounts` maps to account
/// options 1 (Default) / 0 (None).
// The arg list mirrors the platform-wallet `_with_birth_height` FFI surface
// (handle + phrase + network + account options + the birth-height override
// pair + the wallet-id/handle out-params); grouping them into a struct would
// only add indirection over a 1:1 FFI passthrough.
#[allow(clippy::too_many_arguments)]
fn create_wallet_from_mnemonic_impl(
    env: &mut JNIEnv,
    manager_handle: jlong,
    mnemonic: &JString,
    network: jni::sys::jint,
    create_default_accounts: jboolean,
    has_birth_height_override: bool,
    birth_height_override: u32,
    out_wallet_handle: &jni::objects::JLongArray,
    out_wallet_id: &jni::objects::JByteArray,
) {
    // Validate BOTH caller-allocated out-buffers BEFORE the native create.
    // Creation synchronously commits wallet metadata, accounts and address
    // pools through the persistence callbacks, and
    // `platform_wallet_manager_remove_wallet` only unregisters — it fires
    // NO persistence-deletion callback — so nothing fallible may sit
    // between a successful create and the handle/id reaching Kotlin. With
    // the bounds checked here, the post-create region writes below cannot
    // fail (they allocate nothing).
    if out_wallet_handle.is_null()
        || env
            .get_array_length(out_wallet_handle)
            .map_or(true, |len| len < 1)
    {
        let _ = env.exception_clear();
        throw_sdk_exception(env, 1, "outWalletHandle must be a non-null long[1]");
        return;
    }
    if out_wallet_id.is_null()
        || env
            .get_array_length(out_wallet_id)
            .map_or(true, |len| len != 32)
    {
        let _ = env.exception_clear();
        throw_sdk_exception(env, 1, "outWalletId must be a non-null byte[32]");
        return;
    }
    let phrase: String = match env.get_string(mnemonic) {
        Ok(s) => s.into(),
        Err(_) => {
            let _ = env.exception_clear();
            throw_sdk_exception(env, 1, "mnemonic string was null/invalid");
            return;
        }
    };
    let c_phrase = match std::ffi::CString::new(phrase) {
        Ok(c) => c,
        Err(_) => {
            throw_sdk_exception(env, 1, "mnemonic contained an interior NUL");
            return;
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
        platform_wallet_ffi::platform_wallet_manager_create_wallet_from_mnemonic_with_birth_height(
            manager_handle as Handle,
            c_phrase.as_ptr(),
            ffi_network(network),
            account_options,
            has_birth_height_override,
            birth_height_override,
            &mut wallet_handle as *mut Handle,
            &mut wallet_id as *mut [u8; 32],
        )
    };
    if take_pwffi_error(env, result) {
        return;
    }
    // Publish the handle + id into the pre-validated caller buffers.
    let one = [wallet_handle as jlong];
    let id_bytes = wallet_id.map(|b| b as jni::sys::jbyte);
    let published = env
        .set_long_array_region(out_wallet_handle, 0, &one)
        .and_then(|_| env.set_byte_array_region(out_wallet_id, 0, &id_bytes));
    if published.is_err() {
        // Unreachable after the up-front bounds validation (region writes
        // on validated arrays don't allocate and can't go out of bounds);
        // kept as a defensive backstop. NOTE the limits: remove_wallet
        // only unregisters the in-memory registration — rows the create
        // committed through the persistence callbacks may survive, and
        // Kotlin (which never received the id) cannot clean them either.
        unsafe {
            let _ = platform_wallet_ffi::platform_wallet_manager_remove_wallet(
                manager_handle as Handle,
                &wallet_id as *const [u8; 32],
            );
            platform_wallet_ffi::platform_wallet_destroy(wallet_handle as Handle);
        }
        let _ = env.exception_clear();
        throw_sdk_exception(
            env,
            1,
            "failed to publish the wallet handle/id into the caller buffers",
        );
    }
}

/// Create a wallet from a BIP39 mnemonic (fresh-wallet default: birth
/// height resolved from SPV's confirmed header tip). The 32-byte wallet id
/// is written into the caller-allocated `out_wallet_id` (`byte[32]`); the
/// created `PlatformWallet` handle is written into `out_wallet_handle[0]`.
/// Both buffers are validated BEFORE the native create so no fallible JNI
/// work follows the persistence commit. `createDefaultAccounts` maps to
/// account options 1 (Default) / 0 (None).
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_WalletManagerNative_createWalletFromMnemonic(
    mut env: JNIEnv,
    _class: JClass,
    manager_handle: jlong,
    mnemonic: JString,
    network: jni::sys::jint,
    create_default_accounts: jboolean,
    out_wallet_handle: jni::objects::JLongArray,
    out_wallet_id: jni::objects::JByteArray,
) {
    guard(&mut env, (), |env| {
        create_wallet_from_mnemonic_impl(
            env,
            manager_handle,
            &mnemonic,
            network,
            create_default_accounts,
            false,
            0,
            &out_wallet_handle,
            &out_wallet_id,
        )
    })
}

/// Create a wallet from a BIP39 mnemonic with an explicit SPV birth-height
/// override. `hasBirthHeightOverride == false` behaves exactly like
/// [`Java_org_dashfoundation_dashsdk_ffi_WalletManagerNative_createWalletFromMnemonic`]
/// (birth height from the confirmed header tip); `true` pins the
/// compact-filter scan start to `birthHeightOverride` — an imported /
/// restored mnemonic passes `0` for a full historical scan so Core funds
/// and payments received before this device registered the wallet are seen.
/// Mirror of Swift `PlatformWalletManager.createWallet(..., birthHeight:)`.
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_WalletManagerNative_createWalletFromMnemonicWithBirthHeight(
    mut env: JNIEnv,
    _class: JClass,
    manager_handle: jlong,
    mnemonic: JString,
    network: jni::sys::jint,
    create_default_accounts: jboolean,
    has_birth_height_override: jboolean,
    birth_height_override: jni::sys::jint,
    out_wallet_handle: jni::objects::JLongArray,
    out_wallet_id: jni::objects::JByteArray,
) {
    guard(&mut env, (), |env| {
        create_wallet_from_mnemonic_impl(
            env,
            manager_handle,
            &mnemonic,
            network,
            create_default_accounts,
            has_birth_height_override == JNI_TRUE,
            birth_height_override as u32,
            &out_wallet_handle,
            &out_wallet_id,
        )
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

// ── Core transaction builder (1:1 over `core_wallet_tx_builder_*`) ─────
//
// The base refactor replaced the one-shot `core_wallet_send_to_addresses`
// with a step-by-step builder (`transaction_builder.rs`) + separate broadcast
// entry points. Per `packages/kotlin-sdk/CLAUDE.md`, each builder step is
// exported as its OWN thin JNI trampoline (one export = one FFI call, no
// composite stitching); the Kotlin `CoreTransactionBuilder` class orchestrates
// the sequence, mirroring the Swift `CoreTransactionBuilder` + the
// `.coreToCore` flow in `SendViewModel.swift`.
//
// The `*mut FFITransactionBuilder` from [coreTxBuilderNew] crosses as a
// `jlong`. It has PRIVATE fields (the FFI crate is an rlib dependency, so
// cbindgen's C-side field visibility does not apply here); we never
// read/construct its fields — it stays an opaque handle, matching the
// discipline the rest of this module uses.

/// `core_wallet_tx_builder_new` — create a builder for `network`
/// (`Network.ffiValue`: 0 Mainnet, 1 Testnet, 2 Devnet, 3 Regtest). Returns
/// the `*mut FFITransactionBuilder` as a `jlong` (0 after throwing). Free
/// with [coreTxBuilderDestroy], or the consuming finalizers
/// [coreTxBuilderFinalize] / [coreWalletFinalizeSignedPayment].
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_WalletManagerNative_coreTxBuilderNew(
    mut env: JNIEnv,
    _class: JClass,
    network: jni::sys::jint,
) -> jlong {
    guard(&mut env, 0, |env| {
        // SAFETY: the returned pointer is owned by the caller (Kotlin), which
        // frees it via destroy or build_signed exactly once.
        let builder =
            unsafe { platform_wallet_ffi::core_wallet_tx_builder_new(ffi_network(network)) };
        if builder.is_null() {
            throw_sdk_exception(env, 1, "core_wallet_tx_builder_new returned NULL");
            return 0;
        }
        builder as jlong
    })
}

/// `core_wallet_tx_builder_add_output` — append a recipient output. The
/// address is network-checked Rust-side against the builder's network.
/// Rejects a non-positive `amount` at the boundary (a negative jlong would
/// otherwise bit-cast to a huge u64).
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_WalletManagerNative_coreTxBuilderAddOutput(
    mut env: JNIEnv,
    _class: JClass,
    builder: jlong,
    address: JString,
    amount: jlong,
) {
    guard(&mut env, (), |env| {
        if builder == 0 {
            throw_sdk_exception(env, 1, "builder handle is 0");
            return;
        }
        if amount <= 0 {
            throw_sdk_exception(env, 1, "amount must be positive");
            return;
        }
        let Some(address_c) = read_cstring_required(env, &address, "address") else {
            return;
        };
        let result = unsafe {
            platform_wallet_ffi::core_wallet_tx_builder_add_output(
                builder as *mut platform_wallet_ffi::FFITransactionBuilder,
                address_c.as_ptr(),
                amount as u64,
            )
        };
        let _ = take_pwffi_error(env, result);
    })
}

/// `core_wallet_tx_builder_add_op_return` — append a zero-value OP_RETURN
/// output carrying `data` (a MAYACHAIN-style deposit memo). The FFI rejects
/// a payload over the 80-byte standardness limit BEFORE consuming the
/// builder's state, so a refused memo leaves outputs/options intact.
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_WalletManagerNative_coreTxBuilderAddOpReturn(
    mut env: JNIEnv,
    _class: JClass,
    builder: jlong,
    data: JByteArray,
) {
    guard(&mut env, (), |env| {
        if builder == 0 {
            throw_sdk_exception(env, 1, "builder handle is 0");
            return;
        }
        let bytes = match env.convert_byte_array(&data) {
            Ok(b) => b,
            Err(_) => {
                throw_sdk_exception(env, 1, "data must be a byte[]");
                return;
            }
        };
        let result = unsafe {
            platform_wallet_ffi::core_wallet_tx_builder_add_op_return(
                builder as *mut platform_wallet_ffi::FFITransactionBuilder,
                bytes.as_ptr(),
                bytes.len(),
            )
        };
        let _ = take_pwffi_error(env, result);
    })
}

/// `core_wallet_tx_builder_set_change_address` — override the change
/// address (network-checked Rust-side). Optional: `set_funding` also sets a
/// change address, so the `.coreToCore` send path does not call this.
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_WalletManagerNative_coreTxBuilderSetChangeAddress(
    mut env: JNIEnv,
    _class: JClass,
    builder: jlong,
    address: JString,
) {
    guard(&mut env, (), |env| {
        if builder == 0 {
            throw_sdk_exception(env, 1, "builder handle is 0");
            return;
        }
        let Some(address_c) = read_cstring_required(env, &address, "address") else {
            return;
        };
        let result = unsafe {
            platform_wallet_ffi::core_wallet_tx_builder_set_change_address(
                builder as *mut platform_wallet_ffi::FFITransactionBuilder,
                address_c.as_ptr(),
            )
        };
        let _ = take_pwffi_error(env, result);
    })
}

/// `core_wallet_tx_builder_preserve_output_order` — keep outputs in
/// insertion order instead of BIP-69 sorting them at build time. Required
/// for MAYACHAIN-style deposits (vault must stay VOUT0, memo VOUT1).
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_WalletManagerNative_coreTxBuilderPreserveOutputOrder(
    mut env: JNIEnv,
    _class: JClass,
    builder: jlong,
) {
    guard(&mut env, (), |env| {
        if builder == 0 {
            throw_sdk_exception(env, 1, "builder handle is 0");
            return;
        }
        let result = unsafe {
            platform_wallet_ffi::core_wallet_tx_builder_preserve_output_order(
                builder as *mut platform_wallet_ffi::FFITransactionBuilder,
            )
        };
        let _ = take_pwffi_error(env, result);
    })
}

/// `core_wallet_tx_builder_change_to_first_input` — route change to the
/// address of the first selected input (VIN0). Required for MAYACHAIN-style
/// deposits: MAYAChain identifies the depositor by VIN0 and pays refunds
/// there. Overrides any change address `set_funding` assigned.
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_WalletManagerNative_coreTxBuilderChangeToFirstInput(
    mut env: JNIEnv,
    _class: JClass,
    builder: jlong,
) {
    guard(&mut env, (), |env| {
        if builder == 0 {
            throw_sdk_exception(env, 1, "builder handle is 0");
            return;
        }
        let result = unsafe {
            platform_wallet_ffi::core_wallet_tx_builder_change_to_first_input(
                builder as *mut platform_wallet_ffi::FFITransactionBuilder,
            )
        };
        let _ = take_pwffi_error(env, result);
    })
}

/// `core_wallet_tx_builder_set_fee_rate` — set the fee rate in duffs/kB.
/// Rejects a non-positive value at the boundary (a negative jlong would
/// otherwise bit-cast to a huge u64).
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_WalletManagerNative_coreTxBuilderSetFeeRate(
    mut env: JNIEnv,
    _class: JClass,
    builder: jlong,
    sat_per_kb: jlong,
) {
    guard(&mut env, (), |env| {
        if builder == 0 {
            throw_sdk_exception(env, 1, "builder handle is 0");
            return;
        }
        if sat_per_kb <= 0 {
            throw_sdk_exception(env, 1, "satPerKb must be positive");
            return;
        }
        let result = unsafe {
            platform_wallet_ffi::core_wallet_tx_builder_set_fee_rate(
                builder as *mut platform_wallet_ffi::FFITransactionBuilder,
                sat_per_kb as u64,
            )
        };
        let _ = take_pwffi_error(env, result);
    })
}

/// `core_wallet_tx_builder_set_selection_strategy` — set the coin-selection
/// strategy by its `CoreSelectionStrategyFFI` discriminant (0 SmallestFirst,
/// 1 LargestFirst, 2 BranchAndBound, 3 OptimalConsolidation, 4 Random,
/// 5 All). Rejects an out-of-range value at the boundary.
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_WalletManagerNative_coreTxBuilderSetSelectionStrategy(
    mut env: JNIEnv,
    _class: JClass,
    builder: jlong,
    strategy: jni::sys::jint,
) {
    guard(&mut env, (), |env| {
        if builder == 0 {
            throw_sdk_exception(env, 1, "builder handle is 0");
            return;
        }
        let Some(strategy) = core_selection_strategy(strategy) else {
            throw_sdk_exception(env, 1, "selectionStrategy out of range (expected 0..=5)");
            return;
        };
        let result = unsafe {
            platform_wallet_ffi::core_wallet_tx_builder_set_selection_strategy(
                builder as *mut platform_wallet_ffi::FFITransactionBuilder,
                strategy,
            )
        };
        let _ = take_pwffi_error(env, result);
    })
}

/// `core_wallet_tx_builder_set_current_height` — set the chain-tip height
/// coin selection treats as the tip (advisory; `set_funding` /
/// `build_signed` override it with the wallet's last processed height).
/// Rejects a negative value at the boundary.
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_WalletManagerNative_coreTxBuilderSetCurrentHeight(
    mut env: JNIEnv,
    _class: JClass,
    builder: jlong,
    height: jni::sys::jint,
) {
    guard(&mut env, (), |env| {
        if builder == 0 {
            throw_sdk_exception(env, 1, "builder handle is 0");
            return;
        }
        if height < 0 {
            throw_sdk_exception(env, 1, "height must be non-negative");
            return;
        }
        let result = unsafe {
            platform_wallet_ffi::core_wallet_tx_builder_set_current_height(
                builder as *mut platform_wallet_ffi::FFITransactionBuilder,
                height as u32,
            )
        };
        let _ = take_pwffi_error(env, result);
    })
}

/// Atomic finalizer: consumes a configured builder, performs funding and
/// ReservationSet insertion indivisibly in platform-wallet, drops the manager
/// lock, then invokes the mnemonic resolver to sign.
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_WalletManagerNative_coreTxBuilderFinalize(
    mut env: JNIEnv,
    _class: JClass,
    builder: jlong,
    wallet_handle: jlong,
    account_type: jint,
    account_index: jint,
    core_signer_handle: jlong,
) -> jlong {
    guard(&mut env, 0, |env| {
        if builder == 0 {
            throw_sdk_exception(env, 1, "builder handle must be non-zero");
            return 0;
        }
        // From this point JNI owns the builder. Any boundary validation error
        // must destroy it because Kotlin has already zeroed its owner token.
        let destroy_builder = || unsafe {
            platform_wallet_ffi::core_wallet_tx_builder_destroy(
                builder as *mut platform_wallet_ffi::FFITransactionBuilder,
            )
        };
        if wallet_handle == 0 || core_signer_handle == 0 {
            destroy_builder();
            throw_sdk_exception(env, 1, "wallet and signer handles must be non-zero");
            return 0;
        }
        let Some(account_type) = core_account_type(account_type) else {
            destroy_builder();
            throw_sdk_exception(env, 1, "accountType out of range (expected 0..=2)");
            return 0;
        };
        if account_index < 0 {
            destroy_builder();
            throw_sdk_exception(env, 1, "accountIndex must be non-negative");
            return 0;
        }
        let mut transaction_handle: Handle = 0;
        let result = unsafe {
            platform_wallet_ffi::core_wallet_tx_builder_finalize(
                builder as *mut platform_wallet_ffi::FFITransactionBuilder,
                wallet_handle as Handle,
                account_type,
                account_index as u32,
                core_signer_handle as *mut rs_sdk_ffi::MnemonicResolverHandle,
                &mut transaction_handle,
            )
        };
        if take_pwffi_error(env, result) {
            return 0;
        }
        transaction_handle as jlong
    })
}

/// `core_wallet_tx_builder_destroy` — free a builder created by
/// [coreTxBuilderNew] that was NOT consumed by [coreTxBuilderFinalize] /
/// [coreWalletFinalizeSignedPayment].
/// Safe on 0.
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_WalletManagerNative_coreTxBuilderDestroy(
    mut env: JNIEnv,
    _class: JClass,
    builder: jlong,
) {
    guard(&mut env, (), |_| {
        if builder == 0 {
            return;
        }
        // SAFETY: `builder` is a live, not-yet-destroyed FFITransactionBuilder
        // pointer from coreTxBuilderNew; destroy handles it exactly once.
        unsafe {
            platform_wallet_ffi::core_wallet_tx_builder_destroy(
                builder as *mut platform_wallet_ffi::FFITransactionBuilder,
            )
        };
    })
}

/// `core_wallet_sign_message` — sign `message` with the private key behind
/// `address` and return the base64 signature: a classic Dash signed message,
/// verifiable by Dash Core's `verifymessage` RPC, dashj's
/// `ECKey.verifyMessage`, and CrowdNode's server-side check.
///
/// `core_handle` is the transient core-wallet `Handle` from
/// [platformWalletGetCore]. `address` must be a P2PKH address of THIS wallet on
/// its network, belonging to a signable funds account — a foreign or watch-only
/// address throws `ErrorSigningKeyUnavailable` (31), while an unparseable,
/// wrong-network, or non-P2PKH address throws `ErrorInvalidParameter` (2).
/// `message` is signed verbatim (it is length-prefixed into the digest, so
/// trailing whitespace is significant). `core_signer_handle` is the manager's
/// `MnemonicResolverHandle`.
///
/// Moves no value: nothing is selected, reserved, broadcast, or persisted.
/// Returns the base64 signature as a `String`, or null after throwing.
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_WalletManagerNative_coreWalletSignMessage(
    mut env: JNIEnv,
    _class: JClass,
    core_handle: jlong,
    address: JString,
    message: JString,
    core_signer_handle: jlong,
) -> jstring {
    guard(&mut env, ptr::null_mut(), |env| {
        if core_handle == 0 {
            throw_sdk_exception(env, 1, "core handle is 0");
            return ptr::null_mut();
        }
        if core_signer_handle == 0 {
            throw_sdk_exception(env, 1, "coreSignerHandle is 0");
            return ptr::null_mut();
        }
        let Some(address) = read_cstring_required(env, &address, "address") else {
            return ptr::null_mut();
        };
        // The message is read leniently on emptiness — unlike `address`, an empty
        // string is a legitimate thing to sign (the digest length-prefixes it),
        // so `read_cstring_required` (which rejects empty) is wrong here. A JNI
        // read error still throws: silently signing the empty message when the
        // caller supplied text would produce a signature that verifies for a
        // message they never sent.
        if message.is_null() {
            throw_sdk_exception(env, 1, "message was null");
            return ptr::null_mut();
        }
        let message: String = match env.get_string(&message) {
            Ok(v) => v.into(),
            Err(_) => {
                let _ = env.exception_clear();
                throw_sdk_exception(env, 1, "message string was invalid");
                return ptr::null_mut();
            }
        };

        // Both cross as UTF-8 bytes + length (no trailing NUL), so an embedded
        // NUL cannot truncate what actually gets signed.
        let address_bytes = address.as_bytes();
        let message_bytes = message.as_bytes();

        let mut out_signature: *mut c_char = ptr::null_mut();
        let result = unsafe {
            platform_wallet_ffi::core_wallet_sign_message(
                core_handle as Handle,
                address_bytes.as_ptr(),
                address_bytes.len(),
                message_bytes.as_ptr(),
                message_bytes.len(),
                core_signer_handle as *mut rs_sdk_ffi::MnemonicResolverHandle,
                &mut out_signature as *mut *mut c_char,
            )
        };
        if take_pwffi_error(env, result) {
            return ptr::null_mut();
        }
        if out_signature.is_null() {
            throw_sdk_exception(env, 1, "sign_message returned a NULL signature");
            return ptr::null_mut();
        }
        let signature = unsafe { CStr::from_ptr(out_signature) }
            .to_string_lossy()
            .into_owned();
        unsafe { platform_wallet_ffi::core_wallet_free_address(out_signature) };
        // A `new_string` failure must throw like every other failure path:
        // Kotlin declares a non-null return, so a bare null here would surface
        // as an unexplained NullPointerException at the platform-type boundary
        // instead of a DashSdkException.
        match env.new_string(signature) {
            Ok(s) => s.into_raw(),
            Err(_) => {
                let _ = env.exception_clear();
                throw_sdk_exception(env, 1, "failed to allocate the signature string");
                ptr::null_mut()
            }
        }
    })
}

/// `platform_wallet_get_core` — resolve the transient core-wallet `Handle`
/// (as `jlong`) from a `PlatformWallet` handle, for [coreWalletBroadcastSignedTransaction].
/// Free with [coreWalletDestroy]. Returns 0 after throwing.
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_WalletManagerNative_platformWalletGetCore(
    mut env: JNIEnv,
    _class: JClass,
    wallet_handle: jlong,
) -> jlong {
    guard(&mut env, 0, |env| {
        let mut core_handle: Handle = 0;
        let result = unsafe {
            platform_wallet_ffi::platform_wallet_get_core(
                wallet_handle as Handle,
                &mut core_handle as *mut Handle,
            )
        };
        if take_pwffi_error(env, result) {
            return 0;
        }
        core_handle as jlong
    })
}

/// `core_wallet_next_receive_address` — the engine's next unused BIP-44
/// EXTERNAL (receive) address for `account_index`, base58-encoded.
///
/// Kotlin parity for the Swift binding (`SwiftDashSDKReceiveAddressReader`
/// → `coreWallet().nextReceiveAddress(accountIndex:)`): the engine answers
/// from its in-memory used-set, so this is authoritative over the Room
/// `core_addresses` mirror and needs no persistence read. Same cold-start
/// caveat as iOS documents: until SPV replay populates the used-set a
/// fresh install answers index 0.
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_WalletManagerNative_coreWalletNextReceiveAddress(
    mut env: JNIEnv,
    _class: JClass,
    core_handle: jlong,
    account_index: jni::sys::jint,
) -> jstring {
    guard(&mut env, ptr::null_mut(), |env| {
        if core_handle == 0 {
            throw_sdk_exception(env, 1, "core wallet handle is 0");
            return ptr::null_mut();
        }
        if account_index < 0 {
            throw_sdk_exception(env, 1, "accountIndex must be non-negative");
            return ptr::null_mut();
        }

        let mut out_address: *mut c_char = ptr::null_mut();
        let result = unsafe {
            platform_wallet_ffi::core_wallet_next_receive_address(
                core_handle as Handle,
                account_index as u32,
                &mut out_address as *mut *mut c_char,
            )
        };
        if take_pwffi_error(env, result) {
            return ptr::null_mut();
        }

        if out_address.is_null() {
            throw_sdk_exception(env, 1, "next receive address returned NULL");
            return ptr::null_mut();
        }
        // Copy the address out, then free the Rust-owned C string with the
        // module's own free (`core_wallet_free_address`).
        let address = unsafe { CStr::from_ptr(out_address) }
            .to_string_lossy()
            .into_owned();
        unsafe { platform_wallet_ffi::core_wallet_free_address(out_address) };
        env.new_string(address)
            .map(|s| s.into_raw())
            .unwrap_or(ptr::null_mut())
    })
}

/// `core_wallet_next_change_address` — the engine's next unused BIP-44
/// INTERNAL (change) address for `account_index`, base58-encoded. The
/// change-side twin of [coreWalletNextReceiveAddress]; same contract.
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_WalletManagerNative_coreWalletNextChangeAddress(
    mut env: JNIEnv,
    _class: JClass,
    core_handle: jlong,
    account_index: jni::sys::jint,
) -> jstring {
    guard(&mut env, ptr::null_mut(), |env| {
        if core_handle == 0 {
            throw_sdk_exception(env, 1, "core wallet handle is 0");
            return ptr::null_mut();
        }
        if account_index < 0 {
            throw_sdk_exception(env, 1, "accountIndex must be non-negative");
            return ptr::null_mut();
        }

        let mut out_address: *mut c_char = ptr::null_mut();
        let result = unsafe {
            platform_wallet_ffi::core_wallet_next_change_address(
                core_handle as Handle,
                account_index as u32,
                &mut out_address as *mut *mut c_char,
            )
        };
        if take_pwffi_error(env, result) {
            return ptr::null_mut();
        }

        if out_address.is_null() {
            throw_sdk_exception(env, 1, "next change address returned NULL");
            return ptr::null_mut();
        }
        let address = unsafe { CStr::from_ptr(out_address) }
            .to_string_lossy()
            .into_owned();
        unsafe { platform_wallet_ffi::core_wallet_free_address(out_address) };
        env.new_string(address)
            .map(|s| s.into_raw())
            .unwrap_or(ptr::null_mut())
    })
}

/// Consume and broadcast an atomically finalized transaction handle.
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_WalletManagerNative_coreWalletBroadcastSignedTransaction(
    mut env: JNIEnv,
    _class: JClass,
    core_handle: jlong,
    transaction_handle: jlong,
) -> jstring {
    guard(&mut env, ptr::null_mut(), |env| {
        if core_handle == 0 || transaction_handle == 0 {
            throw_sdk_exception(env, 1, "core and transaction handles must be non-zero");
            return ptr::null_mut();
        }
        let mut out_txid: *mut c_char = ptr::null_mut();
        let result = unsafe {
            platform_wallet_ffi::core_wallet_broadcast_signed_transaction(
                core_handle as Handle,
                transaction_handle as Handle,
                &mut out_txid,
            )
        };
        if take_pwffi_error(env, result) {
            return ptr::null_mut();
        }
        if out_txid.is_null() {
            throw_sdk_exception(env, 1, "broadcast returned a NULL txid");
            return ptr::null_mut();
        }
        let txid = unsafe { CStr::from_ptr(out_txid) }
            .to_string_lossy()
            .into_owned();
        unsafe { platform_wallet_ffi::core_wallet_free_address(out_txid) };
        env.new_string(txid)
            .map(|s| s.into_raw())
            .unwrap_or(ptr::null_mut())
    })
}

#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_WalletManagerNative_coreWalletAbandonSignedTransaction(
    mut env: JNIEnv,
    _class: JClass,
    core_handle: jlong,
    transaction_handle: jlong,
) {
    guard(&mut env, (), |env| {
        if transaction_handle == 0 {
            return;
        }
        let result = unsafe {
            platform_wallet_ffi::core_wallet_abandon_signed_transaction(
                core_handle as Handle,
                transaction_handle as Handle,
            )
        };
        let _ = take_pwffi_error(env, result);
    })
}

#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_WalletManagerNative_coreSignedTransactionFree(
    mut env: JNIEnv,
    _class: JClass,
    transaction_handle: jlong,
) {
    guard(&mut env, (), |_| {
        if transaction_handle != 0 {
            platform_wallet_ffi::core_wallet_signed_transaction_free(transaction_handle as Handle);
        }
    })
}

#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_WalletManagerNative_coreSignedTransactionFee(
    mut env: JNIEnv,
    _class: JClass,
    transaction_handle: jlong,
) -> jlong {
    guard(&mut env, 0, |env| {
        let mut fee = 0u64;
        let result = unsafe {
            platform_wallet_ffi::core_wallet_signed_transaction_fee(
                transaction_handle as Handle,
                &mut fee,
            )
        };
        if take_pwffi_error(env, result) {
            return 0;
        }
        fee as jlong
    })
}

/// `core_wallet_signed_transaction_bytes` — the consensus-serialized
/// signed transaction bytes of a finalized-transaction handle from
/// [coreTxBuilderFinalize], WITHOUT consuming the ownership token (mirror of
/// Swift's `FinalizedCoreTransaction.serializedData()`). Lets the caller
/// assert the deposit shape (e.g. MAYACHAIN's vault/OP_RETURN/change output
/// order) before deciding to broadcast. The FFI-owned buffer is copied into
/// the returned `byte[]` and freed here.
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_WalletManagerNative_coreSignedTransactionBytes(
    mut env: JNIEnv,
    _class: JClass,
    transaction_handle: jlong,
) -> jni::sys::jbyteArray {
    guard(&mut env, ptr::null_mut(), |env| {
        if transaction_handle == 0 {
            throw_sdk_exception(env, 1, "transaction handle is 0");
            return ptr::null_mut();
        }
        let mut bytes_ptr: *mut u8 = ptr::null_mut();
        let mut bytes_len: usize = 0;
        let result = unsafe {
            platform_wallet_ffi::core_wallet_signed_transaction_bytes(
                transaction_handle as Handle,
                &mut bytes_ptr,
                &mut bytes_len,
            )
        };
        if take_pwffi_error(env, result) {
            return ptr::null_mut();
        }
        if bytes_ptr.is_null() || bytes_len == 0 {
            // A signed transaction is never 0 bytes — same check Swift makes.
            throw_sdk_exception(
                env,
                1,
                "FFI returned success but finalized transaction bytes were empty",
            );
            return ptr::null_mut();
        }
        // Copy into a JVM array, then free the FFI-owned buffer on every path.
        let array = {
            let slice = unsafe { std::slice::from_raw_parts(bytes_ptr, bytes_len) };
            env.byte_array_from_slice(slice)
        };
        unsafe { platform_wallet_ffi::platform_wallet_bytes_free(bytes_ptr, bytes_len) };
        array.map(|a| a.into_raw()).unwrap_or(ptr::null_mut())
    })
}

/// `core_wallet_destroy` — release a transient core-wallet handle from
/// [platformWalletGetCore]. Safe on 0.
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_WalletManagerNative_coreWalletDestroy(
    mut env: JNIEnv,
    _class: JClass,
    core_handle: jlong,
) {
    guard(&mut env, (), |env| {
        if core_handle == 0 {
            return;
        }
        let result = unsafe { platform_wallet_ffi::core_wallet_destroy(core_handle as Handle) };
        let _ = take_pwffi_error(env, result);
    })
}

// ── Deferred build → broadcast/release core-send (BIP70/BIP270) ───────
//
// ADDITIVE surface over the immediate [coreTxBuilderFinalize] +
// [coreWalletBroadcastSignedTransaction] send path:
// [coreWalletFinalizeSignedPayment] atomically funds, reserves, signs, and
// registers a builder in one native call, returning the raw bytes to hand to a
// merchant server; the reservation is then broadcast on ack — or released on
// nack/abandonment. Backed by the process-global registry in `platform_wallet_ffi`
// (`core_wallet_signed_payment_*`). See `SignedPaymentRegistry`.

/// `core_wallet_signed_payment_finalize` — atomically fund, reserve, sign, and
/// register a builder for deferred (BIP70/BIP270) submission in ONE native
/// operation. Selection and reservation commit as a single unit under the
/// wallet-manager lock, so concurrent deferred builds (or a deferred build
/// racing an immediate send) can no longer double-select an input. CONSUMES
/// [builder]. `accountType`/`accountIndex` are the funding account (0 BIP44,
/// 1 BIP32, 2 CoinJoin); [coreSignerHandle] is a `MnemonicResolverHandle`.
///
/// Returns a big-endian BLOB decoded into a `SignedCoreTransaction`:
/// `u64 token, u64 feeDuffs, u32 txidLen, txid utf8, u32 txBytesLen, txBytes`.
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_WalletManagerNative_coreWalletFinalizeSignedPayment(
    mut env: JNIEnv,
    _class: JClass,
    builder: jlong,
    wallet_handle: jlong,
    account_type: jni::sys::jint,
    account_index: jni::sys::jint,
    core_signer_handle: jlong,
) -> jbyteArray {
    guard(&mut env, ptr::null_mut(), |env| {
        if builder == 0 {
            throw_sdk_exception(env, 1, "builder handle must be non-zero");
            return ptr::null_mut();
        }
        // From here JNI owns the builder. Any pre-call boundary validation must
        // destroy it, because Kotlin has already zeroed its owner token.
        let destroy_builder = || unsafe {
            platform_wallet_ffi::core_wallet_tx_builder_destroy(
                builder as *mut platform_wallet_ffi::FFITransactionBuilder,
            )
        };
        if wallet_handle == 0 || core_signer_handle == 0 {
            destroy_builder();
            throw_sdk_exception(env, 1, "wallet and signer handles must be non-zero");
            return ptr::null_mut();
        }
        let Some(account_type) = core_account_type(account_type) else {
            destroy_builder();
            throw_sdk_exception(env, 1, "accountType out of range (expected 0..=2)");
            return ptr::null_mut();
        };
        if account_index < 0 {
            destroy_builder();
            throw_sdk_exception(env, 1, "accountIndex must be non-negative");
            return ptr::null_mut();
        }

        // Own an out `FFICoreTransaction` on the heap; its fields are private to
        // the FFI crate, so allocate it zeroed and let the FFI fill it in place.
        let mut boxed: Box<std::mem::MaybeUninit<platform_wallet_ffi::FFICoreTransaction>> =
            Box::new(std::mem::MaybeUninit::zeroed());
        let out_tx = boxed
            .as_mut_ptr()
            .cast::<platform_wallet_ffi::FFICoreTransaction>();

        let mut token: u64 = 0;
        let mut fee: u64 = 0;
        let mut out_txid: *mut c_char = ptr::null_mut();
        let mut out_bytes_ptr: *const u8 = ptr::null();
        let mut out_bytes_len: usize = 0;
        let result = unsafe {
            platform_wallet_ffi::core_wallet_signed_payment_finalize(
                builder as *mut platform_wallet_ffi::FFITransactionBuilder,
                wallet_handle as Handle,
                account_type,
                account_index as u32,
                core_signer_handle as *mut rs_sdk_ffi::MnemonicResolverHandle,
                &mut token as *mut u64,
                &mut fee as *mut u64,
                &mut out_txid as *mut *mut c_char,
                out_tx,
                &mut out_bytes_ptr as *mut *const u8,
                &mut out_bytes_len as *mut usize,
            )
        };
        if take_pwffi_error(env, result) {
            // The FFI freed the builder on the error path and left the out struct
            // zeroed (null tx_bytes); dropping `boxed` frees only the box.
            return ptr::null_mut();
        }
        if out_txid.is_null() {
            unsafe { platform_wallet_ffi::core_wallet_transaction_free(out_tx) };
            // The registration already committed and holds the funding
            // reservation; release the token so a defensive-branch failure
            // doesn't orphan it to the TTL backstop (same policy as the
            // byte-array failure path below).
            let _ = unsafe { platform_wallet_ffi::core_wallet_signed_payment_release(token) };
            throw_sdk_exception(env, 1, "finalize returned a NULL txid");
            return ptr::null_mut();
        }

        // Copy the txid out, then free the Rust-owned C string.
        let txid = unsafe { CStr::from_ptr(out_txid) }
            .to_string_lossy()
            .into_owned();
        unsafe { platform_wallet_ffi::core_wallet_free_address(out_txid) };

        // Copy the raw tx bytes (they borrow the still-live `out_tx` buffer).
        let tx_bytes: &[u8] = if out_bytes_ptr.is_null() || out_bytes_len == 0 {
            &[]
        } else {
            unsafe { std::slice::from_raw_parts(out_bytes_ptr, out_bytes_len) }
        };

        // Assemble the big-endian BLOB (matches the register decoder).
        let txid_bytes = txid.into_bytes();
        let mut blob = Vec::with_capacity(8 + 8 + 4 + txid_bytes.len() + 4 + tx_bytes.len());
        blob.extend_from_slice(&token.to_be_bytes());
        blob.extend_from_slice(&fee.to_be_bytes());
        blob.extend_from_slice(&(txid_bytes.len() as u32).to_be_bytes());
        blob.extend_from_slice(&txid_bytes);
        blob.extend_from_slice(&(tx_bytes.len() as u32).to_be_bytes());
        blob.extend_from_slice(tx_bytes);
        let out = match env.byte_array_from_slice(&blob) {
            Ok(array) => array.into_raw(),
            Err(_) => {
                // The registration already committed and is holding the funding
                // reservation; release the token so it isn't orphaned to the TTL
                // backstop when Kotlin never receives it.
                let _ = unsafe { platform_wallet_ffi::core_wallet_signed_payment_release(token) };
                ptr::null_mut()
            }
        };

        // Free the tx bytes now that they are copied into the blob; `boxed` frees
        // the outer box on scope exit.
        unsafe { platform_wallet_ffi::core_wallet_transaction_free(out_tx) };
        out
    })
}

/// `core_wallet_signed_payment_broadcast` — broadcast the payment behind
/// `token`, releasing/keeping its reservation per the broadcast outcome and
/// consuming the token. Rather than double-broadcasting, an unusable token
/// throws one of three sibling codes: `ErrorStaleReservationToken` (34, aged
/// out), `ErrorReservationTokenConsumed` (35, unknown / already broadcast /
/// already released), or `ErrorReservationWalletMismatch` (36, different wallet
/// generation). `coreHandle` must resolve to the wallet the token was minted
/// against. Returns the txid as a lowercase hex string.
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_WalletManagerNative_coreWalletBroadcastSignedPayment(
    mut env: JNIEnv,
    _class: JClass,
    core_handle: jlong,
    token: jlong,
) -> jstring {
    guard(&mut env, ptr::null_mut(), |env| {
        let mut out_txid: *mut c_char = ptr::null_mut();
        let result = unsafe {
            platform_wallet_ffi::core_wallet_signed_payment_broadcast(
                core_handle as Handle,
                token as u64,
                &mut out_txid as *mut *mut c_char,
            )
        };
        if take_pwffi_error(env, result) {
            return ptr::null_mut();
        }
        if out_txid.is_null() {
            throw_sdk_exception(env, 1, "broadcast returned a NULL txid");
            return ptr::null_mut();
        }
        let txid = unsafe { CStr::from_ptr(out_txid) }
            .to_string_lossy()
            .into_owned();
        unsafe { platform_wallet_ffi::core_wallet_free_address(out_txid) };
        env.new_string(txid)
            .map(|s| s.into_raw())
            .unwrap_or(ptr::null_mut())
    })
}

/// `core_wallet_signed_payment_release` — release the funding reservation
/// behind `token` and drop it. Idempotent: releasing an unknown / already-
/// consumed token is a silent no-op (never throws the stale-token error).
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_WalletManagerNative_coreWalletReleaseSignedPayment(
    mut env: JNIEnv,
    _class: JClass,
    token: jlong,
) {
    guard(&mut env, (), |env| {
        let result =
            unsafe { platform_wallet_ffi::core_wallet_signed_payment_release(token as u64) };
        let _ = take_pwffi_error(env, result);
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
    // Length-before-allocation guard: rows are variable but each consumes
    // at least one byte, so `count` can never exceed the remaining
    // payload — bounds `with_capacity` against a hostile raw-JNI blob.
    if count > bytes.len() - cursor {
        throw_sdk_exception(
            env,
            1,
            &format!("recipients blob claims {count} rows but body is too short"),
        );
        return None;
    }
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
        let balance = u64::from_be_bytes(balance_bytes.as_slice().try_into().ok()?);
        // The Kotlin encoder writes this field from a signed long; a set
        // sign bit means a negative amount crossed the boundary — reject
        // rather than treat it as a huge unsigned credit value.
        if balance & (1 << 63) != 0 {
            throw_sdk_exception(
                env,
                1,
                &format!("recipients blob row {i} credits amount out of range"),
            );
            return None;
        }
        entries.push(platform_wallet_ffi::FundingAddressEntryFFI {
            address: platform_wallet_ffi::PlatformAddressFFI {
                address_type: type_byte[0],
                hash,
            },
            has_balance,
            balance,
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
    // Length-before-allocation guard: rows are a fixed 29 bytes, so a
    // header claiming more than the remaining payload holds is malformed
    // — prevents a huge `with_capacity` abort from a raw-JNI blob.
    if count
        .checked_mul(29)
        .is_none_or(|need| bytes.len() - cursor < need)
    {
        throw_sdk_exception(
            env,
            1,
            &format!("outputs blob claims {count} rows but body is too short"),
        );
        return None;
    }
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
        let credits = u64::from_be_bytes(credit_bytes.as_slice().try_into().ok()?);
        // The Kotlin encoder writes this field from a signed long; a set
        // sign bit means a negative amount crossed the boundary — reject
        // rather than treat it as a huge unsigned credit value.
        if credits & (1 << 63) != 0 {
            throw_sdk_exception(
                env,
                1,
                &format!("outputs blob row {i} credits amount out of range"),
            );
            return None;
        }
        outputs.push(platform_wallet_ffi::AddressBalanceEntryFFI {
            address: platform_wallet_ffi::PlatformAddressFFI {
                address_type: type_byte[0],
                hash,
            },
            balance: credits,
            nonce: 0,
            account_index: 0,
            address_index: 0,
            // Request path names only outputs/amounts — 0 per the FFI doc
            // (the height pin is a persistence-round-trip concern).
            as_of_height: 0,
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
        // Reject sign errors at the boundary — negatives would otherwise
        // bit-cast to huge unsigned values on the FFI call.
        if amount_duffs <= 0 {
            throw_sdk_exception(env, 1, "amountDuffs must be positive");
            return ptr::null_mut();
        }
        if account_index < 0 {
            throw_sdk_exception(env, 1, "accountIndex must be non-negative");
            return ptr::null_mut();
        }
        if platform_account_index < 0 {
            throw_sdk_exception(env, 1, "platformAccountIndex must be non-negative");
            return ptr::null_mut();
        }

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
                amount_duffs as u64,
                account_index as u32,
                platform_account_index as u32,
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
        // Reject sign errors at the boundary — negatives would otherwise
        // bit-cast to huge u32s on the FFI call.
        if out_point_vout < 0 {
            throw_sdk_exception(env, 1, "outPointVout must be non-negative");
            return ptr::null_mut();
        }
        if platform_account_index < 0 {
            throw_sdk_exception(env, 1, "platformAccountIndex must be non-negative");
            return ptr::null_mut();
        }

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
            vout: out_point_vout as u32,
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
                platform_account_index as u32,
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
        // Reject a negative account index at the boundary — it would
        // otherwise bit-cast to a huge u32 on the FFI call.
        if account_index < 0 {
            throw_sdk_exception(env, 1, "accountIndex must be non-negative");
            return ptr::null_mut();
        }

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
                account_index as u32,
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
        // Reject sign errors at the boundary — negatives would otherwise
        // bit-cast to huge u32s on the FFI call.
        if account_index < 0 {
            throw_sdk_exception(env, 1, "accountIndex must be non-negative");
            return ptr::null_mut();
        }
        if core_fee_per_byte < 0 {
            throw_sdk_exception(env, 1, "coreFeePerByte must be non-negative");
            return ptr::null_mut();
        }

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
                account_index as u32,
                platform_wallet_ffi::InputSelectionType::Auto,
                ptr::null(),
                0,
                ptr::null(),
                0,
                core_address_c.as_ptr(),
                core_fee_per_byte as u32,
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
        // Reject sign errors at the boundary — negatives would otherwise
        // bit-cast to huge u32s on the FFI call.
        if account_index < 0 {
            throw_sdk_exception(env, 1, "accountIndex must be non-negative");
            return ptr::null_mut();
        }
        if core_fee_per_byte < 0 {
            throw_sdk_exception(env, 1, "coreFeePerByte must be non-negative");
            return ptr::null_mut();
        }

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
                account_index as u32,
                core_fee_per_byte as u32,
                &mut preflight as *mut platform_wallet_ffi::WithdrawalPreflightFFI,
            )
        };

        // Every branch funnels into `out` so the transient handle is
        // destroyed unconditionally below — no early returns past this
        // point (they would leak `addr_handle`).
        let out = if take_pwffi_error(env, preflight_result) {
            ptr::null_mut()
        } else {
            let triple = [
                if preflight.can_withdraw { 1 } else { 0 },
                preflight.net_withdrawable as jlong,
                preflight.estimated_fee as jlong,
            ];
            match env.new_long_array(3) {
                Ok(arr) => {
                    if env.set_long_array_region(&arr, 0, &triple).is_err() {
                        ptr::null_mut()
                    } else {
                        arr.into_raw()
                    }
                }
                Err(_) => ptr::null_mut(),
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
        // Every branch funnels into `out` so the transient handle is
        // destroyed unconditionally below — no early returns past this
        // point (they would leak `addr_handle`).
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
                match env.new_long_array(2) {
                    Ok(arr) => {
                        if env.set_long_array_region(&arr, 0, &pair).is_err() {
                            ptr::null_mut()
                        } else {
                            arr.into_raw()
                        }
                    }
                    Err(_) => ptr::null_mut(),
                }
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

/// Whether the manager has frozen a durable sync watermark this manager's
/// lifetime (dashpay/platform#4069). `true` means a persistence `store()`
/// was rejected — the one remaining fault trigger; the lossless persistence
/// channel cannot drop or lag events — so the affected wallet's persisted
/// `syncedHeight` is deliberately held behind the chain tip and a
/// rescan is pending on the next launch — the host should surface a hard
/// "verification failed / rescan pending" state rather than leave the fault
/// in the error logs. Latches for this manager instance's lifetime (a
/// destroyed-and-recreated manager starts unlatched). Backs
/// `PlatformWalletManager.syncFaultDetected()`.
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_WalletManagerNative_syncFaultDetected(
    mut env: JNIEnv,
    _class: JClass,
    manager_handle: jlong,
) -> jboolean {
    guard(&mut env, JNI_FALSE, |env| {
        let mut detected = false;
        let result = unsafe {
            platform_wallet_ffi::platform_wallet_manager_sync_fault_detected(
                manager_handle as Handle,
                &mut detected as *mut bool,
            )
        };
        if take_pwffi_error(env, result) {
            return JNI_FALSE;
        }
        if detected {
            JNI_TRUE
        } else {
            JNI_FALSE
        }
    })
}

#[cfg(feature = "shielded")]
sync_start_stop!(
    Java_org_dashfoundation_dashsdk_ffi_WalletManagerNative_shieldedSyncStart,
    Java_org_dashfoundation_dashsdk_ffi_WalletManagerNative_shieldedSyncStop,
    Java_org_dashfoundation_dashsdk_ffi_WalletManagerNative_shieldedSyncIsRunning,
    platform_wallet_ffi::platform_wallet_manager_shielded_sync_start,
    platform_wallet_ffi::platform_wallet_manager_shielded_sync_stop,
    platform_wallet_ffi::platform_wallet_manager_shielded_sync_is_running
);

/// Whether a shielded sync **pass is in flight right now** — distinct from
/// `shieldedSyncIsRunning` (which reports whether the background *loop* is
/// alive, and stays `true` for the loop's whole lifetime, including while it
/// sleeps between passes). The Kotlin `ShieldedService` polls THIS to drive
/// its `isSyncing` mirror, mirroring Swift's `isShieldedSyncing()` poll —
/// gating the shielded "Clear" button on the loop-alive flag instead would
/// pin it disabled forever. Backs
/// `PlatformWalletManager.isShieldedSyncing()`.
#[cfg(feature = "shielded")]
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_WalletManagerNative_shieldedSyncIsSyncing(
    mut env: JNIEnv,
    _class: JClass,
    manager_handle: jlong,
) -> jboolean {
    guard(&mut env, JNI_FALSE, |env| {
        let mut syncing = false;
        let result = unsafe {
            platform_wallet_ffi::platform_wallet_manager_shielded_sync_is_syncing(
                manager_handle as Handle,
                &mut syncing as *mut bool,
            )
        };
        if take_pwffi_error(env, result) {
            return JNI_FALSE;
        }
        if syncing {
            JNI_TRUE
        } else {
            JNI_FALSE
        }
    })
}

/// Configure the network-scoped shielded coordinator — opens (or creates)
/// the per-network commitment-tree SQLite file at `db_path` that every
/// subsequent `shieldedBind` on this manager reuses. Idempotent at the
/// path level (same path no-ops; a different path throws — the SQLite
/// handle can't be repointed mid-flight). Mirrors Swift's
/// `PlatformWalletManager.configureShielded(dbPath:)`.
#[cfg(feature = "shielded")]
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_WalletManagerNative_shieldedConfigure(
    mut env: JNIEnv,
    _class: JClass,
    manager_handle: jlong,
    db_path: JString,
) {
    guard(&mut env, (), |env| {
        // db_path is required (read_cstring_required rejects null / empty).
        let Some(db_path_c) = read_cstring_required(env, &db_path, "db_path") else {
            return;
        };
        let result = unsafe {
            platform_wallet_ffi::platform_wallet_manager_configure_shielded(
                manager_handle as Handle,
                db_path_c.as_ptr(),
            )
        };
        let _ = take_pwffi_error(env, result);
    })
}

/// Derive Orchard keys for `walletId` via the SDK-level mnemonic resolver
/// and register the resulting ZIP-32 accounts on the network-scoped
/// shielded coordinator. `accounts` is a JVM `int[]` of account indices
/// (1..=64 entries, all non-negative; the FFI enforces the same length
/// bounds). Idempotent: a second call replaces the previous binding for
/// the same wallet. Requires a prior `shieldedConfigure` on this manager
/// (the FFI throws `ErrorWalletOperation` otherwise). Mirrors Swift's
/// `PlatformWalletManager.bindShielded(walletId:resolver:accounts:)`.
#[cfg(feature = "shielded")]
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_WalletManagerNative_shieldedBind(
    mut env: JNIEnv,
    _class: JClass,
    manager_handle: jlong,
    wallet_id: JByteArray,
    resolver_handle: jlong,
    accounts: jni::objects::JIntArray,
) {
    guard(&mut env, (), |env| {
        let Some(id) = read_id32(env, &wallet_id) else {
            return;
        };
        if resolver_handle == 0 {
            throw_sdk_exception(env, 1, "resolver handle is 0");
            return;
        }
        // Read the accounts (a JVM int[] → Vec<u32>); jni 0.21 has no
        // `convert_int_array`, so read the length then the region into an
        // owned buffer.
        if accounts.is_null() {
            throw_sdk_exception(env, 1, "accounts int[] was null");
            return;
        }
        let len = match env.get_array_length(&accounts) {
            Ok(l) => l as usize,
            Err(_) => {
                let _ = env.exception_clear();
                throw_sdk_exception(env, 1, "accounts int[] was invalid");
                return;
            }
        };
        if len == 0 || len > 64 {
            throw_sdk_exception(
                env,
                1,
                &format!("accounts must have 1..=64 entries, got {len}"),
            );
            return;
        }
        let mut buf = vec![0i32; len];
        if env.get_int_array_region(&accounts, 0, &mut buf).is_err() {
            let _ = env.exception_clear();
            throw_sdk_exception(env, 1, "accounts int[] read failed");
            return;
        }
        // Reject negative entries before the sign-losing cast — a negative
        // int would otherwise bit-cast to a bogus huge u32 account index.
        if buf.iter().any(|&i| i < 0) {
            throw_sdk_exception(env, 1, "accounts must be non-negative");
            return;
        }
        let accounts_u32: Vec<u32> = buf.into_iter().map(|i| i as u32).collect();

        let result = unsafe {
            platform_wallet_ffi::platform_wallet_manager_bind_shielded(
                manager_handle as Handle,
                id.as_ptr(),
                resolver_handle as *mut MnemonicResolverHandle,
                accounts_u32.as_ptr(),
                accounts_u32.len(),
            )
        };
        let _ = take_pwffi_error(env, result);
    })
}

/// Set the background shielded sync interval in seconds. Rejects
/// non-positive values at the boundary (a negative jlong would otherwise
/// bit-cast to a huge u64). Mirrors Swift's
/// `PlatformWalletManager.setShieldedSyncInterval(seconds:)`.
#[cfg(feature = "shielded")]
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_WalletManagerNative_shieldedSyncSetInterval(
    mut env: JNIEnv,
    _class: JClass,
    manager_handle: jlong,
    interval_seconds: jlong,
) {
    guard(&mut env, (), |env| {
        if interval_seconds <= 0 {
            throw_sdk_exception(env, 1, "intervalSeconds must be positive");
            return;
        }
        let result = unsafe {
            platform_wallet_ffi::platform_wallet_manager_shielded_sync_set_interval(
                manager_handle as Handle,
                interval_seconds as u64,
            )
        };
        let _ = take_pwffi_error(env, result);
    })
}

/// Reset the Rust-side shielded state on this manager: quiesce the
/// background sync loop, drop every wallet registration on the
/// network-scoped coordinator, empty the shared commitment tree, and
/// reset the caught-up cooldown. The per-network SQLite file stays on
/// disk but its contents are reset so the next `shieldedBind` +
/// sync cold-rebuilds from index 0. Throws `DashSDKException` on a
/// store-reset failure — the host must NOT wipe its own persistence
/// unless this succeeds. Mirrors Swift's
/// `PlatformWalletManager.clearShieldedStorage()`.
#[cfg(feature = "shielded")]
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_WalletManagerNative_shieldedClear(
    mut env: JNIEnv,
    _class: JClass,
    manager_handle: jlong,
) {
    guard(&mut env, (), |env| {
        let result = unsafe {
            platform_wallet_ffi::platform_wallet_manager_shielded_clear(manager_handle as Handle)
        };
        let _ = take_pwffi_error(env, result);
    })
}

/// Run one forced shielded sync pass across all registered wallets — the
/// user-initiated "Sync Now" entry point (`force=true` on the Rust side
/// bypasses the caught-up cooldown). Blocks the calling thread for the
/// pass; Kotlin wraps it in `Dispatchers.IO`. Mirrors Swift's
/// `PlatformWalletManager.syncShieldedNow()`.
#[cfg(feature = "shielded")]
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_WalletManagerNative_shieldedSyncNow(
    mut env: JNIEnv,
    _class: JClass,
    manager_handle: jlong,
) {
    guard(&mut env, (), |env| {
        let result = unsafe {
            platform_wallet_ffi::platform_wallet_manager_shielded_sync_sync_now(
                manager_handle as Handle,
            )
        };
        let _ = take_pwffi_error(env, result);
    })
}

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
        // Reject sign errors at the boundary — negatives would otherwise
        // bit-cast to huge u32s on the FFI call.
        if start_from_height < 0 {
            throw_sdk_exception(env, 1, "startFromHeight must be non-negative");
            return;
        }
        if llmq_devnet_size < 0 {
            throw_sdk_exception(env, 1, "llmqDevnetSize must be non-negative");
            return;
        }
        if llmq_devnet_threshold < 0 {
            throw_sdk_exception(env, 1, "llmqDevnetThreshold must be non-negative");
            return;
        }

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

/// Rewind a loaded wallet's in-memory compact-filter checkpoint. Equal or
/// forward heights are successful no-ops in the shared implementation; the
/// rewind is non-durable and must be reissued after process death.
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_WalletManagerNative_spvRescanFilters(
    mut env: JNIEnv,
    _class: JClass,
    manager_handle: jlong,
    wallet_id: JByteArray,
    from_height: jint,
) {
    guard(&mut env, (), |env| {
        if from_height < 0 {
            throw_sdk_exception(env, 1, "fromHeight must be non-negative");
            return;
        }
        let Some(wallet_id) = read_id32(env, &wallet_id) else {
            return;
        };
        let result = unsafe {
            platform_wallet_ffi::platform_wallet_manager_spv_rescan_filters(
                manager_handle as Handle,
                wallet_id.as_ptr(),
                from_height as u32,
            )
        };
        let _ = take_pwffi_error(env, result);
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

/// Map the Kotlin core account-type int (0 BIP44, 1 BIP32, 2 CoinJoin,
/// 3 AllSpendable) to `CoreAccountTypeFFI`. Returns `None` for an
/// out-of-range / negative value so the caller can throw
/// `ErrorInvalidParameter` rather than bit-casting into an undefined
/// discriminant.
///
/// 3 is the pooled selector (BIP44 + BIP32 + every DashPay receiving account,
/// change to BIP44) and is the default for sends; the single-account APIs
/// (gap limits, per-account UTXO listing) reject it with
/// `ErrorInvalidParameter` since they address exactly one account.
fn core_account_type(value: jni::sys::jint) -> Option<platform_wallet_ffi::CoreAccountTypeFFI> {
    match value {
        0 => Some(platform_wallet_ffi::CoreAccountTypeFFI::BIP44),
        1 => Some(platform_wallet_ffi::CoreAccountTypeFFI::BIP32),
        2 => Some(platform_wallet_ffi::CoreAccountTypeFFI::CoinJoin),
        3 => Some(platform_wallet_ffi::CoreAccountTypeFFI::AllSpendable),
        _ => None,
    }
}

/// Map the Kotlin selection-strategy int (0 SmallestFirst, 1 LargestFirst,
/// 2 BranchAndBound, 3 OptimalConsolidation, 4 Random, 5 All) to
/// `CoreSelectionStrategyFFI`. Returns `None` for an out-of-range value.
fn core_selection_strategy(
    value: jni::sys::jint,
) -> Option<platform_wallet_ffi::CoreSelectionStrategyFFI> {
    match value {
        0 => Some(platform_wallet_ffi::CoreSelectionStrategyFFI::SmallestFirst),
        1 => Some(platform_wallet_ffi::CoreSelectionStrategyFFI::LargestFirst),
        2 => Some(platform_wallet_ffi::CoreSelectionStrategyFFI::BranchAndBound),
        3 => Some(platform_wallet_ffi::CoreSelectionStrategyFFI::OptimalConsolidation),
        4 => Some(platform_wallet_ffi::CoreSelectionStrategyFFI::Random),
        5 => Some(platform_wallet_ffi::CoreSelectionStrategyFFI::All),
        _ => None,
    }
}

/// `platform_wallet_account_utxos` swept across every account — the
/// engine-side UTXO inventory `PlatformWalletManager.reconcileTxoStore`
/// diffs against the Room `txos` mirror (dropped change outputs of
/// CoinJoin-funded sends leave the mirror short; the engine reloads from
/// that mirror on restart, so an un-reconciled hole becomes a fund-loss).
/// Returns a JSON object `{"utxos":[...],"errors":[...]}` — one `utxos`
/// row per output the engine currently holds, tagged with its owning
/// account. Accounts are enumerated with the same `get_account_balances`
/// sweep the DashPay tab uses; keys-only accounts return no UTXOs and
/// contribute nothing. `network` follows `Network.ffiValue` (0 mainnet,
/// 2 devnet, 3 regtest, else testnet) and selects the address encoding;
/// an output whose script has no address form carries an empty `address`
/// for the caller to skip. A per-account read failure lands in `errors`
/// instead of failing the sweep — the reconciler must still see every
/// account that DID read, so one faulted account cannot mask the others'
/// repair.
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_WalletManagerNative_walletManagerAllUtxosJson(
    mut env: JNIEnv,
    _class: JClass,
    manager_handle: jlong,
    wallet_id: JByteArray,
    network: jni::sys::jint,
) -> jni::sys::jstring {
    guard(&mut env, ptr::null_mut(), |env| {
        let Some(wid) = read_id32(env, &wallet_id) else {
            return ptr::null_mut();
        };
        let net = match network {
            0 => dashcore::Network::Mainnet,
            2 => dashcore::Network::Devnet,
            3 => dashcore::Network::Regtest,
            _ => dashcore::Network::Testnet,
        };
        let mut entries: *const platform_wallet_ffi::AccountBalanceEntryFFI = ptr::null();
        let mut count: usize = 0;
        let result = unsafe {
            platform_wallet_ffi::platform_wallet_manager_get_account_balances(
                manager_handle as Handle,
                wid.as_ptr(),
                &mut entries,
                &mut count,
            )
        };
        if take_pwffi_error(env, result) {
            return ptr::null_mut();
        }
        let mut rows: Vec<String> = Vec::new();
        let mut errors: Vec<String> = Vec::new();
        if !entries.is_null() && count > 0 {
            let accounts = unsafe { std::slice::from_raw_parts(entries, count) };
            for acc in accounts {
                let spec = platform_wallet_ffi::AccountSpecFFI {
                    type_tag: acc.type_tag as u8,
                    standard_tag: acc.standard_tag as u8,
                    index: acc.index,
                    registration_index: acc.registration_index,
                    key_class: acc.key_class,
                    user_identity_id: acc.user_identity_id,
                    friend_identity_id: acc.friend_identity_id,
                    account_xpub_bytes: ptr::null(),
                    account_xpub_bytes_len: 0,
                };
                let mut utxos: *const platform_wallet_ffi::AccountUtxoEntryFFI = ptr::null();
                let mut utxo_count: usize = 0;
                let res = unsafe {
                    platform_wallet_ffi::platform_wallet_account_utxos(
                        manager_handle as Handle,
                        wid.as_ptr(),
                        &spec,
                        &mut utxos,
                        &mut utxo_count,
                    )
                };
                if let Some(msg) = pwffi_error_message(res) {
                    errors.push(format!(
                        "{{\"typeTag\":{},\"index\":{},\"message\":{}}}",
                        acc.type_tag as u8,
                        acc.index,
                        json_escape(&msg),
                    ));
                    continue;
                }
                if utxos.is_null() || utxo_count == 0 {
                    continue;
                }
                let items = unsafe { std::slice::from_raw_parts(utxos, utxo_count) };
                for u in items {
                    let script: &[u8] = if u.script_pubkey.is_null() || u.script_pubkey_len == 0 {
                        &[]
                    } else {
                        unsafe {
                            std::slice::from_raw_parts(u.script_pubkey, u.script_pubkey_len)
                        }
                    };
                    let script_buf = dashcore::ScriptBuf::from(script.to_vec());
                    let address = dashcore::Address::from_script(&script_buf, net)
                        .map(|a| a.to_string())
                        .unwrap_or_default();
                    rows.push(format!(
                        "{{\"typeTag\":{},\"standardTag\":{},\"index\":{},\
                         \"txid\":\"{}\",\"vout\":{},\"amount\":{},\
                         \"address\":{},\"scriptHex\":\"{}\",\
                         \"height\":{},\"isLocked\":{}}}",
                        acc.type_tag as u8,
                        acc.standard_tag as u8,
                        acc.index,
                        hex_lower(&u.outpoint_txid),
                        u.outpoint_vout,
                        u.value_duffs,
                        json_escape(&address),
                        hex_lower(script),
                        u.height,
                        u.is_locked,
                    ));
                }
                unsafe {
                    platform_wallet_ffi::platform_wallet_account_utxos_free(
                        utxos as *mut platform_wallet_ffi::AccountUtxoEntryFFI,
                        utxo_count,
                    )
                };
            }
        }
        unsafe {
            platform_wallet_ffi::platform_wallet_manager_free_account_balances(
                entries as *mut platform_wallet_ffi::AccountBalanceEntryFFI,
                count,
            )
        };
        let json = format!(
            "{{\"utxos\":[{}],\"errors\":[{}]}}",
            rows.join(","),
            errors.join(","),
        );
        env.new_string(json)
            .map(|s| s.into_raw())
            .unwrap_or(ptr::null_mut())
    })
}

/// Extract-and-free a `PlatformWalletFFIResult`'s error message WITHOUT
/// throwing — the per-account soft-fail path of
/// [`Java_org_dashfoundation_dashsdk_ffi_WalletManagerNative_walletManagerAllUtxosJson`]
/// reports account faults in-band so the sweep keeps going. `None` on
/// success.
fn pwffi_error_message(
    mut result: platform_wallet_ffi::PlatformWalletFFIResult,
) -> Option<String> {
    if result.code == platform_wallet_ffi::PlatformWalletFFIResultCode::Success {
        return None;
    }
    let message = if result.message.is_null() {
        format!("platform-wallet error (code {})", result.code as i32)
    } else {
        // SAFETY: non-null message is a valid CString produced by the FFI.
        unsafe { std::ffi::CStr::from_ptr(result.message) }
            .to_string_lossy()
            .into_owned()
    };
    // SAFETY: `result` is a fresh PlatformWalletFFIResult; free its message.
    unsafe { platform_wallet_ffi::platform_wallet_ffi_result_free(&mut result) };
    Some(message)
}

/// Lower-hex of a byte slice (txid bytes are emitted in the same order
/// the changeset path hands Kotlin, so hex→bytes on the Kotlin side
/// reproduces the exact `txos.txid` blob).
fn hex_lower(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        s.push_str(&format!("{:02x}", b));
    }
    s
}

/// Minimal JSON string escape (quotes, backslash, control chars) — the
/// values here are base58/bech32 addresses and FFI error strings.
fn json_escape(value: &str) -> String {
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

/// Read a required 20-byte `byte[]` (e.g. a voting-key hash160) into `[u8; 20]`;
/// throws + returns None on a null/invalid array or a wrong length.
fn read_id20(env: &mut JNIEnv, arr: &JByteArray) -> Option<[u8; 20]> {
    let bytes = match env.convert_byte_array(arr) {
        Ok(b) => b,
        Err(_) => {
            let _ = env.exception_clear();
            throw_sdk_exception(env, 1, "votingKeyId byte[] was null/invalid");
            return None;
        }
    };
    if bytes.len() != 20 {
        throw_sdk_exception(
            env,
            1,
            &format!("votingKeyId must be 20 bytes, got {}", bytes.len()),
        );
        return None;
    }
    let mut id = [0u8; 20];
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

// ── Wallet-memory snapshots (Wave-1B) ─────────────────────────────────
//
// Read-only in-memory-state accessors backing the Kotlin
// `WalletMemoryExplorerView` port (additive methods on
// `ManagedPlatformWallet`). Wallet-handle-scoped, mirroring the existing
// per-wallet accessors above.

/// Owns the list allocation returned by
/// `platform_wallet_tracked_asset_locks_list`. Keeping this guard alive while
/// JVM objects are built makes every early-return/allocation-failure path call
/// the paired native free function.
struct TrackedAssetLockListGuard {
    entries: *const platform_wallet_ffi::core_wallet_types::TrackedAssetLockEntryFFI,
    count: usize,
}

impl Drop for TrackedAssetLockListGuard {
    fn drop(&mut self) {
        unsafe {
            platform_wallet_ffi::platform_wallet_tracked_asset_locks_free(
                self.entries as *mut _,
                self.count,
            )
        }
    }
}

/// Copy an `IdentifierArray` into a flat JVM `byte[]` (concatenated 32-byte
/// ids) and free the Rust buffer. The Kotlin side splits into 32-byte rows.
fn identifier_array_to_flat(env: &mut JNIEnv, mut arr: IdentifierArray) -> jbyteArray {
    let mut flat = Vec::with_capacity(arr.count * 32);
    if !arr.items.is_null() && arr.count > 0 {
        let rows = unsafe { std::slice::from_raw_parts(arr.items, arr.count) };
        for id in rows {
            flat.extend_from_slice(id);
        }
    }
    unsafe {
        platform_wallet_ffi::platform_wallet_identifier_array_free(&mut arr as *mut IdentifierArray)
    };
    env.byte_array_from_slice(&flat)
        .map(|a| a.into_raw())
        .unwrap_or(ptr::null_mut())
}

/// In-memory summary of a wallet's Rust-side state as a `long[4]` =
/// `[identitiesCount, watchedCount, lastScannedIndex, trackedAssetLocksCount]`.
/// Bridges `platform_wallet_get_in_memory_summary` (Swift
/// `wallet.inMemorySummary()` behind `WalletMemoryExplorerView`).
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_WalletManagerNative_walletInMemorySummary(
    mut env: JNIEnv,
    _class: JClass,
    wallet_handle: jlong,
) -> jlongArray {
    guard(&mut env, ptr::null_mut(), |env| {
        let mut summary = platform_wallet_ffi::PlatformWalletMemorySummaryFFI {
            identities_count: 0,
            watched_count: 0,
            last_scanned_index: 0,
            tracked_asset_locks_count: 0,
        };
        let result = unsafe {
            platform_wallet_ffi::platform_wallet_get_in_memory_summary(
                wallet_handle as Handle,
                &mut summary as *mut platform_wallet_ffi::PlatformWalletMemorySummaryFFI,
            )
        };
        if take_pwffi_error(env, result) {
            return ptr::null_mut();
        }
        let quad = [
            summary.identities_count as jlong,
            summary.watched_count as jlong,
            summary.last_scanned_index as jlong,
            summary.tracked_asset_locks_count as jlong,
        ];
        let Ok(arr) = env.new_long_array(4) else {
            return ptr::null_mut();
        };
        if env.set_long_array_region(&arr, 0, &quad).is_err() {
            return ptr::null_mut();
        }
        arr.into_raw()
    })
}

/// Copy the manager-owned tracked-asset-lock snapshot into Kotlin value
/// objects. No Rust pointer escapes this call; the paired list allocation is
/// freed by [`TrackedAssetLockListGuard`] on success and every failure path.
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_WalletManagerNative_trackedAssetLocks(
    mut env: JNIEnv,
    _class: JClass,
    manager_handle: jlong,
    wallet_id: JByteArray,
) -> jobject {
    guard(&mut env, ptr::null_mut(), |env| {
        let Some(wallet_id) = read_id32(env, &wallet_id) else {
            return ptr::null_mut();
        };
        let mut entries = ptr::null();
        let mut count = 0usize;
        let result = unsafe {
            platform_wallet_ffi::platform_wallet_tracked_asset_locks_list(
                manager_handle as Handle,
                wallet_id.as_ptr(),
                &mut entries,
                &mut count,
            )
        };
        let _list_guard = TrackedAssetLockListGuard { entries, count };
        if take_pwffi_error(env, result) {
            return ptr::null_mut();
        }
        if count > 0 && entries.is_null() {
            throw_sdk_exception(
                env,
                99,
                "tracked asset-lock native list returned a null pointer with a non-zero count",
            );
            return ptr::null_mut();
        }
        if count > i32::MAX as usize {
            throw_sdk_exception(env, 99, "tracked asset-lock snapshot is too large");
            return ptr::null_mut();
        }

        let row_class =
            match env.find_class("org/dashfoundation/dashsdk/ffi/TrackedAssetLockNativeData") {
                Ok(class) => class,
                Err(_) => {
                    let _ = env.exception_clear();
                    throw_sdk_exception(env, 99, "tracked asset-lock row class was not found");
                    return ptr::null_mut();
                }
            };
        let rows = match env.new_object_array(count as i32, &row_class, JObject::null()) {
            Ok(rows) => rows,
            Err(_) => {
                let _ = env.exception_clear();
                throw_sdk_exception(env, 99, "tracked asset-lock array allocation failed");
                return ptr::null_mut();
            }
        };

        let native_rows = if count == 0 {
            &[][..]
        } else {
            unsafe { std::slice::from_raw_parts(entries, count) }
        };
        for (index, row) in native_rows.iter().enumerate() {
            let built = env.with_local_frame(8, |env| -> Result<(), jni::errors::Error> {
                let txid = env.byte_array_from_slice(&row.outpoint_txid)?;
                let object = env.new_object(
                    &row_class,
                    "([BIIBIZI)V",
                    &[
                        JValue::Object(txid.as_ref()),
                        JValue::Int(row.outpoint_vout as jint),
                        JValue::Int(row.lock_type as jint),
                        JValue::Byte(row.status as i8),
                        JValue::Int(row.registration_index as jint),
                        JValue::Bool(row.instant_lock_present as u8),
                        JValue::Int(row.chain_lock_height as jint),
                    ],
                )?;
                env.set_object_array_element(&rows, index as i32, object)
            });
            if built.is_err() {
                let _ = env.exception_clear();
                throw_sdk_exception(env, 99, "tracked asset-lock row allocation failed");
                return ptr::null_mut();
            }
        }

        match env.new_object(
            "org/dashfoundation/dashsdk/ffi/TrackedAssetLocksNativeResult",
            "([Lorg/dashfoundation/dashsdk/ffi/TrackedAssetLockNativeData;)V",
            &[JValue::Object(rows.as_ref())],
        ) {
            Ok(result) => result.into_raw(),
            Err(_) => {
                let _ = env.exception_clear();
                throw_sdk_exception(env, 99, "tracked asset-lock result allocation failed");
                ptr::null_mut()
            }
        }
    })
}

/// The ids of every identity the wallet currently manages, as a flat
/// `byte[]` (concatenated 32-byte ids). Bridges
/// `platform_wallet_list_in_memory_identity_ids` (Swift
/// `wallet.inMemoryIdentityIds()`).
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_WalletManagerNative_walletInMemoryIdentityIds(
    mut env: JNIEnv,
    _class: JClass,
    wallet_handle: jlong,
) -> jbyteArray {
    guard(&mut env, ptr::null_mut(), |env| {
        let mut out = IdentifierArray {
            items: ptr::null_mut(),
            count: 0,
        };
        let result = unsafe {
            platform_wallet_ffi::platform_wallet_list_in_memory_identity_ids(
                wallet_handle as Handle,
                &mut out as *mut IdentifierArray,
            )
        };
        if take_pwffi_error(env, result) {
            return ptr::null_mut();
        }
        identifier_array_to_flat(env, out)
    })
}

/// The ids of every out-of-wallet / observed identity, as a flat `byte[]`.
/// Bridges `platform_wallet_list_in_memory_watched_identity_ids` (Swift
/// `wallet.inMemoryWatchedIdentityIds()`).
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_WalletManagerNative_walletInMemoryWatchedIdentityIds(
    mut env: JNIEnv,
    _class: JClass,
    wallet_handle: jlong,
) -> jbyteArray {
    guard(&mut env, ptr::null_mut(), |env| {
        let mut out = IdentifierArray {
            items: ptr::null_mut(),
            count: 0,
        };
        let result = unsafe {
            platform_wallet_ffi::platform_wallet_list_in_memory_watched_identity_ids(
                wallet_handle as Handle,
                &mut out as *mut IdentifierArray,
            )
        };
        if take_pwffi_error(env, result) {
            return ptr::null_mut();
        }
        identifier_array_to_flat(env, out)
    })
}

/// The proTxHashes of every masternode whose voting key hash matches the
/// 20-byte `votingKeyId`, as a flat `byte[]` (concatenated 32-byte
/// proTxHashes; Kotlin splits into 32-byte rows). Replaces dashj's
/// `MasternodeListManager.getMasternodesByVotingKey(votingKeyId)` used by
/// contested-username voting. Returns an empty `byte[]` when the masternode
/// list hasn't synced (SPV client not running / DML unavailable) or no
/// masternode uses the key. Bridges
/// `platform_wallet_manager_masternodes_by_voting_key`.
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_WalletManagerNative_masternodesByVotingKey(
    mut env: JNIEnv,
    _class: JClass,
    manager_handle: jlong,
    voting_key_id: JByteArray,
) -> jbyteArray {
    guard(&mut env, ptr::null_mut(), |env| {
        let Some(key) = read_id20(env, &voting_key_id) else {
            return ptr::null_mut();
        };
        let mut out = IdentifierArray {
            items: ptr::null_mut(),
            count: 0,
        };
        let result = unsafe {
            platform_wallet_ffi::platform_wallet_manager_masternodes_by_voting_key(
                manager_handle as Handle,
                key.as_ptr(),
                &mut out as *mut IdentifierArray,
            )
        };
        if take_pwffi_error(env, result) {
            return ptr::null_mut();
        }
        identifier_array_to_flat(env, out)
    })
}

/// The BIP-9 identity index recorded on a managed-identity snapshot handle,
/// or `-1` when the identity is out-of-wallet (no index). Bridges
/// `managed_identity_get_identity_index` (Swift `mi.getIdentityIndex()`).
/// `identityHandle` comes from `TokensNative.getManagedIdentity`.
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_WalletManagerNative_managedIdentityGetIdentityIndex(
    mut env: JNIEnv,
    _class: JClass,
    identity_handle: jlong,
) -> jlong {
    guard(&mut env, -1, |env| {
        let mut has_index = false;
        let mut index: u32 = 0;
        let result = unsafe {
            platform_wallet_ffi::managed_identity_get_identity_index(
                identity_handle as Handle,
                &mut has_index as *mut bool,
                &mut index as *mut u32,
            )
        };
        if take_pwffi_error(env, result) {
            return -1;
        }
        if has_index {
            index as jlong
        } else {
            -1
        }
    })
}

/// The lifecycle status of a managed-identity snapshot handle as its
/// `IdentityStatusFFI` discriminant (0 Unknown, 1 PendingCreation, 2 Active,
/// 3 FailedCreation, 4 NotFound). Bridges `managed_identity_get_status`
/// (Swift `mi.getStatus()`). Returns `-1` after throwing on error.
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_WalletManagerNative_managedIdentityGetStatus(
    mut env: JNIEnv,
    _class: JClass,
    identity_handle: jlong,
) -> jint {
    guard(&mut env, -1, |env| {
        let mut status: u8 = 0;
        let result = unsafe {
            platform_wallet_ffi::managed_identity_get_status(
                identity_handle as Handle,
                &mut status as *mut u8,
            )
        };
        if take_pwffi_error(env, result) {
            return -1;
        }
        status as jint
    })
}

// ── DAPI address ban list (manager-scoped, Wave-1B) ───────────────────

/// Snapshot of every DAPI address' ban state as a JSON array string, or
/// null when the list is empty / after throwing. Bridges
/// `platform_wallet_manager_address_ban_info` (Swift `BannedAddressesView`).
/// Manager-scoped (takes the manager handle).
///
/// Each element:
/// `{"address": "<uri>", "banned": <bool>, "banCount": <u32>,
/// "bannedUntilMs": <i64>, "reason": "<string|null>"}`.
///
/// The Rust rows (incl. heap-owned `address` / `reason` C strings) are
/// freed via `platform_wallet_manager_address_ban_info_free` before this
/// returns.
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_WalletManagerNative_managerAddressBanInfo(
    mut env: JNIEnv,
    _class: JClass,
    manager_handle: jlong,
) -> jstring {
    guard(&mut env, ptr::null_mut(), |env| {
        let mut entries: *const platform_wallet_ffi::AddressBanInfoFFI = ptr::null();
        let mut count: usize = 0;
        let result = unsafe {
            platform_wallet_ffi::platform_wallet_manager_address_ban_info(
                manager_handle as Handle,
                &mut entries as *mut *const platform_wallet_ffi::AddressBanInfoFFI,
                &mut count as *mut usize,
            )
        };
        if take_pwffi_error(env, result) {
            return ptr::null_mut();
        }

        // Build a JSON array from the rows before freeing them.
        let json = if entries.is_null() || count == 0 {
            String::from("[]")
        } else {
            let rows = unsafe { std::slice::from_raw_parts(entries, count) };
            let mut items: Vec<String> = Vec::with_capacity(count);
            for row in rows {
                let address = cstr_opt(row.address);
                let reason = cstr_opt(row.reason);
                items.push(format!(
                    "{{\"address\":{},\"banned\":{},\"banCount\":{},\"bannedUntilMs\":{},\"reason\":{}}}",
                    json_string_or_null(address.as_deref()),
                    row.banned,
                    row.ban_count,
                    row.banned_until_ms,
                    json_string_or_null(reason.as_deref()),
                ));
            }
            format!("[{}]", items.join(","))
        };

        // Free the Rust-owned rows (walks each row's C strings first).
        if !entries.is_null() && count > 0 {
            unsafe {
                platform_wallet_ffi::platform_wallet_manager_address_ban_info_free(
                    entries as *mut platform_wallet_ffi::AddressBanInfoFFI,
                    count,
                )
            };
        }

        env.new_string(json)
            .map(|s| s.into_raw())
            .unwrap_or(ptr::null_mut())
    })
}

/// Read a nullable Rust C string into an owned `String`, or `None` when the
/// pointer is null. Does NOT free — the paired `*_free` reclaims the buffer.
fn cstr_opt(ptr: *const c_char) -> Option<String> {
    if ptr.is_null() {
        None
    } else {
        Some(
            unsafe { CStr::from_ptr(ptr) }
                .to_string_lossy()
                .into_owned(),
        )
    }
}

/// Encode an optional string as a JSON string literal (with the minimal
/// escaping the ban fields need — quote and backslash), or the bare `null`
/// token when absent.
fn json_string_or_null(s: Option<&str>) -> String {
    match s {
        None => String::from("null"),
        Some(value) => {
            let escaped = value.replace('\\', "\\\\").replace('"', "\\\"");
            format!("\"{escaped}\"")
        }
    }
}

// ── Withdrawal preflight reason (micro-gap, Wave-1B) ──────────────────

/// Preflight an AUTO withdrawal and return the advisory reason string when
/// the account can't fund one — the `success_with_message` reason the
/// existing [`Java_..._walletPlatformAddressPreflightWithdrawal`] discards.
/// Returns the reason string when `can_withdraw == false` and a message was
/// recorded; null when the withdrawal CAN proceed (no reason to show) or no
/// message was set. Throws only on a structural FFI error.
///
/// This is the second half of the micro-gap split: the existing
/// triple-returning entry point stays the authoritative `canWithdraw` gate
/// (source-compatible — its ABI is unchanged), and this sibling surfaces the
/// human-readable "why not" without touching it.
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_WalletManagerNative_walletPlatformAddressPreflightWithdrawalReason(
    mut env: JNIEnv,
    _class: JClass,
    wallet_handle: jlong,
    account_index: jint,
    core_fee_per_byte: jint,
) -> jstring {
    guard(&mut env, ptr::null_mut(), |env| {
        // Reject sign errors at the boundary — negatives would otherwise
        // bit-cast to huge u32s on the FFI call.
        if account_index < 0 {
            throw_sdk_exception(env, 1, "accountIndex must be non-negative");
            return ptr::null_mut();
        }
        if core_fee_per_byte < 0 {
            throw_sdk_exception(env, 1, "coreFeePerByte must be non-negative");
            return ptr::null_mut();
        }

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
        let mut preflight_result = unsafe {
            platform_wallet_ffi::platform_address_wallet_preflight_withdrawal(
                addr_handle,
                account_index as u32,
                core_fee_per_byte as u32,
                &mut preflight as *mut platform_wallet_ffi::WithdrawalPreflightFFI,
            )
        };

        // A structural failure is a non-Success code -> throw via the shared
        // mapping (which also frees the message). A Success code with a
        // non-null message on the `can_withdraw == false` path is the
        // advisory reason we want to surface.
        let out = if preflight_result.code != PlatformWalletFFIResultCode::Success {
            throw_pwffi(env, &mut preflight_result);
            ptr::null_mut()
        } else {
            let reason = cstr_opt(preflight_result.message);
            // The message is only meaningful when the withdrawal is blocked;
            // a fundable preflight has no "reason".
            let out = if !preflight.can_withdraw {
                match reason {
                    Some(msg) if !msg.is_empty() => env
                        .new_string(msg)
                        .map(|s| s.into_raw())
                        .unwrap_or(ptr::null_mut()),
                    _ => ptr::null_mut(),
                }
            } else {
                ptr::null_mut()
            };
            // Free the Success message buffer regardless (take_pwffi_error
            // only frees on the error path).
            unsafe { platform_wallet_ffi_result_free(&mut preflight_result) };
            out
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

/// Throw a `DashSDKException` for a non-Success `PlatformWalletFFIResult`
/// and free its message. Mirrors the throw half of
/// [`crate::support::take_pwffi_error`] for callers that need to inspect a
/// Success-coded message first (so they can't hand the result to
/// `take_pwffi_error`, which treats Success as "no error, nothing to free").
fn throw_pwffi(env: &mut JNIEnv, result: &mut PlatformWalletFFIResult) {
    let message = if result.message.is_null() {
        format!("platform-wallet error (code {})", result.code as i32)
    } else {
        unsafe { CStr::from_ptr(result.message) }
            .to_string_lossy()
            .into_owned()
    };
    throw_sdk_exception(env, result.code as i32 + PWFFI_CODE_OFFSET, &message);
    unsafe { platform_wallet_ffi_result_free(result) };
}
