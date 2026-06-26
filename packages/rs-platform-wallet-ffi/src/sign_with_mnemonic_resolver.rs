//! `dash_sdk_sign_with_mnemonic_resolver_and_path` — sibling of
//! the lower-level `dash_sdk_sign_with_mnemonic_and_path` in
//! `rs-sdk-ffi/src/signer_simple.rs` that uses a Swift-owned
//! [`MnemonicResolverHandle`](crate::MnemonicResolverHandle)
//! callback to fetch the mnemonic instead of taking it as a raw
//! C-string parameter.
//!
//! Same architectural intent as
//! [`crate::dash_sdk_derive_and_persist_identity_keys`]: keep the
//! Swift caller out of the mnemonic-orchestration loop so the
//! `swift-sdk/CLAUDE.md` "no mnemonic round-tripping" rule is
//! satisfied for the platform-address signing path too.
//!
//! The Swift `KeychainSigner.signPlatformAddressOnDemand` path
//! used to:
//!
//! 1. Pull the mnemonic from Keychain into a Swift `String`.
//! 2. Call the lower-level `dash_sdk_sign_with_mnemonic_and_path`
//!    with the raw mnemonic + a derivation path.
//! 3. Get back a signature.
//!
//! With this entry point the same call site can supply a
//! [`MnemonicResolver`](crate::MnemonicResolverHandle) handle
//! whose `resolve` callback is fired by Rust at the moment the
//! mnemonic is needed — Swift's only orchestration is hooking
//! the Keychain read into the resolver impl.
//!
//! # Why this lives in `platform-wallet-ffi` and not `rs-sdk-ffi`
//!
//! The [`MnemonicResolverHandle`](crate::MnemonicResolverHandle)
//! type is defined here. Hoisting it to `rs-sdk-ffi` would force
//! that crate to take on the resolver vtable just so this one
//! sibling FFI could live there too — net zero for the
//! architecture, plus extra cross-crate churn. Symbol names are
//! a flat namespace at the FFI boundary; the C caller doesn't
//! see (or care) which Rust crate produced
//! `dash_sdk_sign_with_mnemonic_resolver_and_path`.

use std::ffi::{c_void, CStr};
use std::os::raw::c_char;
use std::str::FromStr;

use crate::types::{FFINetwork, Network};
use dashcore::secp256k1::Secp256k1;
use key_wallet::bip32::{DerivationPath, ExtendedPrivKey};
use zeroize::Zeroizing;

use crate::identity_keys_from_mnemonic::parse_mnemonic_any_language;
use rs_sdk_ffi::{
    mnemonic_resolver_result, MnemonicResolverHandle, MNEMONIC_RESOLVER_BUFFER_CAPACITY,
};

// One-byte error tags. Mirror the shape of
// `signer_simple::SIGN_WITH_MNEMONIC_ERR_*` so call sites already
// familiar with that surface can read the new one without a
// translation table. Keep the same numeric values where possible.
pub const SIGN_WITH_RESOLVER_OK: u8 = 0;
pub const SIGN_WITH_RESOLVER_ERR_NULL_POINTER: u8 = 1;
pub const SIGN_WITH_RESOLVER_ERR_INVALID_UTF8: u8 = 2;
pub const SIGN_WITH_RESOLVER_ERR_INVALID_MNEMONIC: u8 = 3;
pub const SIGN_WITH_RESOLVER_ERR_INVALID_PATH: u8 = 4;
pub const SIGN_WITH_RESOLVER_ERR_DERIVATION: u8 = 5;
pub const SIGN_WITH_RESOLVER_ERR_SIGN: u8 = 6;
pub const SIGN_WITH_RESOLVER_ERR_BUFFER_TOO_SMALL: u8 = 7;
pub const SIGN_WITH_RESOLVER_ERR_UNSUPPORTED_KEY_TYPE: u8 = 8;
/// Resolver callback returned `mnemonic_resolver_result::NOT_FOUND`.
pub const SIGN_WITH_RESOLVER_ERR_RESOLVER_NOT_FOUND: u8 = 9;
/// Resolver callback returned `mnemonic_resolver_result::OTHER`.
pub const SIGN_WITH_RESOLVER_ERR_RESOLVER_FAILED: u8 = 10;

/// RAII guard that scrubs an [`ExtendedPrivKey`]'s secret scalar on drop, so an
/// early `return`, `?`, or panic between derivation and use can't leak it (the
/// type has no upstream `Drop`/`Zeroize`). Mirrors `WipingXprv` in
/// `rs-sdk-ffi/src/mnemonic_resolver_core_signer.rs`.
struct WipingXprv(ExtendedPrivKey);

impl Drop for WipingXprv {
    fn drop(&mut self) {
        self.0.private_key.non_secure_erase();
    }
}

/// Sign `data` with the ECDSA secp256k1 private key derived from
/// `(mnemonic-via-resolver, derivation_path)`. Mnemonic, seed and
/// derived secret bytes all stay in `Zeroizing` buffers and are
/// scrubbed before this function returns.
///
/// The resolver callback fires exactly once per call — at the
/// start, when the mnemonic is needed.
///
/// Same shape as `dash_sdk_sign_with_mnemonic_and_path`
/// (rs-sdk-ffi/src/signer_simple.rs) with the mnemonic argument
/// replaced by `(wallet_id_bytes, mnemonic_resolver_handle)`. The
/// `key_type` parameter exists for parity (only `0` =
/// `ECDSA_SECP256K1` is supported); other key types fail with
/// [`SIGN_WITH_RESOLVER_ERR_UNSUPPORTED_KEY_TYPE`].
///
/// Returns `0` on success, `-1` on error. On error, `*out_error`
/// is set to one of the `SIGN_WITH_RESOLVER_ERR_*` tags,
/// `*out_signature_len = 0`, and the first
/// `out_signature_capacity` bytes of `out_signature` are zeroed.
///
/// # Safety
/// - `wallet_id_bytes` must be valid for 32 readable bytes.
/// - `mnemonic_resolver_handle` must come from
///   [`crate::dash_sdk_mnemonic_resolver_create`].
/// - `derivation_path_cstr` must be a valid NUL-terminated UTF-8
///   C-string for the duration of the call.
/// - `data` must point at `data_len` readable bytes (may be zero
///   only if `data_len == 0`).
/// - `out_signature` must point at `out_signature_capacity`
///   writable bytes; `out_signature_len` and `out_error` must be
///   writable.
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub unsafe extern "C" fn dash_sdk_sign_with_mnemonic_resolver_and_path(
    mnemonic_resolver_handle: *mut MnemonicResolverHandle,
    wallet_id_bytes: *const u8,
    derivation_path_cstr: *const c_char,
    data: *const u8,
    data_len: usize,
    key_type: u8,
    network: FFINetwork,
    out_signature: *mut u8,
    out_signature_capacity: usize,
    out_signature_len: *mut usize,
    out_error: *mut u8,
) -> i32 {
    // Internal helper that scrubs `out_signature` + `*out_signature_len`
    // and returns the failure tag. Single exit point for every error path.
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
    if mnemonic_resolver_handle.is_null()
        || wallet_id_bytes.is_null()
        || derivation_path_cstr.is_null()
        || out_signature.is_null()
        || out_signature_len.is_null()
        || (data.is_null() && data_len > 0)
    {
        return fail(SIGN_WITH_RESOLVER_ERR_NULL_POINTER);
    }

    // ECDSA-only entry point. Anything else is a contract violation.
    const ECDSA_SECP256K1: u8 = 0;
    if key_type != ECDSA_SECP256K1 {
        return fail(SIGN_WITH_RESOLVER_ERR_UNSUPPORTED_KEY_TYPE);
    }

    // ---- Resolve mnemonic ----------------------------------------------------
    let mut mnemonic_buf: Zeroizing<[u8; MNEMONIC_RESOLVER_BUFFER_CAPACITY]> =
        Zeroizing::new([0u8; MNEMONIC_RESOLVER_BUFFER_CAPACITY]);
    let mut mnemonic_len: usize = 0;

    let resolver = &*mnemonic_resolver_handle;
    let resolver_vtable = &*resolver.vtable;
    let rc = (resolver_vtable.resolve)(
        resolver.ctx as *const c_void,
        wallet_id_bytes,
        mnemonic_buf.as_mut_ptr() as *mut c_char,
        MNEMONIC_RESOLVER_BUFFER_CAPACITY,
        &mut mnemonic_len,
    );
    match rc {
        x if x == mnemonic_resolver_result::SUCCESS => {}
        x if x == mnemonic_resolver_result::NOT_FOUND => {
            return fail(SIGN_WITH_RESOLVER_ERR_RESOLVER_NOT_FOUND);
        }
        x if x == mnemonic_resolver_result::BUFFER_TOO_SMALL => {
            return fail(SIGN_WITH_RESOLVER_ERR_BUFFER_TOO_SMALL);
        }
        _ => return fail(SIGN_WITH_RESOLVER_ERR_RESOLVER_FAILED),
    }
    if mnemonic_len == 0 || mnemonic_len > MNEMONIC_RESOLVER_BUFFER_CAPACITY {
        return fail(SIGN_WITH_RESOLVER_ERR_RESOLVER_FAILED);
    }

    // Parse mnemonic. UTF-8 validation runs on the prefix only —
    // we never construct a `String` (Swift's String can't be
    // zeroized; ours can).
    let mnemonic_str = match std::str::from_utf8(&mnemonic_buf[..mnemonic_len]) {
        Ok(s) => s,
        Err(_) => return fail(SIGN_WITH_RESOLVER_ERR_INVALID_UTF8),
    };
    let mnemonic = match parse_mnemonic_any_language(mnemonic_str) {
        Ok(m) => m,
        Err(_) => return fail(SIGN_WITH_RESOLVER_ERR_INVALID_MNEMONIC),
    };

    // ---- Derive seed + derivation path --------------------------------------
    let seed: Zeroizing<[u8; 64]> = Zeroizing::new(mnemonic.to_seed(""));
    drop(mnemonic);

    let path_str = match CStr::from_ptr(derivation_path_cstr).to_str() {
        Ok(s) => s,
        Err(_) => return fail(SIGN_WITH_RESOLVER_ERR_INVALID_UTF8),
    };
    let path = match DerivationPath::from_str(path_str) {
        Ok(p) => p,
        Err(_) => return fail(SIGN_WITH_RESOLVER_ERR_INVALID_PATH),
    };

    let kw_network: Network = network.into();
    // `WipingXprv` scrubs both scalars on drop, covering the early `return`
    // below (master is guarded the moment it is built) and any panic between
    // here and signing. (Upstream `ExtendedPrivKey` has no `Drop`/`Zeroize`.)
    let master = match ExtendedPrivKey::new_master(kw_network, seed.as_ref()) {
        Ok(m) => WipingXprv(m),
        Err(_) => return fail(SIGN_WITH_RESOLVER_ERR_DERIVATION),
    };
    let secp = Secp256k1::new();
    let derived = match master.0.derive_priv(&secp, &path) {
        Ok(d) => WipingXprv(d),
        Err(_) => return fail(SIGN_WITH_RESOLVER_ERR_DERIVATION),
    };
    let secret_bytes: Zeroizing<[u8; 32]> = Zeroizing::new(derived.0.private_key.secret_bytes());

    // ---- Sign ---------------------------------------------------------------
    let data_slice: &[u8] = if data_len == 0 {
        &[]
    } else {
        std::slice::from_raw_parts(data, data_len)
    };
    let sig_array = match dash_sdk::dpp::dashcore::signer::sign(data_slice, secret_bytes.as_ref()) {
        Ok(s) => s,
        Err(_) => return fail(SIGN_WITH_RESOLVER_ERR_SIGN),
    };

    if sig_array.len() > out_signature_capacity {
        return fail(SIGN_WITH_RESOLVER_ERR_BUFFER_TOO_SMALL);
    }

    std::ptr::copy_nonoverlapping(sig_array.as_ptr(), out_signature, sig_array.len());
    *out_signature_len = sig_array.len();
    if !out_error.is_null() {
        *out_error = SIGN_WITH_RESOLVER_OK;
    }
    0
}

#[cfg(test)]
mod tests {
    use super::*;
    use rs_sdk_ffi::{dash_sdk_mnemonic_resolver_create, dash_sdk_mnemonic_resolver_destroy};
    use std::ffi::CString;

    /// English BIP-39 test vector (all-zero entropy).
    const ENGLISH_PHRASE: &str =
        "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";

    unsafe extern "C" fn english_resolve(
        _ctx: *const c_void,
        _wallet_id_bytes: *const u8,
        out_buf: *mut c_char,
        out_capacity: usize,
        out_len: *mut usize,
    ) -> i32 {
        let phrase = ENGLISH_PHRASE.as_bytes();
        if phrase.len() + 1 > out_capacity {
            return mnemonic_resolver_result::BUFFER_TOO_SMALL;
        }
        std::ptr::copy_nonoverlapping(phrase.as_ptr() as *const c_char, out_buf, phrase.len());
        *out_buf.add(phrase.len()) = 0;
        *out_len = phrase.len();
        mnemonic_resolver_result::SUCCESS
    }

    unsafe extern "C" fn missing_resolve(
        _ctx: *const c_void,
        _wallet_id_bytes: *const u8,
        _out_buf: *mut c_char,
        _out_capacity: usize,
        _out_len: *mut usize,
    ) -> i32 {
        mnemonic_resolver_result::NOT_FOUND
    }

    unsafe extern "C" fn noop_destroy(_ctx: *mut c_void) {}

    fn make_resolver(cb: rs_sdk_ffi::MnemonicResolveCallback) -> *mut MnemonicResolverHandle {
        unsafe { dash_sdk_mnemonic_resolver_create(std::ptr::null_mut(), cb, noop_destroy) }
    }

    #[test]
    fn happy_path_signs_and_returns_signature() {
        let resolver = make_resolver(english_resolve);
        let path = CString::new("m/9'/1'/5'/0'/0'/0'/0'").unwrap();
        let wallet_id = [0u8; 32];
        let data = b"hello";
        let mut sig_buf = [0u8; 128];
        let mut sig_len: usize = 0;
        let mut err: u8 = 0;
        let rc = unsafe {
            dash_sdk_sign_with_mnemonic_resolver_and_path(
                resolver,
                wallet_id.as_ptr(),
                path.as_ptr(),
                data.as_ptr(),
                data.len(),
                0, // ECDSA_SECP256K1
                FFINetwork::Testnet,
                sig_buf.as_mut_ptr(),
                sig_buf.len(),
                &mut sig_len,
                &mut err,
            )
        };
        assert_eq!(rc, 0);
        assert_eq!(err, SIGN_WITH_RESOLVER_OK);
        // dashcore compact-recoverable secp256k1 signature is 65 bytes.
        assert_eq!(sig_len, 65);
        unsafe { dash_sdk_mnemonic_resolver_destroy(resolver) };
    }

    #[test]
    fn missing_resolver_surfaces_not_found_tag() {
        let resolver = make_resolver(missing_resolve);
        let path = CString::new("m/9'/1'/5'/0'/0'/0'/0'").unwrap();
        let wallet_id = [0u8; 32];
        let data = b"x";
        let mut sig_buf = [0u8; 128];
        let mut sig_len: usize = 0;
        let mut err: u8 = 0;
        let rc = unsafe {
            dash_sdk_sign_with_mnemonic_resolver_and_path(
                resolver,
                wallet_id.as_ptr(),
                path.as_ptr(),
                data.as_ptr(),
                data.len(),
                0,
                FFINetwork::Testnet,
                sig_buf.as_mut_ptr(),
                sig_buf.len(),
                &mut sig_len,
                &mut err,
            )
        };
        assert_eq!(rc, -1);
        assert_eq!(err, SIGN_WITH_RESOLVER_ERR_RESOLVER_NOT_FOUND);
        assert_eq!(sig_len, 0);
        unsafe { dash_sdk_mnemonic_resolver_destroy(resolver) };
    }

    #[test]
    fn rejects_unsupported_key_type() {
        let resolver = make_resolver(english_resolve);
        let path = CString::new("m/9'/1'/5'/0'/0'/0'/0'").unwrap();
        let wallet_id = [0u8; 32];
        let data = b"x";
        let mut sig_buf = [0u8; 128];
        let mut sig_len: usize = 0;
        let mut err: u8 = 0;
        let rc = unsafe {
            dash_sdk_sign_with_mnemonic_resolver_and_path(
                resolver,
                wallet_id.as_ptr(),
                path.as_ptr(),
                data.as_ptr(),
                data.len(),
                1, // BLS12_381 — not supported
                FFINetwork::Testnet,
                sig_buf.as_mut_ptr(),
                sig_buf.len(),
                &mut sig_len,
                &mut err,
            )
        };
        assert_eq!(rc, -1);
        assert_eq!(err, SIGN_WITH_RESOLVER_ERR_UNSUPPORTED_KEY_TYPE);
        unsafe { dash_sdk_mnemonic_resolver_destroy(resolver) };
    }

    /// The resolver path signs with the EXACT key the DIP-9 identity-auth path
    /// derives: the produced signature verifies against the compressed pubkey
    /// derived independently at the same path from the same mnemonic. This is
    /// the evidence that identity-key signing needs no dedicated FFI — the
    /// generic path primitive produces a correct, key-bound signature for a
    /// `m/9'/coin'/5'/0'/0'/identity'/key'` path just as it does for addresses.
    #[test]
    fn signs_dip9_identity_path_with_the_derived_key() {
        use key_wallet::bip32::ExtendedPubKey;

        // identity_index = 3, key_index = 2.
        let path_str = "m/9'/1'/5'/0'/0'/3'/2'";
        let resolver = make_resolver(english_resolve);
        let path = CString::new(path_str).unwrap();
        let wallet_id = [0u8; 32];
        let data = b"identity state transition bytes";

        // Independently derive the compressed pubkey at the same path from the
        // same mnemonic the resolver returns.
        let mnemonic = parse_mnemonic_any_language(ENGLISH_PHRASE).expect("mnemonic");
        let seed = mnemonic.to_seed("");
        let secp = Secp256k1::new();
        let master = ExtendedPrivKey::new_master(Network::Testnet, &seed).expect("master");
        let derived = master
            .derive_priv(&secp, &DerivationPath::from_str(path_str).unwrap())
            .expect("derive");
        let expected_pubkey = ExtendedPubKey::from_priv(&secp, &derived)
            .public_key
            .serialize();

        let mut sig_buf = [0u8; 128];
        let mut sig_len: usize = 0;
        let mut err: u8 = 0;
        let rc = unsafe {
            dash_sdk_sign_with_mnemonic_resolver_and_path(
                resolver,
                wallet_id.as_ptr(),
                path.as_ptr(),
                data.as_ptr(),
                data.len(),
                0, // ECDSA_SECP256K1
                FFINetwork::Testnet,
                sig_buf.as_mut_ptr(),
                sig_buf.len(),
                &mut sig_len,
                &mut err,
            )
        };
        assert_eq!(rc, 0);
        assert_eq!(err, SIGN_WITH_RESOLVER_OK);
        dash_sdk::dpp::dashcore::signer::verify_data_signature(
            data,
            &sig_buf[..sig_len],
            &expected_pubkey,
        )
        .expect("signature must verify against the key derived at the DIP-9 path");
        unsafe { dash_sdk_mnemonic_resolver_destroy(resolver) };
    }
}
