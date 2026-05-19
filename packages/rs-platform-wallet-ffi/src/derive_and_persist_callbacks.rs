//! Identity-key persister C-ABI callback handle backing the
//! [`crate::dash_sdk_derive_and_persist_identity_keys`] entry point
//! and friends.
//!
//! The architectural intent is captured in `swift-sdk/CLAUDE.md`'s
//! "no mnemonic round-tripping" rule: derivation pipelines must live
//! on the Rust side, with Swift exposing only the two operations
//! Rust cannot perform from its side — reading the mnemonic from
//! the iOS Keychain and writing the derived key bytes back. This
//! module exposes the **write** half (identity-key persister); the
//! **read** half (mnemonic resolver) lives in
//! [`rs_sdk_ffi::mnemonic_resolver`](::rs_sdk_ffi::mnemonic_resolver)
//! so non-wallet SDK callers can share it.
//!
//! # Why split from the resolver
//!
//! Conceptually they could ride on a single vtable, but the
//! lifetimes are different:
//!
//! - The mnemonic resolver is single-use per call (one read of one
//!   wallet's mnemonic per derivation invocation).
//! - The persister is multi-use per call (`key_count` writes per
//!   invocation).
//!
//! Splitting them also makes future extensions cleaner: a hardware-
//! key-store impl might persist via a Secure Enclave wrapper that has
//! nothing in common with how the mnemonic is fetched. Keeping the
//! persister wallet-side (rather than promoting it to rs-sdk-ffi)
//! reflects that the Keychain-write semantics are wallet-domain:
//! `wallet_id` scoping, identity-index slot tagging, etc.
//!
//! # Synchronous on purpose
//!
//! Unlike the
//! [`SignerVTable`](rs_sdk_ffi::SignerVTable) infrastructure (which
//! uses a `oneshot` channel + `CompletionSlot` because biometric
//! prompts can take seconds), Keychain writes are sub-millisecond,
//! so the persister callback is fully synchronous — no
//! completion-callback ceremony, no `tokio::time::timeout`.
//! The Rust side blocks the calling thread for the duration of one
//! Keychain hop. Callers that already invoke the derivation FFI on
//! a background queue (the iOS pattern) get exactly what they want.
//!
//! # Lifetime / cleanup
//!
//! The Swift side calls [`dash_sdk_identity_key_persister_create`],
//! which returns an opaque `*mut IdentityKeyPersisterHandle` pointer.
//! Pass it to
//! [`crate::dash_sdk_derive_and_persist_identity_keys`]. Then call
//! [`dash_sdk_identity_key_persister_destroy`] — that fires the
//! supplied destructor callback (typically the
//! `Unmanaged.fromOpaque(...).release()` pattern Swift uses for its
//! `KeychainSigner` ctx) so the Swift object can drop.

use std::ffi::c_void;
use std::os::raw::c_char;

// ---------------------------------------------------------------------------
// Identity-key persister — Rust → Swift "save this derived key to Keychain"
// ---------------------------------------------------------------------------

/// `#[repr(C)]` struct passed to [`PersistKeyCallback`]. Pointer-
/// based to keep the call-site stable if we ever add per-key
/// metadata fields without breaking the ABI.
///
/// # Field invariants
///
/// - `wallet_id_bytes` always points at 32 readable bytes.
/// - `derivation_path_cstr` is a NUL-terminated UTF-8 string of
///   the canonical DIP-9 form
///   (`"m/9'/coin'/5'/0'/0'/identity_index'/key_index'"`).
/// - `public_key_bytes` / `public_key_len` carry the raw
///   compressed secp256k1 pubkey (currently always 33 bytes;
///   the explicit length is defensive against future key types).
/// - `public_key_hash_bytes` always points at 20 readable bytes
///   (the HASH160 of the compressed pubkey, computed once on the
///   Rust side via [`crate::utils::platform_wallet_hash160`] so
///   Swift doesn't have to recompute it for the Keychain
///   metadata).
/// - `private_key_bytes` always points at 32 readable bytes
///   (the raw ECDSA scalar). Treat as sensitive — copy into the
///   Keychain immediately and let the local reference drop.
/// - `key_type` / `purpose` / `security_level` are the DPP
///   discriminant bytes (see
///   `dpp::identity::{KeyType, Purpose, SecurityLevel}`). For the
///   identity-registration flow this is always
///   `(ECDSA_SECP256K1=0, AUTHENTICATION=0, MASTER=0 | HIGH=2)` —
///   the MASTER-vs-HIGH choice belongs in Rust where DPP's enum
///   discriminants live, not Swift.
#[repr(C)]
pub struct PersistKeyArgs {
    pub wallet_id_bytes: *const u8,
    pub identity_index: u32,
    pub key_id: u32,
    pub key_index: u32,
    pub derivation_path_cstr: *const c_char,
    pub public_key_bytes: *const u8,
    pub public_key_len: usize,
    pub public_key_hash_bytes: *const u8,
    pub private_key_bytes: *const u8,
    pub key_type: u8,
    pub purpose: u8,
    pub security_level: u8,
}

// Compile-time layout assertions. Mirrors the runtime
// `assertPersistKeyArgsLayout` check on the Swift side. If any
// field is added / reordered / resized below, BOTH sides need
// to be updated; the constants here turn drift into a build
// failure rather than an EXC_BAD_ACCESS at trampoline time.
//
// Layout on 64-bit ABI:
//
// | offset | field                  | size |
// |--------|------------------------|------|
// | 0      | wallet_id_bytes        | 8    |
// | 8      | identity_index         | 4    |
// | 12     | key_id                 | 4    |
// | 16     | key_index              | 4    |
// | 20     | (padding)              | 4    |
// | 24     | derivation_path_cstr   | 8    |
// | 32     | public_key_bytes       | 8    |
// | 40     | public_key_len         | 8    |
// | 48     | public_key_hash_bytes  | 8    |
// | 56     | private_key_bytes      | 8    |
// | 64     | key_type               | 1    |
// | 65     | purpose                | 1    |
// | 66     | security_level         | 1    |
// | 67     | (padding)              | 5    |
//
// Total = 72 bytes, alignment = 8 (pointer / usize).
const _: [u8; 72] = [0u8; std::mem::size_of::<PersistKeyArgs>()];
const _: [u8; 8] = [0u8; std::mem::align_of::<PersistKeyArgs>()];

/// Function pointer type for the per-key persist callback.
///
/// Returns a non-zero `u8` on a successful persist, `0` on
/// failure. On failure the Rust derivation loop aborts with an
/// `ErrorWalletOperation` and the partial state is the caller's
/// responsibility (in practice the Swift side either succeeds or
/// throws, not both).
///
/// `u8` (rather than `bool`) keeps the wire shape representable
/// in Swift's `@convention(c)` typealiases, which can only carry
/// types representable in Objective-C — Swift's `Bool` and
/// `DarwinBoolean` are not, but `UInt8` is. Same ABI footprint.
///
/// # Safety
/// - `args` must point at a valid [`PersistKeyArgs`] for the
///   duration of the call. All pointer fields inside `args` must
///   conform to their documented invariants.
/// - The implementation MUST NOT retain any of the pointer fields
///   past return — the underlying buffers (mnemonic, intermediate
///   xprivs, derived bytes) are zeroized on the Rust side as soon
///   as this function returns.
pub type PersistKeyCallback =
    unsafe extern "C" fn(ctx: *const c_void, args: *const PersistKeyArgs) -> u8;

/// Persist-callback success / failure tags. Mirrors a boolean
/// but in `u8` form so it crosses Swift's `@convention(c)` cleanly.
pub const PERSIST_KEY_SUCCESS: u8 = 1;
pub const PERSIST_KEY_FAILURE: u8 = 0;

/// C-compatible vtable for an identity-key persister.
#[repr(C)]
pub struct IdentityKeyPersisterVTable {
    pub persist_key: PersistKeyCallback,
    pub destroy: unsafe extern "C" fn(ctx: *mut c_void),
}

/// Opaque Rust-side handle to a Swift-owned identity-key persister.
#[repr(C)]
pub struct IdentityKeyPersisterHandle {
    pub(crate) ctx: *mut c_void,
    pub(crate) vtable: *mut IdentityKeyPersisterVTable,
}

unsafe impl Send for IdentityKeyPersisterHandle {}
unsafe impl Sync for IdentityKeyPersisterHandle {}

/// Build a new identity-key persister handle.
///
/// # Safety
/// - `persist_callback` must conform to [`PersistKeyCallback`]'s
///   contract.
/// - `destroy_callback` must safely free `ctx` exactly once.
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_identity_key_persister_create(
    ctx: *mut c_void,
    persist_callback: PersistKeyCallback,
    destroy_callback: unsafe extern "C" fn(ctx: *mut c_void),
) -> *mut IdentityKeyPersisterHandle {
    // Allocate the vtable on the heap so it has a stable address
    // for the lifetime of the handle — same pattern
    // `dash_sdk_signer_create_with_ctx` follows.
    let vtable = Box::into_raw(Box::new(IdentityKeyPersisterVTable {
        persist_key: persist_callback,
        destroy: destroy_callback,
    }));
    Box::into_raw(Box::new(IdentityKeyPersisterHandle { ctx, vtable }))
}

/// Destroy a previously-created persister handle.
///
/// Calls the supplied destructor exactly once with the original
/// `ctx`, then frees the heap-allocated vtable and the handle box.
/// Safe to call with a null pointer (no-op).
///
/// # Safety
/// `handle` must have been produced by
/// [`dash_sdk_identity_key_persister_create`] and must not have
/// been destroyed already.
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_identity_key_persister_destroy(
    handle: *mut IdentityKeyPersisterHandle,
) {
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

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

    static PERSISTER_CALLS: AtomicUsize = AtomicUsize::new(0);
    static PERSISTER_DESTROYED: AtomicBool = AtomicBool::new(false);

    unsafe extern "C" fn fake_persist(_ctx: *const c_void, _args: *const PersistKeyArgs) -> u8 {
        PERSISTER_CALLS.fetch_add(1, Ordering::SeqCst);
        PERSIST_KEY_SUCCESS
    }

    unsafe extern "C" fn fake_persister_destroy(_ctx: *mut c_void) {
        PERSISTER_DESTROYED.store(true, Ordering::SeqCst);
    }

    #[test]
    fn persister_create_destroy_roundtrip() {
        unsafe {
            let h = dash_sdk_identity_key_persister_create(
                std::ptr::null_mut(),
                fake_persist,
                fake_persister_destroy,
            );
            assert!(!h.is_null());

            let pubkey = [0u8; 33];
            let pubhash = [0u8; 20];
            let privkey = [0u8; 32];
            let path = b"m/9'/1'/5'/0'/0'/0'/0'\0";
            let wallet_id = [0u8; 32];
            let args = PersistKeyArgs {
                wallet_id_bytes: wallet_id.as_ptr(),
                identity_index: 0,
                key_id: 0,
                key_index: 0,
                derivation_path_cstr: path.as_ptr() as *const c_char,
                public_key_bytes: pubkey.as_ptr(),
                public_key_len: pubkey.len(),
                public_key_hash_bytes: pubhash.as_ptr(),
                private_key_bytes: privkey.as_ptr(),
                key_type: 0,
                purpose: 0,
                security_level: 0,
            };
            let ok = ((*(*h).vtable).persist_key)((*h).ctx, &args);
            assert_eq!(ok, PERSIST_KEY_SUCCESS);
            dash_sdk_identity_key_persister_destroy(h);
        }
        assert_eq!(PERSISTER_CALLS.load(Ordering::SeqCst), 1);
        assert!(PERSISTER_DESTROYED.load(Ordering::SeqCst));
    }

    #[test]
    fn destroy_persister_null_handle() {
        unsafe {
            // Should be a no-op.
            dash_sdk_identity_key_persister_destroy(std::ptr::null_mut());
        }
    }
}
