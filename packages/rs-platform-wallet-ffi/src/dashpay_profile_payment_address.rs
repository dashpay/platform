//! FFI bindings for the public payment addresses a DashPay profile publishes:
//! which of them the connected network lets a profile write set, and the
//! conversion between the address strings people read and paste and the
//! bytes the profile stores. The wallet decides all of it; these functions
//! only marshal.

use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::ptr;

use platform_wallet::{
    profile_payment_address_from_string, profile_payment_address_to_string,
    ProfilePaymentAddressKind,
};

use crate::error::*;
use crate::handle::*;
use crate::runtime::block_on_worker;
use crate::types::FFINetwork;
use crate::{check_ptr, unwrap_option_or_return, unwrap_result_or_return};

/// Payment address kind: `corePaymentAddress`, a Base58Check Core address.
pub const DASHPAY_PROFILE_PAYMENT_ADDRESS_CORE: u32 = 0;
/// Payment address kind: `platformPaymentAddress`, a bech32m Platform address.
pub const DASHPAY_PROFILE_PAYMENT_ADDRESS_PLATFORM: u32 = 1;
/// Payment address kind: `shieldedAddress`, a bech32m Orchard address.
pub const DASHPAY_PROFILE_PAYMENT_ADDRESS_SHIELDED: u32 = 2;

/// Which payment addresses a profile write may set on the connected network.
/// Mirrors `platform_wallet::ProfilePaymentAddressSupport`.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct DashPayPaymentAddressSupportFFI {
    pub core: bool,
    pub platform: bool,
    pub shielded: bool,
}

fn decode_kind(kind: u32) -> Result<ProfilePaymentAddressKind, PlatformWalletFFIResult> {
    match kind {
        DASHPAY_PROFILE_PAYMENT_ADDRESS_CORE => Ok(ProfilePaymentAddressKind::Core),
        DASHPAY_PROFILE_PAYMENT_ADDRESS_PLATFORM => Ok(ProfilePaymentAddressKind::Platform),
        DASHPAY_PROFILE_PAYMENT_ADDRESS_SHIELDED => Ok(ProfilePaymentAddressKind::Shielded),
        _ => Err(PlatformWalletFFIResult::err(
            PlatformWalletFFIResultCode::ErrorInvalidParameter,
            format!("Unknown payment address kind {kind}"),
        )),
    }
}

/// Which payment addresses a profile write may set on the network this
/// wallet is connected to, from
/// `DashPayView::payment_address_support`. Fetches the network's DashPay
/// contract, so it blocks on the network.
///
/// `*out_support` is all `false` on any error.
///
/// # Safety
/// `out_support` must point to writable memory for one
/// [`DashPayPaymentAddressSupportFFI`].
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_dashpay_profile_payment_address_support(
    wallet_handle: Handle,
    out_support: *mut DashPayPaymentAddressSupportFFI,
) -> PlatformWalletFFIResult {
    check_ptr!(out_support);
    *out_support = DashPayPaymentAddressSupportFFI::default();

    let option = PLATFORM_WALLET_STORAGE.with_item(wallet_handle, |wallet| {
        let identity = wallet.identity().clone();
        block_on_worker(async move { identity.dashpay().payment_address_support().await })
    });
    let result = unwrap_option_or_return!(option);
    let support = unwrap_result_or_return!(result);
    *out_support = DashPayPaymentAddressSupportFFI {
        core: support.core,
        platform: support.platform,
        shielded: support.shielded,
    };
    PlatformWalletFFIResult::ok()
}

/// Turn the address string `text` into the bytes a profile stores for the
/// payment address `kind` (a `DASHPAY_PROFILE_PAYMENT_ADDRESS_*` value), via
/// `profile_payment_address_from_string`. An address of another network, of
/// another kind, or that is not an address fails with a readable message.
///
/// On success the caller owns `*out_bytes` (`*out_len` bytes) and frees it
/// with `platform_wallet_bytes_free(*out_bytes, *out_len)`. On error
/// `*out_bytes` is null and `*out_len` is 0.
///
/// # Safety
/// `text` must be a valid NUL-terminated UTF-8 string. `out_bytes` and
/// `out_len` must point to writable memory.
#[no_mangle]
pub unsafe extern "C" fn dashpay_profile_payment_address_from_string(
    kind: u32,
    text: *const c_char,
    network: FFINetwork,
    out_bytes: *mut *mut u8,
    out_len: *mut usize,
) -> PlatformWalletFFIResult {
    check_ptr!(out_bytes);
    check_ptr!(out_len);
    // Sentinel first: `platform_wallet_bytes_free` reconstructs a `Vec` from
    // any non-null pointer, so an error path must never leave garbage here.
    *out_bytes = ptr::null_mut();
    *out_len = 0;
    check_ptr!(text);

    let kind = unwrap_result_or_return!(decode_kind(kind));
    let text = unwrap_result_or_return!(CStr::from_ptr(text).to_str());
    let bytes = unwrap_result_or_return!(profile_payment_address_from_string(
        kind,
        text,
        network.into()
    ));

    let bytes = bytes.into_boxed_slice();
    *out_len = bytes.len();
    *out_bytes = Box::into_raw(bytes) as *mut u8;
    PlatformWalletFFIResult::ok()
}

/// Render the stored `bytes` (`len` bytes) of the payment address `kind` (a
/// `DASHPAY_PROFILE_PAYMENT_ADDRESS_*` value) as the address string of
/// `network`, via `profile_payment_address_to_string`.
///
/// Succeeds with `*out_string` null when the bytes are not a valid address of
/// that kind; an unknown `kind` is an error. A non-null `*out_string` is
/// owned by the caller and freed with `platform_wallet_string_free`.
///
/// # Safety
/// `bytes` must point to `len` readable bytes, or be null when `len` is 0.
/// `out_string` must point to writable memory for one `*mut c_char`.
#[no_mangle]
pub unsafe extern "C" fn dashpay_profile_payment_address_to_string(
    kind: u32,
    bytes: *const u8,
    len: usize,
    network: FFINetwork,
    out_string: *mut *mut c_char,
) -> PlatformWalletFFIResult {
    check_ptr!(out_string);
    *out_string = ptr::null_mut();
    if len != 0 {
        check_ptr!(bytes);
    }

    let kind = unwrap_result_or_return!(decode_kind(kind));
    let bytes = if len == 0 {
        &[][..]
    } else {
        std::slice::from_raw_parts(bytes, len)
    };
    if let Some(text) = profile_payment_address_to_string(kind, bytes, network.into()) {
        *out_string = unwrap_result_or_return!(CString::new(text)).into_raw();
    }
    PlatformWalletFFIResult::ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use dashcore::address::Payload;
    use dashcore::{Address, Network, PubkeyHash};
    use dpp::address_funds::PlatformAddress;

    const HASH: [u8; 20] = [0x22; 20];

    fn core_address(network: Network) -> String {
        Address::new(
            network,
            Payload::PubkeyHash(PubkeyHash::from_byte_array(HASH)),
        )
        .to_string()
    }

    unsafe fn from_string(
        kind: u32,
        text: &str,
        network: FFINetwork,
    ) -> (PlatformWalletFFIResult, Option<Vec<u8>>) {
        let text = CString::new(text).unwrap();
        let mut bytes: *mut u8 = ptr::dangling_mut();
        let mut len: usize = usize::MAX;
        let result = dashpay_profile_payment_address_from_string(
            kind,
            text.as_ptr(),
            network,
            &mut bytes,
            &mut len,
        );
        if bytes.is_null() {
            assert_eq!(len, 0);
            return (result, None);
        }
        let owned = std::slice::from_raw_parts(bytes, len).to_vec();
        crate::platform_wallet_bytes_free(bytes, len);
        (result, Some(owned))
    }

    unsafe fn to_string(
        kind: u32,
        bytes: &[u8],
        network: FFINetwork,
    ) -> (PlatformWalletFFIResult, Option<String>) {
        let mut out: *mut c_char = ptr::dangling_mut();
        let result = dashpay_profile_payment_address_to_string(
            kind,
            bytes.as_ptr(),
            bytes.len(),
            network,
            &mut out,
        );
        if out.is_null() {
            return (result, None);
        }
        let text = CStr::from_ptr(out).to_str().unwrap().to_string();
        crate::platform_wallet_string_free(out);
        (result, Some(text))
    }

    #[test]
    fn should_round_trip_a_core_address_through_its_stored_bytes() {
        unsafe {
            let text = core_address(Network::Testnet);
            let (result, bytes) = from_string(
                DASHPAY_PROFILE_PAYMENT_ADDRESS_CORE,
                &text,
                FFINetwork::Testnet,
            );
            assert_eq!(result.code, PlatformWalletFFIResultCode::Success);
            let bytes = bytes.expect("stored bytes");
            let mut expected = vec![0x00];
            expected.extend_from_slice(&HASH);
            assert_eq!(bytes, expected);

            let (result, rendered) = to_string(
                DASHPAY_PROFILE_PAYMENT_ADDRESS_CORE,
                &bytes,
                FFINetwork::Testnet,
            );
            assert_eq!(result.code, PlatformWalletFFIResultCode::Success);
            assert_eq!(rendered, Some(text));
        }
    }

    #[test]
    fn should_round_trip_a_platform_address_through_its_stored_bytes() {
        unsafe {
            let text = PlatformAddress::P2pkh(HASH).to_bech32m_string(Network::Testnet);
            let (result, bytes) = from_string(
                DASHPAY_PROFILE_PAYMENT_ADDRESS_PLATFORM,
                &text,
                FFINetwork::Testnet,
            );
            assert_eq!(result.code, PlatformWalletFFIResultCode::Success);
            let bytes = bytes.expect("stored bytes");
            assert_eq!(bytes.len(), 21);

            let (_, rendered) = to_string(
                DASHPAY_PROFILE_PAYMENT_ADDRESS_PLATFORM,
                &bytes,
                FFINetwork::Testnet,
            );
            assert_eq!(rendered, Some(text));
        }
    }

    #[test]
    fn should_refuse_an_address_of_another_network_with_a_message() {
        unsafe {
            let (result, bytes) = from_string(
                DASHPAY_PROFILE_PAYMENT_ADDRESS_CORE,
                &core_address(Network::Mainnet),
                FFINetwork::Testnet,
            );
            assert_ne!(result.code, PlatformWalletFFIResultCode::Success);
            assert!(!result.message.is_null());
            assert!(bytes.is_none());

            let platform = PlatformAddress::P2pkh(HASH).to_bech32m_string(Network::Mainnet);
            let (result, bytes) = from_string(
                DASHPAY_PROFILE_PAYMENT_ADDRESS_PLATFORM,
                &platform,
                FFINetwork::Testnet,
            );
            assert_ne!(result.code, PlatformWalletFFIResultCode::Success);
            assert!(bytes.is_none());
        }
    }

    #[test]
    fn should_refuse_an_unknown_kind() {
        unsafe {
            let (result, bytes) =
                from_string(3, &core_address(Network::Testnet), FFINetwork::Testnet);
            assert_eq!(
                result.code,
                PlatformWalletFFIResultCode::ErrorInvalidParameter
            );
            assert!(bytes.is_none());

            let (result, rendered) = to_string(3, &[0u8; 21], FFINetwork::Testnet);
            assert_eq!(
                result.code,
                PlatformWalletFFIResultCode::ErrorInvalidParameter
            );
            assert!(rendered.is_none());
        }
    }

    #[test]
    fn should_render_nothing_for_bytes_that_are_not_an_address() {
        unsafe {
            for kind in [
                DASHPAY_PROFILE_PAYMENT_ADDRESS_CORE,
                DASHPAY_PROFILE_PAYMENT_ADDRESS_PLATFORM,
                DASHPAY_PROFILE_PAYMENT_ADDRESS_SHIELDED,
            ] {
                let (result, rendered) = to_string(kind, &[0x02; 21], FFINetwork::Testnet);
                assert_eq!(result.code, PlatformWalletFFIResultCode::Success);
                assert!(rendered.is_none());

                let (result, rendered) = to_string(kind, &[], FFINetwork::Testnet);
                assert_eq!(result.code, PlatformWalletFFIResultCode::Success);
                assert!(rendered.is_none());
            }
        }
    }

    #[test]
    fn should_reject_null_pointers() {
        unsafe {
            let mut bytes: *mut u8 = ptr::dangling_mut();
            let mut len: usize = 7;
            let result = dashpay_profile_payment_address_from_string(
                DASHPAY_PROFILE_PAYMENT_ADDRESS_CORE,
                ptr::null(),
                FFINetwork::Testnet,
                &mut bytes,
                &mut len,
            );
            assert_eq!(result.code, PlatformWalletFFIResultCode::ErrorNullPointer);
            assert!(bytes.is_null());
            assert_eq!(len, 0);

            let text = CString::new(core_address(Network::Testnet)).unwrap();
            let result = dashpay_profile_payment_address_from_string(
                DASHPAY_PROFILE_PAYMENT_ADDRESS_CORE,
                text.as_ptr(),
                FFINetwork::Testnet,
                ptr::null_mut(),
                &mut len,
            );
            assert_eq!(result.code, PlatformWalletFFIResultCode::ErrorNullPointer);

            let mut out: *mut c_char = ptr::dangling_mut();
            let result = dashpay_profile_payment_address_to_string(
                DASHPAY_PROFILE_PAYMENT_ADDRESS_CORE,
                ptr::null(),
                21,
                FFINetwork::Testnet,
                &mut out,
            );
            assert_eq!(result.code, PlatformWalletFFIResultCode::ErrorNullPointer);
            assert!(out.is_null());

            let result = dashpay_profile_payment_address_to_string(
                DASHPAY_PROFILE_PAYMENT_ADDRESS_CORE,
                ptr::null(),
                0,
                FFINetwork::Testnet,
                ptr::null_mut(),
            );
            assert_eq!(result.code, PlatformWalletFFIResultCode::ErrorNullPointer);

            let result =
                platform_wallet_dashpay_profile_payment_address_support(0, ptr::null_mut());
            assert_eq!(result.code, PlatformWalletFFIResultCode::ErrorNullPointer);
        }
    }

    #[test]
    fn should_report_no_support_for_an_unknown_wallet() {
        unsafe {
            let mut support = DashPayPaymentAddressSupportFFI {
                core: true,
                platform: true,
                shielded: true,
            };
            let result =
                platform_wallet_dashpay_profile_payment_address_support(9_999_999, &mut support);
            assert_eq!(result.code, PlatformWalletFFIResultCode::NotFound);
            assert_eq!(support, DashPayPaymentAddressSupportFFI::default());
        }
    }
}
