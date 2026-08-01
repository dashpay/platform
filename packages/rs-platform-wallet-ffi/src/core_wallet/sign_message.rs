//! FFI binding for classic Dash **message signing**.
//!
//! Proves ownership of one of the wallet's P2PKH addresses by signing an
//! arbitrary short string with the key behind it, returning the base64
//! signature Dash Core's `verifymessage` RPC and dashj's `ECKey.verifyMessage`
//! accept. Unlike every other entry point in this module it moves no value:
//! nothing is selected, reserved, signed into a transaction, broadcast, or
//! persisted, so there is no reservation for the caller to discharge and no
//! funding-domain question to answer.
//!
//! See `platform_wallet::wallet::core::sign_message` for the digest
//! construction (the historical `"\x19DarkCoin Signed Message:\n"` prefix), the
//! 65-byte recoverable encoding, and why the recovery id is found by trial.

use crate::error::*;
use crate::handle::{Handle, CORE_WALLET_STORAGE};
use crate::runtime::runtime;
use crate::{check_ptr, unwrap_result_or_return};
use platform_wallet::PlatformWalletError;
use rs_sdk_ffi::{MnemonicResolverCoreSigner, MnemonicResolverHandle};
use std::ffi::CString;
use std::os::raw::c_char;

/// Read a UTF-8 string argument passed as pointer + length.
///
/// Pointer + length rather than a NUL-terminated C string because the message is
/// caller text, not an identifier: a length-delimited read cannot silently
/// truncate at an embedded NUL and sign a shorter message than the caller
/// believes it signed — which would verify for a message they never sent, a
/// divergence invisible on this side of the boundary. The address argument uses
/// the same shape for symmetry.
///
/// `on_error` builds the typed error, so each argument reports itself rather
/// than borrowing the other's phrasing.
///
/// `len == 0` yields the empty string WITHOUT dereferencing `ptr`, which may
/// legitimately be null: Swift's established marshalling
/// (`Array(s.utf8).withUnsafeBufferPointer`) hands back a nil `baseAddress` for
/// an empty string, and an empty message is signable. `from_raw_parts` is UB on
/// a null pointer even at length 0, so the zero case must short-circuit rather
/// than rely on the slice being empty.
///
/// # Safety
/// When `len > 0`, `ptr` must be non-null and readable for `len` bytes.
unsafe fn read_utf8(
    ptr: *const u8,
    len: usize,
    on_error: impl FnOnce(std::str::Utf8Error) -> PlatformWalletError,
) -> Result<String, PlatformWalletError> {
    if len == 0 {
        return Ok(String::new());
    }
    let bytes = std::slice::from_raw_parts(ptr, len);
    std::str::from_utf8(bytes)
        .map(|s| s.to_string())
        .map_err(on_error)
}

/// Sign `message` with the private key behind `address` and return the base64
/// signature — a classic Dash signed message.
///
/// * `handle` — a core-wallet handle (`platform_wallet_get_core`).
/// * `address_ptr`/`address_len` — the UTF-8 P2PKH address whose key signs. Must
///   be one of this wallet's own addresses, on the wallet's network, and belong
///   to a signable funds account (BIP44 / BIP32 / CoinJoin /
///   DashPay-receiving). A foreign address, or a watch-only DashPay *external*
///   account's address, fails with
///   [`PlatformWalletFFIResultCode::ErrorSigningKeyUnavailable`] (31); an
///   unparseable, wrong-network, or non-P2PKH address fails with
///   [`PlatformWalletFFIResultCode::ErrorInvalidParameter`] (2).
/// * `message_ptr`/`message_len` — the UTF-8 message to sign, verbatim. It is
///   length-prefixed into the digest, so trailing whitespace and newlines are
///   significant and the verifier must receive the identical bytes. An empty
///   message is valid and signable, and `message_ptr` MAY be null when
///   `message_len` is 0 — the shape host marshalling naturally produces for an
///   empty string.
/// * `core_signer_handle` — the caller's `MnemonicResolverHandle`; ownership is
///   retained by the caller (this function does NOT destroy it).
/// * `out_signature` — receives a heap-allocated C string holding the base64
///   signature. Free with [`super::core_wallet_free_address`].
///
/// # Safety
/// `address_ptr` must be non-null and readable for `address_len` bytes;
/// `message_ptr` must be readable for `message_len` bytes when that length is
/// non-zero (it may be null when the length is 0). Both must stay valid for the
/// duration of the call; `out_signature` must point to writable memory for one
/// `*mut c_char`.
#[no_mangle]
pub unsafe extern "C" fn core_wallet_sign_message(
    handle: Handle,
    address_ptr: *const u8,
    address_len: usize,
    message_ptr: *const u8,
    message_len: usize,
    core_signer_handle: *mut MnemonicResolverHandle,
    out_signature: *mut *mut c_char,
) -> PlatformWalletFFIResult {
    // An address is never legitimately empty, so its pointer must be present.
    // `message_ptr` is deliberately NOT checked: a null pointer with
    // `message_len == 0` is the empty message, which is signable (see
    // [`read_utf8`]). It is only dereferenced when the length is non-zero.
    check_ptr!(address_ptr);
    check_ptr!(core_signer_handle);
    check_ptr!(out_signature);
    *out_signature = std::ptr::null_mut();

    let signer_addr = core_signer_handle as usize;

    // Clone the Arc-backed `CoreWallet` OUT of handle storage before doing any
    // work. `with_item` holds `HandleStorage`'s process-global read guard for
    // the whole closure, and the signing round-trip below re-enters the HOST
    // through the mnemonic-resolver callback — which in production is a
    // Keychain / Android Keystore call that can block on user presence
    // (biometric or PIN prompt). Holding a global read guard across that would
    // stall every other wallet-handle operation for as long as the user takes
    // to respond, and deadlock outright if the host callback re-enters the FFI
    // on any path that needs the write guard (parking_lot's RwLock is neither
    // reentrant nor reader-preferring, so one waiting writer blocks us).
    //
    // Mirrors `core_wallet_broadcast_signed_transaction*`, which clone out for
    // the same reason.
    let Some(wallet) = CORE_WALLET_STORAGE.with_item(handle, Clone::clone) else {
        return PlatformWalletFFIResult::err(
            PlatformWalletFFIResultCode::ErrorInvalidHandle,
            "invalid core wallet handle".to_string(),
        );
    };

    let address = unwrap_result_or_return!(read_utf8(address_ptr, address_len, |e| {
        PlatformWalletError::MessageSigningAddressInvalid {
            address: "<non-UTF-8>".to_string(),
            reason: format!("address is not valid UTF-8: {e}"),
        }
    }));
    // A non-UTF-8 message is reported against the (now known) address, so the
    // error names the signing target the caller asked about.
    let message = unwrap_result_or_return!(read_utf8(message_ptr, message_len, |e| {
        PlatformWalletError::MessageSigningFailed {
            address: address.clone(),
            reason: format!("message is not valid UTF-8: {e}"),
        }
    }));

    // SAFETY: `signer_addr` came from `core_signer_handle`, which the caller
    // pinned alive for this call; the `MnemonicResolverCoreSigner` lives only on
    // this stack frame and is dropped before returning.
    let signer = MnemonicResolverCoreSigner::new(
        signer_addr as *mut MnemonicResolverHandle,
        wallet.wallet_id(),
        wallet.network(),
    );
    let signature = unwrap_result_or_return!(
        runtime().block_on(wallet.sign_message(&address, &message, &signer))
    );

    let c_str = unwrap_result_or_return!(CString::new(signature));
    *out_signature = c_str.into_raw();
    PlatformWalletFFIResult::ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use rs_sdk_ffi::{dash_sdk_mnemonic_resolver_create, dash_sdk_mnemonic_resolver_destroy};
    use std::os::raw::c_void;

    unsafe extern "C" fn never_resolve(
        _ctx: *const c_void,
        _wallet_id_bytes: *const u8,
        _out_buf: *mut c_char,
        _out_capacity: usize,
        _out_len: *mut usize,
    ) -> i32 {
        unreachable!("the handle is rejected long before any mnemonic is resolved");
    }

    unsafe extern "C" fn noop_destroy(_ctx: *mut c_void) {}

    /// `read_utf8` must treat `len == 0` as the empty string without ever
    /// touching `ptr` — a null pointer at length 0 is the shape Swift produces
    /// for an empty `String`, and `from_raw_parts` is UB on null even for an
    /// empty slice.
    #[test]
    fn read_utf8_accepts_a_null_pointer_at_zero_length() {
        let read = unsafe {
            read_utf8(std::ptr::null(), 0, |e| {
                panic!("must not attempt a UTF-8 decode: {e}")
            })
        };
        assert_eq!(read.expect("null at length 0 is the empty string"), "");
    }

    /// **The empty-message contract at the boundary.** An empty Swift `String`
    /// marshals to a nil base address with length 0, so
    /// `core_wallet_sign_message` must NOT reject a null `message_ptr` — only
    /// `address_ptr`, `core_signer_handle` and `out_signature` are
    /// null-checked.
    ///
    /// Asserted by discrimination rather than by signing: with a deliberately
    /// invalid wallet handle the call must fail at the handle lookup
    /// (`ErrorInvalidHandle`), which it can only reach by having accepted the
    /// null message. If someone later adds an unconditional
    /// `check_ptr!(message_ptr)`, the pointer checks run first and the code
    /// becomes `ErrorNullPointer` — failing this test and catching exactly the
    /// regression that would break iOS signing of an empty string.
    ///
    /// The semantic half — that `""` actually signs and verifies against
    /// `signed_msg_hash("")` — is pinned by `empty_message_is_signable` in
    /// `platform_wallet::wallet::core::sign_message`, where the wallet fixtures
    /// live.
    #[test]
    fn empty_message_is_not_a_null_pointer_error() {
        let resolver = unsafe {
            dash_sdk_mnemonic_resolver_create(std::ptr::null_mut(), never_resolve, noop_destroy)
        };
        assert!(!resolver.is_null(), "resolver handle must be created");

        let address = b"yRd4FhXfVGHXpsuZXPNkMrfD9GVj46pnjt";
        let mut out_signature: *mut c_char = std::ptr::null_mut();

        let result = unsafe {
            core_wallet_sign_message(
                Handle::MAX, // never a live core-wallet handle
                address.as_ptr(),
                address.len(),
                std::ptr::null(), // the empty message, as Swift marshals it
                0,
                resolver,
                &mut out_signature,
            )
        };

        assert_eq!(
            result.code,
            PlatformWalletFFIResultCode::ErrorInvalidHandle,
            "a null message at length 0 must be accepted and the call must fail \
             at the handle lookup, not as a null-pointer error"
        );
        assert!(out_signature.is_null());

        unsafe { dash_sdk_mnemonic_resolver_destroy(resolver) };
    }
}
