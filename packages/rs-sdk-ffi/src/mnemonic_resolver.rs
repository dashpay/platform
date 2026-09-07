//! Rust → Swift "fetch BIP-39 mnemonic (+ optional passphrase) for
//! wallet_id" FFI vtable.
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

use key_wallet::mnemonic::Mnemonic;
use zeroize::Zeroizing;

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

/// Maximum BIP-39 passphrase length, in bytes (excluding the trailing
/// NUL), the resolver buffer can hold.
///
/// BIP-39 places no bound on the passphrase; this is the wire cap the
/// resolver contract imposes so the Rust side can keep the buffer on the
/// stack and zero it on drop. 1024 bytes is far beyond anything a person
/// types or a hardware wallet accepts (Trezor caps at 50 bytes).
pub const PASSPHRASE_RESOLVER_BUFFER_CAPACITY: usize = 1024;

/// Resolver result codes returned by [`MnemonicResolveCallback`].
///
/// Mirrors the success/failure shape of `PlatformWalletFFIResult`
/// at a finer-grained level so the Rust side can distinguish
/// "Swift hit the buffer cap" from "Swift had no mnemonic stored
/// for this wallet".
pub mod mnemonic_resolver_result {
    /// Mnemonic copied into the buffer; `out_len` was set. The
    /// passphrase buffer holds the wallet's BIP-39 passphrase with
    /// `*out_passphrase_len` set — `0` when the wallet has none.
    pub const SUCCESS: i32 = 0;
    /// The Swift side has no mnemonic stored for this `wallet_id`.
    /// Surfaced to the Rust caller as
    /// `PlatformWalletFFIResult::ErrorWalletOperation` with a
    /// "mnemonic missing" detail.
    pub const NOT_FOUND: i32 = 1;
    /// Mnemonic exceeded [`super::MNEMONIC_RESOLVER_BUFFER_CAPACITY`]
    /// or the passphrase exceeded
    /// [`super::PASSPHRASE_RESOLVER_BUFFER_CAPACITY`]. Should not happen
    /// in practice — the buffers are sized for every BIP-39 wordlist's
    /// 24-word phrase plus margin, and for any passphrase a person types.
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
/// - `out_passphrase_utf8`: writable buffer for the NUL-terminated
///   UTF-8 BIP-39 passphrase ("25th word") of the wallet, if it has
///   one. Capacity is [`PASSPHRASE_RESOLVER_BUFFER_CAPACITY`] bytes.
/// - `out_passphrase_capacity`: equal to
///   [`PASSPHRASE_RESOLVER_BUFFER_CAPACITY`].
/// - `out_passphrase_len`: receives the byte count written to
///   `out_passphrase_utf8`, EXCLUDING the trailing NUL. Must be set
///   on success — `0` means the wallet has no passphrase, and Rust
///   then derives the seed exactly as it always has (`to_seed("")`).
///
/// # Safety
///
/// - `out_mnemonic_utf8` must be valid for `out_capacity` writable
///   bytes for the duration of this call.
/// - `out_passphrase_utf8` must be valid for `out_passphrase_capacity`
///   writable bytes for the duration of this call.
/// - `wallet_id_bytes` must be valid for 32 readable bytes.
/// - `out_len` and `out_passphrase_len` must each be valid for one
///   `usize` write.
/// - The implementation MUST return one of the
///   [`mnemonic_resolver_result`] codes; any other value is treated
///   as `OTHER`.
pub type MnemonicResolveCallback = unsafe extern "C" fn(
    ctx: *const c_void,
    wallet_id_bytes: *const u8,
    out_mnemonic_utf8: *mut c_char,
    out_capacity: usize,
    out_len: *mut usize,
    out_passphrase_utf8: *mut c_char,
    out_passphrase_capacity: usize,
    out_passphrase_len: *mut usize,
) -> i32;

/// C-compatible vtable for a mnemonic resolver.
#[repr(C)]
pub struct MnemonicResolverVTable {
    /// Synchronous "fetch mnemonic + passphrase for `wallet_id`".
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

/// Why a resolve failed, as the Rust side of the vtable sees it.
///
/// Every consumer of the resolver used to hand-roll the result-code
/// match, the length check, the UTF-8 check and the wordlist parse;
/// this enum is the single shape those checks now report through so
/// each FFI entry point only has to map it onto its own error surface.
///
/// The `Display` text is what the FFI entry points hand back as their
/// error detail, so it deliberately carries no payload: a resolver
/// return code or a bogus length is host-side framing noise, not
/// something a caller can act on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum ResolveSeedError {
    /// The resolver reported no stored mnemonic for this wallet id
    /// ([`mnemonic_resolver_result::NOT_FOUND`]).
    #[error("mnemonic resolver: no mnemonic stored for the supplied wallet_id")]
    NotFound,
    /// The mnemonic or the passphrase did not fit its buffer
    /// ([`mnemonic_resolver_result::BUFFER_TOO_SMALL`]).
    #[error("mnemonic resolver: mnemonic or passphrase exceeded the FFI buffer capacity")]
    BufferTooSmall,
    /// Any other resolver return code (Keychain locked / denied, …).
    /// Carries the raw code.
    #[error("mnemonic resolver: failed (other / Keychain access error)")]
    ResolverFailed(i32),
    /// The resolver claimed a mnemonic length of zero or beyond the
    /// buffer capacity — a framing bug on the host side.
    #[error("mnemonic resolver: returned invalid length")]
    InvalidMnemonicLength(usize),
    /// The resolver claimed a passphrase length beyond the buffer
    /// capacity — a framing bug on the host side.
    #[error("mnemonic resolver: returned invalid length")]
    InvalidPassphraseLength(usize),
    /// Mnemonic or passphrase bytes were not valid UTF-8.
    #[error("mnemonic resolver: returned invalid UTF-8")]
    InvalidUtf8,
    /// The mnemonic matched no supported BIP-39 wordlist / failed its
    /// checksum.
    #[error("mnemonic resolver: returned an invalid mnemonic")]
    InvalidMnemonic,
}

/// Fire the resolver once for `wallet_id` and return the wallet's
/// 64-byte BIP-39 seed — `PBKDF2(mnemonic, passphrase)` with the
/// passphrase the host stored for this wallet (empty when it has none).
///
/// This is the ONE place the resolver vtable is consumed. Every FFI
/// entry point that derives from a Keychain-resident wallet routes
/// through here, so the passphrase cannot be dropped on one path and
/// honoured on another. Both host-written buffers and the mnemonic are
/// held in [`Zeroizing`] storage and scrubbed before this returns; the
/// returned seed is scrubbed when the caller drops it.
///
/// # Safety
/// `handle` must be non-null, come from
/// [`dash_sdk_mnemonic_resolver_create`], and stay valid for the
/// duration of the call.
pub unsafe fn resolve_seed(
    handle: *const MnemonicResolverHandle,
    wallet_id: &[u8; 32],
) -> Result<Zeroizing<[u8; 64]>, ResolveSeedError> {
    let mut mnemonic_buf: Zeroizing<[u8; MNEMONIC_RESOLVER_BUFFER_CAPACITY]> =
        Zeroizing::new([0u8; MNEMONIC_RESOLVER_BUFFER_CAPACITY]);
    let mut mnemonic_len: usize = 0;
    let mut passphrase_buf: Zeroizing<[u8; PASSPHRASE_RESOLVER_BUFFER_CAPACITY]> =
        Zeroizing::new([0u8; PASSPHRASE_RESOLVER_BUFFER_CAPACITY]);
    let mut passphrase_len: usize = 0;

    let resolver = &*handle;
    let vtable = &*resolver.vtable;
    let rc = (vtable.resolve)(
        resolver.ctx as *const c_void,
        wallet_id.as_ptr(),
        mnemonic_buf.as_mut_ptr() as *mut c_char,
        MNEMONIC_RESOLVER_BUFFER_CAPACITY,
        &mut mnemonic_len,
        passphrase_buf.as_mut_ptr() as *mut c_char,
        PASSPHRASE_RESOLVER_BUFFER_CAPACITY,
        &mut passphrase_len,
    );
    match rc {
        x if x == mnemonic_resolver_result::SUCCESS => {}
        x if x == mnemonic_resolver_result::NOT_FOUND => return Err(ResolveSeedError::NotFound),
        x if x == mnemonic_resolver_result::BUFFER_TOO_SMALL => {
            return Err(ResolveSeedError::BufferTooSmall)
        }
        other => return Err(ResolveSeedError::ResolverFailed(other)),
    }
    if mnemonic_len == 0 || mnemonic_len > MNEMONIC_RESOLVER_BUFFER_CAPACITY {
        return Err(ResolveSeedError::InvalidMnemonicLength(mnemonic_len));
    }
    if passphrase_len > PASSPHRASE_RESOLVER_BUFFER_CAPACITY {
        return Err(ResolveSeedError::InvalidPassphraseLength(passphrase_len));
    }

    // UTF-8 validation runs over the claimed prefixes only — no owned
    // `String` is ever built (the buffers are dropped via `Zeroizing`).
    let mnemonic_str = std::str::from_utf8(&mnemonic_buf[..mnemonic_len])
        .map_err(|_| ResolveSeedError::InvalidUtf8)?;
    let passphrase_str = std::str::from_utf8(&passphrase_buf[..passphrase_len])
        .map_err(|_| ResolveSeedError::InvalidUtf8)?;
    let mnemonic = Mnemonic::from_phrase_in_any_language(mnemonic_str)
        .map_err(|_| ResolveSeedError::InvalidMnemonic)?;

    // `to_seed` NFKD-normalizes the passphrase itself (BIP-39 §"From
    // mnemonic to seed"), so the host may pass it exactly as typed.
    let seed = Zeroizing::new(mnemonic.to_seed(passphrase_str));
    drop(mnemonic);
    Ok(seed)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// English BIP-39 test vector (all-zero entropy).
    const ENGLISH_PHRASE: &str =
        "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";

    /// Seed of `ENGLISH_PHRASE` with the empty passphrase (bip39 vectors).
    const SEED_NO_PASSPHRASE: &str = "5eb00bbddcf069084889a8ab9155568165f5c453ccb85e70811aaed6f6da5fc19a5ac40b389cd370d086206dec8aa6c43daea6690f20ad3d8d48b2d2ce9e38e4";
    /// Seed of `ENGLISH_PHRASE` with the passphrase `TREZOR` (bip39 vectors).
    const SEED_TREZOR: &str = "c55257c360c07c72029aebc1b53c05ed0362ada38ead3e3e9efa3708e53495531f09a6987599d18264c1e1c92f2cf141630c7a3c4ab7c81b2f001698e7463b04";

    fn hex(bytes: &[u8]) -> String {
        bytes.iter().map(|b| format!("{b:02x}")).collect()
    }

    /// Copies `phrase` and `passphrase` into the resolver's out buffers
    /// with the wire contract this module documents.
    #[allow(clippy::too_many_arguments)]
    unsafe fn fill(
        phrase: &str,
        passphrase: &str,
        out_buf: *mut c_char,
        out_capacity: usize,
        out_len: *mut usize,
        out_pp: *mut c_char,
        out_pp_capacity: usize,
        out_pp_len: *mut usize,
    ) -> i32 {
        let phrase = phrase.as_bytes();
        let passphrase = passphrase.as_bytes();
        if phrase.len() + 1 > out_capacity || passphrase.len() + 1 > out_pp_capacity {
            return mnemonic_resolver_result::BUFFER_TOO_SMALL;
        }
        std::ptr::copy_nonoverlapping(phrase.as_ptr() as *const c_char, out_buf, phrase.len());
        *out_buf.add(phrase.len()) = 0;
        *out_len = phrase.len();
        std::ptr::copy_nonoverlapping(
            passphrase.as_ptr() as *const c_char,
            out_pp,
            passphrase.len(),
        );
        *out_pp.add(passphrase.len()) = 0;
        *out_pp_len = passphrase.len();
        mnemonic_resolver_result::SUCCESS
    }

    unsafe extern "C" fn resolve_no_passphrase(
        _ctx: *const c_void,
        _wallet_id: *const u8,
        out_buf: *mut c_char,
        out_capacity: usize,
        out_len: *mut usize,
        out_pp: *mut c_char,
        out_pp_capacity: usize,
        out_pp_len: *mut usize,
    ) -> i32 {
        fill(
            ENGLISH_PHRASE,
            "",
            out_buf,
            out_capacity,
            out_len,
            out_pp,
            out_pp_capacity,
            out_pp_len,
        )
    }

    unsafe extern "C" fn resolve_trezor_passphrase(
        _ctx: *const c_void,
        _wallet_id: *const u8,
        out_buf: *mut c_char,
        out_capacity: usize,
        out_len: *mut usize,
        out_pp: *mut c_char,
        out_pp_capacity: usize,
        out_pp_len: *mut usize,
    ) -> i32 {
        fill(
            ENGLISH_PHRASE,
            "TREZOR",
            out_buf,
            out_capacity,
            out_len,
            out_pp,
            out_pp_capacity,
            out_pp_len,
        )
    }

    /// A host that leaves the passphrase buffer untouched but still
    /// reports `len = 0` — the shape a "no passphrase" wallet produces.
    unsafe extern "C" fn resolve_passphrase_len_untouched_buffer(
        _ctx: *const c_void,
        _wallet_id: *const u8,
        out_buf: *mut c_char,
        out_capacity: usize,
        out_len: *mut usize,
        _out_pp: *mut c_char,
        _out_pp_capacity: usize,
        out_pp_len: *mut usize,
    ) -> i32 {
        let phrase = ENGLISH_PHRASE.as_bytes();
        assert!(phrase.len() < out_capacity);
        std::ptr::copy_nonoverlapping(phrase.as_ptr() as *const c_char, out_buf, phrase.len());
        *out_buf.add(phrase.len()) = 0;
        *out_len = phrase.len();
        *out_pp_len = 0;
        mnemonic_resolver_result::SUCCESS
    }

    unsafe extern "C" fn resolve_not_found(
        _ctx: *const c_void,
        _wallet_id: *const u8,
        _out_buf: *mut c_char,
        _out_capacity: usize,
        _out_len: *mut usize,
        _out_pp: *mut c_char,
        _out_pp_capacity: usize,
        _out_pp_len: *mut usize,
    ) -> i32 {
        mnemonic_resolver_result::NOT_FOUND
    }

    unsafe extern "C" fn resolve_absurd_passphrase_len(
        _ctx: *const c_void,
        _wallet_id: *const u8,
        out_buf: *mut c_char,
        out_capacity: usize,
        out_len: *mut usize,
        _out_pp: *mut c_char,
        _out_pp_capacity: usize,
        out_pp_len: *mut usize,
    ) -> i32 {
        let phrase = ENGLISH_PHRASE.as_bytes();
        assert!(phrase.len() < out_capacity);
        std::ptr::copy_nonoverlapping(phrase.as_ptr() as *const c_char, out_buf, phrase.len());
        *out_len = phrase.len();
        *out_pp_len = PASSPHRASE_RESOLVER_BUFFER_CAPACITY + 1;
        mnemonic_resolver_result::SUCCESS
    }

    unsafe extern "C" fn noop_destroy(_ctx: *mut c_void) {}

    fn with_resolver<R>(
        cb: MnemonicResolveCallback,
        body: impl FnOnce(*mut MnemonicResolverHandle) -> R,
    ) -> R {
        let handle =
            unsafe { dash_sdk_mnemonic_resolver_create(std::ptr::null_mut(), cb, noop_destroy) };
        let out = body(handle);
        unsafe { dash_sdk_mnemonic_resolver_destroy(handle) };
        out
    }

    #[test]
    fn empty_passphrase_yields_the_bip39_vector_seed() {
        let seed = with_resolver(resolve_no_passphrase, |h| unsafe {
            resolve_seed(h, &[0u8; 32])
        })
        .expect("resolves");
        assert_eq!(hex(seed.as_ref()), SEED_NO_PASSPHRASE);
    }

    #[test]
    fn passphrase_changes_the_seed_to_the_bip39_vector() {
        let seed = with_resolver(resolve_trezor_passphrase, |h| unsafe {
            resolve_seed(h, &[0u8; 32])
        })
        .expect("resolves");
        assert_eq!(hex(seed.as_ref()), SEED_TREZOR);
    }

    #[test]
    fn zero_passphrase_length_means_no_passphrase_regardless_of_buffer_contents() {
        let seed = with_resolver(resolve_passphrase_len_untouched_buffer, |h| unsafe {
            resolve_seed(h, &[0u8; 32])
        })
        .expect("resolves");
        assert_eq!(hex(seed.as_ref()), SEED_NO_PASSPHRASE);
    }

    #[test]
    fn not_found_is_reported_as_such() {
        let err = with_resolver(resolve_not_found, |h| unsafe {
            resolve_seed(h, &[0u8; 32])
        })
        .expect_err("must fail");
        assert_eq!(err, ResolveSeedError::NotFound);
    }

    #[test]
    fn an_impossible_passphrase_length_is_rejected() {
        let err = with_resolver(resolve_absurd_passphrase_len, |h| unsafe {
            resolve_seed(h, &[0u8; 32])
        })
        .expect_err("must fail");
        assert_eq!(
            err,
            ResolveSeedError::InvalidPassphraseLength(PASSPHRASE_RESOLVER_BUFFER_CAPACITY + 1)
        );
    }
}
