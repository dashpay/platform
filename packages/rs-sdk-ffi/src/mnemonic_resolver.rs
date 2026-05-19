//! Rust → Swift "fetch BIP-39 mnemonic for wallet_id" FFI vtable.
//!
//! The architectural intent is the `swift-sdk/CLAUDE.md` "no
//! mnemonic round-tripping" rule: derivation pipelines must live on
//! the Rust side, with Swift exposing only the two operations Rust
//! cannot perform from its side — reading the mnemonic from the iOS
//! Keychain and writing the derived key bytes back. This module
//! exposes the read half (mnemonic resolver). The write half
//! (identity-key persister) lives in `platform-wallet-ffi` because
//! the Keychain-write semantics are wallet-domain.
//!
//! # Synchronous on purpose
//!
//! Unlike the [`SignerVTable`](crate::signer::VTableSigner)
//! infrastructure (which uses a `oneshot` channel + `CompletionSlot`
//! because biometric prompts can take seconds), Keychain reads are
//! sub-millisecond, so this callback is fully synchronous — no
//! completion-callback ceremony, no `tokio::time::timeout`. The
//! Rust side blocks the calling thread for the duration of one
//! Keychain hop. Callers that already invoke the consuming FFI on
//! a background queue (the iOS pattern) get exactly what they want.
//!
//! # Lifetime / cleanup
//!
//! The Swift side calls [`dash_sdk_mnemonic_resolver_create`], which
//! returns an opaque `*mut MnemonicResolverHandle`. Pass it to any
//! FFI entry point that needs to derive from a wallet-stored
//! mnemonic (e.g. `dash_sdk_derive_and_persist_identity_keys`,
//! `asset_lock_manager_create_funded_proof`). Then call
//! [`dash_sdk_mnemonic_resolver_destroy`] — that fires the supplied
//! destructor callback (typically the
//! `Unmanaged.fromOpaque(...).release()` pattern Swift uses for its
//! `MnemonicResolverAndPersister` ctx) so the Swift object can drop.

use std::ffi::c_void;
use std::os::raw::c_char;

/// Maximum mnemonic length, in bytes (excluding the trailing NUL),
/// the resolver buffer can hold.
///
/// 1024 bytes covers every BIP-39 wordlist with margin to spare:
/// the longest 24-word mnemonics in any supported language come in
/// well under that — Korean (the longest of the supported lists)
/// tops out near 700 bytes for a 24-word phrase. Picking a power-
/// of-two cap keeps the inline Rust stack buffer cheap to
/// zero-on-drop.
pub const MNEMONIC_RESOLVER_BUFFER_CAPACITY: usize = 1024;

/// Resolver result codes returned by [`MnemonicResolveCallback`].
///
/// Mirrors the success/failure shape of `PlatformWalletFFIResult`
/// at a finer-grained level so the Rust side can distinguish
/// "Swift hit the buffer cap" from "Swift had no mnemonic stored
/// for this wallet".
pub mod mnemonic_resolver_result {
    /// Mnemonic copied into the buffer; `out_len` was set.
    pub const SUCCESS: i32 = 0;
    /// The Swift side has no mnemonic stored for this `wallet_id`.
    /// Surfaced to the Rust caller as
    /// `PlatformWalletFFIResult::ErrorWalletOperation` with a
    /// "mnemonic missing" detail.
    pub const NOT_FOUND: i32 = 1;
    /// Mnemonic exceeded [`super::MNEMONIC_RESOLVER_BUFFER_CAPACITY`].
    /// Should not happen in practice — the buffer is sized for
    /// every BIP-39 wordlist's 24-word phrase plus margin.
    pub const BUFFER_TOO_SMALL: i32 = 2;
    /// Anything else (Keychain access denied, decode error, etc.).
    /// Surfaced as `ErrorWalletOperation` with a generic detail
    /// the Swift side hopefully logged before returning.
    pub const OTHER: i32 = 3;
}

/// Function pointer type for the mnemonic-resolve callback.
///
/// # Wire shape
///
/// - `ctx`: opaque Swift-side context (typically the result of
///   `Unmanaged.passRetained(swiftSelf).toOpaque()`). Passed back
///   verbatim by Rust on every invocation.
/// - `wallet_id_bytes`: pointer to a 32-byte wallet id. Valid for
///   the duration of the call only.
/// - `out_mnemonic_utf8`: writable buffer for the NUL-terminated
///   UTF-8 mnemonic. Capacity is
///   [`MNEMONIC_RESOLVER_BUFFER_CAPACITY`] bytes (room for the
///   trailing NUL).
/// - `out_capacity`: equal to [`MNEMONIC_RESOLVER_BUFFER_CAPACITY`].
///   Surfaced explicitly so the implementation can sanity-check
///   without assuming the constant.
/// - `out_len`: receives the byte count written to
///   `out_mnemonic_utf8`, EXCLUDING the trailing NUL. Must be set
///   on success.
///
/// # Safety
///
/// - `out_mnemonic_utf8` must be valid for `out_capacity` writable
///   bytes for the duration of this call.
/// - `wallet_id_bytes` must be valid for 32 readable bytes.
/// - `out_len` must be valid for one `usize` write.
/// - The implementation MUST return one of the
///   [`mnemonic_resolver_result`] codes; any other value is treated
///   as `OTHER`.
pub type MnemonicResolveCallback = unsafe extern "C" fn(
    ctx: *const c_void,
    wallet_id_bytes: *const u8,
    out_mnemonic_utf8: *mut c_char,
    out_capacity: usize,
    out_len: *mut usize,
) -> i32;

/// C-compatible vtable for a mnemonic resolver.
#[repr(C)]
pub struct MnemonicResolverVTable {
    /// Synchronous "fetch mnemonic for `wallet_id`".
    pub resolve: MnemonicResolveCallback,
    /// Destructor for the `ctx` pointer. Invoked exactly once when
    /// the matching `dash_sdk_mnemonic_resolver_destroy` is called.
    pub destroy: unsafe extern "C" fn(ctx: *mut c_void),
}

/// Opaque Rust-side handle to a Swift-owned mnemonic resolver.
///
/// Constructed by [`dash_sdk_mnemonic_resolver_create`], destroyed
/// by [`dash_sdk_mnemonic_resolver_destroy`]. Pass the pointer
/// into any FFI entry point that derives from a wallet-stored
/// mnemonic.
#[repr(C)]
pub struct MnemonicResolverHandle {
    pub ctx: *mut c_void,
    pub vtable: *mut MnemonicResolverVTable,
}

// SAFETY: Swift side promises both pointers are thread-stable for
// the lifetime of this handle. The handle is owned by Swift
// across the FFI; Rust just dereferences it under the same lock
// shape the existing signer FFI uses.
unsafe impl Send for MnemonicResolverHandle {}
unsafe impl Sync for MnemonicResolverHandle {}

/// Build a new resolver handle that wraps a Swift-owned `ctx`
/// pointer plus a pair of function pointers.
///
/// # Safety
/// - `resolve_callback` must conform to [`MnemonicResolveCallback`]'s
///   contract.
/// - `destroy_callback` must safely free `ctx` exactly once.
/// - `ctx` may be null (the destructor will be invoked with null
///   on destroy regardless).
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_mnemonic_resolver_create(
    ctx: *mut c_void,
    resolve_callback: MnemonicResolveCallback,
    destroy_callback: unsafe extern "C" fn(ctx: *mut c_void),
) -> *mut MnemonicResolverHandle {
    // Allocate the vtable on the heap so it has a stable address
    // for the lifetime of the handle — same pattern
    // `dash_sdk_signer_create_with_ctx` follows.
    let vtable = Box::into_raw(Box::new(MnemonicResolverVTable {
        resolve: resolve_callback,
        destroy: destroy_callback,
    }));
    Box::into_raw(Box::new(MnemonicResolverHandle { ctx, vtable }))
}

/// Destroy a previously-created resolver handle.
///
/// Calls the supplied destructor exactly once with the original
/// `ctx`, then frees the heap-allocated vtable and the handle box.
/// Safe to call with a null pointer (no-op).
///
/// # Safety
/// `handle` must have been produced by
/// [`dash_sdk_mnemonic_resolver_create`] and must not have been
/// destroyed already.
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_mnemonic_resolver_destroy(handle: *mut MnemonicResolverHandle) {
    if handle.is_null() {
        return;
    }
    let owned = unsafe { Box::from_raw(handle) };
    unsafe {
        if !owned.vtable.is_null() {
            ((*owned.vtable).destroy)(owned.ctx);
            // Reclaim the vtable box.
            let _ = Box::from_raw(owned.vtable);
        }
    }
}
