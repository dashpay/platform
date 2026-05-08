//! FFI helper for rendering a per-account `ExtendedPubKey` as a
//! BIP32 base58check string (`xpub…` on mainnet, `tpub…` on testnet /
//! devnet / regtest).

use bincode::config;
use key_wallet::bip32::ExtendedPubKey;
use std::ffi::CString;
use std::os::raw::c_char;
use std::ptr;

use crate::error::{PlatformWalletFFIResult, PlatformWalletFFIResultCode};
use crate::{check_ptr, unwrap_result_or_return};

/// Decode a bincode-encoded `ExtendedPubKey` (as emitted by
/// `on_persist_account_registrations_fn`) and render it as a BIP32 base58check
/// string. No network tag is required — the encoded `ExtendedPubKey`
/// carries its own `network` field so the `xpub…`/`tpub…` prefix is
/// produced automatically.
///
/// On success the caller owns `*out_string` and must free it via
/// [`platform_wallet_free_string`].
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_account_xpub_to_string(
    bytes: *const u8,
    bytes_len: usize,
    out_string: *mut *mut c_char,
) -> PlatformWalletFFIResult {
    check_ptr!(bytes);
    check_ptr!(out_string);
    if bytes_len == 0 {
        return PlatformWalletFFIResult::err(
            PlatformWalletFFIResultCode::ErrorInvalidParameter,
            "bytes_len is zero",
        );
    }
    *out_string = ptr::null_mut();

    let slice = std::slice::from_raw_parts(bytes, bytes_len);
    let (xpub, _): (ExtendedPubKey, usize) = unwrap_result_or_return!(
        bincode::decode_from_slice::<ExtendedPubKey, _>(slice, config::standard())
    );

    let cstring = unwrap_result_or_return!(CString::new(xpub.to_string()));
    *out_string = cstring.into_raw();
    PlatformWalletFFIResult::ok()
}

/// Free a C string returned by [`platform_wallet_account_xpub_to_string`].
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_free_string(s: *mut c_char) {
    if s.is_null() {
        return;
    }
    drop(CString::from_raw(s));
}
