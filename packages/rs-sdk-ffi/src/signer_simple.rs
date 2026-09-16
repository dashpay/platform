//! Simple private key signer for iOS FFI

use crate::signer::{signer_handle_from_single_key, VTableSigner};
use crate::types::{FFINetwork, Network, SignerHandle};
use crate::{DashSDKError, DashSDKErrorCode, DashSDKResult};
use dash_sdk::dpp::identity::signer::Signer;
use dash_sdk::dpp::identity::{IdentityPublicKey, KeyType, Purpose, SecurityLevel};
use simple_signer::SingleKeySigner;
use zeroize::Zeroizing;

// Runtime-aware sync/async bridge (PR #3497/#3432); safer than an inline
// `Runtime::new().block_on(...)`.
use dash_async::block_on;

/// Parse a BIP-39 mnemonic phrase against every supported wordlist
/// in turn, returning the first language that yields a valid mnemonic.
///
/// `key_wallet::Mnemonic` only exposes language-tagged constructors
/// (`Mnemonic::from_phrase(phrase, lang)`); the upstream `bip39` crate
/// has a `Mnemonic::parse(phrase)` that auto-detects, but it is not
/// re-exported through the `key_wallet` wrapper. Until that helper is
/// surfaced upstream, callers who accept user-supplied mnemonics need
/// to walk the language list themselves so a French / Japanese / etc.
/// phrase is not rejected as "invalid English".
///
/// BIP-39 wordlists are designed to be mutually exclusive within a
/// single phrase, so the first match is unambiguous.
pub(crate) fn parse_mnemonic_any_language(
    phrase: &str,
) -> Result<key_wallet::mnemonic::Mnemonic, &'static str> {
    use key_wallet::mnemonic::Mnemonic;

    // Upstream's `from_phrase` IS the auto-detecting parse since
    // rust-dashcore#981 — one path, English diagnostics preserved when
    // nothing matches. This wrapper survives only to narrow the error to
    // the `&'static str` its callers report.
    Mnemonic::from_phrase(phrase).map_err(|_| "phrase does not match any supported BIP-39 wordlist")
}

/// Create a signer from a private key.
///
/// Internally wraps a `SingleKeySigner` as a native (non-callback) FFI
/// signer — there is no C-callback bounce on this path. The returned
/// handle still conforms to `Signer<IdentityPublicKey>` via the unified
/// `VTableSigner` wrapper so it can be passed to every other FFI entry
/// point that takes a `SignerHandle`.
///
/// # Safety
/// - `private_key` must be a valid pointer to at least 32 readable bytes.
/// - The function reads exactly `private_key_len` bytes; it must be 32.
/// - `network` selects the network used for WIF encoding / address
///   derivation; it does not affect signing.
/// - The returned handle inside DashSDKResult must be freed with
///   `dash_sdk_signer_destroy`.
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_signer_create_from_private_key(
    private_key: *const u8,
    private_key_len: usize,
    network: FFINetwork,
) -> DashSDKResult {
    if private_key.is_null() {
        return DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
            "Private key is null".to_string(),
        ));
    }

    if private_key_len != 32 {
        return DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
            format!("Private key must be 32 bytes, got {}", private_key_len),
        ));
    }

    let network: Network = network.into();

    // Copy into a zeroizing buffer so the key material doesn't linger on
    // the stack after we hand it off to SingleKeySigner.
    let key_slice = std::slice::from_raw_parts(private_key, 32);
    let mut key_array = Zeroizing::new([0u8; 32]);
    key_array.copy_from_slice(key_slice);

    let signer = match SingleKeySigner::new_from_slice(key_array.as_slice(), network) {
        Ok(s) => s,
        Err(e) => {
            return DashSDKResult::error(DashSDKError::new(DashSDKErrorCode::InvalidParameter, e));
        }
    };

    let handle = signer_handle_from_single_key(signer);
    DashSDKResult::success(handle as *mut std::os::raw::c_void)
}

/// Signature result structure.
#[repr(C)]
pub struct DashSDKSignature {
    pub signature: *mut u8,
    pub signature_len: usize,
}

/// Leak `bytes` into a raw `(ptr, len)` pair suitable for handing across
/// the FFI boundary inside [`DashSDKSignature`], to be reclaimed by
/// [`dash_sdk_signature_free`].
///
/// This mirrors the fix applied to [`crate::types::DashSDKResult::success_binary`]:
/// the source `Vec` may have `capacity > len` (e.g. it came from a
/// `BinaryData` whose inner buffer was over-allocated), but the free path
/// reconstructs with `Vec::from_raw_parts(ptr, len, len)`. Reconstructing
/// with `cap == len` is only sound when the *original* allocation also had
/// `cap == len`; otherwise the global allocator receives a `Layout` that
/// does not match the real allocation on dealloc — undefined behavior /
/// heap corruption.
///
/// `into_boxed_slice()` shrinks the allocation so that its capacity equals
/// its length before we leak it, making the `cap == len` free path sound.
/// A `Box<[u8]>` of length `len` owns an allocation of exactly `len` bytes
/// with the same layout `Vec<u8>` would use, so `Vec::from_raw_parts(ptr,
/// len, len)` reclaims it correctly.
fn leak_binary_to_ptr(bytes: Vec<u8>) -> (*mut u8, usize) {
    let boxed: Box<[u8]> = bytes.into_boxed_slice();
    let len = boxed.len();
    let ptr = Box::into_raw(boxed) as *mut u8;
    (ptr, len)
}

/// Sign data with a signer.
///
/// # Safety
/// - `signer_handle` must be a valid pointer obtained from this SDK and not
///   previously destroyed.
/// - `data` must be a valid pointer to `data_len` readable bytes.
/// - The returned signature pointer inside DashSDKResult must be freed with
///   `dash_sdk_signature_free`.
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_signer_sign(
    signer_handle: *mut SignerHandle,
    data: *const u8,
    data_len: usize,
) -> DashSDKResult {
    if signer_handle.is_null() {
        return DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
            "Signer handle is null".to_string(),
        ));
    }

    if data.is_null() {
        return DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
            "Data is null".to_string(),
        ));
    }

    // Own the data so the future satisfies `block_on`'s `Send + 'static`.
    let data_owned: Vec<u8> = std::slice::from_raw_parts(data, data_len).to_vec();

    // Create a dummy identity public key for signing. SingleKeySigner
    // doesn't actually use the key data, just needs one to satisfy the
    // trait contract.
    let dummy_key = IdentityPublicKey::V0(
        dash_sdk::dpp::identity::identity_public_key::v0::IdentityPublicKeyV0 {
            id: 0,
            key_type: KeyType::ECDSA_SECP256K1,
            purpose: Purpose::AUTHENTICATION,
            security_level: SecurityLevel::HIGH,
            data: vec![0; 33].into(),
            read_only: false,
            disabled_at: None,
            contract_bounds: None,
        },
    );

    // SAFETY: `signer_handle` is valid for the call; `block_on` blocks the
    // caller so the allocation can't be freed mid-sign. `VTableSigner: Send +
    // Sync`. The usize round-trip gives the `async move` a `Send + 'static`
    // capture of the !Clone handle.
    let signer_addr = signer_handle as usize;
    let result = block_on(async move {
        let signer: &VTableSigner = unsafe { &*(signer_addr as *const VTableSigner) };
        signer.sign(&dummy_key, &data_owned).await
    });

    match result {
        Ok(Ok(signature)) => {
            // Shrink the allocation so capacity == len before leaking, so
            // that `dash_sdk_signature_free`'s `Vec::from_raw_parts(ptr, len,
            // len)` reconstruction matches the real allocation layout. See
            // `leak_binary_to_ptr` for why this is required for soundness.
            let (sig_ptr, sig_len) = leak_binary_to_ptr(signature.to_vec());

            let boxed = Box::new(DashSDKSignature {
                signature: sig_ptr,
                signature_len: sig_len,
            });

            DashSDKResult::success(Box::into_raw(boxed) as *mut std::os::raw::c_void)
        }
        Ok(Err(e)) => DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::CryptoError,
            format!("Failed to sign: {}", e),
        )),
        Err(e) => DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InternalError,
            format!("Async bridge failed during signing: {}", e),
        )),
    }
}

/// Free a signature.
///
/// # Safety
/// - `signature` must be a valid pointer returned by this SDK, or null for
///   no-op.
/// - After this call the pointer must not be used again.
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_signature_free(signature: *mut DashSDKSignature) {
    if !signature.is_null() {
        let sig = Box::from_raw(signature);
        if !sig.signature.is_null() {
            // Reconstruct the Vec to properly deallocate.
            let _ = Vec::from_raw_parts(sig.signature, sig.signature_len, sig.signature_len);
        }
    }
}

// ---------------------------------------------------------------------------
// One-shot derive-then-sign FFI for platform-address keys
// ---------------------------------------------------------------------------
//
// The KeychainSigner trampoline calls this on the `key_type == 0xFF`
// branch (platform-address signing). The derived key never crosses
// the FFI as bytes — Swift hands in the mnemonic + a derivation path,
// gets back just a signature. Both the seed and the derived key are
// held inside `Zeroizing` buffers and dropped at the end of the call.
//
// This is intentionally narrow: ECDSA secp256k1 only, single derivation
// path, no metadata. The Swift caller is responsible for pulling the
// path off `PersistentPlatformAddress.derivationPath` and the mnemonic
// off Keychain (`WalletStorage.retrieveMnemonic(for:)`) per signing
// call.

/// Error categories returned via `out_error` from
/// [`dash_sdk_sign_with_mnemonic_and_path`]. Kept as bytes (rather
/// than a `c_char*` or `DashSDKError*`) so the Swift trampoline can
/// branch on a single value without parsing strings — the failure
/// modes here are few enough that a one-byte tag is more useful than
/// a freeform message.
pub const SIGN_WITH_MNEMONIC_OK: u8 = 0;
pub const SIGN_WITH_MNEMONIC_ERR_NULL_POINTER: u8 = 1;
pub const SIGN_WITH_MNEMONIC_ERR_INVALID_UTF8: u8 = 2;
pub const SIGN_WITH_MNEMONIC_ERR_INVALID_MNEMONIC: u8 = 3;
pub const SIGN_WITH_MNEMONIC_ERR_INVALID_PATH: u8 = 4;
pub const SIGN_WITH_MNEMONIC_ERR_DERIVATION: u8 = 5;
pub const SIGN_WITH_MNEMONIC_ERR_SIGN: u8 = 6;
pub const SIGN_WITH_MNEMONIC_ERR_BUFFER_TOO_SMALL: u8 = 7;
pub const SIGN_WITH_MNEMONIC_ERR_UNSUPPORTED_KEY_TYPE: u8 = 8;

/// Derive an ECDSA secp256k1 private key from
/// `(mnemonic, passphrase, derivation_path)`, sign `data` with it,
/// write the signature to `out_signature`. The derived key is held
/// only for the duration of this call; both the seed and the key
/// buffers are zeroed before return via [`zeroize::Zeroizing`].
///
/// `out_signature` must point at a writable buffer of
/// `out_signature_capacity` bytes. On success `*out_signature_len` is
/// set to the actual signature length (65 bytes for compact recoverable
/// ECDSA signatures produced by `dashcore::signer::sign`).
///
/// `key_type` is the standard `KeyType` discriminant byte. For
/// platform-address signing it's always `KeyType::ECDSA_SECP256K1 = 0`;
/// the parameter exists for parity with the trampoline shape and to
/// fail loudly with [`SIGN_WITH_MNEMONIC_ERR_UNSUPPORTED_KEY_TYPE`]
/// if a non-ECDSA key_type is ever requested through this path. Other
/// key types can be added when the `Signer<PlatformAddress>` protocol
/// grows beyond identity keys.
///
/// `network` selects the network passed to
/// `ExtendedPrivKey::new_master`. The derived secret bytes are
/// network-independent (BIP-32 derivation only uses `network` as a
/// tag on the `ExtendedPrivKey`), but we thread it through so the
/// type matches every other key-derivation entry point in this crate.
///
/// On any failure the function writes a non-zero tag into `*out_error`,
/// zeroes the first `out_signature_capacity` bytes of `out_signature`,
/// sets `*out_signature_len = 0`, and returns a non-zero `i32`.
///
/// Return value:
/// - `0`  → success.
/// - `-1` → error (see `*out_error` for the category).
///
/// # Safety
/// - `mnemonic_cstr` and `derivation_path_cstr` must be valid,
///   null-terminated UTF-8 C strings for the duration of the call.
/// - `passphrase_cstr` may be null (empty passphrase).
/// - `data` must point at `data_len` readable bytes (may be zero only
///   if `data_len == 0`).
/// - `out_signature` must point at `out_signature_capacity` writable
///   bytes; `out_signature_len` and `out_error` must be writable.
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub unsafe extern "C" fn dash_sdk_sign_with_mnemonic_and_path(
    mnemonic_cstr: *const std::os::raw::c_char,
    passphrase_cstr: *const std::os::raw::c_char,
    derivation_path_cstr: *const std::os::raw::c_char,
    data: *const u8,
    data_len: usize,
    key_type: u8,
    network: FFINetwork,
    out_signature: *mut u8,
    out_signature_capacity: usize,
    out_signature_len: *mut usize,
    out_error: *mut u8,
) -> i32 {
    use dash_sdk::dpp::dashcore::secp256k1::Secp256k1;
    use dash_sdk::dpp::identity::KeyType;
    use key_wallet::bip32::{DerivationPath, ExtendedPrivKey};
    use std::ffi::CStr;
    use std::str::FromStr;

    // Internal helper that scrubs `out_signature` + `*out_signature_len`
    // and returns the failure tag. Branchless to the eye, single exit
    // point for every error path.
    let fail = |tag: u8| -> i32 {
        if !out_error.is_null() {
            *out_error = tag;
        }
        if !out_signature_len.is_null() {
            *out_signature_len = 0;
        }
        if !out_signature.is_null() && out_signature_capacity > 0 {
            std::ptr::write_bytes(out_signature, 0, out_signature_capacity);
        }
        -1
    };

    // ---- Argument validation -------------------------------------------------
    if mnemonic_cstr.is_null()
        || derivation_path_cstr.is_null()
        || out_signature.is_null()
        || out_signature_len.is_null()
        || (data.is_null() && data_len > 0)
    {
        return fail(SIGN_WITH_MNEMONIC_ERR_NULL_POINTER);
    }

    // ECDSA-only entry point. Any other key_type is a contract
    // violation by the caller — surface it instead of pretending to
    // succeed.
    if key_type != KeyType::ECDSA_SECP256K1 as u8 {
        return fail(SIGN_WITH_MNEMONIC_ERR_UNSUPPORTED_KEY_TYPE);
    }

    // ---- UTF-8 inputs --------------------------------------------------------
    let mnemonic_str = match CStr::from_ptr(mnemonic_cstr).to_str() {
        Ok(s) => s,
        Err(_) => return fail(SIGN_WITH_MNEMONIC_ERR_INVALID_UTF8),
    };
    let passphrase_str: &str = if passphrase_cstr.is_null() {
        ""
    } else {
        match CStr::from_ptr(passphrase_cstr).to_str() {
            Ok(s) => s,
            Err(_) => return fail(SIGN_WITH_MNEMONIC_ERR_INVALID_UTF8),
        }
    };
    let path_str = match CStr::from_ptr(derivation_path_cstr).to_str() {
        Ok(s) => s,
        Err(_) => return fail(SIGN_WITH_MNEMONIC_ERR_INVALID_UTF8),
    };

    // ---- Mnemonic + seed -----------------------------------------------------
    // Walk every supported BIP-39 wordlist (English, Spanish, French,
    // Italian, Japanese, Korean, both Chinese, Czech, Portuguese) so a
    // non-English mnemonic is not falsely rejected. The wordlists are
    // disjoint per the BIP-39 spec, so the first match is the right
    // language.
    let mnemonic = match parse_mnemonic_any_language(mnemonic_str) {
        Ok(m) => m,
        Err(_) => return fail(SIGN_WITH_MNEMONIC_ERR_INVALID_MNEMONIC),
    };
    // `to_seed` returns `[u8; 64]` by value; wrap immediately so the
    // 64 bytes get zeroed when this function returns (success or fail).
    let seed: zeroize::Zeroizing<[u8; 64]> =
        zeroize::Zeroizing::new(mnemonic.to_seed(passphrase_str));

    // ---- Derivation path -----------------------------------------------------
    let path = match DerivationPath::from_str(path_str) {
        Ok(p) => p,
        Err(_) => return fail(SIGN_WITH_MNEMONIC_ERR_INVALID_PATH),
    };

    let network: Network = network.into();

    // ---- Derive ECDSA private key at path -----------------------------------
    let master = match ExtendedPrivKey::new_master(network, seed.as_ref()) {
        Ok(m) => m,
        Err(_) => return fail(SIGN_WITH_MNEMONIC_ERR_DERIVATION),
    };
    let secp = Secp256k1::new();
    let derived = match master.derive_priv(&secp, &path) {
        Ok(d) => d,
        Err(_) => return fail(SIGN_WITH_MNEMONIC_ERR_DERIVATION),
    };
    // Copy the 32 secret bytes into a `Zeroizing` so this fresh array —
    // which has no `Drop` of its own — is scrubbed when the function
    // returns. `derived` self-wipes separately: `ExtendedPrivKey`
    // zeroizes on `Drop`.
    let secret_bytes: zeroize::Zeroizing<[u8; 32]> =
        zeroize::Zeroizing::new(derived.private_key.secret_bytes());

    // ---- Sign ---------------------------------------------------------------
    // `dashcore::signer::sign` returns a 65-byte compact recoverable
    // ECDSA signature — the same shape used by the existing
    // `Signer<PlatformAddress>` impls in `rs-platform-wallet`.
    let data_slice: &[u8] = if data_len == 0 {
        &[]
    } else {
        std::slice::from_raw_parts(data, data_len)
    };
    let sig_array = match dash_sdk::dpp::dashcore::signer::sign(data_slice, secret_bytes.as_ref()) {
        Ok(s) => s,
        Err(_) => return fail(SIGN_WITH_MNEMONIC_ERR_SIGN),
    };

    if sig_array.len() > out_signature_capacity {
        return fail(SIGN_WITH_MNEMONIC_ERR_BUFFER_TOO_SMALL);
    }

    // ---- Copy signature out --------------------------------------------------
    std::ptr::copy_nonoverlapping(sig_array.as_ptr(), out_signature, sig_array.len());
    *out_signature_len = sig_array.len();
    if !out_error.is_null() {
        *out_error = SIGN_WITH_MNEMONIC_OK;
    }

    // `seed` and `secret_bytes` are zeroed via `Zeroizing` on Drop here.
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    /// English BIP-39 test vector (all-zero entropy).
    const ENGLISH_PHRASE: &str =
        "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";

    /// Japanese BIP-39 test vector (all-zero entropy). Uses the
    /// ideographic-space-separated form per the Japanese wordlist
    /// convention; the upstream `bip39` crate normalizes both forms.
    const JAPANESE_PHRASE: &str = "あいこくしん　あいこくしん　あいこくしん　あいこくしん　あいこくしん　あいこくしん　あいこくしん　あいこくしん　あいこくしん　あいこくしん　あいこくしん　あおぞら";

    #[test]
    fn parse_mnemonic_any_language_accepts_english() {
        let m = parse_mnemonic_any_language(ENGLISH_PHRASE)
            .expect("English phrase must parse via auto-detection");
        assert_eq!(m.word_count(), 12);
    }

    #[test]
    fn parse_mnemonic_any_language_accepts_japanese() {
        // Distinct wordlist from English — exercises the
        // "first-language-doesn't-match, fall through" branch of the
        // detection loop. Ensures non-English phrases are not rejected
        // as `ERR_INVALID_MNEMONIC`.
        let m = parse_mnemonic_any_language(JAPANESE_PHRASE)
            .expect("Japanese phrase must parse via auto-detection");
        assert_eq!(m.word_count(), 12);
    }

    #[test]
    fn parse_mnemonic_any_language_rejects_garbage() {
        // No supported wordlist contains these tokens, so every
        // attempt in the loop must fail and the helper must surface
        // the catch-all error.
        assert!(parse_mnemonic_any_language("not a real bip39 phrase at all here").is_err());
    }

    /// Regression test for the signature alloc/free round-trip.
    ///
    /// Mirrors `types::tests::test_success_binary_preserves_capacity_via_shrink`.
    /// The signing path used to do `signature.to_vec().leak()`, which keeps
    /// the source `Vec`'s *actual capacity*, while `dash_sdk_signature_free`
    /// reconstructs with `Vec::from_raw_parts(ptr, len, len)`. When the
    /// source capacity exceeded its length (as `BinaryData`'s inner `Vec`
    /// can), the allocator received a wrong `Layout` on dealloc — undefined
    /// behavior / heap corruption.
    ///
    /// `leak_binary_to_ptr` now shrinks capacity to len via
    /// `into_boxed_slice()` before leaking, so the `cap == len` free path is
    /// sound. This test drives a `capacity > len` buffer through the exact
    /// alloc + free code the FFI uses and asserts the bytes survive the
    /// round-trip and the free does not corrupt the heap.
    #[test]
    fn test_leak_binary_to_ptr_preserves_capacity_via_shrink() {
        // A 65-byte compact-recoverable ECDSA signature living in a buffer
        // with far more capacity than length — the exact mismatch the old
        // `.leak()` path mishandled.
        let signature_bytes: Vec<u8> = (0u8..65).collect();
        let mut over_allocated: Vec<u8> = Vec::with_capacity(128);
        over_allocated.extend_from_slice(&signature_bytes);

        assert_eq!(over_allocated.len(), 65);
        assert!(
            over_allocated.capacity() > over_allocated.len(),
            "capacity ({}) must exceed len ({}) for this test to exercise the bug",
            over_allocated.capacity(),
            over_allocated.len(),
        );

        // Alloc side: exactly what the `Ok(Ok(signature))` arm now does.
        let (sig_ptr, sig_len) = leak_binary_to_ptr(over_allocated);
        assert!(!sig_ptr.is_null());
        assert_eq!(sig_len, 65);

        // The bytes must survive the leak unchanged.
        let round_tripped = unsafe { std::slice::from_raw_parts(sig_ptr, sig_len) };
        assert_eq!(round_tripped, &signature_bytes[..]);

        // Wrap exactly as the sign arm does, then free via the real FFI
        // free function. Under the previous code this `Vec::from_raw_parts(
        // ptr, len, len)` would hit the global allocator with a `Layout`
        // whose size (len) disagreed with the real allocation (cap == 128),
        // which is UB and would trip the allocator's debug checks / a
        // sanitizer. With the `into_boxed_slice()` shrink it is sound.
        let signature = Box::new(DashSDKSignature {
            signature: sig_ptr,
            signature_len: sig_len,
        });
        unsafe {
            dash_sdk_signature_free(Box::into_raw(signature));
        }
    }

    /// Empty-signature edge case: a zero-length leak must produce a
    /// non-dangling, freeable allocation (the free path guards on
    /// `signature_len`/null, and `into_boxed_slice` on an empty `Vec`
    /// yields a dangling-but-valid `Box<[u8]>`).
    #[test]
    fn test_leak_binary_to_ptr_empty() {
        let (sig_ptr, sig_len) = leak_binary_to_ptr(Vec::new());
        assert_eq!(sig_len, 0);

        let signature = Box::new(DashSDKSignature {
            signature: sig_ptr,
            signature_len: sig_len,
        });
        unsafe {
            dash_sdk_signature_free(Box::into_raw(signature));
        }
    }

    /// `dash_sdk_signature_free(null)` must be a no-op, not a crash.
    #[test]
    fn test_dash_sdk_signature_free_null() {
        unsafe {
            dash_sdk_signature_free(std::ptr::null_mut());
        }
    }
}
