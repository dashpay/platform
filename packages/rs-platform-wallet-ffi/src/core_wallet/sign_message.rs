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
use crate::{check_ptr, unwrap_option_or_return, unwrap_result_or_return};
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

    let option = CORE_WALLET_STORAGE.with_item(handle, |wallet| {
        let address = read_utf8(address_ptr, address_len, |e| {
            PlatformWalletError::MessageSigningAddressInvalid {
                address: "<non-UTF-8>".to_string(),
                reason: format!("address is not valid UTF-8: {e}"),
            }
        })?;
        // A non-UTF-8 message is reported against the (now known) address, so the
        // error names the signing target the caller asked about.
        let message = read_utf8(message_ptr, message_len, |e| {
            PlatformWalletError::MessageSigningFailed {
                address: address.clone(),
                reason: format!("message is not valid UTF-8: {e}"),
            }
        })?;
        let network = wallet.network();
        let wallet_id = wallet.wallet_id();
        // SAFETY: `signer_addr` came from `core_signer_handle`, which the caller
        // pinned alive for this call; the `MnemonicResolverCoreSigner` lives
        // only on this stack frame and is dropped before returning.
        let signer = MnemonicResolverCoreSigner::new(
            signer_addr as *mut MnemonicResolverHandle,
            wallet_id,
            network,
        );
        runtime().block_on(wallet.sign_message(&address, &message, &signer))
    });

    let result = unwrap_option_or_return!(option);
    let signature = unwrap_result_or_return!(result);

    let c_str = unwrap_result_or_return!(CString::new(signature));
    *out_signature = c_str.into_raw();
    PlatformWalletFFIResult::ok()
}
